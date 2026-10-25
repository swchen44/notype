//! Production capture-start/readiness observations. Native adapters keep resource
//! ownership and DSP; this module owns only the real handoff and timing math.
use std::sync::{
    atomic::{AtomicBool, AtomicU32, Ordering},
    mpsc, Arc,
};
use std::time::{Duration, Instant};

/// Maximum exported duration; longer observations are explicitly marked capped.
pub const MAX_TIMING_MS: u64 = 86_400_000;
/// Fixed-bound process-local ordinal allocator. Exhaustion never wraps.
pub struct AttemptSequence<const LIMIT: u32>(AtomicU32);
impl<const LIMIT: u32> Default for AttemptSequence<LIMIT> {
    fn default() -> Self {
        Self::new()
    }
}
impl<const LIMIT: u32> AttemptSequence<LIMIT> {
    /// Starts this fixed-bound sequence at zero without allocating or collecting.
    pub const fn new() -> Self {
        Self(AtomicU32::new(0))
    }
    /// Reserves a nonzero ordinal, or returns None forever after exhaustion.
    pub fn next(&self) -> Option<u32> {
        self.0
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                value.checked_add(1).filter(|next| *next <= LIMIT)
            })
            .ok()
            .and_then(|value| value.checked_add(1))
    }
}
static NEXT_ATTEMPT: AttemptSequence<{ u32::MAX }> = AttemptSequence::new();

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Collector clear epoch and preference revision, never an audio owner.
pub struct CollectionToken {
    /// Rejects work captured before the latest Clear.
    pub epoch: u64,
    /// Rejects work captured before an OFF/ON preference transition.
    pub revision: u64,
}
#[derive(Clone, Copy, Debug)]
/// Lock-free intent metadata copied through the existing worker queue.
pub struct AcceptedAttempt {
    /// App-run ordinal only; None means exhausted/unavailable, never reuse.
    pub ordinal: Option<u32>,
    /// Backend acceptance instant; no key value or content is retained.
    pub accepted_at: Instant,
    /// Actual worker dequeue boundary; absent for direct action starts.
    pub dequeued_at: Option<Instant>,
}
impl AcceptedAttempt {
    /// Lock-free metadata allocation. It never wraps or reads settings/collector.
    pub fn now() -> Self {
        Self::at(Instant::now())
    }
    /// Allocates metadata at a caller-captured acceptance boundary.
    pub fn at(accepted_at: Instant) -> Self {
        let ordinal = NEXT_ATTEMPT.next();
        Self {
            ordinal,
            accepted_at,
            dequeued_at: None,
        }
    }
    /// Captures dequeue before logging or invoking the action.
    pub fn dequeued(mut self) -> Self {
        self.dequeued_at = Some(Instant::now());
        self
    }
}

