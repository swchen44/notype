#[path = "../../desktop/src-tauri/src/capture_startup.rs"]
mod capture_startup;
#[path = "../../desktop/src-tauri/src/diagnostics.rs"]
mod diagnostics;
use diagnostics::service::{DiagnosticService, DiagnosticStorage};
use diagnostics::{LogFileStatus, LogLevel, Outcome, Phase, Platform, ReportContext};
use std::{path::Path, sync::Arc};
struct Files;
impl DiagnosticStorage for Files {
    fn load_preference(&self) -> Result<Option<bool>, &'static str> {
        Ok(None)
    }
    fn save_preference(&self, _: bool) -> Result<(), &'static str> {
        Ok(())
    }
    fn write_report(&self, path: &Path, text: &str) -> Result<(), &'static str> {
        std::fs::write(path, text).map_err(|_| "fixed-failure")
    }
}

#[test]
fn export_saves_exact_reviewed_snapshot_and_failures_exclude_private_paths() {
    let directory =
        std::env::temp_dir().join(format!("notype-diagnostic-test-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let diagnostics = DiagnosticService::new();
    diagnostics.initialize(Arc::new(Files));
    diagnostics.record(Phase::HistorySave, Outcome::Failed, Some(11));
    let preview = diagnostics.publish_preview(diagnostics.begin_preview().unwrap(),ReportContext {
        version: "Bearer secret-key https://company.invalid/v1 C:\\Users\\private-user\\audio.wav /home/private-user original speech 原稿",
        platform: Platform::Windows,
        log_level: LogLevel::Debug,
        debug_mode: false,
        always_on_microphone: false,
        model_selected: true,
        model_available: true,
        model_is_custom: false,
        local_log: LogFileStatus::Empty,
    }).unwrap();
    let report = &preview.report;
    let ticket = diagnostics.begin_export(report.id()).unwrap();
    diagnostics.record(Phase::Delivery, Outcome::Ok, Some(1));
    let path = directory.join("private-username-original-speech.txt");
    diagnostics.commit_export(&ticket, &path).unwrap();
    let saved = std::fs::read_to_string(&path).unwrap();
    assert_eq!(saved, report.text());
    assert!(saved.contains(report.id()));
    assert!(!saved.contains("phase=delivery"));
    assert!(!saved.contains("private-username-original-speech"));
    for sentinel in [
        "Bearer",
        "secret-key",
        "company.invalid",
        "private-user",
        "audio.wav",
        "original speech",
        "原稿",
    ] {
        assert!(!saved.contains(sentinel), "private sentinel: {sentinel}");
    }
    let failed_path = directory
        .join("private-sentinel-missing-directory")
        .join("export.txt");
    assert_eq!(
        diagnostics.commit_export(&ticket, &failed_path),
        Err("diagnostic-save-failed")
    );
    assert_eq!(
        diagnostics.commit_export(&ticket, &directory.join("private-username.wav")),
        Err("diagnostic-extension-invalid")
    );
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(directory).unwrap();
}
