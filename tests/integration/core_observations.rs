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

#[test]
fn typed_history_observations_cannot_accept_arbitrary_control_labels() {
    let service = SessionService::in_memory().unwrap();
    let observation = SessionObservations {
        latest_outcome: Some("https://private.invalid/arbitrary-data".into()),
        ..Default::default()
    };
    assert!(matches!(
        service.create_observed("original", SessionOptions::default(), observation),
        Err(notype_core::CoreError::InvalidInput)
    ));
    assert!(service.search("").unwrap().is_empty());
    for code in [
        "Bearer private-key",
        "https://internal.test/?token=secret",
        "provider-auth\n",
        "target-changed",
        "back-configuration-required",
    ] {
        assert!(matches!(
            service.create_observed(
                "original",
                SessionOptions::default(),
                SessionObservations {
                    latest_processing_error: Some(code.into()),
                    ..Default::default()
                }
            ),
            Err(notype_core::CoreError::InvalidInput)
        ));
    }
    for code in [
        "provider-auth",
        "back-provider-timeout",
        "configuration-credentials-unavailable",
        "llm-timeout",
    ] {
        SessionObservations {
            latest_processing_error: Some(code.into()),
            ..Default::default()
        }
        .validate()
        .unwrap();
    }
}

#[tokio::test]
async fn separate_processing_and_delivery_errors_export_and_reopen_and_successful_retry_clears_processing_error(
) {
    use notype_core::{
        DeliveryAdapter, DeliveryFuture, DeliveryOutcome, DeliveryRequest, TextFailure,
    };
    struct Reply(bool);
    impl TextProcessor for Reply {
        fn polish(&self, _: TextRequest, _: CancellationToken) -> TextFuture {
            let failed = self.0;
            Box::pin(async move {
                if failed {
                    Err(TextFailure::Timeout)
                } else {
                    Ok("整理 API。".into())
                }
            })
        }
    }
    struct Input;
    impl DeliveryAdapter for Input {
        fn deliver(&self, _: DeliveryRequest) -> DeliveryFuture {
            Box::pin(async { DeliveryOutcome::Unavailable })
        }
    }
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("sessions.db");
    let service = SessionService::open(&path).unwrap();
    let entry = service
        .create(
            "原稿 API 42",
            SessionOptions {
                original_target: Some("captured-target".into()),
                ..Default::default()
            },
        )
        .unwrap();
    let failed = service
        .process(&entry.id, vec![], Arc::new(Reply(true)))
        .await
        .unwrap();
    assert_eq!(
        failed.observations.latest_processing_error.as_deref(),
        Some("llm-timeout")
    );
    service.original_for_delivery(&entry.id).unwrap();
    let entry = service.deliver(&entry.id, Arc::new(Input)).await.unwrap();
    assert_eq!(entry.error.as_deref(), Some("delivery-unavailable"));
    let destination = directory.path().join("errors.csv");
    service
        .export_csv(&destination, &PlentySpace, &CancellationToken::new())
        .unwrap();
    let rows = read_csv(&destination);
    assert_eq!(rows[0]["error"], "delivery-unavailable");
    assert_eq!(rows[0]["latest_processing_error"], "llm-timeout");
    drop(service);
    let service = SessionService::open(&path).unwrap();
    let saved = service.get(&entry.id).unwrap();
    assert_eq!(saved.error.as_deref(), Some("delivery-unavailable"));
    assert_eq!(
        saved.observations.latest_processing_error.as_deref(),
        Some("llm-timeout")
    );
    let retried = service
        .process(&entry.id, vec![], Arc::new(Reply(false)))
        .await
        .unwrap();
    assert_eq!(retried.observations.latest_processing_error, None);
    assert_eq!(retried.error, None);
    assert_eq!(retried.result.as_deref(), Some("整理 API。"));
}

struct Metered {
    model: &'static str,
    input: u64,
    output: u64,
    amount: &'static str,
}

