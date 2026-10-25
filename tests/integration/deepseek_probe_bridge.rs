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

async fn provider(
    responses: Vec<Value>,
) -> (String, Arc<Mutex<Vec<Value>>>, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}/chat/completions", listener.local_addr().unwrap());
    let received = Arc::new(Mutex::new(Vec::new()));
    let captured = received.clone();
    let task = tokio::spawn(async move {
        for body in responses {
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
            let body = body.to_string();
            stream.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).as_bytes()).await.unwrap();
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
    struct Space;
    impl notype_core::csv::CsvSpaceProbe for Space {
        fn available_bytes(
            &self,
            _: &std::path::Path,
        ) -> Result<u64, notype_core::csv::CsvExportError> {
            Ok(u64::MAX)
        }
    }
    let csv = owned.path().join("history.csv");
    assert_eq!(
        sessions
            .export_csv(&csv, &Space, &Default::default())
            .unwrap()
            .rows,
        5
    );
    let parsed = std::process::Command::new("python")
        .arg("-c").arg("import csv,json,sys; rows=list(csv.DictReader(open(sys.argv[1],encoding='utf-8-sig',newline=''))); row=next(x for x in rows if x['history_id']==sys.argv[2]); assert len(rows)==5; assert row['input_tokens']=='17'; assert row['actual_fee_status']=='unknown'; assert json.loads(row['api_reported_models'])==['unexpected-model']; assert row['initial_stop_to_result_ms']==''; assert 'API1' not in ','.join(row)")
        .arg(csv).arg(&review_id).output().unwrap();
    assert!(
        parsed.status.success(),
        "{}",
        String::from_utf8_lossy(&parsed.stderr)
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
