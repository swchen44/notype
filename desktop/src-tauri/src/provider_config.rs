//! Bounded saved connections, safe views and atomic revision-aware mutations.
//! Stored credentials never share a type with a frontend return value.
use crate::workflow::{ScenarioPrompts, TranslationSettings};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::{Arc, Mutex};
#[path = "provider_capabilities.rs"]
pub mod capabilities;
#[path = "provider_credentials.rs"]
pub mod credentials;
#[path = "provider_config_files.rs"]
pub mod files;

/// Fixed configuration failures; no URL, path, provider message or credential.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConfigError(pub &'static str);
impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for ConfigError {}
/// Closed location/presentation policy; names never select transport behavior.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionKind {
    /// Explicit supported public provider.
    Provider,
    /// Independently configured compatible service.
    Custom,
    /// Already-running local compatible API, not guaranteed local inference.
    Local,
}
/// Explicit wire dialect, independent of kind, name and endpoint text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ProviderDialect {
    /// Generic OpenAI-compatible fields.
    Generic,
    /// Documented DeepSeek controls.
    Deepseek,
    /// OpenRouter reasoning object.
    Openrouter,
    /// Gemini compatibility controls.
    Gemini,
    /// Ollama OpenAI-compatible endpoints only.
    OllamaOpenai,
}
/// Whether to append the exact compatible chat suffix or use the exact URL.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum AddressMode {
    /// Preserve prefix and append chat/completions.
    BaseUrl,
    /// Exact validated generation URL.
    FullChatEndpoint,
}
/// Saved preference, independent of the effective request capability mapping.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ThinkingPreference {
    /// Default preference; mandatory minimum is shown separately.
    Off,
    /// Enable only under a resolved declared/documented capability.
    On,
}
/// Explicit authentication; no implicit public key for anonymous/local requests.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum Authentication {
    /// Anonymous, without home-file reads.
    None,
    /// Explicit bound App key or eligible public home fallback.
    Bearer,
}
/// Capability declaration, labelled user-declared rather than provider verified.
#[derive(Clone, Serialize, Deserialize, Type)]
pub struct CapabilityDeclaration {
    /// non_reasoning, optional or mandatory; unknown is represented by absence.
    pub kind: String,
    /// Required valid minimum for mandatory reasoning.
    pub minimum_effort: Option<String>,
    /// Explicit bounded supported effort vocabulary.
    pub supported_efforts: Vec<String>,
    /// Whether this declared service supports temperature.
    pub temperature_supported: bool,
    /// Whether this declared service supports top_p.
    pub top_p_supported: bool,
}
/// Editable connection fields. No current or resolved credential is returned.
#[derive(Clone, Serialize, Deserialize, Type)]
pub struct ConnectionDraft {
    /// Stable opaque backend-generated ID, never the display name.
    pub id: String,
    /// Trimmed text-only presentation name, at most128 scalars/512 bytes.
    pub display_name: String,
    /// Explicit location policy.
    pub kind: ConnectionKind,
    /// Explicit wire mapping.
    pub dialect: ProviderDialect,
    /// Base URL versus exact generation endpoint.
    pub address_mode: AddressMode,
    /// User address; validated without secret-bearing queries/userinfo/fragments.
    pub api_address: String,
    /// Explicit model-list endpoint when full chat URL is selected.
    pub models_endpoint: Option<String>,
    /// User-authoritative model ID; initially blank on every new record.
    pub model: String,
    /// Explicit auth policy.
    pub authentication: Authentication,
    /// Whole request deadline,1..120000 milliseconds.
    pub timeout_ms: u64,
    /// Raw user preference, not a claim about actual model computation.
    pub thinking_preference: ThinkingPreference,
    /// Remembered ON effort; OFF does not reuse it.
    pub effort: Option<String>,
    /// Advanced explicit declaration for otherwise unknown models.
    pub capability_declaration: Option<CapabilityDeclaration>,
    /// Nullable configured sampling; zero is preserved.
    pub temperature: Option<f64>,
    /// Nullable configured nucleus sampling, strictly greater than zero.
    pub top_p: Option<f64>,
    /// Nullable positive output bound.
    pub max_output_tokens: Option<u32>,
}
/// Explicit write-only credential action. No getter returns this value.
#[derive(Deserialize, Type)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum KeyAction {
    /// Keep only on the unchanged bound route/dialect.
    Keep,
    /// Remove this record's App key.
    Clear,
    /// Replace with the explicitly entered value.
    Replace {
        /// Transient user input; backend-only storage is plaintext, not a vault.
        value: String,
    },
}
/// Safe effective option projection, with configured values retained in draft.
#[derive(Clone, Serialize, Type)]
pub struct RequestResolutionView {
    /// Bounded supported ON effort choices, without duplicated UI capability rules.
    pub supported_efforts: Vec<String>,
    /// non_reasoning, optional, mandatory or unknown.
    pub capability: String,
    /// documented, provider_metadata, user_declared or unknown.
    pub provenance: String,
    /// Actual request switch; None means omitted/unknown.
    pub effective_thinking: Option<bool>,
    /// Actual wire effort; never a provider-reported value.
    pub effective_effort: Option<String>,
    /// non_reasoning, disabled, enabled, mandatory_minimum or unresolved.
    pub reason: String,
    /// Actual serialized temperature or omission.
    pub temperature_wire: Option<f64>,
    /// Actual serialized top_p or omission.
    pub top_p_wire: Option<f64>,
    /// omitted, sent, ignored_by_provider or unsupported.
    pub temperature_resolution: String,
    /// omitted, sent, ignored_by_provider or unsupported.
    pub top_p_resolution: String,
    /// Documented normalization only, never a measured provider result.
    pub documented_top_p: Option<f64>,
}
/// Frontend view: safe draft plus presence and derived configuration status.
#[derive(Serialize, Type)]
pub struct ConnectionView {
    /// Flattened safe editable fields; credentials are held by another private type.
    #[serde(flatten)]
    pub connection: ConnectionDraft,
    /// Presence only of this record's App-owned credential.
    pub app_key_present: bool,
    /// None means eligible home fallback has not been resolved by an operation.
    pub credential_present: Option<bool>,
    /// app, home_env, none or unresolved; no value/prefix/suffix.
    pub credential_source: String,
    /// Canonical validated generation preview, or absent when incomplete.
    pub effective_endpoint: Option<String>,
    /// Safe capability/option projection.
    pub resolution: Option<RequestResolutionView>,
    /// Fixed safe error code, never the rejected input or filesystem detail.
    pub configuration_error: Option<String>,
}
/// Complete safe acknowledged settings snapshot shared by all getter/mutations.
#[derive(Serialize, Type)]
pub struct ConnectionSettingsView {
    /// Fresh schema2; no import of the old single-record schema.
    pub schema_version: u32,
    /// Collection CAS revision, incremented only after durable commit.
    pub revision: u64,
    /// At most20 records; viewing never resolves inactive credentials.
    pub connections: Vec<ConnectionView>,
    /// Explicit active ID; removal never picks another record automatically.
    pub active_connection_id: Option<String>,
    /// Independently current trusted scene prompts.
    pub prompts: ScenarioPrompts,
    /// Independently current translation settings.
    pub translation: TranslationSettings,
}
#[derive(Clone, Serialize, Deserialize)]
struct BoundCredential {
    value: String,
    binding: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct StoredConnection {
    pub(crate) draft: ConnectionDraft,
    key: Option<BoundCredential>,
}
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct StoredSettings {
    schema_version: u32,
    pub(crate) revision: u64,
    pub(crate) connections: Vec<StoredConnection>,
    pub(crate) active_connection_id: Option<String>,
    pub(crate) prompts: ScenarioPrompts,
    pub(crate) translation: TranslationSettings,
}
impl Default for StoredSettings {
    fn default() -> Self {
        Self {
            schema_version: 2,
            revision: 0,
            connections: vec![],
            active_connection_id: None,
            prompts: ScenarioPrompts::default(),
            translation: TranslationSettings::default(),
        }
    }
}
/// Storage transaction boundary. Bytes may contain App credentials and must never
/// be logged, formatted or returned to the frontend. Commit failure preserves disk.
pub trait ConfigurationStorage: Send + Sync {
    /// Read the dedicated bounded schema, without importing any older settings file.
    fn load(&self) -> Result<Option<Vec<u8>>, ConfigError>;
    /// Atomically replace the complete dedicated file; failure preserves old bytes.
    fn commit(&self, bytes: &[u8]) -> Result<(), ConfigError>;
}
/// Flat bounded collection service. Candidate writes precede memory publication;
/// credential/network resolution uses owned snapshots outside this writer lock.
pub struct ConnectionStore {
    state: Mutex<StoredSettings>,
    storage: Arc<dyn ConfigurationStorage>,
}
impl ConnectionStore {
    /// Capture the selected processing configuration for recording. Preparation
    /// failures remain typed and surface only after the original is persisted.
    pub fn processing_snapshot(
        &self,
        reader: &dyn credentials::CredentialReader,
    ) -> crate::workflow::TextConfiguration {
        self.resolve_active(reader)
            .unwrap_or_else(failed_preparation)
    }
    /// Resolve only the active owned snapshot, outside the settings writer lock.
    /// No active connection yields disabled configuration and zero key-file reads.
    pub fn resolve_active(
        &self,
        reader: &dyn credentials::CredentialReader,
    ) -> Result<crate::workflow::TextConfiguration, ConfigError> {
        let state = self.snapshot()?;
        let Some(id) = state.active_connection_id.as_deref() else {
            return Ok(crate::workflow::TextConfiguration {
                prompts: state.prompts,
                translation: state.translation,
                ..Default::default()
            });
        };
        let record = state
            .connections
            .iter()
            .find(|record| record.draft.id == id)
            .ok_or(ConfigError("configuration-store-invalid"))?;
        resolve_record(
            &state,
            record,
            record.draft.clone(),
            KeyAction::Keep,
            reader,
            true,
        )
    }
    /// Resolve an explicit existing editor snapshot for GET only. This does not
    /// persist its fields/key, activate, require a model or create generation rows.
    pub fn resolve_models(
        &self,
        revision: u64,
        draft: ConnectionDraft,
        key_action: KeyAction,
        reader: &dyn credentials::CredentialReader,
    ) -> Result<crate::workflow::TextConfiguration, ConfigError> {
        let state = self.snapshot()?;
        if state.revision != revision {
            return Err(ConfigError("configuration-conflict"));
        }
        validate_draft(&draft)?;
        canonical_models(&draft)?;
        let record = state
            .connections
            .iter()
            .find(|record| record.draft.id == draft.id)
            .ok_or(ConfigError("configuration-connection-missing"))?;
        resolve_record(&state, record, draft, key_action, reader, false)
    }
    /// Load/validate the fresh schema, failing closed on malformed/repeated/dangling IDs.
    pub fn open(storage: Arc<dyn ConfigurationStorage>) -> Result<Self, ConfigError> {
        let state = match storage.load()? {
            None => StoredSettings::default(),
            Some(bytes) => {
                if bytes.len() > 4_194_304 {
                    return Err(ConfigError("configuration-store-invalid"));
                }
                serde_json::from_slice(&bytes)
                    .map_err(|_| ConfigError("configuration-store-invalid"))?
            }
        };
        validate_store(&state)?;
        Ok(Self {
            state: Mutex::new(state),
            storage,
        })
    }
    /// Safe complete view. This performs no credential-file or network operations.
    pub fn view(&self) -> Result<ConnectionSettingsView, ConfigError> {
        let state = self.snapshot()?;
        Ok(view(&state))
    }
    pub(crate) fn snapshot(&self) -> Result<StoredSettings, ConfigError> {
        self.state
            .lock()
            .map(|state| state.clone())
            .map_err(|_| ConfigError("configuration-storage-failed"))
    }
    fn update(
        &self,
        revision: u64,
        operation: impl FnOnce(&mut StoredSettings) -> Result<(), ConfigError>,
    ) -> Result<ConnectionSettingsView, ConfigError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| ConfigError("configuration-storage-failed"))?;
        if state.revision != revision {
            return Err(ConfigError("configuration-conflict"));
        }
        let mut next = state.clone();
        operation(&mut next)?;
        next.revision = next
            .revision
            .checked_add(1)
            .filter(|revision| *revision <= 9_007_199_254_740_991)
            .ok_or(ConfigError("configuration-revision-exhausted"))?;
        validate_store(&next)?;
        let bytes =
            serde_json::to_vec(&next).map_err(|_| ConfigError("configuration-storage-failed"))?;
        if bytes.len() > 4_194_304 {
            return Err(ConfigError("configuration-store-limit"));
        }
        self.storage.commit(&bytes)?;
        *state = next;
        Ok(view(&state))
    }
    /// Add blank-model/OFF/no-key record without activating, querying or generating.
    pub fn create(
        &self,
        revision: u64,
        name: String,
        kind: ConnectionKind,
        dialect: ProviderDialect,
    ) -> Result<ConnectionSettingsView, ConfigError> {
        self.update(revision, |state| {
            if state.connections.len() >= 20 {
                return Err(ConfigError("configuration-connection-limit"));
            }
            let address = if kind == ConnectionKind::Provider {
                preset(dialect).unwrap_or("")
            } else {
                ""
            };
            let draft = ConnectionDraft {
                id: new_id(),
                display_name: name.trim().into(),
                kind,
                dialect,
                address_mode: AddressMode::BaseUrl,
                api_address: address.into(),
                models_endpoint: None,
                model: String::new(),
                authentication: if kind == ConnectionKind::Local {
                    Authentication::None
                } else {
                    Authentication::Bearer
                },
                timeout_ms: 30_000,
                thinking_preference: ThinkingPreference::Off,
                effort: None,
                capability_declaration: None,
                temperature: None,
                top_p: None,
                max_output_tokens: None,
            };
            validate_draft(&draft)?;
            state
                .connections
                .push(StoredConnection { draft, key: None });
            Ok(())
        })
    }
    /// Replace one record under CAS, preserving current global settings and others.
    pub fn save(
        &self,
        revision: u64,
        mut draft: ConnectionDraft,
        key_action: KeyAction,
    ) -> Result<ConnectionSettingsView, ConfigError> {
        draft.display_name = draft.display_name.trim().into();
        validate_draft(&draft)?;
        self.update(revision, |state| {
            let record = state
                .connections
                .iter_mut()
                .find(|record| record.draft.id == draft.id)
                .ok_or(ConfigError("configuration-connection-missing"))?;
            let binding = binding(&draft)?;
            record.key = match key_action {
                KeyAction::Clear => None,
                KeyAction::Keep => {
                    if record
                        .key
                        .as_ref()
                        .is_some_and(|key| key.binding != binding)
                    {
                        return Err(ConfigError("configuration-key-binding-required"));
                    }
                    record.key.clone()
                }
                KeyAction::Replace { value } => {
                    let value = value.trim().to_owned();
                    if value.is_empty() || value.len() > 8192 || value.chars().any(char::is_control)
                    {
                        return Err(ConfigError("configuration-key-invalid"));
                    }
                    Some(BoundCredential { value, binding })
                }
            };
            record.draft = draft;
            Ok(())
        })
    }
    /// Persist explicit active selection; None disables text generation readiness.
    pub fn activate(
        &self,
        revision: u64,
        id: Option<String>,
    ) -> Result<ConnectionSettingsView, ConfigError> {
        self.update(revision, |state| {
            if id.as_ref().is_some_and(|id| {
                !state
                    .connections
                    .iter()
                    .any(|record| record.draft.id == *id)
            }) {
                return Err(ConfigError("configuration-connection-missing"));
            }
            state.active_connection_id = id;
            Ok(())
        })
    }
    /// Remove one ID; deleting active deliberately leaves no replacement selected.
    pub fn remove(&self, revision: u64, id: &str) -> Result<ConnectionSettingsView, ConfigError> {
        self.update(revision, |state| {
            let index = state
                .connections
                .iter()
                .position(|record| record.draft.id == id)
                .ok_or(ConfigError("configuration-connection-missing"))?;
            state.connections.remove(index);
            if state.active_connection_id.as_deref() == Some(id) {
                state.active_connection_id = None;
            }
            Ok(())
        })
    }
    /// Update/reset exactly one prompt while preserving connections/active/other scenes.
    pub fn prompt(
        &self,
        revision: u64,
        scenario: notype_core::Scenario,
        prompt: Option<String>,
    ) -> Result<ConnectionSettingsView, ConfigError> {
        self.update(revision, |state| {
            let defaults = ScenarioPrompts::default();
            let prompt = prompt.unwrap_or(match scenario {
                notype_core::Scenario::Dictation => defaults.dictation,
                notype_core::Scenario::Translation => defaults.translation,
                notype_core::Scenario::Ask => defaults.ask,
            });
            if prompt.trim().is_empty() || prompt.len() > 32768 || prompt.contains('\0') {
                return Err(ConfigError("prompt-invalid"));
            }
            match scenario {
                notype_core::Scenario::Dictation => state.prompts.dictation = prompt,
                notype_core::Scenario::Translation => state.prompts.translation = prompt,
                notype_core::Scenario::Ask => state.prompts.ask = prompt,
            };
            Ok(())
        })
    }
    /// Independently replace validated translation settings under the same CAS.
    pub fn translation(
        &self,
        revision: u64,
        translation: TranslationSettings,
    ) -> Result<ConnectionSettingsView, ConfigError> {
        validate_translation(&translation)?;
        self.update(revision, |state| {
            state.translation = translation;
            Ok(())
        })
    }
}

