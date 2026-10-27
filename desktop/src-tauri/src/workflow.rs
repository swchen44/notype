//! Desktop session orchestration; platform capture and Tauri events stay outside.
use crate::diagnostics::{record as diagnose, Outcome, Phase};
use notype_core::observations::{ModelIdentity, RecordingObservation, SessionObservations};
use notype_core::{
    CancellationToken, CoreError, DeliveryAdapter, DeliveryFuture, DeliveryOutcome,
    DeliveryRequest, RecognitionMode, Scenario, SessionEntry, SessionOptions, SessionService,
    SessionStatus,
};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
};

#[path = "faithful_provider.rs"]
mod faithful_provider;
pub use faithful_provider::experiment;
pub use faithful_provider::{
    probe_processor as probe_text_processor, processor as text_processor,
    processor_with_trace as text_processor_with_trace, RuntimeCredential, ScenarioPrompts,
    TextAuthentication, TextConfiguration, TextProviderProfile, TextTransport, TranslationSettings,
    TranslationTerm,
};
#[path = "recognition.rs"]
pub mod recognition;

struct UnavailableDelivery;
impl DeliveryAdapter for UnavailableDelivery {
    fn deliver(&self, _: DeliveryRequest) -> DeliveryFuture {
        Box::pin(async { DeliveryOutcome::Unavailable })
    }
}

/// Owns one current capture/processing generation and persistent session history.
/// Capture order is begin → source context → processing snapshot → audio → finish.
pub struct DesktopWorkflow {
    sessions: SessionService,
    generation: AtomicU64,
    recording: Mutex<Option<Recording>>,
    notify: Arc<dyn Fn() + Send + Sync>,
    delivery: Arc<dyn DeliveryAdapter>,
    lifecycle: Mutex<u64>,
}

/// Ownership of one recording's short platform operations. Never hold this across decoding.
#[derive(Clone)]
pub struct GenerationToken {
    workflow: Arc<DesktopWorkflow>,
    generation: u64,
}
impl GenerationToken {
    /// Process-local capture identity for platform events; never a history row ID.
    pub fn capture_id(&self) -> u64 {
        self.generation
    }

    /// Run a short resource/UI operation only while this is the latest generation.
    /// The closure must not begin/cancel another generation or call `run` recursively.
    pub fn run<T>(&self, operation: impl FnOnce() -> T) -> Option<T> {
        let owner = self.workflow.lifecycle.lock().ok()?;
        (*owner == self.generation).then(operation)
    }
    /// Play the readiness cue, then mute only if this capture is still ready.
    /// `is_ready` and `apply_mute` are short platform operations and must not reenter
    /// ownership. Feedback may block; cancellation must remain available during it.
    pub fn readiness_feedback(
        &self,
        is_ready: impl Fn() -> bool,
        play_feedback: impl FnOnce(),
        apply_mute: impl FnOnce(),
    ) {
        if self.run(&is_ready) != Some(true) {
            return;
        }
        // Playback is deliberately outside ownership: custom WAV files can be
        // arbitrarily long. A late cue must not block Esc or stop capture.
        play_feedback();
        self.run(|| {
            if is_ready() {
                apply_mute();
            }
        });
    }
    /// Atomically cancel this generation; a late decoder cannot cancel its successor.
    pub fn cancel(&self) -> Result<(), CoreError> {
        let owner = self
            .workflow
            .lifecycle
            .lock()
            .map_err(|_| CoreError::Storage)?;
        if *owner == self.generation {
            self.workflow.cancel_recording_locked()?;
        }
        Ok(())
    }
    /// A drop guard for the same generation-owned cleanup used by native stop tasks.
    pub fn cleanup(&self, operation: impl FnOnce() + Send + 'static) -> GenerationCleanup {
        GenerationCleanup {
            owner: self.clone(),
            operation: Some(Box::new(operation)),
        }
    }
}

/// Runs its owned cleanup at most once when the processing future exits or is dropped.
pub struct GenerationCleanup {
    owner: GenerationToken,
    operation: Option<Box<dyn FnOnce() + Send>>,
}
impl Drop for GenerationCleanup {
    fn drop(&mut self) {
        if let Some(operation) = self.operation.take() {
            self.owner.run(operation);
        }
    }
}

