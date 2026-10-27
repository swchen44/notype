#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/capture_startup.rs"]
mod capture_startup;
#[path = "../../desktop/src-tauri/src/connection_activity.rs"]
mod connection_activity;
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/diagnostics.rs"]
mod diagnostics;
#[allow(dead_code, unused_imports)]
#[path = "../../desktop/src-tauri/src/workflow.rs"]
mod workflow;
use connection_activity::{ActivityStore, ProbeObserver};
use notype_core::observations::{
    ActualCharge, ApiCallDescriptor, ApiCallGuard, ApiCallObserver, ApiOutcome, ApiReceipt,
    Completeness, TokenUsage,
};
use notype_core::{
    CancellationToken, GenerationFuture, SessionOptions, SessionService, TextFuture, TextOutput,
    TextProcessor, TextRequest,
};
use std::sync::Arc;

fn descriptor(model: &str) -> ApiCallDescriptor {
    ApiCallDescriptor {
        requested_model: Some(model.into()),
        ..Default::default()
    }
}
fn receipt(input: Option<u64>, output: Option<u64>, amount: Option<(&str, &str)>) -> ApiReceipt {
    ApiReceipt {
        usage: TokenUsage {
            input,
            output,
            ..Default::default()
        },
        actual_charge: amount.map(|(amount, currency)| {
            ActualCharge::new(amount, currency, "fixture-actual").unwrap()
        }),
        ..Default::default()
    }
}
fn send(store: &Arc<ActivityStore>, connection: &str, model: &str, receipt: ApiReceipt) {
    let observer = store.observer(connection.into(), Arc::new(ProbeObserver));
    let mut call = ApiCallGuard::begin(observer, descriptor(model)).unwrap();
    call.mark_sent().unwrap();
    call.response(receipt);
    call.finish(ApiOutcome::Success);
}

#[test]
fn actual_calls_are_isolated_exact_and_persistent_including_unknown_fields() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("activity.db");
    let store = ActivityStore::open(&path).unwrap();
    assert_eq!(store.summary("first").unwrap().call_count, Some(0));
    send(
        &store,
        "first",
        "model-a",
        receipt(Some(12), Some(3), Some(("0.100000000000000001", "USD"))),
    );
    send(
        &store,
        "first",
        "model-b",
        receipt(Some(8), None, Some(("0.2", "USD"))),
    );
    send(
        &store,
        "first",
        "model-b",
        receipt(None, Some(2), Some(("1.25", "TWD"))),
    );
    send(&store, "first", "model-c", receipt(None, None, None));
    send(
        &store,
        "second",
        "independent",
        receipt(Some(91), Some(8), Some(("0", "USD"))),
    );
    drop(store);
    let store = ActivityStore::open(&path).unwrap();
    let summary = store.summary("first").unwrap();
    assert_eq!(summary.call_count, Some(4));
    assert_eq!(summary.input.status, Completeness::Partial);
    assert_eq!(summary.input.total, None);
    assert_eq!(summary.input.known_subtotal, Some(20));
    assert_eq!(summary.output.known_subtotal, Some(5));
    assert_eq!(summary.fees.status, Completeness::Partial);
    assert!(summary.fees.totals.is_empty());
    assert_eq!(
        summary
            .fees
            .known_subtotals
            .iter()
            .map(|entry| (entry.currency.as_str(), entry.amount.as_str()))
            .collect::<Vec<_>>(),
        [("TWD", "1.25"), ("USD", "0.300000000000000001")]
    );
    assert_eq!(summary.requested_models, ["model-a", "model-b", "model-c"]);
    let second = store.summary("second").unwrap();
    assert_eq!(second.input.total, Some(91));
    assert_eq!(second.fees.totals[0].amount, "0");
}

#[test]
fn unsent_work_is_not_charged_and_clear_fences_late_sent_and_response_callbacks() {
    let store = ActivityStore::memory().unwrap();
    let observer = store.observer("first".into(), Arc::new(ProbeObserver));
    let mut pending = ApiCallGuard::begin(observer.clone(), descriptor("pending")).unwrap();
    assert_eq!(store.summary("first").unwrap().observed_call_count, 0);
    let mut in_flight = ApiCallGuard::begin(observer, descriptor("in-flight")).unwrap();
    in_flight.mark_sent().unwrap();
    assert_eq!(
        store.summary("first").unwrap().input.status,
        Completeness::Unknown
    );
    send(&store, "second", "other", receipt(Some(2), Some(1), None));
    store.test_result("first", 7, true, None).unwrap();
    store.clear("first").unwrap();
    pending.mark_sent().unwrap();
    pending.response(receipt(Some(99), Some(99), Some(("3", "USD"))));
    in_flight.response(receipt(Some(80), Some(90), Some(("4", "USD"))));
    pending.finish(ApiOutcome::Success);
    in_flight.finish(ApiOutcome::Success);
    assert_eq!(store.summary("first").unwrap().call_count, Some(0));
    assert_eq!(store.summary("second").unwrap().input.total, Some(2));
    assert!(
        store.last_test("first", 7).unwrap().unwrap().0,
        "usage reset does not fabricate a new connection test"
    );
    send(&store, "first", "new", receipt(Some(4), Some(1), None));
    assert_eq!(store.summary("first").unwrap().input.total, Some(4));
}