macro_rules! labels {
    ($name:ident { $($variant:ident => $label:literal),+ $(,)? }) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        /// Finite report codes; callers cannot supply freeform text.
        pub enum $name { $(#[doc = concat!("Allowlisted metadata code `", $label, "`.")] $variant),+ }
        impl $name { #[doc = "Returns this allowlisted report code."] pub fn label(self) -> &'static str { match self { $(Self::$variant => $label),+ } } }
    };
}
labels!(CaptureSpan {
    Attempt => "attempt", DispatchQueue => "dispatch-queue", ActionCumulative => "action-cumulative",
    StreamOpen => "stream-open", CommandQueue => "command-queue", FirstChunkCumulative => "consumer-first-chunk-cumulative",
    ChunkProcessing => "chunk-processing", ProcessedCumulative => "processed-ready-cumulative", WaiterScheduling => "waiter-scheduling",
    ReadyQueueCumulative => "ready-queue-cumulative", MainQueue => "main-queue", FrontendRoundTrip => "frontend-round-trip-upper-bound", FrontendCommit => "frontend-commit-local",
    FrontendSettings => "frontend-settings-local", ModelLoad => "model-job", CueSubmission => "cue-call-including-playback"
});
impl CaptureSpan {
    /// Names the duration origin; cumulative spans must not be summed with locals.
    pub fn origin(self) -> &'static str {
        match self {
            Self::Attempt | Self::DispatchQueue => "intent-acceptance",
            Self::ActionCumulative
            | Self::FirstChunkCumulative
            | Self::ProcessedCumulative
            | Self::ReadyQueueCumulative => "action-entry",
            Self::StreamOpen => "ensure-stream-entry",
            Self::CommandQueue => "capture-command-send",
            Self::ChunkProcessing => "consumer-chunk-entry",
            Self::WaiterScheduling => "processed-ready-observation",
            Self::MainQueue => "ready-main-queue-submit",
            Self::FrontendRoundTrip => "ready-native-emit",
            Self::FrontendCommit => "frontend-ready-receipt",
            Self::FrontendSettings => "frontend-show-receipt",
            Self::ModelLoad => "originating-model-job-entry",
            Self::CueSubmission => "cue-function-entry",
        }
    }
}
labels!(CaptureOutcome {
    Started => "started", Ok => "ok", Failed => "failed", Cancelled => "cancelled", Stopped => "stopped",
    Replaced => "replaced", Unavailable => "unavailable", Opened => "opened", Reused => "reused", Reopened => "reopened",
    SharedExisting => "shared-existing-unassociated"
});
impl CaptureOutcome {
    /// Whether this outcome closes an attempt observation, not a microphone.
    pub fn terminal(self) -> bool {
        matches!(
            self,
            Self::Failed | Self::Cancelled | Self::Stopped | Self::Replaced | Self::Unavailable
        )
    }
}
#[derive(Clone, Debug)]
/// Bounded event DTO accepted by the structured-diagnostics collector.
pub struct CaptureEvent {
    /// Collection generation used only to reject stale insertion.
    pub token: CollectionToken,
    /// App-run ordinal; unassociated model work uses None.
    pub attempt: Option<u32>,
    /// Measurement boundary with an explicit documented origin.
    pub span: CaptureSpan,
    /// Fixed completion/disposition code.
    pub outcome: CaptureOutcome,
    /// Integer milliseconds, bounded to MAX_TIMING_MS.
    pub duration_ms: Option<u64>,
    /// True when the raw duration exceeded the export bound.
    pub capped: bool,
}
/// Typed sink; production checks collection permission under its state lock.
pub trait CaptureObserver: Send + Sync {
    /// Receives metadata only, with no audio, paths, persistent/user-content IDs or freeform messages.
    fn record(&self, event: CaptureEvent);
    /// Used only to reject late frontend telemetry, never to gate recording.
    fn accepts(&self, _token: CollectionToken) -> bool {
        true
    }
}
/// One backend monotonic clock; tests substitute the clock, not duration math.
pub trait CaptureClock: Send + Sync {
    /// Returns an Instant from this clock domain.
    fn now(&self) -> Instant;
}
/// Production backend clock. It is never subtracted from frontend performance.now.
pub struct MonotonicClock;
impl CaptureClock for MonotonicClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}
struct TraceInner {
    accepted: AcceptedAttempt,
    token: Option<CollectionToken>,
    action_entry: Instant,
    clock: Arc<dyn CaptureClock>,
    observer: Arc<dyn CaptureObserver>,
    terminal: AtomicBool,
}
#[derive(Clone)]
/// Immutable attempt/origin/collector binding shared by real pipeline handoffs.
pub struct CaptureTrace(Arc<TraceInner>);
impl CaptureTrace {
    /// Starts an observation without changing capture ownership or preferences.
    pub fn begin(
        accepted: AcceptedAttempt,
        token: Option<CollectionToken>,
        clock: Arc<dyn CaptureClock>,
        observer: Arc<dyn CaptureObserver>,
    ) -> Self {
        let action_entry = clock.now();
        let trace = Self(Arc::new(TraceInner {
            accepted,
            token,
            action_entry,
            clock,
            observer,
            terminal: AtomicBool::new(false),
        }));
        trace.record(
            CaptureSpan::Attempt,
            if accepted.ordinal.is_some() {
                CaptureOutcome::Started
            } else {
                CaptureOutcome::Unavailable
            },
            None,
        );
        match accepted.dequeued_at {
            Some(dequeued) => trace.record(
                CaptureSpan::DispatchQueue,
                CaptureOutcome::Ok,
                Some(dequeued.saturating_duration_since(accepted.accepted_at)),
            ),
            None => trace.record(
                CaptureSpan::DispatchQueue,
                CaptureOutcome::Unavailable,
                None,
            ),
        }
        trace.record(CaptureSpan::ActionCumulative, CaptureOutcome::Started, None);
        trace
    }
    /// Reads this trace's backend clock domain.
    pub fn now(&self) -> Instant {
        self.0.clock.now()
    }
    /// Collection permission is observational only; never controls microphone state.
    pub fn collecting(&self) -> bool {
        self.0
            .token
            .is_some_and(|token| self.0.observer.accepts(token))
    }
    /// Whether attempt observations remain open; not a recording-state query.
    pub fn active(&self) -> bool {
        !self.0.terminal.load(Ordering::Acquire)
    }
    /// Records a typed local span; late originating model jobs retain their ordinal.
    pub fn record(&self, span: CaptureSpan, outcome: CaptureOutcome, duration: Option<Duration>) {
        if !self.active() && span != CaptureSpan::ModelLoad {
            return;
        }
        self.emit(span, outcome, duration, self.0.accepted.ordinal);
    }
    fn emit(
        &self,
        span: CaptureSpan,
        outcome: CaptureOutcome,
        duration: Option<Duration>,
        attempt: Option<u32>,
    ) {
        let Some(token) = self.0.token else {
            return;
        };
        self.0.observer.record(CaptureEvent {
            token,
            attempt,
            span,
            outcome,
            duration_ms: duration.map(|value| value.as_millis().min(MAX_TIMING_MS as u128) as u64),
            capped: duration.is_some_and(|value| value.as_millis() > MAX_TIMING_MS as u128),
        });
    }
    /// Records an action-entry cumulative boundary, explicitly labeled as such.
    pub fn cumulative(&self, span: CaptureSpan, outcome: CaptureOutcome) {
        self.record(
            span,
            outcome,
            Some(self.now().saturating_duration_since(self.0.action_entry)),
        );
    }
    /// An existing shared load is never attributed to a later capture.
    pub fn unassociated_model_job(&self) {
        self.emit(
            CaptureSpan::ModelLoad,
            CaptureOutcome::SharedExisting,
            None,
            None,
        );
    }
    /// Closes diagnostic observations once; never stops/cancels an audio resource.
    pub fn finish(&self, outcome: CaptureOutcome) {
        if !outcome.terminal() || self.0.terminal.swap(true, Ordering::AcqRel) {
            return;
        }
        self.emit(
            CaptureSpan::Attempt,
            outcome,
            Some(
                self.now()
                    .saturating_duration_since(self.0.accepted.accepted_at),
            ),
            self.0.accepted.ordinal,
        );
    }
}

