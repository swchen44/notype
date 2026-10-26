#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/capture_startup.rs"]
mod capture_startup;
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/diagnostic_files.rs"]
mod diagnostic_files;
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/diagnostics.rs"]
mod diagnostics;
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/provider_config.rs"]
mod provider_config;
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/provider_models.rs"]
mod provider_models;
#[allow(dead_code, unused_imports)]
#[path = "../../desktop/src-tauri/src/workflow.rs"]
mod workflow;
use provider_config::*;
use provider_models::*;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::oneshot,
};
#[derive(Default)]
struct Storage(Mutex<Option<Vec<u8>>>);
impl ConfigurationStorage for Storage {
    fn load(&self) -> Result<Option<Vec<u8>>, ConfigError> {
        Ok(self.0.lock().unwrap().clone())
    }
    fn commit(&self, bytes: &[u8]) -> Result<(), ConfigError> {
        *self.0.lock().unwrap() = Some(bytes.to_vec());
        Ok(())
    }
}
struct NeverHome;
impl credentials::CredentialReader for NeverHome {
    fn read(&self, _: credentials::PublicKey) -> Result<Option<String>, ConfigError> {
        panic!("local/App-key model query must not read home")
    }
}
fn configured(base: String) -> (Arc<ConnectionStore>, ConnectionDraft, u64) {
    let store = Arc::new(ConnectionStore::open(Arc::new(Storage::default())).unwrap());
    let mut draft = store
        .create(
            0,
            "Owned local".into(),
            ConnectionKind::Local,
            ProviderDialect::Generic,
        )
        .unwrap()
        .connections
        .remove(0)
        .connection;
    draft.api_address = base;
    draft.authentication = Authentication::Bearer;
    let view = store
        .save(
            1,
            draft.clone(),
            KeyAction::Replace {
                value: "OWNED_KEY_SENTINEL".into(),
            },
        )
        .unwrap();
    (store, draft, view.revision)
}
fn request(draft: &ConnectionDraft, revision: u64, id: &str) -> ModelListRequest {
    ModelListRequest {
        request_id: id.into(),
        expected_revision: revision,
        connection: draft.clone(),
        key_action: KeyAction::Keep,
    }
}
async fn fixture(
    wire: String,
    release: Option<oneshot::Receiver<()>>,
) -> (
    String,
    oneshot::Receiver<String>,
    tokio::task::JoinHandle<()>,
) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}/v1", listener.local_addr().unwrap());
    let (sender, receiver) = oneshot::channel();
    let task = tokio::spawn(async move {
        let (mut stream, _) = tokio::time::timeout(Duration::from_secs(3), listener.accept())
            .await
            .unwrap()
            .unwrap();
        let mut bytes = vec![];
        let mut chunk = [0; 4096];
        loop {
            let count = stream.read(&mut chunk).await.unwrap();
            assert!(count > 0);
            bytes.extend_from_slice(&chunk[..count]);
            if bytes.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
                break;
            }
        }
        let _ = sender.send(String::from_utf8(bytes).unwrap());
        if let Some(release) = release {
            let _ = release.await;
        }
        let _ = stream.write_all(wire.as_bytes()).await;
        drop(stream);
        assert!(
            tokio::time::timeout(Duration::from_millis(75), listener.accept())
                .await
                .is_err(),
            "unexpected implicit GET/retry"
        );
    });
    (base, receiver, task)
}
fn wire(status: u16, body: &str, extra: &str) -> String {
    format!(
        "HTTP/1.1 {status} Fixture\r\nContent-Length: {}\r\nConnection: close\r\n{extra}\r\n{body}",
        body.len()
    )
}

