#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/bootstrap_selection.rs"]
mod bootstrap_selection;
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/capture_startup.rs"]
mod capture_startup;
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/diagnostic_files.rs"]
mod diagnostic_files;
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/diagnostics.rs"]
mod diagnostics;
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/managers/model_root_activity.rs"]
mod model_root_activity;
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/managers/model_roots.rs"]
pub mod model_roots;
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/managers/model_roots_files.rs"]
mod model_roots_files;
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/managers/model_runtime_metadata.rs"]
mod model_runtime_metadata;
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/provider_config.rs"]
mod provider_config;
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/settings_journal.rs"]
mod settings_journal;
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/settings_journal_files.rs"]
mod settings_journal_files;
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/settings_transfer.rs"]
mod settings_transfer;
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/settings_yaml.rs"]
mod settings_yaml;
#[allow(dead_code, unused_imports)]
#[path = "../../desktop/src-tauri/src/workflow.rs"]
mod workflow;
use provider_config::*;
use settings_transfer::*;
use std::{fs, sync::Arc};

#[test]
fn whole_archive_utf8_roundtrip_and_previews_never_return_app_keys() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("providers.json");
    let store = ConnectionStore::open(Arc::new(files::FileStorage::new(path.clone()))).unwrap();
    let mut connection = store
        .create(
            0,
            "語言伺服器".into(),
            ConnectionKind::Provider,
            ProviderDialect::Deepseek,
        )
        .unwrap()
        .connections
        .remove(0)
        .connection;
    connection.model = "synthetic-model".into();
    store
        .save(
            1,
            connection,
            KeyAction::Replace {
                value: "SYNTHETIC_TRANSFER_SECRET".into(),
            },
        )
        .unwrap();
    let before = fs::read(&path).unwrap();
    let archive = Archive {
        notype_settings: 1,
        settings: serde_json::json!({"custom_words":["notype","繁體中文"]}),
        ai: store.archive(true).unwrap(),
        model_folder: "D:/模型".into(),
        diagnostics_enabled: true,
    };
    let encoded = archive.encode().unwrap();
    assert!(encoded.contains("SYNTHETIC_TRANSFER_SECRET"));
    let decoded = Archive::decode(encoded.as_bytes()).unwrap();
    let preview = decoded.preview().unwrap();
    assert!(!preview.contains("SYNTHETIC_TRANSFER_SECRET"));
    assert!(preview.contains("********"));
    assert!(preview.contains("DEEPSEEK_API_KEY"));
    assert!(
        Archive::decode(preview.as_bytes()).is_err(),
        "a redacted preview must never become a real imported key"
    );
    let excluded = Archive {
        ai: store.archive(false).unwrap(),
        ..decoded
    };
    assert!(!excluded.encode().unwrap().contains("app_keys"));
    assert!(excluded
        .prepared_providers(&store)
        .unwrap()
        .contains("SYNTHETIC_TRANSFER_SECRET"));
    assert_eq!(before, fs::read(path).unwrap());
}
#[test]
fn unknown_versions_nested_fields_and_retired_secret_fields_are_rejected() {
    let directory = tempfile::tempdir().unwrap();
    let store = ConnectionStore::open(Arc::new(files::FileStorage::new(
        directory.path().join("provider.json"),
    )))
    .unwrap();
    store
        .create(
            0,
            "Synthetic".into(),
            ConnectionKind::Custom,
            ProviderDialect::Generic,
        )
        .unwrap();
    let archive = Archive {
        notype_settings: 1,
        settings: serde_json::json!({"custom_words":[]}),
        ai: store.archive(false).unwrap(),
        model_folder: "C:/models".into(),
        diagnostics_enabled: false,
    };
    let value = serde_json::to_value(archive).unwrap();
    for edit in ["version", "nested", "retired"] {
        let mut bad = value.clone();
        match edit {
            "version" => bad["notype_settings"] = 2.into(),
            "nested" => bad["ai"]["connections"][0]["typo"] = true.into(),
            _ => bad["settings"]["post_process_api_keys"] = serde_json::json!({"x":"SYNTHETIC"}),
        }
        let text = settings_yaml::render(&bad).unwrap();
        assert_eq!(
            Archive::decode(text.as_bytes()).err().as_deref(),
            Some("settings-transfer-invalid")
        );
    }
    let projected=project_settings(serde_json::json!({"post_process_api_keys":{"x":"SYNTHETIC"},"post_process_enabled":true,"custom_words":["你好"]})).unwrap();
    assert_eq!(
        projected,
        serde_json::json!({"post_process_enabled":true,"custom_words":["你好"]})
    );
}
#[test]
fn disk_baseline_allows_only_known_app_autosave_and_rejects_external_changes() {
    use serde_json::json;
    use settings_journal::Domain;
    let baseline = [
        (
            Domain::App,
            Some(json!({"settings":{"old":true},"ownership":"keep"})),
        ),
        (Domain::Providers, Some(json!({"revision":1}))),
        (Domain::ModelRoot, None),
        (Domain::Diagnostics, None),
    ]
    .into_iter()
    .collect::<std::collections::BTreeMap<_, _>>();
    let stamp = DiskStamp::new(baseline.clone(), json!({"new":true}));
    assert!(stamp.matches(&baseline));
    let mut autosaved = baseline.clone();
    autosaved.insert(
        Domain::App,
        Some(json!({"settings":{"new":true},"ownership":"keep"})),
    );
    assert!(stamp.matches(&autosaved));
    for domain in [
        Domain::App,
        Domain::Providers,
        Domain::ModelRoot,
        Domain::Diagnostics,
    ] {
        let mut external = baseline.clone();
        external.insert(domain, Some(json!({"external":true})));
        assert!(!stamp.matches(&external));
    }
    autosaved.get_mut(&Domain::App).unwrap().as_mut().unwrap()["ownership"] = json!("changed");
    assert!(!stamp.matches(&autosaved));
}