#[test]
fn explicit_test_health_is_revision_bound_and_duplicate_admission_is_released() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("activity.db");
    let store = ActivityStore::open(&path).unwrap();
    let first = store.admit_test("first").unwrap();
    assert_eq!(
        store.admit_test("first").err().as_deref(),
        Some("connection-test-busy")
    );
    let independent = store.admit_test("second").unwrap();
    drop(independent);
    drop(first);
    let later = store.admit_test("first").unwrap();
    drop(later);
    store
        .test_result("first", 8, false, Some("processing-timeout"))
        .unwrap();
    assert!(store.last_test("first", 9).unwrap().is_none());
    assert!(store.last_test("second", 8).unwrap().is_none());
    drop(store);
    let store = ActivityStore::open(&path).unwrap();
    let health = store.last_test("first", 8).unwrap().unwrap();
    assert!(!health.0);
    assert_eq!(health.1.as_deref(), Some("processing-timeout"));
    assert!(health.2 > 0);
    store.test_result("first", 9, true, None).unwrap();
    assert!(store.last_test("first", 8).unwrap().is_none());
    assert_eq!(store.last_test("first", 9).unwrap().unwrap().1, None);
}

#[test]
fn unavailable_storage_refuses_admission_before_a_possible_http_send() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("activity.db");
    let store = ActivityStore::open(&path).unwrap();
    // A real external filesystem/database failure, not a mocked observer result.
    rusqlite::Connection::open(&path)
        .unwrap()
        .execute("DROP TABLE connection_calls", [])
        .unwrap();
    let observer = store.observer("connection".into(), Arc::new(ProbeObserver));
    assert!(matches!(
        ApiCallGuard::begin(observer, descriptor("model")),
        Err(notype_core::TextFailure::HistoryStorage)
    ));
    assert_eq!(
        store.summary("connection").err().as_deref(),
        Some("activity-storage-failed")
    );
    assert_eq!(
        store.clear("connection").err().as_deref(),
        Some("activity-storage-failed")
    );
    let corrupt = directory.path().join("corrupt.db");
    std::fs::write(&corrupt, b"owned corrupt fixture").unwrap();
    assert_eq!(
        ActivityStore::open(&corrupt).err().as_deref(),
        Some("activity-storage-failed")
    );
    assert_eq!(std::fs::read(&corrupt).unwrap(), b"owned corrupt fixture");
}

struct MeteredProcessor(Arc<ActivityStore>);
impl TextProcessor for MeteredProcessor {
    fn polish(&self, _: TextRequest, _: CancellationToken) -> TextFuture {
        Box::pin(async { Ok("result".into()) })
    }
    fn generate_observed(
        &self,
        _: TextRequest,
        _: CancellationToken,
        parent: Arc<dyn ApiCallObserver>,
    ) -> GenerationFuture {
        let observer = self.0.observer("session-connection".into(), parent);
        Box::pin(async move {
            let mut call = ApiCallGuard::begin(observer, descriptor("actual-model"))?;
            call.mark_sent()?;
            call.response(receipt(Some(7), Some(3), Some(("0.01", "USD"))));
            call.finish(ApiOutcome::Success);
            Ok(TextOutput {
                text: "result".into(),
                source_language: None,
            })
        })
    }
}
#[tokio::test]
async fn history_deletion_keeps_connection_usage_and_counter_reset_keeps_history() {
    let directory = tempfile::tempdir().unwrap();
    let store = ActivityStore::open(&directory.path().join("activity.db")).unwrap();
    let service = SessionService::open(directory.path().join("history.db")).unwrap();
    let entry = service
        .create("synthetic original", SessionOptions::default())
        .unwrap();
    let result = service
        .process(&entry.id, vec![], Arc::new(MeteredProcessor(store.clone())))
        .await
        .unwrap();
    assert_eq!(result.api_summary.input.total, Some(7));
    assert_eq!(
        store.summary("session-connection").unwrap().input.total,
        Some(7)
    );
    store.clear("session-connection").unwrap();
    assert_eq!(
        service.get(&entry.id).unwrap().api_summary.input.total,
        Some(7)
    );
    send(
        &store,
        "session-connection",
        "probe-model",
        receipt(Some(2), Some(1), None),
    );
    service.delete(&entry.id).unwrap();
    assert_eq!(
        store.summary("session-connection").unwrap().input.total,
        Some(2)
    );
}

