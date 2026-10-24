#[path = "../../desktop/src-tauri/src/workflow.rs"]
mod workflow;
use notype_core::{RecognitionMode, SessionService, SessionStatus};
use std::sync::Arc;
use workflow::{DesktopWorkflow, TextConfiguration};

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
