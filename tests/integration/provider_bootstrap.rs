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
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/provider_models.rs"]
mod provider_models;
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
        ["Q6.gguf", "Other.gguf"]
            .into_iter()
            .map(|filename| model_roots::ArtifactDescriptor {
                model_id: format!("synthetic/stt/{filename}"),
                filename: filename.into(),
                size: Some(3),
                sha256: Some(
                    "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad".into(),
                ),
                repo: None,
                revision: None,
                directory: false,
            })
            .collect(),
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

fn connection() -> provider_config::ConnectionPreset {
    serde_json::from_value(serde_json::json!({
        "display_name":"Synthetic connection","kind":"custom","dialect":"generic",
        "address_mode":"base_url","api_address":"https://synthetic.reserved.invalid/v1",
        "models_endpoint":null,"model":"synthetic-model","authentication":"bearer",
        "timeout_ms":30000,"thinking_preference":"off","effort":null,
        "capability_declaration":{"kind":"non_reasoning","minimum_effort":null,
          "supported_efforts":[],"temperature_supported":true,"top_p_supported":true},
        "temperature":null,"top_p":null,"max_output_tokens":null
    }))
    .unwrap()
}

#[test]
fn actual_provider_commit_failure_and_restart_preserve_freshness_then_existing_key_bytes_win() {
    let directory = tempfile::tempdir().unwrap();
    let config = directory.path().join("config");
    std::fs::create_dir(&config).unwrap();
    let path = config.join("provider_connections_v2.json");
    let store = ConnectionStore::open(Arc::new(FileStorage::new(path.clone()))).unwrap();
    std::fs::rename(&config, directory.path().join("held")).unwrap();
    assert!(store
        .bootstrap_if_empty(connection(), "synthetic-v1")
        .is_err());
    assert_eq!(store.view().unwrap().revision, 0);
    std::fs::rename(directory.path().join("held"), &config).unwrap();
    let reopened = ConnectionStore::open(Arc::new(FileStorage::new(path.clone()))).unwrap();
    assert_eq!(
        reopened
            .bootstrap_if_empty(connection(), "synthetic-v1")
            .unwrap(),
        provider_config::BootstrapImport::Applied
    );
    let draft = reopened.view().unwrap().connections.remove(0).connection;
    reopened
        .save(
            1,
            draft,
            provider_config::KeyAction::Replace {
                value: "SYNTHETIC-PRIVATE-KEY".into(),
            },
        )
        .unwrap();
    let before = std::fs::read(&path).unwrap();
    let mut ignored = connection();
    ignored.api_address = "invalid unused preset".into();
    assert_eq!(
        reopened
            .bootstrap_if_empty(ignored, "synthetic-v2")
            .unwrap(),
        provider_config::BootstrapImport::SkippedExisting
    );
    assert_eq!(std::fs::read(&path).unwrap(), before);
    let safe = serde_json::to_string(&reopened.view().unwrap()).unwrap();
    assert!(!safe.contains("SYNTHETIC-PRIVATE-KEY"));
}

#[test]
fn concurrent_imports_publish_one_identity_and_one_revision() {
    let directory = tempfile::tempdir().unwrap();
    let store = Arc::new(
        ConnectionStore::open(Arc::new(FileStorage::new(
            directory.path().join("provider_connections_v2.json"),
        )))
        .unwrap(),
    );
    let handles = (0..8)
        .map(|_| {
            let store = store.clone();
            std::thread::spawn(move || {
                store
                    .bootstrap_if_empty(connection(), "synthetic-v1")
                    .unwrap()
            })
        })
        .collect::<Vec<_>>();
    let outcomes = handles
        .into_iter()
        .map(|h| h.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        outcomes
            .iter()
            .filter(|o| **o == provider_config::BootstrapImport::Applied)
            .count(),
        1
    );
    assert_eq!(store.view().unwrap().revision, 1);
    assert_eq!(store.view().unwrap().connections.len(), 1);
}

#[test]
fn partial_cross_domain_commit_reopens_and_fills_only_missing_selection() {
    let directory = tempfile::tempdir().unwrap();
    let models = directory.path().join("models");
    let settings_dir = directory.path().join("settings");
    std::fs::create_dir(&models).unwrap();
    std::fs::create_dir(&settings_dir).unwrap();
    std::fs::write(models.join("Q6.gguf"), b"abc").unwrap();
    let roots = make_roots(directory.path());
    let provider_path = directory.path().join("provider_connections_v2.json");
    let provider =
        ConnectionStore::open(Arc::new(FileStorage::new(provider_path.clone()))).unwrap();
    let selection = SelectionStore::new(Arc::new(FileStorage::new(
        settings_dir.join("settings_store.json"),
    )));
    let mut preset = input();
    preset.connection = Some(connection());
    std::fs::rename(&settings_dir, directory.path().join("held-settings")).unwrap();
    assert!(offline_bootstrap::initialize(
        &preset,
        &models,
        &roots,
        &provider,
        &selection,
        serde_json::json!({})
    )
    .is_err());
    assert!(roots.initialized());
    assert_eq!(provider.view().unwrap().revision, 1);
    assert!(selection.pending().unwrap());
    let provider_before = std::fs::read(&provider_path).unwrap();
    let root_before = std::fs::read(directory.path().join("model_storage_v1.json")).unwrap();
    std::fs::rename(directory.path().join("held-settings"), &settings_dir).unwrap();
    let reopened_roots = make_roots(directory.path());
    let reopened_provider =
        ConnectionStore::open(Arc::new(FileStorage::new(provider_path.clone()))).unwrap();
    offline_bootstrap::initialize(
        &preset,
        &models,
        &reopened_roots,
        &reopened_provider,
        &selection,
        serde_json::json!({}),
    )
    .unwrap();
    assert_eq!(std::fs::read(provider_path).unwrap(), provider_before);
    assert_eq!(
        std::fs::read(directory.path().join("model_storage_v1.json")).unwrap(),
        root_before
    );
    assert_eq!(
        selection.read().unwrap().unwrap()["selected_model"],
        "synthetic/stt/Q6.gguf"
    );
}