#[test]
fn confirmed_whole_transfer_reopens_actual_stores_and_preserves_history_and_weights() {
    use diagnostics::service::DiagnosticStorage;
    use settings_journal::{Domain, Storage, Target};
    let directory = tempfile::tempdir().unwrap();
    let data = directory.path().join("data");
    fs::create_dir(&data).unwrap();
    let models = directory.path().join("模型");
    fs::create_dir(&models).unwrap();
    fs::write(models.join("sentinel.gguf"), b"model-not-moved").unwrap();
    let history = data.join("sessions.db");
    drop(notype_core::SessionService::open(&history).unwrap());
    let history_before = fs::read(&history).unwrap();
    let provider_path = data.join(Domain::Providers.filename());
    let store =
        ConnectionStore::open(Arc::new(files::FileStorage::new(provider_path.clone()))).unwrap();
    let mut connection = store
        .create(
            0,
            "Synthetic".into(),
            ConnectionKind::Provider,
            ProviderDialect::Deepseek,
        )
        .unwrap()
        .connections
        .remove(0)
        .connection;
    connection.model = "synthetic".into();
    store
        .save(
            1,
            connection,
            KeyAction::Replace {
                value: "SYNTHETIC_RETAINED_KEY".into(),
            },
        )
        .unwrap();
    let archive = Archive {
        notype_settings: 1,
        settings: serde_json::json!({"custom_words":["新詞"]}),
        ai: store.archive(false).unwrap(),
        model_folder: models.to_string_lossy().into(),
        diagnostics_enabled: false,
    };
    let yaml = archive.encode().unwrap();
    let imported = Archive::decode(yaml.as_bytes()).unwrap();
    let files = settings_journal_files::Files::open(&data).unwrap();
    files
        .write(Domain::App, "{\"settings\":{\"custom_words\":[\"舊詞\"]}}")
        .unwrap();
    files
        .write(
            Domain::Diagnostics,
            "{\"schema_version\":1,\"diagnostics_enabled\":true}",
        )
        .unwrap();
    let root_text = String::from_utf8(model_roots_files::encode(&models, 1).unwrap()).unwrap();
    files.write(Domain::ModelRoot, &root_text).unwrap();
    let targets = [
        Domain::App,
        Domain::Providers,
        Domain::ModelRoot,
        Domain::Diagnostics,
    ]
    .into_iter()
    .map(|domain| Target {
        domain,
        before: files.read(domain).unwrap(),
        after: match domain {
            Domain::App => serde_json::json!({"settings":imported.settings}).to_string(),
            Domain::Providers => imported.prepared_providers(&store).unwrap(),
            Domain::ModelRoot => {
                String::from_utf8(model_roots_files::encode(&models, 2).unwrap()).unwrap()
            }
            Domain::Diagnostics => "{\"schema_version\":1,\"diagnostics_enabled\":false}".into(),
        },
    })
    .collect::<Vec<_>>();
    settings_journal::stage(&files, targets.clone()).unwrap();
    for target in &targets {
        assert_eq!(files.read(target.domain).unwrap(), target.before);
    }
    assert_eq!(
        settings_journal::recover(&files).unwrap(),
        settings_journal::Recovery::Applied
    );
    drop(store);
    let reopened = ConnectionStore::open(Arc::new(files::FileStorage::new(provider_path))).unwrap();
    assert_eq!(
        reopened
            .archive(true)
            .unwrap()
            .app_keys
            .unwrap()
            .values()
            .next()
            .map(String::as_str),
        Some("SYNTHETIC_RETAINED_KEY")
    );
    let selection = bootstrap_selection::SelectionStore::new(Arc::new(files::FileStorage::new(
        data.join(Domain::App.filename()),
    )));
    assert_eq!(
        selection.read().unwrap().unwrap()["custom_words"],
        serde_json::json!(["新詞"])
    );
    let roots = model_roots::RootService::open_unscanned(
        models.clone(),
        Arc::new(model_roots_files::FileStorage::new(
            data.join(Domain::ModelRoot.filename()),
        )),
        Arc::new(model_roots_files::Filesystem::new(|_| false)),
        vec![],
        None,
    );
    assert_eq!(roots.snapshot().root, fs::canonicalize(&models).unwrap());
    assert_eq!(roots.snapshot().revision, 2);
    assert_eq!(
        diagnostic_files::FileStorage::new(Some(data.join(Domain::Diagnostics.filename())))
            .load_preference()
            .unwrap(),
        Some(false)
    );
    assert_eq!(fs::read(history).unwrap(), history_before);
    assert_eq!(
        fs::read(models.join("sentinel.gguf")).unwrap(),
        b"model-not-moved"
    );
}
