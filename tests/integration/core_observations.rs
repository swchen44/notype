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
