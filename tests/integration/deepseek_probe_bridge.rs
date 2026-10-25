#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/capture_startup.rs"]
mod capture_startup;
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/diagnostics.rs"]
mod diagnostics;
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/deepseek_probe/mod.rs"]
mod probe;
#[allow(dead_code, unused_imports)]
#[path = "../../desktop/src-tauri/src/workflow.rs"]
mod workflow;

use notype_core::{Scenario, SessionOptions, SessionService};
use probe::{bridge::BudgetProcessor, round::Round, sqlite_round::SqliteRoundStore};
use serde_json::{json, Value};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

struct Space;
impl notype_core::csv::CsvSpaceProbe for Space {
    fn available_bytes(
        &self,
        _: &std::path::Path,
    ) -> Result<u64, notype_core::csv::CsvExportError> {
        Ok(u64::MAX)
    }
}
fn csv_rows(path: &std::path::Path) -> Value {
    let mut command = std::process::Command::new(if cfg!(windows) { "python" } else { "python3" });
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let result = command.args(["-X", "utf8", "-c", "import csv,json,sys; print(json.dumps(list(csv.DictReader(open(sys.argv[1],encoding='utf-8-sig',newline=''))),ensure_ascii=False))"]).arg(path).output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    serde_json::from_slice(&result.stdout).unwrap()
}

async fn provider(
    responses: Vec<Value>,
) -> (String, Arc<Mutex<Vec<Value>>>, tokio::task::JoinHandle<()>) {
    provider_with(
        responses
            .into_iter()
            .map(|value| (200, value.to_string(), Duration::ZERO))
            .collect(),
    )
    .await
}

async fn provider_with(
    responses: Vec<(u16, String, Duration)>,
) -> (String, Arc<Mutex<Vec<Value>>>, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}/chat/completions", listener.local_addr().unwrap());
    let received = Arc::new(Mutex::new(Vec::new()));
    let captured = received.clone();
    let task = tokio::spawn(async move {
        for (status, body, delay) in responses {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut bytes = Vec::new();
            let mut chunk = [0; 4096];
            let (header_end, length) = loop {
                let n = stream.read(&mut chunk).await.unwrap();
                assert!(n > 0);
                bytes.extend_from_slice(&chunk[..n]);
                if let Some(end) = bytes.windows(4).position(|s| s == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&bytes[..end]);
                    let length = headers
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .map(|n| n.trim().parse::<usize>().unwrap())
                        })
                        .unwrap();
                    break (end + 4, length);
                }
            };
            while bytes.len() < header_end + length {
                let n = stream.read(&mut chunk).await.unwrap();
                assert!(n > 0);
                bytes.extend_from_slice(&chunk[..n]);
            }
            captured
                .lock()
                .unwrap()
                .push(serde_json::from_slice(&bytes[header_end..header_end + length]).unwrap());
            tokio::time::sleep(delay).await;
            let _ = stream.write_all(format!("HTTP/1.1 {status} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).as_bytes()).await;
        }
    });
    (endpoint, received, task)
}

fn response(model: &str, text: &str) -> Value {
    json!({"model":model,"choices":[{"finish_reason":"stop","message":{"role":"assistant","content":text}}],"usage":{"prompt_tokens":17,"completion_tokens":5,"total_tokens":22,"prompt_cache_hit_tokens":0,"prompt_cache_miss_tokens":17}})
}
fn configured(endpoint: String) -> workflow::TextConfiguration {
    workflow::TextConfiguration {
        transport: workflow::TextTransport::OpenAiChat,
        endpoint,
        model: "deepseek-flash".into(),
        authentication: workflow::TextAuthentication::Bearer,
        api_key: "synthetic-fixture-key-not-a-secret".into(),
        timeout_ms: 15000,
        provider_profile: workflow::TextProviderProfile::DeepSeek,
        effort: Some("none".into()),
        max_output_tokens: Some(512),
        ..Default::default()
    }
}

