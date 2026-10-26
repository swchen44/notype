//! Windows artifact/readiness and quota adapters. No App/user settings or key
//! file is touched here. Metadata errors fail closed before selected resolution.
use super::{
    bridge::ResourceGate,
    journal::{plain, Failure},
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
const MIB: u64 = 1_048_576;
/// Fixed fresh live path, independent of user arguments/environment.
pub fn live_root() -> PathBuf {
    PathBuf::from("D:/git/notype-live-probes/P25-live-v4")
}
/// Synthetic no-key sibling; never a substitute live identity.
pub fn smoke_root() -> PathBuf {
    PathBuf::from("D:/git/notype-live-probes/P25-live-v4-smoke")
}
/// Bounded regular-file read; no raw OS error or path is returned.
pub fn read(path: &Path, maximum: u64) -> Result<Vec<u8>, Failure> {
    plain(path)?;
    let file = fs::File::open(path).map_err(|_| Failure::NotReady)?;
    let metadata = file.metadata().map_err(|_| Failure::Storage)?;
    if !metadata.is_file() || metadata.len() > maximum {
        return Err(Failure::Storage);
    }
    let mut bytes = vec![];
    file.take(maximum + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| Failure::Storage)?;
    if bytes.len() as u64 > maximum {
        return Err(Failure::Storage);
    }
    Ok(bytes)
}
/// Compile-time nonsecret source pin only; runtime arguments/env cannot set it.
pub fn source() -> Result<&'static str, Failure> {
    option_env!("NOTYPE_PROVIDER_PROBE_SOURCE_COMMIT")
        .filter(|value| {
            value.len() == 40
                && value
                    .bytes()
                    .all(|b| b.is_ascii_digit() || matches!(b, b'a'..=b'f'))
        })
        .ok_or(Failure::NotReady)
}
/// Actual Windows CNG SHA256 of bounded public artifact/policy/contract bytes.
/// This function is never called with a credential/header/settings document.
#[cfg(windows)]
pub fn sha256(bytes: &[u8]) -> Result<String, Failure> {
    use std::ffi::c_void;
    #[link(name = "bcrypt")]
    unsafe extern "system" {
        fn BCryptOpenAlgorithmProvider(
            handle: *mut *mut c_void,
            algorithm: *const u16,
            implementation: *const u16,
            flags: u32,
        ) -> i32;
        fn BCryptHash(
            handle: *mut c_void,
            secret: *mut u8,
            secret_length: u32,
            input: *mut u8,
            input_length: u32,
            output: *mut u8,
            output_length: u32,
        ) -> i32;
        fn BCryptCloseAlgorithmProvider(handle: *mut c_void, flags: u32) -> i32;
    }
    struct Algorithm(*mut c_void);
    impl Drop for Algorithm {
        fn drop(&mut self) {
            unsafe {
                BCryptCloseAlgorithmProvider(self.0, 0);
            }
        }
    }
    let length = u32::try_from(bytes.len()).map_err(|_| Failure::Storage)?;
    let algorithm = [
        b'S' as u16,
        b'H' as u16,
        b'A' as u16,
        b'2' as u16,
        b'5' as u16,
        b'6' as u16,
        0,
    ];
    let mut handle = std::ptr::null_mut();
    // SAFETY: fixed null-terminated SHA256 name and live output handle. CNG owns
    // the returned handle, closed exactly once. No secret/MAC input is supplied.
    if unsafe { BCryptOpenAlgorithmProvider(&mut handle, algorithm.as_ptr(), std::ptr::null(), 0) }
        < 0
        || handle.is_null()
    {
        return Err(Failure::NotReady);
    }
    let algorithm = Algorithm(handle);
    let mut digest = [0u8; 32];
    // BCryptHash's PUCHAR input is documented read-only despite its C signature:
    // https://learn.microsoft.com/windows/win32/api/bcrypt/nf-bcrypt-bcrypthash
    // SAFETY: input/output lengths match live buffers, null secret means hash,
    // and SHA256 has exactly32 output bytes. Supported on Windows10+.
    if unsafe {
        BCryptHash(
            algorithm.0,
            std::ptr::null_mut(),
            0,
            bytes.as_ptr().cast_mut(),
            length,
            digest.as_mut_ptr(),
            32,
        )
    } < 0
    {
        return Err(Failure::NotReady);
    }
    Ok(digest.iter().map(|byte| format!("{byte:02x}")).collect())
}
/// Unsupported platforms cannot claim Windows artifact/check proof.
#[cfg(not(windows))]
pub fn sha256(_: &[u8]) -> Result<String, Failure> {
    Err(Failure::NotReady)
}
/// Resolve only the current Windows LocalAppData artifact child, without an env
/// override or hardcoded username. The path never enters stdout/CSV/diagnostics.
#[cfg(windows)]
pub fn artifact_directory() -> Result<PathBuf, Failure> {
    use std::os::windows::ffi::OsStringExt;
    use windows_sys::Win32::{
        System::Com::CoTaskMemFree,
        UI::Shell::{FOLDERID_LocalAppData, SHGetKnownFolderPath},
    };
    struct Allocation(*mut u16);
    impl Drop for Allocation {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe {
                    CoTaskMemFree(self.0.cast());
                }
            }
        }
    }
    let mut raw = std::ptr::null_mut();
    // SAFETY: fixed GUID, current-user null token, live output PWSTR; free via
    // the matching CoTaskMem allocator after the bounded string copy.
    let status =
        unsafe { SHGetKnownFolderPath(&FOLDERID_LocalAppData, 0, std::ptr::null_mut(), &mut raw) };
    let allocation = Allocation(raw);
    if status < 0 || allocation.0.is_null() {
        return Err(Failure::NotReady);
    }
    let mut length = 0;
    while length < 32768 && unsafe { *allocation.0.add(length) } != 0 {
        length += 1;
    }
    if length == 0 || length == 32768 {
        return Err(Failure::NotReady);
    }
    // SAFETY: successful bounded terminator walk establishes these code units.
    let home = PathBuf::from(std::ffi::OsString::from_wide(unsafe {
        std::slice::from_raw_parts(allocation.0, length)
    }));
    let root = home.join("notype-study/build-cache/p25-live-v4-artifact");
    plain(&root)?;
    Ok(root)
}
/// There is no portable fallback to HOME/current-directory/user-supplied paths.
#[cfg(not(windows))]
pub fn artifact_directory() -> Result<PathBuf, Failure> {
    Err(Failure::NotReady)
}
/// Actual quota-aware caller-available bytes. No estimate substitutes on error.
#[cfg(windows)]
pub fn available(path: &Path) -> Result<u64, Failure> {
    use std::os::windows::ffi::OsStrExt;
    plain(path)?;
    if !path.is_dir() {
        return Err(Failure::Resource);
    }
    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    if wide.contains(&0) || wide.len() > 32766 {
        return Err(Failure::Resource);
    }
    wide.push(0);
    let mut available = 0u64;
    // SAFETY: null-terminated owned directory, live caller-available output, and
    // documented null unused outputs; no filesystem mutation occurs.
    if unsafe {
        windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW(
            wide.as_ptr(),
            &mut available,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    } == 0
    {
        return Err(Failure::Resource);
    }
    Ok(available)
}
/// Unsupported destination-space observation is failure, not unlimited space.
#[cfg(not(windows))]
pub fn available(_: &Path) -> Result<u64, Failure> {
    Err(Failure::Resource)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    source: String,
    binary_sha256: String,
    binary_bytes: u64,
    lock_sha256: String,
    workflow_sha256: String,
    toolchain: String,
    workflow_run_id: u64,
    artifact_name: String,
    imports: Vec<String>,
}
/// Parent-owned artifact/source readiness receipt, never a per-call attestation.
/// The root auditor supplies these facts only after actual independent gates.
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Readiness {
    /// Exact reviewed/merged source, checked against compiled pin and manifest.
    pub source: String,
    /// Same current EXE hash, independently verified after private download.
    pub binary_sha256: String,
    /// Actual independent Spec0 gate at this exact source.
    pub source_spec_zero: bool,
    /// Actual independent Standards0 gate at this exact source.
    pub source_standards_zero: bool,
    /// Exact-source full applicable CI completed successfully.
    pub all_ci_success: bool,
    /// Exact-source standard Windows contracts completed successfully.
    pub windows_contracts_success: bool,
    /// Service artifact digest and individual file hashes were verified.
    pub artifact_verified: bool,
    /// Actual repository/artifact visibility was verified private.
    pub private_repository: bool,
    /// Monotonic external C staging allocation, including partial/archive copies.
    pub c_allocation_hwm: u64,
    /// Actual new D launcher/instruction allocation, with no cleanup credit.
    pub d_launcher_allocation_hwm: u64,
    /// External retained runtime high-water mark; never reset between processes.
    pub d_runtime_allocation_hwm: u64,
}
/// Current verified source/binary and one externally audited staging ledger.
pub struct Verified {
    pub(super) artifact: PathBuf,
    pub(super) readiness: Readiness,
}
impl Verified {
    /// Verify only fixed local public artifact metadata and actual current EXE.
    /// This discovers no credential and creates no live/smoke state.
    pub fn load() -> Result<Self, Failure> {
        let artifact = artifact_directory()?;
        let exe = std::env::current_exe().map_err(|_| Failure::NotReady)?;
        plain(&exe)?;
        if exe.file_name() != Some(std::ffi::OsStr::new("notype-provider-probe.exe"))
            || fs::canonicalize(exe.parent().ok_or(Failure::NotReady)?)
                .map_err(|_| Failure::NotReady)?
                != fs::canonicalize(&artifact).map_err(|_| Failure::NotReady)?
        {
            return Err(Failure::NotReady);
        }
        let manifest: Manifest =
            serde_json::from_slice(&read(&artifact.join("manifest.json"), 16_384)?)
                .map_err(|_| Failure::NotReady)?;
        let readiness: Readiness =
            serde_json::from_slice(&read(&artifact.join("readiness.json"), 16_384)?)
                .map_err(|_| Failure::NotReady)?;
        if manifest.source != source()?
            || readiness.source != manifest.source
            || readiness.binary_sha256 != manifest.binary_sha256
            || manifest.binary_bytes == 0
            || manifest.binary_bytes > 32 * MIB
            || manifest.workflow_run_id == 0
            || manifest.artifact_name != "notype-provider-probe-v4"
            || manifest.toolchain != "1.99.1"
            || manifest.imports.is_empty()
            || manifest.imports.len() > 128
            || manifest.imports.iter().any(|value| {
                value.len() > 128
                    || !value
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_'))
            })
            || !readiness.source_spec_zero
            || !readiness.source_standards_zero
            || !readiness.all_ci_success
            || !readiness.windows_contracts_success
            || !readiness.artifact_verified
            || !readiness.private_repository
            || readiness.c_allocation_hwm > 112 * MIB
            || readiness.d_launcher_allocation_hwm > MIB
            || readiness.d_runtime_allocation_hwm >= 24 * MIB
        {
            return Err(Failure::NotReady);
        }
        let binary = read(&exe, 32 * MIB)?;
        if binary.len() as u64 != manifest.binary_bytes
            || sha256(&binary)? != manifest.binary_sha256
            || sha256(include_bytes!(
                "../../../../tests/integration/desktop-contracts/Cargo.lock"
            ))? != manifest.lock_sha256
            || sha256(include_bytes!(
                "../../../../.github/workflows/provider-probe-windows.yml"
            ))? != manifest.workflow_sha256
        {
            return Err(Failure::NotReady);
        }
        Ok(Self {
            artifact,
            readiness,
        })
    }
    /// Compose current resource checks before synthetic or live writes/calls.
    pub fn resources(&self, root: PathBuf) -> Arc<QuotaGate> {
        Arc::new(QuotaGate {
            root,
            c_hwm: self.readiness.c_allocation_hwm,
            d_launcher: self.readiness.d_launcher_allocation_hwm,
            runtime_hwm: Mutex::new(self.readiness.d_runtime_allocation_hwm),
        })
    }
}
/// Actual runtime resource guard, preserving pending P14/P28 coexisting budgets.
pub struct QuotaGate {
    root: PathBuf,
    c_hwm: u64,
    d_launcher: u64,
    runtime_hwm: Mutex<u64>,
}
impl QuotaGate {
    /// Retain a same-binary check's peak when entering the later live process.
    pub(super) fn retain_high_water(&self, observed: u64) -> Result<(), Failure> {
        if observed >= 24 * MIB {
            return Err(Failure::Resource);
        }
        let mut high = self.runtime_hwm.lock().map_err(|_| Failure::Resource)?;
        *high = (*high).max(observed);
        Ok(())
    }
    /// Safe monotonic peak for the external retained resource ledger.
    pub fn high_water(&self) -> Result<u64, Failure> {
        self.runtime_hwm
            .lock()
            .map(|value| *value)
            .map_err(|_| Failure::Resource)
    }
}
fn allocated(root: &Path) -> Result<u64, Failure> {
    plain(root)?;
    if !root.exists() {
        return Ok(0);
    }
    let mut total = 0u64;
    let mut pending = vec![root.to_path_buf()];
    let mut count = 0;
    while let Some(path) = pending.pop() {
        count += 1;
        if count > 64 {
            return Err(Failure::Resource);
        }
        plain(&path)?;
        let meta = fs::symlink_metadata(&path).map_err(|_| Failure::Resource)?;
        if meta.is_dir() {
            for entry in fs::read_dir(&path).map_err(|_| Failure::Resource)? {
                pending.push(entry.map_err(|_| Failure::Resource)?.path());
            }
        } else if meta.is_file() {
            total = total
                .checked_add(meta.len())
                .and_then(|value| value.checked_add(4096))
                .ok_or(Failure::Resource)?;
        } else {
            return Err(Failure::Resource);
        }
    }
    Ok(total)
}
impl ResourceGate for QuotaGate {
    fn checkpoint(&self) -> Result<(), Failure> {
        if self.root != live_root() && self.root != smoke_root() {
            return Err(Failure::Resource);
        }
        let allocated = allocated(&live_root())?
            .checked_add(allocated(&smoke_root())?)
            .ok_or(Failure::Resource)?;
        let mut high = self.runtime_hwm.lock().map_err(|_| Failure::Resource)?;
        *high = (*high).max(allocated);
        if *high >= 24 * MIB || self.c_hwm > 112 * MIB || self.d_launcher > MIB {
            return Err(Failure::Resource);
        }
        let c_remaining = 128 * MIB - self.c_hwm;
        let d_remaining = 32 * MIB - high.saturating_add(self.d_launcher).min(32 * MIB);
        // Keep the original P28 allowance, current bootstrap32MiB and Node16MiB
        // alongside the future P14 budget. Actual existing allocations remain
        // reflected in AvailableSpace; they are never refunded or deleted.
        let c_required =
            2 * 1024 * MIB + 32 * MIB + 4257 * MIB + 32 * MIB + 32 * MIB + 16 * MIB + c_remaining;
        let d_required = 1024 * MIB + 32 * MIB + 2817 * MIB + MIB + d_remaining;
        if available(Path::new("C:/"))? < c_required || available(Path::new("D:/"))? < d_required {
            return Err(Failure::Resource);
        }
        Ok(())
    }
}
impl notype_core::csv::CsvSpaceProbe for QuotaGate {
    fn available_bytes(&self, directory: &Path) -> Result<u64, notype_core::csv::CsvExportError> {
        self.checkpoint()
            .and_then(|_| {
                let current = *self.runtime_hwm.lock().map_err(|_| Failure::Resource)?;
                Ok(available(directory)?.min((24 * MIB).saturating_sub(current)))
            })
            .map_err(|_| notype_core::csv::CsvExportError::SpaceUnavailable)
    }
}
/// Write a new bounded typed receipt exclusively. Existing receipts remain
/// retained; report mode calls no write method.
pub fn receipt<T: Serialize>(path: &Path, value: &T) -> Result<(), Failure> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|_| Failure::Storage)?;
    write(path, &bytes)
}
/// Atomically commit new already-sanitized P20 text, preserving any prior file.
/// Only the owned report adapter calls this; raw logging is never connected.
pub fn text(path: &Path, text: &str) -> Result<(), Failure> {
    write(path, text.as_bytes())
}
fn write(path: &Path, bytes: &[u8]) -> Result<(), Failure> {
    plain(path)?;
    if bytes.len() > 65_536 {
        return Err(Failure::Storage);
    }
    if path.exists() {
        return Err(Failure::ReportOnly);
    }
    let partial = path.with_extension(format!(
        "{}.partial",
        path.extension()
            .and_then(|value| value.to_str())
            .ok_or(Failure::Storage)?
    ));
    plain(&partial)?;
    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&partial)
        .map_err(|_| Failure::Storage)?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| Failure::Storage)?;
    drop(file);
    fs::rename(partial, path).map_err(|_| Failure::Storage)
}
