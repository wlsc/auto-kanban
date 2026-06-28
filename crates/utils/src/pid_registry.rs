use std::{
    fs,
    io,
    path::PathBuf,
};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct PidEntry {
    pub pgid: i32,
    pub pid: u32,
    pub parent_pid: u32,
    pub spawned_at: chrono::DateTime<chrono::Utc>,
}

pub fn pid_registry_dir() -> PathBuf {
    std::env::temp_dir().join("auto-kanban").join("pids")
}

pub fn register_child(execution_id: Uuid, entry: PidEntry) -> io::Result<()> {
    let dir = pid_registry_dir();
    fs::create_dir_all(&dir)?;

    let content = serde_json::to_vec(&entry)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    // Atomic write: write to temp file then rename
    let target = dir.join(format!("{execution_id}.json"));
    let tmp = dir.join(format!("{execution_id}.json.tmp"));
    fs::write(&tmp, &content)?;
    fs::rename(&tmp, &target)?;
    Ok(())
}

pub fn unregister_child(execution_id: &Uuid) -> io::Result<()> {
    let path = pid_registry_dir().join(format!("{execution_id}.json"));
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

pub fn load_all_entries() -> io::Result<Vec<(Uuid, PidEntry)>> {
    let dir = pid_registry_dir();
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut entries = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let Some(id_str) = name.strip_suffix(".json") else {
            continue;
        };
        let Ok(id) = Uuid::parse_str(id_str) else {
            continue;
        };
        let Ok(content) = fs::read(&path) else {
            continue;
        };
        let Ok(pid_entry) = serde_json::from_slice::<PidEntry>(&content) else {
            // Corrupt file, remove it
            let _ = fs::remove_file(&path);
            continue;
        };
        entries.push((id, pid_entry));
    }
    Ok(entries)
}

/// Kill an orphaned process group: SIGTERM, wait 2s, then SIGKILL if still alive.
#[cfg(unix)]
pub fn kill_orphaned_process_group(pgid: i32) {
    use nix::{
        sys::signal::{Signal, killpg},
        unistd::Pid,
    };

    let pid = Pid::from_raw(pgid);

    tracing::info!("Killing orphaned process group {pgid}");
    if killpg(pid, Signal::SIGTERM).is_err() {
        return; // group doesn't exist anymore
    }

    std::thread::sleep(std::time::Duration::from_secs(2));

    // Check if still alive by sending signal 0
    if killpg(pid, None).is_ok() {
        tracing::info!("Process group {pgid} still alive after SIGTERM, sending SIGKILL");
        let _ = killpg(pid, Signal::SIGKILL);
    }
}

#[cfg(not(unix))]
pub fn kill_orphaned_process_group(_pgid: i32) {
    // Windows: no-op for now (future: Job Objects)
}

/// Check if a given PID is still running.
#[cfg(unix)]
pub fn is_process_alive(pid: u32) -> bool {
    use nix::{
        sys::signal::kill,
        unistd::Pid,
    };
    // kill with signal 0 checks existence without sending a signal
    kill(Pid::from_raw(pid as i32), None).is_ok()
}

#[cfg(not(unix))]
pub fn is_process_alive(_pid: u32) -> bool {
    // Conservative: assume alive on non-Unix
    true
}

/// Kill all orphaned entries where the parent PID is no longer alive
/// and the entry is less than 24 hours old.
pub fn kill_stale_entries() {
    let entries = match load_all_entries() {
        Ok(e) => e,
        Err(e) => {
            tracing::warn!("Failed to load PID registry entries: {e}");
            return;
        }
    };

    let now = chrono::Utc::now();
    let max_age = chrono::Duration::hours(24);

    let mut killed = 0;
    for (id, entry) in &entries {
        let age = now - entry.spawned_at;
        if age > max_age {
            let _ = unregister_child(id);
            continue;
        }

        if !is_process_alive(entry.parent_pid) {
            eprintln!(
                "   • Killing orphaned process group {} (execution: {}, pid: {})",
                entry.pgid, id, entry.pid
            );
            tracing::info!(
                "Parent PID {} is dead for execution {id}, killing orphaned process group {}",
                entry.parent_pid,
                entry.pgid
            );
            kill_orphaned_process_group(entry.pgid);
            let _ = unregister_child(id);
            killed += 1;
        }
    }

    if killed > 0 {
        eprintln!("   Cleaned up {killed} orphaned process(es) from previous crash.");
    }
}

/// Kill every registered process group whose PID is still alive, then
/// unregister the entry. Used as a backstop on graceful shutdown so that
/// agents survive in the OS even when the DB or in-memory child store
/// has lost track of them.
///
/// Returns the number of process groups that were signalled.
pub fn kill_all_registered_children() -> usize {
    let entries = match load_all_entries() {
        Ok(e) => e,
        Err(e) => {
            tracing::warn!("Failed to load PID registry on shutdown: {e}");
            return 0;
        }
    };

    let mut killed = 0;
    for (id, entry) in entries {
        if is_process_alive(entry.pid) {
            tracing::info!(
                "Killing registered process group {} (execution {id}, pid {})",
                entry.pgid,
                entry.pid
            );
            kill_orphaned_process_group(entry.pgid);
            killed += 1;
        }
        let _ = unregister_child(&id);
    }
    killed
}
