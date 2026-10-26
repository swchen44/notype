//! First-use selection in the existing settings_store.json envelope.
//! Construction/read-only recovery never creates defaults or opens plugin cache.
use crate::provider_config::{validate_bootstrap_id, ConfigError, ConfigurationStorage};
use serde_json::{json, Value};
use std::sync::Arc;

pub struct SelectionStore {
    storage: Arc<dyn ConfigurationStorage>,
}
impl SelectionStore {
    pub fn new(storage: Arc<dyn ConfigurationStorage>) -> Self {
        Self { storage }
    }
    /// Existing settings, including a deliberately blank selection, always win.
    pub fn pending(&self) -> Result<bool, ConfigError> {
        Ok(self.read()?.is_none())
    }
    /// Return existing settings without saving, salvaging or reading credentials.
    pub fn read(&self) -> Result<Option<Value>, ConfigError> {
        let Some(bytes) = self.storage.load()? else {
            return Ok(None);
        };
        let value: Value = serde_json::from_slice(&bytes)
            .map_err(|_| ConfigError("bootstrap-settings-invalid"))?;
        if !value.is_object() {
            return Err(ConfigError("bootstrap-settings-invalid"));
        }
        if let Some(settings) = value.get("settings") {
            if !settings.is_object() {
                return Err(ConfigError("bootstrap-settings-invalid"));
            }
        }
        // An existing envelope without settings is still existing user state.
        Ok(Some(
            value.get("settings").cloned().unwrap_or_else(|| json!({})),
        ))
    }
    /// One durable same-file commit precedes first plugin open/publication.
    pub fn initialize_if_absent(
        &self,
        mut defaults: Value,
        model: &str,
        id: &str,
    ) -> Result<bool, ConfigError> {
        if !self.pending()? {
            return Ok(false);
        }
        validate_bootstrap_id(id)?;
        let settings = defaults
            .as_object_mut()
            .ok_or(ConfigError("bootstrap-settings-invalid"))?;
        settings.insert("selected_model".into(), Value::String(model.into()));
        // Selection is available; permissions/onboarding remain user-controlled.
        let bytes = serde_json::to_vec(&json!({"settings":defaults,
            "notype_bootstrap":{"schema":1,"applied":id}}))
        .map_err(|_| ConfigError("bootstrap-settings-invalid"))?;
        self.storage.commit(&bytes)?;
        Ok(true)
    }
}
