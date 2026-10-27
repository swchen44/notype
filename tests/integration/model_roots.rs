#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/managers/gguf_meta.rs"]
mod gguf_meta;
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/managers/model_capabilities.rs"]
mod model_capabilities;
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/managers/model_root_activity.rs"]
mod model_root_activity;
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/managers/model_roots.rs"]
mod model_roots;
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/managers/model_roots_files.rs"]
mod model_roots_files;
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/managers/model_runtime_metadata.rs"]
mod model_runtime_metadata;
use model_capabilities::{CapabilityProber, Compatibility, GgufHeaderProber};
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

// Exercise the catalog shipped in the App, including bundle/*.gguf entries.
// A synthetic flat-only catalog missed the production folder-selection failure.
fn shipped_catalog() -> Vec<ArtifactDescriptor> {
    let json: serde_json::Value = serde_json::from_str(include_str!(
        "../../desktop/src-tauri/src/catalog/catalog.json"
    ))
    .unwrap();
    json["models"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|model| {
            model["files"].as_array().unwrap().iter().map(|file| {
                let repo = model["id"].as_str().unwrap();
                let filename = file["filename"].as_str().unwrap();
                ArtifactDescriptor {
                    model_id: format!("{repo}/{filename}"),
                    filename: filename.into(),
                    size: file["size_bytes"].as_u64(),
                    sha256: file["sha256"].as_str().map(str::to_owned),
                    repo: Some(repo.into()),
                    revision: model["revision"].as_str().map(str::to_owned),
                    directory: false,
                }
            })
        })
        .collect()
}

#[test]
fn shipped_catalog_does_not_poison_reuse_of_a_valid_flat_model_folder() {
    let fixture = Fixture::new();
    let folder = fixture.path.join("existing models");
    std::fs::create_dir(&folder).unwrap();
    std::fs::write(folder.join("Q6.gguf"), b"abc").unwrap();
    let mut catalog = shipped_catalog();
    assert_eq!(catalog.len(), 367);
    let unsupported: Vec<_> = catalog
        .iter()
        .filter(|d| d.filename.contains('/'))
        .map(|d| d.model_id.clone())
        .collect();
    assert_eq!(unsupported.len(), 6);
    catalog.push(
        fixture.service("unused.json").artifacts()["fixture/speech/Q6.gguf"]
            .descriptor
            .clone()
            .unwrap(),
    );
    let open = || {
        RootService::open(
            fixture.path.join("default"),
            Arc::new(FileStorage::new(fixture.path.join("config.json"))),
            Arc::new(Filesystem::new(|_| false)),
            catalog.clone(),
            None,
        )
    };
    let service = open();
    let candidate = service.validate(&folder, "choose", &|_, _| {}).unwrap();
    assert_eq!(candidate.projection.status, model_roots::RootStatus::Ready);
    assert_eq!(candidate.projection.artifacts.len(), 368);
    service
        .apply(&candidate.id, candidate.revision, || {})
        .unwrap();
    drop(service);
    let service = open();
    service.rescan("rescan", &|_, _| {}).unwrap();
    let activity = service.admit().unwrap();
    let artifacts = service.artifacts();
    assert_eq!(artifacts.len(), 368);
    assert_eq!(
        artifacts["fixture/speech/Q6.gguf"].status,
        ArtifactStatus::Verified
    );
    assert!(artifacts["fixture/speech/Q6.gguf"].supports_download());
    assert_eq!(
        std::fs::read(
            service
                .acquire("fixture/speech/Q6.gguf", &activity)
                .unwrap()
                .path
                .clone()
        )
        .unwrap(),
        b"abc"
    );
    for id in unsupported {
        let record = &artifacts[&id];
        assert_eq!(record.status, ArtifactStatus::Unavailable);
        assert!(!record.present && !record.owned && !record.custom);
        assert!(record.descriptor.is_none() && record.hf_binding.is_none());
        assert!(!record.supports_download());
        assert!(service.acquire(&id, &activity).is_err());
        assert!(service.delete_owned(record, &activity).is_err());
    }
    assert_eq!(std::fs::read_dir(&folder).unwrap().count(), 1);
    assert_eq!(std::fs::read(folder.join("Q6.gguf")).unwrap(), b"abc");
}

