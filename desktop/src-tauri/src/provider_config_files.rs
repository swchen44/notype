//! Dedicated fresh settings file. No old App profile is imported or deleted.
use super::{ConfigError, ConfigurationStorage};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
/// Atomic same-directory store for App-owned credentials and connection fields.
/// No method formats data/path/OS errors; failed commit preserves old disk state.
pub struct FileStorage {
    path: PathBuf,
}
impl FileStorage {
    /// Bind the dedicated path; construction performs no I/O.
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}
/// Reject symlinks/reparse points on every existing path component.
pub fn plain(path: &Path) -> Result<(), ConfigError> {
    for component in path.ancestors() {
        match fs::symlink_metadata(component) {
            Ok(meta) => {
                if meta.file_type().is_symlink() {
                    return Err(ConfigError("configuration-file-unavailable"));
                }
                #[cfg(windows)]
                {
                    use std::os::windows::fs::MetadataExt;
                    if meta.file_attributes() & 0x400 != 0 {
                        return Err(ConfigError("configuration-file-unavailable"));
                    }
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(ConfigError("configuration-file-unavailable")),
        }
    }
    Ok(())
}
impl ConfigurationStorage for FileStorage {
    fn load(&self) -> Result<Option<Vec<u8>>, ConfigError> {
        plain(&self.path)?;
        let file = match File::open(&self.path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(ConfigError("configuration-storage-failed")),
        };
        if !file
            .metadata()
            .map_err(|_| ConfigError("configuration-storage-failed"))?
            .is_file()
        {
            return Err(ConfigError("configuration-store-invalid"));
        }
        let mut bytes = Vec::new();
        file.take(4_194_305)
            .read_to_end(&mut bytes)
            .map_err(|_| ConfigError("configuration-storage-failed"))?;
        if bytes.len() > 4_194_304 {
            return Err(ConfigError("configuration-store-limit"));
        }
        plain(&self.path)?;
        Ok(Some(bytes))
    }
    fn commit(&self, bytes: &[u8]) -> Result<(), ConfigError> {
        if bytes.len() > 4_194_304 {
            return Err(ConfigError("configuration-store-limit"));
        }
        plain(&self.path)?;
        let parent = self
            .path
            .parent()
            .ok_or(ConfigError("configuration-storage-failed"))?;
        if !parent.is_dir() {
            return Err(ConfigError("configuration-storage-failed"));
        }
        let name = self
            .path
            .file_name()
            .ok_or(ConfigError("configuration-storage-failed"))?
            .to_string_lossy();
        let staging = parent.join(format!(
            ".{name}.{}-{}.partial",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let result = (|| {
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let mut file = options
                .open(&staging)
                .map_err(|_| ConfigError("configuration-storage-failed"))?;
            file.write_all(bytes)
                .and_then(|_| file.sync_all())
                .map_err(|_| ConfigError("configuration-storage-failed"))?;
            drop(file);
            plain(&self.path)?;
            fs::rename(&staging, &self.path)
                .map_err(|_| ConfigError("configuration-storage-failed"))?;
            Ok(())
        })();
        if result.is_err() && staging.exists() {
            fs::remove_file(&staging).map_err(|_| ConfigError("configuration-cleanup-failed"))?;
        }
        result
    }
}