#[tokio::test]
async fn unexpected_forward_model_halts_before_automatic_reverse_and_keeps_history_usage() {
    let owned = tempfile::tempdir().unwrap();
    let sessions = SessionService::open(owned.path().join("sessions.db")).unwrap();
    let mut round =
        Round::open(SqliteRoundStore::open_owned(&owned.path().join("round")).unwrap()).unwrap();
    round.arm().unwrap();
    let round = Arc::new(Mutex::new(round));
    let mut responses = vec![response("deepseek-flash", "OK"); 4];
    responses.push(response(
        "unexpected-model",
        r#"{"text":"Keep API_42.","source_language":"zh-TW"}"#,
    ));
    let (endpoint, received, server) = provider(responses).await;
    let inner = workflow::probe_text_processor(configured(endpoint.clone()));
    let mut review_id = String::new();
    for slot in 1..=5 {
        let mut options = SessionOptions::default();
        options.scenario = if slot == 5 {
            Scenario::Translation
        } else {
            Scenario::Ask
        };
        options.translation_target = Some("en-US".into());
        let entry = sessions.create("不要刪除 API_42。", options).unwrap();
        let processor = BudgetProcessor::for_fixture(
            inner.clone(),
            round.clone(),
            entry.id.clone(),
            slot,
            &endpoint,
        )
        .unwrap();
        sessions
            .process_with_review(&entry.id, vec![], processor, slot == 5)
            .await
            .unwrap();
        if slot == 5 {
            review_id = entry.id;
        }
    }
    tokio::time::timeout(Duration::from_secs(2), server)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        received.lock().unwrap().len(),
        5,
        "unexpected model must suppress reverse request"
    );
    let report = round.lock().unwrap().report();
    assert!(report.halted);
    assert_eq!(report.reserved_slots, 5);
    let entry = sessions.get(&review_id).unwrap();
    assert_eq!(entry.api_summary.observed_call_count, 1);
    assert_eq!(entry.api_summary.input.total, Some(17));
    assert!(entry.back_translation.is_none());
    let csv = owned.path().join("history.csv");
    assert_eq!(
        sessions
            .export_csv(&csv, &Space, &Default::default())
            .unwrap()
            .rows,
        5
    );
    let rows = csv_rows(&csv);
    let rows = rows.as_array().unwrap();
    assert_eq!(rows.len(), 5);
    let row = rows
        .iter()
        .find(|row| row["history_id"] == review_id)
        .unwrap();
    assert_eq!(row["input_tokens"], "17");
    assert_eq!(row["actual_fee_status"], "unknown");
    assert_eq!(row["initial_stop_to_result_ms"], "");
    assert_eq!(
        serde_json::from_str::<Value>(row["api_reported_models"].as_str().unwrap()).unwrap(),
        json!(["unexpected-model"])
    );
}

#[tokio::test]
async fn live_target_and_fixture_target_reject_actual_destination_mismatch_before_tcp_send() {
    for live in [true, false] {
        let owned = tempfile::tempdir().unwrap();
        let sessions = SessionService::open(owned.path().join("sessions.db")).unwrap();
        let mut round =
            Round::open(SqliteRoundStore::open_owned(&owned.path().join("round")).unwrap())
                .unwrap();
        round.arm().unwrap();
        let round = Arc::new(Mutex::new(round));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let plain = format!("http://{}/chat/completions", listener.local_addr().unwrap());
        let actual = if live {
            plain.clone()
        } else {
            plain.replacen("http:", "https:", 1)
        };
        let inner = workflow::probe_text_processor(configured(actual));
        let entry = sessions
            .create("只回答 OK。", SessionOptions::default())
            .unwrap();
        let processor = if live {
            BudgetProcessor::live(inner, round.clone(), entry.id.clone(), 1).unwrap()
        } else {
            BudgetProcessor::for_fixture(inner, round.clone(), entry.id.clone(), 1, &plain).unwrap()
        };
        sessions
            .process_with_review(&entry.id, vec![], processor, false)
            .await
            .unwrap();
        assert!(
            tokio::time::timeout(Duration::from_millis(100), listener.accept())
                .await
                .is_err(),
            "mismatched actual destination must never poll HTTP"
        );
        assert_eq!(round.lock().unwrap().report().reserved_slots, 0);
        assert!(round.lock().unwrap().report().halted);
        assert_eq!(
            sessions
                .get(&entry.id)
                .unwrap()
                .api_summary
                .observed_call_count,
            0
        );
    }
}

#[tokio::test]
async fn malformed_optional_usage_halts_instead_of_becoming_zero_reasoning_or_missing_cache() {
    let owned = tempfile::tempdir().unwrap();
    let sessions = SessionService::open(owned.path().join("sessions.db")).unwrap();
    let mut round =
        Round::open(SqliteRoundStore::open_owned(&owned.path().join("round")).unwrap()).unwrap();
    round.arm().unwrap();
    let round = Arc::new(Mutex::new(round));
    let mut malformed = response("deepseek-flash", "OK");
    malformed["usage"]["completion_tokens_details"] = json!({"reasoning_tokens":-1});
    let (endpoint, _, server) = provider(vec![malformed]).await;
    let entry = sessions
        .create("只回答 OK。", SessionOptions::default())
        .unwrap();
    let processor = BudgetProcessor::for_fixture(
        workflow::probe_text_processor(configured(endpoint.clone())),
        round.clone(),
        entry.id.clone(),
        1,
        &endpoint,
    )
    .unwrap();
    sessions
        .process_with_review(&entry.id, vec![], processor, false)
        .await
        .unwrap();
    server.await.unwrap();
    assert!(
        round.lock().unwrap().report().halted,
        "negative reasoning usage must not silently become an absent/zero value"
    );
    assert_eq!(
        sessions.get(&entry.id).unwrap().api_summary.input.total,
        Some(17)
    );
}