#[test]
fn shipped_catalog_leaves_an_empty_safe_folder_selectable_with_no_models() {
    let fixture = Fixture::new();
    let folder = fixture.path.join("empty");
    std::fs::create_dir(&folder).unwrap();
    let service = RootService::open(
        fixture.path.join("default"),
        Arc::new(FileStorage::new(fixture.path.join("config.json"))),
        Arc::new(Filesystem::new(|_| false)),
        shipped_catalog(),
        None,
    );
    let candidate = service.validate(&folder, "empty", &|_, _| {}).unwrap();
    assert_eq!(
        candidate.projection.status,
        model_roots::RootStatus::NoModels
    );
    assert_eq!(candidate.projection.artifacts.len(), 367);
    assert_eq!(candidate.projection.count(), 0);
    assert_eq!(std::fs::read_dir(folder).unwrap().count(), 0);
}

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
            Arc::new(Filesystem::new(|path| {
                matches!(
                    GgufHeaderProber.probe_file(path).verdict,
                    Compatibility::Compatible
                )
            })),
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

fn gguf_header(architecture: &str) -> Vec<u8> {
    let mut bytes = b"GGUF".to_vec();
    bytes.extend_from_slice(&3_u32.to_le_bytes());
    bytes.extend_from_slice(&0_u64.to_le_bytes());
    bytes.extend_from_slice(&1_u64.to_le_bytes());
    let key = b"general.architecture";
    bytes.extend_from_slice(&(key.len() as u64).to_le_bytes());
    bytes.extend_from_slice(key);
    bytes.extend_from_slice(&8_u32.to_le_bytes());
    bytes.extend_from_slice(&(architecture.len() as u64).to_le_bytes());
    bytes.extend_from_slice(architecture.as_bytes());
    bytes
}

#[test]
fn actual_bounded_gguf_probe_admits_supported_custom_speech_as_unverified_and_ignores_llm() {
    let fixture = Fixture::new();
    let folder = fixture.path.join("custom");
    std::fs::create_dir(&folder).unwrap();
    std::fs::write(folder.join("my-voice.gguf"), gguf_header("whisper")).unwrap();
    std::fs::write(folder.join("llm.gguf"), gguf_header("llama")).unwrap();
    let service = fixture.service("config.json");
    let candidate = service.validate(&folder, "custom", &|_, _| {}).unwrap();
    assert_eq!(
        candidate.projection.artifacts["my-voice"].status,
        ArtifactStatus::CustomUnverified
    );
    assert!(!candidate.projection.artifacts.contains_key("llm"));
}

#[test]
fn a_folder_changed_after_validation_cannot_publish_an_outdated_empty_projection() {
    let fixture = Fixture::new();
    let folder = fixture.path.join("changing");
    std::fs::create_dir(&folder).unwrap();
    let service = fixture.service("config.json");
    let before = service.snapshot();
    let candidate = service.validate(&folder, "choose", &|_, _| {}).unwrap();
    std::fs::write(folder.join("Q6.gguf"), b"abc").unwrap();
    assert!(service
        .apply(&candidate.id, candidate.revision, || panic!(
            "not published"
        ))
        .is_err());
    assert_eq!(service.snapshot().generation, before.generation);
    assert!(!fixture.path.join("config.json").exists());
}

#[test]
fn wrong_size_and_hash_are_retained_and_never_replace_the_current_root() {
    let fixture = Fixture::new();
    let service = fixture.service("config.json");
    let old = service.snapshot();
    for (name, bytes) in [("size", b"abcd".as_slice()), ("hash", b"bad".as_slice())] {
        let folder = fixture.path.join(name);
        std::fs::create_dir(&folder).unwrap();
        std::fs::write(folder.join("Q6.gguf"), bytes).unwrap();
        let candidate = service.validate(&folder, name, &|_, _| {}).unwrap();
        assert_eq!(
            candidate.projection.artifacts["fixture/speech/Q6.gguf"].status,
            ArtifactStatus::Invalid
        );
        assert!(service
            .apply(&candidate.id, candidate.revision, || panic!("invalidated"))
            .is_err());
        assert_eq!(service.snapshot().root, old.root);
        assert_eq!(std::fs::read(folder.join("Q6.gguf")).unwrap(), bytes);
    }
}

