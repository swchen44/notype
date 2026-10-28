#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/capture_startup.rs"]
mod capture_startup;
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/diagnostics.rs"]
mod diagnostics;
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/provider_config.rs"]
mod provider_config;
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/settings_yaml.rs"]
mod settings_yaml;
#[allow(dead_code, unused_imports)]
#[path = "../../desktop/src-tauri/src/workflow.rs"]
mod workflow;
use provider_config::*;
use std::sync::Arc;

#[test]
fn actual_provider_archive_yaml_reopens_with_optional_keys_without_changing_source_files() {
    use provider_config::archive::ProviderArchive;
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("provider_connections_v2.json");
    let history = directory.path().join("sessions.db");
    let _sessions = notype_core::SessionService::open(&history).unwrap();
    let store = ConnectionStore::open(Arc::new(files::FileStorage::new(source.clone()))).unwrap();
    let mut draft = store
        .create(
            0,
            "語言研究".into(),
            ConnectionKind::Provider,
            ProviderDialect::Deepseek,
        )
        .unwrap()
        .connections
        .remove(0)
        .connection;
    let id = draft.id.clone();
    draft.model = "fixture-model".into();
    draft.temperature = Some(0.0);
    draft.prompt_tuning.dictation = Some("保留 OpenAI 原文。".into());
    draft.prompt_tuning.translation = Some("保留原文格式。".into());
    store
        .save(
            1,
            draft,
            KeyAction::Replace {
                value: "SYNTHETIC_ARCHIVE_KEY".into(),
            },
        )
        .unwrap();
    store
        .prompt(
            2,
            notype_core::Scenario::Ask,
            Some("SHARED_ASK_RULES".into()),
        )
        .unwrap();
    store.activate(3, Some(id.clone())).unwrap();
    drop(store);
    let reopened =
        ConnectionStore::open(Arc::new(files::FileStorage::new(source.clone()))).unwrap();
    let source_before = std::fs::read(&source).unwrap();
    let history_before = std::fs::read(&history).unwrap();
    for include_keys in [false, true] {
        let snapshot = reopened.archive(include_keys).unwrap();
        let text = settings_yaml::render(&serde_json::to_value(snapshot).unwrap()).unwrap();
        assert!(text.contains("語言研究"));
        assert!(text.contains("DEEPSEEK_API_KEY"));
        assert_eq!(text.contains("SYNTHETIC_ARCHIVE_KEY"), include_keys);
        let exported = directory.path().join(if include_keys {
            "included.yaml"
        } else {
            "excluded.yaml"
        });
        std::fs::write(&exported, text.as_bytes()).unwrap();
        let archive: ProviderArchive = serde_json::from_value(
            settings_yaml::parse(&std::fs::read(&exported).unwrap()).unwrap(),
        )
        .unwrap();
        archive.validate().unwrap();
        assert_eq!(archive.active_connection_id.as_deref(), Some(id.as_str()));
        assert_eq!(archive.prompts.ask, "SHARED_ASK_RULES");
        assert_eq!(archive.connections[0].temperature, Some(0.0));
        assert_eq!(
            archive.connections[0].prompt_tuning.dictation.as_deref(),
            Some("保留 OpenAI 原文。")
        );
        assert_eq!(archive.app_keys.is_some(), include_keys);
        let prepared = reopened.replacement(&archive).unwrap();
        let next_path = directory.path().join(if include_keys {
            "restored-included.json"
        } else {
            "restored-excluded.json"
        });
        ConfigurationStorage::commit(&files::FileStorage::new(next_path.clone()), &prepared)
            .unwrap();
        let next = ConnectionStore::open(Arc::new(files::FileStorage::new(next_path))).unwrap();
        struct NeverHome;
        impl credentials::CredentialReader for NeverHome {
            fn read(&self, _: credentials::PublicKey) -> Result<Option<String>, ConfigError> {
                panic!("restored App key must not fall back to home");
            }
        }
        assert_eq!(
            next.resolve_active(&NeverHome)
                .unwrap()
                .credential_source
                .as_deref(),
            Some("app")
        );
        assert_eq!(next.view().unwrap().revision, 5);
    }
    assert_eq!(std::fs::read(&source).unwrap(), source_before);
    assert_eq!(std::fs::read(&history).unwrap(), history_before);
    assert_eq!(reopened.view().unwrap().revision, 4);
}