#[tokio::test]
async fn explicit_get_uses_only_bound_auth_and_returns_reference_ids_without_mutating_model_or_active(
) {
    let body = r#"{"data":[{"id":"z-model","description":"OWNED_KEY_SENTINEL"},{"id":"a-model"},{"id":"z-model"}]}"#;
    let (base, received, server) = fixture(wire(200, body, ""), None).await;
    let (store, draft, revision) = configured(base);
    let service = ModelDiscovery::new();
    let result = service
        .query(
            store.clone(),
            request(&draft, revision, "first"),
            &NeverHome,
        )
        .await
        .unwrap();
    let http = received.await.unwrap();
    server.await.unwrap();
    assert!(http.starts_with("GET /v1/models HTTP/1.1"));
    assert!(http
        .to_ascii_lowercase()
        .contains("authorization: bearer owned_key_sentinel"));
    assert!(!http.contains("transcript"));
    assert_eq!(result.status, "ok");
    assert_eq!(
        result
            .models
            .iter()
            .map(|row| row.id.as_str())
            .collect::<Vec<_>>(),
        vec!["a-model", "z-model"]
    );
    assert!(!serde_json::to_string(&result)
        .unwrap()
        .contains("OWNED_KEY_SENTINEL"));
    let view = store.view().unwrap();
    assert_eq!(view.revision, revision);
    assert_eq!(view.connections[0].connection.model, "");
    assert_eq!(view.active_connection_id, None);
    let throttled = service
        .query(store, request(&draft, revision, "second"), &NeverHome)
        .await
        .unwrap();
    assert_eq!(throttled.status, "throttled");
    assert!(throttled.retry_after_seconds.unwrap() <= 60);
}

