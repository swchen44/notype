//! Completed-model storage policy shared by display and activation. It receives
//! an owned catalog snapshot and a cache-resolved path, never an AppHandle,
//! settings store or mutable catalog. Display cannot reconcile product state.

use std::fmt;
use std::path::PathBuf;

/// Catalog download state at the start of a lookup; queries never change it.
#[derive(Clone, Copy)]
pub enum DownloadState {
    /// The catalog has no verified completed download.
    Unavailable,
    /// A previously completed entry is currently being downloaded again.
    Downloading,
    /// The catalog advertises a completed file or directory; disk is checked.
    Complete,
}

/// Filesystem routing metadata, without download URLs or settings access.
pub enum StorageSource {
    /// A catalog model. The existing HF cache adapter resolves revisions and
    /// supplies an existing snapshot path; a local mirror file is the fallback.
    HuggingFace {
        /// Existing snapshot path returned by HF cache lookup, if available.
        cached_path: Option<PathBuf>,
    },
    /// A custom or legacy local model under the configured models directory.
    Local,
}

/// Stable storage failure classes. Only Missing permits activation's caller to
/// reconcile catalog availability; a display failure never requests mutation.
#[derive(Debug, PartialEq, Eq)]
pub enum StorageError {
    /// No completed download is advertised.
    Unavailable,
    /// A download is in progress.
    Downloading,
    /// The advertised completed path is absent or not the expected directory.
    Missing,
    /// A local model still has its partial marker.
    Incomplete,
}
impl fmt::Display for StorageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Unavailable => "Model not available",
            Self::Downloading => "Model is currently downloading",
            Self::Missing => "Complete model path not found",
            Self::Incomplete => "Model is incomplete",
        })
    }
}
impl std::error::Error for StorageError {}

/// The production manager decision boundary. Native Manager supplies catalog
/// lookup/reconciliation hooks; both native callers and portable fixtures use
/// these exact methods, so display cannot accidentally select activation policy.
pub trait ModelPathAccess {
    /// Adapter error type, preserving storage failures without Tauri coupling.
    type Error: From<StorageError>;
    /// Obtain one owned metadata/cache snapshot without changing catalog state.
    fn storage_snapshot(&self, model_id: &str) -> Result<ModelStorage, Self::Error>;
    /// Reconcile missing availability while preserving the saved selection.
    fn reconcile_missing(&self, model_id: &str);
    /// Display an existing completed model without cleanup or reconciliation.
    fn get_model_path_for_display(&self, model_id: &str) -> Result<PathBuf, Self::Error> {
        self.storage_snapshot(model_id)?
            .display_path()
            .map_err(Into::into)
    }
    /// Resolve for activation, retaining stale HF partial cleanup and notifying
    /// the manager only when an advertised completed path is missing.
    fn get_model_path(&self, model_id: &str) -> Result<PathBuf, Self::Error> {
        let result = self.storage_snapshot(model_id)?.activation_path();
        if matches!(result, Err(StorageError::Missing)) {
            self.reconcile_missing(model_id);
        }
        result.map_err(Into::into)
    }
}

/// An owned storage-query snapshot. Display is read-only; activation may remove
/// a stale HF mirror partial marker after a completed file is present. Neither
/// operation can modify the catalog or persisted model selection.
pub struct ModelStorage {
    models_dir: PathBuf,
    filename: String,
    is_directory: bool,
    state: DownloadState,
    source: StorageSource,
}

impl ModelStorage {
    /// Captures the manager's current metadata and resolved storage roots.
    /// Construction performs no I/O; filenames come from trusted discovery or
    /// catalog metadata, and an HF cached path must be resolved by the adapter.
    pub fn new(
        models_dir: PathBuf,
        filename: String,
        is_directory: bool,
        state: DownloadState,
        source: StorageSource,
    ) -> Self {
        Self {
            models_dir,
            filename,
            is_directory,
            state,
            source,
        }
    }

    /// Resolves an already completed model without deleting partial files or
    /// changing any state. An absent/incomplete model yields a typed error.
    pub fn display_path(&self) -> Result<PathBuf, StorageError> {
        self.resolve(false)
    }

    /// Uses the same resolver for activation, retaining historical HF mirror
    /// cleanup. Missing is returned for the manager to reconcile its catalog;
    /// this layer never changes catalog entries or persisted preferences.
    pub fn activation_path(&self) -> Result<PathBuf, StorageError> {
        self.resolve(true)
    }

    fn resolve(&self, cleanup_stale_partial: bool) -> Result<PathBuf, StorageError> {
        match self.state {
            DownloadState::Unavailable => return Err(StorageError::Unavailable),
            DownloadState::Downloading => return Err(StorageError::Downloading),
            DownloadState::Complete => {}
        }
        let path = self.models_dir.join(&self.filename);
        let partial = self.models_dir.join(format!("{}.partial", self.filename));
        if let StorageSource::HuggingFace { cached_path } = &self.source {
            if let Some(cached) = cached_path {
                return Ok(cached.clone());
            }
            if !path.exists() {
                return Err(StorageError::Missing);
            }
            if cleanup_stale_partial && partial.exists() {
                // Best-effort cleanup is the pre-existing activation policy.
                let _ = std::fs::remove_file(partial);
            }
            return Ok(path);
        }
        if !path.exists() || (self.is_directory && !path.is_dir()) {
            return Err(StorageError::Missing);
        }
        if partial.exists() {
            return Err(StorageError::Incomplete);
        }
        Ok(path)
    }
}
