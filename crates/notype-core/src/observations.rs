//! Durable product observations. This module never reads diagnostics, transport
//! credentials, provider settings documents or raw network request/response text.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::time::Instant;

/// Identity attached to the engine that produced the original, independently
/// of any later configured selection. It never contains a filesystem path.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelIdentity {
    /// Stable catalog/producer identifier.
    pub id: String,
    /// Display name known by that same producer, if available.
    pub name: Option<String>,
    /// Safe backend label, if the producer exposes it.
    pub backend: Option<String>,
}

/// Received native-rate mono frames before VAD/resampling/decoder padding.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecordingObservation {
    /// Actually received frames, including silence and final drain.
    pub received_frames: u64,
    /// Native mono frame rate; always positive for validated observations.
    pub input_rate: u32,
    /// Known dropped frames, or None when capture quality is not observable.
    pub dropped_frames: Option<u64>,
}
impl RecordingObservation {
    /// Validate the sample clock without treating missing frames as recorded.
    pub fn new(
        received_frames: u64,
        input_rate: u32,
        dropped_frames: Option<u64>,
    ) -> Result<Self, crate::CoreError> {
        if input_rate == 0
            || u128::from(received_frames) * 1000 / u128::from(input_rate) > u128::from(u64::MAX)
        {
            return Err(crate::CoreError::InvalidInput);
        }
        Ok(Self {
            received_frames,
            input_rate,
            dropped_frames,
        })
    }
    /// Milliseconds of received audio, never padded decoder length or UI time.
    pub fn audio_ms(&self) -> u64 {
        (u128::from(self.received_frames) * 1000 / u128::from(self.input_rate.max(1))) as u64
    }
    /// Complete only when the producer explicitly observed zero dropped frames.
    pub fn quality(&self) -> &'static str {
        match self.dropped_frames {
            Some(0) => "complete",
            Some(_) => "incomplete",
            None => "unknown",
        }
    }
}

/// Durable safe observations belonging to one text-history record. Optional
/// fields mean unobserved; diagnostics preferences never govern these values.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SessionObservations {
    /// Received audio and capture quality, independent of decoder PCM padding.
    pub recording: Option<RecordingObservation>,
    /// The configured ASR selection captured before recording.
    pub requested_asr_model: Option<String>,
    /// The actual producer of the original, not the current global selection.
    pub actual_asr_model: Option<ModelIdentity>,
    /// Original text count under count_rule.
    pub original_count: Option<u64>,
    /// Current result count under count_rule, None before a valid result exists.
    pub result_count: Option<u64>,
    /// Persisted rule so future algorithms cannot silently rewrite research data.
    pub count_rule: String,
    /// Accepted stop to durable normalized original, measured in one process.
    pub asr_after_stop_ms: Option<u64>,
    /// First initial capture's usable result, excluding review and delivery.
    pub initial_stop_to_result_ms: Option<u64>,
    /// initial or retry; retry never uses an old recording's stop clock.
    pub latest_processing_kind: Option<String>,
    /// Latest observed processing interval, not human time between attempts.
    pub latest_processing_ms: Option<u64>,
    /// Latest API stage interval, None when no actual network call occurred.
    pub latest_api_ms: Option<u64>,
    /// Typed application outcome label; absent when no endpoint was observed.
    pub latest_outcome: Option<String>,
    /// An adapter/storage failure prevented a complete accounting of all calls.
    pub api_metadata_unavailable: bool,
}
impl Default for SessionObservations {
    fn default() -> Self {
        Self {
            recording: None,
            requested_asr_model: None,
            actual_asr_model: None,
            original_count: None,
            result_count: None,
            count_rule: COUNT_RULE_VERSION.into(),
            asr_after_stop_ms: None,
            initial_stop_to_result_ms: None,
            latest_processing_kind: None,
            latest_processing_ms: None,
            latest_api_ms: None,
            latest_outcome: None,
            api_metadata_unavailable: false,
        }
    }
}
impl SessionObservations {
    /// Validate only typed metadata, never a settings document or raw response.
    pub fn validate(&self) -> Result<(), crate::CoreError> {
        if self
            .latest_processing_kind
            .as_deref()
            .is_some_and(|value| !matches!(value, "initial" | "retry"))
            || self.latest_outcome.as_deref().is_some_and(|value| {
                !matches!(
                    value,
                    "original_only" | "pending" | "failed" | "cancelled" | "interrupted"
                )
            })
        {
            return Err(crate::CoreError::InvalidInput);
        }
        if let Some(recording) = &self.recording {
            RecordingObservation::new(
                recording.received_frames,
                recording.input_rate,
                recording.dropped_frames,
            )?;
        }
        if self
            .requested_asr_model
            .as_ref()
            .is_some_and(|value| !safe_label(value))
            || self.actual_asr_model.as_ref().is_some_and(|model| {
                !safe_label(&model.id)
                    || model.name.as_ref().is_some_and(|value| !safe_label(value))
                    || model
                        .backend
                        .as_ref()
                        .is_some_and(|value| !safe_label(value))
            })
        {
            return Err(crate::CoreError::InvalidInput);
        }
        Ok(())
    }
}

