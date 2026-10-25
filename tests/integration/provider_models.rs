#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/capture_startup.rs"]
mod capture_startup;
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
    for cancel in [true, false] {
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
        if cancel {
            service.cancel("held");
        } else {
            store.remove(revision, &draft.id).unwrap();
        }
        release.send(()).unwrap();
        let result = task.await.unwrap().unwrap();
        server.await.unwrap();
        assert_eq!(result.status, if cancel { "cancelled" } else { "stale" });
        assert!(result.models.is_empty());
        assert_eq!(store.view().unwrap().active_connection_id, None);
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