struct Recording {
    generation: u64,
    options: SessionOptions,
    cancellation: CancellationToken,
    entry: Option<String>,
    processing_snapshot: Option<(TextConfiguration, Vec<String>)>,
    context_captured: bool,
    observations: SessionObservations,
    stopped_at: Option<std::time::Instant>,
    profile: recognition::RecognitionProfile,
}

struct RecordingLease<'a> {
    workflow: &'a DesktopWorkflow,
    generation: u64,
}
impl Drop for RecordingLease<'_> {
    fn drop(&mut self) {
        if let Ok(mut recording) = self.workflow.recording.lock() {
            if recording
                .as_ref()
                .is_some_and(|recording| recording.generation == self.generation)
            {
                recording.take();
            }
        }
        (self.workflow.notify)();
    }
}

impl DesktopWorkflow {
    /// Open orchestration over an existing store; notification callbacks must not reenter ownership mutations.
    pub fn new(sessions: SessionService, notify: Arc<dyn Fn() + Send + Sync>) -> Self {
        Self {
            sessions,
            generation: AtomicU64::new(0),
            recording: Mutex::new(None),
            notify,
            delivery: Arc::new(UnavailableDelivery),
            lifecycle: Mutex::new(0),
        }
    }
    /// Composition selects a guarded platform adapter; portable callers stay unavailable.
    pub fn with_delivery(mut self, delivery: Arc<dyn DeliveryAdapter>) -> Self {
        self.delivery = delivery;
        self
    }
    /// Read/search saved text independently of any active recording or audio buffer.
    pub fn sessions(&self) -> &SessionService {
        &self.sessions
    }
    /// Capture a platform-operation owner; stale owners are safe no-ops after a restart.
    pub fn token(self: &Arc<Self>, generation: u64) -> GenerationToken {
        GenerationToken {
            workflow: Arc::clone(self),
            generation,
        }
    }
    /// Begin with a default local recognition profile, cancelling a previous capture.
    /// The opaque original target belongs only to this new generation.
    pub fn begin(
        &self,
        recognition_mode: RecognitionMode,
        original_target: Option<String>,
    ) -> Result<u64, CoreError> {
        self.begin_with_profile(
            recognition::RecognitionProfile::new(recognition_mode, vec![]),
            original_target,
        )
    }
    /// Atomically replace the active capture and snapshot its recognition mode/terms.
    /// Old decoders may finish, but their generation owners cannot mutate this successor.
    pub fn begin_with_profile(
        &self,
        profile: recognition::RecognitionProfile,
        original_target: Option<String>,
    ) -> Result<u64, CoreError> {
        let mut owner = self.lifecycle.lock().map_err(|_| CoreError::Storage)?;
        let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        self.cancel_recording_locked()?;
        *self.recording.lock().map_err(|_| CoreError::Storage)? = Some(Recording {
            generation,
            options: SessionOptions {
                recognition_mode: profile.mode,
                original_target,
                ..SessionOptions::default()
            },
            cancellation: CancellationToken::new(),
            entry: None,
            processing_snapshot: None,
            context_captured: false,
            observations: SessionObservations::default(),
            stopped_at: None,
            profile,
        });
        *owner = generation;
        Ok(generation)
    }
    /// Snapshot only the configured model identifier before capture, never credentials.
    pub fn capture_requested_model(&self, generation: u64, model: String) -> Result<(), CoreError> {
        let mut recording = self.recording.lock().map_err(|_| CoreError::Storage)?;
        let recording = recording
            .as_mut()
            .filter(|r| {
                r.generation == generation && r.entry.is_none() && !r.cancellation.is_cancelled()
            })
            .ok_or(CoreError::InvalidTransition)?;
        recording.observations.requested_asr_model = (!model.trim().is_empty()).then_some(model);
        recording.observations.validate()
    }
    /// Stamp accepted stop before feedback/UI/buffer/scheduling, independent of diagnostics.
    pub fn accept_stop(
        &self,
        generation: u64,
        stopped_at: std::time::Instant,
    ) -> Result<(), CoreError> {
        let mut recording = self.recording.lock().map_err(|_| CoreError::Storage)?;
        let recording = recording
            .as_mut()
            .filter(|r| {
                r.generation == generation && r.entry.is_none() && !r.cancellation.is_cancelled()
            })
            .ok_or(CoreError::InvalidTransition)?;
        if recording.stopped_at.is_some() {
            return Err(CoreError::InvalidTransition);
        }
        recording.stopped_at = Some(stopped_at);
        Ok(())
    }
    /// Attach accepted native frames and the identity returned by the actual producer lease.
    pub fn capture_recognition_observation(
        &self,
        generation: u64,
        recording_audio: Option<RecordingObservation>,
        model: Option<ModelIdentity>,
    ) -> Result<(), CoreError> {
        let mut recording = self.recording.lock().map_err(|_| CoreError::Storage)?;
        let recording = recording
            .as_mut()
            .filter(|r| {
                r.generation == generation && r.entry.is_none() && !r.cancellation.is_cancelled()
            })
            .ok_or(CoreError::InvalidTransition)?;
        recording.observations.recording = recording_audio;
        recording.observations.actual_asr_model = model;
        recording.observations.validate()
    }
    /// Capture scenario, prompts, translation settings and glossary once, before audio starts.
    /// Source context must be captured first; later preference changes affect only retry/new captures.
    pub fn capture_processing_snapshot(
        &self,
        generation: u64,
        scenario: Scenario,
        configuration: TextConfiguration,
        glossary: Vec<String>,
    ) -> Result<(), CoreError> {
        let mut recording = self.recording.lock().map_err(|_| CoreError::Storage)?;
        let recording = recording
            .as_mut()
            .filter(|recording| {
                recording.generation == generation
                    && !recording.cancellation.is_cancelled()
                    && recording.entry.is_none()
            })
            .ok_or(CoreError::InvalidTransition)?;
        if recording.processing_snapshot.is_some() {
            return Err(CoreError::Busy);
        }
        recording.options.scenario = scenario;
        recording.options.translation_target = if scenario == Scenario::Translation {
            Some(configuration.translation.target_language.clone())
        } else {
            None
        };
        recording.processing_snapshot = Some((configuration, glossary));
        Ok(())
    }
    /// The exact profile captured at recording start, never the current preferences.
    pub fn recognition_profile(
        &self,
        generation: u64,
    ) -> Result<recognition::RecognitionProfile, CoreError> {
        self.recording
            .lock()
            .map_err(|_| CoreError::Storage)?
            .as_ref()
            .filter(|recording| {
                recording.generation == generation && !recording.cancellation.is_cancelled()
            })
            .map(|recording| recording.profile.clone())
            .ok_or(CoreError::InvalidTransition)
    }
    /// Return the active capture/processing generation, not a platform cleanup permission.
    /// Use `token` for atomic resource ownership instead of checking this then mutating globals.
    pub fn active_generation(&self) -> Result<Option<u64>, CoreError> {
        Ok(self
            .recording
            .lock()
            .map_err(|_| CoreError::Storage)?
            .as_ref()
            .map(|recording| recording.generation))
    }
    /// Cancel the current workflow and invalidate late recognition/HTTP results.
    /// Platform callers should use `cancel_active_with` to serialize their short resource cleanup.
    pub fn cancel_active(&self) -> Result<(), CoreError> {
        self.cancel_active_with(|_| {}).map(|_| ())
    }
    /// Cancel and clean up the current capture atomically with respect to a new begin.
    /// The callback must be short and must not recursively mutate workflow ownership.
    pub fn cancel_active_with(&self, cleanup: impl FnOnce(u64)) -> Result<u64, CoreError> {
        let owner = self.lifecycle.lock().map_err(|_| CoreError::Storage)?;
        let cancelled = self.cancel_recording_locked();
        cleanup(*owner);
        cancelled.map(|_| *owner)
    }
    fn cancel_recording_locked(&self) -> Result<(), CoreError> {
        let active = self
            .recording
            .lock()
            .map_err(|_| CoreError::Storage)?
            .take();
        if let Some(recording) = active {
            recording.cancellation.cancel();
            if let Some(id) = recording.entry {
                self.sessions.cancel(&id)?;
                (self.notify)();
            }
        }
        Ok(())
    }
    /// Save the original with its captured recognition profile before processing any text.
    /// A captured processing snapshot wins over fallback arguments. Cancellation/old generations
    /// cannot create a successor's entry; successful unreviewed output uses the guarded delivery adapter.
    pub async fn finish(
        &self,
        generation: u64,
        transcript: &str,
        configuration: TextConfiguration,
        glossary: Vec<String>,
    ) -> Result<SessionEntry, CoreError> {
        let (entry, cancellation, processing_snapshot, stopped_at) = {
            let mut recording = self.recording.lock().map_err(|_| CoreError::Storage)?;
            let recording = recording
                .as_mut()
                .filter(|recording| {
                    recording.generation == generation && !recording.cancellation.is_cancelled()
                })
                .ok_or(CoreError::InvalidTransition)?;
            if recording.entry.is_some() {
                return Err(CoreError::Busy);
            }
            let normalized = recording.profile.normalize(transcript);
            let save_started = std::time::Instant::now();
            diagnose(Phase::HistorySave, Outcome::Started, None);
            let saved = match normalized {
                Ok(original) => self.sessions.create_observed(
                    &original,
                    recording.options.clone(),
                    recording.observations.clone(),
                ),
                Err(_) => self.sessions.create_recognition_failure_observed(
                    transcript,
                    recording.options.clone(),
                    recording.observations.clone(),
                ),
            };
            diagnose(
                Phase::HistorySave,
                if saved.is_ok() {
                    Outcome::Ok
                } else {
                    Outcome::Failed
                },
                Some(save_started.elapsed().as_millis().min(u64::MAX as u128) as u64),
            );
            let entry = saved?;
            let entry = if let Some(stopped_at) = recording.stopped_at {
                self.sessions
                    .observe_original_ready(&entry.id, stopped_at)?
            } else {
                entry
            };
            recording.entry = Some(entry.id.clone());
            (
                entry,
                recording.cancellation.clone(),
                recording.processing_snapshot.clone(),
                recording.stopped_at,
            )
        };
        let (configuration, glossary) = processing_snapshot.unwrap_or((configuration, glossary));
        let _lease = RecordingLease {
            workflow: self,
            generation,
        };
        (self.notify)();
        if entry.error.as_deref() == Some("recognition-script-failed") {
            return Ok(entry);
        }
        if !configuration.processing_enabled {
            let raw = self.sessions.original_for_delivery(&entry.id)?;
            return tokio::select! {
                biased;
                _=cancellation.cancelled()=>self.sessions.cancel(&raw.id),
                delivered=self.deliver(&raw.id)=>delivered,
            };
        }
        let processing_started = std::time::Instant::now();
        diagnose(Phase::TextProcessing, Outcome::Started, None);
        let result = tokio::select! {
            biased;
            _ = cancellation.cancelled() => self.sessions.cancel(&entry.id),
            result = self.sessions.process_with_stop(&entry.id,glossary,faithful_provider::scoped_processor(configuration.clone(),crate::diagnostics::service::global().api_connection_scope(Some(&entry.id),configuration.connection_id.as_deref())),configuration.translation.back_translation,stopped_at) => result,
        };
        diagnose(
            Phase::TextProcessing,
            match &result {
                Ok(entry) if entry.status == SessionStatus::Cancelled => Outcome::Cancelled,
                Ok(entry) if entry.status == SessionStatus::Failed || entry.error.is_some() => {
                    Outcome::Failed
                }
                Ok(_) => Outcome::Ok,
                Err(_) => Outcome::Failed,
            },
            Some(
                processing_started
                    .elapsed()
                    .as_millis()
                    .min(u64::MAX as u128) as u64,
            ),
        );
        let result = result?;
        let result = if result.status == SessionStatus::Failed && !cancellation.is_cancelled() {
            self.sessions.original_for_delivery(&result.id)?
        } else { result };
        (self.notify)();
        if result.status == SessionStatus::Pending
            && !result.review_required
        {
            tokio::select! {
                biased;
                _=cancellation.cancelled()=>self.sessions.cancel(&result.id),
                delivered=self.deliver(&result.id)=>delivered,
            }
        } else {
            Ok(result)
        }
    }
    /// Select saved raw text for later copy/delivery without running a provider or changing focus.
    pub fn choose_original(&self, id: &str) -> Result<SessionEntry, CoreError> {
        let entry = self.sessions.choose_original(id)?;
        (self.notify)();
        Ok(entry)
    }
    /// ASR adapters call this when text exists but cannot satisfy the local profile.
    /// The raw text remains readable; no provider or delivery is invoked.
    pub fn finish_recognition_failure(
        &self,
        generation: u64,
        raw: &str,
    ) -> Result<SessionEntry, CoreError> {
        let entry = {
            let mut recording = self.recording.lock().map_err(|_| CoreError::Storage)?;
            let recording = recording
                .as_mut()
                .filter(|recording| {
                    recording.generation == generation && !recording.cancellation.is_cancelled()
                })
                .ok_or(CoreError::InvalidTransition)?;
            if recording.entry.is_some() {
                return Err(CoreError::Busy);
            }
            let entry = self
                .sessions
                .create_recognition_failure(raw, recording.options.clone())?;
            recording.entry = Some(entry.id.clone());
            entry
        };
        let _lease = RecordingLease {
            workflow: self,
            generation,
        };
        (self.notify)();
        Ok(entry)
    }
    /// Cancel a persisted session; in-flight late replies are ignored by the shared core.
    pub fn cancel(&self, id: &str) -> Result<SessionEntry, CoreError> {
        let entry = self.sessions.cancel(id)?;
        (self.notify)();
        Ok(entry)
    }
    /// Explicitly delete saved text. This never records or retrieves audio.
    pub fn delete(&self, id: &str) -> Result<(), CoreError> {
        self.sessions.delete(id)?;
        (self.notify)();
        Ok(())
    }
    /// Process saved raw text with the supplied current settings, without ASR or microphone use.
    /// Script-conversion failures require a new recognition result; optional review still gates delivery.
    pub async fn retry(
        &self,
        id: &str,
        configuration: TextConfiguration,
        glossary: Vec<String>,
    ) -> Result<SessionEntry, CoreError> {
        if self.sessions.get(id)?.error.as_deref() == Some("recognition-script-failed") {
            return Err(CoreError::InvalidTransition);
        }
        (self.notify)();
        let entry = self
            .sessions
            .process_with_review(
                id,
                glossary,
                faithful_provider::scoped_processor(
                    configuration.clone(),
                    crate::diagnostics::service::global()
                        .api_connection_scope(Some(id), configuration.connection_id.as_deref()),
                ),
                configuration.translation.back_translation,
            )
            .await?;
        (self.notify)();
        if entry.status == SessionStatus::Pending
            && !entry.review_required
            && entry.selection == notype_core::TextSelection::Polished
        {
            self.deliver(id).await
        } else {
            Ok(entry)
        }
    }
    /// Attempt guarded, deduplicated delivery to the captured target; never focus an arbitrary window.
    pub async fn deliver(&self, id: &str) -> Result<SessionEntry, CoreError> {
        let started = std::time::Instant::now();
        diagnose(Phase::Delivery, Outcome::Started, None);
        let result = self.sessions.deliver(id, Arc::clone(&self.delivery)).await;
        diagnose(
            Phase::Delivery,
            match &result {
                Ok(entry) if entry.status == SessionStatus::Delivered => Outcome::Ok,
                Ok(_) => Outcome::Unavailable,
                Err(_) => Outcome::Failed,
            },
            Some(started.elapsed().as_millis().min(u64::MAX as u128) as u64),
        );
        let entry = result?;
        (self.notify)();
        Ok(entry)
    }
    /// Approve exactly the displayed revision without delivering or changing foreground focus.
    pub fn confirm_review(&self, id: &str, revision: i64) -> Result<SessionEntry, CoreError> {
        let entry = self.sessions.confirm_review(id, revision)?;
        (self.notify)();
        Ok(entry)
    }
    /// Capture application-only metadata once, before the processing snapshot or audio.
    /// This cannot add websites, page/input contents or overwrite a previous capture.
    pub fn capture_source_context(
        &self,
        generation: u64,
        context: notype_core::SourceContext,
    ) -> Result<(), CoreError> {
        if !context.is_valid() {
            return Err(CoreError::InvalidInput);
        }
        let mut recording = self.recording.lock().map_err(|_| CoreError::Storage)?;
        let recording = recording
            .as_mut()
            .filter(|r| {
                r.generation == generation && !r.cancellation.is_cancelled() && r.entry.is_none()
            })
            .ok_or(CoreError::InvalidTransition)?;
        if recording.context_captured || recording.processing_snapshot.is_some() {
            return Err(CoreError::Busy);
        }
        recording.options.source_context = context;
        recording.context_captured = true;
        Ok(())
    }
}
