#[path = "../../desktop/src-tauri/src/capture_startup.rs"]
mod capture_startup;
#[path = "../../desktop/src-tauri/src/diagnostics.rs"]
mod diagnostics;

use diagnostics::{Diagnostics, LogFileStatus, LogLevel, Outcome, Phase, Platform, ReportContext};

fn context(version: &str) -> ReportContext<'_> {
    ReportContext {
        version,
        platform: Platform::Windows,
        log_level: LogLevel::Debug,
        debug_mode: false,
        always_on_microphone: false,
        model_selected: true,
        model_available: true,
        model_is_custom: false,
        local_log: LogFileStatus::Empty,
    }
}

#[test]
fn report_retains_latest_128_stages_with_bounded_durations() {
    let mut diagnostics = Diagnostics::new();
    for _ in 0..130 {
        diagnostics.record(Phase::ModelLoad, Outcome::Started, Some(u64::MAX));
    }
    let report = diagnostics.report(context("0.1.0"));
    assert_eq!(
        report
            .text()
            .lines()
            .filter(|line| line.starts_with("elapsed_ms="))
            .count(),
        128
    );
    assert!(report.text().contains("events_discarded=2"));
    assert!(report.text().contains("duration_ms=86400000"));
    assert!(report.text().len() < 32_768);
}

#[test]
fn public_report_explains_empty_log_and_rejects_private_freeform_metadata() {
    let mut diagnostics = Diagnostics::new();
    diagnostics.record(Phase::ShortcutAccepted, Outcome::Ok, Some(7));
    diagnostics.record(Phase::Transcription, Outcome::Failed, Some(1200));
    let secrets = "Bearer secret-key https://company.invalid/v1 C:\\Users\\private-user\\audio.wav /home/private-user original speech 原稿";
    let report = diagnostics.report(context(secrets));
    assert!(report.text().contains("local_log=empty"));
    assert!(report
        .text()
        .contains("Empty logs do not establish that no errors occurred."));
    assert!(report
        .text()
        .contains("phase=shortcut-accepted outcome=ok duration_ms=7"));
    assert!(report.text().contains("code=transcription-failed"));
    for sentinel in [
        "Bearer",
        "secret-key",
        "company.invalid",
        "private-user",
        "audio.wav",
        "original speech",
        "原稿",
    ] {
        assert!(
            !report.text().contains(sentinel),
            "private sentinel: {sentinel}"
        );
    }
    assert!(report.text().contains("version=unavailable"));
    assert!(report.id().len() <= 32);
}

#[test]
fn report_declares_api_timing_meaning_and_shared_render_limits() {
    let report = Diagnostics::new().report(context("0.1.0"));
    assert!(report.text().contains("notype diagnostic report v2"));
    assert!(report
        .text()
        .contains("report_limit_bytes=65536 report_limit_lines=512"));
    assert!(report
        .text()
        .contains("DNS/TLS subphase timing is unobservable"));
    assert!(report.text().contains("report_truncated=false"));
}
