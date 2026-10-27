#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/capture_startup.rs"]
mod capture_startup;
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/diagnostics.rs"]
mod diagnostics;
#[allow(dead_code, unused_imports)]
#[path = "../../desktop/src-tauri/src/workflow.rs"]
mod workflow;
use notype_core::{RecognitionMode, SessionService, SessionStatus};
use std::sync::Arc;
use workflow::{DesktopWorkflow, TextConfiguration};

#[tokio::test]
async fn disabled_ai_delivers_original_on_finish_and_retry_in_all_three_scenarios_without_http() {
    use notype_core::{
        DeliveryAdapter, DeliveryFuture, DeliveryOutcome, DeliveryRequest, Scenario,
        SessionOptions, TextSelection,
    };
    use std::sync::Mutex;
    use std::time::Duration;
    struct Input(Arc<Mutex<Vec<DeliveryRequest>>>);
    impl DeliveryAdapter for Input {
        fn deliver(&self, request: DeliveryRequest) -> DeliveryFuture {
            self.0.lock().unwrap().push(request);
            Box::pin(async { DeliveryOutcome::Delivered })
        }
    }
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let writes = Arc::new(Mutex::new(vec![]));
    let workflow = DesktopWorkflow::new(SessionService::in_memory().unwrap(), Arc::new(|| {}))
        .with_delivery(Arc::new(Input(writes.clone())));
    let mut configuration = TextConfiguration {
        processing_enabled: false,
        transport: workflow::TextTransport::OpenAiChat,
        endpoint: format!(
            "http://{}/v1/chat/completions",
            listener.local_addr().unwrap()
        ),
        model: "owned-fixture-model".into(),
        authentication: workflow::TextAuthentication::None,
        timeout_ms: 1000,
        ..Default::default()
    };
    configuration.translation.back_translation = true;
    for scenario in [Scenario::Dictation, Scenario::Translation, Scenario::Ask] {
        let generation = workflow
            .begin(
                RecognitionMode::TraditionalEnglish,
                Some("captured-target".into()),
            )
            .unwrap();
        workflow
            .capture_processing_snapshot(generation, scenario, configuration.clone(), vec![])
            .unwrap();
        workflow
            .accept_stop(
                generation,
                std::time::Instant::now() - Duration::from_millis(80),
            )
            .unwrap();
        // The captured master switch wins over preferences supplied after recording.
        let finished = workflow
            .finish(
                generation,
                "原稿 API 42",
                TextConfiguration::default(),
                vec![],
            )
            .await
            .unwrap();
        assert_eq!(finished.scenario, scenario);
        assert_eq!(finished.status, SessionStatus::Delivered);
        assert_eq!(finished.selection, TextSelection::Original);
        assert_eq!(finished.result, None);
        assert!(!finished.review_required);
        assert_eq!(finished.api_summary.call_count, Some(0));
        assert!(finished.observations.asr_after_stop_ms.unwrap() >= 80);
        assert!(finished
            .observations
            .initial_stop_to_result_ms
            .is_some_and(|ms| ms >= finished.observations.asr_after_stop_ms.unwrap()));
        let entry = workflow
            .sessions()
            .create(
                "原稿 API 42",
                SessionOptions {
                    original_target: Some("captured-target".into()),
                    scenario,
                    translation_target: (scenario == Scenario::Translation).then(|| "en".into()),
                    ..Default::default()
                },
            )
            .unwrap();
        let result = workflow
            .retry(&entry.id, configuration.clone(), vec![])
            .await
            .unwrap();
        assert_eq!(result.status, SessionStatus::Delivered);
        assert_eq!(result.selection, TextSelection::Original);
        assert_eq!(result.result, None);
        assert!(!result.review_required);
    }
    let writes = writes.lock().unwrap();
    assert_eq!(writes.len(), 6);
    assert!(writes
        .iter()
        .all(|write| write.text == "原稿 API 42" && write.original_target == "captured-target"));
    assert!(
        tokio::time::timeout(Duration::from_millis(50), listener.accept())
            .await
            .is_err(),
        "AI off still sent an HTTP request"
    );
}