#[test]
fn failed_durable_commit_keeps_the_old_generation_file_and_usable_artifact() {
    use model_roots::{RootError, RootResult, RootStorage};
    struct FailingStorage(FileStorage);
    impl RootStorage for FailingStorage {
        fn load(&self) -> RootResult<Option<Vec<u8>>> {
            self.0.load()
        }
        fn commit(&self, _: &[u8]) -> RootResult<()> {
            Err(RootError("model-root-persistence-failed"))
        }
    }
    let fixture = Fixture::new();
    let first = fixture.path.join("old");
    let next = fixture.path.join("next");
    std::fs::create_dir(&first).unwrap();
    std::fs::create_dir(&next).unwrap();
    std::fs::write(first.join("Q6.gguf"), b"abc").unwrap();
    let initial = fixture.service("config.json");
    let candidate = initial.validate(&first, "first", &|_, _| {}).unwrap();
    initial.apply(&candidate.id, 0, || {}).unwrap();
    let persisted = std::fs::read(fixture.path.join("config.json")).unwrap();
    let service = RootService::open(
        fixture.path.join("default"),
        Arc::new(FailingStorage(FileStorage::new(
            fixture.path.join("config.json"),
        ))),
        Arc::new(Filesystem::new(|_| false)),
        vec![initial.artifacts()["fixture/speech/Q6.gguf"]
            .descriptor
            .clone()
            .unwrap()],
        None,
    );
    let old = service.snapshot();
    let candidate = service.validate(&next, "next", &|_, _| {}).unwrap();
    assert_eq!(
        service
            .apply(&candidate.id, candidate.revision, || panic!("invalidated"))
            .unwrap_err()
            .0,
        "model-root-persistence-failed"
    );
    assert_eq!(service.snapshot().generation, old.generation);
    assert_eq!(
        std::fs::read(fixture.path.join("config.json")).unwrap(),
        persisted
    );
    assert_eq!(
        std::fs::read(
            &service
                .acquire("fixture/speech/Q6.gguf", &service.admit().unwrap())
                .unwrap()
                .path
        )
        .unwrap(),
        b"abc"
    );
}

#[test]
fn read_only_selected_folder_loads_without_probe_files_and_refuses_mutation() {
    let fixture = Fixture::new();
    let folder = fixture.path.join("readonly");
    std::fs::create_dir(&folder).unwrap();
    std::fs::write(folder.join("Q6.gguf"), b"abc").unwrap();
    let original = std::fs::metadata(&folder).unwrap().permissions();
    let mut readonly = original.clone();
    readonly.set_readonly(true);
    std::fs::set_permissions(&folder, readonly).unwrap();
    let service = fixture.service("config.json");
    let candidate = service.validate(&folder, "readonly", &|_, _| {}).unwrap();
    let writable = candidate.projection.writable;
    service
        .apply(&candidate.id, candidate.revision, || {})
        .unwrap();
    let load = service.acquire("fixture/speech/Q6.gguf", &service.admit().unwrap());
    let mutation = service
        .admit_mutation("fixture/speech/Q6.gguf")
        .err()
        .unwrap()
        .0;
    let entries = std::fs::read_dir(&folder).unwrap().count();
    std::fs::set_permissions(&folder, original).unwrap();
    assert!(!writable);
    assert!(load.is_ok());
    assert_eq!(mutation, "model-root-readonly");
    assert_eq!(entries, 1);
}

