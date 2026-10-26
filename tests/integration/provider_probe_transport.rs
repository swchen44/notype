//! Actual experiment wire through the production resolver/provider/SQLite seam.
use super::provider_config;
use super::*;
use workflow::experiment::*;
struct Memory;
impl provider_config::ConfigurationStorage for Memory {
    fn load(&self) -> Result<Option<Vec<u8>>, provider_config::ConfigError> {
        Ok(None)
    }
    fn commit(&self, _: &[u8]) -> Result<(), provider_config::ConfigError> {
        Ok(())
    }
}
struct NoHome;
impl provider_config::credentials::CredentialReader for NoHome {
    fn read(
        &self,
        _: provider_config::credentials::PublicKey,
    ) -> Result<Option<String>, provider_config::ConfigError> {
        panic!("fixture never reads real home")
    }
}
struct ObservedOnly(ExperimentProcessor);
impl notype_core::TextProcessor for ObservedOnly {
    fn polish(
        &self,
        _: notype_core::TextRequest,
        _: notype_core::CancellationToken,
    ) -> notype_core::TextFuture {
        Box::pin(async { Err(notype_core::TextFailure::HistoryStorage) })
    }
    fn generate_observed(
        &self,
        request: notype_core::TextRequest,
        cancel: notype_core::CancellationToken,
        observer: Arc<dyn notype_core::observations::ApiCallObserver>,
    ) -> notype_core::GenerationFuture {
        self.0.generate_observed(request, cancel, observer)
    }
}
fn good_reply() -> Value {
    json!({"model":"google/gemma-4-31b-it","usage":{"prompt_tokens":5,"completion_tokens":2,"total_tokens":7,"cost":0},"choices":[{"finish_reason":"stop","message":{"role":"assistant","content":"OK"}}]})
}
fn openrouter_configuration(owned: &OwnedLoopback) -> TextConfiguration {
    use provider_config::*;
    let store = ConnectionStore::open(Arc::new(Memory)).unwrap();
    let mut draft = store
        .create(
            0,
            "Owned fixture".into(),
            ConnectionKind::Local,
            ProviderDialect::Openrouter,
        )
        .unwrap()
        .connections
        .remove(0)
        .connection;
    draft.api_address = owned.endpoint();
    draft.address_mode = AddressMode::FullChatEndpoint;
    draft.model = "google/gemma-4-31b-it:free".into();
    draft.max_output_tokens = Some(512);
    let saved = store.save(1, draft.clone(), KeyAction::Clear).unwrap();
    store.activate(saved.revision, Some(draft.id)).unwrap();
    store.resolve_active(&NoHome).unwrap()
}
async fn owned_openrouter_response(
    reply: Value,
) -> (notype_core::SessionEntry, Value, AdverseLatch) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let owned = OwnedLoopback::from_listener(&listener).unwrap();
    listener.set_nonblocking(true).unwrap();
    let listener = TcpListener::from_std(listener).unwrap();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut bytes = vec![];
        let mut buffer = [0; 8192];
        let (end, length) = loop {
            let n = stream.read(&mut buffer).await.unwrap();
            assert!(n > 0);
            bytes.extend_from_slice(&buffer[..n]);
            if let Some(end) = bytes.windows(4).position(|b| b == b"\r\n\r\n") {
                let header = std::str::from_utf8(&bytes[..end]).unwrap();
                let length = header
                    .lines()
                    .find_map(|line| {
                        line.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .map(|v| v.trim().parse::<usize>().unwrap())
                    })
                    .unwrap();
                break (end + 4, length);
            }
        };
        while bytes.len() < end + length {
            let n = stream.read(&mut buffer).await.unwrap();
            assert!(n > 0);
            bytes.extend_from_slice(&buffer[..n]);
        }
        let body: Value = serde_json::from_slice(&bytes[end..end + length]).unwrap();
        let response = reply.to_string();
        stream
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    response.len(),
                    response
                )
                .as_bytes(),
            )
            .await
            .unwrap();
        body
    });
    let latch = AdverseLatch::new();
    let processor = experiment_processor(
        openrouter_configuration(&owned),
        ExperimentPolicy::OwnedLoopback(
            owned,
            ExperimentKind::OpenRouterGemma31bFreeGoogleAiStudio,
            latch.clone(),
        ),
        diagnostics::api::TraceContext::global(None),
    )
    .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let sessions = SessionService::open(directory.path().join("sessions.db")).unwrap();
    let entry = sessions
        .create("only OK", SessionOptions::default())
        .unwrap();
    let result = sessions
        .process(&entry.id, vec![], Arc::new(ObservedOnly(processor)))
        .await
        .unwrap();
    (result, server.await.unwrap(), latch)
}