#[derive(Clone, Copy, Debug)]
/// Original native stream status, excluding names or device identifiers.
pub enum StreamDisposition {
    /// Native stream was opened from the closed state.
    Opened,
    /// Existing stream passed the original aliveness check.
    Reused,
    /// Existing failed stream followed the original reopen path.
    Reopened,
}
impl StreamDisposition {
    fn outcome(self) -> CaptureOutcome {
        match self {
            Self::Opened => CaptureOutcome::Opened,
            Self::Reused => CaptureOutcome::Reused,
            Self::Reopened => CaptureOutcome::Reopened,
        }
    }
}
/// The native adapter retains its original manager locks/state/mode policy.
pub trait RecorderPort {
    /// Original native VAD/capture policy, opaque to timing logic.
    type Policy: Copy;
    /// Original native error type; it never enters timing metadata.
    type Error;
    /// Delegates the existing open/reuse/reopen operation unchanged.
    fn ensure_stream(&mut self) -> Result<StreamDisposition, Self::Error>;
    /// Sends the existing recording request and its owned one-shot readiness.
    fn request_capture(
        &mut self,
        policy: Self::Policy,
        ready: ReadyRequest,
    ) -> Result<(), Self::Error>;
}
/// Exact production open -> capture-command -> readiness handoff.
pub fn start_recording<P: RecorderPort>(
    port: &mut P,
    policy: P::Policy,
    trace: CaptureTrace,
) -> Result<ReadyWaiter, P::Error> {
    let open_at = trace.now();
    trace.record(CaptureSpan::StreamOpen, CaptureOutcome::Started, None);
    let disposition = match port.ensure_stream() {
        Ok(value) => value,
        Err(error) => {
            trace.record(
                CaptureSpan::StreamOpen,
                CaptureOutcome::Failed,
                Some(trace.now().saturating_duration_since(open_at)),
            );
            return Err(error);
        }
    };
    trace.record(
        CaptureSpan::StreamOpen,
        disposition.outcome(),
        Some(trace.now().saturating_duration_since(open_at)),
    );
    let (sender, receiver) = mpsc::channel();
    let request = ReadyRequest {
        sent_at: trace.now(),
        trace: trace.clone(),
        sender,
    };
    port.request_capture(policy, request)?;
    trace.cumulative(CaptureSpan::ActionCumulative, CaptureOutcome::Ok);
    Ok(ReadyWaiter { receiver, trace })
}
/// Owned one-shot command payload; created by the real startup service.
pub struct ReadyRequest {
    sent_at: Instant,
    trace: CaptureTrace,
    sender: mpsc::Sender<ProcessedReady>,
}
impl ReadyRequest {
    /// Command-send Instant in the trace's clock domain.
    pub fn sent_at(&self) -> Instant {
        self.sent_at
    }
    /// Consumer-thread observation; never invoked by the real-time audio callback.
    pub fn command_received(&self) {
        self.trace.record(
            CaptureSpan::CommandQueue,
            CaptureOutcome::Ok,
            Some(self.trace.now().saturating_duration_since(self.sent_at)),
        );
    }
}
struct ProcessedReady {
    processed_at: Instant,
    trace: CaptureTrace,
}
/// Compatibility for the standalone recorder CLI/native tests. Both paths keep
/// the same signal-after-processing semantics; only the observed path has timing.
pub enum ReadySignal {
    /// Standalone recorder/CLI compatibility with its original unit signal.
    Legacy(mpsc::Sender<()>),
    /// Production capture carries processed timestamps and correlation.
    Observed(ReadyRequest),
}
impl ReadySignal {
    /// Consumer-thread observation; never invoked by the real-time audio callback.
    pub fn command_received(&self) {
        if let Self::Observed(request) = self {
            request.command_received();
        }
    }
}
/// Wraps actual production DSP, never duplicates it. Empty/idle chunks cannot
/// resolve readiness. Native CaptureProcessor delegates its captured chunk here.
pub fn process_captured_chunk(
    signal: &mut Option<ReadySignal>,
    samples: &[f32],
    process: impl FnOnce(&[f32]),
) {
    if samples.is_empty() {
        process(samples);
        return;
    }
    let first = signal.take();
    let chunk_at = match &first {
        Some(ReadySignal::Observed(request)) => {
            request
                .trace
                .cumulative(CaptureSpan::FirstChunkCumulative, CaptureOutcome::Ok);
            Some(request.trace.now())
        }
        _ => None,
    };
    process(samples);
    match first {
        Some(ReadySignal::Observed(request)) => {
            let processed_at = request.trace.now();
            request.trace.record(
                CaptureSpan::ChunkProcessing,
                CaptureOutcome::Ok,
                Some(processed_at.saturating_duration_since(chunk_at.unwrap_or(processed_at))),
            );
            request
                .trace
                .cumulative(CaptureSpan::ProcessedCumulative, CaptureOutcome::Ok);
            let _ = request.sender.send(ProcessedReady {
                processed_at,
                trace: request.trace,
            });
        }
        Some(ReadySignal::Legacy(sender)) => {
            let _ = sender.send(());
        }
        None => {}
    }
}
/// Dedicated-worker one-shot receiver; it does not own recording state.
pub struct ReadyWaiter {
    receiver: mpsc::Receiver<ProcessedReady>,
    trace: CaptureTrace,
}
impl ReadyWaiter {
    /// Waits for processed first samples and measures observation-to-wakeup delay.
    pub fn wait(self) -> Option<ReadyObservation> {
        match self.receiver.recv() {
            Ok(ready) => {
                ready.trace.record(
                    CaptureSpan::WaiterScheduling,
                    CaptureOutcome::Ok,
                    Some(
                        ready
                            .trace
                            .now()
                            .saturating_duration_since(ready.processed_at),
                    ),
                );
                Some(ReadyObservation { trace: ready.trace })
            }
            Err(_) => {
                self.trace.finish(CaptureOutcome::Unavailable);
                None
            }
        }
    }
}
/// Successful waiter handoff, distinct from raw hardware callback timing.
pub struct ReadyObservation {
    /// The original immutable attempt, not whichever capture is now current.
    pub trace: CaptureTrace,
}
impl ReadyObservation {
    /// Captures main-queue submission before a native scheduling operation.
    pub fn queue(self) -> ReadyDispatch {
        let queued_at = self.trace.now();
        self.trace
            .cumulative(CaptureSpan::ReadyQueueCumulative, CaptureOutcome::Started);
        ReadyDispatch {
            queued_at,
            trace: self.trace,
        }
    }
}
/// Main-thread adapter checks its original capture ticket before commit.
pub struct ReadyDispatch {
    queued_at: Instant,
    trace: CaptureTrace,
}
impl ReadyDispatch {
    /// Measures main-queue delay only after the adapter validates its P21 owner.
    pub fn commit(self, emit: impl FnOnce(EmissionStamp) -> bool) {
        self.trace.record(
            CaptureSpan::FrontendRoundTrip,
            CaptureOutcome::Started,
            None,
        );
        self.trace
            .record(CaptureSpan::FrontendCommit, CaptureOutcome::Started, None);
        let emitted_at = self.trace.now();
        let elapsed = emitted_at.saturating_duration_since(self.queued_at);
        let ok = emit(EmissionStamp {
            trace: self.trace.clone(),
            emitted_at,
        });
        if !ok {
            self.trace
                .record(CaptureSpan::FrontendRoundTrip, CaptureOutcome::Failed, None);
            self.trace
                .record(CaptureSpan::FrontendCommit, CaptureOutcome::Failed, None);
        }
        self.trace.record(
            CaptureSpan::MainQueue,
            if ok {
                CaptureOutcome::Ok
            } else {
                CaptureOutcome::Failed
            },
            Some(elapsed),
        );
    }
}
#[derive(Clone)]
/// Backend event-emission boundary used for a round-trip upper bound only.
pub struct EmissionStamp {
    /// The original immutable attempt, not whichever capture is now current.
    pub trace: CaptureTrace,
    /// Backend emission Instant; frontend absolute clocks are not accepted.
    pub emitted_at: Instant,
}
