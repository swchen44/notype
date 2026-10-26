//! Explicit bounded reference-model GET. Never saves/activates or sends chat.
use crate::{
    diagnostics::{self, api as trace},
    provider_config::{self as config, ConfigError, ConnectionDraft, ConnectionStore, KeyAction},
    workflow::{RuntimeCredential, TextConfiguration, TextProviderProfile},
};
use notype_core::{gateway_error::GatewayError, CancellationToken};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::{
    collections::BTreeSet,
    hash::{Hash, Hasher},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
/// Explicit editor request; replacement key is write-only transient input.
#[derive(Deserialize, Type)]
pub struct ModelListRequest {
    /// Opaque caller ID for cancelling this one editor request, max64 ASCII.
    pub request_id: String,
    /// Actual acknowledged collection revision for stale-response fencing.
    pub expected_revision: u64,
    /// Current edited form, not automatically saved or activated.
    pub connection: ConnectionDraft,
    /// Explicit selected-record credential policy.
    pub key_action: KeyAction,
}
/// Safe reference identifier; no provider description/metadata is projected.
#[derive(Serialize, Type)]
pub struct ModelRow {
    /// Bounded control-free ID for explicit user Copy/manual paste.
    pub id: String,
}
/// Bounded result envelope; all errors are fixed codes, never upstream messages.
#[derive(Serialize, Type)]
pub struct ModelListResult {
    /// Original request identity for editor stale-response suppression.
    pub request_id: String,
    /// Collection revision accepted at request start.
    pub configuration_revision: u64,
    /// Existing edited connection ID, never inferred from a name.
    pub connection_id: String,
    /// ok, unsupported, cancelled, error, throttled or stale.
    pub status: String,
    /// Reference IDs only, never a selected model or saved configuration.
    pub models: Vec<ModelRow>,
    /// Fixed safe error code.
    pub error_code: Option<String>,
    /// Bounded numeric advice; no automatic retry is scheduled.
    pub retry_after_seconds: Option<u64>,
    /// Actual selected credential provenance, never its value.
    pub credential_source: String,
    /// No sensitive response cache is used in this implementation.
    pub cache_hit: bool,
}
struct Flight {
    request: String,
    connection: String,
    cancellation: CancellationToken,
}
struct Cooldown {
    connection: String,
    binding: String,
    credential: u64,
    revision: u64,
    ready: Instant,
}
/// App-owned bounded admission/cancellation state. Cooldowns are conservative;
/// metadata GETs never create generation/billing history observations.
pub struct ModelDiscovery {
    tracing: Option<trace::TraceContext>,
    clock: Arc<dyn crate::capture_startup::CaptureClock>,
    flights: Mutex<Vec<Flight>>,
    cooldowns: Mutex<Vec<Cooldown>>,
}
impl Default for ModelDiscovery {
    fn default() -> Self {
        Self::new()
    }
}
impl ModelDiscovery {
    /// Empty service; construction performs no key lookup, probe or request.
    pub fn new() -> Self {
        Self::with_clock(Arc::new(crate::capture_startup::MonotonicClock))
    }
    /// Compose the monotonic admission clock; transport deadlines remain actual.
    pub fn with_clock(clock: Arc<dyn crate::capture_startup::CaptureClock>) -> Self {
        Self::with_diagnostics(clock, None)
    }
    /// Compose the same public diagnostics ports for an App-owned service.
    /// None uses the process service; observations never authorize GET admission.
    pub fn with_diagnostics(
        clock: Arc<dyn crate::capture_startup::CaptureClock>,
        tracing: Option<trace::TraceContext>,
    ) -> Self {
        Self {
            tracing,
            clock,
            flights: Mutex::new(vec![]),
            cooldowns: Mutex::new(vec![]),
        }
    }
    /// Cancel only an already-running explicit request. No lookup/network follows.
    pub fn cancel(&self, request: &str) {
        if let Ok(flights) = self.flights.lock() {
            if let Some(flight) = flights.iter().find(|flight| flight.request == request) {
                flight.cancellation.cancel();
            }
        }
    }
    /// Perform at most one explicit GET for this route/credential revision per60s.
    /// All settings/credential resolution completes outside history/collector locks.
    pub async fn query(
        &self,
        store: Arc<ConnectionStore>,
        request: ModelListRequest,
        reader: &dyn config::credentials::CredentialReader,
    ) -> Result<ModelListResult, ConfigError> {
        if request.request_id.is_empty()
            || request.request_id.len() > 64
            || !request
                .request_id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        {
            return Err(ConfigError("model-list-request-invalid"));
        }
        let mut result = ModelListResult {
            request_id: request.request_id.clone(),
            configuration_revision: request.expected_revision,
            connection_id: request.connection.id.clone(),
            status: "error".into(),
            models: vec![],
            error_code: None,
            retry_after_seconds: None,
            credential_source: "unresolved".into(),
            cache_hit: false,
        };
        let cancellation = CancellationToken::new();
        {
            let mut flights = self
                .flights
                .lock()
                .map_err(|_| ConfigError("model-list-unavailable"))?;
            if flights.len() >= 20
                || flights.iter().any(|flight| {
                    flight.connection == request.connection.id
                        || flight.request == request.request_id
                })
            {
                result.status = "throttled".into();
                result.error_code = Some("model-list-in-flight".into());
                return Ok(result);
            }
            flights.push(Flight {
                request: request.request_id.clone(),
                connection: request.connection.id.clone(),
                cancellation: cancellation.clone(),
            });
        }
        let _flight = FlightGuard {
            service: self,
            request: request.request_id.clone(),
        };
        let endpoint = config::canonical_models(&request.connection)?;
        let configuration = store.resolve_models(
            request.expected_revision,
            request.connection.clone(),
            request.key_action,
            reader,
        )?;
        result.credential_source = configuration
            .credential_source
            .clone()
            .unwrap_or_else(|| "none".into());
        let authorization = configuration
            .authorization_header()
            .map_err(|_| ConfigError("configuration-credentials-unavailable"))?;
        // This private cooldown fingerprint is never a diagnostic identity, export,
        // cache value or secret getter. It prevents credential-crossing result reuse.
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        authorization
            .as_ref()
            .map(|value| value.as_bytes())
            .hash(&mut hasher);
        let credential = hasher.finish();
        let binding = endpoint.as_str().to_owned();
        {
            let now = self.clock.now();
            let mut cooldowns = self
                .cooldowns
                .lock()
                .map_err(|_| ConfigError("model-list-unavailable"))?;
            cooldowns.retain(|entry| entry.ready > now);
            if let Some(entry) = cooldowns.iter().find(|entry| {
                entry.connection == request.connection.id
                    && entry.binding == binding
                    && entry.credential == credential
                    && entry.revision == request.expected_revision
            }) {
                result.status = "throttled".into();
                result.error_code = Some("model-list-cooldown".into());
                result.retry_after_seconds = Some(
                    entry
                        .ready
                        .saturating_duration_since(now)
                        .as_secs()
                        .saturating_add(1)
                        .min(86_400),
                );
                return Ok(result);
            }
            if cooldowns.len() >= 128 {
                result.status = "throttled".into();
                result.error_code = Some("model-list-cooldown".into());
                return Ok(result);
            }
            cooldowns.push(Cooldown {
                connection: request.connection.id.clone(),
                binding: binding.clone(),
                credential,
                revision: request.expected_revision,
                ready: now + Duration::from_secs(60),
            });
        }
        let scope = if self.tracing.is_none() {
            diagnostics::service::global().api_connection_scope(None, Some(&request.connection.id))
        } else {
            None
        };
        let options = trace_options(&configuration, &request.connection, &endpoint);
        let mut trace = match &self.tracing {
            Some(context) => context.begin(options, None, Arc::new(Cancel(cancellation.clone()))),
            None => trace::CallTrace::global(
                scope,
                options,
                None,
                Arc::new(Cancel(cancellation.clone())),
            ),
        };
        let mut builder = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .connect_timeout(Duration::from_secs(5))
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never());
        if configuration.bypass_proxy {
            builder = builder.no_proxy();
        }
        let client = builder.build();
        let client = match client {
            Ok(client) => client,
            Err(_) => {
                trace.finish(
                    trace::ApiResult::Failed,
                    trace::ApiErrorCategory::Configuration,
                );
                return Err(ConfigError("model-list-unavailable"));
            }
        };
        let mut get = client.get(endpoint);
        if let Some(header) = authorization {
            get = get.header(reqwest::header::AUTHORIZATION, header);
        }
        let operation = async {
            if let Some(event) = trace.metadata() {
                event.possibly_sent = true;
            }
            trace.observe(trace::ApiSpan::SendStarted, trace::ApiResult::Started);
            let mut response = get
                .send()
                .await
                .map_err(|error| transport_error(&mut trace, error))?;
            let status = response.status().as_u16();
            if let Some(event) = trace.metadata() {
                event.http_status = Some(status);
            }
            trace.observe(trace::ApiSpan::HeadersReceived, trace::ApiResult::Success);
            if let Some(seconds) = response
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<u64>().ok())
                .filter(|seconds| *seconds <= 86_400)
            {
                result.retry_after_seconds = Some(seconds);
                if let Ok(mut cooldowns) = self.cooldowns.lock() {
                    if let Some(entry) = cooldowns.iter_mut().find(|entry| {
                        entry.connection == request.connection.id
                            && entry.binding == binding
                            && entry.credential == credential
                            && entry.revision == request.expected_revision
                    }) {
                        entry.ready = self.clock.now() + Duration::from_secs(seconds.max(60));
                    }
                }
            }
            let status_error = GatewayError::from_status(status);
            let mut bytes = Vec::new();
            while let Some(chunk) = match response.chunk().await {
                Ok(chunk) => chunk,
                Err(error) => {
                    trace.body(bytes.len() as u64, false, false);
                    return Err(status_error.map_or_else(
                        || transport_error(&mut trace, error),
                        |error| ConfigError(error.history_code(false)),
                    ));
                }
            } {
                if bytes.len().saturating_add(chunk.len()) > 2_097_152 {
                    trace.body(bytes.len().saturating_add(chunk.len()) as u64, false, true);
                    return Err(ConfigError(
                        status_error.map_or("model-list-response-limit", |error| {
                            error.history_code(false)
                        }),
                    ));
                }
                bytes.extend_from_slice(&chunk);
                if let Some(event) = trace.metadata() {
                    event.body_bytes = bytes.len() as u64;
                }
            }
            trace.body(bytes.len() as u64, true, false);
            if matches!(status, 404 | 405 | 501) {
                return Err(ConfigError("model-list-unsupported"));
            }
            let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|_| {
                ConfigError(
                    status_error.map_or("llm-invalid-response", |error| error.history_code(false)),
                )
            })?;
            if let Some(error) = GatewayError::classify(status, Some(&value)) {
                return Err(ConfigError(error.history_code(false)));
            }
            let models = project_models(&value, configuration.runtime_credential.as_ref())?;
            if let Some(event) = trace.metadata() {
                event.model_list_count = Some(models.len() as u32);
            }
            Ok(models)
        };
        let outcome = tokio::select! {biased;_=cancellation.cancelled()=>Err(ConfigError("model-list-cancelled")),value=tokio::time::timeout(Duration::from_secs(10),operation)=>value.unwrap_or(Err(ConfigError("model-list-timeout")))};
        let (trace_result, category) = match &outcome {
            Ok(_) => (trace::ApiResult::Success, trace::ApiErrorCategory::None),
            Err(error) => model_failure(error.0),
        };
        if let Some(event) = trace.metadata() {
            if category == trace::ApiErrorCategory::Timeout
                && event.timeout_source == trace::TimeoutSource::None
            {
                event.timeout_source = if outcome
                    .as_ref()
                    .err()
                    .is_some_and(|error| error.0 == "provider-timeout")
                    || event
                        .http_status
                        .is_some_and(|status| matches!(status, 408 | 504))
                {
                    trace::TimeoutSource::ProviderStatus
                } else {
                    trace::TimeoutSource::LocalDeadline
                };
            }
        }
        trace.finish(trace_result, category);
        if store.view()?.revision != request.expected_revision {
            result.status = "stale".into();
            result.error_code = Some("model-list-stale".into());
            return Ok(result);
        }
        match outcome {
            Ok(models) => {
                result.models = models;
                result.status = "ok".into();
            }
            Err(error) => {
                result.status = match error.0 {
                    "model-list-cancelled" => "cancelled",
                    "model-list-unsupported" | "model-list-schema-unavailable" => "unsupported",
                    _ => "error",
                }
                .into();
                result.error_code = Some(error.0.into());
            }
        }
        Ok(result)
    }
}
struct FlightGuard<'a> {
    service: &'a ModelDiscovery,
    request: String,
}
impl Drop for FlightGuard<'_> {
    fn drop(&mut self) {
        if let Ok(mut flights) = self.service.flights.lock() {
            flights.retain(|flight| flight.request != self.request);
        }
    }
}
struct Cancel(CancellationToken);
impl trace::CancellationFlag for Cancel {
    fn cancelled(&self) -> bool {
        self.0.is_cancelled()
    }
}
fn transport_error(trace: &mut trace::CallTrace, error: reqwest::Error) -> ConfigError {
    if let Some(event) = trace.metadata() {
        event.connect_related = Some(error.is_connect());
        if error.is_timeout() {
            event.timeout_source = trace::TimeoutSource::TransportPredicate;
        }
    }
    ConfigError(if error.is_timeout() {
        "model-list-timeout"
    } else {
        "provider-transport"
    })
}
/// Project only IDs, deterministically deduplicated. A reflected resolved key is
/// rejected before it can reach a safe DTO, DOM or explicit Copy operation.
pub fn project_models(
    value: &serde_json::Value,
    credential: Option<&RuntimeCredential>,
) -> Result<Vec<ModelRow>, ConfigError> {
    let data = value["data"]
        .as_array()
        .ok_or(ConfigError("model-list-schema-unavailable"))?;
    if data.len() > 2000 {
        return Err(ConfigError("model-list-response-limit"));
    }
    let mut ids = BTreeSet::new();
    for row in data {
        let id = row["id"]
            .as_str()
            .filter(|id| {
                !id.trim().is_empty() && id.len() <= 256 && !id.chars().any(char::is_control)
            })
            .ok_or(ConfigError("model-list-schema-unavailable"))?;
        if credential.is_some_and(|key| key.reflected_in(id)) {
            return Err(ConfigError("model-list-private-response"));
        }
        ids.insert(id.to_owned());
    }
    Ok(ids.into_iter().map(|id| ModelRow { id }).collect())
}
fn trace_options(
    configuration: &TextConfiguration,
    draft: &ConnectionDraft,
    endpoint: &reqwest::Url,
) -> trace::SafeOptions {
    trace::SafeOptions {
        scenario: trace::ApiScenario::Metadata,
        purpose: trace::ApiPurpose::ModelList,
        method: trace::ApiMethod::Get,
        profile: match configuration.provider_profile {
            TextProviderProfile::Generic => trace::ApiProfile::Generic,
            TextProviderProfile::DeepSeek => trace::ApiProfile::DeepSeek,
            TextProviderProfile::OpenRouter => trace::ApiProfile::OpenRouter,
            TextProviderProfile::Gemini => trace::ApiProfile::Gemini,
            TextProviderProfile::OllamaOpenAi => trace::ApiProfile::OllamaOpenAi,
        },
        endpoint: if config::loopback(endpoint) {
            trace::EndpointClass::Loopback
        } else if config::credentials::public_key(draft).is_some() {
            trace::EndpointClass::PublicPreset
        } else {
            trace::EndpointClass::Other
        },
        auth: if draft.authentication == config::Authentication::Bearer {
            trace::AuthKind::Bearer
        } else {
            trace::AuthKind::None
        },
        credential_source: match configuration.credential_source.as_deref() {
            Some("app") => trace::CredentialSource::App,
            Some("home_env") => trace::CredentialSource::HomeEnv,
            Some("none") => trace::CredentialSource::None,
            _ => trace::CredentialSource::Unknown,
        },
        credential_present: configuration.runtime_credential.is_some(),
        model_present: !draft.model.is_empty(),
        effort: trace::Effort::Omitted,
        thinking: None,
        max_output_tokens: None,
        timeout_ms: 10_000,
        request_bytes: 0,
        message_count: 0,
        non_streaming: true,
        plain_text_only: true,
    }
}
fn model_failure(code: &str) -> (trace::ApiResult, trace::ApiErrorCategory) {
    use trace::{ApiErrorCategory as E, ApiResult as R};
    match code {
        "model-list-cancelled" => (R::Cancelled, E::Cancelled),
        "model-list-timeout" | "provider-timeout" => (R::Timeout, E::Timeout),
        "provider-auth" => (R::Failed, E::Auth),
        "provider-permission" => (R::Failed, E::Permission),
        "provider-balance" => (R::Failed, E::Balance),
        "provider-rate-limit" => (R::Failed, E::RateLimit),
        "provider-context" => (R::Failed, E::Context),
        "provider-validation" => (R::Failed, E::Validation),
        "provider-unavailable" => (R::Failed, E::Unavailable),
        "provider-transport" => (R::Failed, E::Transport),
        "model-list-response-limit" => (R::Failed, E::Oversize),
        _ => (R::Failed, E::Protocol),
    }
}
