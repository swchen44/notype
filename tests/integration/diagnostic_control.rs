#[path = "../../desktop/src-tauri/src/capture_startup.rs"]
mod capture_startup;
#[path = "../../desktop/src-tauri/src/diagnostics.rs"]
mod diagnostics;
#[path = "../../desktop/src-tauri/src/diagnostic_files.rs"]
mod files;
use diagnostics::service::{DiagnosticService, DiagnosticStorage};
use diagnostics::{LogFileStatus, LogLevel, Outcome, Phase, Platform, ReportContext};
use files::FileStorage;
use std::{
    path::{Path, PathBuf},
    sync::{mpsc, Arc, Barrier, Mutex},
    thread,
    time::Duration,
};

struct Fixture(PathBuf);
impl Fixture {
    fn new(name: &str) -> Self {
        let root =
            std::env::temp_dir().join(format!("notype-control-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        Self(root)
    }
    fn storage(&self) -> Arc<FileStorage> {
        Arc::new(FileStorage::new(Some(
            self.0.join("diagnostics-preferences.json"),
        )))
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}
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
fn old_profile_defaults_on_and_explicit_off_applies_before_startup_on_restart() {
    let fixture = Fixture::new("restart");
    let old_settings = fixture.0.join("settings_store.json");
    std::fs::write(&old_settings, "old unrelated settings fixture").unwrap();
    let storage = fixture.storage();
    let service = DiagnosticService::new();
    assert!(service.initialize(storage.clone()).effective_enabled);
    assert!(!service.set_enabled(false).effective_enabled);
    let restarted = DiagnosticService::new();
    assert!(!restarted.initialize(storage).effective_enabled);
    restarted.record(Phase::Startup, Outcome::Started, None);
    let report = restarted
        .publish_preview(restarted.begin_preview().unwrap(), context("0.1.0"))
        .unwrap();
    assert!(report.report.text().contains("events_retained=0"));
    assert_eq!(
        std::fs::read_to_string(old_settings).unwrap(),
        "old unrelated settings fixture"
    );
}

#[test]
fn missing_key_defaults_on_but_malformed_oversized_unreadable_fail_closed() {
    let fixture = Fixture::new("read-errors");
    let preference = fixture.0.join("diagnostics-preferences.json");
    for (content, expected) in [
        ("{\"schema_version\":1}", true),
        ("{\"diagnostics_enabled\":false}", false),
        ("{\"diagnostics_enabled\":\"true\"}", false),
        ("[]", false),
        ("invalid", false),
        ("{\"schema_version\":2}", false),
    ] {
        std::fs::write(&preference, content).unwrap();
        let service = DiagnosticService::new();
        assert_eq!(
            service.initialize(fixture.storage()).effective_enabled,
            expected,
            "{content}"
        );
    }
    std::fs::write(&preference, "x".repeat(4097)).unwrap();
    assert_eq!(
        DiagnosticService::new()
            .initialize(fixture.storage())
            .persistence,
        "load_failed"
    );
    std::fs::remove_file(&preference).unwrap();
    std::fs::create_dir(&preference).unwrap();
    assert_eq!(
        DiagnosticService::new()
            .initialize(fixture.storage())
            .persistence,
        "load_failed"
    );
}

struct ControlledStorage {
    inner: Arc<FileStorage>,
    fail_save: Mutex<bool>,
    write_entered: Option<Arc<Barrier>>,
    write_release: Option<Arc<Barrier>>,
    save_barrier: Option<(Arc<Barrier>, Arc<Barrier>)>,
    save_calls: Mutex<usize>,
}
impl ControlledStorage {
    fn plain(inner: Arc<FileStorage>) -> Self {
        Self {
            inner,
            fail_save: Mutex::new(false),
            write_entered: None,
            write_release: None,
            save_barrier: None,
            save_calls: Mutex::new(0),
        }
    }
}
impl DiagnosticStorage for ControlledStorage {
    fn load_preference(&self) -> Result<Option<bool>, &'static str> {
        self.inner.load_preference()
    }
    fn save_preference(&self, enabled: bool) -> Result<(), &'static str> {
        let first = {
            let mut count = self.save_calls.lock().unwrap();
            *count += 1;
            *count == 1
        };
        if first {
            if let Some((entered, release)) = &self.save_barrier {
                entered.wait();
                release.wait();
            }
        }
        if *self.fail_save.lock().unwrap() {
            return Err("synthetic-save-failure");
        }
        self.inner.save_preference(enabled)
    }
    fn write_report(&self, path: &Path, text: &str) -> Result<(), &'static str> {
        if let Some(entered) = &self.write_entered {
            entered.wait();
        }
        if let Some(release) = &self.write_release {
            release.wait();
        }
        self.inner.write_report(path, text)
    }
}

