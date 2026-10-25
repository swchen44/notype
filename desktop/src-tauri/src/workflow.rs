//! Desktop session orchestration; platform capture and Tauri events stay outside.
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
pub use faithful_provider::{
    ScenarioPrompts, TextAuthentication, TextConfiguration, TextTransport, TranslationSettings,
    TranslationTerm,
};

struct UnavailableDelivery;
impl DeliveryAdapter for UnavailableDelivery {
    fn deliver(&self, _: DeliveryRequest) -> DeliveryFuture {
        Box::pin(async { DeliveryOutcome::Unavailable })
    }
}

pub struct DesktopWorkflow {
    sessions: SessionService,
    generation: AtomicU64,
    recording: Mutex<Option<Recording>>,
    notify: Arc<dyn Fn() + Send + Sync>,
}

struct Recording {
    generation: u64,
    options: SessionOptions,
    cancellation: CancellationToken,
    entry: Option<String>,
    processing_snapshot: Option<(TextConfiguration, Vec<String>)>,
    context_captured: bool,
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
    pub fn new(sessions: SessionService, notify: Arc<dyn Fn() + Send + Sync>) -> Self {
        Self {
            sessions,
            generation: AtomicU64::new(0),
            recording: Mutex::new(None),
            notify,
        }
    }
    pub fn sessions(&self) -> &SessionService {
        &self.sessions
    }
    pub fn begin(
        &self,
        recognition_mode: RecognitionMode,
        original_target: Option<String>,
    ) -> Result<u64, CoreError> {
        let generation = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        self.cancel_active()?;
        *self.recording.lock().map_err(|_| CoreError::Storage)? = Some(Recording {
            generation,
            options: SessionOptions {
                recognition_mode,
                original_target,
                ..SessionOptions::default()
            },
            cancellation: CancellationToken::new(),
            entry: None,
            processing_snapshot: None,
            context_captured: false,
        });
        Ok(generation)
    }
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
    pub fn active_generation(&self) -> Result<Option<u64>, CoreError> {
        Ok(self
            .recording
            .lock()
            .map_err(|_| CoreError::Storage)?
            .as_ref()
            .map(|recording| recording.generation))
    }
    pub fn cancel_active(&self) -> Result<(), CoreError> {
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
    pub async fn finish(
        &self,
        generation: u64,
        transcript: &str,
        configuration: TextConfiguration,
        glossary: Vec<String>,
    ) -> Result<SessionEntry, CoreError> {
        let (entry, cancellation, processing_snapshot) = {
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
                .create(transcript, recording.options.clone())?;
            recording.entry = Some(entry.id.clone());
            (
                entry,
                recording.cancellation.clone(),
                recording.processing_snapshot.clone(),
            )
        };
        let (configuration, glossary) = processing_snapshot.unwrap_or((configuration, glossary));
        let _lease = RecordingLease {
            workflow: self,
            generation,
        };
        (self.notify)();
        let result = tokio::select! {
            biased;
            _ = cancellation.cancelled() => self.sessions.cancel(&entry.id),
            result = self.sessions.process_with_review(&entry.id,glossary,faithful_provider::processor(configuration.clone()),configuration.translation.back_translation) => result,
        };
        let result = result?;
        (self.notify)();
        if result.status == SessionStatus::Pending
            && !result.review_required
            && result.selection == notype_core::TextSelection::Polished
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
    pub fn choose_original(&self, id: &str) -> Result<SessionEntry, CoreError> {
        let entry = self.sessions.choose_original(id)?;
        (self.notify)();
        Ok(entry)
    }
    pub fn cancel(&self, id: &str) -> Result<SessionEntry, CoreError> {
        let entry = self.sessions.cancel(id)?;
        (self.notify)();
        Ok(entry)
    }
    pub fn delete(&self, id: &str) -> Result<(), CoreError> {
        self.sessions.delete(id)?;
        (self.notify)();
        Ok(())
    }
    pub async fn retry(
        &self,
        id: &str,
        configuration: TextConfiguration,
        glossary: Vec<String>,
    ) -> Result<SessionEntry, CoreError> {
        (self.notify)();
        let entry = self
            .sessions
            .process_with_review(
                id,
                glossary,
                faithful_provider::processor(configuration.clone()),
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
    pub async fn deliver(&self, id: &str) -> Result<SessionEntry, CoreError> {
        let entry = self
            .sessions
            .deliver(id, Arc::new(UnavailableDelivery))
            .await?;
        (self.notify)();
        Ok(entry)
    }
    pub fn confirm_review(&self, id: &str, revision: i64) -> Result<SessionEntry, CoreError> {
        let entry = self.sessions.confirm_review(id, revision)?;
        (self.notify)();
        Ok(entry)
    }
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