#[test]
fn default_legacy_cache_is_read_only_and_never_fills_an_explicit_root() {
    let fixture = Fixture::new();
    let legacy = fixture.path.join("legacy");
    let snapshot = legacy.join("models--fixture--speech/snapshots/pin");
    std::fs::create_dir_all(&snapshot).unwrap();
    std::fs::write(snapshot.join("Q6.gguf"), b"abc").unwrap();
    let descriptor = fixture.service("unused.json").artifacts()["fixture/speech/Q6.gguf"]
        .descriptor
        .clone()
        .unwrap();
    let service = RootService::open(
        fixture.path.join("default"),
        Arc::new(FileStorage::new(fixture.path.join("config.json"))),
        Arc::new(Filesystem::new(|_| false)),
        vec![descriptor],
        Some(legacy),
    );
    let record = service.artifacts()["fixture/speech/Q6.gguf"].clone();
    assert!(record.legacy && !record.owned);
    assert!(service
        .acquire(&record.key.model_id, &service.admit().unwrap())
        .is_ok());
    assert!(service
        .delete_owned(&record, &service.admit_write().unwrap())
        .is_err());
    let selected = fixture.path.join("selected");
    std::fs::create_dir(&selected).unwrap();
    let candidate = service.validate(&selected, "explicit", &|_, _| {}).unwrap();
    service
        .apply(&candidate.id, candidate.revision, || {})
        .unwrap();
    assert!(!service.artifacts()[&record.key.model_id].status.loadable());
    assert!(snapshot.join("Q6.gguf").exists());
    assert_eq!(std::fs::read_dir(selected).unwrap().count(), 0);
}

#[test]
fn directory_engine_without_member_hash_contract_stays_visible_unavailable() {
    let fixture = Fixture::new();
    let folder = fixture.path.join("imported");
    std::fs::create_dir_all(folder.join("engine")).unwrap();
    std::fs::write(folder.join("engine/weights.bin"), b"retain").unwrap();
    let service = RootService::open(
        fixture.path.join("default"),
        Arc::new(FileStorage::new(fixture.path.join("config.json"))),
        Arc::new(Filesystem::new(|_| false)),
        vec![ArtifactDescriptor {
            model_id: "legacy-engine".into(),
            filename: "engine".into(),
            size: None,
            sha256: None,
            repo: None,
            revision: None,
            directory: true,
        }],
        None,
    );
    let candidate = service.validate(&folder, "import", &|_, _| {}).unwrap();
    let record = &candidate.projection.artifacts["legacy-engine"];
    assert!(record.present);
    assert_eq!(record.status, ArtifactStatus::Unavailable);
    assert!(!record.owned);
    assert_eq!(
        std::fs::read(folder.join("engine/weights.bin")).unwrap(),
        b"retain"
    );
}

#[test]
fn cancellation_from_real_scan_progress_never_publishes_the_candidate() {
    let fixture = Fixture::new();
    let folder = fixture.path.join("cancelled");
    std::fs::create_dir(&folder).unwrap();
    std::fs::write(folder.join("Q6.gguf"), b"abc").unwrap();
    let service = fixture.service("config.json");
    let work = service.prepare_validation(&folder, "cancel").unwrap();
    assert_eq!(
        service
            .run_validation(work, &|_, _| service.cancel_scan("cancel"))
            .err()
            .unwrap()
            .0,
        "model-root-cancelled"
    );
    assert!(!service.busy());
    assert!(!fixture.path.join("config.json").exists());
    assert_eq!(std::fs::read(folder.join("Q6.gguf")).unwrap(), b"abc");
}

