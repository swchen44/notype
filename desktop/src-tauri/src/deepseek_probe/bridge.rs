//! Composes the actual provider's request observer with durable single-round
//! admission. This wrapper is used through SessionService, never as a second HTTP client.
use super::{
    policy::{Denied, WireFacts},
    round::{Outcome, Round, RoundStore, SendPermit},
};
use notype_core::{
    observations::{
        ApiCallDescriptor, ApiCallObserver, ApiEndpointClass, ApiOutcome, ApiProviderProfile,
        ApiPurpose, ApiReceipt,
    },
    BackTranslationRequest, CancellationToken, GenerationFuture, TextFailure, TextFuture,
    TextProcessor, TextRequest,
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

#[derive(Clone)]
enum Target {
    Live,
    #[cfg(test)]
    Fixture(String),
}
impl Target {
    fn endpoint(&self) -> &str {
        match self {
            Self::Live => "https://api.deepseek.com/chat/completions",
            #[cfg(test)]
            Self::Fixture(endpoint) => endpoint,
        }
    }
    fn matches(&self, class: ApiEndpointClass) -> bool {
        match self {
            Self::Live => class == ApiEndpointClass::DeepSeekHttps,
            #[cfg(test)]
            Self::Fixture(_) => class == ApiEndpointClass::Loopback,
        }
    }
    fn admit<S: RoundStore>(
        &self,
        round: &mut Round<S>,
        slot: u8,
        history: &str,
        call: &str,
        wire: &WireFacts<'_>,
    ) -> Result<SendPermit, Denied> {
        match self {
            Self::Live => round.admit(slot, history, call, wire),
            #[cfg(test)]
            Self::Fixture(_) => round.admit_fixture(slot, history, call, wire),
        }
    }
}

/// One fixed history group's guarded provider. Only observed SessionService
/// calls may spend; ordinary non-observed TextProcessor entry points fail closed.
pub struct BudgetProcessor<S: RoundStore> {
    inner: Arc<dyn TextProcessor>,
    round: Arc<Mutex<Round<S>>>,
    history: String,
    slot: u8,
    target: Target,
    cancellation_requested: Arc<AtomicBool>,
    sent: Arc<tokio::sync::Notify>,
}
impl<S: RoundStore + Send + 'static> BudgetProcessor<S> {
    /// Fixed public HTTPS target; there is no runtime URL or environment override.
    pub fn live(
        inner: Arc<dyn TextProcessor>,
        round: Arc<Mutex<Round<S>>>,
        history: String,
        slot: u8,
    ) -> Result<Arc<Self>, Denied> {
        Self::new(inner, round, history, slot, Target::Live)
    }
    /// Explicit owned loopback substitution available only in test compilation.
    /// It preserves the actual endpoint class and never claims live HTTPS evidence.
    #[cfg(test)]
    pub fn for_fixture(
        inner: Arc<dyn TextProcessor>,
        round: Arc<Mutex<Round<S>>>,
        history: String,
        slot: u8,
        endpoint: &str,
    ) -> Result<Arc<Self>, Denied> {
        let port = endpoint
            .strip_prefix("http://127.0.0.1:")
            .and_then(|value| value.strip_suffix("/chat/completions"))
            .and_then(|value| value.parse::<u16>().ok())
            .filter(|port| *port > 0);
        if port.is_none() {
            return Err(Denied::WireContract);
        }
        Self::new(
            inner,
            round,
            history,
            slot,
            Target::Fixture(endpoint.into()),
        )
    }
    fn new(
        inner: Arc<dyn TextProcessor>,
        round: Arc<Mutex<Round<S>>>,
        history: String,
        slot: u8,
        target: Target,
    ) -> Result<Arc<Self>, Denied> {
        if !(1..=8).contains(&slot) || slot == 6 || !super::round::valid_id(&history) {
            return Err(Denied::Schedule);
        }
        Ok(Arc::new(Self {
            inner,
            round,
            history,
            slot,
            target,
            cancellation_requested: Arc::new(AtomicBool::new(false)),
            sent: Arc::new(tokio::sync::Notify::new()),
        }))
    }
    /// Wait for durable reservation and the core possibly-sent boundary.
    /// A notification does not prove server acceptance or a charge.
    pub async fn wait_until_sent(&self) {
        self.sent.notified().await;
    }
    /// Mark the schedule's intentional slot-7 cancellation before core cancellation.
    /// An arbitrary dropped future is not converted into expected cancellation.
    pub fn request_cancellation(&self) -> Result<(), Denied> {
        if self.slot != 7 {
            return Err(Denied::Schedule);
        }
        self.cancellation_requested.store(true, Ordering::SeqCst);
        Ok(())
    }
    fn observer(&self, core: Arc<dyn ApiCallObserver>, slot: u8) -> Arc<dyn ApiCallObserver> {
        Arc::new(BudgetObserver {
            core,
            round: self.round.clone(),
            history: self.history.clone(),
            slot,
            target: self.target.clone(),
            call: Mutex::new(None),
            cancellation_requested: self.cancellation_requested.clone(),
            sent: self.sent.clone(),
        })
    }
}
impl<S: RoundStore + Send + 'static> TextProcessor for BudgetProcessor<S> {
    fn polish(&self, _: TextRequest, _: CancellationToken) -> TextFuture {
        Box::pin(async { Err(TextFailure::HistoryStorage) })
    }
    fn generate_observed(
        &self,
        request: TextRequest,
        cancel: CancellationToken,
        observer: Arc<dyn ApiCallObserver>,
    ) -> GenerationFuture {
        self.inner
            .generate_observed(request, cancel, self.observer(observer, self.slot))
    }
    fn back_translate_observed(
        &self,
        request: BackTranslationRequest,
        cancel: CancellationToken,
        observer: Arc<dyn ApiCallObserver>,
    ) -> TextFuture {
        if self.slot != 5 {
            return Box::pin(async { Err(TextFailure::HistoryStorage) });
        }
        self.inner
            .back_translate_observed(request, cancel, self.observer(observer, 6))
    }
}