#[tokio::test]
async fn blocking_readiness_feedback_leaves_cancel_restart_and_stop_available() {
    use std::sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc,
    };
    use std::time::Duration;
    for restart in [false, true] {
        let workflow = Arc::new(DesktopWorkflow::new(
            SessionService::in_memory().unwrap(),
            Arc::new(|| {}),
        ));
        let generation = workflow
            .begin(RecognitionMode::TraditionalEnglish, None)
            .unwrap();
        let owner = workflow.token(generation);
        let capture_ready = Arc::new(AtomicBool::new(true));
        let muted = Arc::new(AtomicUsize::new(0));
        let (started, waiting) = mpsc::channel();
        let (release, gate) = mpsc::channel();
        let feedback = {
            let owner = owner.clone();
            let capture_ready = capture_ready.clone();
            let muted = muted.clone();
            std::thread::spawn(move || {
                owner.readiness_feedback(
                    || capture_ready.load(Ordering::SeqCst),
                    || {
                        started.send(()).unwrap();
                        gate.recv().unwrap();
                    },
                    || {
                        muted.fetch_add(1, Ordering::SeqCst);
                    },
                )
            })
        };
        waiting.recv_timeout(Duration::from_secs(2)).unwrap();
        let (done, completed) = mpsc::channel();
        let operation = {
            let workflow = workflow.clone();
            let capture_ready = capture_ready.clone();
            std::thread::spawn(move || {
                let next = if restart {
                    workflow
                        .cancel_active_with(|_| capture_ready.store(false, Ordering::SeqCst))
                        .unwrap();
                    Some(
                        workflow
                            .begin(RecognitionMode::TraditionalEnglish, None)
                            .unwrap(),
                    )
                } else {
                    // The native stop readiness handoff uses this same ownership gate.
                    owner.run(|| capture_ready.store(false, Ordering::SeqCst));
                    None
                };
                done.send(next).unwrap();
            })
        };
        let before_sound_ended = completed.recv_timeout(Duration::from_secs(2));
        release.send(()).unwrap(); // Always release/join, including RED, so no blocked thread leaks.
        operation.join().unwrap();
        feedback.join().unwrap();
        assert!(
            before_sound_ended.is_ok(),
            "cancel/stop waited for the feedback sound"
        );
        assert!(!capture_ready.load(Ordering::SeqCst));
        assert_eq!(
            muted.load(Ordering::SeqCst),
            0,
            "late cue must not mute a stopped or new capture"
        );
        if let Some(next) = before_sound_ended.unwrap() {
            assert_eq!(workflow.active_generation().unwrap(), Some(next));
            let saved = workflow
                .finish(next, "录音 B API", TextConfiguration::default(), vec![])
                .await
                .unwrap();
            assert_eq!(
                workflow.sessions().get(&saved.id).unwrap().transcript,
                "錄音 B API"
            );
        }
    }
}