#[derive(Clone, Copy)]
enum ReverseResult {
    Timeout,
    Invalid,
    MissingSource,
    Cancelled,
    Pending,
    Success,
}
struct ReviewFixture {
    reverse: ReverseResult,
    started: Arc<tokio::sync::Notify>,
}
fn review_processor(reverse: ReverseResult) -> Arc<ReviewFixture> {
    Arc::new(ReviewFixture {
        reverse,
        started: Arc::new(tokio::sync::Notify::new()),
    })
}
fn review_session(service: &SessionService) -> notype_core::SessionEntry {
    service
        .create(
            "original API",
            SessionOptions {
                scenario: notype_core::Scenario::Translation,
                translation_target: Some("en-US".into()),
                ..Default::default()
            },
        )
        .unwrap()
}
impl TextProcessor for ReviewFixture {
    fn polish(&self, _: TextRequest, _: CancellationToken) -> TextFuture {
        Box::pin(async { Ok("forward API 42".into()) })
    }
    fn generate_observed(
        &self,
        _: TextRequest,
        _: CancellationToken,
        observer: Arc<dyn ApiCallObserver>,
    ) -> GenerationFuture {
        let missing = matches!(self.reverse, ReverseResult::MissingSource);
        Box::pin(async move {
            let mut call = ApiCallGuard::begin(observer, ApiCallDescriptor::default())?;
            call.mark_sent()?;
            call.response(ApiReceipt {
                usage: TokenUsage {
                    input: Some(5),
                    output: Some(3),
                    ..Default::default()
                },
                ..Default::default()
            });
            call.finish(ApiOutcome::Success);
            Ok(TextOutput {
                text: "forward API 42".into(),
                source_language: (!missing).then(|| "zh-TW".into()),
            })
        })
    }
    fn back_translate_observed(
        &self,
        _: notype_core::BackTranslationRequest,
        cancel: CancellationToken,
        observer: Arc<dyn ApiCallObserver>,
    ) -> TextFuture {
        let reverse = self.reverse;
        let started = self.started.clone();
        Box::pin(async move {
            let mut call = ApiCallGuard::begin(
                observer,
                ApiCallDescriptor {
                    purpose: ApiPurpose::BackTranslation,
                    ..Default::default()
                },
            )?;
            call.mark_sent()?;
            started.notify_one();
            if matches!(reverse, ReverseResult::Pending) {
                cancel.cancelled().await;
            }
            match reverse {
                ReverseResult::Timeout => {
                    call.finish(ApiOutcome::Timeout);
                    Err(notype_core::TextFailure::Timeout)
                }
                ReverseResult::Invalid => {
                    call.finish(ApiOutcome::InvalidResponse);
                    Err(notype_core::TextFailure::InvalidResponse)
                }
                ReverseResult::Success => {
                    call.response(ApiReceipt {
                        usage: TokenUsage {
                            input: Some(2),
                            output: Some(1),
                            ..Default::default()
                        },
                        ..Default::default()
                    });
                    call.finish(ApiOutcome::Success);
                    Ok("回譯 API 42".into())
                }
                _ => {
                    call.finish(ApiOutcome::Cancelled);
                    Err(notype_core::TextFailure::Cancelled)
                }
            }
        })
    }
}

