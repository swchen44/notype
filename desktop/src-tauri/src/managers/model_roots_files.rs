//! Bounded filesystem adapter for model roots. Validation, scans and activation
//! never modify the selected folder. Only the separate preference file is written.
use super::model_roots::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::UNIX_EPOCH,
};
const MAX_ENTRIES: usize = 2048;
const MAX_STORE: usize = 16_384;
static NEXT: AtomicU64 = AtomicU64::new(1);

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Preference {
    schema: u8,
    revision: u64,
    root: String,
}
pub fn encode(root: &Path, revision: u64) -> RootResult<Vec<u8>> {
    let root = display_path(root);
    let bytes = serde_json::to_vec(&Preference {
        schema: 1,
        revision,
        root,
    })
    .map_err(|_| RootError("model-root-persistence-failed"))?;
    if bytes.len() > MAX_STORE {
        return Err(RootError("model-root-limit"));
    }
    Ok(bytes)
}
pub fn decode(bytes: &[u8]) -> RootResult<(PathBuf, u64)> {
    if bytes.len() > MAX_STORE {
        return Err(RootError("model-root-limit"));
    }
    let preference: Preference =
        serde_json::from_slice(bytes).map_err(|_| RootError("model-root-invalid"))?;
    if preference.schema != 1
        || preference.revision >= 9_007_199_254_740_991
        || preference.root.is_empty()
    {
        return Err(RootError("model-root-invalid"));
    }
    Ok((PathBuf::from(preference.root), preference.revision))
}
pub fn display_path(path: &Path) -> String {
    let text = path.to_string_lossy();
    #[cfg(windows)]
    {
        return text.strip_prefix(r"\\?\").unwrap_or(&text).to_owned();
    }
    #[cfg(not(windows))]
    {
        text.into_owned()
    }
}

pub struct FileStorage {
    path: PathBuf,
}
impl FileStorage {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}
impl RootStorage for FileStorage {
    fn accepts_root(&self, root: &Path) -> bool {
        let Some(parent) = self.path.parent() else {
            return false;
        };
        let Ok(parent) = fs::canonicalize(parent) else {
            return false;
        };
        !parent.starts_with(root)
    }
    fn load(&self) -> RootResult<Option<Vec<u8>>> {
        plain_ancestors(&self.path)?;
        let file = match File::open(&self.path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(RootError("model-root-persistence-failed")),
        };
        if !file
            .metadata()
            .map_err(|_| RootError("model-root-persistence-failed"))?
            .is_file()
        {
            return Err(RootError("model-root-invalid"));
        }
        let mut bytes = Vec::new();
        file.take((MAX_STORE + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|_| RootError("model-root-persistence-failed"))?;
        if bytes.len() > MAX_STORE {
            return Err(RootError("model-root-limit"));
        }
        plain_ancestors(&self.path)?;
        Ok(Some(bytes))
    }
    fn commit(&self, bytes: &[u8]) -> RootResult<()> {
        if bytes.len() > MAX_STORE {
            return Err(RootError("model-root-limit"));
        }
        plain_ancestors(&self.path)?;
        let parent = self
            .path
            .parent()
            .ok_or(RootError("model-root-persistence-failed"))?;
        let staging = parent.join(format!(
            ".notype-model-root-{}-{}.partial",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let mut created = false;
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
                .map_err(|_| RootError("model-root-persistence-failed"))?;
            created = true;
            file.write_all(bytes)
                .and_then(|_| file.sync_all())
                .map_err(|_| RootError("model-root-persistence-failed"))?;
            drop(file);
            plain_ancestors(&self.path)?;
            fs::rename(&staging, &self.path).map_err(|_| RootError("model-root-persistence-failed"))
        })();
        if result.is_err() && created {
            let _ = fs::remove_file(&staging);
        }
        result
    }
}

fn reparse(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return true;
        }
    }
    false
}
pub fn plain_ancestors(path: &Path) -> RootResult<()> {
    for component in path.ancestors() {
        match fs::symlink_metadata(component) {
            Ok(metadata) if reparse(&metadata) => return Err(RootError("model-root-invalid")),
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(RootError("model-root-unavailable")),
        }
    }
    Ok(())
}
fn cancelled(cancel: &AtomicBool) -> RootResult<()> {
    if cancel.load(Ordering::Acquire) {
        Err(RootError("model-root-cancelled"))
    } else {
        Ok(())
    }
}
fn input_path(path: &Path) -> RootResult<()> {
    let text = path.to_str().ok_or(RootError("model-root-invalid"))?;
    if !path.is_absolute() || text.len() > 4096 || text.chars().any(|c| c.is_control()) {
        return Err(RootError("model-root-invalid"));
    }
    #[cfg(windows)]
    {
        let bytes = text.as_bytes();
        if bytes.len() < 3
            || !bytes[0].is_ascii_alphabetic()
            || bytes[1] != b':'
            || !matches!(bytes[2], b'/' | b'\\')
            || text[2..].contains(':')
        {
            return Err(RootError("model-root-invalid"));
        }
        if !windows_io::local_drive(bytes[0]) {
            return Err(RootError("model-root-invalid"));
        }
        for part in text[3..].split(['/', '\\']) {
            if part.ends_with(['.', ' ']) {
                return Err(RootError("model-root-invalid"));
            }
        }
    }
    Ok(())
}
fn open_plain(path: &Path, directory: bool, hold: bool) -> RootResult<File> {
    plain_ancestors(path)?;
    let metadata = fs::symlink_metadata(path).map_err(|_| RootError("model-root-unavailable"))?;
    if reparse(&metadata)
        || (directory && !metadata.is_dir())
        || (!directory && !metadata.is_file())
    {
        return Err(RootError("model-root-invalid"));
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.share_mode(if hold {
            if directory {
                3
            } else {
                1
            }
        } else {
            7
        });
        options.custom_flags(0x0020_0000 | if directory { 0x0200_0000 } else { 0 });
    }
    #[cfg(not(windows))]
    let _ = hold;
    let file = options
        .open(path)
        .map_err(|_| RootError("model-root-unavailable"))?;
    plain_ancestors(path)?;
    Ok(file)
}
fn identity(file: &File) -> RootResult<Fingerprint> {
    let metadata = file
        .metadata()
        .map_err(|_| RootError("model-root-unavailable"))?;
    let modified = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|t| t.as_nanos())
        .unwrap_or(0);
    #[cfg(unix)]
    let (volume, index) = {
        use std::os::unix::fs::MetadataExt;
        (metadata.dev(), metadata.ino())
    };
    #[cfg(windows)]
    let (volume, index) = windows_io::identity(file)?;
    #[cfg(not(any(unix, windows)))]
    let (volume, index) = (0, 0);
    Ok(Fingerprint {
        volume,
        file: index,
        size: metadata.len(),
        modified,
    })
}
fn same_directory(left: &Fingerprint, right: &Fingerprint) -> bool {
    left.volume == right.volume && left.file == right.file
}
fn name_ok(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 512
        && name != "."
        && name != ".."
        && !name.contains(['/', '\\', ':'])
        && !name.chars().any(|c| c.is_control())
}
fn hash(file: &mut File, cancel: &AtomicBool) -> RootResult<String> {
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 65_536];
    loop {
        cancelled(cancel)?;
        let read = file
            .read(&mut buffer)
            .map_err(|_| RootError("model-artifact-invalid"))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

/// Keeps the exact file and root ancestry pinned while native code opens/uses
/// the path. Windows denies file replacement/write and ancestor replacement.
pub struct ArtifactUse {
    pub key: ArtifactKey,
    pub path: PathBuf,
    _handles: Vec<File>,
}
pub struct RootUse {
    _handles: Vec<File>,
}
type Prober = dyn Fn(&Path) -> bool + Send + Sync;
pub struct Filesystem {
    hashing: Mutex<()>,
    custom: Arc<Prober>,
}
impl Filesystem {
    pub fn new(prober: impl Fn(&Path) -> bool + Send + Sync + 'static) -> Self {
        Self {
            hashing: Mutex::new(()),
            custom: Arc::new(prober),
        }
    }
    pub fn create_default(&self, path: &Path) -> RootResult<()> {
        plain_ancestors(path)?;
        fs::create_dir_all(path).map_err(|_| RootError("model-root-unavailable"))
    }
    pub fn canonical_root(&self, path: &Path) -> RootResult<PathBuf> {
        input_path(path)?;
        plain_ancestors(path)?;
        let canonical = fs::canonicalize(path).map_err(|_| RootError("model-root-unavailable"))?;
        open_plain(&canonical, true, false)?;
        fs::read_dir(&canonical).map_err(|_| RootError("model-root-unavailable"))?;
        Ok(canonical)
    }
    pub fn valid_input(&self, path: &Path) -> bool {
        input_path(path).is_ok()
    }
    pub fn writable(&self, root: &Path) -> RootResult<bool> {
        let file = open_plain(root, true, false)?;
        if file
            .metadata()
            .map_err(|_| RootError("model-root-unavailable"))?
            .permissions()
            .readonly()
        {
            return Ok(false);
        }
        #[cfg(windows)]
        {
            return Ok(windows_io::writable(root));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            Ok(file
                .metadata()
                .map_err(|_| RootError("model-root-unavailable"))?
                .permissions()
                .mode()
                & 0o222
                != 0)
        }
        #[cfg(not(any(windows, unix)))]
        {
            Ok(false)
        }
    }
    /// Snapshot-file links are the sole allowed link form; resolve exactly one
    /// link and require its plain regular target in this repo's blobs subtree.
    fn resolve_hf(
        &self,
        hub: &Path,
        descriptor: &ArtifactDescriptor,
    ) -> RootResult<Option<PathBuf>> {
        let Some(repo) = descriptor.repo.as_deref() else {
            return Ok(None);
        };
        if !repo.split('/').all(name_ok) {
            return Err(RootError("model-root-invalid"));
        }
        let repo = hub.join(format!("models--{}", repo.replace('/', "--")));
        let mut revisions = Vec::new();
        if let Some(revision) = descriptor.revision.as_ref().filter(|r| name_ok(r)) {
            revisions.push(revision.clone());
        }
        let reference = repo.join("refs/main");
        if reference.exists() {
            let mut bytes = String::new();
            open_plain(&reference, false, false)?
                .take(129)
                .read_to_string(&mut bytes)
                .map_err(|_| RootError("model-root-invalid"))?;
            let revision = bytes.trim();
            if revision.len() <= 128 && name_ok(revision) {
                revisions.push(revision.to_owned());
            }
        }
        for revision in revisions {
            let pointer = repo
                .join("snapshots")
                .join(revision)
                .join(&descriptor.filename);
            let metadata = match fs::symlink_metadata(&pointer) {
                Ok(metadata) => metadata,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(_) => return Err(RootError("model-root-unavailable")),
            };
            plain_ancestors(pointer.parent().ok_or(RootError("model-root-invalid"))?)?;
            if metadata.file_type().is_symlink() {
                let link = fs::read_link(&pointer).map_err(|_| RootError("model-root-invalid"))?;
                let target = pointer.parent().unwrap().join(link);
                plain_ancestors(&target)?;
                let target =
                    fs::canonicalize(target).map_err(|_| RootError("model-artifact-invalid"))?;
                let blobs = fs::canonicalize(repo.join("blobs"))
                    .map_err(|_| RootError("model-root-invalid"))?;
                if target.parent() != Some(blobs.as_path()) {
                    return Err(RootError("model-root-invalid"));
                }
                open_plain(&target, false, false)?;
                return Ok(Some(target));
            }
            if reparse(&metadata) {
                return Err(RootError("model-root-invalid"));
            }
            open_plain(&pointer, false, false)?;
            return Ok(Some(pointer));
        }
        Ok(None)
    }
    pub fn scan(
        &self,
        root: &RootSnapshot,
        catalog: &[ArtifactDescriptor],
        cancel: &AtomicBool,
        progress: &(dyn Fn(u32, u32) + Send + Sync),
    ) -> RootResult<Projection> {
        if !root.valid_path {
            return Err(RootError("model-root-invalid"));
        }
        let _one_hash_worker = self.hashing.lock().unwrap_or_else(|e| e.into_inner());
        cancelled(cancel)?;
        let directory = open_plain(&root.root, true, false)?;
        let root_identity = identity(&directory)?;
        let mut entries = Vec::new();
        for entry in fs::read_dir(&root.root).map_err(|_| RootError("model-root-unavailable"))? {
            cancelled(cancel)?;
            if entries.len() >= MAX_ENTRIES {
                return Err(RootError("model-root-limit"));
            }
            let entry = entry.map_err(|_| RootError("model-root-unavailable"))?;
            if let Some(name) = entry.file_name().to_str().filter(|s| name_ok(s)) {
                entries.push(name.to_owned());
            }
        }
        let mut projection = Projection {
            root_identity: Some(root_identity),
            artifacts: BTreeMap::new(),
            writable: self.writable(&root.root)?,
            status: RootStatus::NoModels,
        };
        let total = (catalog.len() + entries.len()).min(u32::MAX as usize) as u32;
        let mut checked = 0;
        for descriptor in catalog {
            cancelled(cancel)?;
            if !name_ok(&descriptor.filename) {
                return Err(RootError("model-root-invalid"));
            }
            let direct = root.root.join(&descriptor.filename);
            let partial = root.root.join(format!("{}.partial", descriptor.filename));
            let extracting = root
                .root
                .join(format!("{}.extracting", descriptor.filename));
            let mut legacy = false;
            let mut hf_binding = None;
            let mut path = None;
            let mut status = ArtifactStatus::Unavailable;
            if partial.exists() || extracting.exists() {
                status = ArtifactStatus::Incomplete;
            } else if fs::symlink_metadata(&direct).is_ok() {
                path = Some(direct.clone());
            } else if let Some(found) =
                self.resolve_hf(&root.root.join(".hf-cache/hub"), descriptor)?
            {
                path = Some(found);
                hf_binding = Some((root.root.join(".hf-cache/hub"), descriptor.clone()));
            } else if let Some(hub) = &root.legacy_cache {
                if let Some(found) = self.resolve_hf(hub, descriptor)? {
                    path = Some(found);
                    legacy = true;
                    hf_binding = Some((hub.clone(), descriptor.clone()));
                }
            }
            let mut fingerprint = Fingerprint {
                volume: 0,
                file: 0,
                size: 0,
                modified: 0,
            };
            let mut digest = String::new();
            if let Some(candidate) = &path {
                if descriptor.directory {
                    // Archive hashes are not required-member hash contracts.
                    status = ArtifactStatus::Unavailable;
                } else {
                    match open_plain(candidate, false, true) {
                        Ok(mut file) => {
                            fingerprint = identity(&file)?;
                            if descriptor.size.is_some_and(|size| size != fingerprint.size) {
                                status = ArtifactStatus::Invalid;
                            } else if let Some(expected) = &descriptor.sha256 {
                                digest = hash(&mut file, cancel)?;
                                status = if digest == *expected && identity(&file)? == fingerprint {
                                    ArtifactStatus::Verified
                                } else {
                                    ArtifactStatus::Invalid
                                };
                            } else {
                                status = ArtifactStatus::Unavailable;
                            }
                        }
                        Err(_) => status = ArtifactStatus::Invalid,
                    }
                }
            }
            projection.artifacts.insert(
                descriptor.model_id.clone(),
                ArtifactRecord {
                    key: ArtifactKey {
                        generation: root.generation,
                        model_id: descriptor.model_id.clone(),
                        content_identity: digest,
                        fingerprint,
                    },
                    filename: descriptor.filename.clone(),
                    present: path.is_some() || status == ArtifactStatus::Incomplete,
                    path: path.unwrap_or(direct),
                    status,
                    legacy,
                    custom: false,
                    owned: false,
                    hf_binding,
                },
            );
            checked += 1;
            progress(checked, total);
        }
        for filename in entries {
            cancelled(cancel)?;
            checked += 1;
            progress(checked, total);
            if filename.starts_with('.')
                || !filename.ends_with(".gguf")
                || catalog.iter().any(|d| d.filename == filename)
            {
                continue;
            }
            let path = root.root.join(&filename);
            let Ok(mut file) = open_plain(&path, false, true) else {
                continue;
            };
            if !(self.custom)(&path) {
                continue;
            }
            let fingerprint = identity(&file)?;
            let digest = hash(&mut file, cancel)?;
            if identity(&file)? != fingerprint {
                return Err(RootError("model-root-stale"));
            }
            let id = filename.trim_end_matches(".gguf").to_owned();
            let incomplete = root.root.join(format!("{filename}.partial")).exists();
            projection.artifacts.insert(
                id.clone(),
                ArtifactRecord {
                    key: ArtifactKey {
                        generation: root.generation,
                        model_id: id,
                        content_identity: digest,
                        fingerprint,
                    },
                    filename,
                    path,
                    status: if incomplete {
                        ArtifactStatus::Incomplete
                    } else {
                        ArtifactStatus::CustomUnverified
                    },
                    legacy: false,
                    custom: true,
                    owned: false,
                    present: true,
                    hf_binding: None,
                },
            );
        }
        self.revalidate(root, &projection)?;
        projection.status = if projection
            .artifacts
            .values()
            .any(|r| r.status == ArtifactStatus::Invalid)
        {
            RootStatus::Invalid
        } else if projection.count() != 0 {
            RootStatus::Ready
        } else {
            RootStatus::NoModels
        };
        Ok(projection)
    }
    pub fn revalidate(&self, root: &RootSnapshot, projection: &Projection) -> RootResult<()> {
        self.check_root_identity(root, projection)?;
        for record in projection
            .artifacts
            .values()
            .filter(|r| r.status.loadable())
        {
            if let Some((hub, descriptor)) = &record.hf_binding {
                if self.resolve_hf(hub, descriptor)?.as_ref() != Some(&record.path) {
                    return Err(RootError("model-root-stale"));
                }
            }
            let file = open_plain(&record.path, false, true)?;
            if identity(&file)? != record.key.fingerprint {
                return Err(RootError("model-root-stale"));
            }
        }
        Ok(())
    }
    pub fn check_root_identity(
        &self,
        root: &RootSnapshot,
        projection: &Projection,
    ) -> RootResult<()> {
        if !root.valid_path {
            return Err(RootError("model-root-invalid"));
        }
        let current = identity(&open_plain(&root.root, true, false)?)?;
        if !projection
            .root_identity
            .as_ref()
            .is_some_and(|old| same_directory(old, &current))
        {
            return Err(RootError("model-root-stale"));
        }
        Ok(())
    }
    pub fn hold_root(&self, root: &RootSnapshot, projection: &Projection) -> RootResult<RootUse> {
        let handles = root
            .root
            .ancestors()
            .map(|path| open_plain(path, true, true))
            .collect::<RootResult<Vec<_>>>()?;
        self.check_root_identity(root, projection)?;
        Ok(RootUse { _handles: handles })
    }
    pub fn hold_projection(
        &self,
        root: &RootSnapshot,
        projection: &Projection,
    ) -> RootResult<RootUse> {
        let mut held = self.hold_root(root, projection)?;
        for record in projection
            .artifacts
            .values()
            .filter(|record| record.status.loadable())
        {
            let artifact = self.acquire(root, record)?;
            held._handles.extend(artifact._handles);
        }
        self.revalidate(root, projection)?;
        Ok(held)
    }
    pub fn acquire(&self, root: &RootSnapshot, record: &ArtifactRecord) -> RootResult<ArtifactUse> {
        if let Some((hub, descriptor)) = &record.hf_binding {
            if self.resolve_hf(hub, descriptor)?.as_ref() != Some(&record.path) {
                return Err(RootError("model-artifact-invalid"));
            }
        }
        if root
            .root
            .join(format!("{}.partial", record.filename))
            .exists()
            || root
                .root
                .join(format!("{}.extracting", record.filename))
                .exists()
        {
            return Err(RootError("model-artifact-unavailable"));
        }
        let mut handles = Vec::new();
        for ancestor in record.path.ancestors().skip(1) {
            handles.push(open_plain(ancestor, true, true)?);
        }
        let file = open_plain(&record.path, false, true)?;
        if identity(&file)? != record.key.fingerprint {
            return Err(RootError("model-artifact-invalid"));
        }
        // Selected roots never use an ambient cache, even with the same model ID.
        if !record.legacy && !record.path.starts_with(&root.root) {
            return Err(RootError("model-root-invalid"));
        }
        handles.push(file);
        Ok(ArtifactUse {
            key: record.key.clone(),
            path: record.path.clone(),
            _handles: handles,
        })
    }
    pub fn delete_owned(&self, record: &ArtifactRecord) -> RootResult<()> {
        #[cfg(windows)]
        {
            windows_io::delete_exact(&record.path, &record.key.fingerprint)
        }
        #[cfg(not(windows))]
        {
            let file = open_plain(&record.path, false, true)?;
            if identity(&file)? != record.key.fingerprint {
                return Err(RootError("model-root-stale"));
            }
            fs::remove_file(&record.path).map_err(|_| RootError("model-root-readonly"))
        }
    }
}

#[cfg(windows)]
mod windows_io {
    use super::*;
    use std::{
        ffi::c_void,
        os::windows::{
            ffi::OsStrExt,
            io::{AsRawHandle, FromRawHandle},
        },
    };
    #[repr(C)]
    struct FileTime {
        low: u32,
        high: u32,
    }
    #[repr(C)]
    struct Info {
        attrs: u32,
        created: FileTime,
        accessed: FileTime,
        written: FileTime,
        volume: u32,
        size_high: u32,
        size_low: u32,
        links: u32,
        index_high: u32,
        index_low: u32,
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GetFileInformationByHandle(handle: *mut c_void, info: *mut Info) -> i32;
        fn GetDriveTypeW(root: *const u16) -> u32;
        fn CreateFileW(
            path: *const u16,
            access: u32,
            share: u32,
            security: *const c_void,
            creation: u32,
            flags: u32,
            template: *mut c_void,
        ) -> *mut c_void;
        fn SetFileInformationByHandle(
            handle: *mut c_void,
            class: i32,
            info: *const c_void,
            size: u32,
        ) -> i32;
    }
    pub(super) fn identity(file: &File) -> RootResult<(u64, u64)> {
        let mut info = std::mem::MaybeUninit::<Info>::uninit();
        if unsafe { GetFileInformationByHandle(file.as_raw_handle(), info.as_mut_ptr()) } == 0 {
            return Err(RootError("model-root-unavailable"));
        }
        let info = unsafe { info.assume_init() };
        Ok((
            u64::from(info.volume),
            (u64::from(info.index_high) << 32) | u64::from(info.index_low),
        ))
    }
    pub(super) fn local_drive(letter: u8) -> bool {
        let root = [u16::from(letter), 58, 92, 0];
        matches!(unsafe { GetDriveTypeW(root.as_ptr()) }, 2 | 3 | 5 | 6)
    }
    pub(super) fn writable(root: &Path) -> bool {
        let path: Vec<u16> = root.as_os_str().encode_wide().chain(Some(0)).collect();
        let handle = unsafe {
            CreateFileW(
                path.as_ptr(),
                0x2 | 0x4,
                7,
                std::ptr::null(),
                3,
                0x0200_0000 | 0x0020_0000,
                std::ptr::null_mut(),
            )
        };
        if handle as isize == -1 {
            return false;
        }
        drop(unsafe { File::from_raw_handle(handle) });
        true
    }
    pub(super) fn delete_exact(path: &Path, expected: &Fingerprint) -> RootResult<()> {
        use std::os::windows::fs::OpenOptionsExt;
        plain_ancestors(path)?;
        let file = OpenOptions::new()
            .access_mode(0x8000_0000 | 0x0001_0000)
            .share_mode(3)
            .custom_flags(0x0020_0000)
            .open(path)
            .map_err(|_| RootError("model-root-busy"))?;
        if super::identity(&file)? != *expected {
            return Err(RootError("model-root-stale"));
        }
        let disposition = 1_i32;
        if unsafe {
            SetFileInformationByHandle(
                file.as_raw_handle(),
                4,
                &disposition as *const i32 as *const c_void,
                4,
            )
        } == 0
        {
            return Err(RootError("model-root-readonly"));
        }
        Ok(())
    }
}