#[tokio::test]
async fn esc_then_new_recording_survives_old_stop_and_decoder_cleanup_and_saves_original() {
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    };
    for old_stop_already_ran in [false, true] {
        let workflow = Arc::new(DesktopWorkflow::new(
            SessionService::in_memory().unwrap(),
            Arc::new(|| {}),
        ));
        let old = workflow
            .begin(RecognitionMode::TraditionalEnglish, None)
            .unwrap();
        let owner = workflow.token(old);
        let queued_ui_owner = owner.clone();
        let ui = Arc::new(Mutex::new("recording A"));
        let cleanup_count = Arc::new(AtomicUsize::new(0));
        let stop_count = Arc::new(AtomicUsize::new(0));
        let commit_count = Arc::new(AtomicUsize::new(0));
        let (release, gate) = tokio::sync::oneshot::channel();
        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
        let task = {
            let ui = ui.clone();
            let cleanup_count = cleanup_count.clone();
            let stop_count = stop_count.clone();
            let commit_count = commit_count.clone();
            tokio::spawn(async move {
                let _guard = owner.cleanup(move || {
                    *ui.lock().unwrap() = "idle / engine unloaded / cancel removed";
                    cleanup_count.fetch_add(1, Ordering::SeqCst);
                });
                if old_stop_already_ran {
                    owner.run(|| {
                        stop_count.fetch_add(1, Ordering::SeqCst);
                    });
                }
                ready_tx.send(()).unwrap();
                gate.await.unwrap();
                if !old_stop_already_ran {
                    owner.run(|| {
                        stop_count.fetch_add(1, Ordering::SeqCst);
                    });
                }
                // A delayed stop commit (including an extra-buffer wait) must
                // revalidate ownership before draining or closing any recorder.
                owner.run(|| {
                    commit_count.fetch_add(1, Ordering::SeqCst);
                });
                // Actual native late-decoder cancellation uses this public owner.
                owner.cancel().unwrap();
            })
        };
        ready_rx.await.unwrap();
        workflow.cancel_active().unwrap(); // Esc A; no need to await the uncancellable decoder.
        let current = workflow
            .begin(RecognitionMode::TraditionalEnglish, None)
            .unwrap();
        *ui.lock().unwrap() = "recording B / engine retained / cancel installed";
        release.send(()).unwrap();
        task.await.unwrap();
        queued_ui_owner.run(|| {
            *ui.lock().unwrap() = "stale queued overlay hide";
        });
        assert_eq!(workflow.active_generation().unwrap(), Some(current));
        assert_eq!(
            *ui.lock().unwrap(),
            "recording B / engine retained / cancel installed"
        );
        assert_eq!(cleanup_count.load(Ordering::SeqCst), 0);
        assert_eq!(commit_count.load(Ordering::SeqCst), 0);
        assert_eq!(
            stop_count.load(Ordering::SeqCst),
            usize::from(old_stop_already_ran)
        );
        let entry = workflow
            .finish(current, "保留B的录音", TextConfiguration::default(), vec![])
            .await
            .unwrap();
        assert_eq!(
            workflow.sessions().get(&entry.id).unwrap().transcript,
            "保留B的錄音"
        );
        let cleanup_b = Arc::new(AtomicUsize::new(0));
        let completed = cleanup_b.clone();
        let guard_b = workflow.token(current).cleanup(move || {
            completed.fetch_add(1, Ordering::SeqCst);
        });
        drop(guard_b);
        assert_eq!(
            cleanup_b.load(Ordering::SeqCst),
            1,
            "B's own cleanup still runs exactly once"
        );
    }
}

#[tokio::test]
async fn saved_original_converts_slash_prose_and_preserves_explicit_file_paths() {
    for (mode, raw, expected) in [
        (
            RecognitionMode::TraditionalEnglish,
            r"请启用/关闭功能，并保留 /tmp/头发/log.txt 和 src/头发.rs。还要启动/停止录音，使用 ./头发/日志 和 ../头发/日志，保留\\server\头发\log.txt。",
            r"請啟用/關閉功能，並保留 /tmp/头发/log.txt 和 src/头发.rs。還要啟動/停止錄音，使用 ./头发/日志 和 ../头发/日志，保留\\server\头发\log.txt。",
        ),
        (
            RecognitionMode::SimplifiedEnglish,
            r"請啟用/關閉功能，並保留 /tmp/頭髮/log.txt 和 src/頭髮.rs。還要啟動/停止錄音，使用 ./頭髮/日誌 和 ../頭髮/日誌，保留\\server\頭髮\log.txt。",
            r"请启用/关闭功能，并保留 /tmp/頭髮/log.txt 和 src/頭髮.rs。还要启动/停止录音，使用 ./頭髮/日誌 和 ../頭髮/日誌，保留\\server\頭髮\log.txt。",
        ),
    ] {
        let workflow = DesktopWorkflow::new(SessionService::in_memory().unwrap(), Arc::new(|| {}));
        let generation = workflow.begin(mode, None).unwrap();
        let saved = workflow
            .finish(generation, raw, TextConfiguration::default(), vec![])
            .await
            .unwrap();
        assert_eq!(
            workflow.sessions().get(&saved.id).unwrap().transcript,
            expected
        );
    }
}