pub(crate) fn safe_label(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 256 && !value.chars().any(char::is_control)
}

/// Why one actual HTTP request was issued. A retry keeps the same purpose but
/// receives a new durable session-revision/call identity.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApiPurpose {
    /// Produce the selected scenario's result.
    #[default]
    Generation,
    /// Independently translate the generated result back for human review.
    BackTranslation,
}

/// Observed local completion, without assuming whether the provider charged.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApiOutcome {
    /// Prepared/in-flight; no terminal endpoint has been observed.
    #[default]
    Pending,
    /// Valid provider text was accepted.
    Success,
    /// A response arrived but failed output validation.
    InvalidResponse,
    /// HTTP/transport or explicit provider failure.
    Failed,
    /// Local deadline elapsed; the request may have charged.
    Timeout,
    /// User cancelled locally; the request may have charged.
    Cancelled,
    /// Future/process ended without a normal terminal observation.
    Interrupted,
}

/// Explicit transport profile; arbitrary endpoint/configuration strings cannot
/// be passed as a substitute label into the durable accounting boundary.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApiProviderProfile {
    /// Generic OpenAI-compatible request options.
    #[default]
    Generic,
    /// Explicit DeepSeek option rules.
    #[serde(rename = "deepseek")]
    DeepSeek,
    /// Explicit OpenRouter compatibility mapping.
    OpenRouter,
    /// Explicit Gemini OpenAI compatibility mapping.
    Gemini,
    /// Explicit Ollama OpenAI compatibility mapping, without native APIs.
    OllamaOpenAi,
    /// Shared noncompany development HTTP protocol.
    Development,
}

/// Safe classification of the actual serialized request's destination.
/// No URL, credential, path or organization address is retained.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApiEndpointClass {
    /// Any destination outside the two explicitly recognized classes.
    #[default]
    Other,
    /// Exact reviewed public HTTPS chat endpoint.
    DeepSeekHttps,
    /// Plain HTTP on a loopback IP, used only by owned test fixtures.
    Loopback,
}

