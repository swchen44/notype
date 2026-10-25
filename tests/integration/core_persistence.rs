use notype_core::{CoreError, RecognitionMode, SessionOptions, SessionService, SessionStatus};

#[cfg(unix)]
#[test]
fn last_service_owner_releases_store_while_inherited_pre_exec_child_is_held() {
    use std::{
        io::{Read, Write},
        net::Shutdown,
        os::unix::{net::UnixStream, process::CommandExt},
        process::{Command, ExitStatus, Stdio},
        thread::JoinHandle,
        time::Duration,
    };
    const CHILD: &str = "NOTYPE_CORE_OWNED_PREEXEC_CHILD";
    if std::env::var_os(CHILD).as_deref() == Some(std::ffi::OsStr::new("1")) {
        return; // Only this selected helper body executes after the owned exec.
    }
    struct HeldChild {
        release: UnixStream,
        worker: Option<JoinHandle<std::io::Result<ExitStatus>>>,
    }
    impl HeldChild {
        fn finish(&mut self) -> std::thread::Result<std::io::Result<ExitStatus>> {
            let _ = self.release.write(&[1]);
            let _ = self.release.shutdown(Shutdown::Both);
            self.worker
                .take()
                .map(|worker| worker.join())
                .unwrap_or_else(|| Ok(Err(std::io::Error::from_raw_os_error(5))))
        }
    }
    impl Drop for HeldChild {
        fn drop(&mut self) {
            if self.worker.is_some() {
                let _ = self.finish();
            }
        }
    }
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("fork-owned.db");
    let first = SessionService::open(&path).unwrap();
    let held = first.clone();
    assert!(matches!(
        SessionService::open(&path),
        Err(CoreError::AlreadyOwned)
    ));
    drop(first);
    let (parent_stream, child_stream) = UnixStream::pair().unwrap();
    // Configure both child and parent deadlines before fork. Drop always releases
    // the byte gate and joins/reaps the owned child, including setup/assert errors.
    for stream in [&parent_stream, &child_stream] {
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(10)))
            .unwrap();
    }
    let executable = std::env::current_exe().unwrap();
    let worker = std::thread::spawn(move || {
        let mut command = Command::new(executable);
        command
            .args([
                "--exact",
                "last_service_owner_releases_store_while_inherited_pre_exec_child_is_held",
            ])
            .env(CHILD, "1")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        // SAFETY: all allocation, socket creation and timeout configuration happen
        // before fork. The child closure only borrows its precreated socket and
        // performs bounded one-byte read/write syscalls. It takes no locks, uses
        // no SQLite/async/logging/environment/formatting, and creates only stack
        // bytes or an allocation-free OS error. Captured streams stay alive until
        // exec/error; the parent owns the matching release endpoint and cleanup.
        unsafe {
            command.pre_exec(move || {
                let mut stream = &child_stream;
                let mut byte = [0];
                if stream.write(&[1])? != 1 || stream.read(&mut byte)? != 1 || byte != [1] {
                    return Err(std::io::Error::from_raw_os_error(5));
                }
                Ok(())
            });
        }
        command.spawn()?.wait()
    });
    let mut child = HeldChild {
        release: parent_stream,
        worker: Some(worker),
    };
    let mut byte = [0];
    let handshake = child.release.read_exact(&mut byte);
    // Store every result until AFTER cleanup. Reopen happens before release,
    // while the forked child necessarily still retains the original descriptor.
    let clone_blocks = matches!(SessionService::open(&path), Err(CoreError::AlreadyOwned));
    drop(held);
    let reopened = SessionService::open(&path);
    let completed = child.finish();
    assert!(
        handshake.is_ok() && byte == [1],
        "owned pre-exec handshake failed: {handshake:?}"
    );
    assert!(
        completed.unwrap().unwrap().success(),
        "owned child did not exit successfully"
    );
    assert!(
        clone_blocks,
        "a live service clone must retain exclusive ownership"
    );
    assert!(
        reopened.is_ok(),
        "final service owner must release before child exec; got {:?}",
        reopened.err()
    );
}

#[test]
fn translation_scene_and_target_survive_sqlite_restart_as_immutable_entry_metadata() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("scene-history.db");
    let service = SessionService::open(&path).unwrap();
    let entry = service
        .create(
            "不要删除 API_42",
            SessionOptions {
                recognition_mode: RecognitionMode::SimplifiedEnglish,
                original_target: None,
                scenario: notype_core::Scenario::Translation,
                translation_target: Some("en-US".into()),
                ..SessionOptions::default()
            },
        )
        .unwrap();
    drop(service);
    let saved = SessionService::open(&path).unwrap().get(&entry.id).unwrap();
    assert_eq!(saved.scenario, notype_core::Scenario::Translation);
    assert_eq!(saved.translation_target.as_deref(), Some("en-US"));
    assert_eq!(saved.transcript, "不要删除 API_42");
    assert_eq!(saved.created_at, entry.created_at);
    assert_eq!(saved.status, SessionStatus::Recognized);
}