#[tokio::test]
async fn original_history_preserves_unquoted_paths_while_converting_adjacent_prose() {
    for (mode, raw, expected) in [
        (
            RecognitionMode::TraditionalEnglish,
            r"请打开\\server\头发\log.txt，然后发展。 /tmp/头发/log.txt； ./头发/log.txt ../头发/log.txt 头发/log.txt 头发\log.txt",
            r"請打開\\server\头发\log.txt，然後發展。 /tmp/头发/log.txt； ./头发/log.txt ../头发/log.txt 头发/log.txt 头发\log.txt",
        ),
        (
            RecognitionMode::SimplifiedEnglish,
            r"請打開\\server\頭髮\log.txt，然後發展。 /tmp/頭髮/log.txt； ./頭髮/log.txt ../頭髮/log.txt 頭髮/log.txt 頭髮\log.txt",
            r"请打开\\server\頭髮\log.txt，然后发展。 /tmp/頭髮/log.txt； ./頭髮/log.txt ../頭髮/log.txt 頭髮/log.txt 頭髮\log.txt",
        ),
    ] {
        let workflow = DesktopWorkflow::new(SessionService::in_memory().unwrap(), Arc::new(|| {}));
        let generation = workflow.begin(mode, None).unwrap();
        let entry = workflow
            .finish(generation, raw, TextConfiguration::default(), vec![])
            .await
            .unwrap();
        assert_eq!(entry.transcript, expected);
        assert_eq!(
            workflow.sessions().get(&entry.id).unwrap().transcript,
            expected
        );
    }
}

