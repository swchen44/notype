//! notype data placement is independent of the executable/npm package directory.

use std::path::PathBuf;
use std::sync::OnceLock;
use tauri::Manager;

static CONFIGURED_DATA_DIR: OnceLock<Option<PathBuf>> = OnceLock::new();

/// Capture an optional isolated per-user directory before Tauri initializes.
/// Relative overrides are rejected so a launch cannot create package-local data.
pub fn init() {
    CONFIGURED_DATA_DIR.get_or_init(|| {
        let configured = std::env::var_os("NOTYPE_DATA_DIR").map(PathBuf::from);
        if let Some(directory) = configured {
            let directory = match crate::data_policy::resolve_data_dir(&directory, None) {
                Ok(directory) => directory,
                Err(error) => {
                    eprintln!("{error}");
                    std::process::exit(2);
                }
            };
            if let Err(error) = std::fs::create_dir_all(&directory) {
                eprintln!("Unable to create notype data directory: {error}");
                std::process::exit(2);
            }
            std::env::set_var("HF_HOME", directory.join("huggingface"));
            Some(directory)
        } else {
            None
        }
    });
}

/// notype is distributed as a portable npm payload on every supported target.
pub fn is_portable() -> bool {
    true
}

/// An explicitly selected data root, if one was provided at process startup.
pub fn data_dir() -> Option<&'static PathBuf> {
    CONFIGURED_DATA_DIR
        .get()
        .and_then(|directory| directory.as_ref())
}

/// Resolve the stable per-user notype directory supplied by platform composition.
pub fn app_data_dir(app: &tauri::AppHandle) -> Result<PathBuf, tauri::Error> {
    let per_user = app.path().app_data_dir()?;
    crate::data_policy::resolve_data_dir(&per_user, data_dir().map(PathBuf::as_path))
        .map_err(std::io::Error::other)
        .map_err(Into::into)
}

/// Resolve logs outside the npm package tree.
pub fn app_log_dir(app: &tauri::AppHandle) -> Result<PathBuf, tauri::Error> {
    if let Some(directory) = data_dir() {
        Ok(directory.join("logs"))
    } else {
        app.path().app_log_dir()
    }
}

/// Resolve a relative resource in the persistent data directory.
pub fn resolve_app_data(app: &tauri::AppHandle, relative: &str) -> Result<PathBuf, tauri::Error> {
    Ok(app_data_dir(app)?.join(relative))
}

/// Settings use the same isolated root or Tauri's stable per-user app directory.
pub fn store_path(relative: &str) -> PathBuf {
    data_dir()
        .map(|directory| directory.join(relative))
        .unwrap_or_else(|| PathBuf::from(relative))
}
