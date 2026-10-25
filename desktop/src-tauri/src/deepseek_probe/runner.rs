//! Fixed synthetic schedule through the real SessionService. There is no audio,
//! target delivery, configurable sample, automatic recovery or extra paid call.
use super::{
    bridge::{BudgetProcessor, Target},
    policy::{schedule, Denied, Purpose},
    round::{Round, RoundStore},
};
use notype_core::{
    Scenario, SessionEntry, SessionOptions, SessionService, SessionStatus, TextProcessor,
};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

/// Fixed, secret-free runner failures; raw transport/store errors are excluded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunFailure {
    /// The round is reopened, halted or otherwise unavailable.
    NotReady,
    /// The preceding result needs local quality acceptance before continuing.
    ReviewRequired,
    /// The local reviewer rejected an output; no further calls are permitted.
    Quality,
    /// A provider result failed; no retry is added beyond the fixed schedule.
    Provider,
    /// Durable storage/ownership or task completion failed.
    Storage,
}
impl RunFailure {
    /// A stable report code that never includes a credential, path or raw error.
    pub fn code(self) -> &'static str {
        match self {
            Self::NotReady => "probe-not-ready",
            Self::ReviewRequired => "probe-review-required",
            Self::Quality => "probe-quality-rejected",
            Self::Provider => "probe-provider-failed",
            Self::Storage => "probe-storage-failed",
        }
    }
}

/// One locally reviewable step. The forward/reverse pair is a single step,
/// and the cancellation/retry steps intentionally refer to the same history.
pub struct Step {
    /// First fixed request slot represented by the step.
    pub first_slot: u8,
    /// Last fixed request slot, including a successful automatic reverse call.
    pub last_slot: u8,
    /// Actual durable history with synthetic original/result and usage summary.
    pub history: SessionEntry,
}