/// Safe request projection known only after the actual wire is serialized.
/// It deliberately has no request text, endpoint, headers or credential field.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ApiCallDescriptor {
    /// Configured versus actual wire options, independent of diagnostics ON/OFF.
    pub parameters: Option<RequestParameters>,
    /// Derived from the actual transport URL, never supplied as a provider preference.
    pub endpoint_class: ApiEndpointClass,
    /// Scenario generation or independent reverse translation.
    pub purpose: ApiPurpose,
    /// Explicit generic/deepseek/development profile label.
    pub profile: ApiProviderProfile,
    /// Model name actually sent, not a later preference.
    pub requested_model: Option<String>,
    /// Explicit effort sent; None means omitted/provider default.
    pub requested_effort: Option<String>,
    /// Explicit request output limit, if supplied.
    pub max_output_tokens: Option<u32>,
    /// Exact serialized HTTP body length; no body content is retained.
    pub wire_bytes: u64,
    /// Effective request thinking switch, if the explicit profile sends one.
    pub thinking: Option<bool>,
    /// Whether this actual request uses the non-streaming response protocol.
    pub non_streaming: bool,
    /// Actual serialized message count, without message contents.
    pub message_count: u32,
    /// Every actual message is plain text, with no tools or attachments.
    pub plain_text_only: bool,
}
impl Default for ApiCallDescriptor {
    fn default() -> Self {
        Self {
            parameters: None,
            endpoint_class: Default::default(),
            purpose: Default::default(),
            profile: Default::default(),
            requested_model: None,
            requested_effort: None,
            max_output_tokens: None,
            wire_bytes: 0,
            thinking: None,
            non_streaming: true,
            message_count: 0,
            plain_text_only: false,
        }
    }
}
impl ApiCallDescriptor {
    pub(crate) fn valid(&self) -> bool {
        self.requested_model.as_ref().is_none_or(|s| safe_label(s))
            && self
                .requested_effort
                .as_ref()
                .is_none_or(|s| valid_effort(s))
            && self
                .parameters
                .as_ref()
                .is_none_or(RequestParameters::valid)
    }
}

/// Credential-free configured/wire request tuple. Numeric decimal strings are
/// constructed from finite validated inputs, not arbitrary provider metadata.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct RequestParameters {
    /// User OFF/ON preference, distinct from mandatory minimum resolution.
    pub requested_thinking: bool,
    /// Remembered ON preference, even when OFF ignores it on the wire.
    pub preference_effort: Option<String>,
    /// Effective request switch, absent when no switch is sent.
    pub effective_thinking: Option<bool>,
    /// Actual wire effort; reported effort remains receipt-only.
    pub effective_effort: Option<String>,
    /// documented, provider_metadata, user_declared or unknown.
    pub capability_provenance: String,
    /// non_reasoning, disabled, enabled, mandatory_minimum or unresolved.
    pub resolution_reason: String,
    /// Configured finite temperature decimal, absent when omitted.
    pub configured_temperature: Option<String>,
    /// Actual serialized temperature, absent when omitted/ignored.
    pub wire_temperature: Option<String>,
    /// omitted, sent, ignored_by_provider or unsupported.
    pub temperature_resolution: String,
    /// Configured finite top_p decimal.
    pub configured_top_p: Option<String>,
    /// Actual serialized top_p, without silently applying documented normalization.
    pub wire_top_p: Option<String>,
    /// omitted, sent, ignored_by_provider or unsupported.
    pub top_p_resolution: String,
    /// Documented normalization only, not an observed server value.
    pub documented_top_p: Option<String>,
    /// Configured/serialized positive output maximum, absent when omitted.
    pub max_output_tokens: Option<u32>,
}
impl RequestParameters {
    fn valid(&self) -> bool {
        let number = |value: &Option<String>, upper: f64, positive: bool| {
            value.as_ref().is_none_or(|text| {
                text.len() <= 400
                    && text.parse::<f64>().is_ok_and(|value| {
                        value.is_finite()
                            && value <= upper
                            && if positive { value > 0.0 } else { value >= 0.0 }
                    })
            })
        };
        self.preference_effort.as_deref().is_none_or(valid_effort)
            && self.effective_effort.as_deref().is_none_or(valid_effort)
            && [
                "documented",
                "provider_metadata",
                "user_declared",
                "unknown",
            ]
            .contains(&self.capability_provenance.as_str())
            && [
                "non_reasoning",
                "disabled",
                "enabled",
                "mandatory_minimum",
                "unresolved",
            ]
            .contains(&self.resolution_reason.as_str())
            && [&self.temperature_resolution, &self.top_p_resolution]
                .iter()
                .all(|value| {
                    ["omitted", "sent", "ignored_by_provider", "unsupported"]
                        .contains(&value.as_str())
                })
            && number(&self.configured_temperature, 2.0, false)
            && number(&self.wire_temperature, 2.0, false)
            && number(&self.configured_top_p, 1.0, true)
            && number(&self.wire_top_p, 1.0, true)
            && number(&self.documented_top_p, 1.0, true)
            && self
                .max_output_tokens
                .is_none_or(|value| value > 0 && value <= 1_000_000)
    }
}

