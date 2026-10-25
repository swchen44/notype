//! Production structured-diagnostics ownership and serialization. Collector
//! state never covers I/O. Export commit and Clear share an independent gate.

use super::{Diagnostics, Outcome, Phase, Report, ReportContext};
use crate::capture_startup::{CaptureEvent, CaptureObserver, CollectionToken};
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};

/// Production filesystem boundary. Implementations expose only fixed errors;
/// no settings document, raw log or destination path enters collector state.
pub trait DiagnosticStorage: Send + Sync {
    /// None denotes an absent preference; errors fail closed, never default ON.
    fn load_preference(&self) -> Result<Option<bool>, &'static str>;
    /// Explicitly persist only the structured-diagnostics preference.
    fn save_preference(&self, enabled: bool) -> Result<(), &'static str>;
    /// Atomically save an already sanitized immutable report to a chosen path.
    fn write_report(&self, destination: &Path, text: &str) -> Result<(), &'static str>;
}

/// Acknowledged control state. Revision orders preference changes; epoch orders
/// Clear. Unsaved effective OFF is deliberately distinct from saved OFF.
#[derive(Clone)]
pub struct Status {
    /// Whether initial preference loading has completed, including read failure.
    pub initialized: bool,
    /// Whether new structured stage records are currently retained.
    pub effective_enabled: bool,
    /// Last known saved/default preference; None means unreadable/unknown.
    pub saved_preference: Option<bool>,
    /// uninitialized, default, saved, pending, unsaved or load_failed.
    pub persistence: &'static str,
    /// Requested value while pending/unsaved, without optimistic enablement.
    pub desired_enabled: Option<bool>,
    /// Monotonic preference-operation revision, independent of stage records.
    pub revision: u64,
    /// Clear generation; unchanged by preference I/O or stage recording.
    pub epoch: u64,
}

/// Private preview preparation token: captures a bounded event snapshot before
/// context gathering. A Clear invalidates its epoch before publication.
pub struct PreviewTicket {
    epoch: u64,
    events: Diagnostics,
}
/// The immutable current preview and its Clear generation.
pub struct Preview {
    /// The exact sanitized snapshot; no caller-provided replacement text.
    pub report: Report,
    /// Generation used to suppress late frontend responses after Clear.
    pub epoch: u64,
}
/// Export identity captured before the native dialog. Final commit revalidates
/// it while holding the separate commit gate, before touching the destination.
pub struct ExportTicket {
    epoch: u64,
    id: String,
}
impl ExportTicket {
    /// Generation of the requested snapshot, also used for cancellation replies.
    pub fn epoch(&self) -> u64 {
        self.epoch
    }
    /// Opaque preview identity used in the proposed file name, never a path.
    pub fn id(&self) -> &str {
        &self.id
    }
}

