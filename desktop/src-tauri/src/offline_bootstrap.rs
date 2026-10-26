//! Platform-independent first-use coordinator. No credential reader or HTTP port.
use crate::bootstrap_selection::SelectionStore;
use crate::managers::model_roots::{ArtifactStatus, Candidate, RootService};
use crate::provider_config::{
    validate_bootstrap_id, ConfigError, ConnectionPreset, ConnectionStore,
};
use serde::{Deserialize, Serialize};
use std::{
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
};

#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SttPreset {
    pub model_id: String,
    pub root: String,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BootstrapInput {
    pub schema: u8,
    pub bootstrap_id: String,
    pub stt: SttPreset,
    pub connection: Option<ConnectionPreset>,
}
impl BootstrapInput {
    pub fn parse(bytes: &[u8]) -> Result<Self, ConfigError> {
        if bytes.len() > 65_536 {
            return Err(ConfigError("bootstrap-invalid"));
        }
        let input: Self =
            serde_json::from_slice(bytes).map_err(|_| ConfigError("bootstrap-invalid"))?;
        input.validate_envelope()?;
        Ok(input)
    }
    pub fn validate_envelope(&self) -> Result<(), ConfigError> {
        validate_bootstrap_id(&self.bootstrap_id)?;
        if self.schema != 1
            || self.stt.root != "models"
            || self.stt.model_id.is_empty()
            || self.stt.model_id.len() > 256
            || self
                .stt
                .model_id
                .chars()
                .any(|c| c.is_control() || c == '$' || c == '%')
        {
            return Err(ConfigError("bootstrap-invalid"));
        }
        Ok(())
    }
}
/// Finite safe local status. No paths, raw input, endpoint or model values.
#[derive(Clone, Serialize, specta::Type)]
pub struct BootstrapStatus {
    pub state: String,
    pub code: Option<String>,
    pub can_retry: bool,
}
impl BootstrapStatus {
    fn ready() -> Self {
        Self {
            state: "ready".into(),
            code: None,
            can_retry: false,
        }
    }
    fn pending() -> Self {
        Self {
            state: "pending".into(),
            code: None,
            can_retry: false,
        }
    }
    fn incomplete(code: &str) -> Self {
        Self {
            state: "incomplete".into(),
            code: Some(code.into()),
            can_retry: true,
        }
    }
}
static STATUS: Mutex<Option<BootstrapStatus>> = Mutex::new(None);
static RUNTIME_READY: AtomicBool = AtomicBool::new(false);
pub fn status() -> BootstrapStatus {
    let status = STATUS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
        .unwrap_or_else(BootstrapStatus::ready);
    if status.state == "ready" && !RUNTIME_READY.load(Ordering::Acquire) {
        BootstrapStatus::pending()
    } else {
        status
    }
}
pub fn domains_ready() -> bool {
    STATUS
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
        .is_none_or(|s| s.state == "ready")
}
pub fn runtime_ready() {
    RUNTIME_READY.store(true, Ordering::Release);
}
pub fn blocked() -> bool {
    status().state != "ready"
}
pub fn begin() {
    RUNTIME_READY.store(false, Ordering::Release);
    *STATUS.lock().unwrap_or_else(|e| e.into_inner()) = Some(BootstrapStatus::pending());
}
pub fn finish(result: Result<(), ConfigError>) -> BootstrapStatus {
    let status = match result {
        Ok(()) => BootstrapStatus::ready(),
        Err(error) => BootstrapStatus::incomplete(error.0),
    };
    *STATUS.lock().unwrap_or_else(|e| e.into_inner()) = Some(status.clone());
    status
}
pub fn admit() -> Result<(), String> {
    if blocked() {
        Err("bootstrap-incomplete".into())
    } else {
        Ok(())
    }
}
/// Read-only recovery IPC and root repair remain reachable; all other admission
/// is rejected before dispatch while defaults/transactions are pending.
pub fn recovery_command(command: &str) -> bool {
    matches!(
        command,
        "get_bootstrap_status"
            | "retry_offline_bootstrap"
            | "get_app_settings"
            | "get_default_settings"
            | "get_text_configuration"
            | "get_history_entries"
            | "get_workflow_readiness"
            | "get_model_root"
            | "get_model_catalog"
            | "get_available_models"
            | "get_current_model"
            | "validate_model_root"
            | "apply_model_root"
            | "cancel_model_root_scan"
            | "rescan_local_models"
            | "show_main_window_command"
            | "cancel_operation"
            | "get_model_path"
            | "show_model_root"
            | "is_portable"
            | "is_update_checks_locked"
            | "plugin:dialog|open"
            | "plugin:event|listen"
            | "plugin:event|unlisten"
    )
}
/// Whole required-domain validation precedes every first-use durable write.
pub fn initialize(
    input: &BootstrapInput,
    models: &Path,
    roots: &RootService,
    provider: &ConnectionStore,
    selection: &SelectionStore,
    defaults: serde_json::Value,
) -> Result<(), ConfigError> {
    input.validate_envelope()?;
    let root_pending = !roots.initialized();
    let selection_pending = selection.pending()?;
    let provider_pending = input.connection.is_some() && provider.bootstrap_pending()?;
    if provider_pending {
        input
            .connection
            .as_ref()
            .ok_or(ConfigError("bootstrap-invalid"))?
            .validate()?;
    }
    let candidate: Option<Candidate> = if root_pending {
        let candidate = roots
            .validate(models, "bundle_bootstrap", &|_, _| {})
            .map_err(|_| ConfigError("bootstrap-model-unavailable"))?;
        Some(candidate)
    } else {
        None
    };
    let mut selected_default = input.stt.model_id.as_str();
    if root_pending || selection_pending {
        let projection = candidate
            .as_ref()
            .map(|c| c.projection.clone())
            .unwrap_or_else(|| roots.projection());
        if !projection
            .artifacts
            .get(&input.stt.model_id)
            .is_some_and(|artifact| artifact.status == ArtifactStatus::Verified)
        {
            if root_pending {
                return Err(ConfigError("bootstrap-model-unavailable"));
            }
            // The explicit root wins. Leave ordinary first-run model choice to
            // the user instead of selecting a different artifact automatically.
            selected_default = "";
        }
    }
    if let Some(candidate) = candidate {
        roots
            .apply_bootstrap(&candidate.id, candidate.revision, &input.bootstrap_id)
            .map_err(|_| ConfigError("bootstrap-root-persistence-failed"))?;
    }
    if provider_pending {
        provider.bootstrap_if_empty(
            input
                .connection
                .clone()
                .ok_or(ConfigError("bootstrap-invalid"))?,
            &input.bootstrap_id,
        )?;
    }
    if selection_pending {
        selection.initialize_if_absent(defaults, selected_default, &input.bootstrap_id)?;
    }
    Ok(())
}