#[tokio::test]
async fn incomplete_review_keeps_forward_and_terminal_evidence_without_initial_ready_endpoint() {
    for (reverse, error, calls) in [
        (ReverseResult::Timeout, "back-translation-timeout", 2),
        (ReverseResult::Invalid, "back-translation-failed", 2),
        (ReverseResult::MissingSource, "back-translation-failed", 1),
        (ReverseResult::Cancelled, "cancelled", 2),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let service = SessionService::open(directory.path().join("sessions.db")).unwrap();
        let original = review_session(&service);
        let entry = service
            .process_with_stop(
                &original.id,
                vec![],
                review_processor(reverse),
                true,
                Some(std::time::Instant::now() - std::time::Duration::from_millis(80)),
            )
            .await
            .unwrap();
        assert_eq!(entry.result.as_deref(), Some("forward API 42"));
        assert_eq!(entry.observations.result_count, Some(3));
        assert_eq!(entry.back_translation, None);
        assert_eq!(entry.error.as_deref(), Some(error));
        assert!(entry.review_required);
        assert!(!entry.review_confirmed);
        assert!(service.confirm_review(&entry.id, entry.revision).is_err());
        assert!(entry.observations.latest_processing_ms.is_some());
        assert!(entry.observations.latest_api_ms.is_some());
        assert_eq!(entry.api_summary.call_count, Some(calls));
        assert_eq!(entry.api_summary.input.known_subtotal, Some(5));
        assert_eq!(entry.observations.initial_stop_to_result_ms, None);
        let destination = directory.path().join("incomplete.csv");
        service
            .export_csv(&destination, &PlentySpace, &CancellationToken::new())
            .unwrap();
        let rows = read_csv(&destination);
        assert_eq!(rows[0]["initial_stop_to_result_ms"], "");
        assert_eq!(rows[0]["result_text"], "forward API 42");
        assert_eq!(rows[0]["result_count"], "3");
        assert_ne!(rows[0]["latest_processing_ms"], "");
        assert_ne!(rows[0]["api_stage_ms"], "");
        // Even a successful retry with a new stop argument cannot invent the first
        // initial-ready endpoint that the incomplete initial pair never reached.
        let retry = service
            .process_with_stop(
                &entry.id,
                vec![],
                review_processor(ReverseResult::Success),
                true,
                Some(std::time::Instant::now()),
            )
            .await
            .unwrap();
        assert!(retry.back_translation.is_some());
        assert_eq!(
            retry.observations.latest_processing_kind.as_deref(),
            Some("retry")
        );
        assert_eq!(retry.observations.initial_stop_to_result_ms, None);
        assert_eq!(retry.api_summary.call_count, Some(calls + 2));
    }
}

#[tokio::test]
async fn cancelling_pending_reverse_keeps_forward_count_and_fenced_unknown_initial_readiness() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("sessions.db");
    let service = SessionService::open(&path).unwrap();
    let original = review_session(&service);
    let processor = review_processor(ReverseResult::Pending);
    let started = processor.started.clone();
    let worker = service.clone();
    let id = original.id.clone();
    let task = tokio::spawn(async move {
        worker
            .process_with_stop(
                &id,
                vec![],
                processor,
                true,
                Some(std::time::Instant::now()),
            )
            .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(2), started.notified())
        .await
        .unwrap();
    let during = service.get(&original.id).unwrap();
    assert_eq!(during.result.as_deref(), Some("forward API 42"));
    assert_eq!(during.observations.result_count, Some(3));
    service.cancel(&original.id).unwrap();
    let cancelled = task.await.unwrap().unwrap();
    assert_eq!(cancelled.status, notype_core::SessionStatus::Cancelled);
    assert_eq!(cancelled.result.as_deref(), Some("forward API 42"));
    assert_eq!(cancelled.observations.result_count, Some(3));
    assert!(cancelled.observations.latest_processing_ms.is_some());
    assert_eq!(cancelled.observations.initial_stop_to_result_ms, None);
    assert_eq!(cancelled.api_summary.call_count, Some(2));
    assert_eq!(cancelled.api_summary.input.known_subtotal, Some(5));
    drop(service);
    let service = SessionService::open(&path).unwrap();
    let saved = service.get(&original.id).unwrap();
    assert_eq!(saved.observations.initial_stop_to_result_ms, None);
    let destination = directory.path().join("cancelled.csv");
    service
        .export_csv(&destination, &PlentySpace, &CancellationToken::new())
        .unwrap();
    let rows = read_csv(&destination);
    assert_eq!(rows[0]["initial_stop_to_result_ms"], "");
    assert_eq!(rows[0]["result_count"], "3");
    assert_ne!(rows[0]["latest_processing_ms"], "");
}