#[tokio::test]
async fn fixed_round_runs_eight_guarded_calls_with_review_cancel_retry_and_one_csv_row_per_history()
{
    let owned = tempfile::tempdir().unwrap();
    let sessions = SessionService::open(owned.path().join("sessions.db")).unwrap();
    let mut round =
        Round::open(SqliteRoundStore::open_owned(&owned.path().join("round")).unwrap()).unwrap();
    round.arm().unwrap();
    let round = Arc::new(Mutex::new(round));
    let mut replies = vec![
        (
            200,
            response("deepseek-flash", "OK").to_string(),
            Duration::ZERO
        );
        8
    ];
    replies[4].1 = response(
        "deepseek-flash",
        r#"{"text":"Keep API_42 and timeout 0.5.","source_language":"zh-TW"}"#,
    )
    .to_string();
    replies[5].1 = response("deepseek-flash", "保留 API_42 與 timeout 0.5。").to_string();
    replies[6].2 = Duration::from_millis(150);
    let (endpoint, received, server) = provider_with(replies).await;
    let inner = workflow::probe_text_processor(configured(endpoint.clone()));
    let mut runner =
        probe::runner::ProbeRunner::for_fixture(sessions.clone(), round.clone(), inner, &endpoint)
            .unwrap();
    let mut steps = Vec::new();
    while let Some(step) = runner.next().await.unwrap() {
        if step.first_slot == 7 {
            assert_eq!(step.history.status, notype_core::SessionStatus::Cancelled);
        }
        runner.accept_output(true).unwrap();
        steps.push(step);
    }
    server.await.unwrap();
    assert_eq!(steps.len(), 7);
    assert_eq!(received.lock().unwrap().len(), 8);
    let report = round.lock().unwrap().report();
    assert!(!report.halted);
    assert_eq!(report.reserved_slots, 8);
    assert_eq!(report.reservation_units, 835584);
    assert_eq!(report.slots[4].history_id, report.slots[5].history_id);
    assert_eq!(report.slots[6].history_id, report.slots[7].history_id);
    assert_eq!(steps[5].history.id, steps[6].history.id);
    let reverse = sessions.get(&steps[4].history.id).unwrap();
    assert_eq!(reverse.api_summary.input.total, Some(34));
    assert_eq!(reverse.api_summary.output.total, Some(10));
    let retry = sessions.get(&steps[6].history.id).unwrap();
    assert_eq!(retry.api_summary.observed_call_count, 2);
    assert_eq!(
        retry.api_summary.input.status,
        notype_core::observations::Completeness::Partial
    );
    assert_eq!(retry.api_summary.input.known_subtotal, Some(17));
    assert!(retry.result.is_some());
    assert!(matches!(
        retry.api_summary.fees.status,
        notype_core::observations::Completeness::Unknown
    ));
    for wire in received.lock().unwrap().iter() {
        assert_eq!(wire["stream"], false);
        assert_eq!(wire["thinking"]["type"], "disabled");
        assert_eq!(wire["reasoning_effort"], "none");
        assert_eq!(wire["max_tokens"], 512);
        assert!(wire["messages"].as_array().unwrap().len() <= 4);
        assert!(wire.to_string().len() <= 8192);
    }
    let csv = owned.path().join("round.csv");
    assert_eq!(
        sessions
            .export_csv(&csv, &Space, &Default::default())
            .unwrap()
            .rows,
        6
    );
    assert!(std::fs::read(&csv)
        .unwrap()
        .starts_with(&[0xef, 0xbb, 0xbf]));
    let rows = csv_rows(&csv);
    let rows = rows.as_array().unwrap();
    assert_eq!(rows.len(), 6);
    let pair = rows
        .iter()
        .find(|row| row["history_id"] == steps[4].history.id)
        .unwrap();
    assert_eq!(pair["input_tokens"], "34");
    assert_eq!(pair["output_tokens"], "10");
    assert_eq!(pair["back_translation"], "保留 API_42 與 timeout 0.5。");
    let retried = rows
        .iter()
        .find(|row| row["history_id"] == steps[6].history.id)
        .unwrap();
    assert_eq!(retried["api_observed_call_count"], "2");
    assert_eq!(retried["input_tokens_status"], "partial");
    assert_eq!(retried["input_tokens"], "");
    assert_eq!(retried["input_tokens_known_subtotal"], "17");
    assert!(rows.iter().all(|row| row["actual_fee_status"] == "unknown"));
    assert!(!std::fs::read_to_string(csv)
        .unwrap()
        .contains("synthetic-fixture-key-not-a-secret"));
}