#[test]
fn invalid_receipt_metadata_is_unknown_instead_of_free_or_overflowed() {
    let store = ActivityStore::memory().unwrap();
    send(
        &store,
        "first",
        "model",
        ApiReceipt {
            usage: TokenUsage {
                input: Some(u64::MAX),
                output: Some(0),
                ..Default::default()
            },
            actual_charge: Some(ActualCharge {
                amount: "-9".into(),
                currency: "USD".into(),
                provenance: "fixture".into(),
            }),
            ..Default::default()
        },
    );
    let summary = store.summary("first").unwrap();
    assert_eq!(summary.input.status, Completeness::Unknown);
    assert_eq!(summary.output.total, Some(0));
    assert_eq!(summary.fees.status, Completeness::Unknown);
    assert!(summary.fees.known_subtotals.is_empty());
}

#[tokio::test]
async fn actual_transport_accounts_probe_calls_and_only_openrouter_returned_costs() {
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };
    use workflow::{TextAuthentication, TextConfiguration, TextProviderProfile, TextTransport};
    for (profile, status, content, expected_amount, success) in [
        (
            TextProviderProfile::OpenRouter,
            200,
            r#"{"model":"actual-model","usage":{"prompt_tokens":9,"completion_tokens":2,"cost":0.100000000000000001},"choices":[{"finish_reason":"stop","message":{"role":"assistant","content":"OK"}}]}"#,
            Some("0.100000000000000001"),
            true,
        ),
        (
            TextProviderProfile::OpenRouter,
            429,
            r#"{"usage":{"prompt_tokens":9,"completion_tokens":2,"cost":0},"error":{"message":"synthetic rate limit"}}"#,
            Some("0"),
            false,
        ),
        (
            TextProviderProfile::Generic,
            200,
            r#"{"model":"actual-model","usage":{"prompt_tokens":9,"completion_tokens":2,"cost":0.1},"choices":[{"finish_reason":"stop","message":{"role":"assistant","content":"OK"}}]}"#,
            None,
            true,
        ),
        (
            TextProviderProfile::OpenRouter,
            200,
            "not-json",
            None,
            false,
        ),
    ] {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!(
            "http://{}/v1/chat/completions",
            listener.local_addr().unwrap()
        );
        let server = tokio::spawn(async move {
            let (mut stream, _) =
                tokio::time::timeout(std::time::Duration::from_secs(3), listener.accept())
                    .await
                    .unwrap()
                    .unwrap();
            let mut request = Vec::new();
            let mut buffer = [0u8; 4096];
            loop {
                let size = stream.read(&mut buffer).await.unwrap();
                assert_ne!(size, 0);
                request.extend_from_slice(&buffer[..size]);
                if let Some(end) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n") {
                    let head = String::from_utf8_lossy(&request[..end]).to_ascii_lowercase();
                    let length = head
                        .lines()
                        .find_map(|line| {
                            line.strip_prefix("content-length:")
                                .map(|n| n.trim().parse::<usize>().unwrap())
                        })
                        .unwrap();
                    if request.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            let end = request
                .windows(4)
                .position(|bytes| bytes == b"\r\n\r\n")
                .unwrap()
                + 4;
            let request: serde_json::Value = serde_json::from_slice(&request[end..]).unwrap();
            assert_eq!(request["model"], "test-model");
            stream.write_all(format!("HTTP/1.1 {status} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{content}",content.len()).as_bytes()).await.unwrap();
            stream.shutdown().await.unwrap();
            assert!(
                tokio::time::timeout(std::time::Duration::from_millis(80), listener.accept())
                    .await
                    .is_err(),
                "test never automatically retries"
            );
        });
        let store = ActivityStore::memory().unwrap();
        let meter = store.clone();
        let processor = workflow::text_processor(TextConfiguration {
            transport: TextTransport::OpenAiChat,
            authentication: TextAuthentication::None,
            endpoint,
            model: "test-model".into(),
            provider_profile: profile,
            timeout_ms: 2000,
            observer_wrapper: Some(Arc::new(move |parent| {
                meter.observer("explicit-probe".into(), parent)
            })),
            ..Default::default()
        });
        let result = processor
            .generate_observed(
                TextRequest {
                    transcript: "Reply OK".into(),
                    glossary: vec![],
                    recognition_mode: None,
                    scenario: notype_core::Scenario::Dictation,
                    translation_target: None,
                    source_context: Default::default(),
                    back_translation: false,
                },
                CancellationToken::new(),
                Arc::new(ProbeObserver),
            )
            .await;
        assert_eq!(result.is_ok(), success);
        let summary = store.summary("explicit-probe").unwrap();
        assert_eq!(summary.call_count, Some(1));
        if content != "not-json" {
            assert_eq!(summary.input.total, Some(9));
            assert_eq!(summary.output.total, Some(2));
        }
        match expected_amount {
            Some(amount) => {
                assert_eq!(summary.fees.status, Completeness::Complete);
                assert_eq!(summary.fees.totals[0].amount, amount);
                assert_eq!(summary.fees.totals[0].currency, "USD");
            }
            None => {
                assert_eq!(summary.fees.status, Completeness::Unknown);
                assert!(summary.fees.totals.is_empty());
            }
        }
        server.await.unwrap();
    }
}