#[tokio::test]
async fn successful_pair_and_original_only_initial_endpoints_survive_subsequent_retries() {
    struct Disabled;
    impl TextProcessor for Disabled {
        fn polish(&self, _: TextRequest, _: CancellationToken) -> TextFuture {
            Box::pin(async { Err(notype_core::TextFailure::ConfigurationRequired) })
        }
    }
    for original_only in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let service = SessionService::open(directory.path().join("sessions.db")).unwrap();
        let original = service
            .create_observed(
                "original API",
                SessionOptions {
                    scenario: notype_core::Scenario::Translation,
                    translation_target: Some("en-US".into()),
                    ..Default::default()
                },
                SessionObservations {
                    asr_after_stop_ms: Some(17),
                    ..Default::default()
                },
            )
            .unwrap();
        let processor: Arc<dyn TextProcessor> = if original_only {
            Arc::new(Disabled)
        } else {
            review_processor(ReverseResult::Success)
        };
        let initial = service
            .process_with_stop(
                &original.id,
                vec![],
                processor,
                true,
                Some(std::time::Instant::now() - std::time::Duration::from_millis(80)),
            )
            .await
            .unwrap();
        let first = initial.observations.initial_stop_to_result_ms.unwrap();
        if original_only {
            assert_eq!(first, 17);
            assert_eq!(
                initial.observations.latest_outcome.as_deref(),
                Some("original_only")
            );
        } else {
            assert!(first >= 80);
            assert!(initial.error.is_none());
            assert!(initial.back_translation.is_some());
            assert_eq!(initial.api_summary.call_count, Some(2));
        }
        let retry = service
            .process_with_stop(
                &original.id,
                vec![],
                review_processor(ReverseResult::Timeout),
                true,
                Some(std::time::Instant::now()),
            )
            .await
            .unwrap();
        assert_eq!(retry.observations.initial_stop_to_result_ms, Some(first));
        assert_eq!(
            retry.observations.latest_processing_kind.as_deref(),
            Some("retry")
        );
        assert_eq!(retry.observations.result_count, Some(3));
        let destination = directory.path().join("retained.csv");
        service
            .export_csv(&destination, &PlentySpace, &CancellationToken::new())
            .unwrap();
        assert_eq!(
            read_csv(&destination)[0]["initial_stop_to_result_ms"],
            first.to_string()
        );
    }
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
    assert_eq!(row["asr_tokens_status"], "not_reported");
    assert_eq!(row["asr_tokens"], "");
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

struct MutateAfterSnapshot {
    service: SessionService,
    delete: String,
}
impl CsvSpaceProbe for MutateAfterSnapshot {
    fn available_bytes(&self, _: &std::path::Path) -> Result<u64, CsvExportError> {
        self.service.delete(&self.delete).unwrap();
        self.service
            .create("new after snapshot", SessionOptions::default())
            .unwrap();
        Ok(u64::MAX)
    }
}
#[test]
fn csv_snapshot_survives_concurrent_delete_and_insert_without_holding_writer() {
    let directory = tempfile::tempdir().unwrap();
    let service = SessionService::open(directory.path().join("sessions.db")).unwrap();
    let old = service
        .create("snapshot original", SessionOptions::default())
        .unwrap();
    let destination = directory.path().join("research.csv");
    service
        .export_csv(
            &destination,
            &MutateAfterSnapshot {
                service: service.clone(),
                delete: old.id.clone(),
            },
            &CancellationToken::new(),
        )
        .unwrap();
    let rows = read_csv(&destination);
    assert_eq!(rows.as_array().unwrap().len(), 1);
    assert_eq!(rows[0]["history_id"], old.id);
    assert_eq!(rows[0]["original_text"], "snapshot original");
    assert_eq!(
        service.search("").unwrap()[0].transcript,
        "new after snapshot"
    );
}
struct NoSpace;
impl CsvSpaceProbe for NoSpace {
    fn available_bytes(&self, _: &std::path::Path) -> Result<u64, CsvExportError> {
        Ok(0)
    }
}
#[test]
fn csv_failure_and_cancellation_preserve_destination_and_leave_no_owned_partial() {
    let directory = tempfile::tempdir().unwrap();
    let service = SessionService::open(directory.path().join("sessions.db")).unwrap();
    service
        .create("original", SessionOptions::default())
        .unwrap();
    let destination = directory.path().join("research.csv");
    std::fs::write(&destination, b"existing destination").unwrap();
    assert_eq!(
        service
            .export_csv(&destination, &NoSpace, &CancellationToken::new())
            .unwrap_err(),
        CsvExportError::SpaceUnavailable
    );
    let cancel = CancellationToken::new();
    cancel.cancel();
    assert_eq!(
        service
            .export_csv(&destination, &PlentySpace, &cancel)
            .unwrap_err(),
        CsvExportError::Cancelled
    );
    assert_eq!(
        std::fs::read(&destination).unwrap(),
        b"existing destination"
    );
    let blocked = directory.path().join("directory.csv");
    std::fs::create_dir(&blocked).unwrap();
    std::fs::write(blocked.join("sentinel"), b"keep").unwrap();
    assert_eq!(
        service
            .export_csv(&blocked, &PlentySpace, &CancellationToken::new())
            .unwrap_err(),
        CsvExportError::WriteFailed
    );
    assert_eq!(std::fs::read(blocked.join("sentinel")).unwrap(), b"keep");
    assert!(std::fs::read_dir(directory.path())
        .unwrap()
        .all(|entry| !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .ends_with(".partial")));
}
#[test]
fn large_csv_exports_every_row_with_fixed_columns_and_exact_unicode() {
    let directory = tempfile::tempdir().unwrap();
    let service = SessionService::open(directory.path().join("sessions.db")).unwrap();
    for i in 0..1200 {
        service
            .create(
                &format!("原稿 API {i},\"line\"\n第二行"),
                SessionOptions::default(),
            )
            .unwrap();
    }
    let destination = directory.path().join("all.csv");
    assert_eq!(
        service
            .export_csv(&destination, &PlentySpace, &CancellationToken::new())
            .unwrap()
            .rows,
        1200
    );
    let rows = read_csv(&destination);
    let rows = rows.as_array().unwrap();
    assert_eq!(rows.len(), 1200);
    let width = rows[0].as_object().unwrap().len();
    assert!(rows
        .iter()
        .all(|row| row.as_object().unwrap().len() == width));
    assert_eq!(
        rows.iter()
            .map(|row| row["history_id"].as_str().unwrap())
            .collect::<std::collections::HashSet<_>>()
            .len(),
        1200
    );
}

