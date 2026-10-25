#[path = "../../desktop/src-tauri/src/managers/model_storage.rs"]
mod model_storage;
use model_storage::{DownloadState, ModelStorage, StorageError, StorageSource};
use std::path::PathBuf;

struct Fixture(PathBuf);
impl Fixture {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!("notype-storage-{name}-{}", std::process::id()));
        std::fs::create_dir_all(root.join("models")).unwrap();
        Self(root)
    }
    fn storage(&self, filename: &str, directory: bool, source: StorageSource) -> ModelStorage {
        ModelStorage::new(self.0.join("models"), filename.into(), directory, DownloadState::Complete, source)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) { std::fs::remove_dir_all(&self.0).unwrap(); }
}

#[test]
fn hf_display_preserves_stale_partial_and_catalog_selection_while_activation_cleans_partial() {
    let fixture = Fixture::new("hf-fallback");
    let complete = fixture.0.join("models/catalog.gguf");
    let partial = fixture.0.join("models/catalog.gguf.partial");
    let settings = fixture.0.join("settings.json");
    let catalog = fixture.0.join("catalog.json");
    std::fs::write(&complete, "verified fixture").unwrap();
    std::fs::write(&partial, "stale fixture").unwrap();
    std::fs::write(&settings, r#"{"selected_model":"catalog"}"#).unwrap();
    std::fs::write(&catalog, r#"{"catalog":{"is_downloaded":true}}"#).unwrap();
    let storage = fixture.storage("catalog.gguf", false, StorageSource::HuggingFace { cached_path: None });
    assert_eq!(storage.display_path().unwrap(), complete);
    assert_eq!(std::fs::read_to_string(&partial).unwrap(), "stale fixture");
    assert_eq!(std::fs::read_to_string(&settings).unwrap(), r#"{"selected_model":"catalog"}"#);
    assert_eq!(std::fs::read_to_string(&catalog).unwrap(), r#"{"catalog":{"is_downloaded":true}}"#);
    assert_eq!(storage.activation_path().unwrap(), complete);
    assert!(!partial.exists());
    std::fs::remove_file(&complete).unwrap();
    assert_eq!(storage.display_path(), Err(StorageError::Missing));
    assert_eq!(std::fs::read_to_string(&settings).unwrap(), r#"{"selected_model":"catalog"}"#);
    assert_eq!(std::fs::read_to_string(&catalog).unwrap(), r#"{"catalog":{"is_downloaded":true}}"#);
    std::fs::write(&complete, "verified replacement").unwrap();
    assert_eq!(storage.display_path().unwrap(), complete);
    std::fs::remove_file(&complete).unwrap();
    assert_eq!(storage.activation_path(), Err(StorageError::Missing));
}

#[test]
fn hf_cache_resolved_snapshot_takes_precedence_over_local_mirror_without_cleanup() {
    let fixture = Fixture::new("hf-cache");
    let cache_dir = fixture.0.join("huggingface/hub/models--fixture--speech/snapshots/commit");
    std::fs::create_dir_all(&cache_dir).unwrap();
    let cached_path = cache_dir.join("catalog.gguf");
    std::fs::write(&cached_path, "cached fixture").unwrap();
    std::fs::write(fixture.0.join("models/catalog.gguf"), "mirror fixture").unwrap();
    let partial = fixture.0.join("models/catalog.gguf.partial");
    std::fs::write(&partial, "stale mirror partial").unwrap();
    let storage = fixture.storage("catalog.gguf", false, StorageSource::HuggingFace {cached_path: Some(cached_path.clone())});
    assert_eq!(storage.display_path().unwrap(), cached_path);
    assert_eq!(storage.activation_path().unwrap(), cached_path);
    assert_eq!(std::fs::read_to_string(partial).unwrap(), "stale mirror partial");
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
    assert_eq!(std::fs::read_to_string(&partial).unwrap(), "incomplete fixture");
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
    assert_eq!(std::fs::read_to_string(&partial).unwrap(), "directory fixture partial");
    std::fs::remove_dir(&complete).unwrap();
    std::fs::write(&complete, "not a directory").unwrap();
    assert_eq!(storage.display_path(), Err(StorageError::Missing));
    for (state, error) in [(DownloadState::Unavailable, StorageError::Unavailable), (DownloadState::Downloading, StorageError::Downloading)] {
        let storage = ModelStorage::new(fixture.0.join("models"), "custom-directory".into(), false, state, StorageSource::Local);
        assert_eq!(storage.display_path(), Err(error));
        assert_eq!(std::fs::read_to_string(&complete).unwrap(), "not a directory");
        assert_eq!(std::fs::read_to_string(&partial).unwrap(), "directory fixture partial");
    }
}