#[tokio::test]
async fn captured_target_delivery_uses_one_adapter_and_never_retries_uncertain_write() {
    use notype_core::{
        DeliveryAdapter, DeliveryFuture, DeliveryOutcome, DeliveryRequest, SessionOptions,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};
    struct NativeBoundary(Arc<AtomicUsize>);
    impl DeliveryAdapter for NativeBoundary {
        fn deliver(&self, request: DeliveryRequest) -> DeliveryFuture {
            assert_eq!(request.original_target, "opaque-target");
            assert_eq!(request.text, "fixture original");
            self.0.fetch_add(1, Ordering::SeqCst);
            Box::pin(async { DeliveryOutcome::Uncertain })
        }
    }
    let writes = Arc::new(AtomicUsize::new(0));
    let workflow = DesktopWorkflow::new(SessionService::in_memory().unwrap(), Arc::new(|| {}))
        .with_delivery(Arc::new(NativeBoundary(writes.clone())));
    let entry = workflow
        .sessions()
        .create(
            "fixture original",
            SessionOptions {
                original_target: Some("opaque-target".into()),
                ..Default::default()
            },
        )
        .unwrap();
    workflow.choose_original(&entry.id).unwrap();
    let entry = workflow.deliver(&entry.id).await.unwrap();
    assert_eq!(entry.status, SessionStatus::DeliveryUnknown);
    assert_eq!(
        workflow.deliver(&entry.id).await.unwrap().status,
        SessionStatus::DeliveryUnknown
    );
    assert_eq!(writes.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn saved_original_distinguishes_hair_from_development_in_overlapping_words() {
    for (mode, raw, expected) in [
        (RecognitionMode::TraditionalEnglish, "保留发展", "保留發展"),
        (
            RecognitionMode::TraditionalEnglish,
            "保留发展空间",
            "保留發展空間",
        ),
        (
            RecognitionMode::TraditionalEnglish,
            "请打开https://example.com/头发，然后保留发展。",
            "請打開https://example.com/头发，然後保留發展。",
        ),
        (
            RecognitionMode::TraditionalEnglish,
            "留发与发展",
            "留髮與發展",
        ),
        (
            RecognitionMode::TraditionalEnglish,
            "头发、长发、理发、染发",
            "頭髮、長髮、理髮、染髮",
        ),
        (
            RecognitionMode::TraditionalEnglish,
            "保留發展，頭髮與發展。",
            "保留發展，頭髮與發展。",
        ),
        (
            RecognitionMode::SimplifiedEnglish,
            "保留發展，頭髮與發展。",
            "保留发展，头发与发展。",
        ),
    ] {
        let workflow = DesktopWorkflow::new(SessionService::in_memory().unwrap(), Arc::new(|| {}));
        let recording = workflow.begin(mode, None).unwrap();
        let entry = workflow
            .finish(recording, raw, TextConfiguration::default(), vec![])
            .await
            .unwrap();
        assert_eq!(entry.transcript, expected, "{raw}");
    }
}

#[tokio::test]
async fn both_profiles_keep_code_literals_whitespace_and_manual_terms_exact() {
    let literals = "```rust\nlet 头发 = \"頭髮\";\n```\nhttps://example.test/头发?q=42，保留 0.5 和 API_v2。 \"发型\" \"頭髮\"";
    for (mode, raw_prefix, expected_prefix) in [
        (
            RecognitionMode::TraditionalEnglish,
            "保留发展，不要删除发型项目",
            "保留發展，不要刪除发型项目",
        ),
        (
            RecognitionMode::SimplifiedEnglish,
            "保留發展，不要刪除发型项目",
            "保留发展，不要删除发型项目",
        ),
    ] {
        let workflow = DesktopWorkflow::new(SessionService::in_memory().unwrap(), Arc::new(|| {}));
        let recording = workflow
            .begin_with_profile(
                workflow::recognition::RecognitionProfile::new(mode, vec!["发型项目".into()]),
                None,
            )
            .unwrap();
        let raw = format!(" \t{raw_prefix}🙂\r\n{literals}");
        let entry = workflow
            .finish(recording, &raw, TextConfiguration::default(), vec![])
            .await
            .unwrap();
        assert_eq!(
            entry.transcript,
            format!(" \t{expected_prefix}🙂\r\n{literals}")
        );
    }
}

#[tokio::test]
async fn traditional_profile_converts_prose_after_url_sentence_separator() {
    let workflow = DesktopWorkflow::new(SessionService::in_memory().unwrap(), Arc::new(|| {}));
    let recording = workflow
        .begin(RecognitionMode::TraditionalEnglish, None)
        .unwrap();
    let entry = workflow
        .finish(
            recording,
            "请打开https://example.com/头发，然后检查发展。",
            TextConfiguration::default(),
            vec![],
        )
        .await
        .unwrap();
    assert_eq!(
        entry.transcript,
        "請打開https://example.com/头发，然後檢查發展。"
    );
}

#[tokio::test]
async fn simplified_profile_converts_prose_after_path_sentence_separator() {
    let workflow = DesktopWorkflow::new(SessionService::in_memory().unwrap(), Arc::new(|| {}));
    let recording = workflow
        .begin(RecognitionMode::SimplifiedEnglish, None)
        .unwrap();
    let entry = workflow
        .finish(
            recording,
            r"請打開C:\資料\頭髮，然後保留發展。",
            TextConfiguration::default(),
            vec![],
        )
        .await
        .unwrap();
    assert_eq!(entry.transcript, r"请打开C:\資料\頭髮，然后保留发展。");
}

#[tokio::test]
async fn recorded_original_uses_captured_traditional_profile_before_text_processing() {
    let workflow = DesktopWorkflow::new(SessionService::in_memory().unwrap(), Arc::new(|| {}));
    let recording = workflow
        .begin(RecognitionMode::TraditionalEnglish, None)
        .unwrap();
    let entry = workflow
        .finish(
            recording,
            "今天不要删除 API 的 session_id，保留 42 个 request，头发和发展。",
            TextConfiguration::default(),
            vec![],
        )
        .await
        .unwrap();
    assert_eq!(
        entry.transcript,
        "今天不要刪除 API 的 session_id，保留 42 個 request，頭髮和發展。"
    );
    assert_eq!(entry.error.as_deref(), Some("configuration-required"));
}

#[tokio::test]
async fn script_profiles_preserve_literal_code_urls_and_exact_manual_terms() {
    let workflow = DesktopWorkflow::new(SessionService::in_memory().unwrap(), Arc::new(|| {}));
    let recording = workflow
        .begin_with_profile(
            workflow::recognition::RecognitionProfile::new(
                RecognitionMode::TraditionalEnglish,
                vec!["发型项目".into()],
            ),
            None,
        )
        .unwrap();
    let entry = workflow.finish(recording,
        r#"不要删除发型项目，保留 0.5 和 API_v2。`变量 = "发型"` https://example.test/发型?q=42 C:\资料\发型.txt "发型""#,
        TextConfiguration::default(), vec!["发型项目".into()]).await.unwrap();
    assert_eq!(
        entry.transcript,
        r#"不要刪除发型项目，保留 0.5 和 API_v2。`变量 = "发型"` https://example.test/发型?q=42 C:\资料\发型.txt "发型""#
    );
}

#[tokio::test]
async fn unavailable_script_conversion_keeps_original_failed_without_text_retry() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("sessions.db");
    let workflow = DesktopWorkflow::new(SessionService::open(&path).unwrap(), Arc::new(|| {}));
    let recording = workflow
        .begin(RecognitionMode::TraditionalEnglish, None)
        .unwrap();
    let entry = workflow
        .finish_recognition_failure(recording, "可恢复的原稿 API_42")
        .unwrap();
    assert_eq!(entry.transcript, "可恢复的原稿 API_42");
    assert_eq!(entry.status, SessionStatus::Failed);
    assert_eq!(entry.error.as_deref(), Some("recognition-script-failed"));
    assert!(workflow
        .retry(&entry.id, TextConfiguration::default(), vec![])
        .await
        .is_err());
    assert_eq!(
        workflow.sessions().get(&entry.id).unwrap().error,
        entry.error
    );
    drop(workflow);
    let reopened = SessionService::open(path).unwrap();
    assert_eq!(
        reopened.get(&entry.id).unwrap().transcript,
        "可恢复的原稿 API_42"
    );
    assert_eq!(
        reopened.choose_original(&entry.id).unwrap().status,
        SessionStatus::Pending
    );
}

#[tokio::test]
async fn desktop_session_saves_original_without_audio_and_missing_configuration_is_recoverable() {
    let directory = tempfile::tempdir().unwrap();
    let sessions = SessionService::open(directory.path().join("sessions.db")).unwrap();
    let workflow = DesktopWorkflow::new(sessions, Arc::new(|| {}));
    let recording = workflow
        .begin(RecognitionMode::SimplifiedEnglish, None)
        .unwrap();
    let entry = workflow
        .finish(
            recording,
            "不要删除 API_42",
            TextConfiguration::default(),
            vec!["API_42".into()],
        )
        .await
        .unwrap();
    assert_eq!(entry.status, SessionStatus::Pending);
    assert_eq!(entry.selection, notype_core::TextSelection::Original);
    assert_eq!(entry.error.as_deref(), Some("configuration-required"));
    assert_eq!(
        entry.recognition_mode,
        Some(RecognitionMode::SimplifiedEnglish)
    );
    assert_eq!(entry.transcript, "不要删除 API_42");
    assert_eq!(entry.result, None);
    assert_eq!(workflow.sessions().search("API_42").unwrap().len(), 1);
    assert_eq!(
        workflow.choose_original(&entry.id).unwrap().status,
        SessionStatus::Pending
    );
    assert_eq!(
        workflow.deliver(&entry.id).await.unwrap().status,
        SessionStatus::Pending
    );
    drop(workflow);
    let reopened = SessionService::open(directory.path().join("sessions.db")).unwrap();
    assert_eq!(
        reopened.get(&entry.id).unwrap().transcript,
        "不要删除 API_42"
    );
    assert_eq!(
        std::fs::read_dir(directory.path())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "wav"))
            .count(),
        0
    );
}