#[test]
fn durable_domains_do_not_admit_capture_or_api_before_native_runtime_is_ready() {
    offline_bootstrap::begin();
    assert!(offline_bootstrap::admit().is_err());
    offline_bootstrap::finish(Ok(()));
    assert!(offline_bootstrap::domains_ready());
    assert!(offline_bootstrap::admit().is_err());
    assert!(offline_bootstrap::recovery_command("get_history_entries"));
    assert!(offline_bootstrap::recovery_command("copy_history_text"));
    assert!(offline_bootstrap::recovery_command(
        "get_text_configuration"
    ));
    for command in [
        "list_text_models",
        "initialize_shortcuts",
        "debug_begin_session",
        "retry_history_entry_processing",
        "save_text_connection",
    ] {
        assert!(!offline_bootstrap::recovery_command(command));
    }
    offline_bootstrap::runtime_ready();
    assert!(offline_bootstrap::admit().is_ok());
}

#[test]
fn explicit_user_root_with_another_model_ignores_missing_bundle_selection_without_autochoosing() {
    let directory = tempfile::tempdir().unwrap();
    let user_models = directory.path().join("user-models");
    std::fs::create_dir(&user_models).unwrap();
    std::fs::write(user_models.join("Other.gguf"), b"abc").unwrap();
    let roots = make_roots(directory.path());
    let candidate = roots
        .validate(&user_models, "user-choice", &|_, _| {})
        .unwrap();
    roots
        .apply(&candidate.id, candidate.revision, || {})
        .unwrap();
    let provider = ConnectionStore::open(Arc::new(FileStorage::new(
        directory.path().join("provider_connections_v2.json"),
    )))
    .unwrap();
    let selection = SelectionStore::new(Arc::new(FileStorage::new(
        directory.path().join("settings_store.json"),
    )));
    offline_bootstrap::initialize(
        &input(),
        &directory.path().join("missing-bundle-models"),
        &roots,
        &provider,
        &selection,
        serde_json::json!({"onboarding_completed":false}),
    )
    .unwrap();
    let settings = selection.read().unwrap().unwrap();
    assert_eq!(settings["selected_model"], "");
    assert_eq!(settings["onboarding_completed"], false);
    assert_eq!(roots.projection().count(), 1);
}

#[tokio::test]
async fn bootstrap_and_safe_view_send_zero_http_until_an_explicit_model_list() {
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
        time::{timeout, Duration},
    };
    struct NeverHome;
    impl provider_config::credentials::CredentialReader for NeverHome {
        fn read(
            &self,
            _: provider_config::credentials::PublicKey,
        ) -> Result<Option<String>, provider_config::ConfigError> {
            panic!("bootstrap or anonymous explicit operation must not read home credentials")
        }
    }
    let directory = tempfile::tempdir().unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut preset = connection();
    preset.api_address = format!("http://{}/v1", listener.local_addr().unwrap());
    preset.authentication = provider_config::Authentication::None;
    let store = Arc::new(
        ConnectionStore::open(Arc::new(FileStorage::new(
            directory.path().join("provider_connections_v2.json"),
        )))
        .unwrap(),
    );
    store.bootstrap_if_empty(preset, "synthetic-v1").unwrap();
    let view = store.view().unwrap();
    assert!(!view.connections[0].app_key_present);
    assert!(timeout(Duration::from_millis(30), listener.accept())
        .await
        .is_err());
    let draft = store.view().unwrap().connections.remove(0).connection;
    let discovery = provider_models::ModelDiscovery::new();
    let request = provider_models::ModelListRequest {
        request_id: "explicit".into(),
        expected_revision: 1,
        connection: draft,
        key_action: provider_config::KeyAction::Keep,
    };
    let server = async {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = vec![0; 8192];
        let n = socket.read(&mut request).await.unwrap();
        let text = String::from_utf8_lossy(&request[..n]);
        assert!(text.starts_with("GET /v1/models "));
        assert!(!text.to_ascii_lowercase().contains("authorization:"));
        let body = r#"{"data":[{"id":"synthetic-explicit"}]}"#;
        socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body).as_bytes()).await.unwrap();
    };
    let (_, result) = tokio::join!(server, discovery.query(store, request, &NeverHome));
    assert_eq!(result.unwrap().models[0].id, "synthetic-explicit");
}