#[test]
fn old_pre_release_history_requires_an_explicit_reset_without_rewriting_text() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("core-v2.db");
    let fixture = rusqlite::Connection::open(&path).unwrap();
    fixture.execute_batch("CREATE TABLE sessions (
        id TEXT PRIMARY KEY,transcript TEXT NOT NULL,result TEXT,created_at TEXT NOT NULL,updated_at TEXT NOT NULL,
        status TEXT NOT NULL,error TEXT,recognition_mode TEXT,original_target TEXT,
        selection TEXT NOT NULL DEFAULT 'polished',attempt INTEGER NOT NULL DEFAULT 0
    ); INSERT INTO sessions VALUES ('legacy-id','不要删除 API 42','不要删除 API 42。','2026-10-24T00:00:00.000Z','2026-10-24T00:01:00.000Z','pending',NULL,'simplified_english','old-target','polished',1); PRAGMA user_version=2;").unwrap();
    drop(fixture);
    let before = std::fs::read(&path).unwrap();
    assert!(matches!(
        SessionService::open(&path),
        Err(CoreError::ResetRequired)
    ));
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn stored_context_exposes_only_the_allowed_application_name() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("earlier-v3.db");
    let service = SessionService::open(&path).unwrap();
    let entry = service
        .create("preserve original", SessionOptions::default())
        .unwrap();
    drop(service);
    let fixture = rusqlite::Connection::open(&path).unwrap();
    fixture
        .execute(
            "UPDATE sessions SET source_context=?2 WHERE id=?1",
            rusqlite::params![
                entry.id,
                r#"{"application":"Test Editor","website_domain":"legacy.example"}"#
            ],
        )
        .unwrap();
    drop(fixture);
    let reopened = SessionService::open(&path).unwrap();
    let saved = reopened.get(&entry.id).unwrap();
    assert_eq!(saved.transcript, "preserve original");
    assert_eq!(saved.created_at, entry.created_at);
    assert_eq!(
        serde_json::to_value(saved.source_context).unwrap(),
        serde_json::json!({"application":"Test Editor"})
    );
}

#[test]
fn original_and_captured_profile_survive_sqlite_restart_without_a_recording() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory
        .path()
        .join("version-independent")
        .join("history.db");
    let service = SessionService::open(&path).unwrap();
    let entry = service
        .create(
            "不要删除 42 个 API request",
            SessionOptions {
                recognition_mode: RecognitionMode::SimplifiedEnglish,
                original_target: Some("control-from-session-start".into()),
                ..SessionOptions::default()
            },
        )
        .unwrap();
    drop(service);
    let reopened = SessionService::open(&path).unwrap();
    let saved = reopened.get(&entry.id).unwrap();
    assert_eq!(saved.transcript, "不要删除 42 个 API request");
    assert_eq!(saved.created_at, entry.created_at);
    assert_eq!(
        saved.recognition_mode,
        Some(RecognitionMode::SimplifiedEnglish)
    );
    assert_eq!(
        saved.original_target.as_deref(),
        Some("control-from-session-start")
    );
    assert_eq!(saved.status, SessionStatus::Recognized);
}

#[test]
fn database_ownership_is_exclusive_and_lasts_until_the_last_service_clone_drops() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("history.db");
    let first = SessionService::open(&path).unwrap();
    let held = first.clone();
    assert!(matches!(
        SessionService::open(&path),
        Err(CoreError::AlreadyOwned)
    ));
    drop(first);
    assert!(matches!(
        SessionService::open(&path),
        Err(CoreError::AlreadyOwned)
    ));
    drop(held);
    assert!(SessionService::open(&path).is_ok());
}

#[tokio::test]
async fn fresh_history_reopens_interrupted_work_without_losing_the_saved_original() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("fresh.db");
    let service = SessionService::open(&path).unwrap();
    let entry = service
        .create("原稿 API 42", SessionOptions::default())
        .unwrap();
    let started = std::sync::Arc::new(tokio::sync::Notify::new());
    let processor = std::sync::Arc::new(WaitForCancellation(started.clone()));
    let worker = service.clone();
    let id = entry.id.clone();
    let task = tokio::spawn(async move { worker.process(&id, vec![], processor).await });
    started.notified().await;
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    drop(service);
    let reopened = SessionService::open(&path).unwrap();
    let recovered = reopened.get(&entry.id).unwrap();
    assert_eq!(recovered.transcript, "原稿 API 42");
    assert_eq!(recovered.created_at, entry.created_at);
    assert_eq!(recovered.status, SessionStatus::Failed);
    assert_eq!(recovered.error.as_deref(), Some("interrupted"));
    assert_eq!(
        recovered.recognition_mode,
        Some(RecognitionMode::TraditionalEnglish)
    );
}

