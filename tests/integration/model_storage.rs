#[path = "../../desktop/src-tauri/src/managers/model_storage.rs"]
mod model_storage;
use model_storage::{DownloadState, ModelPathAccess, ModelStorage, StorageError, StorageSource};
use std::cell::RefCell;
use std::path::PathBuf;

#[test]
fn selected_root_display_and_activation_never_run_legacy_partial_cleanup() {
    let fixture = Fixture::new("selected-root");
    let path = fixture.0.join("models/catalog.gguf");
    let partial = fixture.0.join("models/catalog.gguf.partial");
    std::fs::write(&path, b"verified fixture").unwrap();
    std::fs::write(&partial, b"retain unowned bytes").unwrap();
    let snapshot = ModelStorage::selected_root(path.clone());
    assert_eq!(snapshot.display_path().unwrap(), path);
    assert_eq!(snapshot.activation_path().unwrap(), path);
    assert_eq!(std::fs::read(partial).unwrap(), b"retain unowned bytes");
}

struct Fixture(PathBuf);
impl Fixture {
    fn new(name: &str) -> Self {
        let root =
            std::env::temp_dir().join(format!("notype-storage-{name}-{}", std::process::id()));
        std::fs::create_dir_all(root.join("models")).unwrap();
        Self(root)
    }
    fn storage(&self, filename: &str, directory: bool, source: StorageSource) -> ModelStorage {
        ModelStorage::new(
            self.0.join("models"),
            filename.into(),
            directory,
            DownloadState::Complete,
            source,
        )
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn hf_display_preserves_stale_partial_while_activation_cleans_partial() {
    let fixture = Fixture::new("hf-fallback");
    let complete = fixture.0.join("models/catalog.gguf");
    let partial = fixture.0.join("models/catalog.gguf.partial");
    std::fs::write(&complete, "verified fixture").unwrap();
    std::fs::write(&partial, "stale fixture").unwrap();
    let storage = fixture.storage(
        "catalog.gguf",
        false,
        StorageSource::HuggingFace { cached_path: None },
    );
    assert_eq!(storage.display_path().unwrap(), complete);
    assert_eq!(std::fs::read_to_string(&partial).unwrap(), "stale fixture");
    assert_eq!(storage.activation_path().unwrap(), complete);
    assert!(!partial.exists());
    std::fs::remove_file(&complete).unwrap();
    assert_eq!(storage.display_path(), Err(StorageError::Missing));
    std::fs::write(&complete, "verified replacement").unwrap();
    assert_eq!(storage.display_path().unwrap(), complete);
    std::fs::remove_file(&complete).unwrap();
    assert_eq!(storage.activation_path(), Err(StorageError::Missing));
}

struct Host {
    directory: PathBuf,
    catalog_available: RefCell<bool>,
    selection: RefCell<String>,
    reconciliations: RefCell<Vec<String>>,
}

#[test]
fn a_missing_selected_root_snapshot_cannot_reconcile_a_successor_by_model_id() {
    struct SelectedHost(RefCell<u32>);
    impl ModelPathAccess for SelectedHost {
        type Error = StorageError;
        fn storage_snapshot(&self, _: &str) -> Result<ModelStorage, Self::Error> {
            Ok(ModelStorage::selected_root(PathBuf::new()))
        }
        fn reconcile_missing(&self, _: &str) {
            *self.0.borrow_mut() += 1;
        }
    }
    let host = SelectedHost(RefCell::new(0));
    assert_eq!(host.get_model_path("same-id"), Err(StorageError::Missing));
    assert_eq!(*host.0.borrow(), 0);
}
impl ModelPathAccess for Host {
    type Error = StorageError;
    fn storage_snapshot(&self, _: &str) -> Result<ModelStorage, Self::Error> {
        Ok(ModelStorage::new(
            self.directory.clone(),
            "catalog.gguf".into(),
            false,
            if *self.catalog_available.borrow() {
                DownloadState::Complete
            } else {
                DownloadState::Unavailable
            },
            StorageSource::HuggingFace { cached_path: None },
        ))
    }
    fn reconcile_missing(&self, id: &str) {
        self.reconciliations.borrow_mut().push(id.into());
        *self.catalog_available.borrow_mut() = false;
        // Same production contract: catalog availability changes, saved
        // selection is preserved for a currently loaded engine/re-download.
    }
}

#[test]
fn display_query_never_activates_or_reconciles_catalog() {
    let fixture = Fixture::new("manager-route");
    let host = Host {
        directory: fixture.0.join("models"),
        catalog_available: RefCell::new(true),
        selection: RefCell::new("catalog".into()),
        reconciliations: RefCell::new(Vec::new()),
    };
    let complete = host.directory.join("catalog.gguf");
    let partial = host.directory.join("catalog.gguf.partial");
    std::fs::write(&complete, "verified fixture").unwrap();
    std::fs::write(&partial, "stale partial").unwrap();
    assert_eq!(
        host.get_model_path_for_display("catalog").unwrap(),
        complete
    );
    assert!(partial.exists());
    assert_eq!(host.get_model_path("catalog").unwrap(), complete);
    assert!(!partial.exists());
    std::fs::remove_file(&complete).unwrap();
    assert_eq!(
        host.get_model_path_for_display("catalog"),
        Err(StorageError::Missing)
    );
    assert!(host.reconciliations.borrow().is_empty());
    assert!(*host.catalog_available.borrow());
    assert_eq!(*host.selection.borrow(), "catalog");
    // Positive control uses the exact production activation decision method.
    assert_eq!(host.get_model_path("catalog"), Err(StorageError::Missing));
    assert_eq!(*host.reconciliations.borrow(), vec!["catalog"]);
    assert!(!*host.catalog_available.borrow());
    assert_eq!(*host.selection.borrow(), "catalog");
}

#[test]
fn hf_cache_resolved_snapshot_takes_precedence_over_local_mirror_without_cleanup() {
    let fixture = Fixture::new("hf-cache");
    let cache_dir = fixture
        .0
        .join("huggingface/hub/models--fixture--speech/snapshots/commit");
    std::fs::create_dir_all(&cache_dir).unwrap();
    let cached_path = cache_dir.join("catalog.gguf");
    std::fs::write(&cached_path, "cached fixture").unwrap();
    std::fs::write(fixture.0.join("models/catalog.gguf"), "mirror fixture").unwrap();
    let partial = fixture.0.join("models/catalog.gguf.partial");
    std::fs::write(&partial, "stale mirror partial").unwrap();
    let storage = fixture.storage(
        "catalog.gguf",
        false,
        StorageSource::HuggingFace {
            cached_path: Some(cached_path.clone()),
        },
    );
    assert_eq!(storage.display_path().unwrap(), cached_path);
    assert_eq!(storage.activation_path().unwrap(), cached_path);
    assert_eq!(
        std::fs::read_to_string(partial).unwrap(),
        "stale mirror partial"
    );
}

#[test]
fn custom_files_remain_incomplete_until_partial_removed_and_missing_display_is_not_sticky() {
    let fixture = Fixture::new("custom-file");
    let complete = fixture.0.join("models/custom.gguf");
    let partial = fixture.0.join("models/custom.gguf.partial");
    std::fs::write(&complete, "custom fixture").unwrap();
    std::fs::write(&partial, "incomplete fixture").unwrap();
    let storage = fixture.storage("custom.gguf", false, StorageSource::Local);
    assert_eq!(storage.display_path(), Err(StorageError::Incomplete));
    assert_eq!(storage.activation_path(), Err(StorageError::Incomplete));
    assert_eq!(
        std::fs::read_to_string(&partial).unwrap(),
        "incomplete fixture"
    );
    std::fs::remove_file(&partial).unwrap();
    assert_eq!(storage.display_path().unwrap(), complete);
    std::fs::remove_file(&complete).unwrap();
    assert_eq!(storage.display_path(), Err(StorageError::Missing));
    std::fs::write(&complete, "returned fixture").unwrap();
    assert_eq!(storage.display_path().unwrap(), complete);
}

#[test]
fn custom_directory_and_download_state_checks_preserve_owned_files() {
    let fixture = Fixture::new("custom-directory");
    let complete = fixture.0.join("models/custom-directory");
    let partial = fixture.0.join("models/custom-directory.partial");
    std::fs::create_dir(&complete).unwrap();
    let storage = fixture.storage("custom-directory", true, StorageSource::Local);
    assert_eq!(storage.display_path().unwrap(), complete);
    std::fs::write(&partial, "directory fixture partial").unwrap();
    assert_eq!(storage.display_path(), Err(StorageError::Incomplete));
    assert_eq!(storage.activation_path(), Err(StorageError::Incomplete));
    assert_eq!(
        std::fs::read_to_string(&partial).unwrap(),
        "directory fixture partial"
    );
    std::fs::remove_dir(&complete).unwrap();
    std::fs::write(&complete, "not a directory").unwrap();
    assert_eq!(storage.display_path(), Err(StorageError::Missing));
    for (state, error) in [
        (DownloadState::Unavailable, StorageError::Unavailable),
        (DownloadState::Downloading, StorageError::Downloading),
    ] {
        let storage = ModelStorage::new(
            fixture.0.join("models"),
            "custom-directory".into(),
            false,
            state,
            StorageSource::Local,
        );
        assert_eq!(storage.display_path(), Err(error));
        assert_eq!(
            std::fs::read_to_string(&complete).unwrap(),
            "not a directory"
        );
        assert_eq!(
            std::fs::read_to_string(&partial).unwrap(),
            "directory fixture partial"
        );
    }
}
