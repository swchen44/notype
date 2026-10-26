#[path = "../../desktop/src-tauri/src/capture_startup.rs"]
mod capture_startup;
#[path = "../../desktop/src-tauri/src/diagnostics.rs"]
mod diagnostics;
use diagnostics::api::*;
use diagnostics::service::{DiagnosticService, DiagnosticStorage};
use diagnostics::{LogFileStatus, LogLevel, Platform, ReportContext};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::{Duration, Instant};
struct Storage;
impl DiagnosticStorage for Storage {
    fn load_preference(&self) -> Result<Option<bool>, &'static str> {
        Ok(None)
    }
    fn save_preference(&self, _: bool) -> Result<(), &'static str> {
        Ok(())
    }
    fn write_report(&self, _: &std::path::Path, _: &str) -> Result<(), &'static str> {
        Ok(())
    }
}
struct Sink(Arc<DiagnosticService>);
impl ApiObserver for Sink {
    fn begin(&self, scope: Option<ScopeBinding>, revision: Option<u64>) -> Option<TraceIdentity> {
        self.0.begin_api(scope, revision)
    }
    fn record(&self, event: ApiEvent) {
        self.0.record_api(event)
    }
}
struct Clock(Mutex<Instant>);
impl capture_startup::CaptureClock for Clock {
    fn now(&self) -> Instant {
        *self.0.lock().unwrap()
    }
}
struct Cancel(AtomicBool);
impl CancellationFlag for Cancel {
    fn cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}
