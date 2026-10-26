//! Sealed transport to actual SessionService observer composition. Helper SQLite
//! locks never cover core callbacks, another SQLite store, cancellation or await.
use super::{
    journal::{Failure, Journal, Outcome, Record, Snapshot, State},
    policy::{schedule, Provider, Purpose, MAX_CALLS},
};
use crate::workflow::experiment::{AdverseLatch, ExperimentProcessor};
use notype_core::{
    observations::{
        ApiCallDescriptor, ApiCallObserver, ApiEndpointClass, ApiOutcome, ApiProviderProfile,
        ApiPurpose, ApiReceipt,
    },
    BackTranslationRequest, CancellationToken, GenerationFuture, TextFailure, TextFuture,
    TextProcessor, TextRequest,
};
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

/// Owns one helper journal and a read-only path to its independent core ledger.
/// Construction never admits a call, discovers a key or changes core schema.
pub struct Coordinator {
    journal: Mutex<Journal>,
    core_path: PathBuf,
    failed: AtomicBool,
    latch: AdverseLatch,
    last_openrouter: Mutex<Option<std::time::Instant>>,
}
impl Coordinator {
    /// Bind already created owned stores. A retained journal remains report-only.
    pub fn new(journal: Journal, core_path: PathBuf, latch: AdverseLatch) -> Arc<Self> {
        Arc::new(Self {
            journal: Mutex::new(journal),
            core_path,
            failed: AtomicBool::new(false),
            latch,
            last_openrouter: Mutex::new(None),
        })
    }
    /// Safe reservation/source snapshot, never provider/body/settings data.
    pub fn snapshot(&self) -> Result<Snapshot, Failure> {
        self.journal
            .lock()
            .map(|j| j.snapshot())
            .map_err(|_| Failure::Storage)
    }
    /// Monotonic stop. Callback persistence failures remain latched in memory.
    pub fn halt(&self) -> Result<(), Failure> {
        self.failed.store(true, Ordering::SeqCst);
        self.journal.lock().map_err(|_| Failure::Storage)?.halt()
    }
    fn eligible(&self) -> Result<(), Failure> {
        if self.failed.load(Ordering::SeqCst) || self.latch.observed().is_some() {
            return Err(Failure::Halted);
        }
        let journal = self.journal.lock().map_err(|_| Failure::Storage)?;
        if journal.report_only() {
            return Err(Failure::ReportOnly);
        }
        if journal.snapshot().halted || !journal.snapshot().armed {
            return Err(Failure::Halted);
        }
        Ok(())
    }
    async fn wait_pace(&self, slot: u8) -> Result<(), TextFailure> {
        if !(9..=12).contains(&slot) {
            return Ok(());
        }
        let last = *self
            .last_openrouter
            .lock()
            .map_err(|_| TextFailure::HistoryStorage)?;
        if let Some(last) = last {
            let wait = std::time::Duration::from_secs(10).saturating_sub(last.elapsed());
            if !wait.is_zero() {
                tokio::time::sleep(wait).await;
            }
        }
        Ok(())
    }
    /// Verify actual durable core receipts after its transaction and commit a
    /// checked helper checkpoint before automatic reverse or any later slot.
    pub fn reconcile(&self) -> Result<(), Failure> {
        let snapshot = self.snapshot()?;
        let pending = snapshot
            .records
            .iter()
            .find(|r| r.state == State::Finished)
            .cloned();
        if let Some(record) = pending {
            verify_core(&self.core_path, &record)?;
            self.journal
                .lock()
                .map_err(|_| Failure::Storage)?
                .reconciled(snapshot.revision, record.slot, &record.call)?;
        }
        Ok(())
    }
    fn before_begin(&self, slot: u8) -> Result<(), Failure> {
        self.eligible()?;
        self.reconcile()?;
        self.eligible()?;
        let state = self.snapshot()?;
        if slot == 0
            || usize::from(slot) > MAX_CALLS
            || state.records[usize::from(slot - 1)].state != State::NotRun
            || state.records[..usize::from(slot - 1)]
                .iter()
                .any(|r| r.state != State::Reconciled)
        {
            return Err(Failure::Schedule);
        }
        Ok(())
    }
    fn mutate(
        &self,
        operation: impl FnOnce(&mut Journal) -> Result<(), Failure>,
    ) -> Result<(), Failure> {
        let mut journal = self.journal.lock().map_err(|_| Failure::Storage)?;
        operation(&mut journal)
    }
    /// Persist only the actual current scheduled cancellation request. Core
    /// cancellation follows outside this lock and before the timer settles.
    pub(super) fn request_cancel(&self) -> Result<String, Failure> {
        let record = self.snapshot()?.records[6].clone();
        self.mutate(|journal| journal.request_cancellation(&record.call))?;
        Ok(record.history)
    }
}
fn expected(slot: u8, descriptor: &ApiCallDescriptor) -> bool {
    let Some(spec) = schedule().get(usize::from(slot.saturating_sub(1))).copied() else {
        return false;
    };
    let deepseek = spec.provider == Provider::DeepSeek;
    slot > 0
        && slot <= 12
        && descriptor.profile
            == if deepseek {
                ApiProviderProfile::DeepSeek
            } else {
                ApiProviderProfile::OpenRouter
            }
        && descriptor.requested_model.as_deref() == Some(spec.model())
        && descriptor.requested_effort.as_deref() == if deepseek { Some("none") } else { None }
        && descriptor.thinking == Some(false)
        && descriptor.max_output_tokens == Some(512)
        && descriptor.non_streaming
        && descriptor.plain_text_only
        && (1..=8192).contains(&descriptor.wire_bytes)
        && (1..=4).contains(&descriptor.message_count)
        && descriptor.purpose
            == if spec.purpose == Purpose::Reverse {
                ApiPurpose::BackTranslation
            } else {
                ApiPurpose::Generation
            }
        && matches!(
            descriptor.endpoint_class,
            ApiEndpointClass::DeepSeekHttps | ApiEndpointClass::Other | ApiEndpointClass::Loopback
        )
}
fn verify_core(path: &Path, record: &Record) -> Result<(), Failure> {
    super::journal::plain(path)?;
    let connection =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|_| Failure::Storage)?;
    let (history,revision,descriptor,send,outcome,receipt,finished):(String,u64,String,String,String,Option<String>,Option<String>)=connection.query_row("SELECT session_id,revision,descriptor,send_state,outcome,receipt,finished_at FROM api_calls WHERE id=?1",[&record.call],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?,row.get(5)?,row.get(6)?))).map_err(|_|Failure::Storage)?;
    if history != record.history
        || revision != record.processing_revision
        || !matches!(send.as_str(), "possibly_sent" | "response_received")
        || finished.is_none()
    {
        return Err(Failure::Storage);
    }
    let descriptor: ApiCallDescriptor =
        serde_json::from_str(&descriptor).map_err(|_| Failure::Storage)?;
    if !expected(record.slot, &descriptor) {
        return Err(Failure::Storage);
    }
    match record.outcome {
        Some(Outcome::Success) => {
            let receipt: ApiReceipt =
                serde_json::from_str(receipt.as_deref().ok_or(Failure::Storage)?)
                    .map_err(|_| Failure::Storage)?;
            if outcome != "success" || !crate::workflow::experiment::coherent_receipt(&receipt) {
                return Err(Failure::Storage);
            }
        }
        Some(Outcome::ExpectedCancellation)
            if matches!(outcome.as_str(), "cancelled" | "interrupted")
                && record.slot == 7
                && record.cancellation_requested => {}
        Some(Outcome::Failed) => return Err(Failure::Halted),
        _ => return Err(Failure::Storage),
    }
    Ok(())
}
/// Private wrapper admits only the sealed provider and real observed calls.
/// It cannot be built from an arbitrary TextProcessor or unobserved sender.
pub(super) struct BudgetProcessor {
    inner: Arc<ExperimentProcessor>,
    coordinator: Arc<Coordinator>,
    history: String,
    slot: u8,
    sent: tokio::sync::watch::Sender<bool>,
}
impl BudgetProcessor {
    pub(super) fn new(
        inner: ExperimentProcessor,
        coordinator: Arc<Coordinator>,
        history: String,
        slot: u8,
    ) -> Arc<Self> {
        let (sent, _) = tokio::sync::watch::channel(false);
        Arc::new(Self {
            inner: Arc::new(inner),
            coordinator,
            history,
            slot,
            sent,
        })
    }
    pub(super) async fn wait_until_sent(&self) -> Result<(), Failure> {
        let mut receiver = self.sent.subscribe();
        loop {
            if *receiver.borrow_and_update() {
                return Ok(());
            }
            receiver.changed().await.map_err(|_| Failure::Storage)?;
        }
    }
    fn observer(&self, core: Arc<dyn ApiCallObserver>, slot: u8) -> Arc<dyn ApiCallObserver> {
        Arc::new(Observer {
            inner: self.inner.clone(),
            coordinator: self.coordinator.clone(),
            history: self.history.clone(),
            slot,
            core,
            call: Mutex::new(None),
            sent: self.sent.clone(),
        })
    }
}
impl TextProcessor for BudgetProcessor {
    fn polish(&self, _: TextRequest, _: CancellationToken) -> TextFuture {
        Box::pin(async { Err(TextFailure::HistoryStorage) })
    }
    fn generate_observed(
        &self,
        request: TextRequest,
        cancel: CancellationToken,
        core: Arc<dyn ApiCallObserver>,
    ) -> GenerationFuture {
        let inner = self.inner.clone();
        let coordinator = self.coordinator.clone();
        let slot = self.slot;
        let observer = self.observer(core, slot);
        Box::pin(async move {
            coordinator.wait_pace(slot).await?;
            inner.generate_observed(request, cancel, observer).await
        })
    }
    fn back_translate_observed(
        &self,
        request: BackTranslationRequest,
        cancel: CancellationToken,
        core: Arc<dyn ApiCallObserver>,
    ) -> TextFuture {
        if !matches!(self.slot, 5 | 10) {
            return Box::pin(async { Err(TextFailure::HistoryStorage) });
        }
        let inner = self.inner.clone();
        let coordinator = self.coordinator.clone();
        let slot = self.slot + 1;
        let observer = self.observer(core, slot);
        Box::pin(async move {
            coordinator.wait_pace(slot).await?;
            inner
                .back_translate_observed(request, cancel, observer)
                .await
        })
    }
}
#[derive(Clone)]
struct Call {
    id: String,
    receipt_valid: Option<bool>,
}
struct Observer {
    inner: Arc<ExperimentProcessor>,
    coordinator: Arc<Coordinator>,
    history: String,
    slot: u8,
    core: Arc<dyn ApiCallObserver>,
    call: Mutex<Option<Call>>,
    sent: tokio::sync::watch::Sender<bool>,
}
impl Observer {
    fn fail(&self) -> TextFailure {
        self.inner.observer_failed();
        let _ = self.coordinator.halt();
        TextFailure::HistoryStorage
    }
    fn current(&self, id: &str) -> Result<Call, TextFailure> {
        self.call
            .lock()
            .map_err(|_| TextFailure::HistoryStorage)?
            .as_ref()
            .filter(|call| call.id == id)
            .cloned()
            .ok_or(TextFailure::HistoryStorage)
    }
}
impl ApiCallObserver for Observer {
    fn processing_revision(&self) -> Option<u64> {
        self.core.processing_revision()
    }
    fn begin(&self, descriptor: ApiCallDescriptor) -> Result<String, TextFailure> {
        if self.inner.adverse().is_some() || !expected(self.slot, &descriptor) {
            return Err(self.fail());
        }
        self.coordinator
            .before_begin(self.slot)
            .map_err(|_| self.fail())?;
        let revision = self
            .core
            .processing_revision()
            .filter(|revision| *revision > 0)
            .ok_or_else(|| self.fail())?;
        let id = self.core.begin(descriptor).map_err(|_| self.fail())?;
        if self
            .coordinator
            .mutate(|journal| journal.link(self.slot, &self.history, &id, revision))
            .is_err()
        {
            self.core.finish(&id, ApiOutcome::Failed, 0);
            return Err(self.fail());
        }
        let assigned = {
            let mut holder = self.call.lock().map_err(|_| TextFailure::HistoryStorage)?;
            if holder.is_some() {
                false
            } else {
                *holder = Some(Call {
                    id: id.clone(),
                    receipt_valid: None,
                });
                true
            }
        };
        if !assigned {
            return Err(self.fail());
        }
        Ok(id)
    }
    fn mark_sent(&self, id: &str) -> Result<(), TextFailure> {
        self.current(id).map_err(|_| self.fail())?;
        self.coordinator.eligible().map_err(|_| self.fail())?;
        self.coordinator
            .mutate(|journal| journal.reserve(self.slot, id))
            .map_err(|_| self.fail())?;
        self.core.mark_sent(id).map_err(|_| self.fail())?;
        if (9..=12).contains(&self.slot) {
            *self
                .coordinator
                .last_openrouter
                .lock()
                .map_err(|_| self.fail())? = Some(std::time::Instant::now());
        }
        self.sent.send_replace(true);
        Ok(())
    }
    fn response(&self, id: &str, receipt: ApiReceipt) {
        self.core.response(id, receipt.clone());
        let valid = crate::workflow::experiment::coherent_receipt(&receipt);
        let assigned = {
            match self.call.lock() {
                Ok(mut holder) => match holder.as_mut().filter(|call| call.id == id) {
                    Some(call) => {
                        call.receipt_valid = Some(valid);
                        true
                    }
                    None => false,
                },
                Err(_) => false,
            }
        };
        if !assigned || !valid {
            self.fail();
        }
    }
    fn finish(&self, id: &str, outcome: ApiOutcome, elapsed: u64) {
        self.core.finish(id, outcome, elapsed);
        let call = match self.current(id) {
            Ok(call) => call,
            Err(_) => {
                self.fail();
                return;
            }
        };
        let record = match self.coordinator.snapshot() {
            Ok(snapshot) => snapshot.records[usize::from(self.slot - 1)].clone(),
            Err(_) => {
                self.fail();
                return;
            }
        };
        let classified = if outcome == ApiOutcome::Success
            && call.receipt_valid == Some(true)
            && self.inner.adverse().is_none()
        {
            Outcome::Success
        } else if self.slot == 7
            && record.cancellation_requested
            && self.inner.adverse().is_none()
            && matches!(outcome, ApiOutcome::Cancelled | ApiOutcome::Interrupted)
        {
            Outcome::ExpectedCancellation
        } else {
            Outcome::Failed
        };
        if self
            .coordinator
            .mutate(|journal| journal.finish(self.slot, id, classified))
            .is_err()
        {
            self.fail();
        }
    }
    fn unavailable(&self) {
        self.core.unavailable();
        self.fail();
    }
}