/// Explicit generic effort spelling; provider support remains server-specific.
pub fn valid_effort(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 32
        && value
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, b'_' | b'-'))
}

/// Provider-reported token totals and optional breakdowns. None is unknown,
/// never inferred from the text and never replaced with a fabricated zero.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TokenUsage {
    /// Prompt/input total, including any cached portion already counted here.
    pub input: Option<u64>,
    /// Completion/output total, including any reasoning portion already counted.
    pub output: Option<u64>,
    /// Optional cached input subset; not added to input again.
    pub cached_input: Option<u64>,
    /// Optional uncached input subset.
    pub uncached_input: Option<u64>,
    /// Optional reasoning output subset; not added to output again.
    pub reasoning_output: Option<u64>,
}

/// Actual returned billing receipt, never a configured or calculated price.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActualCharge {
    /// Canonical exact nonnegative decimal, scale at most 18.
    pub amount: String,
    /// Explicit supported currency, normalized to uppercase.
    pub currency: String,
    /// Adapter's documented receipt-contract identifier, without endpoint data.
    pub provenance: String,
}
impl ActualCharge {
    /// Validate an explicit actual receipt; never accepts a token-rate estimate.
    pub fn new(amount: &str, currency: &str, provenance: &str) -> Result<Self, crate::CoreError> {
        let amount = Decimal::parse(amount).ok_or(crate::CoreError::InvalidInput)?;
        let currency = currency.to_ascii_uppercase();
        if !(3..=12).contains(&currency.len())
            || !currency.bytes().all(|c| c.is_ascii_uppercase())
            || !safe_label(provenance)
        {
            return Err(crate::CoreError::InvalidInput);
        }
        Ok(Self {
            amount: amount.text(),
            currency,
            provenance: provenance.into(),
        })
    }
}