#[tokio::test]
async fn error_and_response_contract_matrix_halts_after_one_reserved_call_without_retry() {
    let mut variants = Vec::new();
    variants.push((
        401,
        json!({"error":{"message":"fixture denied"}}).to_string(),
        Duration::ZERO,
    ));
    variants.push((
        429,
        json!({"error":{"message":"fixture limited"}}).to_string(),
        Duration::ZERO,
    ));
    variants.push((200, "malformed-json".into(), Duration::ZERO));
    variants.push((200, "x".repeat(1_048_577), Duration::ZERO));
    for change in 0..7 {
        let mut value = response("deepseek-flash", "OK");
        match change {
            0 => value["usage"]["total_tokens"] = json!(99),
            1 => value["usage"]["prompt_tokens"] = json!(32769),
            2 => value["usage"]["completion_tokens"] = json!(513),
            3 => {
                value["choices"][0]["message"]["reasoning_content"] =
                    json!("nonsecret forbidden reasoning fixture")
            }
            4 => value["usage"]["prompt_cache_hit_tokens"] = json!(-1),
            5 => value["choices"][0]["finish_reason"] = json!("length"),
            _ => {
                value["usage"]
                    .as_object_mut()
                    .unwrap()
                    .remove("total_tokens");
            }
        }
        variants.push((200, value.to_string(), Duration::ZERO));
    }
    variants.push((
        200,
        response("deepseek-flash", "OK").to_string(),
        Duration::from_millis(150),
    ));
    for (index, reply) in variants.into_iter().enumerate() {
        let owned = tempfile::tempdir().unwrap();
        let sessions = SessionService::open(owned.path().join("sessions.db")).unwrap();
        let mut round =
            Round::open(SqliteRoundStore::open_owned(&owned.path().join("round")).unwrap())
                .unwrap();
        round.arm().unwrap();
        let round = Arc::new(Mutex::new(round));
        let (endpoint, received, server) = provider_with(vec![reply]).await;
        let mut config = configured(endpoint.clone());
        if index == 11 {
            config.timeout_ms = 50;
        }
        let mut runner = probe::runner::ProbeRunner::for_fixture(
            sessions,
            round.clone(),
            workflow::probe_text_processor(config),
            &endpoint,
        )
        .unwrap();
        assert!(runner.next().await.is_err(), "matrix {index} must stop");
        assert!(
            runner.next().await.is_err(),
            "matrix {index} must not auto retry"
        );
        server.await.unwrap();
        assert_eq!(received.lock().unwrap().len(), 1, "matrix {index}");
        let report = round.lock().unwrap().report();
        assert!(report.halted, "matrix {index}");
        assert_eq!(report.reserved_slots, 1);
    }
}

#[tokio::test]
async fn unarmed_round_and_core_pre_send_storage_failure_never_poll_http() {
    for storage_failure in [false, true] {
        let owned = tempfile::tempdir().unwrap();
        let database = owned.path().join("sessions.db");
        let sessions = SessionService::open(&database).unwrap();
        let mut round =
            Round::open(SqliteRoundStore::open_owned(&owned.path().join("round")).unwrap())
                .unwrap();
        if storage_failure {
            round.arm().unwrap();
            rusqlite::Connection::open(&database).unwrap().execute_batch("CREATE TRIGGER reject_dispatch BEFORE UPDATE OF send_state ON api_calls WHEN NEW.send_state='possibly_sent' BEGIN SELECT RAISE(FAIL,'owned fixture dispatch'); END;").unwrap();
        }
        let round = Arc::new(Mutex::new(round));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}/chat/completions", listener.local_addr().unwrap());
        let entry = sessions
            .create("只回答 OK。", SessionOptions::default())
            .unwrap();
        let processor = BudgetProcessor::for_fixture(
            workflow::probe_text_processor(configured(endpoint.clone())),
            round.clone(),
            entry.id.clone(),
            1,
            &endpoint,
        )
        .unwrap();
        sessions
            .process_with_review(&entry.id, vec![], processor, false)
            .await
            .unwrap();
        assert!(
            tokio::time::timeout(Duration::from_millis(100), listener.accept())
                .await
                .is_err()
        );
        assert_eq!(
            round.lock().unwrap().report().reserved_slots,
            u8::from(storage_failure)
        );
        assert!(round.lock().unwrap().report().halted);
        assert_eq!(sessions.get(&entry.id).unwrap().transcript, "只回答 OK。");
        assert_eq!(
            sessions
                .get(&entry.id)
                .unwrap()
                .api_summary
                .observed_call_count,
            0
        );
    }
}