struct State {
    status: Status,
    events: Diagnostics,
    preview: Option<Report>,
    api_registry: super::api::ApiRegistry,
}
/// Process-owned service. Starts uninitialized/effectively OFF. All I/O is
/// outside state; export_commit → state is the only nested lock order.
pub struct DiagnosticService {
    state: Mutex<State>,
    export_commit: Mutex<()>,
    preference_update: Mutex<()>,
    storage: OnceLock<Arc<dyn DiagnosticStorage>>,
}
impl Default for DiagnosticService {
    fn default() -> Self {
        Self::new()
    }
}
impl DiagnosticService {
    /// Constructs a disabled empty collector without reading any file.
    pub fn new() -> Self {
        Self {
            state: Mutex::new(State {
                status: Status {
                    initialized: false,
                    effective_enabled: false,
                    saved_preference: None,
                    persistence: "uninitialized",
                    desired_enabled: None,
                    revision: 0,
                    epoch: 0,
                },
                events: Diagnostics::new(),
                preview: None,
                api_registry: super::api::ApiRegistry::new(),
            }),
            export_commit: Mutex::new(()),
            preference_update: Mutex::new(()),
            storage: OnceLock::new(),
        }
    }
    /// Loads the dedicated preference before startup records. This operation is
    /// serialized with toggles; a missing value alone defaults ON.
    pub fn initialize(&self, storage: Arc<dyn DiagnosticStorage>) -> Status {
        let _serial = self
            .preference_update
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if self.status().initialized {
            return self.status();
        }
        let loaded = storage.load_preference(); // Never under the collector lock.
        let _ = self.storage.set(storage);
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.status.initialized = true;
        match loaded {
            Ok(value) => {
                state.status.effective_enabled = value.unwrap_or(true);
                state.status.saved_preference = Some(value.unwrap_or(true));
                state.status.persistence = if value.is_some() { "saved" } else { "default" };
            }
            Err(_) => {
                state.status.effective_enabled = false;
                state.status.persistence = "load_failed";
            }
        }
        state.status.clone()
    }
    /// Returns current effective/persistence state without reading storage.
    pub fn status(&self) -> Status {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .status
            .clone()
    }
    /// Serializes a preference change. OFF stops immediately before saving; ON
    /// enables only after success. Any write failure leaves effective OFF.
    pub fn set_enabled(&self, enabled: bool) -> Status {
        let _serial = self
            .preference_update
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let Some(storage) = self.storage.get() else {
            return self.status();
        };
        {
            let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            state.status.revision = state.status.revision.saturating_add(1);
            state.status.effective_enabled = false;
            state.status.desired_enabled = Some(enabled);
            state.status.persistence = "pending";
        }
        let result = storage.save_preference(enabled);
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if result.is_ok() {
            state.status.effective_enabled = enabled;
            state.status.saved_preference = Some(enabled);
            state.status.persistence = "saved";
            state.status.desired_enabled = None;
        } else {
            state.status.persistence = "unsaved";
        }
        state.status.clone()
    }
    /// Retains typed stage data only while initialized and effectively ON.
    pub fn record(&self, phase: Phase, outcome: Outcome, duration: Option<u64>) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if state.status.initialized && state.status.effective_enabled {
            state.events.record(phase, outcome, duration);
        }
    }
    /// Immutable collection generation for one capture. Preference revision
    /// prevents an OFF/ON cycle from resurrecting an older in-flight trace.
    pub fn capture_token(&self) -> Option<CollectionToken> {
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        (state.status.initialized && state.status.effective_enabled).then_some(CollectionToken {
            epoch: state.status.epoch,
            revision: state.status.revision,
        })
    }
    /// Generic collector generation, identical to the existing capture fence.
    pub fn collection_token(&self) -> Option<CollectionToken> {
        self.capture_token()
    }
    /// Maps a workflow-owned ID only in the private bounded adapter registry.
    /// Effective ON, generation and allocation are checked under this one lock.
    pub fn api_scope(
        &self,
        source: Option<&str>,
        connection: Option<u32>,
    ) -> Option<super::api::ScopeBinding> {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if !state.status.initialized || !state.status.effective_enabled {
            return None;
        }
        let token = CollectionToken {
            epoch: state.status.epoch,
            revision: state.status.revision,
        };
        Some(state.api_registry.scope(token, source, connection))
    }
    /// Reserves an ordinal only under effective ON and the matching origin token.
    pub fn begin_api(
        &self,
        scope: Option<super::api::ScopeBinding>,
        revision: Option<u64>,
    ) -> Option<super::api::TraceIdentity> {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if !state.status.initialized || !state.status.effective_enabled {
            return None;
        }
        let token = CollectionToken {
            epoch: state.status.epoch,
            revision: state.status.revision,
        };
        if scope.is_some_and(|scope| scope.token != token) {
            return None;
        }
        state.api_registry.call(token, scope, revision)
    }
    /// Atomically validates generation and inserts into the shared ring, with no I/O.
    pub fn record_api(&self, event: super::api::ApiEvent) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if state.status.initialized
            && state.status.effective_enabled
            && event.identity.binding.token.epoch == state.status.epoch
            && event.identity.binding.token.revision == state.status.revision
        {
            state.events.record_api(event);
        }
    }
    /// Epoch and effective preference are checked in the same collector lock
    /// as insertion. No capture/target identity or user content crosses here.
    pub fn record_capture(&self, event: CaptureEvent) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if state.status.initialized
            && state.status.effective_enabled
            && event.token.epoch == state.status.epoch
            && event.token.revision == state.status.revision
        {
            state.events.record_capture(event);
        }
    }
    /// Captures a bounded preparation token before context I/O, with no cache
    /// mutation. OFF still permits reviewing/exporting existing records.
    pub fn begin_preview(&self) -> Result<PreviewTicket, &'static str> {
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if !state.status.initialized {
            return Err("diagnostic-not-ready");
        }
        Ok(PreviewTicket {
            epoch: state.status.epoch,
            events: state.events.clone(),
        })
    }
    /// Sanitizes/render outside state, then publishes only if Clear has not
    /// invalidated the preparation token. Replaces the prior cached identity.
    pub fn publish_preview(
        &self,
        ticket: PreviewTicket,
        context: ReportContext<'_>,
    ) -> Result<Preview, &'static str> {
        let report = ticket.events.report(context);
        let _commit = self.export_commit.lock().unwrap_or_else(|e| e.into_inner());
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if ticket.epoch != state.status.epoch {
            return Err("diagnostic-preview-expired");
        }
        state.preview = Some(report.clone());
        Ok(Preview {
            report,
            epoch: ticket.epoch,
        })
    }
    /// Validates an identity before a dialog; no report clone or path retained.
    pub fn begin_export(&self, id: &str) -> Result<ExportTicket, &'static str> {
        let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if !state
            .preview
            .as_ref()
            .is_some_and(|report| report.id() == id)
        {
            return Err("diagnostic-preview-expired");
        }
        Ok(ExportTicket {
            epoch: state.status.epoch,
            id: id.into(),
        })
    }
    /// Revalidates after the dialog before any destination I/O. Only the commit
    /// gate spans write/cleanup; Clear cannot acknowledge while this can commit.
    pub fn commit_export(
        &self,
        ticket: &ExportTicket,
        destination: &Path,
    ) -> Result<(), &'static str> {
        let _commit = self.export_commit.lock().unwrap_or_else(|e| e.into_inner());
        let report = {
            let state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            if ticket.epoch != state.status.epoch {
                return Err("diagnostic-preview-expired");
            }
            state
                .preview
                .as_ref()
                .filter(|report| report.id() == ticket.id)
                .cloned()
                .ok_or("diagnostic-preview-expired")?
        };
        if destination.extension().and_then(|s| s.to_str()) != Some("txt") {
            return Err("diagnostic-extension-invalid");
        }
        self.storage
            .get()
            .ok_or("diagnostic-not-ready")?
            .write_report(destination, report.text())
            .map_err(|_| "diagnostic-save-failed")
    }
    /// Linearizes Clear against preview publication/final export. Resets only
    /// structured in-memory data; preference state and manual files are kept.
    pub fn clear(&self) -> u64 {
        let _commit = self.export_commit.lock().unwrap_or_else(|e| e.into_inner());
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        state.status.epoch = state.status.epoch.saturating_add(1);
        state.events = Diagnostics::new();
        state.preview = None;
        state.status.epoch
    }
}
static GLOBAL: OnceLock<DiagnosticService> = OnceLock::new();
/// The shared app-run service, lazily created OFF without performing I/O.
pub fn global() -> &'static DiagnosticService {
    GLOBAL.get_or_init(DiagnosticService::new)
}

impl CaptureObserver for DiagnosticService {
    fn accepts(&self, token: CollectionToken) -> bool {
        self.capture_token() == Some(token)
    }
    fn record(&self, event: CaptureEvent) {
        self.record_capture(event);
    }
}