fn resolve_record(
    state: &StoredSettings,
    record: &StoredConnection,
    draft: ConnectionDraft,
    key_action: KeyAction,
    reader: &dyn credentials::CredentialReader,
    generation: bool,
) -> Result<crate::workflow::TextConfiguration, ConfigError> {
    use crate::workflow::{
        TextAuthentication, TextConfiguration, TextProviderProfile, TextTransport,
    };
    let endpoint = canonical_chat(&draft)?;
    let binding = binding(&draft)?;
    let resolution = capabilities::resolve(&draft)?;
    if generation && draft.model.trim().is_empty() {
        return Err(ConfigError("configuration-model-required"));
    }
    if generation && resolution.reason == "unresolved" {
        return Err(ConfigError("configuration-capability-required"));
    }
    let app_key = match key_action {
        KeyAction::Clear => None,
        KeyAction::Keep => {
            if record
                .key
                .as_ref()
                .is_some_and(|key| key.binding != binding)
            {
                return Err(ConfigError("configuration-key-binding-required"));
            }
            record.key.as_ref().map(|key| key.value.clone())
        }
        KeyAction::Replace { value } => {
            let value = value.trim().to_owned();
            if value.is_empty() || value.len() > 8192 || value.chars().any(char::is_control) {
                return Err(ConfigError("configuration-key-invalid"));
            }
            Some(value)
        }
    };
    let (key, source) = if draft.authentication == Authentication::None {
        (None, "none")
    } else if let Some(key) = app_key {
        (Some(key), "app")
    } else if let Some(name) = credentials::public_key(&draft) {
        (
            Some(
                reader
                    .read(name)?
                    .filter(|value| !value.trim().is_empty())
                    .ok_or(ConfigError("configuration-credentials-unavailable"))?,
            ),
            "home_env",
        )
    } else {
        return Err(ConfigError("configuration-credentials-unavailable"));
    };
    let parameters = notype_core::observations::RequestParameters {
        requested_thinking: draft.thinking_preference == ThinkingPreference::On,
        preference_effort: draft.effort.clone(),
        effective_thinking: resolution.effective_thinking,
        effective_effort: resolution.effective_effort.clone(),
        capability_provenance: resolution.provenance.clone(),
        resolution_reason: resolution.reason.clone(),
        configured_temperature: draft.temperature.map(|value| value.to_string()),
        wire_temperature: resolution.temperature_wire.map(|value| value.to_string()),
        temperature_resolution: resolution.temperature_resolution.clone(),
        configured_top_p: draft.top_p.map(|value| value.to_string()),
        wire_top_p: resolution.top_p_wire.map(|value| value.to_string()),
        top_p_resolution: resolution.top_p_resolution.clone(),
        documented_top_p: resolution.documented_top_p.map(|value| value.to_string()),
        max_output_tokens: draft.max_output_tokens,
    };
    let mut configuration = TextConfiguration {
        connection_id: Some(draft.id.clone()),
        transport: TextTransport::OpenAiChat,
        endpoint: endpoint.into(),
        model: draft.model,
        authentication: if draft.authentication == Authentication::None {
            TextAuthentication::None
        } else {
            TextAuthentication::Bearer
        },
        api_key: String::new(),
        timeout_ms: draft.timeout_ms,
        provider_profile: match draft.dialect {
            ProviderDialect::Generic => TextProviderProfile::Generic,
            ProviderDialect::Deepseek => TextProviderProfile::DeepSeek,
            ProviderDialect::Openrouter => TextProviderProfile::OpenRouter,
            ProviderDialect::Gemini => TextProviderProfile::Gemini,
            ProviderDialect::OllamaOpenai => TextProviderProfile::OllamaOpenAi,
        },
        effort: resolution.effective_effort,
        thinking_override: resolution.effective_thinking,
        temperature: resolution.temperature_wire,
        top_p: resolution.top_p_wire,
        max_output_tokens: draft.max_output_tokens,
        request_parameters: Some(parameters),
        credential_source: Some(source.into()),
        bypass_proxy: draft.kind == ConnectionKind::Local,
        prompts: state.prompts.clone(),
        translation: state.translation.clone(),
        ..Default::default()
    };
    configuration.runtime_credential = key.map(crate::workflow::RuntimeCredential::new);
    Ok(configuration)
}