async fn adverse_headers_case(off_before: bool) {
    use diagnostics::{api::*, service::*};
    struct Storage(bool);
    impl DiagnosticStorage for Storage {
        fn load_preference(&self) -> Result<Option<bool>, &'static str> {
            Ok(Some(!self.0))
        }
        fn save_preference(&self, _: bool) -> Result<(), &'static str> {
            Ok(())
        }
        fn write_report(&self, _: &std::path::Path, _: &str) -> Result<(), &'static str> {
            Ok(())
        }
    }
    struct ObservedHeaders {
        service: Arc<DiagnosticService>,
        seen: std::sync::Mutex<Option<oneshot::Sender<u16>>>,
    }
    impl ApiObserver for ObservedHeaders {
        fn begin(
            &self,
            scope: Option<ScopeBinding>,
            revision: Option<u64>,
        ) -> Option<TraceIdentity> {
            self.service.begin_api(scope, revision)
        }
        fn record(&self, event: ApiEvent) {
            if event.span == ApiSpan::HeadersReceived {
                if let Some(sender) = self.seen.lock().unwrap().take() {
                    sender.send(event.http_status.unwrap()).unwrap();
                }
            }
            self.service.record_api(event);
        }
    }
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let owned = OwnedLoopback::from_listener(&listener).unwrap();
    listener.set_nonblocking(true).unwrap();
    let listener = TcpListener::from_std(listener).unwrap();
    let (release, held) = oneshot::channel();
    let (written, headers_written) = oneshot::channel();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut bytes = vec![];
        let mut buffer = [0; 8192];
        while !bytes.windows(4).any(|p| p == b"\r\n\r\n") {
            let n = stream.read(&mut buffer).await.unwrap();
            assert!(n > 0);
            bytes.extend_from_slice(&buffer[..n]);
        }
        stream.write_all(b"HTTP/1.1 429 Too Many Requests\r\nContent-Length: 9999\r\nConnection: close\r\n\r\n{\"error\":").await.unwrap();
        written.send(()).unwrap();
        let _ = held.await;
    });
    let service = Arc::new(DiagnosticService::new());
    service.initialize(Arc::new(Storage(off_before)));
    let (sender, seen) = oneshot::channel();
    let trace = TraceContext::new(
        None,
        Arc::new(capture_startup::MonotonicClock),
        Arc::new(ObservedHeaders {
            service: service.clone(),
            seen: std::sync::Mutex::new(Some(sender)),
        }),
    );
    let latch = AdverseLatch::new();
    let processor = experiment_processor(
        openrouter_configuration(&owned),
        ExperimentPolicy::OwnedLoopback(
            owned.clone(),
            ExperimentKind::OpenRouterGemma31bFreeGoogleAiStudio,
            latch.clone(),
        ),
        trace,
    )
    .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let sessions = SessionService::open(directory.path().join("sessions.db")).unwrap();
    let entry = sessions
        .create("owned cancellation original", SessionOptions::default())
        .unwrap();
    let worker = sessions.clone();
    let id = entry.id.clone();
    let task = tokio::spawn(async move {
        worker
            .process(&id, vec![], Arc::new(ObservedOnly(processor)))
            .await
    });
    if off_before {
        assert!(!service.status().effective_enabled);
        tokio::time::timeout(Duration::from_secs(2), headers_written)
            .await
            .unwrap()
            .unwrap();
        let _ = tokio::time::timeout(Duration::from_secs(2), async {
            while latch.observed().is_none() {
                tokio::task::yield_now().await;
            }
        })
        .await;
    } else {
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(2), seen)
                .await
                .unwrap()
                .unwrap(),
            429
        );
    }
    let observed = latch.observed();
    service.clear();
    service.set_enabled(false);
    sessions.cancel(&entry.id).unwrap();
    let result = task.await.unwrap().unwrap();
    release.send(()).unwrap();
    server.await.unwrap();
    assert_eq!(
        observed,
        Some(AdverseSignal {
            category: AdverseCategory::Http,
            http_status: Some(429)
        })
    );
    assert_eq!(latch.observed(), observed);
    assert_eq!(result.transcript, "owned cancellation original");
    assert_eq!(result.api_summary.call_count, Some(1));
    assert!(result.api_summary.input.total.is_none());
    assert!(experiment_processor(
        openrouter_configuration(&owned),
        ExperimentPolicy::OwnedLoopback(
            owned,
            ExperimentKind::OpenRouterGemma31bFreeGoogleAiStudio,
            latch
        ),
        TraceContext::global(None)
    )
    .is_err());
}

