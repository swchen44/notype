#[path = "../../desktop/src-tauri/src/capture_observations.rs"]
mod capture_observations;
#[path = "../../desktop/src-tauri/src/capture_startup.rs"]
mod capture_startup;
#[path = "../../desktop/src-tauri/src/diagnostics.rs"]
mod diagnostics;
#[path = "../../desktop/src-tauri/src/overlay_lifecycle.rs"]
mod overlay_lifecycle;
use capture_observations::{CaptureObservations, FrontendObservation};
use capture_startup::*;
use diagnostics::service::{DiagnosticService, DiagnosticStorage};
use overlay_lifecycle::{CaptureId, OverlayLifecycle, Presentation};
use std::{
    path::Path,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
struct Clock {
    origin: Instant,
    ms: AtomicU64,
}
impl Clock {
    fn advance(&self, ms: u64) {
        self.ms.fetch_add(ms, Ordering::SeqCst);
    }
}
impl CaptureClock for Clock {
    fn now(&self) -> Instant {
        self.origin + Duration::from_millis(self.ms.load(Ordering::SeqCst))
    }
}
struct Storage;
impl DiagnosticStorage for Storage {
    fn load_preference(&self) -> Result<Option<bool>, &'static str> {
        Ok(None)
    }
    fn save_preference(&self, _: bool) -> Result<(), &'static str> {
        Ok(())
    }
    fn write_report(&self, _: &Path, _: &str) -> Result<(), &'static str> {
        Ok(())
    }
}
fn context() -> diagnostics::ReportContext<'static> {
    diagnostics::ReportContext {
        version: "0.1.0",
        platform: diagnostics::Platform::Windows,
        log_level: diagnostics::LogLevel::Debug,
        debug_mode: false,
        always_on_microphone: false,
        model_selected: true,
        model_available: true,
        model_is_custom: false,
        local_log: diagnostics::LogFileStatus::Empty,
    }
}
fn report(service: &DiagnosticService) -> String {
    service
        .publish_preview(service.begin_preview().unwrap(), context())
        .unwrap()
        .report
        .text()
        .into()
}
fn setup() -> (Arc<DiagnosticService>, Arc<Clock>) {
    let service = Arc::new(DiagnosticService::new());
    service.initialize(Arc::new(Storage));
    (
        service,
        Arc::new(Clock {
            origin: Instant::now(),
            ms: AtomicU64::new(0),
        }),
    )
}
fn trace(service: &Arc<DiagnosticService>, clock: &Arc<Clock>, ordinal: u32) -> CaptureTrace {
    CaptureTrace::begin(
        AcceptedAttempt {
            ordinal: Some(ordinal),
            accepted_at: clock.now(),
            dequeued_at: None,
        },
        service.capture_token(),
        clock.clone(),
        service.clone(),
    )
}
struct Recorder {
    signal: Option<ReadySignal>,
}
impl RecorderPort for Recorder {
    type Policy = ();
    type Error = ();
    fn ensure_stream(&mut self) -> Result<StreamDisposition, ()> {
        Ok(StreamDisposition::Reused)
    }
    fn request_capture(&mut self, _: (), ready: ReadyRequest) -> Result<(), ()> {
        ready.send(|_, signal| {
            signal.command_received();
            self.signal = Some(signal);
        });
        Ok(())
    }
}

#[test]
fn clear_and_off_on_invalidate_old_capture_observations_without_disabling_recording_handoff() {
    let (service, clock) = setup();
    let old = trace(&service, &clock, 1);
    service.clear();
    old.cumulative(CaptureSpan::ProcessedCumulative, CaptureOutcome::Ok);
    assert!(report(&service).contains("events_retained=0"));
    let old = trace(&service, &clock, 2);
    service.set_enabled(false);
    service.set_enabled(true);
    old.finish(CaptureOutcome::Stopped);
    assert!(!report(&service).contains("capture_outcome=stopped"));
    assert!(!old.collecting());
    service.set_enabled(false);
    let disabled = trace(&service, &clock, 3);
    let mut recorder = Recorder { signal: None };
    let waiter = start_recording(&mut recorder, (), disabled).unwrap();
    process_captured_chunk(&mut recorder.signal, &[0.0; 480], |_| {});
    assert!(
        waiter.wait().is_some(),
        "collection is never capture authority"
    );
}