#[tokio::test]
async fn saved_scene_supplements_reach_only_the_matching_loopback_system_message() {
    use notype_core::{RecognitionMode, Scenario, SessionService};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    struct NeverHome;
    impl credentials::CredentialReader for NeverHome {
        fn read(&self, _: credentials::PublicKey) -> Result<Option<String>, ConfigError> {
            panic!("local anonymous fixture must not read credentials");
        }
    }
    for (scenario, supplement) in [
        (Scenario::Dictation, "DICTATION_ONLY"),
        (Scenario::Translation, "TRANSLATION_ONLY"),
        (Scenario::Ask, "ASK_ONLY"),
    ] {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = format!("http://{}/v1", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (mut stream, _) =
                tokio::time::timeout(std::time::Duration::from_secs(5), listener.accept())
                    .await
                    .unwrap()
                    .unwrap();
            let mut bytes = Vec::new();
            let mut buffer = [0; 4096];
            let (end, length) = loop {
                let size = stream.read(&mut buffer).await.unwrap();
                assert!(size > 0);
                bytes.extend_from_slice(&buffer[..size]);
                if let Some(end) = bytes.windows(4).position(|p| p == b"\r\n\r\n") {
                    let header = std::str::from_utf8(&bytes[..end]).unwrap().to_lowercase();
                    assert!(header.starts_with("post /v1/chat/completions http/1.1"));
                    assert!(!header.contains("authorization:"));
                    let length = header
                        .lines()
                        .find_map(|line| {
                            line.strip_prefix("content-length:")
                                .map(|s| s.trim().parse::<usize>().unwrap())
                        })
                        .unwrap();
                    break (end + 4, length);
                }
            };
            while bytes.len() < end + length {
                let size = stream.read(&mut buffer).await.unwrap();
                assert!(size > 0);
                bytes.extend_from_slice(&buffer[..size]);
            }
            let request: serde_json::Value =
                serde_json::from_slice(&bytes[end..end + length]).unwrap();
            let body = r#"{"choices":[{"message":{"role":"assistant","content":"Synthetic result."},"finish_reason":"stop"}]}"#;
            stream.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body).as_bytes()).await.unwrap();
            assert!(
                tokio::time::timeout(std::time::Duration::from_millis(75), listener.accept())
                    .await
                    .is_err(),
                "unexpected additional request"
            );
            request
        });
        let directory = tempfile::tempdir().unwrap();
        let store = ConnectionStore::open(Arc::new(files::FileStorage::new(
            directory.path().join("connections.json"),
        )))
        .unwrap();
        let mut draft = store
            .create(
                0,
                "Local fixture".into(),
                ConnectionKind::Local,
                ProviderDialect::Generic,
            )
            .unwrap()
            .connections
            .remove(0)
            .connection;
        draft.api_address = address;
        draft.model = "synthetic-model".into();
        draft.capability_declaration = Some(CapabilityDeclaration {
            kind: "non_reasoning".into(),
            minimum_effort: None,
            supported_efforts: vec![],
            temperature_supported: true,
            top_p_supported: true,
        });
        draft.prompt_tuning = ConnectionPromptTuning {
            dictation: Some("DICTATION_ONLY".into()),
            translation: Some("TRANSLATION_ONLY".into()),
            ask: Some("ASK_ONLY".into()),
        };
        let id = draft.id.clone();
        store.save(1, draft, KeyAction::Keep).unwrap();
        store.activate(2, Some(id)).unwrap();
        let config = store.resolve_active(&NeverHome).unwrap();
        let expected = config.prompt(scenario).to_owned();
        assert!(expected.ends_with(supplement));
        let workflow =
            workflow::DesktopWorkflow::new(SessionService::in_memory().unwrap(), Arc::new(|| {}));
        let generation = workflow
            .begin(RecognitionMode::TraditionalEnglish, None)
            .unwrap();
        workflow
            .capture_processing_snapshot(generation, scenario, config.clone(), vec![])
            .unwrap();
        workflow
            .finish(generation, "Synthetic source.", config, vec![])
            .await
            .unwrap();
        let request = server.await.unwrap();
        assert_eq!(request["messages"][1]["role"], "system");
        assert_eq!(request["messages"][1]["content"], expected);
        for other in ["DICTATION_ONLY", "TRANSLATION_ONLY", "ASK_ONLY"] {
            assert_eq!(expected.contains(other), other == supplement);
        }
    }
}

