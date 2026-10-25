#[path = "../../desktop/src-tauri/src/diagnostics.rs"]
mod diagnostics;
use diagnostics::{Diagnostics, LogFileStatus, LogLevel, Outcome, Phase, Platform, ReportContext};

#[test]
fn export_saves_exact_reviewed_snapshot_and_failures_exclude_private_paths() {
    let directory =
        std::env::temp_dir().join(format!("notype-diagnostic-test-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let mut diagnostics = Diagnostics::new();
    diagnostics.record(Phase::HistorySave, Outcome::Failed, Some(11));
    let report = diagnostics.report(ReportContext {
        version: "Bearer secret-key https://company.invalid/v1 C:\\Users\\private-user\\audio.wav /home/private-user original speech 原稿",
        platform: Platform::Windows,
        log_level: LogLevel::Debug,
        debug_mode: false,
        always_on_microphone: false,
        model_selected: true,
        model_available: true,
        model_is_custom: false,
        local_log: LogFileStatus::Empty,
    });
    diagnostics.record(Phase::Delivery, Outcome::Ok, Some(1));
    let path = directory.join("private-username-original-speech.txt");
    report.export_to(&path).unwrap();
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
        report.export_to(&failed_path),
        Err("diagnostic-save-failed")
    );
    assert_eq!(
        report.export_to(&directory.join("private-username.wav")),
        Err("diagnostic-extension-invalid")
    );
    std::fs::remove_file(path).unwrap();
    std::fs::remove_dir(directory).unwrap();
}
