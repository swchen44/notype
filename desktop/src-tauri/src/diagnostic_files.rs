//! Dedicated bounded diagnostics preference and atomic manual-export storage.
//! Never reads the main settings file, raw logs, credentials or user content.

use crate::diagnostics::service::DiagnosticStorage;
use std::{
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Production file adapter. The resolved dedicated preference path is private;
/// None represents failed app-data resolution and fails closed on all saves.
pub struct FileStorage {
    preference: Option<PathBuf>,
}
impl FileStorage {
    /// Bind only the diagnostics preference path; construction performs no I/O.
    pub fn new(preference: Option<PathBuf>) -> Self {
        Self { preference }
    }
}
impl DiagnosticStorage for FileStorage {
    fn load_preference(&self) -> Result<Option<bool>, &'static str> {
        let path = self
            .preference
            .as_ref()
            .ok_or("diagnostic-preference-load-failed")?;
        let file = match File::open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err("diagnostic-preference-load-failed"),
        };
        let mut bytes = Vec::new();
        file.take(4097)
            .read_to_end(&mut bytes)
            .map_err(|_| "diagnostic-preference-load-failed")?;
        if bytes.len() > 4096 {
            return Err("diagnostic-preference-load-failed");
        }
        let value: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|_| "diagnostic-preference-load-failed")?;
        let object = value
            .as_object()
            .ok_or("diagnostic-preference-load-failed")?;
        if object
            .get("schema_version")
            .is_some_and(|version| version.as_u64() != Some(1))
        {
            return Err("diagnostic-preference-load-failed");
        }
        match object.get("diagnostics_enabled") {
            None => Ok(None),
            Some(value) => value
                .as_bool()
                .map(Some)
                .ok_or("diagnostic-preference-load-failed"),
        }
    }
    fn save_preference(&self, enabled: bool) -> Result<(), &'static str> {
        let path = self
            .preference
            .as_ref()
            .ok_or("diagnostic-preference-save-failed")?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|_| "diagnostic-preference-save-failed")?;
        }
        atomic_write(
            path,
            format!("{{\"schema_version\":1,\"diagnostics_enabled\":{enabled}}}\n").as_bytes(),
        )
        .map_err(|_| "diagnostic-preference-save-failed")
    }
    fn write_report(&self, destination: &Path, text: &str) -> Result<(), &'static str> {
        atomic_write(destination, text.as_bytes()).map_err(|_| "diagnostic-save-failed")
    }
}

fn atomic_write(destination: &Path, bytes: &[u8]) -> Result<(), &'static str> {
    let parent = destination.parent().ok_or("diagnostic-save-failed")?;
    let basename = destination.file_name().ok_or("diagnostic-save-failed")?;
    let mut staged = None;
    for _ in 0..8 {
        let mut name = OsString::from(".notype-");
        name.push(basename);
        name.push(format!(
            "-{}-{}.tmp",
            std::process::id(),
            TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let path = parent.join(name);
        match OpenOptions::new().create_new(true).write(true).open(&path) {
            Ok(file) => {
                staged = Some((path, file));
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(_) => return Err("diagnostic-save-failed"),
        }
    }
    let (temporary, file) = staged.ok_or("diagnostic-save-failed")?;
    let result = (|| {
        let mut file = file;
        file.write_all(bytes)?;
        file.flush()?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, destination)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result.map_err(|_| "diagnostic-save-failed")
}
