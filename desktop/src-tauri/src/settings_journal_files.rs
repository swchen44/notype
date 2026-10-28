//! Fixed App-owned transaction storage. Hold the OS lock through stage/recovery.
use crate::settings_journal::{Domain, Error, Storage, MAX_FILE_BYTES, MAX_JOURNAL_BYTES};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
const JOURNAL: &str = "settings-import-v1.json";

pub struct Files {
    root: PathBuf,
    _owner: File,
}
impl Files {
    pub fn open(root: &Path) -> Result<Self, Error> {
        plain(root)?;
        if !root.is_dir() {
            return Err(Error::Io);
        }
        let path = root.join("settings-import.lock");
        plain(&path)?;
        let owner = private_options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .map_err(|_| Error::Io)?;
        owner.try_lock().map_err(|error| match error {
            std::fs::TryLockError::WouldBlock => Error::Locked,
            std::fs::TryLockError::Error(_) => Error::Io,
        })?;
        Ok(Self {
            root: root.to_owned(),
            _owner: owner,
        })
    }
}
pub fn read(path: &Path, limit: usize) -> Result<Option<Vec<u8>>, Error> {
    plain(path)?;
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(Error::Io),
    };
    if !file.metadata().map_err(|_| Error::Io)?.is_file() {
        return Err(Error::Io);
    }
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| Error::Io)?;
    if bytes.len() > limit {
        return Err(Error::Invalid);
    }
    plain(path)?;
    Ok(Some(bytes))
}
/// Same-directory, exclusive temporary file; publication follows sync_all.
/// Failed writes preserve the previous destination. No path/content in errors.
pub fn atomic_write(path: &Path, bytes: &[u8], limit: usize) -> Result<(), Error> {
    if bytes.len() > limit {
        return Err(Error::Invalid);
    }
    plain(path)?;
    let parent = path.parent().ok_or(Error::Io)?;
    if available_bytes(parent)? < bytes.len() as u64 + 4096 {
        return Err(Error::NoSpace);
    }
    let temporary = parent.join(format!(
        ".notype-transfer-{}-{}.partial",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| {
        let mut file = private_options()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(io_error)?;
        file.write_all(bytes)
            .and_then(|_| file.sync_all())
            .map_err(io_error)?;
        drop(file);
        plain(path)?;
        fs::rename(&temporary, path).map_err(io_error)?;
        #[cfg(unix)]
        {
            let _ = File::open(parent).and_then(|directory| directory.sync_all());
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}
fn io_error(error: std::io::Error) -> Error {
    if error.kind() == std::io::ErrorKind::StorageFull {
        Error::NoSpace
    } else {
        Error::Io
    }
}
fn private_options() -> OpenOptions {
    #[allow(unused_mut)]
    let mut options = OpenOptions::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options
}
pub fn plain(path: &Path) -> Result<(), Error> {
    for component in path.ancestors() {
        match fs::symlink_metadata(component) {
            Ok(meta) => {
                if meta.file_type().is_symlink() {
                    return Err(Error::Io);
                }
                #[cfg(windows)]
                {
                    use std::os::windows::fs::MetadataExt;
                    if meta.file_attributes() & 0x400 != 0 {
                        return Err(Error::Io);
                    }
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(Error::Io),
        }
    }
    Ok(())
}
impl Storage for Files {
    fn available_bytes(&self) -> Result<u64, Error> {
        available_bytes(&self.root)
    }
    fn read(&self, domain: Domain) -> Result<Option<String>, Error> {
        read(&self.root.join(domain.filename()), MAX_FILE_BYTES)?
            .map(|bytes| String::from_utf8(bytes).map_err(|_| Error::Invalid))
            .transpose()
    }
    fn write(&self, domain: Domain, text: &str) -> Result<(), Error> {
        atomic_write(
            &self.root.join(domain.filename()),
            text.as_bytes(),
            MAX_FILE_BYTES,
        )
    }
    fn read_journal(&self) -> Result<Option<Vec<u8>>, Error> {
        read(&self.root.join(JOURNAL), MAX_JOURNAL_BYTES)
    }
    fn write_journal(&self, bytes: &[u8]) -> Result<(), Error> {
        atomic_write(&self.root.join(JOURNAL), bytes, MAX_JOURNAL_BYTES)
    }
    fn clear_journal(&self) -> Result<(), Error> {
        let path = self.root.join(JOURNAL);
        plain(&path)?;
        fs::remove_file(path).map_err(|_| Error::Io)
    }
}
fn available_bytes(path: &Path) -> Result<u64, Error> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn GetDiskFreeSpaceExW(
                path: *const u16,
                available: *mut u64,
                total: *mut u64,
                free: *mut u64,
            ) -> i32;
        }
        let path: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let mut available = 0;
        if unsafe {
            GetDiskFreeSpaceExW(
                path.as_ptr(),
                &mut available,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        } == 0
        {
            return Err(Error::Io);
        }
        Ok(available)
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        Ok(u64::MAX)
    }
}
