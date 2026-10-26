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
impl ExperimentProcessor {
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
    let (_, _, latch) = policy.parts();
    let mut inner = ChatProcessor::new_with_policy(configuration, true)?;
    inner.tracing = tracing;
    Ok(ExperimentProcessor { inner, latch })
}
