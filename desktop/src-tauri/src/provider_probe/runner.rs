//! Fixed serial schedule through actual SessionService. Slot7 always settles
//! both its processing task and owned timer before reconciliation or retry8.
use super::{
    bridge::{BudgetProcessor, Coordinator, ResourceGate},
    configuration::{self, LiveConfigurations},
    journal::{Failure, Identity, Journal, Snapshot},
    policy::{quality, schedule, Purpose, MAX_CALLS},
};
use crate::{
    diagnostics::api::TraceContext,
    workflow::experiment::{AdverseLatch, OwnedLoopback},
};
use notype_core::{SessionEntry, SessionOptions, SessionService};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

enum Source {
    Owned(OwnedLoopback),
    OwnedCredentials(OwnedLoopback, Box<LiveConfigurations>),
    Live(Box<LiveConfigurations>),
}
/// Private one-shot preparation capability. Creation retains an unarmed marker
/// before public evidence/key lookup; failure never deletes or reuses it.
pub(super) struct PreparedRoot(PathBuf);
impl PreparedRoot {
    pub(super) fn create(root: &Path) -> Result<Self, Failure> {
        super::journal::plain(root)?;
        std::fs::create_dir(root).map_err(|_| Failure::ReportOnly)?;
        use std::io::Write;
        let mut marker = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(root.join("preparation.lock"))
            .map_err(|_| Failure::Storage)?;
        marker
            .write_all(b"notype-owned-P25-live-v4-preparation\n")
            .and_then(|_| marker.sync_all())
            .map_err(|_| Failure::Storage)?;
        Ok(Self(root.to_path_buf()))
    }
    pub(super) fn path(&self) -> &Path {
        &self.0
    }
}
/// Owned cancellation-delay port used by real smoke and synthetic race cases.
/// It cannot alter a live policy or authorize a request/reservation.
pub trait CancellationTimer: Send + Sync {
    /// Wait after both durable commits. Drop never claims intentional or remote cancellation.
    fn wait(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>;
}
/// Fixed production25ms delay with no runtime argument/environment override.
pub struct RuntimeCancellationTimer;
impl CancellationTimer for RuntimeCancellationTimer {
    fn wait(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
        Box::pin(tokio::time::sleep(Duration::from_millis(25)))
    }
}
/// An actual settled processing step. Reverse and retry keep their real histories.
pub struct Step {
    /// First immutable slot represented by this public processing step.
    pub first_slot: u8,
    /// Last slot, including an actual automatic reverse if scheduled.
    pub last_slot: u8,
    /// Actual persisted core history and observations, not a fabricated result.
    pub history: SessionEntry,
}
/// One fresh fixed experiment. No replacement/extra slot or arbitrary processor
/// can be injected. All snapshots remain backend memory only.
pub struct ProbeRunner {
    sessions: SessionService,
    coordinator: Arc<Coordinator>,
    source: Source,
    latch: AdverseLatch,
    tracing: TraceContext,
    next: u8,
    retry_history: Option<String>,
    timer: Arc<dyn CancellationTimer>,
}
impl ProbeRunner {
    /// Build only a fresh owned-loopback fixture for check/tests. The socket
    /// capability cannot authorize HTTPS or load any real home credential.
    pub fn for_owned_fixture(
        root: &Path,
        endpoint: OwnedLoopback,
        identity: Identity,
        tracing: TraceContext,
        timer: Arc<dyn CancellationTimer>,
        resource: Arc<dyn ResourceGate>,
    ) -> Result<Self, Failure> {
        Self::fresh(
            root,
            Source::Owned(endpoint),
            identity,
            tracing,
            timer,
            resource,
        )
    }
    pub(super) fn live(
        root: PreparedRoot,
        configuration: LiveConfigurations,
        identity: Identity,
        tracing: TraceContext,
        resource: Arc<dyn ResourceGate>,
    ) -> Result<Self, Failure> {
        Self::prepared(
            root,
            Source::Live(Box::new(configuration)),
            identity,
            tracing,
            Arc::new(RuntimeCancellationTimer),
            resource,
        )
    }
    pub(super) fn smoke(
        root: PreparedRoot,
        endpoint: OwnedLoopback,
        configuration: LiveConfigurations,
        identity: Identity,
        tracing: TraceContext,
        resource: Arc<dyn ResourceGate>,
    ) -> Result<Self, Failure> {
        Self::prepared(
            root,
            Source::OwnedCredentials(endpoint, Box::new(configuration)),
            identity,
            tracing,
            Arc::new(RuntimeCancellationTimer),
            resource,
        )
    }
    fn fresh(
        root: &Path,
        source: Source,
        identity: Identity,
        tracing: TraceContext,
        timer: Arc<dyn CancellationTimer>,
        resource: Arc<dyn ResourceGate>,
    ) -> Result<Self, Failure> {
        super::journal::plain(root)?;
        std::fs::create_dir(root).map_err(|_| Failure::ReportOnly)?;
        Self::open_created(root, source, identity, tracing, timer, resource)
    }
    fn prepared(
        root: PreparedRoot,
        source: Source,
        identity: Identity,
        tracing: TraceContext,
        timer: Arc<dyn CancellationTimer>,
        resource: Arc<dyn ResourceGate>,
    ) -> Result<Self, Failure> {
        Self::open_created(root.path(), source, identity, tracing, timer, resource)
    }
    fn open_created(
        root: &Path,
        source: Source,
        identity: Identity,
        tracing: TraceContext,
        timer: Arc<dyn CancellationTimer>,
        resource: Arc<dyn ResourceGate>,
    ) -> Result<Self, Failure> {
        let results = root.join("results");
        std::fs::create_dir(&results).map_err(|_| Failure::Storage)?;
        let core_path = results.join("sessions.db");
        let sessions = SessionService::open(&core_path).map_err(|_| Failure::Storage)?;
        let mut journal = Journal::open(&root.join("round"), identity)?;
        journal.arm()?;
        let latch = AdverseLatch::new();
        let coordinator = Coordinator::new(journal, core_path, latch.clone(), resource);
        Ok(Self {
            sessions,
            coordinator,
            source,
            latch,
            tracing,
            next: 1,
            retry_history: None,
            timer,
        })
    }
    /// Actual saved histories for safe exports/research, never a credential view.
    pub fn sessions(&self) -> &SessionService {
        &self.sessions
    }
    /// Safe durable authorization snapshot; fees remain in production history.
    pub fn snapshot(&self) -> Result<Snapshot, Failure> {
        self.coordinator.snapshot()
    }
    /// Persist an irreversible local stop, retaining reservations and evidence.
    pub fn halt(&self) -> Result<(), Failure> {
        self.coordinator.halt()
    }
    /// Use the production atomic export, then independent bounded readback.
    /// A failed write/parse/reconciliation irreversibly stops later requests.
    pub fn export(
        &self,
        destination: &Path,
        space: &dyn notype_core::csv::CsvSpaceProbe,
    ) -> Result<super::export::CsvProof, Failure> {
        let result = (|| {
            self.sessions
                .export_csv(destination, space, &Default::default())
                .map_err(|_| Failure::Storage)?;
            let entries = self.sessions.search("").map_err(|_| Failure::Storage)?;
            let bytes = super::native::read(destination, 8 * 1024 * 1024)?;
            super::export::verify(&bytes, &entries, &self.snapshot()?)
        })();
        if result.is_err() {
            self.halt()?;
        }
        result
    }
    /// Execute only the next fixed step, including actual automatic reverse.
    /// Failure halts; no implicit POST retry, replacement slot or delivery exists.
    pub async fn next(&mut self) -> Result<Option<Step>, Failure> {
        let mut lease = Settlement {
            coordinator: self.coordinator.clone(),
            armed: true,
        };
        let result = self.next_inner().await;
        if result.is_err() {
            self.halt()?;
        }
        lease.armed = false;
        result
    }
    async fn next_inner(&mut self) -> Result<Option<Step>, Failure> {
        if usize::from(self.next) > MAX_CALLS {
            return Ok(None);
        }
        let slot = schedule()[usize::from(self.next - 1)];
        if self.latch.observed().is_some() || self.snapshot()?.halted {
            return Err(Failure::Halted);
        }
        let id = if slot.number == 8 {
            self.retry_history.clone().ok_or(Failure::Schedule)?
        } else {
            self.sessions
                .create(
                    slot.corpus,
                    SessionOptions {
                        scenario: slot.scenario(),
                        translation_target: if slot.scenario() == notype_core::Scenario::Translation
                        {
                            Some("en-US".into())
                        } else {
                            None
                        },
                        ..Default::default()
                    },
                )
                .map_err(|_| Failure::Storage)?
                .id
        };
        let processor = match &self.source {
            Source::Owned(endpoint) => configuration::owned_processor(
                endpoint,
                slot,
                self.latch.clone(),
                self.tracing.clone(),
            ),
            Source::Live(configuration) => {
                configuration.processor(slot, self.latch.clone(), self.tracing.clone())
            }
            Source::OwnedCredentials(endpoint, configuration) => configuration.processor_owned(
                endpoint,
                slot,
                self.latch.clone(),
                self.tracing.clone(),
            ),
        }?;
        let processor =
            BudgetProcessor::new(processor, self.coordinator.clone(), id.clone(), slot.number);
        let processed = if slot.number == 7 {
            self.cancel_step(&id, processor).await
        } else {
            self.sessions
                .process_with_review(
                    &id,
                    vec![],
                    processor,
                    slot.purpose == Purpose::ForwardReview,
                )
                .await
                .map_err(|_| Failure::Storage)
        };
        let history = match processed {
            Ok(history) => history,
            Err(error) => {
                self.halt()?;
                return Err(error);
            }
        };
        if self.coordinator.reconcile().is_err()
            || self.latch.observed().is_some()
            || self.snapshot()?.halted
        {
            self.halt()?;
            return Err(Failure::Halted);
        }
        let last = if slot.purpose == Purpose::ForwardReview {
            slot.number + 1
        } else {
            slot.number
        };
        if slot.number != 7
            && (history.error.is_some()
                || history
                    .result
                    .as_deref()
                    .is_none_or(|text| !quality(slot, text))
                || slot.purpose == Purpose::ForwardReview
                    && history
                        .back_translation
                        .as_deref()
                        .is_none_or(|text| !quality(schedule()[usize::from(last - 1)], text)))
        {
            self.halt()?;
            return Err(Failure::Quality);
        }
        if slot.number == 7 {
            self.retry_history = Some(id);
        }
        self.next = last + 1;
        Ok(Some(Step {
            first_slot: slot.number,
            last_slot: last,
            history,
        }))
    }
    async fn cancel_step(
        &self,
        id: &str,
        processor: Arc<BudgetProcessor>,
    ) -> Result<SessionEntry, Failure> {
        let mut settlement = Settlement {
            coordinator: self.coordinator.clone(),
            armed: true,
        };
        let sessions = self.sessions.clone();
        let history = id.to_owned();
        let processing = processor.clone();
        let mut request = OwnedTask::new(tokio::spawn(async move {
            sessions
                .process(&history, vec![], processing)
                .await
                .map_err(|_| Failure::Storage)
        }));
        let sessions = self.sessions.clone();
        let coordinator = self.coordinator.clone();
        let delay = self.timer.clone();
        let mut timer = OwnedTask::new(tokio::spawn(async move {
            processor.wait_until_sent().await?;
            delay.wait().await;
            let history = coordinator.request_cancel()?;
            sessions.cancel(&history).map_err(|_| Failure::Storage)?;
            Ok::<(), Failure>(())
        }));
        let result = request.join().await;
        timer.abort();
        let timer_result = timer.join().await;
        if let Ok(Err(error)) = timer_result {
            return Err(error);
        }
        if timer_result
            .as_ref()
            .is_err_and(|error| !error.is_cancelled())
        {
            return Err(Failure::Storage);
        }
        result.map_err(|_| Failure::Storage)??;
        settlement.armed = false;
        self.sessions.get(id).map_err(|_| Failure::Storage)
    }
}
struct Settlement {
    coordinator: Arc<Coordinator>,
    armed: bool,
}
impl Drop for Settlement {
    fn drop(&mut self) {
        if self.armed {
            let _ = self.coordinator.halt();
        }
    }
}
struct OwnedTask<T> {
    handle: Option<tokio::task::JoinHandle<T>>,
}
#[derive(Clone, Copy)]
enum JoinFailure {
    Cancelled,
    Failed,
}
impl JoinFailure {
    fn is_cancelled(&self) -> bool {
        matches!(self, Self::Cancelled)
    }
}
impl<T> OwnedTask<T> {
    fn new(handle: tokio::task::JoinHandle<T>) -> Self {
        Self {
            handle: Some(handle),
        }
    }
    fn abort(&self) {
        if let Some(handle) = &self.handle {
            handle.abort();
        }
    }
    async fn join(&mut self) -> Result<T, JoinFailure> {
        let Some(handle) = self.handle.as_mut() else {
            return Err(JoinFailure::Failed);
        };
        let result = handle.await.map_err(|error| {
            if error.is_cancelled() {
                JoinFailure::Cancelled
            } else {
                JoinFailure::Failed
            }
        });
        self.handle.take();
        result
    }
}
impl<T> Drop for OwnedTask<T> {
    fn drop(&mut self) {
        self.abort();
    }
}