#[test]
fn failed_off_save_stays_off_unsaved_and_restart_uses_actual_disk_preference() {
    let fixture = Fixture::new("save-errors");
    let storage = Arc::new(ControlledStorage::plain(fixture.storage()));
    let service = DiagnosticService::new();
    service.initialize(storage.clone());
    assert!(service.set_enabled(true).effective_enabled);
    *storage.fail_save.lock().unwrap() = true;
    let off = service.set_enabled(false);
    assert!(!off.effective_enabled);
    assert_eq!(off.persistence, "unsaved");
    assert_eq!(off.saved_preference, Some(true));
    service.record(Phase::Transcription, Outcome::Ok, Some(20));
    let preview = service
        .publish_preview(service.begin_preview().unwrap(), context("0.1.0"))
        .unwrap();
    assert!(preview.report.text().contains("events_retained=0"));
    service.clear();
    assert_eq!(service.status().persistence, "unsaved");
    assert!(!service.set_enabled(true).effective_enabled); // failed ON cannot enable
    assert!(
        DiagnosticService::new()
            .initialize(fixture.storage())
            .effective_enabled
    );
    *storage.fail_save.lock().unwrap() = false;
    assert!(!service.set_enabled(false).effective_enabled);
    assert!(
        !DiagnosticService::new()
            .initialize(fixture.storage())
            .effective_enabled
    );
}

#[test]
fn clear_during_context_gather_rejects_old_preview_publication() {
    let fixture = Fixture::new("context");
    let service = DiagnosticService::new();
    service.initialize(fixture.storage());
    let pending = service.begin_preview().unwrap();
    service.clear();
    assert_eq!(
        service.publish_preview(pending, context("0.1.0")).err(),
        Some("diagnostic-preview-expired")
    );
}

#[test]
fn clear_first_rejects_dialog_return_without_touching_destination() {
    let fixture = Fixture::new("clear-first");
    let service = DiagnosticService::new();
    service.initialize(fixture.storage());
    let preview = service
        .publish_preview(service.begin_preview().unwrap(), context("0.1.0"))
        .unwrap();
    let ticket = service.begin_export(preview.report.id()).unwrap();
    let existing = fixture.0.join("existing.txt");
    let missing = fixture.0.join("missing.txt");
    std::fs::write(&existing, "DESTINATION SENTINEL").unwrap();
    service.clear(); // native dialog has not returned yet
    for path in [&existing, &missing] {
        assert_eq!(
            service.commit_export(&ticket, path),
            Err("diagnostic-preview-expired")
        );
    }
    assert_eq!(
        std::fs::read_to_string(existing).unwrap(),
        "DESTINATION SENTINEL"
    );
    assert!(!missing.exists());
    assert!(!std::fs::read_dir(&fixture.0).unwrap().any(|entry| entry
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with(".notype-")));
}

#[test]
fn export_first_blocks_clear_ack_until_write_and_cleanup_finish() {
    for fail in [false, true] {
        let fixture = Fixture::new(if fail {
            "export-first-fail"
        } else {
            "export-first"
        });
        let entered = Arc::new(Barrier::new(2));
        let release = Arc::new(Barrier::new(2));
        let mut storage = ControlledStorage::plain(fixture.storage());
        storage.write_entered = Some(entered.clone());
        storage.write_release = Some(release.clone());
        let service = Arc::new(DiagnosticService::new());
        service.initialize(Arc::new(storage));
        let preview = service
            .publish_preview(service.begin_preview().unwrap(), context("0.1.0"))
            .unwrap();
        let ticket = service.begin_export(preview.report.id()).unwrap();
        let destination = fixture.0.join("report.txt");
        if fail {
            std::fs::create_dir(&destination).unwrap();
            std::fs::write(destination.join("sentinel"), "OLD SENTINEL").unwrap();
        } else {
            std::fs::write(&destination, "OLD SENTINEL").unwrap();
        }
        let writer = {
            let service = service.clone();
            let destination = destination.clone();
            thread::spawn(move || service.commit_export(&ticket, &destination))
        };
        entered.wait(); // commit gate is held; the writer is inside final I/O
        service.record(Phase::MicrophoneReady, Outcome::Ok, Some(8));
        assert!(!service.set_enabled(false).effective_enabled); // collector/preference locks are free
        let (started_tx, started_rx) = mpsc::channel();
        let (done_tx, done_rx) = mpsc::channel();
        let clearer = {
            let service = service.clone();
            thread::spawn(move || {
                started_tx.send(()).unwrap();
                let epoch = service.clear();
                done_tx.send(epoch).unwrap();
            })
        };
        started_rx.recv().unwrap();
        assert!(done_rx.recv_timeout(Duration::from_millis(30)).is_err());
        release.wait();
        assert_eq!(
            writer.join().unwrap(),
            if fail {
                Err("diagnostic-save-failed")
            } else {
                Ok(())
            }
        );
        assert_eq!(done_rx.recv_timeout(Duration::from_secs(2)).unwrap(), 1);
        clearer.join().unwrap();
        let content = std::fs::read_to_string(if fail {
            destination.join("sentinel")
        } else {
            destination.clone()
        })
        .unwrap();
        assert_eq!(
            content,
            if fail {
                "OLD SENTINEL"
            } else {
                preview.report.text()
            }
        );
        assert!(!std::fs::read_dir(&fixture.0).unwrap().any(|entry| entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".notype-")));
        assert!(service.begin_export(preview.report.id()).is_err());
    }
}