#[test]
fn real_preview_preserves_local_origins_attempt_correlation_and_bounded_truncation() {
    let (service, clock) = setup();
    let pending = trace(&service, &clock, 7);
    pending.record(
        CaptureSpan::ChunkProcessing,
        CaptureOutcome::Ok,
        Some(Duration::from_secs(172800)),
    );
    let complete = trace(&service, &clock, 8);
    complete.unassociated_model_job();
    complete.cumulative(CaptureSpan::ActionCumulative, CaptureOutcome::Ok);
    complete.finish(CaptureOutcome::Stopped);
    let text = report(&service);
    assert!(text.contains("attempt_summary=7 complete=false incomplete=true"));
    assert!(text.contains("attempt_summary=8 complete=true incomplete=false"));
    assert!(text.contains("duration_ms=86400000 capped=true attempt=7"));
    assert!(text.contains("attempt=unassociated span=model-job"));
    assert!(text.contains("origin=consumer-chunk-entry"));
    service.clear();
    let truncated = trace(&service, &clock, 9);
    for _ in 0..diagnostics::MAX_EVENTS {
        service.record(diagnostics::Phase::Startup, diagnostics::Outcome::Ok, None);
    }
    truncated.finish(CaptureOutcome::Stopped);
    let text = report(&service);
    assert!(text.contains("ring_truncated=true"));
    assert!(text.contains("attempt_summary=9 complete=false incomplete=true truncated=true"));
}

#[test]
fn production_frontend_ack_registry_distinguishes_round_trip_from_local_commit_and_rejects_stale_duplicate_ack(
) {
    let (service, clock) = setup();
    let trace = trace(&service, &clock, 15);
    let registry = CaptureObservations::new();
    registry.bind(987654321, trace.clone());
    let mut recorder = Recorder { signal: None };
    let waiter = start_recording(&mut recorder, (), trace).unwrap();
    process_captured_chunk(&mut recorder.signal, &[0.0; 480], |_| {});
    let dispatch = waiter.wait().unwrap().queue();
    clock.advance(110);
    dispatch.commit(|stamp| {
        registry.emitted(987654321, 5, stamp);
        true
    });
    clock.advance(25);
    assert!(registry.frontend(987654321, 5, FrontendObservation::Received, 0));
    assert!(registry.frontend(987654321, 5, FrontendObservation::Committed, 75));
    assert!(registry.frontend(987654321, 5, FrontendObservation::Settings, 500));
    assert!(!registry.frontend(987654321, 5, FrontendObservation::Received, 0));
    assert!(!registry.frontend(987654320, 5, FrontendObservation::Committed, 75));
    let text = report(&service);
    for (span, duration) in [
        ("main-queue", 110),
        ("frontend-round-trip-upper-bound", 25),
        ("frontend-commit-local", 75),
        ("frontend-settings-local", 500),
    ] {
        assert!(
            text.lines()
                .any(|line| line.contains(&format!("span={span} "))
                    && line.contains(&format!("duration_ms={duration} "))),
            "{span}: {text}"
        );
    }
    assert!(
        !text.contains("987654321"),
        "capture owner never enters a report"
    );
    registry.finish(987654321, CaptureOutcome::Stopped);
    assert!(!registry.frontend(987654321, 5, FrontendObservation::Committed, 12));
}

