//! Model-root authority and generation-fenced work. Selected model preferences
//! remain owned by AppSettings; this service never selects, moves or repairs.
use super::model_root_activity::{ActivityGate, ActivityLease};
use super::model_roots_files::{ArtifactUse, Filesystem, RootUse};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RootError(pub &'static str);
impl fmt::Display for RootError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for RootError {}
pub type RootResult<T> = Result<T, RootError>;

pub trait RootStorage: Send + Sync {
    fn accepts_root(&self, _root: &Path) -> bool {
        true
    }
    fn load(&self) -> RootResult<Option<Vec<u8>>>;
    fn commit(&self, bytes: &[u8]) -> RootResult<()>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RootKind {
    Default,
    Selected,
    Bundled,
}
#[derive(Clone, Debug)]
pub struct RootSnapshot {
    pub generation: u64,
    pub revision: u64,
    pub root: PathBuf,
    pub kind: RootKind,
    pub legacy_cache: Option<PathBuf>,
    pub valid_path: bool,
}
#[derive(Clone)]
pub struct RootActivity {
    pub root: Arc<RootSnapshot>,
    pub directory: Option<Arc<RootUse>>,
    mutation: Option<Arc<MutationOwner>>,
    pub lease: ActivityLease,
}
struct MutationOwner {
    model_id: String,
    active: Arc<Mutex<BTreeSet<String>>>,
}
impl Drop for MutationOwner {
    fn drop(&mut self) {
        self.active
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&self.model_id);
    }
}

#[derive(Clone, Debug)]
pub struct ArtifactDescriptor {
    pub model_id: String,
    pub filename: String,
    pub size: Option<u64>,
    pub sha256: Option<String>,
    pub repo: Option<String>,
    pub revision: Option<String>,
    pub directory: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArtifactStatus {
    Verified,
    CustomUnverified,
    Invalid,
    Incomplete,
    Unavailable,
}
impl ArtifactStatus {
    pub fn loadable(self) -> bool {
        matches!(self, Self::Verified | Self::CustomUnverified)
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fingerprint {
    pub volume: u64,
    pub file: u64,
    pub size: u64,
    pub modified: u128,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactKey {
    pub generation: u64,
    pub model_id: String,
    pub content_identity: String,
    pub fingerprint: Fingerprint,
}
#[derive(Clone, Debug)]
pub struct ArtifactRecord {
    pub key: ArtifactKey,
    pub filename: String,
    pub path: PathBuf,
    pub status: ArtifactStatus,
    pub legacy: bool,
    pub custom: bool,
    pub owned: bool,
    pub present: bool,
    pub hf_binding: Option<(PathBuf, ArtifactDescriptor)>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RootStatus {
    Ready,
    NoModels,
    Unavailable,
    Invalid,
}
#[derive(Clone, Debug)]
pub struct Projection {
    pub root_identity: Option<Fingerprint>,
    pub artifacts: BTreeMap<String, ArtifactRecord>,
    pub writable: bool,
    pub status: RootStatus,
}
impl Projection {
    pub fn unavailable() -> Self {
        Self {
            root_identity: None,
            artifacts: BTreeMap::new(),
            writable: false,
            status: RootStatus::Unavailable,
        }
    }
    pub fn count(&self) -> usize {
        self.artifacts
            .values()
            .filter(|a| a.status.loadable())
            .count()
    }
}
#[derive(Clone)]
pub struct Candidate {
    pub id: String,
    pub request_id: String,
    pub generation: u64,
    pub revision: u64,
    pub snapshot: Arc<RootSnapshot>,
    pub projection: Arc<Projection>,
}
/// An accepted scan owns its generation and admission before a worker is queued.
pub struct ScanWork {
    activity: RootActivity,
    serial: u64,
    cancel: Arc<AtomicBool>,
    folder: PathBuf,
    request: String,
}
impl ScanWork {
    pub fn generation(&self) -> u64 {
        self.activity.root.generation
    }
}

struct Scan {
    serial: u64,
    request_id: String,
    cancel: Arc<AtomicBool>,
}
struct State {
    root: Arc<RootSnapshot>,
    projection: Arc<Projection>,
    candidate: Option<Candidate>,
    scan: Option<Scan>,
    refresh: Option<Scan>,
    serial: u64,
    initialized: bool,
    owned: BTreeSet<(String, String)>,
}

pub struct RootService {
    gate: ActivityGate,
    state: Mutex<State>,
    storage: Arc<dyn RootStorage>,
    files: Arc<Filesystem>,
    catalog: Vec<ArtifactDescriptor>,
    mutations: Arc<Mutex<BTreeSet<String>>>,
}
impl RootService {
    /// Opens only the dedicated root preference. Invalid/missing selected roots
    /// are unavailable and retained; construction never repairs them.
    pub fn open(
        default: PathBuf,
        storage: Arc<dyn RootStorage>,
        files: Arc<Filesystem>,
        catalog: Vec<ArtifactDescriptor>,
        legacy_cache: Option<PathBuf>,
    ) -> Self {
        Self::open_mode(default, storage, files, catalog, legacy_cache, true)
    }
    pub fn open_unscanned(
        default: PathBuf,
        storage: Arc<dyn RootStorage>,
        files: Arc<Filesystem>,
        catalog: Vec<ArtifactDescriptor>,
        legacy_cache: Option<PathBuf>,
    ) -> Self {
        Self::open_mode(default, storage, files, catalog, legacy_cache, false)
    }
    fn open_mode(
        default: PathBuf,
        storage: Arc<dyn RootStorage>,
        files: Arc<Filesystem>,
        catalog: Vec<ArtifactDescriptor>,
        legacy_cache: Option<PathBuf>,
        scan_initial: bool,
    ) -> Self {
        let loaded = storage.load().and_then(|bytes| match bytes {
            Some(bytes) => super::model_roots_files::decode(&bytes).map(Some),
            None => Ok(None),
        });
        let (root, revision, kind, initialized, mut valid) = match loaded {
            Ok(Some((root, revision))) => (root, revision, RootKind::Selected, true, true),
            Ok(None) => (default, 0, RootKind::Default, false, true),
            Err(_) => (default, 0, RootKind::Selected, true, false),
        };
        if !initialized {
            let _ = files.create_default(&root);
        }
        let valid_path = valid && files.valid_input(&root);
        let root = match files.canonical_root(&root) {
            Ok(canonical) => canonical,
            Err(_) => {
                valid = false;
                root
            }
        };
        if !storage.accepts_root(&root) {
            valid = false;
        }
        let snapshot = Arc::new(RootSnapshot {
            generation: 1,
            revision,
            root,
            legacy_cache: if kind == RootKind::Default {
                legacy_cache
            } else {
                None
            },
            kind,
            valid_path,
        });
        let projection = if valid && scan_initial {
            files
                .scan(&snapshot, &catalog, &AtomicBool::new(false), &|_, _| {})
                .unwrap_or_else(|_| Projection::unavailable())
        } else {
            Projection::unavailable()
        };
        Self {
            gate: ActivityGate::new(),
            state: Mutex::new(State {
                root: snapshot,
                projection: Arc::new(projection),
                candidate: None,
                scan: None,
                refresh: None,
                serial: 0,
                initialized,
                owned: BTreeSet::new(),
            }),
            storage,
            files,
            catalog,
            mutations: Arc::new(Mutex::new(BTreeSet::new())),
        }
    }
    pub fn snapshot(&self) -> Arc<RootSnapshot> {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .root
            .clone()
    }
    pub fn projection(&self) -> Arc<Projection> {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .projection
            .clone()
    }
    pub fn artifacts(&self) -> BTreeMap<String, ArtifactRecord> {
        self.projection().artifacts.clone()
    }
    pub fn busy(&self) -> bool {
        self.gate.busy()
    }
    pub fn admit(&self) -> RootResult<RootActivity> {
        let lease = self
            .gate
            .enter()
            .map_err(|_| RootError("model-root-busy"))?;
        Ok(RootActivity {
            root: self.snapshot(),
            lease,
            directory: None,
            mutation: None,
        })
    }
    /// Queue admission can reserve without disk I/O. Child workers clone this
    /// owner instead of releasing it when cancellation is merely requested.
    pub fn reserve(&self) -> RootResult<ActivityLease> {
        self.gate.enter().map_err(|_| RootError("model-root-busy"))
    }
    fn start_scan(
        &self,
        request: &str,
        validation: bool,
    ) -> RootResult<(RootActivity, u64, Arc<AtomicBool>)> {
        if request.is_empty()
            || request.len() > 80
            || !request
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        {
            return Err(RootError("model-root-invalid"));
        }
        let activity = self.admit()?;
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let old = if validation {
            state.scan.take()
        } else {
            state.refresh.take()
        };
        if let Some(old) = old {
            old.cancel.store(true, Ordering::Release);
        }
        if validation {
            state.candidate = None;
        }
        state.serial = state
            .serial
            .checked_add(1)
            .filter(|n| *n < 9_007_199_254_740_991)
            .ok_or(RootError("model-root-limit"))?;
        let serial = state.serial;
        let cancel = Arc::new(AtomicBool::new(false));
        let scan = Some(Scan {
            serial,
            request_id: request.to_owned(),
            cancel: cancel.clone(),
        });
        if validation {
            state.scan = scan;
        } else {
            state.refresh = scan;
        }
        Ok((activity, serial, cancel))
    }
    pub fn cancel_scan(&self, request: &str) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        for scan in [&state.scan, &state.refresh].into_iter().flatten() {
            if scan.request_id == request {
                scan.cancel.store(true, Ordering::Release);
            }
        }
        if state
            .candidate
            .as_ref()
            .is_some_and(|c| c.request_id == request)
        {
            state.candidate = None;
        }
    }
    pub fn validate(
        &self,
        folder: &Path,
        request: &str,
        progress: &(dyn Fn(u32, u32) + Send + Sync),
    ) -> RootResult<Candidate> {
        self.run_validation(self.prepare_validation(folder, request)?, progress)
    }
    pub fn prepare_validation(&self, folder: &Path, request: &str) -> RootResult<ScanWork> {
        if folder.as_os_str().len() > 4096 {
            return Err(RootError("model-root-invalid"));
        }
        let (activity, serial, cancel) = self.start_scan(request, true)?;
        Ok(ScanWork {
            activity,
            serial,
            cancel,
            folder: folder.to_owned(),
            request: request.to_owned(),
        })
    }
    pub fn prepare_rescan(&self, request: &str) -> RootResult<ScanWork> {
        let (activity, serial, cancel) = self.start_scan(request, false)?;
        let folder = activity.root.root.clone();
        Ok(ScanWork {
            activity,
            serial,
            cancel,
            folder,
            request: request.to_owned(),
        })
    }
    pub fn run_validation(
        &self,
        work: ScanWork,
        progress: &(dyn Fn(u32, u32) + Send + Sync),
    ) -> RootResult<Candidate> {
        let ScanWork {
            activity,
            serial,
            cancel,
            folder,
            request,
        } = work;
        if cancel.load(Ordering::Acquire) {
            return Err(RootError("model-root-cancelled"));
        }
        let root = self.files.canonical_root(&folder)?;
        if !self.storage.accepts_root(&root) {
            return Err(RootError("model-root-invalid"));
        }
        let snapshot = Arc::new(RootSnapshot {
            root,
            kind: RootKind::Selected,
            legacy_cache: None,
            generation: activity.root.generation,
            revision: activity.root.revision,
            valid_path: true,
        });
        let projection = Arc::new(
            self.files
                .scan(&snapshot, &self.catalog, &cancel, progress)?,
        );
        let candidate = Candidate {
            id: format!("{}-{serial}", snapshot.generation),
            request_id: request.to_owned(),
            generation: snapshot.generation,
            revision: snapshot.revision,
            snapshot,
            projection,
        };
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if cancel.load(Ordering::Acquire) {
            return Err(RootError("model-root-cancelled"));
        }
        if state.root.generation != candidate.generation
            || !state.scan.as_ref().is_some_and(|s| s.serial == serial)
        {
            return Err(RootError("model-root-stale"));
        }
        state.scan = None;
        state.candidate = Some(candidate.clone());
        Ok(candidate)
    }
    /// The callback is a prepared, infallible memory-only cached-engine
    /// invalidation. All fallible validation/storage occurs before publication.
    pub fn apply(
        &self,
        candidate_id: &str,
        revision: u64,
        invalidate: impl FnOnce(),
    ) -> RootResult<Arc<RootSnapshot>> {
        let _transition = self
            .gate
            .transition()
            .map_err(|_| RootError("model-root-busy"))?;
        let candidate = {
            let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            if state.root.revision != revision {
                return Err(RootError("model-root-stale"));
            }
            state
                .candidate
                .as_ref()
                .filter(|c| {
                    c.id == candidate_id
                        && c.revision == revision
                        && c.generation == state.root.generation
                })
                .cloned()
                .ok_or(RootError("model-root-stale"))?
        };
        if candidate.projection.status == RootStatus::Invalid {
            return Err(RootError("model-root-invalid"));
        }
        let _verified_handles = self
            .files
            .hold_projection(&candidate.snapshot, &candidate.projection)?;
        let revision = revision
            .checked_add(1)
            .filter(|n| *n < 9_007_199_254_740_991)
            .ok_or(RootError("model-root-limit"))?;
        let generation = candidate
            .generation
            .checked_add(1)
            .filter(|n| *n < 9_007_199_254_740_991)
            .ok_or(RootError("model-root-limit"))?;
        let root = Arc::new(RootSnapshot {
            generation,
            revision,
            ..(*candidate.snapshot).clone()
        });
        let mut projection = (*candidate.projection).clone();
        for record in projection.artifacts.values_mut() {
            record.key.generation = generation;
        }
        let projection = Arc::new(projection);
        let bytes = super::model_roots_files::encode(&root.root, revision)?;
        self.storage.commit(&bytes)?;
        {
            let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            state.root = root.clone();
            state.projection = projection;
            state.candidate = None;
            state.scan = None;
            state.refresh = None;
            state.initialized = true;
            state.owned.clear();
            invalidate();
        }
        Ok(root)
    }
    pub fn rescan(
        &self,
        request: &str,
        progress: &(dyn Fn(u32, u32) + Send + Sync),
    ) -> RootResult<()> {
        self.run_rescan(self.prepare_rescan(request)?, progress)
    }
    pub fn run_rescan(
        &self,
        work: ScanWork,
        progress: &(dyn Fn(u32, u32) + Send + Sync),
    ) -> RootResult<()> {
        let ScanWork {
            activity,
            serial,
            cancel,
            ..
        } = work;
        if cancel.load(Ordering::Acquire) {
            return Err(RootError("model-root-cancelled"));
        }
        let result = self
            .files
            .scan(&activity.root, &self.catalog, &cancel, progress);
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if cancel.load(Ordering::Acquire) {
            return Err(RootError("model-root-cancelled"));
        }
        if state.root.generation != activity.root.generation
            || !state.refresh.as_ref().is_some_and(|s| s.serial == serial)
        {
            return Err(RootError("model-root-stale"));
        }
        state.refresh = None;
        let mut projection = result.unwrap_or_else(|_| Projection::unavailable());
        for record in projection.artifacts.values_mut() {
            record.owned = state.owned.contains(&(
                record.key.model_id.clone(),
                record.key.content_identity.clone(),
            ));
        }
        state.projection = Arc::new(projection);
        Ok(())
    }
    pub fn current(&self, key: &ArtifactKey) -> bool {
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.root.generation == key.generation
            && state
                .projection
                .artifacts
                .get(&key.model_id)
                .is_some_and(|r| r.status.loadable() && r.key == *key)
    }
    /// Physical source is revalidated before reusing an idle cached engine.
    /// This read-only check never reconciles selection or repairs files.
    pub fn usable(&self, key: &ArtifactKey) -> bool {
        let Ok(activity) = self.admit() else {
            return false;
        };
        self.current(key) && self.acquire(&key.model_id, &activity).is_ok()
    }
    pub fn with_current<T>(&self, key: &ArtifactKey, publish: impl FnOnce() -> T) -> Option<T> {
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if state.root.generation == key.generation
            && state
                .projection
                .artifacts
                .get(&key.model_id)
                .is_some_and(|r| r.status.loadable() && r.key == *key)
        {
            Some(publish())
        } else {
            None
        }
    }
    pub fn acquire(&self, id: &str, activity: &RootActivity) -> RootResult<Arc<ArtifactUse>> {
        let (record, projection) = {
            let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            if state.root.generation != activity.root.generation {
                return Err(RootError("model-root-stale"));
            }
            (
                state
                    .projection
                    .artifacts
                    .get(id)
                    .cloned()
                    .ok_or(RootError("model-artifact-unavailable"))?,
                state.projection.clone(),
            )
        };
        if !record.status.loadable() {
            return Err(RootError("model-artifact-unavailable"));
        }
        self.files
            .check_root_identity(&activity.root, &projection)?;
        self.files.acquire(&activity.root, &record).map(Arc::new)
    }
    pub fn writable(&self, activity: &RootActivity) -> RootResult<PathBuf> {
        if self.snapshot().generation != activity.root.generation {
            return Err(RootError("model-root-stale"));
        }
        self.files
            .check_root_identity(&activity.root, &self.projection())?;
        if !self.files.writable(&activity.root.root)? {
            return Err(RootError("model-root-readonly"));
        }
        Ok(activity.root.root.clone())
    }
    pub fn location(&self, activity: &RootActivity) -> RootResult<PathBuf> {
        if self.snapshot().generation != activity.root.generation {
            return Err(RootError("model-root-stale"));
        }
        self.files
            .check_root_identity(&activity.root, &self.projection())?;
        Ok(activity.root.root.clone())
    }
    pub fn admit_write(&self) -> RootResult<RootActivity> {
        let mut activity = self.admit()?;
        self.writable(&activity)?;
        activity.directory = Some(Arc::new(
            self.files.hold_root(&activity.root, &self.projection())?,
        ));
        Ok(activity)
    }
    pub fn admit_mutation(&self, model_id: &str) -> RootResult<RootActivity> {
        let mut activity = self.admit_write()?;
        if !self
            .mutations
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(model_id.to_owned())
        {
            return Err(RootError("model-root-busy"));
        }
        activity.mutation = Some(Arc::new(MutationOwner {
            model_id: model_id.to_owned(),
            active: self.mutations.clone(),
        }));
        Ok(activity)
    }
    pub fn mark_unavailable(&self, id: &str) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let mut projection = (*state.projection).clone();
        if let Some(record) = projection.artifacts.get_mut(id) {
            record.status = ArtifactStatus::Unavailable;
        }
        projection.status = if projection.count() == 0 {
            RootStatus::NoModels
        } else {
            RootStatus::Ready
        };
        state.projection = Arc::new(projection);
    }
    pub fn delete_owned(&self, record: &ArtifactRecord, activity: &RootActivity) -> RootResult<()> {
        self.writable(activity)?;
        if !self.current(&record.key) || !record.owned || record.legacy || record.custom {
            return Err(RootError("model-root-readonly"));
        }
        self.files.delete_owned(record)?;
        self.mark_unavailable(&record.key.model_id);
        Ok(())
    }
    pub fn snapshot_projection(&self) -> (Arc<RootSnapshot>, Arc<Projection>) {
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        (state.root.clone(), state.projection.clone())
    }
    pub fn mark_owned(&self, id: &str) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(record) = state
            .projection
            .artifacts
            .get(id)
            .filter(|r| r.status == ArtifactStatus::Verified && !r.legacy && !r.custom)
        {
            let identity = record.key.content_identity.clone();
            state.owned.insert((id.to_owned(), identity));
            let mut projection = (*state.projection).clone();
            if let Some(record) = projection.artifacts.get_mut(id) {
                record.owned = true;
            }
            state.projection = Arc::new(projection);
        }
    }
    /// P29 default only: explicit or corrupt initialized preferences always win.
    pub fn initialize_default_if_unset(&self, folder: &Path) -> RootResult<bool> {
        if self
            .state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .initialized
        {
            return Ok(false);
        }
        let candidate = self.validate(folder, "bundle_default", &|_, _| {})?;
        if self
            .state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .initialized
        {
            return Ok(false);
        }
        self.apply(&candidate.id, candidate.revision, || {})?;
        Ok(true)
    }
}
