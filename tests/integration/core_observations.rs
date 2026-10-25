use notype_core::csv::{CsvExportError, CsvSpaceProbe};
use notype_core::observations::{
    ActualCharge, ApiCallDescriptor, ApiCallGuard, ApiCallObserver, ApiOutcome, ApiPurpose,
    ApiReceipt, TokenUsage,
};
use notype_core::{
    observations::{ModelIdentity, RecordingObservation, SessionObservations},
    SessionOptions, SessionService,
};
use notype_core::{
    CancellationToken, GenerationFuture, TextFuture, TextOutput, TextProcessor, TextRequest,
};
use std::sync::Arc;

#[test]
fn fresh_history_persists_received_audio_models_and_both_text_counts() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("sessions.db");
    let service = SessionService::open(&path).unwrap();
    let entry = service
        .create_observed(
            "我們用 API 測試。",
            SessionOptions::default(),
            SessionObservations {
                recording: Some(RecordingObservation::new(24_000, 48_000, Some(0)).unwrap()),
                requested_asr_model: Some("selected-model".into()),
                actual_asr_model: Some(ModelIdentity {
                    id: "producer-model".into(),
                    name: Some("Producer".into()),
                    backend: Some("fixture".into()),
                }),
                ..SessionObservations::default()
            },
        )
        .unwrap();
    assert_eq!(
        entry.observations.recording.as_ref().unwrap().audio_ms(),
        500
    );
    assert_eq!(entry.observations.original_count, Some(6));
    assert_eq!(entry.observations.result_count, None);
    drop(service);
    let saved = SessionService::open(&path).unwrap().get(&entry.id).unwrap();
    assert_eq!(
        saved.observations.actual_asr_model.unwrap().id,
        "producer-model"
    );
    assert_eq!(saved.observations.recording.unwrap().audio_ms(), 500);
    assert_eq!(saved.created_at, entry.created_at);
}

struct Metered {
    model: &'static str,
    input: u64,
    output: u64,
    amount: &'static str,
}
impl TextProcessor for Metered {
    fn polish(&self, _: TextRequest, _: CancellationToken) -> TextFuture {
        Box::pin(async { Ok("整理 API。".into()) })
    }
    fn generate_observed(
        &self,
        _: TextRequest,
        _: CancellationToken,
        observer: Arc<dyn ApiCallObserver>,
    ) -> GenerationFuture {
        let (model, input, output, amount) = (self.model, self.input, self.output, self.amount);
        Box::pin(async move {
            let mut call = ApiCallGuard::begin(
                observer,
                ApiCallDescriptor {
                    purpose: ApiPurpose::Generation,
                    requested_model: Some(model.into()),
                    requested_effort: Some("none".into()),
                    ..Default::default()
                },
            )?;
            call.mark_sent()?;
            call.response(ApiReceipt {
                reported_model: Some(model.into()),
                usage: TokenUsage {
                    input: Some(input),
                    output: Some(output),
                    ..Default::default()
                },
                actual_charge: Some(
                    ActualCharge::new(amount, "USD", "fixture-returned-charge").unwrap(),
                ),
                ..Default::default()
            });
            call.finish(ApiOutcome::Success);
            Ok(TextOutput {
                text: "整理 API。".into(),
                source_language: None,
            })
        })
    }
}

#[tokio::test]
async fn history_sums_every_reported_call_without_erasing_retry_models_or_efforts() {
    let service = SessionService::in_memory().unwrap();
    let entry = service
        .create("原稿 API", SessionOptions::default())
        .unwrap();
    service
        .process(
            &entry.id,
            vec![],
            Arc::new(Metered {
                model: "model-a",
                input: 120,
                output: 30,
                amount: "0.001",
            }),
        )
        .await
        .unwrap();
    let entry = service
        .process(
            &entry.id,
            vec![],
            Arc::new(Metered {
                model: "model-b",
                input: 20,
                output: 10,
                amount: "0.002",
            }),
        )
        .await
        .unwrap();
    assert_eq!(entry.api_summary.call_count, Some(2));
    assert_eq!(entry.api_summary.input.total, Some(140));
    assert_eq!(entry.api_summary.output.total, Some(40));
    assert_eq!(entry.api_summary.fees.totals[0].amount, "0.003");
    assert_eq!(
        entry.api_summary.requested_models,
        vec!["model-a", "model-b"]
    );
    assert_eq!(entry.api_summary.model_efforts.len(), 2);
    assert_eq!(entry.observations.original_count, Some(3));
    assert_eq!(entry.observations.result_count, Some(3));
    service.delete(&entry.id).unwrap();
    assert!(service.get(&entry.id).is_err());
}

