#[path = "../../desktop/src-tauri/src/diagnostics.rs"]
mod diagnostics;
use diagnostics::service::{DiagnosticService, DiagnosticStorage};
use diagnostics::{LogFileStatus, LogLevel, Outcome, Phase, Platform, ReportContext};
use std::{
    path::Path,
    sync::{Arc, Mutex},
};

struct Storage {
    saved: Mutex<Option<bool>>,
    fail: Mutex<bool>,
}
impl DiagnosticStorage for Storage {
    fn load_preference(&self) -> Result<Option<bool>, &'static str> {
        Ok(*self.saved.lock().unwrap())
    }
    fn save_preference(&self, enabled: bool) -> Result<(), &'static str> {
        if *self.fail.lock().unwrap() {
            return Err("synthetic-failure");
        }
        *self.saved.lock().unwrap() = Some(enabled);
        Ok(())
    }
    fn write_report(&self, _: &Path, _: &str) -> Result<(), &'static str> {
        Ok(())
    }
}
fn context() -> ReportContext<'static> {
    ReportContext {
        version: "0.1.0",
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
fn uninitialized_and_persisted_off_drop_startup_and_failed_off_save_never_resumes_collection() {
    let service = DiagnosticService::new();
    service.record(Phase::Startup, Outcome::Started, None);
    assert!(!service.status().effective_enabled);
    let storage = Arc::new(Storage {
        saved: Mutex::new(Some(false)),
        fail: Mutex::new(false),
    });
    assert!(!service.initialize(storage.clone()).effective_enabled);
    let preview = service
        .publish_preview(service.begin_preview().unwrap(), context())
        .unwrap();
    assert!(preview.report.text().contains("events_retained=0"));
    assert!(service.set_enabled(true).effective_enabled);
    service.record(Phase::Startup, Outcome::Ok, Some(5));
    *storage.fail.lock().unwrap() = true;
    let off = service.set_enabled(false);
    assert!(!off.effective_enabled);
    assert_eq!(off.persistence, "unsaved");
    assert_eq!(off.saved_preference, Some(true));
    service.record(Phase::Transcription, Outcome::Ok, Some(7));
    let preview = service
        .publish_preview(service.begin_preview().unwrap(), context())
        .unwrap();
    assert!(preview.report.text().contains("events_retained=1"));
    assert!(!preview.report.text().contains("phase=transcription"));
    let epoch = service.clear();
    assert_eq!(service.status().epoch, epoch);
    assert_eq!(service.status().persistence, "unsaved");
    assert!(!service.status().effective_enabled);
}
