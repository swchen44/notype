//! Closed, Rust-only provider experiment capabilities. No IPC, Serde, runtime
//! destination override or unobserved processor conversion is exposed.
use super::*;
use std::sync::atomic::{AtomicU32, Ordering};

/// Public, nonsecret experiment identities; Gemini has no live constructor.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ExperimentKind {
    /// Fixed DeepSeek Flash, OFF/none, max512 output.
    DeepSeekFlash,
    /// Fixed free Gemma31B through the sole Google AI Studio route.
    OpenRouterGemma31bFreeGoogleAiStudio,
}
/// Observed adverse categories; no error text, URL, body or secret is retained.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdverseCategory {
    /// Actual non-success response headers were observed.
    Http,
    /// Actual provider/output/usage/identity/cost contract failure.
    Contract,
    /// Actual transport or deadline failure.
    Transport,
    /// Durable observer, journal or evidence failure.
    Storage,
    /// Fixed-corpus automated quality check failed.
    Quality,
}
/// Immutable first adverse observation for this round.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdverseSignal {
    /// Closed category of an actual observed failure.
    pub category: AdverseCategory,
    /// Actual numeric response status, absent if no headers were observed.
    pub http_status: Option<u16>,
}
/// Round-wide monotonic first-failure latch, independent of diagnostics/clear.
/// Clones share one allocation; no reset or clearing operation exists.
#[derive(Clone, Default)]
pub struct AdverseLatch(Arc<AtomicU32>);
impl AdverseLatch {
    /// Empty round latch; the runner creates exactly one per fresh round.
    pub fn new() -> Self {
        Self::default()
    }
    /// Read the first safe observed failure, never raw provider information.
    pub fn observed(&self) -> Option<AdverseSignal> {
        let value = self.0.load(Ordering::SeqCst);
        let category = match value >> 16 {
            1 => AdverseCategory::Http,
            2 => AdverseCategory::Contract,
            3 => AdverseCategory::Transport,
            4 => AdverseCategory::Storage,
            5 => AdverseCategory::Quality,
            _ => return None,
        };
        Some(AdverseSignal {
            category,
            http_status: u16::try_from(value & 0xffff)
                .ok()
                .filter(|value| *value > 0),
        })
    }
    pub(super) fn mark(&self, category: AdverseCategory, status: Option<u16>) {
        let code = match category {
            AdverseCategory::Http => 1,
            AdverseCategory::Contract => 2,
            AdverseCategory::Transport => 3,
            AdverseCategory::Storage => 4,
            AdverseCategory::Quality => 5,
        };
        let _ = self.0.compare_exchange(
            0,
            (code << 16) | u32::from(status.unwrap_or(0)),
            Ordering::SeqCst,
            Ordering::SeqCst,
        );
    }
}
/// Owned listening socket capability. It can authorize only this bound loopback
/// address and fixed chat path, and can never become a live HTTPS capability.
#[derive(Clone)]
pub struct OwnedLoopback {
    address: std::net::SocketAddr,
}
impl OwnedLoopback {
    /// Verify the caller owns an already-bound nonzero loopback listener.
    pub fn from_listener(listener: &std::net::TcpListener) -> Result<Self, TextFailure> {
        let address = listener
            .local_addr()
            .map_err(|_| TextFailure::ConfigurationRequired)?;
        if !address.ip().is_loopback() || address.port() == 0 {
            return Err(TextFailure::ConfigurationRequired);
        }
        Ok(Self { address })
    }
    /// Synthetic fixed destination; never a configuration/credential getter.
    pub fn endpoint(&self) -> String {
        format!("http://{}/v1/chat/completions", self.address)
    }
}
/// Closed experiment policy; all live route/model literals are code owned.
#[derive(Clone)]
pub enum ExperimentPolicy {
    /// Fixed official DeepSeek endpoint only.
    DeepSeekFlash(AdverseLatch),
    /// Fixed official OpenRouter endpoint with its reviewed sole free route.
    OpenRouterGemma31bFreeGoogleAiStudio(AdverseLatch),
    /// Check/test substitution bound to an owned socket, never public HTTPS.
    OwnedLoopback(OwnedLoopback, ExperimentKind, AdverseLatch),
}
impl ExperimentPolicy {
    fn parts(&self) -> (String, ExperimentKind, AdverseLatch) {
        match self {
            Self::DeepSeekFlash(latch) => (
                "https://api.deepseek.com/chat/completions".into(),
                ExperimentKind::DeepSeekFlash,
                latch.clone(),
            ),
            Self::OpenRouterGemma31bFreeGoogleAiStudio(latch) => (
                "https://openrouter.ai/api/v1/chat/completions".into(),
                ExperimentKind::OpenRouterGemma31bFreeGoogleAiStudio,
                latch.clone(),
            ),
            Self::OwnedLoopback(endpoint, kind, latch) => {
                (endpoint.endpoint(), *kind, latch.clone())
            }
        }
    }
}
/// Sealed observed-only transport. Only the helper's private budget wrapper
/// implements TextProcessor; no conversion to an arbitrary processor is offered.
pub struct ExperimentProcessor {
    inner: ChatProcessor,
    latch: AdverseLatch,
}

