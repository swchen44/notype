#[path = "../../desktop/src-tauri/src/workflow.rs"]
mod workflow;
use notype_core::{RecognitionMode, SessionService, SessionStatus};
use std::sync::Arc;
use workflow::{DesktopWorkflow, TextConfiguration};

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
    assert_eq!(entry.status, SessionStatus::Failed);
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