#[tokio::test]
async fn cancelled_recording_cannot_create_history_or_start_processing_after_stt_returns() {
    let workflow = DesktopWorkflow::new(SessionService::in_memory().unwrap(), Arc::new(|| {}));
    let recording = workflow
        .begin(RecognitionMode::TraditionalEnglish, None)
        .unwrap();
    workflow.cancel_active().unwrap();
    assert!(workflow
        .finish(
            recording,
            "late STT text",
            TextConfiguration::default(),
            vec![]
        )
        .await
        .is_err());
    assert!(workflow.sessions().search("").unwrap().is_empty());
    let next = workflow
        .begin(RecognitionMode::SimplifiedEnglish, None)
        .unwrap();
    assert!(workflow
        .finish(
            recording,
            "old recording",
            TextConfiguration::default(),
            vec![]
        )
        .await
        .is_err());
    let entry = workflow
        .finish(next, "new recording", TextConfiguration::default(), vec![])
        .await
        .unwrap();
    assert_eq!(entry.transcript, "new recording");
    assert_eq!(workflow.sessions().search("").unwrap().len(), 1);
}

#[tokio::test]
async fn received_audio_and_producer_snapshot_survive_stop_scheduling_without_api() {
    use notype_core::observations::{Completeness, ModelIdentity, RecordingObservation};
    struct MemoryDiagnostics;
    impl diagnostics::service::DiagnosticStorage for MemoryDiagnostics {
        fn load_preference(&self) -> Result<Option<bool>, &'static str> {
            Ok(Some(false))
        }
        fn save_preference(&self, _: bool) -> Result<(), &'static str> {
            Ok(())
        }
        fn write_report(&self, _: &std::path::Path, _: &str) -> Result<(), &'static str> {
            Err("unused-fixture")
        }
    }
    let diagnostics = diagnostics::service::global();
    assert!(
        !diagnostics
            .initialize(Arc::new(MemoryDiagnostics))
            .effective_enabled
    );
    let workflow = DesktopWorkflow::new(SessionService::in_memory().unwrap(), Arc::new(|| {}));
    let generation = workflow
        .begin(RecognitionMode::TraditionalEnglish, None)
        .unwrap();
    workflow
        .capture_requested_model(generation, "requested-model".into())
        .unwrap();
    let stopped = std::time::Instant::now() - std::time::Duration::from_millis(80);
    workflow.accept_stop(generation, stopped).unwrap();
    workflow
        .capture_recognition_observation(
            generation,
            Some(RecordingObservation::new(24000, 48000, Some(0)).unwrap()),
            Some(ModelIdentity {
                id: "producer-model".into(),
                name: None,
                backend: Some("fixture".into()),
            }),
        )
        .unwrap();
    let entry = workflow
        .finish(
            generation,
            "我們用 API 測試。",
            TextConfiguration::default(),
            vec![],
        )
        .await
        .unwrap();
    assert_eq!(entry.observations.recording.unwrap().audio_ms(), 500);
    assert_eq!(
        entry.observations.requested_asr_model.as_deref(),
        Some("requested-model")
    );
    assert_eq!(
        entry.observations.actual_asr_model.unwrap().id,
        "producer-model"
    );
    assert!(entry.observations.asr_after_stop_ms.unwrap() >= 80);
    assert!(entry
        .observations
        .initial_stop_to_result_ms
        .is_some_and(|ms| ms >= entry.observations.asr_after_stop_ms.unwrap()));
    assert_eq!(
        entry.observations.latest_outcome.as_deref(),
        Some("original_only")
    );
    assert_eq!(entry.api_summary.input.status, Completeness::NotApplicable);
    let saved = workflow.sessions().get(&entry.id).unwrap();
    assert_eq!(saved.observations.original_count, Some(6));
    let initial_duration = saved.observations.initial_stop_to_result_ms;
    let retried = workflow
        .retry(
            &entry.id,
            TextConfiguration {
                processing_enabled: false,
                ..Default::default()
            },
            vec![],
        )
        .await
        .unwrap();
    assert_eq!(
        retried.observations.initial_stop_to_result_ms, initial_duration,
        "raw retry reused the old capture's clock"
    );
    let observation = serde_json::to_value(saved.observations).unwrap();
    diagnostics.clear();
    assert!(diagnostics.set_enabled(true).effective_enabled);
    diagnostics.clear();
    assert_eq!(
        serde_json::to_value(workflow.sessions().get(&entry.id).unwrap().observations).unwrap(),
        observation
    );
}