#[test]
fn late_ready_and_frontend_ack_do_not_attach_to_a_successor_or_cleared_attempt() {
    let (service, clock) = setup();
    let registry = CaptureObservations::new();
    let old = trace(&service, &clock, 20);
    registry.bind(1, old.clone());
    let mut recorder = Recorder { signal: None };
    let waiter = start_recording(&mut recorder, (), old).unwrap();
    let next = trace(&service, &clock, 21);
    registry.bind(2, next);
    process_captured_chunk(&mut recorder.signal, &[0.0; 480], |_| {});
    let old_ready = waiter.wait().unwrap().queue();
    old_ready.commit(|stamp| {
        registry.emitted(1, 1, stamp);
        true
    });
    assert!(!registry.frontend(1, 1, FrontendObservation::Received, 0));
    service.clear();
    assert!(!registry.frontend(2, 2, FrontendObservation::Settings, 500));
    assert!(report(&service).contains("events_retained=0"));
}

#[test]
fn stopped_attempt_with_an_unfinished_measured_stage_is_reported_incomplete() {
    let (service, clock) = setup();
    let attempt = trace(&service, &clock, 30);
    attempt.cumulative(CaptureSpan::ActionCumulative, CaptureOutcome::Ok);
    attempt.record(CaptureSpan::ModelLoad, CaptureOutcome::Started, None);
    attempt.finish(CaptureOutcome::Stopped);
    assert!(report(&service).contains("attempt_summary=30 complete=false incomplete=true"));
}

#[test]
fn oversized_frontend_duration_is_explicitly_capped_in_the_real_report() {
    let (service, clock) = setup();
    let registry = CaptureObservations::new();
    registry.bind(3, trace(&service, &clock, 31));
    assert!(registry.frontend(3, 3, FrontendObservation::Settings, MAX_TIMING_MS + 1));
    let text = report(&service);
    assert!(text
        .lines()
        .any(|line| line.contains("span=frontend-settings-local ")
            && line.contains("duration_ms=86400000 capped=true")));
}

#[test]
fn originating_model_job_keeps_its_ordinal_after_a_later_capture_observes_shared_loading() {
    let (service, clock) = setup();
    let first = trace(&service, &clock, 40);
    first.record(CaptureSpan::ModelLoad, CaptureOutcome::Started, None);
    let later = trace(&service, &clock, 41);
    later.unassociated_model_job();
    first.finish(CaptureOutcome::Stopped);
    clock.advance(120);
    first.record(
        CaptureSpan::ModelLoad,
        CaptureOutcome::Ok,
        Some(Duration::from_millis(120)),
    );
    let text = report(&service);
    assert!(text
        .lines()
        .any(|line| line.contains("attempt=40 span=model-job")
            && line.contains("capture_outcome=ok")));
    assert!(!text
        .lines()
        .any(|line| line.contains("attempt=41 span=model-job")));
    assert!(text.contains("attempt=unassociated span=model-job"));
}

#[test]
fn settings_observation_uses_actual_overlay_revision_gate_before_registry_deduplication() {
    let (service, clock) = setup();
    let registry = CaptureObservations::new();
    let mut lifecycle = OverlayLifecycle::new();
    let current = lifecycle
        .reserve(CaptureId(4), Presentation::Recording)
        .unwrap();
    registry.bind(4, trace(&service, &clock, 42));
    let mut callbacks = 0;
    // Settings intentionally has no emitted-ready revision in the inner registry.
    // Exercise the same outer policy method used by the native ACK adapter.
    assert_eq!(
        lifecycle.observe(current.capture, current.revision + 1, || {
            callbacks += 1;
            registry.frontend(4, current.revision + 1, FrontendObservation::Settings, 999)
        }),
        None
    );
    assert_eq!(callbacks, 0, "stale settings ACK never enters the registry");
    assert!(!report(&service).contains("span=frontend-settings-local"));
    assert_eq!(
        lifecycle.observe(current.capture, current.revision, || {
            callbacks += 1;
            registry.frontend(4, current.revision, FrontendObservation::Settings, 500)
        }),
        Some(true)
    );
    assert_eq!(callbacks, 1, "a rejected ACK did not consume the dedup bit");
    assert!(report(&service)
        .lines()
        .any(|line| line.contains("span=frontend-settings-local ")
            && line.contains("duration_ms=500 ")));
}
