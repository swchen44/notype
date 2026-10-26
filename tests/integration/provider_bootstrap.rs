#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/capture_startup.rs"]
mod capture_startup;
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/diagnostics.rs"]
mod diagnostics;
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/managers/model_root_activity.rs"]
pub mod model_root_activity;
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/managers/model_roots.rs"]
pub mod model_roots;
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/managers/model_roots_files.rs"]
pub mod model_roots_files;
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/managers/model_runtime_metadata.rs"]
pub mod model_runtime_metadata;
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/provider_config.rs"]
mod provider_config;
#[allow(dead_code, unused_imports)]
#[path = "../../desktop/src-tauri/src/workflow.rs"]
mod workflow;
mod managers {
    pub use crate::model_roots;
}
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/bootstrap_selection.rs"]
mod bootstrap_selection;
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/offline_bootstrap.rs"]
mod offline_bootstrap;
use bootstrap_selection::SelectionStore;
use provider_config::{files::FileStorage, ConnectionStore};
use std::{path::Path, sync::Arc};

fn make_roots(directory: &Path) -> model_roots::RootService {
    model_roots::RootService::open(
        directory.join("default"),
        Arc::new(model_roots_files::FileStorage::new(
            directory.join("model_storage_v1.json"),
        )),
        Arc::new(model_roots_files::Filesystem::new(|_| true)),
        vec![model_roots::ArtifactDescriptor {
            model_id: "synthetic/stt/Q6.gguf".into(),
            filename: "Q6.gguf".into(),
            size: Some(3),
            sha256: Some("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad".into()),
            repo: None,
            revision: None,
            directory: false,
        }],
        None,
    )
}
fn input() -> offline_bootstrap::BootstrapInput {
    offline_bootstrap::BootstrapInput::parse(
        br#"{"schema":1,"bootstrap_id":"synthetic-v1",
      "stt":{"model_id":"synthetic/stt/Q6.gguf","root":"models"}}"#,
    )
    .unwrap()
}

#[test]
fn missing_required_model_does_not_initialize_any_domain_then_restored_fixture_retries() {
    let directory = tempfile::tempdir().unwrap();
    let models = directory.path().join("models");
    std::fs::create_dir(&models).unwrap();
    let roots = make_roots(directory.path());
    let provider = ConnectionStore::open(Arc::new(FileStorage::new(
        directory.path().join("provider_connections_v2.json"),
    )))
    .unwrap();
    let selection = SelectionStore::new(Arc::new(FileStorage::new(
        directory.path().join("settings_store.json"),
    )));
    assert!(offline_bootstrap::initialize(
        &input(),
        &models,
        &roots,
        &provider,
        &selection,
        serde_json::json!({})
    )
    .is_err());
    assert!(selection.pending().unwrap());
    assert!(!roots.initialized());
    std::fs::write(models.join("Q6.gguf"), b"abc").unwrap();
    offline_bootstrap::initialize(
        &input(),
        &models,
        &roots,
        &provider,
        &selection,
        serde_json::json!({"onboarding_completed":false}),
    )
    .unwrap();
    assert_eq!(
        selection.read().unwrap().unwrap()["selected_model"],
        "synthetic/stt/Q6.gguf"
    );
    let before = std::fs::read(directory.path().join("settings_store.json")).unwrap();
    let reopened = make_roots(directory.path());
    // Unused default disappearance cannot override a persisted user root/selection.
    std::fs::rename(&models, directory.path().join("moved-models")).unwrap();
    offline_bootstrap::initialize(
        &input(),
        &models,
        &reopened,
        &provider,
        &selection,
        serde_json::json!({}),
    )
    .unwrap();
    assert_eq!(
        std::fs::read(directory.path().join("settings_store.json")).unwrap(),
        before
    );
}

#[test]
fn real_settings_parent_failure_leaves_freshness_for_restart_and_retry() {
    let directory = tempfile::tempdir().unwrap();
    let config = directory.path().join("config");
    std::fs::create_dir(&config).unwrap();
    let path = config.join("settings_store.json");
    let selection = SelectionStore::new(Arc::new(FileStorage::new(path.clone())));
    std::fs::rename(&config, directory.path().join("held")).unwrap();
    assert!(selection
        .initialize_if_absent(serde_json::json!({}), "synthetic-stt", "synthetic-v1")
        .is_err());
    assert!(selection.pending().unwrap());
    std::fs::rename(directory.path().join("held"), &config).unwrap();
    let restarted = SelectionStore::new(Arc::new(FileStorage::new(path)));
    assert!(restarted
        .initialize_if_absent(serde_json::json!({}), "synthetic-stt", "synthetic-v1")
        .unwrap());
    assert_eq!(
        restarted.read().unwrap().unwrap()["selected_model"],
        "synthetic-stt"
    );
}

#[test]
fn malformed_same_store_bootstrap_marker_fails_closed_and_keeps_actual_bytes() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("settings_store.json");
    let bytes = br#"{"settings":{"selected_model":"kept"},"notype_bootstrap":{"schema":99,"applied":"synthetic-v1"}}"#;
    std::fs::write(&path, bytes).unwrap();
    let selection = SelectionStore::new(Arc::new(FileStorage::new(path.clone())));
    assert!(selection.pending().is_err());
    assert_eq!(std::fs::read(path).unwrap(), bytes);
}
