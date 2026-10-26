//! Owned ephemeral settings and opaque credential transfer. No App settings,
//! secret serialization, alternate dotenv parser or Custom fallback is added.
use super::super::provider_config as config;
use super::{
    journal::Failure,
    policy::{Provider, Slot},
};
use crate::{
    diagnostics::api::TraceContext,
    workflow::{
        self,
        experiment::{
            self, AdverseLatch, ExperimentKind, ExperimentPolicy, ExperimentProcessor,
            OwnedLoopback,
        },
        TextConfiguration,
    },
};
use std::sync::Arc;
struct Memory;
impl config::ConfigurationStorage for Memory {
    fn load(&self) -> Result<Option<Vec<u8>>, config::ConfigError> {
        Ok(None)
    }
    fn commit(&self, _: &[u8]) -> Result<(), config::ConfigError> {
        Ok(())
    }
}
struct NoHome;
impl config::credentials::CredentialReader for NoHome {
    fn read(
        &self,
        _: config::credentials::PublicKey,
    ) -> Result<Option<String>, config::ConfigError> {
        Err(config::ConfigError("configuration-credentials-unavailable"))
    }
}
fn draft(
    store: &config::ConnectionStore,
    provider: Provider,
    owned: Option<&OwnedLoopback>,
) -> Result<config::ConnectionDraft, Failure> {
    use config::*;
    if provider == Provider::GeminiSealed {
        return Err(Failure::Contract);
    }
    let dialect = if provider == Provider::DeepSeek {
        ProviderDialect::Deepseek
    } else {
        ProviderDialect::Openrouter
    };
    let view = store
        .create(
            0,
            "Fixed experiment memory record".into(),
            if owned.is_some() {
                ConnectionKind::Local
            } else {
                ConnectionKind::Provider
            },
            dialect,
        )
        .map_err(|_| Failure::Contract)?;
    let mut draft = view
        .connections
        .into_iter()
        .next()
        .ok_or(Failure::Contract)?
        .connection;
    if let Some(endpoint) = owned {
        draft.api_address = endpoint.endpoint();
        draft.address_mode = AddressMode::FullChatEndpoint;
        draft.authentication = Authentication::None;
    }
    draft.model = if provider == Provider::DeepSeek {
        "deepseek-flash"
    } else {
        "google/gemma-4-31b-it:free"
    }
    .into();
    draft.max_output_tokens = Some(512);
    draft.timeout_ms = if owned.is_some() { 3000 } else { 30_000 };
    if provider == Provider::DeepSeek {
        draft.temperature = Some(0.0);
        draft.top_p = Some(0.9);
    }
    Ok(draft)
}
fn resolve(
    provider: Provider,
    owned: Option<&OwnedLoopback>,
    reader: &dyn config::credentials::CredentialReader,
) -> Result<TextConfiguration, Failure> {
    let store = config::ConnectionStore::open(Arc::new(Memory)).map_err(|_| Failure::Storage)?;
    let draft = draft(&store, provider, owned)?;
    let view = store
        .save(1, draft.clone(), config::KeyAction::Clear)
        .map_err(|_| Failure::Contract)?;
    store
        .activate(view.revision, Some(draft.id))
        .map_err(|_| Failure::Storage)?;
    store.resolve_active(reader).map_err(|_| Failure::NotReady)
}
pub(super) fn owned_processor(
    endpoint: &OwnedLoopback,
    slot: Slot,
    latch: AdverseLatch,
    tracing: TraceContext,
) -> Result<ExperimentProcessor, Failure> {
    if slot.provider == Provider::GeminiSealed {
        return Err(Failure::Contract);
    }
    let kind = if slot.provider == Provider::DeepSeek {
        ExperimentKind::DeepSeekFlash
    } else {
        ExperimentKind::OpenRouterGemma31bFreeGoogleAiStudio
    };
    let mut configuration = resolve(slot.provider, Some(endpoint), &NoHome)?;
    if slot.number == 1 {
        configuration.prompts.ask = "Reply with exactly OK and no other text.".into();
    }
    experiment::experiment_processor(
        configuration,
        ExperimentPolicy::OwnedLoopback(endpoint.clone(), kind, latch),
        tracing,
    )
    .map_err(|_| Failure::Contract)
}
/// Frozen backend-only public snapshots. No serialization/debug/key getter.
pub(super) struct LiveConfigurations {
    deepseek: TextConfiguration,
    openrouter: TextConfiguration,
    custom: config::ConnectionDraft,
}
impl LiveConfigurations {
    /// Caller must finish source/artifact/check/fresh-metadata/resource gates
    /// first. Only two exact named public keys can be requested, never Gemini.
    pub(super) fn resolve(
        reader: &dyn config::credentials::CredentialReader,
    ) -> Result<Self, Failure> {
        let deepseek = resolve(Provider::DeepSeek, None, reader)?;
        let openrouter = resolve(Provider::OpenRouter, None, reader)?;
        let store =
            config::ConnectionStore::open(Arc::new(Memory)).map_err(|_| Failure::Storage)?;
        let mut custom = draft(&store, Provider::DeepSeek, None)?;
        custom.kind = config::ConnectionKind::Custom;
        custom.address_mode = config::AddressMode::FullChatEndpoint;
        custom.api_address = "https://api.deepseek.com/chat/completions".into();
        let view = store
            .save(1, custom.clone(), config::KeyAction::Clear)
            .map_err(|_| Failure::Contract)?;
        if view.connections[0].app_key_present {
            return Err(Failure::Contract);
        }
        store
            .activate(view.revision, Some(custom.id.clone()))
            .map_err(|_| Failure::Storage)?;
        if !matches!(
            store.resolve_active(reader),
            Err(config::ConfigError("configuration-credentials-unavailable"))
        ) {
            return Err(Failure::Contract);
        }
        Ok(Self {
            deepseek,
            openrouter,
            custom,
        })
    }
    pub(super) fn processor(
        &self,
        slot: Slot,
        latch: AdverseLatch,
        tracing: TraceContext,
    ) -> Result<ExperimentProcessor, Failure> {
        if slot.provider == Provider::GeminiSealed {
            return Err(Failure::Contract);
        }
        if slot.number == 1 {
            return experiment::experiment_processor(
                custom_snapshot(self.deepseek.clone(), self.custom.clone())?,
                ExperimentPolicy::DeepSeekFlash(latch),
                tracing,
            )
            .map_err(|_| Failure::Contract);
        }
        let (configuration, policy) = if slot.provider == Provider::DeepSeek {
            (
                self.deepseek.clone(),
                ExperimentPolicy::DeepSeekFlash(latch),
            )
        } else {
            (
                self.openrouter.clone(),
                ExperimentPolicy::OpenRouterGemma31bFreeGoogleAiStudio(latch),
            )
        };
        experiment::experiment_processor(configuration, policy, tracing)
            .map_err(|_| Failure::Contract)
    }
    pub(super) fn processor_owned(
        &self,
        endpoint: &OwnedLoopback,
        slot: Slot,
        latch: AdverseLatch,
        tracing: TraceContext,
    ) -> Result<ExperimentProcessor, Failure> {
        let (mut configuration, kind) = match slot.provider {
            Provider::DeepSeek => (
                if slot.number == 1 {
                    custom_snapshot(self.deepseek.clone(), self.custom.clone())?
                } else {
                    self.deepseek.clone()
                },
                ExperimentKind::DeepSeekFlash,
            ),
            Provider::OpenRouter => (
                self.openrouter.clone(),
                ExperimentKind::OpenRouterGemma31bFreeGoogleAiStudio,
            ),
            Provider::GeminiSealed => return Err(Failure::Contract),
        };
        configuration.endpoint = endpoint.endpoint();
        experiment::experiment_processor(
            configuration,
            ExperimentPolicy::OwnedLoopback(endpoint.clone(), kind, latch),
            tracing,
        )
        .map_err(|_| Failure::Contract)
    }
}
fn custom_snapshot(
    mut named: TextConfiguration,
    custom: config::ConnectionDraft,
) -> Result<TextConfiguration, Failure> {
    use config::*;
    let endpoint = canonical_chat(&custom).map_err(|_| Failure::Contract)?;
    let resolution = capabilities::resolve(&custom).map_err(|_| Failure::Contract)?;
    let parameters = named.request_parameters.as_ref().ok_or(Failure::Contract)?;
    if custom.kind != ConnectionKind::Custom
        || custom.authentication != Authentication::Bearer
        || custom.dialect != ProviderDialect::Deepseek
        || custom.address_mode != AddressMode::FullChatEndpoint
        || endpoint.as_str() != "https://api.deepseek.com/chat/completions"
        || named.endpoint != endpoint.as_str()
        || named.model != custom.model
        || named.provider_profile != workflow::TextProviderProfile::DeepSeek
        || named.runtime_credential.is_none()
        || !named.api_key.is_empty()
        || custom.thinking_preference != ThinkingPreference::Off
        || parameters.requested_thinking
        || parameters.preference_effort != custom.effort
        || named.thinking_override != resolution.effective_thinking
        || named.effort != resolution.effective_effort
        || named.temperature != resolution.temperature_wire
        || named.top_p != resolution.top_p_wire
        || parameters.configured_temperature != custom.temperature.map(|v| v.to_string())
        || parameters.configured_top_p != custom.top_p.map(|v| v.to_string())
        || parameters.temperature_resolution != resolution.temperature_resolution
        || parameters.top_p_resolution != resolution.top_p_resolution
        || named.max_output_tokens != custom.max_output_tokens
        || named.timeout_ms != custom.timeout_ms
    {
        return Err(Failure::Contract);
    }
    named.endpoint = endpoint.into();
    named.connection_id = Some(custom.id);
    named.credential_source = Some("explicit_ephemeral_from_named".into());
    named.prompts.ask = "Reply with exactly OK and no other text.".into();
    Ok(named)
}
