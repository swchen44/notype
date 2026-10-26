#[path = "../../desktop/src-tauri/src/managers/model_root_activity.rs"]
mod model_root_activity;
#[path = "../../desktop/src-tauri/src/managers/model_roots.rs"]
mod model_roots;
#[path = "../../desktop/src-tauri/src/managers/model_roots_files.rs"]
mod model_roots_files;
use model_roots::{ArtifactDescriptor, ArtifactStatus, RootService};
use model_roots_files::{FileStorage, Filesystem};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
static NEXT: AtomicU64 = AtomicU64::new(0);

struct Fixture {
    path: PathBuf,
    parent: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let parent = std::env::var_os("NOTYPE_TEST_FIXTURE_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        let parent = std::fs::canonicalize(parent).unwrap();
        let path = PathBuf::from(model_roots_files::display_path(&parent)).join(format!(
            "notype-p28-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir(&path).unwrap();
        Self { path, parent }
    }
    fn service(&self, config: &str) -> RootService {
        RootService::open(
            self.path.join("default"),
            Arc::new(FileStorage::new(self.path.join(config))),
            Arc::new(Filesystem::new(|_| false)),
            vec![ArtifactDescriptor {
                model_id: "fixture/speech/Q6.gguf".into(),
                filename: "Q6.gguf".into(),
                size: Some(3),
                sha256: Some(
                    "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad".into(),
                ),
                repo: Some("fixture/speech".into()),
                revision: Some("pin".into()),
                directory: false,
            }],
            None,
        )
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let actual = std::fs::canonicalize(&self.path).unwrap();
        assert_eq!(actual.parent(), Some(self.parent.as_path()));
        assert!(actual
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("notype-p28-"));
        std::fs::remove_dir_all(&self.path).unwrap();
    }
}

#[test]
fn manually_moved_whole_folder_is_reused_with_fresh_config_and_persists() {
    let fixture = Fixture::new();
    let folder = fixture.path.join("models old");
    std::fs::create_dir(&folder).unwrap();
    std::fs::write(folder.join("Q6.gguf"), b"abc").unwrap();
    let service = fixture.service("first.json");
    let candidate = service.validate(&folder, "choose1", &|_, _| {}).unwrap();
    service
        .apply(&candidate.id, candidate.revision, || {})
        .unwrap();
    assert_eq!(
        service.artifacts()["fixture/speech/Q6.gguf"].status,
        ArtifactStatus::Verified
    );
    let moved = fixture.path.join("模型 new");
    std::fs::rename(&folder, &moved).unwrap();
    drop(service);
    let fresh = fixture.service("reinstalled.json");
    let candidate = fresh.validate(&moved, "choose2", &|_, _| {}).unwrap();
    fresh
        .apply(&candidate.id, candidate.revision, || {})
        .unwrap();
    drop(fresh);
    let restarted = fixture.service("reinstalled.json");
    let activity = restarted.admit().unwrap();
    let artifact = restarted
        .acquire("fixture/speech/Q6.gguf", &activity)
        .unwrap();
    assert_eq!(std::fs::read(&artifact.path).unwrap(), b"abc");
    assert_eq!(
        restarted.snapshot().root,
        std::fs::canonicalize(&moved).unwrap()
    );
}

#[test]
fn a_partial_marker_added_after_scan_prevents_activation_without_cleanup() {
    let fixture = Fixture::new();
    let folder = fixture.path.join("selected");
    std::fs::create_dir(&folder).unwrap();
    std::fs::write(folder.join("Q6.gguf"), b"abc").unwrap();
    let service = fixture.service("config.json");
    let candidate = service.validate(&folder, "choose", &|_, _| {}).unwrap();
    service
        .apply(&candidate.id, candidate.revision, || {})
        .unwrap();
    std::fs::write(folder.join("Q6.gguf.partial"), b"retain partial").unwrap();
    let activity = service.admit().unwrap();
    assert!(service
        .acquire("fixture/speech/Q6.gguf", &activity)
        .is_err());
    assert_eq!(
        std::fs::read(folder.join("Q6.gguf.partial")).unwrap(),
        b"retain partial"
    );
}

#[test]
fn replacing_the_root_even_with_the_same_hardlinked_artifact_is_rejected() {
    let fixture = Fixture::new();
    let folder = fixture.path.join("selected");
    std::fs::create_dir(&folder).unwrap();
    std::fs::write(folder.join("Q6.gguf"), b"abc").unwrap();
    let service = fixture.service("config.json");
    let candidate = service.validate(&folder, "choose", &|_, _| {}).unwrap();
    service
        .apply(&candidate.id, candidate.revision, || {})
        .unwrap();
    let old = fixture.path.join("old root");
    std::fs::rename(&folder, &old).unwrap();
    std::fs::create_dir(&folder).unwrap();
    std::fs::hard_link(old.join("Q6.gguf"), folder.join("Q6.gguf")).unwrap();
    assert!(service
        .acquire("fixture/speech/Q6.gguf", &service.admit().unwrap())
        .is_err());
}

#[test]
fn invalid_persisted_root_stays_unavailable_without_a_relative_or_default_fallback() {
    let fixture = Fixture::new();
    std::fs::write(
        fixture.path.join("config.json"),
        br#"{"schema":1,"revision":4,"root":"."}"#,
    )
    .unwrap();
    let service = fixture.service("config.json");
    assert_eq!(
        service.projection().status,
        model_roots::RootStatus::Unavailable
    );
    assert_eq!(service.snapshot().revision, 4);
    assert!(!fixture.path.join("default").exists());
}

#[test]
fn choosing_a_folder_that_contains_the_preferences_cannot_write_inside_it() {
    let fixture = Fixture::new();
    let service = fixture.service("config.json");
    assert!(service
        .validate(&fixture.path, "choose", &|_, _| {})
        .is_err());
    assert!(!fixture.path.join("config.json").exists());
}

#[test]
fn rescan_cannot_turn_corrupt_root_preferences_into_a_silent_default_fallback() {
    let fixture = Fixture::new();
    std::fs::create_dir(fixture.path.join("default")).unwrap();
    std::fs::write(fixture.path.join("default/Q6.gguf"), b"abc").unwrap();
    std::fs::write(
        fixture.path.join("config.json"),
        b"corrupt preference sentinel",
    )
    .unwrap();
    let service = fixture.service("config.json");
    service.rescan("rescan", &|_, _| {}).unwrap();
    assert_eq!(
        service.projection().status,
        model_roots::RootStatus::Unavailable
    );
    assert_eq!(
        std::fs::read(fixture.path.join("config.json")).unwrap(),
        b"corrupt preference sentinel"
    );
}

#[test]
fn an_older_queued_scan_cannot_replace_the_newest_candidate_when_workers_start_out_of_order() {
    let fixture = Fixture::new();
    let first = fixture.path.join("first");
    let second = fixture.path.join("second");
    std::fs::create_dir(&first).unwrap();
    std::fs::create_dir(&second).unwrap();
    std::fs::write(second.join("Q6.gguf"), b"abc").unwrap();
    let service = fixture.service("config.json");
    let old = service.prepare_validation(&first, "first").unwrap();
    let newest = service.prepare_validation(&second, "second").unwrap();
    let candidate = service.run_validation(newest, &|_, _| {}).unwrap();
    assert_eq!(
        service
            .apply(&candidate.id, candidate.revision, || {})
            .unwrap_err()
            .0,
        "model-root-busy"
    );
    assert_eq!(
        service.run_validation(old, &|_, _| {}).err().unwrap().0,
        "model-root-cancelled"
    );
    service
        .apply(&candidate.id, candidate.revision, || {})
        .unwrap();
    assert_eq!(
        service.snapshot().root,
        std::fs::canonicalize(second).unwrap()
    );
}

#[test]
fn current_root_terminal_refresh_does_not_discard_a_validated_folder_waiting_for_busy_retry() {
    let fixture = Fixture::new();
    let folder = fixture.path.join("next");
    std::fs::create_dir(&folder).unwrap();
    let service = fixture.service("config.json");
    let candidate = service.validate(&folder, "choose", &|_, _| {}).unwrap();
    let caller = service.admit_mutation("download").unwrap();
    let worker = caller.clone();
    drop(caller);
    assert_eq!(
        service
            .apply(&candidate.id, candidate.revision, || {})
            .unwrap_err()
            .0,
        "model-root-busy"
    );
    service.rescan("terminal_refresh", &|_, _| {}).unwrap();
    drop(worker);
    service
        .apply(&candidate.id, candidate.revision, || {})
        .unwrap();
    assert_eq!(
        service.snapshot().root,
        std::fs::canonicalize(folder).unwrap()
    );
}