/// Serial one-round schedule. Every next step requires accepting the previous
/// local output. This does not replace the App's revision-bound delivery review.
pub struct ProbeRunner<S: RoundStore> {
    sessions: SessionService,
    round: Arc<Mutex<Round<S>>>,
    inner: Arc<dyn TextProcessor>,
    target: Target,
    next_slot: u8,
    retry_history: Option<String>,
    awaiting_review: bool,
    stopped: bool,
}
impl<S: RoundStore + Send + 'static> ProbeRunner<S> {
    /// Build the fixed public HTTPS schedule. The caller must already own and arm
    /// the fresh round after local key entry and final confirmation.
    pub fn live(
        sessions: SessionService,
        round: Arc<Mutex<Round<S>>>,
        inner: Arc<dyn TextProcessor>,
    ) -> Result<Self, Denied> {
        Self::new(sessions, round, inner, Target::Live)
    }
    /// Owned loopback schedule available only in test compilation, never the helper.
    #[cfg(test)]
    pub fn for_fixture(
        sessions: SessionService,
        round: Arc<Mutex<Round<S>>>,
        inner: Arc<dyn TextProcessor>,
        endpoint: &str,
    ) -> Result<Self, Denied> {
        Self::new(sessions, round, inner, Target::fixture(endpoint)?)
    }
    fn new(
        sessions: SessionService,
        round: Arc<Mutex<Round<S>>>,
        inner: Arc<dyn TextProcessor>,
        target: Target,
    ) -> Result<Self, Denied> {
        {
            let state = round.lock().map_err(|_| Denied::Storage)?.report();
            if state.report_only {
                return Err(Denied::ReportOnly);
            }
            if state.halted || state.reserved_slots != 0 {
                return Err(Denied::Halted);
            }
        }
        Ok(Self {
            sessions,
            round,
            inner,
            target,
            next_slot: 1,
            retry_history: None,
            awaiting_review: false,
            stopped: false,
        })
    }
    /// Accept or reject the preceding synthetic output. Rejection persists a halt;
    /// it never refunds a reservation or starts a replacement request.
    pub fn accept_output(&mut self, accepted: bool) -> Result<(), RunFailure> {
        if !self.awaiting_review || self.stopped {
            return Err(RunFailure::NotReady);
        }
        self.awaiting_review = false;
        if !accepted {
            self.stop()?;
            return Err(RunFailure::Quality);
        }
        Ok(())
    }
    fn stop(&mut self) -> Result<(), RunFailure> {
        self.stopped = true;
        let mut round = self.round.lock().map_err(|_| RunFailure::Storage)?;
        if !round.report().halted {
            round.halt().map_err(|_| RunFailure::Storage)?;
        }
        Ok(())
    }
    /// Process exactly the next approved step. There is no automatic retry after
    /// failure. Optional back-translation uses SessionService's real reverse path.
    pub async fn next(&mut self) -> Result<Option<Step>, RunFailure> {
        if self.stopped {
            return Err(RunFailure::NotReady);
        }
        if self.awaiting_review {
            return Err(RunFailure::ReviewRequired);
        }
        if self.next_slot > 8 {
            return Ok(None);
        }
        if self
            .round
            .lock()
            .map_err(|_| RunFailure::Storage)?
            .report()
            .halted
        {
            self.stopped = true;
            return Err(RunFailure::NotReady);
        }
        let slot = self.next_slot;
        let spec = &schedule()[usize::from(slot - 1)];
        let id = if slot == 8 {
            self.retry_history.clone().ok_or(RunFailure::Storage)?
        } else {
            let scenario = match spec.purpose {
                Purpose::Dictation => Scenario::Dictation,
                Purpose::Translation | Purpose::ForwardReview => Scenario::Translation,
                _ => Scenario::Ask,
            };
            match self.sessions.create(
                spec.sample,
                SessionOptions {
                    scenario,
                    translation_target: Some("en-US".into()),
                    ..Default::default()
                },
            ) {
                Ok(entry) => entry.id,
                Err(_) => {
                    self.stop()?;
                    return Err(RunFailure::Storage);
                }
            }
        };
        let processor = BudgetProcessor::new(
            self.inner.clone(),
            self.round.clone(),
            id.clone(),
            slot,
            self.target.clone(),
        )
        .map_err(|_| RunFailure::Storage)?;
        let processed = if slot == 7 {
            self.process_cancellation(&id, processor).await
        } else {
            self.sessions
                .process_with_review(&id, vec![], processor, spec.review)
                .await
                .map_err(|_| RunFailure::Storage)
        };
        let history = match processed {
            Ok(entry) => entry,
            Err(error) => {
                self.stop()?;
                return Err(error);
            }
        };
        let halted = self
            .round
            .lock()
            .map_err(|_| RunFailure::Storage)?
            .report()
            .halted;
        if halted
            || (slot != 7 && (history.status != SessionStatus::Pending || history.error.is_some()))
            || (slot == 5 && history.back_translation.is_none())
        {
            self.stop()?;
            return Err(RunFailure::Provider);
        }
        if slot == 1 && history.result.as_deref().map(str::trim) != Some("OK") {
            self.stop()?;
            return Err(RunFailure::Quality);
        }
        if slot == 7 {
            self.retry_history = Some(id);
        }
        let last_slot = if slot == 5 { 6 } else { slot };
        self.next_slot = last_slot + 1;
        self.awaiting_review = true;
        Ok(Some(Step {
            first_slot: slot,
            last_slot,
            history,
        }))
    }
    async fn process_cancellation(
        &self,
        id: &str,
        processor: Arc<BudgetProcessor<S>>,
    ) -> Result<SessionEntry, RunFailure> {
        let sessions = self.sessions.clone();
        let owned_id = id.to_owned();
        let running_processor = processor.clone();
        let mut task = tokio::spawn(async move {
            sessions
                .process_with_review(&owned_id, vec![], running_processor, false)
                .await
        });
        tokio::select! {
            result=&mut task=>{
                let entry=result.map_err(|_|RunFailure::Storage)?.map_err(|_|RunFailure::Storage)?;
                if entry.status==SessionStatus::Pending&&entry.error.is_none(){processor.request_cancellation().map_err(|_|RunFailure::Storage)?;self.sessions.cancel(id).map_err(|_|RunFailure::Storage)}else{Ok(entry)}
            }
            _=processor.wait_until_sent()=>{
                tokio::time::sleep(Duration::from_millis(25)).await;
                let current=match self.sessions.get(id){Ok(entry)=>entry,Err(_)=>{task.abort();let _=task.await;return Err(RunFailure::Storage);}};
                if current.status==SessionStatus::Processing||(current.status==SessionStatus::Pending&&current.error.is_none()){
                    processor.request_cancellation().map_err(|_|RunFailure::Storage)?;
                    if self.sessions.cancel(id).is_err(){task.abort();let _=task.await;return Err(RunFailure::Storage);}
                }
                task.await.map_err(|_|RunFailure::Storage)?.map_err(|_|RunFailure::Storage)?;
                self.sessions.get(id).map_err(|_|RunFailure::Storage)
            }
        }
    }
}