#[test]
fn root_local_hf_snapshot_requires_the_catalog_hash_and_is_never_claimed_as_owned() {
    let fixture = Fixture::new();
    let folder = fixture.path.join("hf");
    let snapshot = folder.join(".hf-cache/hub/models--fixture--speech/snapshots/pin");
    std::fs::create_dir_all(&snapshot).unwrap();
    std::fs::write(snapshot.join("Q6.gguf"), b"abc").unwrap();
    let service = fixture.service("config.json");
    let candidate = service.validate(&folder, "hf", &|_, _| {}).unwrap();
    let record = &candidate.projection.artifacts["fixture/speech/Q6.gguf"];
    assert_eq!(record.status, ArtifactStatus::Verified);
    assert!(record.hf_binding.is_some() && !record.legacy && !record.owned);
    service
        .apply(&candidate.id, candidate.revision, || {})
        .unwrap();
    assert_eq!(
        std::fs::read(
            &service
                .acquire(&record.key.model_id, &service.admit().unwrap())
                .unwrap()
                .path
        )
        .unwrap(),
        b"abc"
    );
    std::fs::write(snapshot.join("Q6.gguf"), b"bad").unwrap();
    service.rescan("bad_hash", &|_, _| {}).unwrap();
    assert_eq!(
        service.artifacts()[&record.key.model_id].status,
        ArtifactStatus::Invalid
    );
    assert_eq!(std::fs::read(snapshot.join("Q6.gguf")).unwrap(), b"bad");
}

