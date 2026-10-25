//! Typed API tracing. This module owns no request, key, model, URL or text body.
use crate::capture_startup::{CaptureClock, CollectionToken, MonotonicClock};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Instant;

macro_rules! labels {
    ($name:ident { $($variant:ident => $label:literal),+ $(,)? }) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        #[doc = "Closed diagnostic vocabulary; contains no caller text."]
        pub enum $name { $(#[doc = concat!("Fixed code `", $label, "`.")] $variant),+ }
        impl $name { #[doc = "Fixed report label."] pub fn label(self) -> &'static str { match self { $(Self::$variant => $label),+ } } }
    };
}
labels!(ApiSpan { Prepared => "prepared", Admitted => "admitted", SendStarted => "send_started", HeadersReceived => "headers_received", BodyRead => "body_read", ResponseClassified => "response_classified", Finished => "finished" });
labels!(ApiResult { Started => "started", Success => "success", Failed => "failed", Timeout => "timeout", Cancelled => "cancelled", Interrupted => "interrupted" });
labels!(ApiErrorCategory { None => "none", Configuration => "configuration", InvalidPrompt => "invalid_prompt", HistoryStorage => "history_storage", Auth => "auth", Permission => "permission", Balance => "balance", RateLimit => "rate_limit", Context => "context", Validation => "validation", Unavailable => "unavailable", Transport => "transport", Protocol => "protocol", Oversize => "oversize", Timeout => "timeout", Cancelled => "cancelled", Unknown => "unknown" });
labels!(ApiPurpose { Generation => "generation", BackTranslation => "back_translation", ModelList => "model_list" });
labels!(ApiMethod { Post => "post", Get => "get" });
labels!(ApiScenario { Dictation => "dictation", Translation => "translation", Ask => "ask", Metadata => "metadata" });
labels!(AuthKind { None => "none", Bearer => "bearer", Unspecified => "unspecified" });
labels!(CredentialSource { App => "app", HomeEnv => "home_env", None => "none", Unknown => "unknown" });
labels!(ApiProfile { Generic => "generic", DeepSeek => "deepseek", OpenRouter => "openrouter", Gemini => "gemini", OllamaOpenAi => "ollama_openai" });
labels!(EndpointClass { PublicPreset => "public_preset", Loopback => "loopback", Other => "other" });
labels!(Effort { Omitted => "omitted", None => "none", Minimal => "minimal", Low => "low", Medium => "medium", High => "high", Xhigh => "xhigh", Other => "other" });
labels!(TimeoutSource { None => "none", LocalDeadline => "local_deadline", ProviderStatus => "provider_status", TransportPredicate => "transport_predicate", Unknown => "unknown" });

/// Safe immutable request projection; strings cannot enter this boundary.
#[derive(Clone, Copy, Debug)]
pub struct SafeOptions {
    /// Actual scenario, distinct from transport purpose and user labels.
    pub scenario: ApiScenario,
    /// Actual request purpose, including metadata GET without generation ledger rows.
    pub purpose: ApiPurpose,
    /// Actual HTTP method.
    pub method: ApiMethod,
    /// Explicit profile, never inferred from a display name.
    pub profile: ApiProfile,
    /// Coarse route class without destination text.
    pub endpoint: EndpointClass,
    /// Actual auth policy without credential fragments.
    pub auth: AuthKind,
    /// Already-resolved credential provenance; never triggers credential reads.
    pub credential_source: CredentialSource,
    /// Whether a credential was resolved for this operation.
    pub credential_present: bool,
    /// Whether a model was configured, without copying its identifier.
    pub model_present: bool,
    /// Finite projection of actual wire effort.
    pub effort: Effort,
    /// Effective request switch, absent when omitted or unknown.
    pub thinking: Option<bool>,
    /// Actual bounded output limit, absent when omitted.
    pub max_output_tokens: Option<u32>,
    /// Configured whole deadline, clamped at the report boundary.
    pub timeout_ms: u64,
    /// Serialized request bytes; decoded response bytes have separate semantics.
    pub request_bytes: u64,
    /// Actual message count, never the messages.
    pub message_count: u32,
    /// Explicitly nonstreaming operation.
    pub non_streaming: bool,
    /// Whether actual messages were plain text without tools.
    pub plain_text_only: bool,
}

/// Local numeric associations only; no durable/account/provider identity.
#[derive(Clone, Copy, Debug, Default)]
pub struct ApiScope {
    /// Process-local session surrogate, scoped to collection generation.
    pub session: Option<u32>,
    /// Actual accepted observer revision, never a guessed next revision.
    pub revision: Option<u64>,
    /// Process-local connection surrogate, never its stored ID/name/address.
    pub connection: Option<u32>,
}
/// Generation-fenced workflow association, allocated only while collection is ON.
#[derive(Clone, Copy, Debug)]
pub struct ScopeBinding {
    /// Clear/preference generation which admitted this association.
    pub token: CollectionToken,
    /// Numeric surrogate projection.
    pub scope: ApiScope,
    /// Earlier registry eviction means correlation cannot be assumed persistent.
    pub correlation_reassigned: bool,
}
/// One admitted diagnostic call. It cannot authorize HTTP or business spending.
#[derive(Clone, Copy, Debug)]
pub struct TraceIdentity {
    /// Immutable collection generation fence.
    pub binding: ScopeBinding,
    /// Nonzero call ordinal within this generation.
    pub call: u32,
}
/// Typed event, capped before insertion into the shared 128-row ring.
#[derive(Clone, Copy, Debug)]
pub struct ApiEvent {
    /// Safe identity allocated under the service admission lock.
    pub identity: TraceIdentity,
    /// Request metadata projection, without strings.
    pub options: SafeOptions,
    /// Last actual observation boundary.
    pub span: ApiSpan,
    /// Last observed nonterminal stage, including for interrupted Drop.
    pub last_span: ApiSpan,
    /// Fixed local disposition, not a remote billing assertion.
    pub result: ApiResult,
    /// Fixed category; never an upstream error message.
    pub error: ApiErrorCategory,
    /// Milliseconds since prepared in this monotonic clock domain.
    pub call_elapsed_ms: u64,
    /// Local observed span duration, not DNS/TLS/server time.
    pub duration_ms: Option<u64>,
    /// A duration exceeded the 24-hour bound.
    pub capped: bool,
    /// Actual received status, separate from projected error HTTP status.
    pub http_status: Option<u16>,
    /// Decoded bytes delivered by reqwest, saturated at u32::MAX.
    pub body_bytes: u64,
    /// Byte counter saturated; separate from parser cap and report truncation.
    pub bytes_capped: bool,
    /// Complete body reached EOF, not proof of valid application text.
    pub body_complete: bool,
    /// Actual parser body bound was crossed.
    pub body_cap_hit: bool,
    /// Existing durable observer admitted the operation, absent for direct calls.
    pub durable_admission: bool,
    /// Existing mark-sent succeeded; this is not proof bytes reached a server.
    pub possibly_sent: bool,
    /// Actual local cancellation token state at this observation.
    pub cancel_requested: bool,
    /// Observable timeout source, never parsed from an error string.
    pub timeout_source: TimeoutSource,
    /// reqwest typed is_connect predicate where available.
    pub connect_related: Option<bool>,
    /// Response model presence only.
    pub reported_model_present: bool,
    /// Exact comparison while both model strings are already owned by provider.
    pub model_matches_requested: Option<bool>,
}
/// Collector adapter. Both allocation and every insertion must fence generation.
pub trait ApiObserver: Send + Sync {
    /// Atomically allocate only if the optional originating generation is current.
    fn begin(&self, scope: Option<ScopeBinding>, revision: Option<u64>) -> Option<TraceIdentity>;
    /// Accept only the original active generation; failure never affects HTTP.
    fn record(&self, event: ApiEvent);
}
/// Read-only local cancellation predicate; never implies remote cancellation.
pub trait CancellationFlag: Send + Sync {
    /// Actual cancellation state at observation/drop.
    fn cancelled(&self) -> bool;
}
struct NeverCancelled;
impl CancellationFlag for NeverCancelled {
    fn cancelled(&self) -> bool {
        false
    }
}

/// Production clock/collector composition shared by direct, workflow and owned
/// adapter callers. Replacing these ports never replaces transport/accounting.
#[derive(Clone)]
pub struct TraceContext {
    scope: Option<ScopeBinding>,
    clock: Arc<dyn CaptureClock>,
    observer: Arc<dyn ApiObserver>,
}
impl TraceContext {
    /// Use the process-owned service and monotonic backend clock.
    pub fn global(scope: Option<ScopeBinding>) -> Self {
        Self::new(scope, Arc::new(MonotonicClock), Arc::new(GlobalObserver))
    }
    /// Explicit port composition; scope admission remains the observer's job.
    pub fn new(
        scope: Option<ScopeBinding>,
        clock: Arc<dyn CaptureClock>,
        observer: Arc<dyn ApiObserver>,
    ) -> Self {
        Self {
            scope,
            clock,
            observer,
        }
    }
    /// Begin at actual serialized-wire boundary, without business admission.
    pub fn begin(
        &self,
        options: SafeOptions,
        revision: Option<u64>,
        cancellation: Arc<dyn CancellationFlag>,
    ) -> CallTrace {
        CallTrace::begin(
            self.scope,
            options,
            revision,
            Arc::clone(&self.clock),
            Arc::clone(&self.observer),
            cancellation,
        )
    }
}

/// In-memory source-ID association owned by the service adapter, outside the
/// typed collector. IDs never implement Debug/Serialize and are capped at128.
pub(crate) struct ApiRegistry {
    token: Option<CollectionToken>,
    sessions: VecDeque<(String, u32)>,
    session_sequence: u32,
    call_sequence: u32,
    reassigned: bool,
}
impl ApiRegistry {
    pub(crate) fn new() -> Self {
        Self {
            token: None,
            sessions: VecDeque::new(),
            session_sequence: 0,
            call_sequence: 0,
            reassigned: false,
        }
    }
    fn reset(&mut self, token: CollectionToken) {
        if self.token != Some(token) {
            *self = Self::new();
            self.token = Some(token);
        }
    }
    pub(crate) fn scope(
        &mut self,
        token: CollectionToken,
        source: Option<&str>,
        connection: Option<u32>,
    ) -> ScopeBinding {
        self.reset(token);
        let session = source
            .filter(|source| source.len() <= 128)
            .and_then(|source| {
                if let Some((_, ordinal)) = self.sessions.iter().find(|(id, _)| id == source) {
                    return Some(*ordinal);
                }
                let ordinal = self.session_sequence.checked_add(1)?;
                self.session_sequence = ordinal;
                if self.sessions.len() == 128 {
                    self.sessions.pop_front();
                    self.reassigned = true;
                }
                self.sessions.push_back((source.to_owned(), ordinal));
                Some(ordinal)
            });
        ScopeBinding {
            token,
            scope: ApiScope {
                session,
                revision: None,
                connection,
            },
            correlation_reassigned: self.reassigned,
        }
    }
    pub(crate) fn call(
        &mut self,
        token: CollectionToken,
        binding: Option<ScopeBinding>,
        revision: Option<u64>,
    ) -> Option<TraceIdentity> {
        self.reset(token);
        let mut binding = binding.unwrap_or(ScopeBinding {
            token,
            scope: ApiScope::default(),
            correlation_reassigned: false,
        });
        if binding.token != token {
            return None;
        }
        binding.scope.revision = revision.filter(|revision| *revision > 0);
        let call = self.call_sequence.checked_add(1)?;
        self.call_sequence = call;
        Some(TraceIdentity { binding, call })
    }
}

/// Exactly-once local trace. Drop records interrupted if the provider future is
/// dropped by its caller; diagnostics never own or change the business attempt.
pub struct CallTrace {
    event: Option<ApiEvent>,
    clock: Arc<dyn CaptureClock>,
    observer: Arc<dyn ApiObserver>,
    cancellation: Arc<dyn CancellationFlag>,
    started: Instant,
    span_started: Instant,
    body_observed: bool,
    terminal: bool,
    emitted: u8,
}
impl CallTrace {
    /// Begin after wire serialization; disabled admission creates an inert handle
    /// without allocating IDs or registry entries. Tests inject clock/sink ports.
    pub fn begin(
        scope: Option<ScopeBinding>,
        options: SafeOptions,
        revision: Option<u64>,
        clock: Arc<dyn CaptureClock>,
        observer: Arc<dyn ApiObserver>,
        cancellation: Arc<dyn CancellationFlag>,
    ) -> Self {
        let started = clock.now();
        let event = observer.begin(scope, revision).map(|identity| ApiEvent {
            identity,
            options,
            span: ApiSpan::Prepared,
            last_span: ApiSpan::Prepared,
            result: ApiResult::Started,
            error: ApiErrorCategory::None,
            call_elapsed_ms: 0,
            duration_ms: None,
            capped: false,
            http_status: None,
            body_bytes: 0,
            bytes_capped: false,
            body_complete: false,
            body_cap_hit: false,
            durable_admission: false,
            possibly_sent: false,
            cancel_requested: false,
            timeout_source: TimeoutSource::None,
            connect_related: None,
            reported_model_present: false,
            model_matches_requested: None,
        });
        let mut trace = Self {
            event,
            clock,
            observer,
            cancellation,
            started,
            span_started: started,
            body_observed: false,
            terminal: false,
            emitted: 0,
        };
        trace.observe(ApiSpan::Prepared, ApiResult::Started);
        trace
    }
    /// Production direct-call constructor; associated workflow calls pass binding.
    pub fn global(
        scope: Option<ScopeBinding>,
        options: SafeOptions,
        revision: Option<u64>,
        cancellation: Arc<dyn CancellationFlag>,
    ) -> Self {
        Self::begin(
            scope,
            options,
            revision,
            Arc::new(MonotonicClock),
            Arc::new(GlobalObserver),
            cancellation,
        )
    }
    /// Actual observed stage. No per-chunk call is needed or permitted by transport.
    pub fn observe(&mut self, span: ApiSpan, result: ApiResult) {
        if self.terminal || self.emitted >= 8 || (span != ApiSpan::Finished && self.emitted >= 7) {
            return;
        }
        if let Some(event) = &mut self.event {
            let now = self.clock.now();
            let elapsed = now
                .saturating_duration_since(self.started)
                .as_millis()
                .min(u128::from(u64::MAX)) as u64;
            let duration = now
                .saturating_duration_since(self.span_started)
                .as_millis()
                .min(u128::from(u64::MAX)) as u64;
            if span != ApiSpan::Finished {
                event.last_span = span;
            }
            event.span = span;
            event.result = result;
            event.call_elapsed_ms = elapsed.min(86_400_000);
            event.duration_ms = Some(duration.min(86_400_000));
            event.capped |= elapsed > 86_400_000 || duration > 86_400_000;
            event.cancel_requested = self.cancellation.cancelled();
            self.observer.record(*event);
            self.emitted += 1;
            self.span_started = now;
        }
    }
    /// Update fixed flags before their corresponding stage observation.
    pub fn metadata(&mut self) -> Option<&mut ApiEvent> {
        self.event.as_mut()
    }
    /// Aggregate decoded body count exactly once, including partial/capped bodies.
    pub fn body(&mut self, bytes: u64, complete: bool, cap_hit: bool) {
        if self.body_observed {
            return;
        }
        self.body_observed = true;
        if let Some(event) = &mut self.event {
            event.body_bytes = bytes.min(u64::from(u32::MAX));
            event.bytes_capped = bytes > u64::from(u32::MAX);
            event.body_complete = complete;
            event.body_cap_hit = cap_hit;
        }
        self.observe(
            ApiSpan::BodyRead,
            if complete {
                ApiResult::Success
            } else {
                ApiResult::Failed
            },
        );
    }
    /// Exactly one local terminal. Never asserts remote cancellation or zero cost.
    pub fn finish(&mut self, result: ApiResult, error: ApiErrorCategory) {
        if self.terminal {
            return;
        }
        if let Some(event) = &mut self.event {
            event.error = error;
        }
        if self.event.is_some_and(|event| event.http_status.is_some()) && !self.body_observed {
            self.body(self.event.map_or(0, |event| event.body_bytes), false, false);
        }
        if result != ApiResult::Interrupted
            && self.event.is_some_and(|event| {
                event.body_complete && event.span != ApiSpan::ResponseClassified
            })
        {
            self.observe(ApiSpan::ResponseClassified, result);
        }
        self.observe(ApiSpan::Finished, result);
        self.terminal = true;
    }
    /// Inert fallback cancellation port for metadata-only caller composition.
    pub fn never_cancelled() -> Arc<dyn CancellationFlag> {
        Arc::new(NeverCancelled)
    }
}
impl Drop for CallTrace {
    fn drop(&mut self) {
        self.finish(ApiResult::Interrupted, ApiErrorCategory::Unknown);
    }
}
struct GlobalObserver;
impl ApiObserver for GlobalObserver {
    fn begin(&self, scope: Option<ScopeBinding>, revision: Option<u64>) -> Option<TraceIdentity> {
        super::service::global().begin_api(scope, revision)
    }
    fn record(&self, event: ApiEvent) {
        super::service::global().record_api(event);
    }
}