/// Bounded safe response metadata. Raw bodies, reasoning and errors are absent.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ApiReceipt {
    /// The model string explicitly returned by the server.
    pub reported_model: Option<String>,
    /// Only a supported response's explicitly reported effort, otherwise None.
    pub reported_effort: Option<String>,
    /// Independently optional usage fields.
    pub usage: TokenUsage,
    /// Actual amount/currency under a documented adapter contract only.
    pub actual_charge: Option<ActualCharge>,
    /// Provider-reported prompt plus completion count, not a locally inferred total.
    pub total_tokens: Option<u64>,
    /// Whether a reasoning field was nonempty or malformed; no reasoning text is retained.
    pub reasoning_nonempty: bool,
    /// A reported usage field had an invalid type/range or conflicting breakdown.
    /// Ordinary history still preserves each independently valid count.
    pub usage_invalid: bool,
}
const MAX_USAGE: u64 = 9_007_199_254_740_991;
impl ApiReceipt {
    /// Parse only standard compatible chat metadata. DeepSeek has no documented
    /// monetary receipt: no arbitrary cost/price field is interpreted as billing.
    pub fn from_chat_json(value: &serde_json::Value) -> Self {
        let token = |pointer: &str| {
            value
                .pointer(pointer)
                .and_then(serde_json::Value::as_u64)
                .filter(|n| *n <= MAX_USAGE)
        };
        Self {
            reported_model: value["model"].as_str().map(str::to_owned),
            reported_effort: value["reasoning_effort"].as_str().map(str::to_owned),
            usage: TokenUsage {
                input: token("/usage/prompt_tokens"),
                output: token("/usage/completion_tokens"),
                cached_input: token("/usage/prompt_cache_hit_tokens")
                    .or_else(|| token("/usage/prompt_tokens_details/cached_tokens")),
                uncached_input: token("/usage/prompt_cache_miss_tokens"),
                reasoning_output: token("/usage/completion_tokens_details/reasoning_tokens"),
            },
            actual_charge: None,
            total_tokens: token("/usage/total_tokens"),
            reasoning_nonempty: value["choices"].as_array().is_some_and(|choices| {
                choices.iter().any(|choice| {
                    choice["message"]
                        .get("reasoning_content")
                        .is_some_and(|reasoning| {
                            !reasoning.is_null()
                                && reasoning.as_str().is_none_or(|text| !text.is_empty())
                        })
                })
            }),
            usage_invalid: [
                "/usage/prompt_tokens", "/usage/completion_tokens", "/usage/total_tokens",
                "/usage/prompt_cache_hit_tokens", "/usage/prompt_cache_miss_tokens",
                "/usage/prompt_tokens_details/cached_tokens", "/usage/completion_tokens_details/reasoning_tokens",
            ].iter().any(|pointer| value.pointer(pointer).is_some_and(|field| !field.is_null() && field.as_u64().is_none_or(|count| count > MAX_USAGE)))
                || matches!((token("/usage/prompt_cache_hit_tokens"), token("/usage/prompt_tokens_details/cached_tokens")), (Some(left),Some(right)) if left != right),
        }
        .sanitized()
    }
    pub(crate) fn sanitized(mut self) -> Self {
        self.total_tokens = self.total_tokens.filter(|value| *value <= MAX_USAGE);
        self.reported_model = self.reported_model.filter(|s| safe_label(s));
        self.reported_effort = self.reported_effort.filter(|s| valid_effort(s));
        for field in [
            &mut self.usage.input,
            &mut self.usage.output,
            &mut self.usage.cached_input,
            &mut self.usage.uncached_input,
            &mut self.usage.reasoning_output,
        ] {
            if field.is_some_and(|n| n > MAX_USAGE) {
                *field = None;
            }
        }
        self.actual_charge = self.actual_charge.and_then(|value| {
            ActualCharge::new(&value.amount, &value.currency, &value.provenance).ok()
        });
        self
    }
}

/// Formal transport accounting port. begin/mark_sent can refuse spending;
/// response/finish retain a bounded receipt for the final store transaction.
pub trait ApiCallObserver: Send + Sync {
    /// Actual accepted revision, read-only and independent of admission/spending.
    /// Custom/direct callers remain unassociated by default.
    fn processing_revision(&self) -> Option<u64> {
        None
    }
    /// Durably prepare one call after its actual wire/options are known.
    fn begin(&self, descriptor: ApiCallDescriptor) -> Result<String, crate::TextFailure>;
    /// Final pre-send boundary: return an error before HTTP is polled.
    fn mark_sent(&self, call_id: &str) -> Result<(), crate::TextFailure>;
    /// Observe safe metadata before application text validation.
    fn response(&self, call_id: &str, receipt: ApiReceipt);
    /// Finish local outcome/time without claiming that cancellation was free.
    fn finish(&self, call_id: &str, outcome: ApiOutcome, elapsed_ms: u64);
    /// An uninstrumented adapter cannot claim zero network use.
    fn unavailable(&self);
}

