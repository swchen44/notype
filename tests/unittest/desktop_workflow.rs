#[path = "../../desktop/src-tauri/src/workflow.rs"]
mod workflow;
use notype_core::{RecognitionMode, SessionService, SessionStatus};
use std::sync::Arc;
use workflow::{DesktopWorkflow, TextConfiguration};

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
