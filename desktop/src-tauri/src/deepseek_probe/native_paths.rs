//! Fixed known-folder layout and actual destination-space adapter for the helper.
//! Neither environment nor command-line input selects a live data root.
use super::policy::Denied;
#[cfg(windows)]
use super::policy::ROUND_ID;
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Fixed local evidence paths. Round storage remains separate from history,
/// CSV and receipt files so its strict lock/database allowlist stays valid.
pub struct TrialPaths {
    /// Fixed exclusive one-shot round directory.
    pub round: PathBuf,
    /// Separate directory for owned sessions.db, history.csv and receipt.md.
    pub results: PathBuf,
}
impl TrialPaths {
    /// Resolve the OS known folder without reading an environment/config override.
    #[cfg(windows)]
    pub fn discover() -> Result<Self, Denied> {
        use std::os::windows::ffi::OsStringExt;
        use windows_sys::Win32::{
            System::Com::CoTaskMemFree,
            UI::Shell::{FOLDERID_LocalAppData, SHGetKnownFolderPath},
        };
        struct Allocation(*mut u16);
        impl Drop for Allocation {
            fn drop(&mut self) {
                if !self.0.is_null() {
                    // SAFETY: SHGetKnownFolderPath owns this CoTaskMem allocation; it is
                    // freed exactly once after copying, on success or error.
                    unsafe {
                        CoTaskMemFree(self.0.cast());
                    }
                }
            }
        }
        let mut raw = std::ptr::null_mut();
        // SAFETY: GUID is static, token null means current user, and raw is a
        // writable PWSTR output. No borrowed string outlives its allocation.
        let status = unsafe {
            SHGetKnownFolderPath(&FOLDERID_LocalAppData, 0, std::ptr::null_mut(), &mut raw)
        };
        let allocation = Allocation(raw);
        if status < 0 || allocation.0.is_null() {
            return Err(Denied::Storage);
        }
        let mut length = 0usize;
        // SAFETY: the successful API returns a null-terminated Windows path.
        // Bound the walk to the platform's maximum extended path length.
        while length < 32768 && unsafe { *allocation.0.add(length) } != 0 {
            length += 1;
        }
        if length == 0 || length == 32768 {
            return Err(Denied::Storage);
        }
        // SAFETY: the preceding bounded null walk establishes these live code
        // units. OsString copies them before the allocation is released.
        let folder = PathBuf::from(std::ffi::OsString::from_wide(unsafe {
            std::slice::from_raw_parts(allocation.0, length)
        }));
        if !folder.is_absolute() {
            return Err(Denied::Storage);
        }
        plain(&folder)?;
        let parent = folder.join("notype-study").join("deepseek-probe");
        Ok(Self {
            round: parent.join(ROUND_ID),
            results: parent.join(format!("{ROUND_ID}-results")),
        })
    }
    /// The helper does not provide a live path on unsupported platforms.
    #[cfg(not(windows))]
    pub fn discover() -> Result<Self, Denied> {
        Err(Denied::Storage)
    }

    /// Read-only preflight. Existing round evidence must be opened report-only;
    /// orphan result paths are rejected rather than reset before asking for a key.
    pub fn preflight(&self) -> Result<bool, Denied> {
        plain(&self.round)?;
        plain(&self.results)?;
        fn metadata(path: &Path) -> Result<Option<fs::Metadata>, Denied> {
            match fs::symlink_metadata(path) {
                Ok(meta) => Ok(Some(meta)),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
                Err(_) => Err(Denied::Storage),
            }
        }
        let round = metadata(&self.round)?;
        let results = metadata(&self.results)?;
        if round.as_ref().is_some_and(|meta| !meta.is_dir())
            || results.as_ref().is_some_and(|meta| !meta.is_dir())
            || round.is_none() && results.is_some()
        {
            return Err(Denied::Storage);
        }
        Ok(round.is_some())
    }
    /// Create only the fixed parent after local confirmation, never the round.
    /// The round adapter remains the exclusive creator of its one-shot marker.
    pub fn prepare_parent(&self) -> Result<(), Denied> {
        let parent = self.round.parent().ok_or(Denied::Storage)?;
        plain(parent)?;
        let mut missing = Vec::new();
        let mut cursor = parent;
        loop {
            match fs::symlink_metadata(cursor) {
                Ok(_) => break,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    missing.push(cursor.to_owned());
                    cursor = cursor.parent().ok_or(Denied::Storage)?;
                }
                Err(_) => return Err(Denied::Storage),
            }
        }
        for directory in missing.into_iter().rev() {
            match fs::create_dir(&directory) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(_) => return Err(Denied::Storage),
            }
            plain(&directory)?;
        }
        plain(parent)
    }
    /// Exclusively create results after the round marker exists. Failure leaves
    /// evidence intact; no automatic reset/replacement or second round is offered.
    pub fn create_results(&self) -> Result<(), Denied> {
        plain(&self.results)?;
        fs::create_dir(&self.results).map_err(|_| Denied::Storage)?;
        plain(&self.results)
    }
}

fn plain(path: &Path) -> Result<(), Denied> {
    if !path.is_absolute() {
        return Err(Denied::Storage);
    }
    for ancestor in path.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(meta) => {
                if meta.file_type().is_symlink() || !meta.is_dir() {
                    return Err(Denied::Storage);
                }
                #[cfg(windows)]
                {
                    use std::os::windows::fs::MetadataExt;
                    if meta.file_attributes() & 0x400 != 0 {
                        return Err(Denied::Storage);
                    }
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(Denied::Storage),
        }
    }
    Ok(())
}

/// Actual quota-aware destination-space observation; no estimate replaces errors.
pub struct AvailableSpace;
#[cfg(windows)]
impl notype_core::csv::CsvSpaceProbe for AvailableSpace {
    fn available_bytes(&self, directory: &Path) -> Result<u64, notype_core::csv::CsvExportError> {
        use notype_core::csv::CsvExportError;
        use std::os::windows::ffi::OsStrExt;
        plain(directory).map_err(|_| CsvExportError::SpaceUnavailable)?;
        if !directory.is_dir() {
            return Err(CsvExportError::SpaceUnavailable);
        }
        let mut wide: Vec<u16> = directory.as_os_str().encode_wide().collect();
        if wide.contains(&0) || wide.len() >= 32767 {
            return Err(CsvExportError::SpaceUnavailable);
        }
        wide.push(0);
        let mut available = 0u64;
        // SAFETY: the null-terminated directory buffer and caller-available u64
        // out-pointer live across the call; unused optional outputs are null.
        if unsafe {
            windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW(
                wide.as_ptr(),
                &mut available,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        } == 0
        {
            return Err(CsvExportError::SpaceUnavailable);
        }
        Ok(available)
    }
}
#[cfg(not(windows))]
impl notype_core::csv::CsvSpaceProbe for AvailableSpace {
    fn available_bytes(&self, _: &Path) -> Result<u64, notype_core::csv::CsvExportError> {
        Err(notype_core::csv::CsvExportError::SpaceUnavailable)
    }
}