/// Exactly one production request's lifecycle. Drop records an incomplete
/// outcome; it cannot overwrite the current session or trigger another request.
pub struct ApiCallGuard {
    observer: Arc<dyn ApiCallObserver>,
    id: String,
    started: Instant,
    finished: bool,
}
impl ApiCallGuard {
    /// Prepare a validated safe descriptor before constructing a possible send.
    pub fn begin(
        observer: Arc<dyn ApiCallObserver>,
        descriptor: ApiCallDescriptor,
    ) -> Result<Self, crate::TextFailure> {
        if !descriptor.valid() {
            return Err(crate::TextFailure::ConfigurationRequired);
        }
        let id = observer.begin(descriptor)?;
        Ok(Self {
            observer,
            id,
            started: Instant::now(),
            finished: false,
        })
    }
    /// Refusable boundary immediately before the actual HTTP future is polled.
    pub fn mark_sent(&mut self) -> Result<(), crate::TextFailure> {
        self.observer.mark_sent(&self.id)
    }
    /// Safe model/usage is captured even if output validation later fails.
    pub fn response(&mut self, receipt: ApiReceipt) {
        self.observer.response(&self.id, receipt.sanitized());
    }
    /// Record one terminal observation; no further HTTP work is performed.
    pub fn finish(mut self, outcome: ApiOutcome) {
        self.observer.finish(
            &self.id,
            outcome,
            self.started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
        );
        self.finished = true;
    }
}
impl Drop for ApiCallGuard {
    fn drop(&mut self) {
        if !self.finished {
            self.observer.finish(
                &self.id,
                ApiOutcome::Interrupted,
                self.started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
            );
        }
    }
}

/// Completeness of a history-level field; unknown is never displayed as free.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Completeness {
    /// No observed API use and no missing instrumentation.
    #[default]
    NotApplicable,
    /// Every relevant possibly-sent call reported this field.
    Complete,
    /// Some, but not all, relevant calls reported this field.
    Partial,
    /// No reliable total, including unsupported/overflow observations.
    Unknown,
}
/// One nullable exact token aggregate, independent of the other token fields.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct UsageTotal {
    /// Whether total is complete, partial, unknown or not applicable.
    pub status: Completeness,
    /// Complete total only; None for missing observations.
    pub total: Option<u64>,
    /// Sum of reported values, explicitly not necessarily the full total.
    pub known_subtotal: Option<u64>,
    /// Number of relevant calls reporting a valid value.
    pub reported_calls: u64,
}
/// One currency's exact amount; never combined with another currency.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CurrencyAmount {
    /// Explicit reported currency.
    pub currency: String,
    /// Canonical exact decimal sum.
    pub amount: String,
}
/// Per-currency actual-fee totals and clearly separated partial subtotals.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FeeTotal {
    /// Completeness across all possibly-sent calls.
    pub status: Completeness,
    /// Full per-currency totals only when all relevant receipts are available.
    pub totals: Vec<CurrencyAmount>,
    /// Reported per-currency amounts, never presented as the complete total.
    pub known_subtotals: Vec<CurrencyAmount>,
    /// Number of calls with an actual, validated amount and currency.
    pub reported_calls: u64,
}
/// Preserve which requested/reported model was associated with each effort.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct ModelEffort {
    /// Safe option tuple bound to these requested/reported models, if observed.
    #[serde(default)]
    pub parameters: Option<RequestParameters>,
    /// Model sent to the API.
    pub requested_model: Option<String>,
    /// Model explicitly returned by the API.
    pub reported_model: Option<String>,
    /// Explicit requested effort, absent for provider default.
    pub requested_effort: Option<String>,
    /// Effort explicitly reported by the API, not inferred.
    pub reported_effort: Option<String>,
}
/// Fixed summary for one history ID, accumulated over retries/back-translation.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ApiSummary {
    /// Complete count if instrumentation is known complete, otherwise None.
    pub call_count: Option<u64>,
    /// Number of possibly-sent calls actually present in the durable ledger.
    pub observed_call_count: u64,
    /// Independent prompt/input token summary.
    pub input: UsageTotal,
    /// Independent completion/output token summary.
    pub output: UsageTotal,
    /// Actual returned money, by currency; never an estimate.
    pub fees: FeeTotal,
    /// Sorted distinct requested model names.
    pub requested_models: Vec<String>,
    /// Sorted distinct response model names.
    pub reported_models: Vec<String>,
    /// Sorted distinct model/effort tuples, without a per-call export.
    pub model_efforts: Vec<ModelEffort>,
}