#[derive(Clone)]
pub(super) struct ExperimentContract {
    endpoint: String,
    kind: ExperimentKind,
    latch: AdverseLatch,
}
impl ExperimentContract {
    fn model(&self) -> &'static str {
        match self.kind {
            ExperimentKind::DeepSeekFlash => "deepseek-flash",
            ExperimentKind::OpenRouterGemma31bFreeGoogleAiStudio => "google/gemma-4-31b-it:free",
        }
    }
    pub(super) fn apply(&self, body: &mut Value) {
        if self.kind == ExperimentKind::OpenRouterGemma31bFreeGoogleAiStudio {
            body["provider"] = json!({"only":["google-ai-studio"],"allow_fallbacks":false,"require_parameters":true,"max_price":{"prompt":0,"completion":0,"request":0}});
        }
    }
    pub(super) fn validate_wire(
        &self,
        endpoint: &reqwest::Url,
        wire: &[u8],
        descriptor: &ApiCallDescriptor,
    ) -> Result<(), TextFailure> {
        let invalid = || {
            self.latch.mark(AdverseCategory::Contract, None);
            TextFailure::ConfigurationRequired
        };
        if self.latch.observed().is_some()
            || endpoint.as_str() != self.endpoint
            || wire.len() > 8192
            || wire.is_empty()
        {
            return Err(invalid());
        }
        let body: Value = serde_json::from_slice(wire).map_err(|_| invalid())?;
        let messages = body["messages"].as_array().ok_or_else(invalid)?;
        let deepseek = self.kind == ExperimentKind::DeepSeekFlash;
        let profile = if deepseek {
            ApiProviderProfile::DeepSeek
        } else {
            ApiProviderProfile::OpenRouter
        };
        let effort = if deepseek { Some("none") } else { None };
        if body["model"] != self.model()
            || body["stream"] != false
            || body["max_tokens"] != 512
            || messages.is_empty()
            || messages.len() > 4
            || messages.iter().any(|message| {
                message.as_object().is_none_or(|object| object.len() != 2)
                    || !matches!(message["role"].as_str(), Some("system" | "user"))
                    || !message["content"].is_string()
            })
            || descriptor.profile != profile
            || descriptor.requested_model.as_deref() != Some(self.model())
            || descriptor.requested_effort.as_deref() != effort
            || descriptor.thinking != Some(false)
            || descriptor.max_output_tokens != Some(512)
            || descriptor.wire_bytes != wire.len() as u64
            || descriptor.message_count != messages.len() as u32
            || !descriptor.non_streaming
            || !descriptor.plain_text_only
        {
            return Err(invalid());
        }
        let fields: &[&str] = if deepseek {
            &[
                "model",
                "messages",
                "stream",
                "max_tokens",
                "thinking",
                "reasoning_effort",
                "temperature",
            ]
        } else {
            &[
                "model",
                "messages",
                "stream",
                "max_tokens",
                "reasoning",
                "provider",
            ]
        };
        if body.as_object().is_none_or(|object| {
            object.len() != fields.len() || object.keys().any(|key| !fields.contains(&key.as_str()))
        }) {
            return Err(invalid());
        }
        if deepseek {
            if body["thinking"] != json!({"type":"disabled"})
                || body["reasoning_effort"] != "none"
                || body["temperature"].as_f64() != Some(0.0)
            {
                return Err(invalid());
            }
        } else if body["reasoning"] != json!({"enabled":false})
            || body["provider"]
                != json!({"only":["google-ai-studio"],"allow_fallbacks":false,"require_parameters":true,"max_price":{"prompt":0,"completion":0,"request":0}})
        {
            return Err(invalid());
        }
        Ok(())
    }
    pub(super) fn response_valid(&self, value: &Value, receipt: &ApiReceipt, raw: &[u8]) -> bool {
        let zero_cost = self.kind != ExperimentKind::OpenRouterGemma31bFreeGoogleAiStudio
            || value["usage"]["cost"]
                .as_f64()
                .is_some_and(|cost| cost.is_finite() && cost == 0.0);
        coherent_receipt(receipt) && zero_cost && no_numeric_underflow(raw)
    }
    pub(super) fn contract_failed(&self, status: Option<u16>) {
        self.latch.mark(AdverseCategory::Contract, status);
    }
    pub(super) fn headers(&self, status: u16) {
        if !(200..300).contains(&status) {
            self.latch.mark(AdverseCategory::Http, Some(status));
        }
    }
    pub(super) fn failed(&self, error: TextFailure, status: Option<u16>) {
        let category = match error {
            TextFailure::Cancelled => return,
            TextFailure::Timeout
            | TextFailure::Provider(GatewayError::Transport | GatewayError::ProviderTimeout) => {
                AdverseCategory::Transport
            }
            TextFailure::HistoryStorage => AdverseCategory::Storage,
            _ => AdverseCategory::Contract,
        };
        self.latch.mark(category, status);
    }
}
/// Validate number lexemes without parsing another response or extracting any
/// field. The ordinary JSON parser remains authoritative; this bounded guard
/// only prevents a nonzero cost/usage number from silently rounding to zero.
/// Quoted text is skipped and no raw bytes leave this backend boundary.
pub(crate) fn no_numeric_underflow(bytes: &[u8]) -> bool {
    let (mut index, mut quoted, mut escaped) = (0, false, false);
    while index < bytes.len() {
        let byte = bytes[index];
        if quoted {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                quoted = false;
            }
            index += 1;
            continue;
        }
        if byte == b'"' {
            quoted = true;
            index += 1;
            continue;
        }
        if byte.is_ascii_digit() || byte == b'-' {
            let start = index;
            while index < bytes.len()
                && matches!(bytes[index], b'0'..=b'9' | b'.' | b'-' | b'+' | b'e' | b'E')
            {
                index += 1;
            }
            let Ok(token) = std::str::from_utf8(&bytes[start..index]) else {
                return false;
            };
            if token.parse::<f64>().is_ok_and(|value| value == 0.0)
                && token
                    .split(['e', 'E'])
                    .next()
                    .is_some_and(|part| part.bytes().any(|b| matches!(b, b'1'..=b'9')))
            {
                return false;
            }
            continue;
        }
        index += 1;
    }
    true
}
/// Same bounded typed receipt contract for the common parser, composed observer
/// and independent persisted-core reconciliation. Cost stays a separate gate.
pub(crate) fn coherent_receipt(receipt: &ApiReceipt) -> bool {
    let (Some(input), Some(output), Some(total)) = (
        receipt.usage.input,
        receipt.usage.output,
        receipt.total_tokens,
    ) else {
        return false;
    };
    let usage_valid = input <= 32768
        && output <= 512
        && input.checked_add(output) == Some(total)
        && !receipt.usage_invalid
        && !receipt.reasoning_nonempty
        && receipt
            .usage
            .reasoning_output
            .is_none_or(|value| value == 0)
        && receipt
            .usage
            .cached_input
            .is_none_or(|value| value <= input)
        && receipt
            .usage
            .uncached_input
            .is_none_or(|value| value <= input)
        && match (receipt.usage.cached_input, receipt.usage.uncached_input) {
            (Some(hit), Some(miss)) => hit.checked_add(miss) == Some(input),
            _ => true,
        };
    let identity_valid = receipt.reported_model.as_deref().is_some_and(|model| {
        !model.trim().is_empty() && model.len() <= 256 && !model.chars().any(char::is_control)
    });
    usage_valid && identity_valid
}
impl ExperimentProcessor {
    /// Latch an actual composed-observer failure; this can only halt, never
    /// authorize, clear or change another response observation.
    pub fn observer_failed(&self) {
        self.latch.mark(AdverseCategory::Storage, None);
    }
    /// Read shared monotonic status without a marking/reset capability.
    pub fn adverse(&self) -> Option<AdverseSignal> {
        self.latch.observed()
    }
    /// Use the actual production observed generation path and observer identity.
    pub fn generate_observed(
        &self,
        request: TextRequest,
        cancel: CancellationToken,
        observer: Arc<dyn ApiCallObserver>,
    ) -> GenerationFuture {
        self.inner.generate_observed(request, cancel, observer)
    }
    /// Use the same common send gate for an actual automatic reverse call.
    pub fn back_translate_observed(
        &self,
        request: BackTranslationRequest,
        cancel: CancellationToken,
        observer: Arc<dyn ApiCallObserver>,
    ) -> TextFuture {
        self.inner
            .back_translate_observed(request, cancel, observer)
    }
}
/// Consume the production resolved snapshot with direct/no-retry/no-redirect
/// networking. Construction alone performs no HTTP or credential discovery.
pub fn experiment_processor(
    configuration: TextConfiguration,
    policy: ExperimentPolicy,
    tracing: diagnostic::TraceContext,
) -> Result<ExperimentProcessor, TextFailure> {
    let (endpoint, kind, latch) = policy.parts();
    if latch.observed().is_some() {
        return Err(TextFailure::ConfigurationRequired);
    }
    let fixture = matches!(policy, ExperimentPolicy::OwnedLoopback(..));
    let expected_profile = if kind == ExperimentKind::DeepSeekFlash {
        TextProviderProfile::DeepSeek
    } else {
        TextProviderProfile::OpenRouter
    };
    let expected_model = if kind == ExperimentKind::DeepSeekFlash {
        "deepseek-flash"
    } else {
        "google/gemma-4-31b-it:free"
    };
    let expected_effort = if kind == ExperimentKind::DeepSeekFlash {
        Some("none")
    } else {
        None
    };
    if configuration.endpoint != endpoint
        || configuration.provider_profile != expected_profile
        || configuration.model != expected_model
        || configuration.thinking_override != Some(false)
        || configuration.effort.as_deref() != expected_effort
        || configuration.max_output_tokens != Some(512)
        || (!fixture
            && (configuration.authentication != TextAuthentication::Bearer
                || configuration.runtime_credential.is_none()
                || !configuration.api_key.is_empty()))
    {
        latch.mark(AdverseCategory::Contract, None);
        return Err(TextFailure::ConfigurationRequired);
    }
    let mut inner = ChatProcessor::new_with_policy(configuration, true)?;
    inner.tracing = tracing;
    inner.experiment = Some(ExperimentContract {
        endpoint,
        kind,
        latch: latch.clone(),
    });
    Ok(ExperimentProcessor { inner, latch })
}