#[test]
fn handy_import_is_read_only_text_only_and_idempotent_even_after_destination_deletion() {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("handy.db");
    let audio = directory.path().join("handy-1.wav");
    std::fs::write(&audio, b"source-recording-is-not-ours").unwrap();
    let fixture = rusqlite::Connection::open(&source).unwrap();
    fixture.execute_batch("CREATE TABLE transcription_history (
        id INTEGER PRIMARY KEY AUTOINCREMENT,file_name TEXT NOT NULL,timestamp INTEGER NOT NULL,
        saved BOOLEAN NOT NULL DEFAULT 0,title TEXT NOT NULL,transcription_text TEXT NOT NULL,
        post_processed_text TEXT,post_process_prompt TEXT,post_process_requested BOOLEAN NOT NULL DEFAULT 0
    ); INSERT INTO transcription_history VALUES (1,'handy-1.wav',1700000000,0,'title','原稿 API 42','整理結果',NULL,1);
    INSERT INTO transcription_history VALUES (2,'missing.wav',1700000001,1,'title','第二份原稿',NULL,NULL,0);
    INSERT INTO transcription_history VALUES (3,'empty.wav',1700000002,0,'title','  ',NULL,NULL,0);
    PRAGMA user_version=4;").unwrap();
    drop(fixture);
    let before = std::fs::read(&source).unwrap();
    assert!(matches!(
        SessionService::open(&source),
        Err(CoreError::LegacyImportRequired)
    ));
    let service = SessionService::open(directory.path().join("notype.db")).unwrap();
    let report = service.import_handy_history(&source).unwrap();
    assert_eq!((report.imported, report.skipped), (2, 1));
    let imported = service.search("API").unwrap().remove(0);
    assert_eq!(imported.transcript, "原稿 API 42");
    assert_eq!(imported.result.as_deref(), Some("整理結果"));
    assert_eq!(imported.created_at, "2023-11-14T22:13:20.000Z");
    assert_eq!(imported.status, SessionStatus::Pending);
    assert_eq!(imported.recognition_mode, None);
    assert_eq!(imported.original_target, None);
    assert_eq!(
        service.search("第二份").unwrap()[0].status,
        SessionStatus::Recognized
    );
    service.delete(&imported.id).unwrap();
    assert_eq!(service.import_handy_history(&source).unwrap().imported, 0);
    assert!(service.search("API").unwrap().is_empty());
    assert_eq!(std::fs::read(&source).unwrap(), before);
    assert_eq!(
        std::fs::read(&audio).unwrap(),
        b"source-recording-is-not-ours"
    );
}

#[test]
fn future_schema_is_rejected_without_rewriting_or_resetting_the_existing_database() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("future.db");
    let fixture = rusqlite::Connection::open(&path).unwrap();
    fixture.execute_batch("CREATE TABLE sessions (future_data TEXT); INSERT INTO sessions VALUES ('keep me'); PRAGMA user_version=99;").unwrap();
    drop(fixture);
    let before = std::fs::read(&path).unwrap();
    assert!(matches!(
        SessionService::open(&path),
        Err(CoreError::UnsupportedSchema)
    ));
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

struct WaitForCancellation(std::sync::Arc<tokio::sync::Notify>);
impl notype_core::TextProcessor for WaitForCancellation {
    fn polish(
        &self,
        _: notype_core::TextRequest,
        cancellation: notype_core::CancellationToken,
    ) -> notype_core::TextFuture {
        let started = self.0.clone();
        Box::pin(async move {
            started.notify_one();
            cancellation.cancelled().await;
            Err(notype_core::TextFailure::Cancelled)
        })
    }
}

#[tokio::test]
async fn second_instance_cannot_recover_or_interrupt_an_owners_active_session() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("active.db");
    let service = SessionService::open(&path).unwrap();
    let entry = service
        .create(
            "active original",
            SessionOptions {
                recognition_mode: RecognitionMode::TraditionalEnglish,
                original_target: None,
                ..SessionOptions::default()
            },
        )
        .unwrap();
    let started = std::sync::Arc::new(tokio::sync::Notify::new());
    let processor = std::sync::Arc::new(WaitForCancellation(started.clone()));
    let worker = service.clone();
    let id = entry.id.clone();
    let task = tokio::spawn(async move { worker.process(&id, vec![], processor).await });
    started.notified().await;
    assert!(matches!(
        SessionService::open(&path),
        Err(CoreError::AlreadyOwned)
    ));
    assert_eq!(
        service.get(&entry.id).unwrap().status,
        SessionStatus::Processing
    );
    service.cancel(&entry.id).unwrap();
    assert_eq!(
        task.await.unwrap().unwrap().status,
        SessionStatus::Cancelled
    );
}