#[test]
fn nonempty_unversioned_database_requires_reset_without_modifying_its_data() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("old.db");
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection
        .execute_batch(
            "CREATE TABLE old_notes(note TEXT); INSERT INTO old_notes VALUES ('keep unchanged');",
        )
        .unwrap();
    drop(connection);
    let before = std::fs::read(&path).unwrap();
    assert!(matches!(
        SessionService::open(&path),
        Err(notype_core::CoreError::ResetRequired)
    ));
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

struct PlentySpace;
impl CsvSpaceProbe for PlentySpace {
    fn available_bytes(&self, _: &std::path::Path) -> Result<u64, CsvExportError> {
        Ok(u64::MAX)
    }
}
fn read_csv(path: &std::path::Path) -> serde_json::Value {
    let executable = std::env::var_os("NOTYPE_TEST_PYTHON")
        .unwrap_or_else(|| if cfg!(windows) { "python" } else { "python3" }.into());
    let mut command = std::process::Command::new(executable);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let output=command.args(["-X","utf8","-c","import csv,json,sys; f=open(sys.argv[1],encoding='utf-8-sig',newline=''); print(json.dumps(list(csv.DictReader(f)),ensure_ascii=False))"]).arg(path).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
#[tokio::test]
async fn one_utf8_csv_row_joins_exact_texts_and_every_api_total_by_history_id() {
    let directory = tempfile::tempdir().unwrap();
    let service = SessionService::open(directory.path().join("sessions.db")).unwrap();
    let original = "=SUM(A1:A2)\r\n第一行,\"API\"";
    let entry = service
        .create_observed(
            original,
            SessionOptions::default(),
            SessionObservations {
                recording: Some(RecordingObservation::new(24_000, 48_000, Some(0)).unwrap()),
                ..Default::default()
            },
        )
        .unwrap();
    service
        .process(
            &entry.id,
            vec![],
            Arc::new(Metered {
                model: "model-a",
                input: 120,
                output: 30,
                amount: "0.001",
            }),
        )
        .await
        .unwrap();
    service
        .process(
            &entry.id,
            vec![],
            Arc::new(Metered {
                model: "model-b",
                input: 20,
                output: 10,
                amount: "0.002",
            }),
        )
        .await
        .unwrap();
    let destination = directory.path().join("research.csv");
    let exported = service
        .export_csv(&destination, &PlentySpace, &CancellationToken::new())
        .unwrap();
    assert_eq!(exported.rows, 1);
    assert!(std::fs::read(&destination)
        .unwrap()
        .starts_with(&[0xef, 0xbb, 0xbf]));
    let rows = read_csv(&destination);
    assert_eq!(rows.as_array().unwrap().len(), 1);
    let row = &rows[0];
    assert_eq!(row["history_id"], entry.id);
    assert_eq!(row["original_text"], original);
    assert_eq!(row["result_text"], "整理 API。");
    assert_eq!(row["original_count"], "7");
    assert_eq!(row["result_count"], "3");
    assert_eq!(row["recording_audio_seconds"], "0.500");
    assert_eq!(row["input_tokens"], "140");
    assert_eq!(row["output_tokens"], "40");
    let amounts: serde_json::Value =
        serde_json::from_str(row["actual_fee_totals_by_currency"].as_str().unwrap()).unwrap();
    assert_eq!(
        amounts,
        serde_json::json!([{"currency":"USD","amount":"0.003"}])
    );
    let models: serde_json::Value =
        serde_json::from_str(row["api_requested_models"].as_str().unwrap()).unwrap();
    assert_eq!(models, serde_json::json!(["model-a", "model-b"]));
}