#[test]
fn explicit_hf_write_preparation_preserves_unknown_bytes_and_owns_directory_lifetime() {
    let fixture = Fixture::new();
    let service = fixture.service("config.json");
    let mut owner = service.admit_mutation("fixture/speech/Q6.gguf").unwrap();
    service
        .prepare_hf_write("fixture/speech/Q6.gguf", &mut owner)
        .unwrap();
    let blob = owner.root.root.join(".hf-cache/hub/models--fixture--speech/blobs/ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    std::fs::write(&blob, b"unowned").unwrap();
    let worker = owner.clone();
    drop(owner);
    assert!(service.admit_mutation("fixture/speech/Q6.gguf").is_err());
    drop(worker);
    let mut retry = service.admit_mutation("fixture/speech/Q6.gguf").unwrap();
    assert_eq!(
        service
            .prepare_hf_write("fixture/speech/Q6.gguf", &mut retry)
            .err()
            .unwrap()
            .0,
        "model-artifact-invalid"
    );
    assert_eq!(std::fs::read(blob).unwrap(), b"unowned");
}

#[test]
fn rejecting_a_transfer_path_cannot_delete_or_modify_foreign_bytes() {
    let fixture = Fixture::new();
    let service = fixture.service("config.json");
    let owner = service.admit_mutation("fixture/speech/Q6.gguf").unwrap();
    let outside = fixture.path.join("foreign.partial");
    std::fs::write(&outside, b"foreign sentinel").unwrap();
    assert!(service
        .verify_download(&outside, Some("bad"), &owner)
        .is_err());
    assert_eq!(std::fs::read(&outside).unwrap(), b"foreign sentinel");
    let owned = owner.root.root.join("download.partial");
    use std::io::Write;
    service
        .open_partial(&owned, &owner)
        .unwrap()
        .write_all(b"wrong hash")
        .unwrap();
    assert_eq!(
        service
            .verify_download(&owned, Some("bad"), &owner)
            .err()
            .unwrap()
            .0,
        "model-artifact-invalid"
    );
    assert!(!owned.exists());
}

#[test]
fn a_late_missing_result_cannot_clear_the_same_model_in_a_successor_root() {
    let fixture = Fixture::new();
    let service = fixture.service("config.json");
    let mut previous = None;
    for name in ["first", "successor"] {
        let folder = fixture.path.join(name);
        std::fs::create_dir(&folder).unwrap();
        std::fs::write(folder.join("Q6.gguf"), b"abc").unwrap();
        let candidate = service.validate(&folder, name, &|_, _| {}).unwrap();
        service
            .apply(&candidate.id, candidate.revision, || {})
            .unwrap();
        if let Some(key) = previous.as_ref() {
            service.mark_unavailable(key);
        }
        let record = service.artifacts()["fixture/speech/Q6.gguf"].clone();
        assert_eq!(record.status, ArtifactStatus::Verified);
        previous = Some(record.key);
    }
}

#[test]
fn actual_atomic_preference_rename_failure_preserves_destination_and_removes_only_own_stage() {
    let fixture = Fixture::new();
    std::fs::create_dir(fixture.path.join("config.json")).unwrap();
    std::fs::write(fixture.path.join("config.json/sentinel"), b"retain").unwrap();
    let service = fixture.service("config.json");
    let folder = fixture.path.join("selected");
    std::fs::create_dir(&folder).unwrap();
    let before = service.snapshot();
    let candidate = service.validate(&folder, "choose", &|_, _| {}).unwrap();
    assert_eq!(
        service
            .apply(&candidate.id, candidate.revision, || panic!("published"))
            .unwrap_err()
            .0,
        "model-root-persistence-failed"
    );
    assert_eq!(service.snapshot().generation, before.generation);
    assert_eq!(
        std::fs::read(fixture.path.join("config.json/sentinel")).unwrap(),
        b"retain"
    );
    assert!(!std::fs::read_dir(&fixture.path).unwrap().any(|entry| entry
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with(".notype-model-root-")));
}

#[test]
fn replacing_owned_download_with_identical_external_bytes_does_not_inherit_delete_permission() {
    let fixture = Fixture::new();
    let service = fixture.service("config.json");
    let path = service.snapshot().root.join("Q6.gguf");
    std::fs::write(&path, b"abc").unwrap();
    service.rescan("download", &|_, _| {}).unwrap();
    service.mark_owned(
        "fixture/speech/Q6.gguf",
        &service.artifacts()["fixture/speech/Q6.gguf"]
            .key
            .fingerprint,
    );
    assert!(service.artifacts()["fixture/speech/Q6.gguf"].owned);
    std::fs::rename(&path, path.with_extension("original")).unwrap();
    std::fs::write(&path, b"abc").unwrap();
    service.rescan("external", &|_, _| {}).unwrap();
    let record = service.artifacts()["fixture/speech/Q6.gguf"].clone();
    assert!(!record.owned);
    assert!(service
        .delete_owned(
            &record,
            &service.admit_mutation(&record.key.model_id).unwrap()
        )
        .is_err());
    assert_eq!(std::fs::read(path).unwrap(), b"abc");
}

#[test]
fn checked_partial_handle_verifies_and_publishes_without_overwriting_a_destination() {
    use std::io::Write;
    let fixture = Fixture::new();
    let service = fixture.service("config.json");
    let owner = service.admit_mutation("fixture/speech/Q6.gguf").unwrap();
    let partial = owner.root.root.join("Q6.gguf.partial");
    let destination = owner.root.root.join("Q6.gguf");
    service
        .open_partial(&partial, &owner)
        .unwrap()
        .write_all(b"abc")
        .unwrap();
    let verified = service
        .verify_download(
            &partial,
            Some("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"),
            &owner,
        )
        .unwrap();
    std::fs::write(&destination, b"retain foreign destination").unwrap();
    assert!(service
        .publish_download(verified, &destination, &owner)
        .is_err());
    assert_eq!(
        std::fs::read(&destination).unwrap(),
        b"retain foreign destination"
    );
    std::fs::remove_file(&destination).unwrap();
    let verified = service
        .verify_download(
            &partial,
            Some("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"),
            &owner,
        )
        .unwrap();
    let fingerprint = service
        .publish_download(verified, &destination, &owner)
        .unwrap();
    assert!(!partial.exists());
    service.rescan("complete", &|_, _| {}).unwrap();
    service.mark_owned("fixture/speech/Q6.gguf", &fingerprint);
    let record = service.artifacts()["fixture/speech/Q6.gguf"].clone();
    assert_eq!(record.status, ArtifactStatus::Verified);
    assert!(record.owned);
    assert_eq!(std::fs::read(destination).unwrap(), b"abc");
}

#[test]
fn an_unowned_partial_cannot_be_opened_for_resume_or_removed() {
    let fixture = Fixture::new();
    let service = fixture.service("config.json");
    let owner = service.admit_mutation("fixture/speech/Q6.gguf").unwrap();
    let partial = owner.root.root.join("Q6.gguf.partial");
    std::fs::write(&partial, b"unowned sentinel").unwrap();
    assert!(service.open_partial(&partial, &owner).is_err());
    assert!(service.remove_partial(&partial, &owner).is_err());
    assert_eq!(std::fs::read(partial).unwrap(), b"unowned sentinel");
}

#[test]
fn runtime_capabilities_do_not_follow_the_same_model_id_into_a_different_artifact() {
    use model_runtime_metadata::{Capabilities, RuntimeMetadata};
    let fixture = Fixture::new();
    let service = fixture.service("config.json");
    let metadata = RuntimeMetadata::default();
    let mut predecessor = None;
    for (index, name) in ["first", "next"].into_iter().enumerate() {
        let folder = fixture.path.join(name);
        std::fs::create_dir(&folder).unwrap();
        let mut bytes = gguf_header("whisper");
        bytes.push(index as u8);
        std::fs::write(folder.join("same.gguf"), bytes).unwrap();
        let candidate = service.validate(&folder, name, &|_, _| {}).unwrap();
        service
            .apply(&candidate.id, candidate.revision, || {})
            .unwrap();
        let key = service.artifacts()["same"].key.clone();
        assert!(metadata.get(&key).is_none());
        if let Some(old) = predecessor.as_ref() {
            assert!(!metadata.publish(
                &service,
                old,
                Capabilities {
                    streaming: false,
                    translation: false,
                    language_detection: false,
                    languages: vec!["stale".into()],
                }
            ));
        }
        assert!(metadata.publish(
            &service,
            &key,
            Capabilities {
                streaming: index == 0,
                translation: false,
                language_detection: false,
                languages: vec![name.into()],
            }
        ));
        assert_eq!(metadata.get(&key).unwrap().languages, vec![name]);
        predecessor = Some(key);
    }
}

#[cfg(unix)]
#[test]
fn dangling_partial_link_never_creates_an_external_target_or_applies_as_a_valid_candidate() {
    let fixture = Fixture::new();
    let service = fixture.service("config.json");
    let owner = service.admit_mutation("fixture/speech/Q6.gguf").unwrap();
    let partial = owner.root.root.join("Q6.gguf.partial");
    let outside = fixture.path.join("never-created");
    std::os::unix::fs::symlink(&outside, &partial).unwrap();
    assert!(service.open_partial(&partial, &owner).is_err());
    assert!(!outside.exists());
    drop(owner);
    let folder = fixture.path.join("selected");
    std::fs::create_dir(&folder).unwrap();
    std::os::unix::fs::symlink(&outside, folder.join("Q6.gguf.partial")).unwrap();
    let candidate = service.validate(&folder, "dangling", &|_, _| {}).unwrap();
    assert_eq!(
        candidate.projection.status,
        model_roots::RootStatus::Invalid
    );
    assert!(service
        .apply(&candidate.id, candidate.revision, || panic!("published"))
        .is_err());
    assert!(!outside.exists());
}

#[cfg(unix)]
#[test]
fn hf_single_relative_blob_link_is_accepted_and_an_escape_is_rejected_without_touching_target() {
    use std::os::unix::fs::symlink;
    let fixture = Fixture::new();
    let folder = fixture.path.join("hf");
    let repo = folder.join(".hf-cache/hub/models--fixture--speech");
    let snapshot = repo.join("snapshots/pin");
    std::fs::create_dir_all(&snapshot).unwrap();
    std::fs::create_dir(repo.join("blobs")).unwrap();
    std::fs::write(repo.join("blobs/abc"), b"abc").unwrap();
    let pointer = snapshot.join("Q6.gguf");
    symlink("../../blobs/abc", &pointer).unwrap();
    let service = fixture.service("config.json");
    let candidate = service.validate(&folder, "safe", &|_, _| {}).unwrap();
    assert_eq!(candidate.projection.count(), 1);
    std::fs::remove_file(&pointer).unwrap();
    let outside = fixture.path.join("outside");
    std::fs::write(&outside, b"abc").unwrap();
    symlink(&outside, &pointer).unwrap();
    assert!(service.validate(&folder, "escape", &|_, _| {}).is_err());
    assert_eq!(std::fs::read(outside).unwrap(), b"abc");
}