fn context() -> ReportContext<'static> {
    ReportContext {
        version: "0.1.0",
        platform: Platform::Windows,
        log_level: LogLevel::Debug,
        debug_mode: false,
        always_on_microphone: false,
        model_selected: true,
        model_available: true,
        model_is_custom: false,
        local_log: LogFileStatus::Empty,
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
fn options() -> SafeOptions {
    SafeOptions {
        scenario: ApiScenario::Dictation,
        purpose: ApiPurpose::Generation,
        method: ApiMethod::Post,
        profile: ApiProfile::Generic,
        endpoint: EndpointClass::Loopback,
        auth: AuthKind::Bearer,
        credential_source: CredentialSource::Unknown,
        credential_present: true,
        model_present: true,
        effort: Effort::Omitted,
        thinking: None,
        max_output_tokens: Some(512),
        timeout_ms: 1000,
        request_bytes: 100,
        message_count: 3,
        non_streaming: true,
        plain_text_only: true,
    }
}
fn service() -> Arc<DiagnosticService> {
    let service = Arc::new(DiagnosticService::new());
    service.initialize(Arc::new(Storage));
    service
}

#[test]
fn production_trace_uses_injected_clock_and_exactly_one_truthful_drop_terminal() {
    let service = service();
    let clock = Arc::new(Clock(Mutex::new(Instant::now())));
    let cancel = Arc::new(Cancel(AtomicBool::new(false)));
    let scope = service.api_scope(Some("PRIVATE_HISTORY_UUID"), Some(2));
    let mut trace = CallTrace::begin(
        scope,
        options(),
        Some(7),
        clock.clone(),
        Arc::new(Sink(service.clone())),
        cancel.clone(),
    );
    *clock.0.lock().unwrap() += Duration::from_millis(100);
    trace.metadata().unwrap().possibly_sent = true;
    trace.observe(ApiSpan::SendStarted, ApiResult::Started);
    *clock.0.lock().unwrap() += Duration::from_millis(25);
    trace.metadata().unwrap().http_status = Some(200);
    trace.observe(ApiSpan::HeadersReceived, ApiResult::Success);
    trace.metadata().unwrap().body_bytes = 13;
    cancel.0.store(true, Ordering::Release);
    drop(trace);
    let text = report(&service);
    assert!(text.contains("api_span=headers_received last_span=headers_received api_result=success category=none call_elapsed_ms=125"));
    assert!(text.contains("duration_ms=25"));
    assert_eq!(text.matches("api_span=finished").count(), 1);
    assert!(text.contains("api_result=interrupted"));
    assert!(text.contains("cancel_requested=true"));
    assert!(text.contains("body_bytes=13"));
    assert!(text.contains("revision=7"));
    assert!(!text.contains("PRIVATE_HISTORY_UUID"));
}

#[test]
fn disabled_and_old_generations_allocate_nothing_and_cannot_resurrect() {
    let service = Arc::new(DiagnosticService::new());
    assert!(service.api_scope(Some("private"), None).is_none());
    assert!(service.begin_api(None, None).is_none());
    service.initialize(Arc::new(Storage));
    let old = service.api_scope(Some("private"), None).unwrap();
    assert_eq!(old.scope.session, Some(1));
    assert_eq!(service.begin_api(Some(old), Some(1)).unwrap().call, 1);
    service.set_enabled(false);
    assert!(service.api_scope(Some("other"), None).is_none());
    assert!(service.begin_api(None, None).is_none());
    service.set_enabled(true);
    assert!(service.begin_api(Some(old), Some(1)).is_none());
    let new = service.api_scope(Some("new"), None).unwrap();
    assert_eq!(new.scope.session, Some(1));
    assert_eq!(service.begin_api(Some(new), Some(2)).unwrap().call, 1);
    service.clear();
    assert!(service.begin_api(Some(new), Some(2)).is_none());
}

#[test]
fn clear_and_off_on_barriers_reject_all_late_phases_including_drop() {
    for clear in [true, false] {
        let service = service();
        let scope = service.api_scope(Some("private"), None);
        let trace = CallTrace::begin(
            scope,
            options(),
            Some(1),
            Arc::new(capture_startup::MonotonicClock),
            Arc::new(Sink(service.clone())),
            CallTrace::never_cancelled(),
        );
        let barrier = Arc::new(std::sync::Barrier::new(2));
        let worker_barrier = barrier.clone();
        let worker = std::thread::spawn(move || {
            let mut trace = trace;
            worker_barrier.wait();
            trace.observe(ApiSpan::SendStarted, ApiResult::Started);
            drop(trace);
        });
        if clear {
            service.clear();
        } else {
            service.set_enabled(false);
            service.set_enabled(true);
        }
        let before = report(&service);
        barrier.wait();
        worker.join().unwrap();
        let after = report(&service);
        assert_eq!(
            before
                .lines()
                .filter(|line| line.starts_with("elapsed_ms="))
                .collect::<Vec<_>>(),
            after
                .lines()
                .filter(|line| line.starts_with("elapsed_ms="))
                .collect::<Vec<_>>()
        );
    }
}

#[test]
fn mixed_ring_and_large_call_population_keep_whole_rows_and_mandatory_bounds() {
    let service = service();
    for _ in 0..100 {
        service.record(
            diagnostics::Phase::ModelLoad,
            diagnostics::Outcome::Ok,
            Some(1),
        );
        let mut trace = CallTrace::begin(
            None,
            options(),
            None,
            Arc::new(capture_startup::MonotonicClock),
            Arc::new(Sink(service.clone())),
            CallTrace::never_cancelled(),
        );
        trace.metadata().unwrap().http_status = Some(200);
        trace.observe(ApiSpan::HeadersReceived, ApiResult::Success);
        trace.body(u64::MAX, true, true);
        trace.finish(ApiResult::Failed, ApiErrorCategory::Oversize);
        trace.finish(ApiResult::Success, ApiErrorCategory::None);
    }
    let text = report(&service);
    assert!(text.len() <= 65536);
    assert!(text.lines().count() <= 512);
    assert!(text.contains("events_retained=128"));
    assert!(text.contains("ring_truncated=true"));
    assert!(text.contains("report_truncated="));
    assert!(text.contains("bytes_capped=true"));
    assert!(
        text.contains("api_call=100 "),
        "bounded report must retain the latest failure tail"
    );
    assert!(text.ends_with('\n'));
}
