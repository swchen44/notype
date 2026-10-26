//! Actual experiment wire through the production resolver/provider/SQLite seam.
use super::*;
use workflow::experiment::*;
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/provider_config.rs"]
mod provider_config;
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
#[tokio::test]
async fn sealed_openrouter_sends_only_the_reviewed_zero_price_google_route() {
    use provider_config::*;
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
        let response=json!({"model":"google/gemma-4-31b-it","usage":{"prompt_tokens":5,"completion_tokens":2,"total_tokens":7,"cost":0},"choices":[{"finish_reason":"stop","message":{"role":"assistant","content":"OK"}}]}).to_string();
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
    let processor = experiment_processor(
        store.resolve_active(&NoHome).unwrap(),
        ExperimentPolicy::OwnedLoopback(
            owned,
            ExperimentKind::OpenRouterGemma31bFreeGoogleAiStudio,
            AdverseLatch::new(),
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
    assert_eq!(result.result.as_deref(), Some("OK"));
    assert_eq!(result.api_summary.call_count, Some(1));
    assert_eq!(
        result.api_summary.fees.totals.len(),
        0,
        "zero validation must not invent an actual-money receipt"
    );
    let body = server.await.unwrap();
    assert_eq!(
        body["provider"],
        json!({"only":["google-ai-studio"],"allow_fallbacks":false,"require_parameters":true,"max_price":{"prompt":0,"completion":0,"request":0}})
    );
    assert_eq!(body["reasoning"], json!({"enabled":false}));
}
