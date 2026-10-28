//! Optional per-connection supplements, composed before a session snapshot is used.
use super::ConfigError;
use crate::workflow::ScenarioPrompts;
use serde::{Deserialize, Serialize};
use specta::Type;

/// Independent scene supplements. None preserves the shared prompt byte for byte.
#[derive(Clone, Default, Serialize, Deserialize, Type)]
#[serde(default, deny_unknown_fields)]
pub struct ConnectionPromptTuning {
    /// Optional dictation supplement, at most 8192 UTF-8 bytes.
    pub dictation: Option<String>,
    /// Optional translation supplement, at most 8192 UTF-8 bytes.
    pub translation: Option<String>,
    /// Optional ask supplement, at most 8192 UTF-8 bytes.
    pub ask: Option<String>,
}

impl ConnectionPromptTuning {
    pub(super) fn validate(&self) -> Result<(), ConfigError> {
        for prompt in [&self.dictation, &self.translation, &self.ask]
            .into_iter()
            .flatten()
        {
            if prompt.trim().is_empty() || prompt.len() > 8192 || prompt.contains('\0') {
                return Err(ConfigError("prompt-tuning-invalid"));
            }
        }
        Ok(())
    }

    pub(super) fn compose(&self, shared: &ScenarioPrompts) -> Result<ScenarioPrompts, ConfigError> {
        self.validate()?;
        Ok(ScenarioPrompts {
            dictation: append(&shared.dictation, &self.dictation)?,
            translation: append(&shared.translation, &self.translation)?,
            ask: append(&shared.ask, &self.ask)?,
        })
    }
}

fn append(shared: &str, tuning: &Option<String>) -> Result<String, ConfigError> {
    let combined = match tuning {
        Some(extra) => format!("{shared}\n\n{extra}"),
        None => shared.to_owned(),
    };
    if combined.len() > 32768 {
        return Err(ConfigError("prompt-tuning-too-long"));
    }
    Ok(combined)
}
