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
#[allow(dead_code, unused_imports)]
#[path = "../../desktop/src-tauri/src/workflow.rs"]
mod workflow;
use diagnostics::{
    api::*, service::DiagnosticService, LogFileStatus, LogLevel, Platform, ReportContext,
};
use notype_core::{Scenario, SessionOptions, SessionService};
use provider_config::*;
use serde_json::{json, Value};
use std::{sync::Arc, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::{mpsc, oneshot},
};
struct Sink(Arc<DiagnosticService>);
impl ApiObserver for Sink {
    fn begin(&self, scope: Option<ScopeBinding>, revision: Option<u64>) -> Option<TraceIdentity> {
        self.0.begin_api(scope, revision)
    }
    fn record(&self, event: ApiEvent) {
        self.0.record_api(event)
    }
}
struct NeverHome;
impl credentials::CredentialReader for NeverHome {
    fn read(&self, _: credentials::PublicKey) -> Result<Option<String>, ConfigError> {
        panic!("owned App/anonymous fixture must not read home")
    }
}
fn context() -> ReportContext<'static> {
    ReportContext {
        version: "0.1.0",
        platform: Platform::Linux,
        log_level: LogLevel::Debug,
        debug_mode: false,
        always_on_microphone: false,
        model_selected: true,
        model_available: true,
        model_is_custom: false,
        local_log: LogFileStatus::Empty,
    }
}
fn service(directory: &std::path::Path) -> Arc<DiagnosticService> {
    let service = Arc::new(DiagnosticService::new());
    service.initialize(Arc::new(diagnostic_files::FileStorage::new(Some(
        directory.join("diagnostics.json"),
    ))));
    service
}
fn report(service: &DiagnosticService) -> String {
    service
        .publish_preview(service.begin_preview().unwrap(), context())
        .unwrap()
        .report
        .text()
        .into()
}
fn traced(
    service: &Arc<DiagnosticService>,
    id: &str,
    configuration: workflow::TextConfiguration,
) -> Arc<dyn notype_core::TextProcessor> {
    let scope = service.api_connection_scope(Some(id), configuration.connection_id.as_deref());
    workflow::text_processor_with_trace(
        configuration,
        TraceContext::new(
            scope,
            Arc::new(capture_startup::MonotonicClock),
            Arc::new(Sink(service.clone())),
        ),
    )
}
fn configure(
    directory: &std::path::Path,
    endpoint: &str,
    review: bool,
) -> (Arc<ConnectionStore>, ConnectionDraft, u64) {
    let store = Arc::new(
        ConnectionStore::open(Arc::new(files::FileStorage::new(
            directory.join("connections.json"),
        )))
        .unwrap(),
    );
    let mut draft = store
        .create(
            0,
            "PRIVATE_CONNECTION_NAME".into(),
            ConnectionKind::Local,
            ProviderDialect::Deepseek,
        )
        .unwrap()
        .connections
        .remove(0)
        .connection;
    draft.api_address = endpoint.into();
    draft.address_mode = AddressMode::FullChatEndpoint;
    draft.model = "MODEL_PRIVATE_SENTINEL".into();
    draft.authentication = Authentication::Bearer;
    draft.temperature = Some(0.0);
    draft.top_p = Some(0.5);
    draft.max_output_tokens = Some(512);
    let view = store
        .save(
            1,
            draft.clone(),
            KeyAction::Replace {
                value: "OWNED_KEY_SENTINEL".into(),
            },
        )
        .unwrap();
    let active = store
        .activate(view.revision, Some(draft.id.clone()))
        .unwrap();
    let mut translation = active.translation;
    translation.back_translation = review;
    let saved = store.translation(active.revision, translation).unwrap();
    (store, draft, saved.revision)
}
async fn fixture(
    responses: Vec<String>,
    first_release: Option<oneshot::Receiver<()>>,
) -> (
    String,
    mpsc::Receiver<(String, Value)>,
    tokio::task::JoinHandle<()>,
) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!(
        "http://{}/v1/chat/completions",
        listener.local_addr().unwrap()
    );
    let (sender, receiver) = mpsc::channel(8);
    let task = tokio::spawn(async move {
        let mut release = first_release;
        for response in responses {
            let (mut stream, _) = tokio::time::timeout(Duration::from_secs(3), listener.accept())
                .await
                .unwrap()
                .unwrap();
            let mut bytes = vec![];
            let mut buffer = [0; 4096];
            let (end, length) = loop {
                let count = stream.read(&mut buffer).await.unwrap();
                assert!(count > 0);
                bytes.extend_from_slice(&buffer[..count]);
                if let Some(end) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
                    let headers = std::str::from_utf8(&bytes[..end]).unwrap();
                    let length = headers
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .map(|value| value.trim().parse::<usize>().unwrap())
                        })
                        .unwrap();
                    break (end + 4, length);
                }
            };
            while bytes.len() < end + length {
                let count = stream.read(&mut buffer).await.unwrap();
                assert!(count > 0);
                bytes.extend_from_slice(&buffer[..count]);
            }
            sender
                .send((
                    String::from_utf8(bytes[..end].to_vec()).unwrap(),
                    serde_json::from_slice(&bytes[end..end + length]).unwrap(),
                ))
                .await
                .unwrap();
            if let Some(held) = release.take() {
                let _ = held.await;
            }
            let _ = stream.write_all(response.as_bytes()).await;
        }
        assert!(
            tokio::time::timeout(Duration::from_millis(75), listener.accept())
                .await
                .is_err(),
            "unexpected automatic send"
        );
    });
    (endpoint, receiver, task)
}
fn wire(status: u16, body: &str) -> String {
    format!(
        "HTTP/1.1 {status} Fixture\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}
fn chat(content: &str, model: &str, input: u64, output: u64) -> String {
    wire(200,&json!({"model":model,"usage":{"prompt_tokens":input,"completion_tokens":output},"choices":[{"finish_reason":"stop","message":{"role":"assistant","content":content}}]}).to_string())
}
struct Space;
impl notype_core::csv::CsvSpaceProbe for Space {
    fn available_bytes(
        &self,
        _: &std::path::Path,
    ) -> Result<u64, notype_core::csv::CsvExportError> {
        Ok(u64::MAX)
    }
}

#[tokio::test]
async fn real_snapshot_forward_reverse_sqlite_csv_and_manual_report_share_no_secret_pipeline() {
    let directory = tempfile::tempdir().unwrap();
    let diagnostics = service(directory.path());
    let sessions = SessionService::open(directory.path().join("sessions.db")).unwrap();
    let (release, held) = oneshot::channel();
    let forward = json!({"text":"PRIVATE_GENERATED_RESULT","source_language":"zh-TW"}).to_string();
    let (endpoint, mut requests, server) = fixture(
        vec![
            chat(&forward, "forward-producer", 3, 2),
            chat("PRIVATE_REVERSE_RESULT", "reverse-producer", 5, 4),
        ],
        Some(held),
    )
    .await;
    let (store, mut draft, revision) = configure(directory.path(), &endpoint, true);
    let snapshot = store.resolve_active(&NeverHome).unwrap();
    let entry = sessions
        .create(
            "RAW_SPEECH_SENTINEL",
            SessionOptions {
                scenario: Scenario::Translation,
                translation_target: Some("en-US".into()),
                ..Default::default()
            },
        )
        .unwrap();
    let processor = traced(&diagnostics, &entry.id, snapshot);
    let worker_sessions = sessions.clone();
    let id = entry.id.clone();
    let task = tokio::spawn(async move {
        worker_sessions
            .process_with_review(&id, vec![], processor, true)
            .await
    });
    let (first_header, first_body) = requests.recv().await.unwrap();
    assert!(first_header
        .to_ascii_lowercase()
        .contains("authorization: bearer owned_key_sentinel"));
    assert!(first_body.to_string().contains("RAW_SPEECH_SENTINEL"));
    assert_eq!(first_body["model"], "MODEL_PRIVATE_SENTINEL");
    assert_eq!(first_body["thinking"]["type"], "disabled");
    assert_eq!(first_body["temperature"], 0.0);
    assert!(first_body.get("top_p").is_none());
    draft.model = "NEW_MODEL".into();
    let changed = store
        .save(
            revision,
            draft.clone(),
            KeyAction::Replace {
                value: "NEW_KEY_SENTINEL".into(),
            },
        )
        .unwrap();
    store.remove(changed.revision, &draft.id).unwrap();
    release.send(()).unwrap();
    let processed = task.await.unwrap().unwrap();
    let (reverse_header, reverse_body) = requests.recv().await.unwrap();
    server.await.unwrap();
    assert!(reverse_header
        .to_ascii_lowercase()
        .contains("authorization: bearer owned_key_sentinel"));
    assert!(!reverse_header.contains("NEW_KEY_SENTINEL"));
    assert_eq!(reverse_body["model"], "MODEL_PRIVATE_SENTINEL");
    assert!(!reverse_body.to_string().contains("RAW_SPEECH_SENTINEL"));
    assert_eq!(
        processed.result.as_deref(),
        Some("PRIVATE_GENERATED_RESULT")
    );
    assert_eq!(
        processed.back_translation.as_deref(),
        Some("PRIVATE_REVERSE_RESULT")
    );
    assert_eq!(processed.api_summary.call_count, Some(2));
    assert_eq!(processed.api_summary.input.total, Some(8));
    assert_eq!(processed.api_summary.output.total, Some(6));
    let params = processed.api_summary.model_efforts[0]
        .parameters
        .as_ref()
        .unwrap();
    assert!(!params.requested_thinking);
    assert_eq!(params.effective_thinking, Some(false));
    assert_eq!(params.configured_temperature.as_deref(), Some("0"));
    assert_eq!(params.wire_temperature.as_deref(), Some("0"));
    assert_eq!(params.configured_top_p.as_deref(), Some("0.5"));
    assert_eq!(params.wire_top_p, None);
    assert_eq!(params.top_p_resolution, "ignored_by_provider");
    let csv = directory.path().join("history.csv");
    sessions
        .export_csv(&csv, &Space, &notype_core::CancellationToken::new())
        .unwrap();
    let csv_text = std::fs::read_to_string(&csv).unwrap();
    assert!(csv_text.contains("api_request_options"));
    assert!(csv_text.contains("configured_temperature"));
    let executable = std::env::var_os("NOTYPE_TEST_PYTHON")
        .unwrap_or_else(|| if cfg!(windows) { "python" } else { "python3" }.into());
    let mut command = std::process::Command::new(executable);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let output=command.args(["-X","utf8","-c","import csv,json,sys; f=open(sys.argv[1],encoding='utf-8-sig',newline=''); print(json.dumps(list(csv.DictReader(f))))"]).arg(&csv).output().unwrap();
    assert!(output.status.success());
    let rows: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(rows.as_array().unwrap().len(), 1);
    assert_eq!(rows[0]["history_id"], entry.id);
    assert_eq!(rows[0]["format_version"], "2");
    let tuples: Value =
        serde_json::from_str(rows[0]["api_request_options"].as_str().unwrap()).unwrap();
    assert_eq!(tuples.as_array().unwrap().len(), 2);
    assert_eq!(tuples[0]["parameters"]["configured_temperature"], "0");
    assert_eq!(tuples[0]["parameters"]["wire_top_p"], Value::Null);
    for secret in [
        "OWNED_KEY_SENTINEL",
        "NEW_KEY_SENTINEL",
        "PRIVATE_CONNECTION_NAME",
        endpoint.as_str(),
    ] {
        assert!(!csv_text.contains(secret));
    }
    let preview = diagnostics
        .publish_preview(diagnostics.begin_preview().unwrap(), context())
        .unwrap();
    let expected = preview.report.text().to_owned();
    let ticket = diagnostics.begin_export(preview.report.id()).unwrap();
    let destination = directory.path().join("report.txt");
    diagnostics.commit_export(&ticket, &destination).unwrap();
    assert_eq!(std::fs::read_to_string(&destination).unwrap(), expected);
    assert!(expected.contains("purpose=generation"));
    assert!(expected.contains("purpose=back_translation"));
    assert!(expected.contains("revision=1"));
    assert!(expected.contains("session=1"));
    assert_eq!(expected.matches("api_span=finished").count(), 2);
    for secret in [
        "OWNED_KEY_SENTINEL",
        "NEW_KEY_SENTINEL",
        "MODEL_PRIVATE_SENTINEL",
        "PRIVATE_CONNECTION_NAME",
        "RAW_SPEECH_SENTINEL",
        "PRIVATE_GENERATED_RESULT",
        "PRIVATE_REVERSE_RESULT",
        entry.id.as_str(),
        endpoint.as_str(),
    ] {
        assert!(!expected.contains(secret), "unexpected private metadata");
    }
    let disabled = store.resolve_active(&NeverHome).unwrap();
    let retried = sessions
        .process(&entry.id, vec![], traced(&diagnostics, &entry.id, disabled))
        .await
        .unwrap();
    assert_eq!(retried.error.as_deref(), Some("configuration-required"));
    assert_eq!(retried.api_summary.call_count, Some(2));
    assert_eq!(retried.transcript, "RAW_SPEECH_SENTINEL");
}

#[tokio::test]
async fn diagnostic_off_keeps_accounting_and_actual_http_status_separate_from_error_category() {
    for enabled in [true, false] {
        let directory = tempfile::tempdir().unwrap();
        let diagnostics = service(directory.path());
        if !enabled {
            diagnostics.set_enabled(false);
        }
        let sessions = SessionService::open(directory.path().join("sessions.db")).unwrap();
        let body = r#"{"model":"PRIVATE_MODEL","usage":{"prompt_tokens":11,"completion_tokens":2},"error":{"code":429,"message":"PRIVATE_ERROR_BODY"}}"#;
        let (endpoint, mut requests, server) = fixture(vec![wire(200, body)], None).await;
        let (store, _, _) = configure(directory.path(), &endpoint, false);
        let entry = sessions
            .create("original", SessionOptions::default())
            .unwrap();
        let result = sessions
            .process(
                &entry.id,
                vec![],
                traced(
                    &diagnostics,
                    &entry.id,
                    store.resolve_active(&NeverHome).unwrap(),
                ),
            )
            .await
            .unwrap();
        requests.recv().await.unwrap();
        server.await.unwrap();
        assert_eq!(result.error.as_deref(), Some("provider-rate-limit"));
        assert_eq!(result.api_summary.call_count, Some(1));
        assert_eq!(result.api_summary.input.total, Some(11));
        let report = report(&diagnostics);
        if enabled {
            assert!(report.contains("http_status=200"));
            assert!(report.contains("category=rate_limit"));
            assert!(report.contains("durable_admission=true"));
        } else {
            assert!(!report.contains("api_call="));
        }
        assert!(!report.contains("PRIVATE_ERROR_BODY"));
        assert!(!report.contains("PRIVATE_MODEL"));
    }
}

#[tokio::test]
async fn actual_outer_cancellation_and_clear_fence_provider_drop_without_late_resurrection() {
    for change in [0, 1, 2] {
        let directory = tempfile::tempdir().unwrap();
        let diagnostics = service(directory.path());
        let sessions = SessionService::open(directory.path().join("sessions.db")).unwrap();
        let (release, held) = oneshot::channel();
        let (endpoint, mut requests, server) =
            fixture(vec![chat("late response", "model", 1, 1)], Some(held)).await;
        let (store, _, _) = configure(directory.path(), &endpoint, false);
        let entry = sessions
            .create("original", SessionOptions::default())
            .unwrap();
        let processor = traced(
            &diagnostics,
            &entry.id,
            store.resolve_active(&NeverHome).unwrap(),
        );
        let worker = sessions.clone();
        let id = entry.id.clone();
        let task = tokio::spawn(async move { worker.process(&id, vec![], processor).await });
        requests.recv().await.unwrap();
        let preview = diagnostics
            .publish_preview(diagnostics.begin_preview().unwrap(), context())
            .unwrap();
        let old_ticket = diagnostics.begin_export(preview.report.id()).unwrap();
        let preserved = directory.path().join("preserved.txt");
        std::fs::write(&preserved, "PREEXISTING_DESTINATION").unwrap();
        if change == 1 {
            diagnostics.clear();
            assert_eq!(
                diagnostics.commit_export(&old_ticket, &preserved),
                Err("diagnostic-preview-expired")
            );
            assert_eq!(
                std::fs::read_to_string(&preserved).unwrap(),
                "PREEXISTING_DESTINATION"
            );
        } else if change == 2 {
            diagnostics.set_enabled(false);
            diagnostics.set_enabled(true);
        }
        let before = report(&diagnostics);
        sessions.cancel(&entry.id).unwrap();
        release.send(()).unwrap();
        let result = task.await.unwrap().unwrap();
        server.await.unwrap();
        assert_eq!(result.status, notype_core::SessionStatus::Cancelled);
        assert_eq!(result.transcript, "original");
        let after = report(&diagnostics);
        if change != 0 {
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
        } else {
            assert!(after.contains("api_result=interrupted"));
            assert!(after.contains("cancel_requested=true"));
            assert_eq!(after.matches("api_span=finished").count(), 1);
        }
        assert_eq!(result.api_summary.call_count, Some(1));
    }
}

#[tokio::test]
async fn real_sqlite_admission_and_mark_sent_faults_never_poll_http() {
    for mark_sent in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let diagnostics = service(directory.path());
        let path = directory.path().join("sessions.db");
        let sessions = SessionService::open(&path).unwrap();
        let sql = if mark_sent {
            "CREATE TRIGGER reject_send BEFORE UPDATE OF send_state ON api_calls WHEN NEW.send_state='possibly_sent' BEGIN SELECT RAISE(ABORT,'owned storage fault'); END;"
        } else {
            "CREATE TRIGGER reject_prepare BEFORE INSERT ON api_calls BEGIN SELECT RAISE(ABORT,'owned storage fault'); END;"
        };
        rusqlite::Connection::open(&path)
            .unwrap()
            .execute_batch(sql)
            .unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!(
            "http://{}/v1/chat/completions",
            listener.local_addr().unwrap()
        );
        let (store, _, _) = configure(directory.path(), &endpoint, false);
        let entry = sessions
            .create("original", SessionOptions::default())
            .unwrap();
        let result = sessions
            .process(
                &entry.id,
                vec![],
                traced(
                    &diagnostics,
                    &entry.id,
                    store.resolve_active(&NeverHome).unwrap(),
                ),
            )
            .await
            .unwrap();
        assert_eq!(result.error.as_deref(), Some("history-storage-failed"));
        assert_eq!(result.transcript, "original");
        assert_eq!(result.api_summary.call_count, Some(0));
        assert!(
            tokio::time::timeout(Duration::from_millis(75), listener.accept())
                .await
                .is_err()
        );
        let report = report(&diagnostics);
        assert!(!report.contains("possibly_sent=true"));
        assert!(report.contains("category=history_storage"));
        assert!(report.contains(if mark_sent {
            "api_span=pre_send_rejected"
        } else {
            "api_span=admission_failed"
        }));
    }
}

#[tokio::test]
async fn known_resolved_secret_reflections_never_enter_history_csv_or_report_but_usage_survives() {
    let key = "unique_key_k83_sentinel_471ab9d5";
    for (status, content, reported_model, expected) in [
        (200, "safe text", key, "llm-invalid-response"),
        (200, key, "safe-model", "llm-invalid-response"),
        (401, key, key, "provider-auth"),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let diagnostics = service(directory.path());
        let sessions = SessionService::open(directory.path().join("sessions.db")).unwrap();
        let body=json!({"model":reported_model,"usage":{"prompt_tokens":9,"completion_tokens":3},"choices":[{"finish_reason":"stop","message":{"role":"assistant","content":content}}]}).to_string();
        assert!(body.contains(key));
        let (endpoint, mut requests, server) = fixture(vec![wire(status, &body)], None).await;
        let (store, draft, revision) = configure(directory.path(), &endpoint, false);
        store
            .save(revision, draft, KeyAction::Replace { value: key.into() })
            .unwrap();
        let entry = sessions
            .create("original must remain", SessionOptions::default())
            .unwrap();
        let result = sessions
            .process(
                &entry.id,
                vec![],
                traced(
                    &diagnostics,
                    &entry.id,
                    store.resolve_active(&NeverHome).unwrap(),
                ),
            )
            .await
            .unwrap();
        let (header, _) = requests.recv().await.unwrap();
        assert!(header.contains(key));
        server.await.unwrap();
        assert_eq!(result.error.as_deref(), Some(expected));
        assert_eq!(result.result, None);
        assert_eq!(result.transcript, "original must remain");
        assert_eq!(result.api_summary.input.total, Some(9));
        assert!(!serde_json::to_string(&result).unwrap().contains(key));
        let csv = directory.path().join("safe.csv");
        sessions
            .export_csv(&csv, &Space, &notype_core::CancellationToken::new())
            .unwrap();
        assert!(!std::fs::read_to_string(csv).unwrap().contains(key));
        assert!(!report(&diagnostics).contains(key));
    }
}

#[tokio::test]
async fn actual_retry_has_same_local_session_and_distinct_accepted_observer_revision() {
    let directory = tempfile::tempdir().unwrap();
    let diagnostics = service(directory.path());
    let sessions = SessionService::open(directory.path().join("sessions.db")).unwrap();
    let (endpoint, mut requests, server) = fixture(
        vec![
            wire(429, "<html>PRIVATE_ERROR_BODY</html>"),
            chat("valid retry", "returned-model", 4, 2),
        ],
        None,
    )
    .await;
    let (store, _, _) = configure(directory.path(), &endpoint, false);
    let entry = sessions
        .create("original", SessionOptions::default())
        .unwrap();
    let first = sessions
        .process(
            &entry.id,
            vec![],
            traced(
                &diagnostics,
                &entry.id,
                store.resolve_active(&NeverHome).unwrap(),
            ),
        )
        .await
        .unwrap();
    requests.recv().await.unwrap();
    assert_eq!(first.api_summary.call_count, Some(1));
    assert_eq!(first.error.as_deref(), Some("provider-rate-limit"));
    let second = sessions
        .process(
            &entry.id,
            vec![],
            traced(
                &diagnostics,
                &entry.id,
                store.resolve_active(&NeverHome).unwrap(),
            ),
        )
        .await
        .unwrap();
    requests.recv().await.unwrap();
    server.await.unwrap();
    assert_eq!(second.id, entry.id);
    assert_eq!(second.result.as_deref(), Some("valid retry"));
    assert_eq!(second.api_summary.call_count, Some(2));
    let report = report(&diagnostics);
    assert!(report.contains("session=1 revision=1"));
    assert!(report.contains("session=1 revision=2"));
    assert!(report.contains("api_call=1 "));
    assert!(report.contains("api_call=2 "));
    assert_eq!(report.matches("api_span=finished").count(), 2);
    assert!(!report.contains("PRIVATE_ERROR_BODY"));
}

#[tokio::test]
async fn failed_capture_keeps_saved_translation_target_for_repaired_retry() {
    let directory = tempfile::tempdir().unwrap();
    let (endpoint, mut requests, server) = fixture(
        vec![chat("translated result", "returned-model", 3, 1)],
        None,
    )
    .await;
    let (store, draft, revision) = configure(directory.path(), &endpoint, false);
    let view = store
        .save(revision, draft.clone(), KeyAction::Clear)
        .unwrap();
    let mut translation = view.translation;
    translation.target_language = "ja-JP".into();
    let view = store.translation(view.revision, translation).unwrap();
    let captured = store.processing_snapshot(&NeverHome);
    assert_eq!(
        captured.preparation_failure,
        Some(notype_core::PreparationFailure::Credentials)
    );
    let workflow = workflow::DesktopWorkflow::new(
        SessionService::open(directory.path().join("sessions.db")).unwrap(),
        Arc::new(|| {}),
    );
    let generation = workflow
        .begin(notype_core::RecognitionMode::TraditionalEnglish, None)
        .unwrap();
    workflow
        .capture_processing_snapshot(generation, Scenario::Translation, captured.clone(), vec![])
        .unwrap();
    let original = workflow
        .finish(generation, "translation original", captured, vec![])
        .await
        .unwrap();
    assert_eq!(original.transcript, "translation original");
    assert_eq!(
        original.error.as_deref(),
        Some("configuration-credentials-unavailable")
    );
    assert_eq!(original.translation_target.as_deref(), Some("ja-JP"));
    assert!(
        requests.try_recv().is_err(),
        "failed preparation must not send"
    );
    let view = store
        .save(
            view.revision,
            draft,
            KeyAction::Replace {
                value: "REPAIRED_OWNED_KEY".into(),
            },
        )
        .unwrap();
    let mut later_translation = view.translation;
    later_translation.target_language = "fr-FR".into();
    store.translation(view.revision, later_translation).unwrap();
    let repaired = workflow
        .retry(&original.id, store.processing_snapshot(&NeverHome), vec![])
        .await
        .unwrap();
    assert_eq!(repaired.id, original.id);
    assert_eq!(repaired.translation_target.as_deref(), Some("ja-JP"));
    assert_eq!(repaired.result.as_deref(), Some("translated result"));
    assert_eq!(
        workflow
            .sessions()
            .get(&original.id)
            .unwrap()
            .translation_target
            .as_deref(),
        Some("ja-JP")
    );
    let (_, body) = requests.recv().await.unwrap();
    let content: Value =
        serde_json::from_str(body["messages"][2]["content"].as_str().unwrap()).unwrap();
    assert_eq!(content["translation_target"], "ja-JP");
    server.await.unwrap();
}

#[tokio::test]
async fn actual_error_transport_and_bound_reports_retain_only_status_and_safe_lifecycle() {
    let oversized = "x".repeat(1_048_577);
    for (raw, expected_status, category, body_complete) in [
        (
            wire(401, "<html>PRIVATE_ERROR_BODY</html>"),
            Some(401),
            "auth",
            true,
        ),
        (
            wire(503, "<html>PRIVATE_ERROR_BODY</html>"),
            Some(503),
            "unavailable",
            true,
        ),
        (
            wire(200, "not JSON PRIVATE_ERROR_BODY"),
            Some(200),
            "protocol",
            true,
        ),
        (wire(200, &oversized), Some(200), "oversize", false),
        (String::new(), None, "transport", false),
        (
            "HTTP/1.1 200 OK\r\nContent-Length: 500\r\nConnection: close\r\n\r\n{\"choices\":"
                .into(),
            Some(200),
            "transport",
            false,
        ),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let diagnostics = service(directory.path());
        let sessions = SessionService::open(directory.path().join("sessions.db")).unwrap();
        let (endpoint, mut requests, server) = fixture(vec![raw], None).await;
        let (store, _, _) = configure(directory.path(), &endpoint, false);
        let entry = sessions
            .create("original", SessionOptions::default())
            .unwrap();
        let result = sessions
            .process(
                &entry.id,
                vec![],
                traced(
                    &diagnostics,
                    &entry.id,
                    store.resolve_active(&NeverHome).unwrap(),
                ),
            )
            .await
            .unwrap();
        requests.recv().await.unwrap();
        server.await.unwrap();
        assert_eq!(result.api_summary.call_count, Some(1));
        let report = report(&diagnostics);
        assert!(report.contains(&format!("category={category}")));
        if let Some(status) = expected_status {
            assert!(report.contains(&format!("http_status={status}")));
        }
        assert!(report.contains(&format!("body_complete={body_complete}")));
        assert!(!report.contains("PRIVATE_ERROR_BODY"));
        assert!(!report.contains("OWNED_KEY_SENTINEL"));
    }
}

#[tokio::test]
async fn generation_stalled_body_timeout_keeps_observed_headers_separate_from_request_wait() {
    let directory = tempfile::tempdir().unwrap();
    let diagnostic = service(directory.path());
    let sessions = SessionService::open(directory.path().join("sessions.db")).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!(
        "http://{}/v1/chat/completions",
        listener.local_addr().unwrap()
    );
    let partial = b"{\"choices\":[{\"partial\":\"PRIVATE_PARTIAL_BODY";
    let (ready, headers_sent) = oneshot::channel();
    let (release, held) = oneshot::channel();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut bytes = vec![];
        let mut buffer = [0; 4096];
        let (end, length) = loop {
            let n = stream.read(&mut buffer).await.unwrap();
            assert!(n > 0);
            bytes.extend_from_slice(&buffer[..n]);
            if let Some(end) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
                let headers = std::str::from_utf8(&bytes[..end]).unwrap();
                let length = headers
                    .lines()
                    .find_map(|line| {
                        line.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .map(|value| value.trim().parse::<usize>().unwrap())
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
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 100000\r\nConnection: close\r\n\r\n")
            .await
            .unwrap();
        stream.write_all(partial).await.unwrap();
        ready.send(()).unwrap();
        let _ = held.await;
    });
    let (store, mut draft, revision) = configure(directory.path(), &endpoint, false);
    draft.timeout_ms = 2000;
    store.save(revision, draft, KeyAction::Keep).unwrap();
    let entry = sessions
        .create("stalled body original", SessionOptions::default())
        .unwrap();
    let processor = traced(
        &diagnostic,
        &entry.id,
        store.resolve_active(&NeverHome).unwrap(),
    );
    let worker = sessions.clone();
    let id = entry.id.clone();
    let task = tokio::spawn(async move { worker.process(&id, vec![], processor).await });
    headers_sent.await.unwrap();
    // Observe the real headers stage while the server is still withholding EOF.
    // This checks attribution, without asserting a machine/network latency target.
    let header_before_timeout = tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            let text = report(&diagnostic);
            if let Some(line) = text
                .lines()
                .find(|line| line.contains("api_span=headers_received "))
            {
                assert!(!text.contains("api_span=finished"));
                break line.to_owned();
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let result = tokio::time::timeout(Duration::from_secs(4), task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let _ = release.send(());
    server.await.unwrap();
    assert_eq!(result.error.as_deref(), Some("llm-timeout"));
    assert_eq!(result.transcript, "stalled body original");
    assert_eq!(result.result, None);
    assert_eq!(result.api_summary.call_count, Some(1));
    assert_eq!(result.api_summary.input.total, None);
    assert_eq!(result.api_summary.output.total, None);
    assert!(result.api_summary.reported_models.is_empty());
    let text = report(&diagnostic);
    let header_after_timeout = text
        .lines()
        .find(|line| line.contains("api_span=headers_received "))
        .unwrap();
    assert_eq!(header_after_timeout, header_before_timeout);
    let terminal = text
        .lines()
        .find(|line| line.contains("api_span=finished "))
        .unwrap();
    assert!(terminal.contains("http_status=200"));
    assert!(terminal.contains("api_result=timeout"));
    assert!(terminal.contains("category=timeout"));
    assert!(terminal.contains("body_complete=false"));
    assert!(terminal.contains(&format!("body_bytes={}", partial.len())));
    assert!(
        terminal.contains("timeout_source=local_deadline")
            || terminal.contains("timeout_source=transport_predicate")
    );
    assert_eq!(text.matches("api_span=finished").count(), 1);
    assert!(!text.contains("PRIVATE_PARTIAL_BODY"));
    assert!(text.contains("DNS/TLS subphase timing is unobservable"));
}

// This role is invoked only by the parent regression below. It receives a
// completely cleared environment plus synthetic loopback proxy/route values.
#[cfg(unix)]
#[tokio::test]
#[ignore = "controlled child role; exercised by custom_https_and_local_proxy_policy"]
async fn owned_proxy_generation_child() {
    let role = std::env::var("NOTYPE_OWNED_PROXY_ROLE").unwrap();
    assert!(matches!(role.as_str(), "local" | "custom"));
    let endpoint = std::env::var("NOTYPE_OWNED_PROXY_ENDPOINT").unwrap();
    let directory = tempfile::tempdir().unwrap();
    let (store, mut draft, revision) = configure(directory.path(), &endpoint, false);
    if role == "custom" {
        draft.kind = ConnectionKind::Custom;
        store.save(revision, draft, KeyAction::Keep).unwrap();
    }
    let sessions = SessionService::open(directory.path().join("sessions.db")).unwrap();
    let entry = sessions
        .create("proxy fixture original", SessionOptions::default())
        .unwrap();
    let result = sessions
        .process(
            &entry.id,
            vec![],
            workflow::text_processor(store.resolve_active(&NeverHome).unwrap()),
        )
        .await
        .unwrap();
    assert_eq!(result.api_summary.call_count, Some(1));
    assert_eq!(result.transcript, "proxy fixture original");
    if role == "local" {
        assert_eq!(result.result.as_deref(), Some("proxy fixture result"));
    } else {
        assert_eq!(result.error.as_deref(), Some("provider-transport"));
        assert_eq!(result.result, None);
    }
}

#[cfg(unix)]
#[tokio::test]
async fn custom_https_and_local_proxy_policy_use_only_owned_routes_and_cleared_child_env() {
    struct OwnedChild(Option<std::process::Child>);
    impl Drop for OwnedChild {
        fn drop(&mut self) {
            if let Some(child) = &mut self.0 {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
    for local in [false, true] {
        let proxy = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let destination = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let proxy_url = format!("http://{}", proxy.local_addr().unwrap());
        let target = destination.local_addr().unwrap();
        let endpoint = format!(
            "{}://{target}/v1/chat/completions",
            if local { "http" } else { "https" }
        );
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        command
            .args(["--exact", "owned_proxy_generation_child", "--ignored"])
            .env_clear()
            .env(
                "NOTYPE_OWNED_PROXY_ROLE",
                if local { "local" } else { "custom" },
            )
            .env("NOTYPE_OWNED_PROXY_ENDPOINT", endpoint)
            .env("HTTP_PROXY", &proxy_url)
            .env("http_proxy", &proxy_url)
            .env("HTTPS_PROXY", &proxy_url)
            .env("https_proxy", &proxy_url)
            .env("NO_PROXY", "")
            .env("no_proxy", "")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        let mut child = OwnedChild(Some(command.spawn().unwrap()));
        let (via_proxy,mut stream)=tokio::time::timeout(Duration::from_secs(5),async{tokio::select!{result=proxy.accept()=>{let(stream,_)=result.unwrap();(true,stream)},result=destination.accept()=>{let(stream,_)=result.unwrap();(false,stream)}}}).await.unwrap();
        assert_eq!(
            via_proxy, !local,
            "regular Custom HTTPS must retain proxy behavior; Local must bypass it"
        );
        let mut bytes = vec![];
        let mut buffer = [0; 4096];
        let (end, length) = loop {
            let n = stream.read(&mut buffer).await.unwrap();
            assert!(n > 0);
            bytes.extend_from_slice(&buffer[..n]);
            assert!(bytes.len() <= 32768);
            if let Some(end) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
                let headers = std::str::from_utf8(&bytes[..end]).unwrap();
                let length = headers
                    .lines()
                    .find_map(|line| {
                        line.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .map(|value| value.trim().parse::<usize>().unwrap())
                    })
                    .unwrap_or(0);
                assert!(length <= 32768);
                break (end + 4, length);
            }
        };
        while bytes.len() < end + length {
            let n = stream.read(&mut buffer).await.unwrap();
            assert!(n > 0);
            bytes.extend_from_slice(&buffer[..n]);
        }
        if local {
            assert!(bytes.starts_with(b"POST /v1/chat/completions HTTP/1.1"));
            assert!(std::str::from_utf8(&bytes)
                .unwrap()
                .contains("proxy fixture original"));
            stream
                .write_all(chat("proxy fixture result", "fixture", 3, 1).as_bytes())
                .await
                .unwrap();
        } else {
            assert!(bytes.starts_with(format!("CONNECT {target} HTTP/1.1").as_bytes()));
            assert!(!std::str::from_utf8(&bytes)
                .unwrap()
                .contains("OWNED_KEY_SENTINEL"));
            stream
                .write_all(
                    b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                )
                .await
                .unwrap();
        }
        drop(stream);
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if child.0.as_mut().unwrap().try_wait().unwrap().is_some() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        let output = child.0.take().unwrap().wait_with_output().unwrap();
        assert!(output.stdout.len() < 16384 && output.stderr.len() < 16384);
        assert!(
            output.status.success(),
            "owned child failed: {}",
            String::from_utf8_lossy(&output.stdout)
        );
        assert!(
            tokio::time::timeout(Duration::from_millis(75), proxy.accept())
                .await
                .is_err()
        );
        assert!(
            tokio::time::timeout(Duration::from_millis(75), destination.accept())
                .await
                .is_err()
        );
    }
}
