//! Local Settings projections. Folder paths are intentionally confined to these
//! explicit settings responses; diagnostics and history never receive them.
use super::model::ModelInfo;
use serde::{Deserialize, Serialize};
use specta::Type;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ModelRootStatus {
    Ready,
    NoModels,
    Unavailable,
    Invalid,
}

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct ModelRootView {
    pub generation: u64,
    pub revision: u64,
    pub folder: String,
    pub status: ModelRootStatus,
    pub writable: bool,
    pub busy: bool,
    pub model_count: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct ModelRootCandidate {
    pub request_id: String,
    pub candidate_id: String,
    pub root: ModelRootView,
}

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ModelArtifactStatus {
    Verified,
    CustomUnverified,
    Invalid,
    Incomplete,
    Unavailable,
}

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
pub struct ModelArtifactView {
    pub source: ModelArtifactSource,
    pub status: ModelArtifactStatus,
    pub can_delete: bool,
    pub can_download: bool,
}

#[derive(Clone, Debug, Serialize, Type)]
pub struct ModelCatalogView {
    pub generation: u64,
    pub root: ModelRootView,
    pub current_model: String,
    pub models: Vec<ModelInfo>,
    pub artifacts: BTreeMap<String, ModelArtifactView>,
}

/// Events carry no paths or raw filesystem errors.
#[derive(Clone, Debug, Serialize, Type)]
pub struct ModelRootScanProgress {
    pub generation: u64,
    pub request_id: String,
    pub checked: u32,
    pub total: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ModelArtifactSource {
    SelectedRoot,
    LegacyCache,
}