#[tokio::test]
async fn adverse_http_headers_survive_stalled_body_cancellation_and_diagnostics_clear_off() {
    adverse_headers_case(false).await;
}
#[tokio::test]
async fn adverse_http_headers_are_recorded_with_diagnostics_off_before_request_allocation() {
    adverse_headers_case(true).await;
}
#[tokio::test]
async fn sealed_openrouter_sends_only_the_reviewed_zero_price_google_route() {
    let (result, body, latch) = owned_openrouter_response(good_reply()).await;
    assert_eq!(result.result.as_deref(), Some("OK"));
    assert!(latch.observed().is_none());
    assert_eq!(result.api_summary.call_count, Some(1));
    assert_eq!(
        result.api_summary.fees.totals.len(),
        0,
        "zero validation must not invent an actual-money receipt"
    );
    assert_eq!(
        body["provider"],
        json!({"only":["google-ai-studio"],"allow_fallbacks":false,"require_parameters":true,"max_price":{"prompt":0,"completion":0,"request":0}})
    );
    assert_eq!(body["reasoning"], json!({"enabled":false}));
}

#[tokio::test]
async fn sealed_openrouter_requires_numeric_zero_cost_without_discarding_actual_usage() {
    for cost in [
        None,
        Some(Value::Null),
        Some(json!("0")),
        Some(json!(0.001)),
        Some(json!(-1)),
    ] {
        let mut reply = good_reply();
        if let Some(cost) = cost {
            reply["usage"]["cost"] = cost;
        } else {
            reply["usage"].as_object_mut().unwrap().remove("cost");
        }
        let (result, _, latch) = owned_openrouter_response(reply).await;
        assert_eq!(result.error.as_deref(), Some("llm-invalid-response"));
        assert!(result.result.is_none());
        assert_eq!(result.api_summary.input.total, Some(5));
        assert_eq!(result.api_summary.output.total, Some(2));
        assert!(result.api_summary.fees.totals.is_empty());
        assert_eq!(
            latch.observed().unwrap().category,
            AdverseCategory::Contract
        );
    }
    let mut reply = good_reply();
    reply["usage"]["cost"] = json!(0.0);
    let (result, _, latch) = owned_openrouter_response(reply).await;
    assert_eq!(result.result.as_deref(), Some("OK"));
    assert!(latch.observed().is_none());
}
