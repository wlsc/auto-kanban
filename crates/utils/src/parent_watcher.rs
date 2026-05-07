use crate::pid_registry;

/// Spawn a background thread that watches for the parent process to die.
/// When detected, kills all registered child processes and exits.
///
/// On macOS, uses kqueue(EVFILT_PROC, NOTE_EXIT) for instant notification.
/// On Linux, polls with kill(ppid, 0) every 5 seconds.
pub fn spawn_parent_death_watcher() {
    let parent_pid = get_parent_pid();
    if parent_pid <= 1 {
        // Already orphaned (init/launchd is parent) or can't determine
        return;
    }

    std::thread::spawn(move || {
        tracing::debug!("Parent death watcher started, monitoring PID {parent_pid}");
        wait_for_parent_exit(parent_pid);
        tracing::warn!("Parent process {parent_pid} died, cleaning up child processes");
        pid_registry::kill_stale_entries();
        std::process::exit(1);
    });
}

#[cfg(unix)]
fn get_parent_pid() -> u32 {
    nix::unistd::getppid().as_raw() as u32
}

#[cfg(not(unix))]
fn get_parent_pid() -> u32 {
    0 // Can't reliably get parent PID on Windows without extra deps
}

#[cfg(target_os = "macos")]
fn wait_for_parent_exit(parent_pid: u32) {
    use std::mem;

    // kqueue + EVFILT_PROC with NOTE_EXIT gives instant notification
    unsafe {
        let kq = libc::kqueue();
        if kq < 0 {
            tracing::warn!("kqueue() failed, falling back to polling");
            poll_parent_exit(parent_pid);
            return;
        }

        let mut event: libc::kevent = mem::zeroed();
        event.ident = parent_pid as usize;
        event.filter = libc::EVFILT_PROC;
        event.flags = libc::EV_ADD | libc::EV_ONESHOT;
        event.fflags = libc::NOTE_EXIT;
        event.data = 0;
        event.udata = std::ptr::null_mut();

        let ret = libc::kevent(kq, &event, 1, std::ptr::null_mut(), 0, std::ptr::null());
        if ret < 0 {
            libc::close(kq);
            tracing::warn!("kevent() registration failed, falling back to polling");
            poll_parent_exit(parent_pid);
            return;
        }

        // Wait for the event (blocks until parent exits)
        let mut out_event: libc::kevent = mem::zeroed();
        let n = libc::kevent(
            kq,
            std::ptr::null(),
            0,
            &mut out_event as *mut _,
            1,
            std::ptr::null(), // no timeout - wait forever
        );
        libc::close(kq);

        if n < 0 {
            tracing::warn!("kevent() wait failed, falling back to polling");
            poll_parent_exit(parent_pid);
        }
        // n >= 1 means parent exited
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
fn wait_for_parent_exit(parent_pid: u32) {
    poll_parent_exit(parent_pid);
}

#[cfg(not(unix))]
fn wait_for_parent_exit(_parent_pid: u32) {
    // Windows: no parent death detection, just sleep forever
    loop {
        std::thread::sleep(std::time::Duration::from_secs(3600));
    }
}

#[cfg(unix)]
fn poll_parent_exit(parent_pid: u32) {
    loop {
        if !pid_registry::is_process_alive(parent_pid) {
            return;
        }
        std::thread::sleep(std::time::Duration::from_secs(5));
    }
}
