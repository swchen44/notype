//! Backend-only whole-configuration archive. No home credential reads or live writes.
use crate::settings_journal::Domain;
use crate::{
    provider_config::{archive::ProviderArchive, ConnectionStore},
    settings_yaml,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

/// Backend-only disk baseline. App cache autosave is the only permitted drift.
pub struct DiskStamp {
    files: BTreeMap<Domain, Option<Value>>,
    autosave: Value,
}
impl DiskStamp {
    pub fn new(files: BTreeMap<Domain, Option<Value>>, app_cache: Value) -> Self {
        let mut autosave = files
            .get(&Domain::App)
            .and_then(Clone::clone)
            .unwrap_or_else(|| serde_json::json!({}));
        if let Some(envelope) = autosave.as_object_mut() {
            envelope.insert("settings".into(), app_cache);
        }
        Self { files, autosave }
    }
    pub fn matches(&self, current: &BTreeMap<Domain, Option<Value>>) -> bool {
        self.files.len() == 4
            && current.len() == 4
            && self.files.iter().all(|(domain, before)| {
                current.get(domain).is_some_and(|now| {
                    now == before
                        || (*domain == Domain::App && now.as_ref() == Some(&self.autosave))
                })
            })
    }
}

/// Secret-capable transfer document; never Debug/IPC.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Archive {
    pub notype_settings: u8,
    pub settings: Value,
    pub ai: ProviderArchive,
    pub model_folder: String,
    pub diagnostics_enabled: bool,
}
const RETIRED: [&str; 6] = [
    "post_process_provider_id",
    "post_process_providers",
    "post_process_api_keys",
    "post_process_models",
    "post_process_prompts",
    "post_process_selected_prompt_id",
];

pub fn project_settings(mut value: Value) -> Result<Value, String> {
    let object = value.as_object_mut().ok_or("settings-transfer-invalid")?;
    for key in RETIRED {
        object.remove(key);
    }
    Ok(value)
}
impl Archive {
    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        let value = settings_yaml::parse(bytes).map_err(|error| error.to_string())?;
        let archive: Self =
            serde_json::from_value(value.clone()).map_err(|_| "settings-transfer-invalid")?;
        // Reject fields silently dropped by nested provider deserializers too.
        if serde_json::to_value(&archive).map_err(|_| "settings-transfer-invalid")? != value {
            return Err("settings-transfer-invalid".into());
        }
        archive.validate()?;
        Ok(archive)
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.notype_settings != 1
            || !self.settings.is_object()
            || RETIRED.iter().any(|key| self.settings.get(key).is_some())
            || self.model_folder.is_empty()
            || self.model_folder.len() > 8192
            || self.model_folder.chars().any(char::is_control)
        {
            return Err("settings-transfer-invalid".into());
        }
        self.ai
            .validate()
            .map_err(|_| "settings-transfer-invalid".to_string())
    }
    pub fn encode(&self) -> Result<String, String> {
        self.validate()?;
        settings_yaml::render(&serde_json::to_value(self).map_err(|_| "settings-transfer-invalid")?)
            .map_err(|error| error.to_string())
    }
    /// Safe UI projection: actual App keys are never sent to the webview.
    pub fn preview(&self) -> Result<String, String> {
        let mut value = serde_json::to_value(self).map_err(|_| "settings-transfer-invalid")?;
        let ai = value
            .get_mut("ai")
            .and_then(Value::as_object_mut)
            .ok_or("settings-transfer-invalid")?;
        if let Some(keys) = ai.get_mut("app_keys").and_then(Value::as_object_mut) {
            for value in keys.values_mut() {
                *value = Value::String("********".into());
            }
        }
        settings_yaml::render(&value).map_err(|error| error.to_string())
    }
    pub fn prepared_providers(&self, store: &ConnectionStore) -> Result<String, String> {
        String::from_utf8(
            store
                .replacement(&self.ai)
                .map_err(|_| "settings-transfer-invalid")?,
        )
        .map_err(|_| "settings-transfer-invalid".into())
    }
}
/// Stable logical fingerprints tolerate JSON map order/autosave whitespace only.
pub fn digest(value: &Value) -> String {
    let bytes = serde_json::to_vec(value).expect("JSON value is serializable");
    format!("{:x}", Sha256::digest(bytes))
}
/// Names only, never setting/credential values. Arrays are one readable change.
pub fn changed_fields(before: &Value, after: &Value) -> Vec<String> {
    fn visit(a: &Value, b: &Value, prefix: &str, out: &mut Vec<String>) {
        if a == b || out.len() >= 256 {
            return;
        }
        if let (Some(a), Some(b)) = (a.as_object(), b.as_object()) {
            let keys: std::collections::BTreeSet<_> = a.keys().chain(b.keys()).collect();
            for key in keys {
                let path = if prefix.is_empty() {
                    key.to_string()
                } else {
                    format!("{prefix}.{key}")
                };
                visit(
                    a.get(key).unwrap_or(&Value::Null),
                    b.get(key).unwrap_or(&Value::Null),
                    &path,
                    out,
                );
            }
        } else {
            out.push(prefix.into());
        }
    }
    let mut fields = Vec::new();
    visit(before, after, "", &mut fields);
    fields
}