struct Receipts(Vec<ApiReceipt>);
impl TextProcessor for Receipts {
    fn polish(&self, _: TextRequest, _: CancellationToken) -> TextFuture {
        Box::pin(async { Ok("result".into()) })
    }
    fn generate_observed(
        &self,
        _: TextRequest,
        _: CancellationToken,
        observer: Arc<dyn ApiCallObserver>,
    ) -> GenerationFuture {
        let receipts = self.0.clone();
        Box::pin(async move {
            for receipt in receipts {
                let mut call = ApiCallGuard::begin(observer.clone(), ApiCallDescriptor::default())?;
                call.mark_sent()?;
                call.response(receipt);
                call.finish(ApiOutcome::Success);
            }
            Ok(TextOutput {
                text: "result".into(),
                source_language: None,
            })
        })
    }
}
#[tokio::test]
async fn independent_partial_usage_and_exact_currencies_never_fake_missing_zero() {
    use notype_core::observations::Completeness;
    let service = SessionService::in_memory().unwrap();
    let entry = service
        .create("original", SessionOptions::default())
        .unwrap();
    let receipts = vec![
        ApiReceipt {
            usage: TokenUsage {
                input: Some(0),
                output: Some(4),
                cached_input: Some(80),
                reasoning_output: Some(3),
                ..Default::default()
            },
            actual_charge: Some(ActualCharge::new("0.1", "USD", "fixture").unwrap()),
            ..Default::default()
        },
        ApiReceipt {
            usage: TokenUsage {
                input: Some(0),
                output: None,
                ..Default::default()
            },
            actual_charge: Some(ActualCharge::new("0.2", "USD", "fixture").unwrap()),
            ..Default::default()
        },
        ApiReceipt {
            usage: TokenUsage {
                input: Some(0),
                output: Some(0),
                ..Default::default()
            },
            actual_charge: Some(ActualCharge::new("12", "TWD", "fixture").unwrap()),
            ..Default::default()
        },
    ];
    let result = service
        .process(&entry.id, vec![], Arc::new(Receipts(receipts)))
        .await
        .unwrap();
    assert_eq!(result.api_summary.input.status, Completeness::Complete);
    assert_eq!(result.api_summary.input.total, Some(0));
    assert_eq!(result.api_summary.output.status, Completeness::Partial);
    assert_eq!(result.api_summary.output.total, None);
    assert_eq!(result.api_summary.output.known_subtotal, Some(4));
    assert_eq!(result.api_summary.output.reported_calls, 2);
    assert_eq!(
        result
            .api_summary
            .fees
            .totals
            .iter()
            .map(|value| (value.currency.as_str(), value.amount.as_str()))
            .collect::<Vec<_>>(),
        vec![("TWD", "12"), ("USD", "0.3")]
    );
    let partial = service
        .process(
            &entry.id,
            vec![],
            Arc::new(Receipts(vec![ApiReceipt::default()])),
        )
        .await
        .unwrap();
    assert_eq!(partial.api_summary.input.status, Completeness::Partial);
    assert_eq!(partial.api_summary.input.known_subtotal, Some(0));
    assert_eq!(partial.api_summary.fees.status, Completeness::Partial);
    assert!(partial.api_summary.fees.totals.is_empty());
    assert_eq!(partial.api_summary.fees.known_subtotals.len(), 2);
}

