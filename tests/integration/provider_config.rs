#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/capture_startup.rs"]
mod capture_startup;
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/diagnostics.rs"]
mod diagnostics;
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/provider_config.rs"]
mod provider_config;
#[allow(dead_code, unused_imports)]
#[path = "../../desktop/src-tauri/src/workflow.rs"]
mod workflow;
use provider_config::*;
use std::sync::Arc;

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
    store.activate(1, Some(id.clone())).unwrap();
    drop(store);
    let reopened = ConnectionStore::open(Arc::new(files::FileStorage::new(path.clone()))).unwrap();
    assert_eq!(reopened.view().unwrap().active_connection_id, Some(id));
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