struct Call {
    id: String,
    descriptor: ApiCallDescriptor,
    permit: Option<SendPermit>,
    valid_receipt: Option<bool>,
}
struct BudgetObserver<S: RoundStore> {
    core: Arc<dyn ApiCallObserver>,
    round: Arc<Mutex<Round<S>>>,
    history: String,
    slot: u8,
    target: Target,
    call: Mutex<Option<Call>>,
    cancellation_requested: Arc<AtomicBool>,
    sent: Arc<tokio::sync::Notify>,
}
fn failure() -> TextFailure {
    TextFailure::HistoryStorage
}
fn receipt_valid(receipt: &ApiReceipt) -> bool {
    let (Some(input), Some(output), Some(total)) = (
        receipt.usage.input,
        receipt.usage.output,
        receipt.total_tokens,
    ) else {
        return false;
    };
    receipt.reported_model.as_deref() == Some("deepseek-flash")
        && input <= 32768
        && output <= 512
        && input.checked_add(output) == Some(total)
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
        }
}
impl<S: RoundStore + Send> BudgetObserver<S> {
    fn halt(&self) {
        if let Ok(mut round) = self.round.lock() {
            if !round.report().halted {
                let _ = round.halt();
            }
        }
    }
    fn wire<'a>(&'a self, d: &'a ApiCallDescriptor) -> WireFacts<'a> {
        WireFacts {
            endpoint: self.target.endpoint(),
            model: d.requested_model.as_deref().unwrap_or(""),
            deepseek_profile: d.profile == ApiProviderProfile::DeepSeek,
            effort: d.requested_effort.as_deref(),
            thinking: d.thinking,
            non_streaming: d.non_streaming,
            max_output_tokens: d.max_output_tokens,
            wire_bytes: d.wire_bytes,
            message_count: d.message_count,
            plain_text_only: d.plain_text_only,
        }
    }
}
impl<S: RoundStore + Send> ApiCallObserver for BudgetObserver<S> {
    fn begin(&self, descriptor: ApiCallDescriptor) -> Result<String, TextFailure> {
        let expected = if self.slot == 6 {
            ApiPurpose::BackTranslation
        } else {
            ApiPurpose::Generation
        };
        if descriptor.purpose != expected || !self.target.matches(descriptor.endpoint_class) {
            self.halt();
            return Err(failure());
        }
        {
            let round = self.round.lock().map_err(|_| failure())?;
            if round.report().halted || round.report().report_only {
                return Err(failure());
            }
        }
        let mut call = self.call.lock().map_err(|_| failure())?;
        if call.is_some() {
            self.halt();
            return Err(failure());
        }
        let id = self
            .core
            .begin(descriptor.clone())
            .inspect_err(|_| self.halt())?;
        *call = Some(Call {
            id: id.clone(),
            descriptor,
            permit: None,
            valid_receipt: None,
        });
        Ok(id)
    }
    fn mark_sent(&self, id: &str) -> Result<(), TextFailure> {
        let mut holder = self.call.lock().map_err(|_| failure())?;
        let call = holder
            .as_mut()
            .filter(|call| call.id == id && call.permit.is_none())
            .ok_or_else(failure)?;
        let permit = {
            let mut round = self.round.lock().map_err(|_| failure())?;
            self.target
                .admit(
                    &mut round,
                    self.slot,
                    &self.history,
                    id,
                    &self.wire(&call.descriptor),
                )
                .map_err(|_| failure())?
        };
        call.permit = Some(permit);
        self.core.mark_sent(id).inspect_err(|_| self.halt())?;
        self.sent.notify_one();
        Ok(())
    }
    fn response(&self, id: &str, receipt: ApiReceipt) {
        self.core.response(id, receipt.clone());
        let valid = receipt_valid(&receipt);
        match self.call.lock() {
            Ok(mut call) => {
                if let Some(call) = call.as_mut().filter(|call| call.id == id) {
                    call.valid_receipt = Some(valid);
                } else {
                    self.halt();
                }
            }
            Err(_) => self.halt(),
        }
        if !valid {
            self.halt();
        }
    }
    fn finish(&self, id: &str, outcome: ApiOutcome, elapsed_ms: u64) {
        self.core.finish(id, outcome, elapsed_ms);
        let Ok(mut holder) = self.call.lock() else {
            self.halt();
            return;
        };
        let Some(call) = holder.as_mut().filter(|call| call.id == id) else {
            self.halt();
            return;
        };
        let Some(permit) = call.permit.take() else {
            self.halt();
            return;
        };
        let outcome = if call.valid_receipt == Some(false) {
            Outcome::ContractViolation
        } else if outcome == ApiOutcome::Success && call.valid_receipt == Some(true) {
            if self.slot == 7 {
                Outcome::CancelRacedWithCompletion
            } else {
                Outcome::Success
            }
        } else if self.slot == 7
            && self.cancellation_requested.load(Ordering::SeqCst)
            && matches!(outcome, ApiOutcome::Cancelled | ApiOutcome::Interrupted)
        {
            Outcome::IntentionalCancellation
        } else {
            Outcome::Failed
        };
        if let Ok(mut round) = self.round.lock() {
            let _ = round.finish(&permit, outcome);
        }
    }
    fn unavailable(&self) {
        self.core.unavailable();
        self.halt();
    }
}