#[derive(Clone, Copy)]
struct Decimal(i128);
impl Decimal {
    const SCALE: i128 = 1_000_000_000_000_000_000;
    fn parse(value: &str) -> Option<Self> {
        if value.is_empty() || value.len() > 64 {
            return None;
        }
        let mut parts = value.split('.');
        let whole = parts.next()?;
        let fraction = parts.next().unwrap_or("");
        if parts.next().is_some()
            || whole.is_empty()
            || fraction.len() > 18
            || !whole.bytes().all(|b| b.is_ascii_digit())
            || !fraction.bytes().all(|b| b.is_ascii_digit())
        {
            return None;
        }
        let whole = whole.parse::<i128>().ok()?.checked_mul(Self::SCALE)?;
        let fraction = if fraction.is_empty() {
            0
        } else {
            fraction
                .parse::<i128>()
                .ok()?
                .checked_mul(10_i128.checked_pow((18 - fraction.len()) as u32)?)?
        };
        Some(Self(whole.checked_add(fraction)?))
    }
    fn text(self) -> String {
        let whole = self.0 / Self::SCALE;
        let fraction = self.0 % Self::SCALE;
        if fraction == 0 {
            whole.to_string()
        } else {
            format!(
                "{whole}.{}",
                format!("{fraction:018}").trim_end_matches('0')
            )
        }
    }
}