#[test]
fn serialized_rapid_toggles_and_restart_agree_with_last_request() {
    for first in [false, true] {
        let fixture = Fixture::new(if first { "on-off" } else { "off-on" });
        let entered = Arc::new(Barrier::new(2));
        let release = Arc::new(Barrier::new(2));
        let mut storage = ControlledStorage::plain(fixture.storage());
        storage.save_barrier = Some((entered.clone(), release.clone()));
        let service = Arc::new(DiagnosticService::new());
        service.initialize(Arc::new(storage));
        let a = {
            let service = service.clone();
            thread::spawn(move || service.set_enabled(first))
        };
        entered.wait();
        assert!(!service.status().effective_enabled);
        let pending = service.status();
        assert_eq!(pending.persistence, "pending");
        service.clear();
        let cleared = service.status();
        assert_eq!(
            (
                cleared.persistence,
                cleared.revision,
                cleared.desired_enabled,
                cleared.saved_preference
            ),
            (
                pending.persistence,
                pending.revision,
                pending.desired_enabled,
                pending.saved_preference
            )
        );
        let (started, ready) = mpsc::channel();
        let b = {
            let service = service.clone();
            thread::spawn(move || {
                started.send(()).unwrap();
                service.set_enabled(!first)
            })
        };
        ready.recv().unwrap();
        release.wait();
        assert_eq!(a.join().unwrap().revision, 1);
        let last = b.join().unwrap();
        assert_eq!(last.revision, 2);
        assert_eq!(last.effective_enabled, !first);
        assert_eq!(service.status().effective_enabled, !first);
        assert_eq!(
            DiagnosticService::new()
                .initialize(fixture.storage())
                .effective_enabled,
            !first
        );
    }
}

#[test]
fn actual_preview_export_omits_all_secret_inputs_and_atomic_failure_preserves_existing_files() {
    let fixture = Fixture::new("privacy-export");
    let service = DiagnosticService::new();
    service.initialize(fixture.storage());
    let unsafe_version="Bearer secret-key\nAPI_Key=SECOND-secret\nToKeN=短秘密\nurl=https://company.invalid/?token=third\njwt=header.payload.signature\nencoded=%61bcdef\nC:\\Users\\private-user\\audio.wav original speech 原稿";
    let preview = service
        .publish_preview(service.begin_preview().unwrap(), context(unsafe_version))
        .unwrap();
    let ticket = service.begin_export(preview.report.id()).unwrap();
    let destination = fixture.0.join("reviewed.txt");
    service.commit_export(&ticket, &destination).unwrap();
    let exported = std::fs::read_to_string(&destination).unwrap();
    assert_eq!(exported, preview.report.text());
    for sentinel in [
        "Bearer",
        "secret-key",
        "SECOND-secret",
        "短秘密",
        "company.invalid",
        "header.payload",
        "%61bcdef",
        "private-user",
        "audio.wav",
        "original speech",
        "原稿",
    ] {
        assert!(!exported.contains(sentinel), "{sentinel}");
    }
    assert!(exported.contains("version=unavailable"));
    let failed_destination = fixture.0.join("directory.txt");
    std::fs::create_dir(&failed_destination).unwrap();
    std::fs::write(failed_destination.join("sentinel"), "KEEP").unwrap();
    assert_eq!(
        service.commit_export(&ticket, &failed_destination),
        Err("diagnostic-save-failed")
    );
    assert_eq!(
        std::fs::read_to_string(failed_destination.join("sentinel")).unwrap(),
        "KEEP"
    );
    assert!(!std::fs::read_dir(&fixture.0).unwrap().any(|entry| entry
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with(".notype-")));
    service.clear();
    assert_eq!(std::fs::read_to_string(destination).unwrap(), exported);
}