#[tokio::test]
async fn checked_totals_report_overflow_as_unknown_and_preserve_reported_call_counts() {
    use notype_core::observations::Completeness;
    let service = SessionService::in_memory().unwrap();
    let entry = service
        .create("original", SessionOptions::default())
        .unwrap();
    let receipt = ApiReceipt {
        usage: TokenUsage {
            input: Some(9_007_199_254_740_991),
            ..Default::default()
        },
        actual_charge: Some(ActualCharge::new("100000000000000000000", "USD", "fixture").unwrap()),
        ..Default::default()
    };
    let entry = service
        .process(
            &entry.id,
            vec![],
            Arc::new(Receipts(vec![receipt.clone(), receipt])),
        )
        .await
        .unwrap();
    assert_eq!(entry.api_summary.input.status, Completeness::Unknown);
    assert_eq!(entry.api_summary.input.total, None);
    assert_eq!(entry.api_summary.input.known_subtotal, None);
    assert_eq!(entry.api_summary.input.reported_calls, 2);
    assert_eq!(entry.api_summary.fees.reported_calls, 2);
    assert_eq!(entry.api_summary.fees.status, Completeness::Unknown);
    assert!(entry.api_summary.fees.totals.is_empty());
    assert!(entry.api_summary.fees.known_subtotals.is_empty());
}
struct DelayedReceipt {
    sent: Arc<tokio::sync::Notify>,
    release: Arc<tokio::sync::Notify>,
}
impl TextProcessor for DelayedReceipt {
    fn polish(&self, _: TextRequest, _: CancellationToken) -> TextFuture {
        Box::pin(async { Ok("result".into()) })
    }
    fn generate_observed(
        &self,
        _: TextRequest,
        _: CancellationToken,
        observer: Arc<dyn ApiCallObserver>,
    ) -> GenerationFuture {
        let sent = self.sent.clone();
        let release = self.release.clone();
        Box::pin(async move {
            let mut call = ApiCallGuard::begin(observer, ApiCallDescriptor::default())?;
            call.mark_sent()?;
            sent.notify_one();
            release.notified().await;
            call.response(ApiReceipt {
                usage: TokenUsage {
                    input: Some(9),
                    output: Some(2),
                    ..Default::default()
                },
                ..Default::default()
            });
            call.finish(ApiOutcome::Success);
            Ok(TextOutput {
                text: "late result".into(),
                source_language: None,
            })
        })
    }
}
#[tokio::test]
async fn deleting_during_an_observed_call_cascades_ledger_and_late_completion_cannot_recreate_it() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("sessions.db");
    let service = SessionService::open(&path).unwrap();
    let entry = service
        .create("original", SessionOptions::default())
        .unwrap();
    let sent = Arc::new(tokio::sync::Notify::new());
    let release = Arc::new(tokio::sync::Notify::new());
    let task_service = service.clone();
    let id = entry.id.clone();
    let processor = Arc::new(DelayedReceipt {
        sent: sent.clone(),
        release: release.clone(),
    });
    let task = tokio::spawn(async move { task_service.process(&id, vec![], processor).await });
    sent.notified().await;
    let active = service.get(&entry.id).unwrap();
    assert_eq!(active.api_summary.call_count, None);
    service.delete(&entry.id).unwrap();
    release.notify_one();
    assert!(task.await.unwrap().is_err());
    assert!(service.search("").unwrap().is_empty());
    let connection = rusqlite::Connection::open(&path).unwrap();
    assert_eq!(
        connection
            .query_row("SELECT COUNT(*) FROM api_calls", [], |row| row
                .get::<_, u64>(0))
            .unwrap(),
        0
    );
}