#[tokio::test]
async fn real_http_status_schema_private_reflection_and_body_limits_are_safe() {
    let oversized = "x".repeat(2_097_153);
    let models_overflow=serde_json::json!({"data":(0..2001).map(|index|serde_json::json!({"id":format!("model-{index}")})).collect::<Vec<_>>()}).to_string();
    for (status, body, extra, expected_status, expected_error) in [
        (
            401,
            "<html>Bearer provider-secret</html>",
            "",
            "error",
            Some("provider-auth"),
        ),
        (
            429,
            "not JSON",
            "Retry-After: 7\r\n",
            "error",
            Some("provider-rate-limit"),
        ),
        (
            503,
            "<html>unavailable</html>",
            "",
            "error",
            Some("provider-unavailable"),
        ),
        (504, "", "", "error", Some("provider-timeout")),
        (
            404,
            "<html>not supported</html>",
            "",
            "unsupported",
            Some("model-list-unsupported"),
        ),
        (405, "", "", "unsupported", Some("model-list-unsupported")),
        (501, "", "", "unsupported", Some("model-list-unsupported")),
        (
            200,
            r#"{"error":{"code":401,"message":"provider-secret"}}"#,
            "",
            "error",
            Some("provider-auth"),
        ),
        (
            200,
            "<html>not json</html>",
            "",
            "error",
            Some("llm-invalid-response"),
        ),
        (
            200,
            r#"{"models":[]}"#,
            "",
            "unsupported",
            Some("model-list-schema-unavailable"),
        ),
        (200, r#"{"data":[]}"#, "", "ok", None),
        (
            200,
            r#"{"data":[{"id":"prefix-OWNED_KEY_SENTINEL-suffix"}]}"#,
            "",
            "error",
            Some("model-list-private-response"),
        ),
        (
            200,
            oversized.as_str(),
            "",
            "error",
            Some("model-list-response-limit"),
        ),
        (
            200,
            models_overflow.as_str(),
            "",
            "error",
            Some("model-list-response-limit"),
        ),
    ] {
        let (base, received, server) = fixture(wire(status, body, extra), None).await;
        let (store, draft, revision) = configured(base);
        let result = ModelDiscovery::new()
            .query(store, request(&draft, revision, "case"), &NeverHome)
            .await
            .unwrap();
        received.await.unwrap();
        server.await.unwrap();
        assert_eq!(result.status, expected_status, "HTTP {status}");
        assert_eq!(result.error_code.as_deref(), expected_error);
        if status == 429 {
            assert_eq!(result.retry_after_seconds, Some(7));
        }
        assert!(!serde_json::to_string(&result)
            .unwrap()
            .contains("provider-secret"));
    }
}

#[tokio::test]
async fn cancellation_and_collection_edits_fence_late_replies_and_inflight_clicks() {
    for change in ["cancel", "remove", "prompt"] {
        let (release, held) = oneshot::channel();
        let (base, received, server) = fixture(
            wire(200, r#"{"data":[{"id":"late-model"}]}"#, ""),
            Some(held),
        )
        .await;
        let (store, draft, revision) = configured(base);
        let service = Arc::new(ModelDiscovery::new());
        let worker_service = service.clone();
        let worker_store = store.clone();
        let worker_request = request(&draft, revision, "held");
        let task = tokio::spawn(async move {
            worker_service
                .query(worker_store, worker_request, &NeverHome)
                .await
        });
        received.await.unwrap();
        let duplicate = service
            .query(
                store.clone(),
                request(&draft, revision, "duplicate"),
                &NeverHome,
            )
            .await
            .unwrap();
        assert_eq!(
            duplicate.error_code.as_deref(),
            Some("model-list-in-flight")
        );
        if change == "cancel" {
            service.cancel("held");
        } else if change == "remove" {
            store.remove(revision, &draft.id).unwrap();
        } else {
            store
                .prompt(
                    revision,
                    notype_core::Scenario::Ask,
                    Some("changed while GET is held".into()),
                )
                .unwrap();
        }
        release.send(()).unwrap();
        let result = task.await.unwrap().unwrap();
        server.await.unwrap();
        assert_eq!(
            result.status,
            if change == "cancel" {
                "cancelled"
            } else {
                "stale"
            }
        );
        assert!(result.models.is_empty());
        assert_eq!(store.view().unwrap().active_connection_id, None);
        if change == "prompt" {
            let blocked = service
                .query(
                    store.clone(),
                    request(&draft, store.view().unwrap().revision, "after-edit"),
                    &NeverHome,
                )
                .await
                .unwrap();
            assert_eq!(blocked.status, "throttled");
            assert_eq!(blocked.error_code.as_deref(), Some("model-list-cooldown"));
        }
    }
}

#[tokio::test]
async fn redirects_truncated_transfer_and_unrelated_list_routes_never_forward_credentials() {
    let destination = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let redirect = wire(
        302,
        "",
        &format!(
            "Location: http://{}/other\r\n",
            destination.local_addr().unwrap()
        ),
    );
    for (raw, expected) in [
        (redirect, "provider-protocol"),
        (String::new(), "provider-transport"),
        (
            "HTTP/1.1 200 OK\r\nContent-Length: 500\r\nConnection: close\r\n\r\n{\"data\":".into(),
            "provider-transport",
        ),
    ] {
        let (base, received, server) = fixture(raw, None).await;
        let (store, draft, revision) = configured(base);
        let result = ModelDiscovery::new()
            .query(store, request(&draft, revision, "transport"), &NeverHome)
            .await
            .unwrap();
        received.await.unwrap();
        server.await.unwrap();
        assert_eq!(result.error_code.as_deref(), Some(expected));
    }
    assert!(
        tokio::time::timeout(Duration::from_millis(30), destination.accept())
            .await
            .is_err()
    );
    let (store, mut draft, revision) = configured("http://127.0.0.1:1234/v1".into());
    draft.address_mode = AddressMode::FullChatEndpoint;
    draft.api_address = "http://127.0.0.1:1234/v1/chat/completions".into();
    draft.models_endpoint = Some(format!(
        "http://{}/models",
        destination.local_addr().unwrap()
    ));
    assert_eq!(
        ModelDiscovery::new()
            .query(store, request(&draft, revision, "unbound"), &NeverHome)
            .await
            .err()
            .unwrap()
            .0,
        "configuration-model-list-binding-invalid"
    );
    assert!(
        tokio::time::timeout(Duration::from_millis(30), destination.accept())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn cooldown_uses_the_production_monotonic_clock_without_waiting_or_automatic_refresh() {
    struct Clock(Mutex<std::time::Instant>);
    impl capture_startup::CaptureClock for Clock {
        fn now(&self) -> std::time::Instant {
            *self.0.lock().unwrap()
        }
    }
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}/v1", listener.local_addr().unwrap());
    let count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let accepted = count.clone();
    let server = tokio::spawn(async move {
        for _ in 0..2 {
            let (mut stream, _) = tokio::time::timeout(Duration::from_secs(3), listener.accept())
                .await
                .unwrap()
                .unwrap();
            let mut bytes = vec![];
            let mut buffer = [0; 1024];
            while !bytes.windows(4).any(|part| part == b"\r\n\r\n") {
                let n = stream.read(&mut buffer).await.unwrap();
                assert!(n > 0);
                bytes.extend_from_slice(&buffer[..n]);
            }
            assert!(bytes.starts_with(b"GET /v1/models "));
            accepted.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            stream
                .write_all(wire(200, r#"{"data":[{"id":"clock-model"}]}"#, "").as_bytes())
                .await
                .unwrap();
        }
        assert!(
            tokio::time::timeout(Duration::from_millis(75), listener.accept())
                .await
                .is_err()
        );
    });
    let (store, draft, revision) = configured(base);
    let clock = Arc::new(Clock(Mutex::new(std::time::Instant::now())));
    let discovery = ModelDiscovery::with_clock(clock.clone());
    assert_eq!(
        discovery
            .query(store.clone(), request(&draft, revision, "one"), &NeverHome)
            .await
            .unwrap()
            .status,
        "ok"
    );
    *clock.0.lock().unwrap() += Duration::from_secs(59);
    assert_eq!(
        discovery
            .query(
                store.clone(),
                request(&draft, revision, "blocked"),
                &NeverHome
            )
            .await
            .unwrap()
            .status,
        "throttled"
    );
    assert_eq!(count.load(std::sync::atomic::Ordering::SeqCst), 1);
    *clock.0.lock().unwrap() += Duration::from_secs(2);
    assert_eq!(
        discovery
            .query(store, request(&draft, revision, "two"), &NeverHome)
            .await
            .unwrap()
            .status,
        "ok"
    );
    server.await.unwrap();
    assert_eq!(count.load(std::sync::atomic::Ordering::SeqCst), 2);
}

async fn shared_route_credential_cooldown(status: u16, mutate_globals: bool) {
    struct Clock(Mutex<std::time::Instant>);
    impl capture_startup::CaptureClock for Clock {
        fn now(&self) -> std::time::Instant {
            *self.0.lock().unwrap()
        }
    }
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}/v1", listener.local_addr().unwrap());
    let count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let accepted = count.clone();
    let server = tokio::spawn(async move {
        let mut requests = vec![];
        for index in 0..4 {
            let (mut stream, _) = tokio::time::timeout(Duration::from_secs(3), listener.accept())
                .await
                .unwrap()
                .unwrap();
            let mut bytes = vec![];
            let mut chunk = [0; 4096];
            while !bytes.windows(4).any(|part| part == b"\r\n\r\n") {
                let n = stream.read(&mut chunk).await.unwrap();
                assert!(n > 0);
                bytes.extend_from_slice(&chunk[..n]);
            }
            requests.push(String::from_utf8(bytes).unwrap());
            accepted.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let response = if index == 0 {
                wire(
                    status,
                    r#"{"data":[{"id":"owned-model"}]}"#,
                    if status == 429 {
                        "Retry-After: 120\r\n"
                    } else {
                        ""
                    },
                )
            } else {
                wire(200, r#"{"data":[{"id":"owned-model"}]}"#, "")
            };
            stream.write_all(response.as_bytes()).await.unwrap();
        }
        assert!(
            tokio::time::timeout(Duration::from_millis(75), listener.accept())
                .await
                .is_err()
        );
        requests
    });
    let (store, draft, revision) = configured(base.clone());
    let view = store
        .create(
            revision,
            "Other record".into(),
            ConnectionKind::Local,
            ProviderDialect::Generic,
        )
        .unwrap();
    let mut other = draft.clone();
    other.id = view.connections[1].connection.id.clone();
    other.display_name = "Other record".into();
    let view = store
        .save(
            view.revision,
            other.clone(),
            KeyAction::Replace {
                value: "OWNED_KEY_SENTINEL".into(),
            },
        )
        .unwrap();
    let clock = Arc::new(Clock(Mutex::new(std::time::Instant::now())));
    let discovery = ModelDiscovery::with_clock(clock.clone());
    let first = discovery
        .query(
            store.clone(),
            request(&draft, view.revision, "first"),
            &NeverHome,
        )
        .await
        .unwrap();
    assert_eq!(first.status, if status == 200 { "ok" } else { "error" });
    let mut revision = view.revision;
    if mutate_globals {
        let view = store
            .prompt(
                revision,
                notype_core::Scenario::Ask,
                Some("unrelated prompt".into()),
            )
            .unwrap();
        other.display_name = "Renamed unrelated record".into();
        let view = store
            .save(view.revision, other.clone(), KeyAction::Keep)
            .unwrap();
        let view = store
            .activate(view.revision, Some(other.id.clone()))
            .unwrap();
        let view = store
            .create(
                view.revision,
                "New unrelated".into(),
                ConnectionKind::Custom,
                ProviderDialect::Generic,
            )
            .unwrap();
        revision = view.revision;
    }
    let same = if mutate_globals { &draft } else { &other };
    let blocked = discovery
        .query(
            store.clone(),
            request(same, revision, "blocked"),
            &NeverHome,
        )
        .await
        .unwrap();
    assert_eq!(blocked.status, "throttled");
    assert_eq!(blocked.error_code.as_deref(), Some("model-list-cooldown"));
    assert_eq!(count.load(std::sync::atomic::Ordering::SeqCst), 1);
    let mut different_key = request(&draft, revision, "different-key");
    different_key.key_action = KeyAction::Replace {
        value: "DIFFERENT_OWNED_KEY".into(),
    };
    assert_eq!(
        discovery
            .query(store.clone(), different_key, &NeverHome)
            .await
            .unwrap()
            .status,
        "ok"
    );
    let mut different_route = request(&draft, revision, "different-route");
    different_route.connection.api_address = format!("{base}/other");
    different_route.key_action = KeyAction::Replace {
        value: "OWNED_KEY_SENTINEL".into(),
    };
    assert_eq!(
        discovery
            .query(store.clone(), different_route, &NeverHome)
            .await
            .unwrap()
            .status,
        "ok"
    );
    *clock.0.lock().unwrap() += Duration::from_secs(59);
    assert_eq!(
        discovery
            .query(
                store.clone(),
                request(same, revision, "before-minimum"),
                &NeverHome
            )
            .await
            .unwrap()
            .status,
        "throttled"
    );
    *clock.0.lock().unwrap() += Duration::from_secs(2);
    if status == 429 {
        assert_eq!(
            discovery
                .query(
                    store.clone(),
                    request(same, revision, "retry-after"),
                    &NeverHome
                )
                .await
                .unwrap()
                .status,
            "throttled"
        );
        assert_eq!(count.load(std::sync::atomic::Ordering::SeqCst), 3);
        *clock.0.lock().unwrap() += Duration::from_secs(60);
    }
    assert_eq!(
        discovery
            .query(
                store.clone(),
                request(same, revision, "expired"),
                &NeverHome
            )
            .await
            .unwrap()
            .status,
        "ok"
    );
    assert_eq!(store.view().unwrap().revision, revision);
    let requests = server.await.unwrap();
    assert_eq!(count.load(std::sync::atomic::Ordering::SeqCst), 4);
    assert!(requests[1]
        .to_ascii_lowercase()
        .contains("authorization: bearer different_owned_key"));
    assert!(requests[2].starts_with("GET /v1/other/models "));
    assert!(requests[3]
        .to_ascii_lowercase()
        .contains("authorization: bearer owned_key_sentinel"));
}

#[tokio::test]
async fn unrelated_collection_edits_cannot_reset_route_credential_retry_after() {
    shared_route_credential_cooldown(429, true).await;
}

#[tokio::test]
async fn another_record_cannot_reset_the_same_route_credential_cooldown() {
    shared_route_credential_cooldown(200, false).await;
}

#[tokio::test]
async fn model_get_uses_the_real_typed_service_and_exact_owned_report_export() {
    use diagnostics::{
        api::*, service::DiagnosticService, LogFileStatus, LogLevel, Platform, ReportContext,
    };
    struct Sink(Arc<DiagnosticService>);
    impl ApiObserver for Sink {
        fn begin(
            &self,
            scope: Option<ScopeBinding>,
            revision: Option<u64>,
        ) -> Option<TraceIdentity> {
            self.0.begin_api(scope, revision)
        }
        fn record(&self, event: ApiEvent) {
            self.0.record_api(event)
        }
    }
    for (body, timeout) in [
        (
            r#"{"data":[{"id":"PRIVATE_MODEL_LIST_ID"}],"usage":{"prompt_tokens":99999}}"#,
            false,
        ),
        (
            r#"{"error":{"code":504,"message":"PRIVATE_ERROR_BODY"}}"#,
            true,
        ),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let diagnostic = Arc::new(DiagnosticService::new());
        diagnostic.initialize(Arc::new(diagnostic_files::FileStorage::new(Some(
            directory.path().join("diagnostics.json"),
        ))));
        let (base, received, server) = fixture(wire(200, body, ""), None).await;
        let (store, draft, revision) = configured(base);
        let binding = diagnostic.api_connection_scope(None, Some(&draft.id));
        let context = TraceContext::new(
            binding,
            Arc::new(capture_startup::MonotonicClock),
            Arc::new(Sink(diagnostic.clone())),
        );
        let discovery = ModelDiscovery::with_diagnostics(
            Arc::new(capture_startup::MonotonicClock),
            Some(context),
        );
        let result = discovery
            .query(store, request(&draft, revision, "report"), &NeverHome)
            .await
            .unwrap();
        received.await.unwrap();
        server.await.unwrap();
        if !timeout {
            assert_eq!(result.models[0].id, "PRIVATE_MODEL_LIST_ID");
        }
        let preview = diagnostic
            .publish_preview(
                diagnostic.begin_preview().unwrap(),
                ReportContext {
                    version: "0.1.0",
                    platform: Platform::Linux,
                    log_level: LogLevel::Debug,
                    debug_mode: false,
                    always_on_microphone: false,
                    model_selected: false,
                    model_available: false,
                    model_is_custom: false,
                    local_log: LogFileStatus::Empty,
                },
            )
            .unwrap();
        let text = preview.report.text();
        assert!(text.contains("purpose=model_list method=get"));
        assert!(text.contains("durable_admission=false"));
        assert!(text.contains("http_status=200"));
        if timeout {
            assert!(text.contains("timeout_source=provider_status"));
        } else {
            assert!(text.contains("model_list_count=Some(1)"));
        }
        for secret in [
            "OWNED_KEY_SENTINEL",
            "PRIVATE_MODEL_LIST_ID",
            "PRIVATE_ERROR_BODY",
            draft.id.as_str(),
        ] {
            assert!(!text.contains(secret));
        }
        let ticket = diagnostic.begin_export(preview.report.id()).unwrap();
        let destination = directory.path().join("models.txt");
        diagnostic.commit_export(&ticket, &destination).unwrap();
        assert_eq!(std::fs::read_to_string(destination).unwrap(), text);
    }
}

#[tokio::test]
async fn model_get_real_whole_deadline_bounds_stalled_headers_and_partial_body() {
    async fn run(partial_body: bool) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}/v1", listener.local_addr().unwrap());
        let (release, held) = oneshot::channel();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut bytes = vec![];
            let mut buffer = [0; 1024];
            while !bytes.windows(4).any(|part| part == b"\r\n\r\n") {
                let n = stream.read(&mut buffer).await.unwrap();
                assert!(n > 0);
                bytes.extend_from_slice(&buffer[..n]);
            }
            if partial_body {
                stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 1000\r\nConnection: close\r\n\r\n{\"data\":").await.unwrap();
            }
            let _ = held.await;
        });
        let (store, draft, revision) = configured(base);
        let result = tokio::time::timeout(
            Duration::from_secs(15),
            ModelDiscovery::new().query(store, request(&draft, revision, "deadline"), &NeverHome),
        )
        .await
        .unwrap()
        .unwrap();
        let _ = release.send(());
        server.await.unwrap();
        assert_eq!(result.error_code.as_deref(), Some("model-list-timeout"));
        assert!(result.models.is_empty());
    }
    tokio::join!(run(false), run(true));
}
