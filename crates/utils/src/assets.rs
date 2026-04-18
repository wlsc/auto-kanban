use directories::ProjectDirs;
use rust_embed::RustEmbed;

const PROJECT_ROOT: &str = env!("CARGO_MANIFEST_DIR");

pub fn asset_dir() -> std::path::PathBuf {
    let path = if cfg!(debug_assertions) {
        std::path::PathBuf::from(PROJECT_ROOT).join("../../dev_assets")
    } else {
        let new_dirs = ProjectDirs::from("", "", "auto-kanban")
            .expect("OS didn't give us a home directory");
        let new_path = new_dirs.data_dir().to_path_buf();

        // Migrate from legacy "ai.bloop.auto-kanban" / "de.wlsc.auto-kanban" directory if it exists
        if !new_path.exists() {
            if let Some(old_dirs) = ProjectDirs::from("ai", "bloop", "auto-kanban") {
                let old_path = old_dirs.data_dir().to_path_buf();
                if old_path.exists() {
                    tracing::info!(
                        "Migrating data directory from {} to {}",
                        old_path.display(),
                        new_path.display()
                    );
                    if let Err(e) = std::fs::rename(&old_path, &new_path) {
                        tracing::warn!(
                            "Failed to migrate data directory (will copy instead): {e}"
                        );
                        // Fall through — the directory will be created below
                    }
                }
            }
        }

        new_path
    };

    // Ensure the directory exists
    if !path.exists() {
        std::fs::create_dir_all(&path).expect("Failed to create asset directory");
    }

    path
    // ✔ macOS → ~/Library/Application Support/auto-kanban
    // ✔ Linux → ~/.local/share/auto-kanban   (respects XDG_DATA_HOME)
    // ✔ Windows → %APPDATA%\auto-kanban
}

pub fn config_path() -> std::path::PathBuf {
    asset_dir().join("config.json")
}

pub fn profiles_path() -> std::path::PathBuf {
    asset_dir().join("profiles.json")
}

pub fn credentials_path() -> std::path::PathBuf {
    asset_dir().join("credentials.json")
}

#[derive(RustEmbed)]
#[folder = "../../assets/sounds"]
pub struct SoundAssets;

#[derive(RustEmbed)]
#[folder = "../../assets/scripts"]
pub struct ScriptAssets;