#[test]
fn actual_file_store_reopens_safe_records_and_rejects_corrupt_collection_identity() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("connections-v2.json");
    let store = ConnectionStore::open(Arc::new(files::FileStorage::new(path.clone()))).unwrap();
    let first = store
        .create(
            0,
            "duplicate".into(),
            ConnectionKind::Provider,
            ProviderDialect::Deepseek,
        )
        .unwrap();
    let id = first.connections[0].connection.id.clone();
    let mut connection = first.connections[0].connection.clone();
    connection.prompt_tuning.translation = Some("PERSISTED_TRANSLATION_SUPPLEMENT".into());
    store.save(1, connection, KeyAction::Keep).unwrap();
    store.activate(2, Some(id.clone())).unwrap();
    drop(store);
    let reopened = ConnectionStore::open(Arc::new(files::FileStorage::new(path.clone()))).unwrap();
    assert_eq!(reopened.view().unwrap().active_connection_id, Some(id));
    let tuning = &reopened.view().unwrap().connections[0]
        .connection
        .prompt_tuning;
    assert_eq!(
        tuning.translation.as_deref(),
        Some("PERSISTED_TRANSLATION_SUPPLEMENT")
    );
    assert!(tuning.dictation.is_none());
    assert!(tuning.ask.is_none());
    let good = std::fs::read(&path).unwrap();
    drop(reopened);
    for duplicate in [true, false] {
        let mut value: serde_json::Value = serde_json::from_slice(&good).unwrap();
        if duplicate {
            let record = value["connections"][0].clone();
            value["connections"].as_array_mut().unwrap().push(record);
        } else {
            value["active_connection_id"] = serde_json::json!("missing-id");
        }
        std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        assert_eq!(
            ConnectionStore::open(Arc::new(files::FileStorage::new(path.clone())))
                .err()
                .unwrap()
                .0,
            "configuration-store-invalid"
        );
    }
}

#[test]
fn public_cas_admits_one_concurrent_mutation_without_lost_updates() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("connections-v2.json");
    let store =
        Arc::new(ConnectionStore::open(Arc::new(files::FileStorage::new(path.clone()))).unwrap());
    let barrier = Arc::new(std::sync::Barrier::new(3));
    let handles = (0..2)
        .map(|_| {
            let store = store.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                store
                    .create(
                        0,
                        "concurrent".into(),
                        ConnectionKind::Custom,
                        ProviderDialect::Generic,
                    )
                    .map(|view| view.revision)
                    .map_err(|error| error.0)
            })
        })
        .collect::<Vec<_>>();
    barrier.wait();
    let results = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|result| **result == Err("configuration-conflict"))
            .count(),
        1
    );
    assert_eq!(store.view().unwrap().connections.len(), 1);
    drop(store);
    assert_eq!(
        ConnectionStore::open(Arc::new(files::FileStorage::new(path)))
            .unwrap()
            .view()
            .unwrap()
            .revision,
        1
    );
}

#[cfg(unix)]
#[test]
fn real_filesystem_commit_failure_preserves_both_memory_and_existing_disk_bytes() {
    use std::os::unix::fs::PermissionsExt;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("connections-v2.json");
    let store = ConnectionStore::open(Arc::new(files::FileStorage::new(path.clone()))).unwrap();
    store
        .create(
            0,
            "kept".into(),
            ConnectionKind::Custom,
            ProviderDialect::Generic,
        )
        .unwrap();
    let before = std::fs::read(&path).unwrap();
    struct Restore(std::path::PathBuf);
    impl Drop for Restore {
        fn drop(&mut self) {
            let _ = std::fs::set_permissions(&self.0, std::fs::Permissions::from_mode(0o700));
        }
    }
    let _restore = Restore(directory.path().to_owned());
    std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o500)).unwrap();
    assert!(store
        .create(
            1,
            "must not publish".into(),
            ConnectionKind::Custom,
            ProviderDialect::Generic
        )
        .is_err());
    assert_eq!(store.view().unwrap().revision, 1);
    assert_eq!(store.view().unwrap().connections.len(), 1);
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn owned_home_file_is_bounded_literal_and_never_uses_process_or_repository_values() {
    use credentials::{CredentialReader, HomeFileReader, PublicKey};
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(
        directory.path().join(".env"),
        "UNRELATED=other-secret\nDEEPSEEK_API_KEY='owned-only'\nGEMINI_API_KEY=other-provider\n",
    )
    .unwrap();
    let reader = HomeFileReader::new(directory.path().into());
    assert_eq!(
        reader.read(PublicKey::DeepSeek).unwrap().as_deref(),
        Some("owned-only")
    );
    assert_eq!(reader.read(PublicKey::OpenRouter).unwrap(), None);
    #[cfg(unix)]
    {
        std::fs::remove_file(directory.path().join(".env")).unwrap();
        std::fs::write(
            directory.path().join("target"),
            "DEEPSEEK_API_KEY=must-not-follow",
        )
        .unwrap();
        std::os::unix::fs::symlink(
            directory.path().join("target"),
            directory.path().join(".env"),
        )
        .unwrap();
        assert!(reader.read(PublicKey::DeepSeek).is_err());
    }
}
