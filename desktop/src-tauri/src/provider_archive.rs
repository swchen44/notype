//! Backend-only transfer data. Never return this secret-capable type as an IPC view.
use super::{
    binding, credentials, validate_store, BoundCredential, ConfigError, ConnectionDraft,
    ConnectionStore, StoredConnection, StoredSettings,
};
use crate::workflow::{ScenarioPrompts, TranslationSettings};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Owned saved-provider projection; None means keys deliberately excluded.
/// No Debug or frontend Type derivation: included values are actual App secrets.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProviderArchive {
    pub connections: Vec<ConnectionDraft>,
    pub active_connection_id: Option<String>,
    pub prompts: ScenarioPrompts,
    pub translation: TranslationSettings,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app_keys: Option<BTreeMap<String, String>>,
    pub home_key_variables: BTreeMap<String, String>,
}

impl ConnectionStore {
    /// Explicit backend export only; never resolves or reads home credentials.
    pub(crate) fn archive(&self, include_app_keys: bool) -> Result<ProviderArchive, ConfigError> {
        let state = self.snapshot()?;
        let home_key_variables = references(
            &state
                .connections
                .iter()
                .map(|r| r.draft.clone())
                .collect::<Vec<_>>(),
        );
        let app_keys = include_app_keys.then(|| {
            state
                .connections
                .iter()
                .filter_map(|record| {
                    record
                        .key
                        .as_ref()
                        .map(|key| (record.draft.id.clone(), key.value.clone()))
                })
                .collect()
        });
        let archive = ProviderArchive {
            home_key_variables,
            connections: state.connections.into_iter().map(|r| r.draft).collect(),
            active_connection_id: state.active_connection_id,
            prompts: state.prompts,
            translation: state.translation,
            app_keys,
        };
        archive.validate()?;
        Ok(archive)
    }

    /// Prepare a complete next-startup file; this never mutates the live store.
    pub(crate) fn replacement(&self, archive: &ProviderArchive) -> Result<Vec<u8>, ConfigError> {
        let current = self.snapshot()?;
        archive.validate()?;
        let mut effective = archive.clone();
        if effective.app_keys.is_none() {
            let mut retained = BTreeMap::new();
            for draft in &effective.connections {
                let Some(previous) = current.connections.iter().find(|r| r.draft.id == draft.id)
                else {
                    continue;
                };
                let Some(key) = &previous.key else {
                    continue;
                };
                if previous.draft.authentication == draft.authentication
                    && binding(draft).ok().as_deref() == Some(key.binding.as_str())
                {
                    retained.insert(draft.id.clone(), key.value.clone());
                }
            }
            effective.app_keys = Some(retained);
        }
        let next_revision = current
            .revision
            .checked_add(1)
            .filter(|r| *r <= 9_007_199_254_740_991)
            .ok_or(ConfigError("configuration-revision-exhausted"))?;
        let state = effective.state(next_revision, current.bootstrap_applied)?;
        serde_json::to_vec(&state).map_err(|_| ConfigError("configuration-archive-invalid"))
    }
}

impl ProviderArchive {
    /// Validate an isolated archive, without resolving credentials or applying it.
    pub(crate) fn validate(&self) -> Result<(), ConfigError> {
        self.state(0, None).map(|_| ())
    }

    fn state(
        &self,
        revision: u64,
        bootstrap_applied: Option<String>,
    ) -> Result<StoredSettings, ConfigError> {
        if self.home_key_variables != references(&self.connections)
            || self.app_keys.as_ref().is_some_and(|keys| {
                keys.keys()
                    .any(|id| !self.connections.iter().any(|draft| &draft.id == id))
            })
        {
            return Err(ConfigError("configuration-archive-invalid"));
        }
        let connections = self
            .connections
            .iter()
            .map(|draft| {
                let key = match self.app_keys.as_ref().and_then(|keys| keys.get(&draft.id)) {
                    Some(value) => Some(BoundCredential {
                        value: value.clone(),
                        binding: binding(draft)?,
                    }),
                    None => None,
                };
                Ok(StoredConnection {
                    draft: draft.clone(),
                    key,
                })
            })
            .collect::<Result<Vec<_>, ConfigError>>()?;
        let state = StoredSettings {
            revision,
            bootstrap_applied,
            connections,
            active_connection_id: self.active_connection_id.clone(),
            prompts: self.prompts.clone(),
            translation: self.translation.clone(),
            ..StoredSettings::default()
        };
        validate_store(&state)?;
        if serde_json::to_vec(&state)
            .map_err(|_| ConfigError("configuration-archive-invalid"))?
            .len()
            > 4_194_304
        {
            return Err(ConfigError("configuration-store-limit"));
        }
        Ok(state)
    }
}

fn references(connections: &[ConnectionDraft]) -> BTreeMap<String, String> {
    connections
        .iter()
        .filter_map(|draft| {
            credentials::public_key(draft).map(|key| (draft.id.clone(), key.name().into()))
        })
        .collect()
}