/// Convert a failed selected snapshot into a processor result that is surfaced
/// only after SessionService saves the original. No filesystem error is exposed.
pub fn failed_preparation(error: ConfigError) -> crate::workflow::TextConfiguration {
    use notype_core::PreparationFailure as P;
    let failure = match error.0 {
        "configuration-credentials-unavailable"
        | "configuration-home-file-invalid"
        | "configuration-home-file-unavailable"
        | "configuration-home-unavailable" => P::Credentials,
        "configuration-capability-required" | "configuration-capability-invalid" => P::Capability,
        "configuration-address-invalid"
        | "configuration-address-required"
        | "configuration-address-mode-invalid"
        | "configuration-key-binding-required"
        | "configuration-local-loopback-required" => P::Route,
        "configuration-store-invalid"
        | "configuration-storage-failed"
        | "configuration-file-unavailable" => P::Storage,
        _ => P::Options,
    };
    crate::workflow::TextConfiguration {
        preparation_failure: Some(failure),
        ..Default::default()
    }
}
fn new_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let tick = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!(
        "c-{tick:032x}-{:016x}",
        NEXT.fetch_add(1, Ordering::Relaxed)
    )
}
fn validate_store(state: &StoredSettings) -> Result<(), ConfigError> {
    if state.schema_version != 2
        || state.connections.len() > 20
        || state.revision > 9_007_199_254_740_991
    {
        return Err(ConfigError("configuration-store-invalid"));
    }
    let mut ids = std::collections::HashSet::new();
    for record in &state.connections {
        validate_draft(&record.draft)?;
        if !ids.insert(&record.draft.id) {
            return Err(ConfigError("configuration-store-invalid"));
        }
        if let Some(key) = &record.key {
            if key.value.is_empty()
                || key.value.len() > 8192
                || key.value.chars().any(char::is_control)
                || key.binding != binding(&record.draft)?
            {
                return Err(ConfigError("configuration-store-invalid"));
            }
        }
    }
    if state
        .active_connection_id
        .as_ref()
        .is_some_and(|id| !ids.contains(id))
    {
        return Err(ConfigError("configuration-store-invalid"));
    }
    for prompt in [
        &state.prompts.dictation,
        &state.prompts.translation,
        &state.prompts.ask,
    ] {
        if prompt.trim().is_empty() || prompt.len() > 32768 || prompt.contains('\0') {
            return Err(ConfigError("prompt-invalid"));
        }
    }
    validate_translation(&state.translation)
}
fn validate_translation(value: &TranslationSettings) -> Result<(), ConfigError> {
    if value.target_language.trim().is_empty()
        || value.target_language.len() > 100
        || value.domain.len() > 2000
        || value.tone.len() > 2000
        || value.glossary.len() > 1000
        || value.glossary.iter().any(|term| {
            term.source.trim().is_empty()
                || term.target.trim().is_empty()
                || term.target_language.trim().is_empty()
                || term.source.len() > 1000
                || term.target.len() > 1000
                || term.target_language.len() > 100
        })
    {
        return Err(ConfigError("translation-settings-invalid"));
    }
    Ok(())
}
fn validate_draft(value: &ConnectionDraft) -> Result<(), ConfigError> {
    if value.id.is_empty()
        || value.id.len() > 64
        || !value
            .id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        || value.display_name.trim().is_empty()
        || value.display_name.len() > 512
        || value.display_name.chars().count() > 128
        || value.display_name.chars().any(char::is_control)
    {
        return Err(ConfigError("configuration-name-invalid"));
    }
    if value.model.len() > 256
        || value.model.chars().any(char::is_control)
        || value.api_address.len() > 2048
        || value.api_address.chars().any(char::is_control)
        || value
            .models_endpoint
            .as_ref()
            .is_some_and(|s| s.len() > 2048 || s.chars().any(char::is_control))
        || !(1..=120_000).contains(&value.timeout_ms)
    {
        return Err(ConfigError("configuration-invalid-options"));
    }
    if !value.api_address.is_empty() {
        canonical_chat(value)?;
    }
    capabilities::validate(value)?;
    Ok(())
}
/// Documented public preset address only; model IDs and keys are never defaulted.
pub fn preset(dialect: ProviderDialect) -> Option<&'static str> {
    match dialect {
        ProviderDialect::Deepseek => Some("https://api.deepseek.com"),
        ProviderDialect::Openrouter => Some("https://openrouter.ai/api/v1"),
        ProviderDialect::Gemini => Some("https://generativelanguage.googleapis.com/v1beta/openai/"),
        _ => None,
    }
}
/// Validate/canonicalize an exact route without inferring dialect from URL text.
pub fn canonical_chat(draft: &ConnectionDraft) -> Result<reqwest::Url, ConfigError> {
    let mut url = valid_url(&draft.api_address)?;
    if draft.kind == ConnectionKind::Local && !loopback(&url) {
        return Err(ConfigError("configuration-local-loopback-required"));
    }
    if draft.address_mode == AddressMode::BaseUrl {
        let path = url.path().trim_end_matches('/');
        if path.ends_with("/chat/completions") {
            return Err(ConfigError("configuration-address-mode-invalid"));
        }
        let path = format!("{path}/chat/completions");
        url.set_path(&path);
    }
    Ok(url)
}
/// Model listing requires an explicit exact endpoint for FullChatEndpoint.
/// It never synthesizes a route by stripping the generation suffix.
pub fn canonical_models(draft: &ConnectionDraft) -> Result<reqwest::Url, ConfigError> {
    let chat = canonical_chat(draft)?;
    let (list, prefix) = match draft.address_mode {
        AddressMode::BaseUrl => {
            let mut base = valid_url(&draft.api_address)?;
            let prefix = base.path().trim_end_matches('/').to_owned();
            base.set_path(&format!("{prefix}/models"));
            (base, prefix)
        }
        AddressMode::FullChatEndpoint => {
            let endpoint = draft
                .models_endpoint
                .as_deref()
                .ok_or(ConfigError("configuration-model-list-unavailable"))?;
            let list = valid_url(endpoint)?;
            let prefix = chat
                .path()
                .strip_suffix("/chat/completions")
                .unwrap_or_else(|| {
                    chat.path()
                        .rsplit_once('/')
                        .map_or("", |(prefix, _)| prefix)
                })
                .to_owned();
            (list, prefix)
        }
    };
    if chat.scheme() != list.scheme()
        || chat.host_str() != list.host_str()
        || chat.port_or_known_default() != list.port_or_known_default()
        || !(list.path() == prefix || list.path().starts_with(&format!("{prefix}/")))
    {
        return Err(ConfigError("configuration-model-list-binding-invalid"));
    }
    Ok(list)
}
fn valid_url(input: &str) -> Result<reqwest::Url, ConfigError> {
    if input.trim().is_empty() {
        return Err(ConfigError("configuration-address-required"));
    }
    let url = reqwest::Url::parse(input.trim())
        .map_err(|_| ConfigError("configuration-address-invalid"))?;
    let lower = url.path().to_ascii_lowercase();
    if url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || url.query().is_some()
        || lower.contains("%2f")
        || lower.contains("%5c")
        || lower.contains("%2e")
        || !(url.scheme() == "https" || url.scheme() == "http" && loopback(&url))
    {
        return Err(ConfigError("configuration-address-invalid"));
    }
    Ok(url)
}
pub(crate) fn loopback(url: &reqwest::Url) -> bool {
    url.host_str().is_some_and(|host| {
        host == "localhost"
            || host
                .trim_matches(['[', ']'])
                .parse::<std::net::IpAddr>()
                .is_ok_and(|ip| ip.is_loopback())
    })
}
fn binding(draft: &ConnectionDraft) -> Result<String, ConfigError> {
    Ok(format!("{:?}|{}", draft.dialect, canonical_chat(draft)?))
}
fn view(state: &StoredSettings) -> ConnectionSettingsView {
    ConnectionSettingsView {
        schema_version: 2,
        revision: state.revision,
        active_connection_id: state.active_connection_id.clone(),
        prompts: state.prompts.clone(),
        translation: state.translation.clone(),
        connections: state
            .connections
            .iter()
            .map(|record| {
                let endpoint = canonical_chat(&record.draft);
                let resolution = capabilities::resolve(&record.draft);
                let error = endpoint
                    .as_ref()
                    .err()
                    .copied()
                    .or_else(|| resolution.as_ref().err().copied())
                    .or_else(|| {
                        record
                            .draft
                            .model
                            .trim()
                            .is_empty()
                            .then_some(ConfigError("configuration-model-required"))
                    })
                    .or_else(|| {
                        resolution
                            .as_ref()
                            .ok()
                            .is_some_and(|value| value.reason == "unresolved")
                            .then_some(ConfigError("configuration-capability-required"))
                    });
                let has_key = record.key.is_some();
                let effective_app_key =
                    has_key && record.draft.authentication == Authentication::Bearer;
                let eligible = credentials::public_key(&record.draft).is_some();
                ConnectionView {
                    connection: record.draft.clone(),
                    app_key_present: has_key,
                    credential_present: if effective_app_key {
                        Some(true)
                    } else if record.draft.authentication == Authentication::None || !eligible {
                        Some(false)
                    } else {
                        None
                    },
                    credential_source: if effective_app_key {
                        "app"
                    } else if record.draft.authentication == Authentication::None {
                        "none"
                    } else if eligible {
                        "unresolved"
                    } else {
                        "none"
                    }
                    .into(),
                    effective_endpoint: endpoint.ok().map(|url| url.to_string()),
                    resolution: resolution.ok(),
                    configuration_error: error.map(|error| error.0.into()),
                }
            })
            .collect(),
    }
}