pub(crate) struct SummaryAccumulator {
    count: u64,
    unknown: bool,
    input: RunningUsage,
    output: RunningUsage,
    fees: BTreeMap<String, Decimal>,
    fee_count: u64,
    overflow: bool,
    requested: BTreeSet<String>,
    reported: BTreeSet<String>,
    pairs: BTreeSet<ModelEffort>,
}
impl SummaryAccumulator {
    pub(crate) fn new(unknown: bool) -> Self {
        Self {
            count: 0,
            unknown,
            input: RunningUsage::default(),
            output: RunningUsage::default(),
            fees: BTreeMap::new(),
            fee_count: 0,
            overflow: false,
            requested: BTreeSet::new(),
            reported: BTreeSet::new(),
            pairs: BTreeSet::new(),
        }
    }
    pub(crate) fn add(&mut self, descriptor: ApiCallDescriptor, receipt: ApiReceipt) {
        self.count += 1;
        self.input.add(receipt.usage.input);
        self.output.add(receipt.usage.output);
        if let Some(charge) = receipt.actual_charge {
            if let Some(value) = Decimal::parse(&charge.amount) {
                self.fee_count += 1;
                let old = self.fees.entry(charge.currency).or_insert(Decimal(0));
                if let Some(sum) = old.0.checked_add(value.0) {
                    old.0 = sum;
                } else {
                    self.overflow = true;
                }
            }
        }
        if let Some(value) = &descriptor.requested_model {
            self.requested.insert(value.clone());
        }
        if let Some(value) = &receipt.reported_model {
            self.reported.insert(value.clone());
        }
        self.pairs.insert(ModelEffort {
            parameters: descriptor.parameters,
            requested_model: descriptor.requested_model,
            reported_model: receipt.reported_model,
            requested_effort: descriptor.requested_effort,
            reported_effort: receipt.reported_effort,
        });
    }
    pub(crate) fn finish(self) -> ApiSummary {
        let amount: Vec<_> = self
            .fees
            .into_iter()
            .map(|(currency, value)| CurrencyAmount {
                currency,
                amount: value.text(),
            })
            .collect();
        let fee_status = if self.overflow {
            Completeness::Unknown
        } else {
            complete_status(self.count, self.fee_count, self.unknown)
        };
        ApiSummary {
            call_count: (!self.unknown).then_some(self.count),
            observed_call_count: self.count,
            input: self.input.finish(self.unknown),
            output: self.output.finish(self.unknown),
            fees: FeeTotal {
                status: fee_status,
                totals: if fee_status == Completeness::Complete {
                    amount.clone()
                } else {
                    vec![]
                },
                known_subtotals: if self.overflow { vec![] } else { amount },
                reported_calls: self.fee_count,
            },
            requested_models: self.requested.into_iter().collect(),
            reported_models: self.reported.into_iter().collect(),
            model_efforts: self.pairs.into_iter().collect(),
        }
    }
}
fn complete_status(count: u64, reported: u64, unknown: bool) -> Completeness {
    if count == 0 && !unknown {
        Completeness::NotApplicable
    } else if reported == 0 {
        Completeness::Unknown
    } else if reported == count && !unknown {
        Completeness::Complete
    } else {
        Completeness::Partial
    }
}
struct RunningUsage {
    count: u64,
    reported: u64,
    sum: Option<u64>,
}
impl Default for RunningUsage {
    fn default() -> Self {
        Self {
            count: 0,
            reported: 0,
            sum: Some(0),
        }
    }
}
impl RunningUsage {
    fn add(&mut self, value: Option<u64>) {
        self.count += 1;
        if let Some(value) = value {
            self.reported += 1;
            self.sum = self
                .sum
                .and_then(|sum| sum.checked_add(value))
                .filter(|sum| *sum <= MAX_USAGE);
        }
    }
    fn finish(self, unknown: bool) -> UsageTotal {
        let status = if self.sum.is_none() {
            Completeness::Unknown
        } else {
            complete_status(self.count, self.reported, unknown)
        };
        UsageTotal {
            status,
            total: if status == Completeness::Complete {
                self.sum
            } else {
                None
            },
            known_subtotal: if self.reported > 0 { self.sum } else { None },
            reported_calls: self.reported,
        }
    }
}

/// Stable research-count definition used by persisted observations and CSV.
pub const COUNT_RULE_VERSION: &str = "zh-en-v1";

/// Count Han individually and other alphanumeric word runs once. Internal
/// apostrophes join a word; hyphens/underscores separate words. Whitespace,
/// punctuation, emoji and combining marks add no unit. This is not token usage.
pub fn mixed_count(text: &str) -> u64 {
    let mut count = 0;
    let mut word = false;
    let mut characters = text.chars().peekable();
    while let Some(character) = characters.next() {
        if is_han(character) {
            count += u64::from(word) + 1;
            word = false;
        } else if is_mark(character) {
            continue;
        } else if character.is_alphanumeric()
            || matches!(character, '\'' | '’')
                && word
                && characters
                    .peek()
                    .is_some_and(|next| next.is_alphanumeric() && !is_han(*next))
        {
            word = true;
        } else {
            count += u64::from(word);
            word = false;
        }
    }
    count + u64::from(word)
}

fn is_han(character: char) -> bool {
    matches!(character as u32, 0x3006..=0x3007)
        || character.is_alphabetic()
            && matches!(character as u32, 0x3400..=0x4dbf | 0x4e00..=0x9fff | 0xf900..=0xfaff | 0x20000..=0x2fa1f | 0x30000..=0x33fff)
}

fn is_mark(character: char) -> bool {
    matches!(character as u32, 0x0300..=0x036f | 0x1ab0..=0x1aff | 0x1dc0..=0x1dff | 0x20d0..=0x20ff | 0xfe00..=0xfe0f | 0xfe20..=0xfe2f | 0xe0100..=0xe01ef)
}
