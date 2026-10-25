//! One-shot admission and reservation lifecycle. Storage is a synchronous,
//! durable system boundary; it must commit before returning a send permit.
use super::policy::{schedule, validate_wire, Denied, WireFacts, BUDGET_UNITS, RESERVATION_UNITS};

/// Whether this process exclusively created the round or reopened a report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    /// This process exclusively created the owned store and may reserve once.
    Fresh,
    /// The round already existed; opening evidence never reauthorizes a send.
    ReportOnly,
}

/// Bounded storage snapshot, independent of credentials and product history.
#[derive(Clone, Default)]
pub struct Snapshot {
    /// Monotonic durable compare-and-commit revision, initially zero.
    pub revision: u64,
    /// Whether the single arm operation committed before any slot reservation.
    pub armed: bool,
    /// Persisted admission halt; a final receipt may still complete the active slot.
    pub halted: bool,
    /// Fixed ordered schedule records; untouched trailing entries remain NotRun.
    pub slots: [SlotRecord; 8],
}

/// Shared persisted-state invariant; adapters must reject inconsistent evidence.
pub(super) fn validate_snapshot(state: &Snapshot) -> Result<(), Denied> {
    let mut not_run = false;
    let mut reserved = 0u64;
    let mut finished = 0u64;
    let mut skipped = 0u64;
    let mut fatal = false;
    for (index, slot) in state.slots.iter().enumerate() {
        match slot.state {
            SlotState::NotRun | SlotState::Skipped => {
                if !slot.history_id.is_empty()
                    || !slot.call_id.is_empty()
                    || (slot.state == SlotState::Skipped && index != 5)
                {
                    return Err(Denied::Storage);
                }
            }
            _ if !valid_id(&slot.history_id) || !valid_id(&slot.call_id) => {
                return Err(Denied::Storage);
            }
            _ => {}
        }
        if slot.state == SlotState::NotRun {
            not_run = true;
            continue;
        }
        if not_run || !state.armed || fatal {
            return Err(Denied::Storage);
        }
        if index > 0
            && !matches!(
                state.slots[index - 1].state,
                SlotState::Finished(_) | SlotState::Skipped
            )
        {
            return Err(Denied::Storage);
        }
        if slot.state == SlotState::Skipped {
            if state.slots[4].state != SlotState::Finished(Outcome::Success) {
                return Err(Denied::Storage);
            }
            skipped += 1;
            continue;
        }
        reserved += 1;
        for (prior_index, prior) in state.slots[..index].iter().enumerate() {
            if prior.history_id.is_empty() {
                continue;
            }
            if prior.call_id == slot.call_id
                || ((schedule()[prior_index].history_group == schedule()[index].history_group)
                    != (prior.history_id == slot.history_id))
            {
                return Err(Denied::Storage);
            }
        }
        if let SlotState::Finished(outcome) = slot.state {
            finished += 1;
            fatal = outcome != Outcome::Success
                && !(index == 6
                    && matches!(
                        outcome,
                        Outcome::IntentionalCancellation | Outcome::CancelRacedWithCompletion
                    ));
        }
    }
    if reserved > 8
        || reserved
            .checked_mul(RESERVATION_UNITS)
            .filter(|total| *total <= BUDGET_UNITS)
            .is_none()
        || (fatal && !state.halted)
    {
        return Err(Denied::Storage);
    }
    // Every arm/admit/finish/skip commits once. Explicit halt can commit once,
    // including before a final receipt; a fatal finish itself also sets halted.
    let operations = u64::from(state.armed) + reserved + finished + skipped;
    let revision_valid = if fatal {
        state.revision == operations || state.revision == operations + 1
    } else {
        state.revision == operations + u64::from(state.halted)
    };
    if !revision_valid {
        return Err(Denied::Storage);
    }
    Ok(())
}

/// Bounded persisted admission state. PossiblySent conservatively includes a
/// permit whose HTTP future was later cancelled before reaching the network.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum SlotState {
    #[default]
    /// No reservation or local linkage exists for this slot.
    NotRun,
    /// Reservation committed; dispatch/completion may be unknown after interruption.
    PossiblySent,
    /// Terminal classification retaining the same full reservation and identifiers.
    Finished(Outcome),
    /// Only slot 6 may be explicitly omitted; it has no reservation or identifiers.
    Skipped,
}

/// Safe local correlation identifiers; never provider text or Authorization.
#[derive(Debug, Clone, Default)]
pub struct SlotRecord {
    /// Durable lifecycle classification; never a provider response body.
    pub state: SlotState,
    /// Bounded local history correlation key; grouped by the fixed schedule.
    pub history_id: String,
    /// Bounded local call correlation key, unique across reserved slots.
    pub call_id: String,
}

/// Terminal classification; only the deliberately scheduled slot-7 cancellation
/// may continue. A network/storage/contract failure must halt remaining sends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Successful terminal attempt; it does not refund the reservation.
    Success,
    /// Expected cancellation; only slot 7 may continue to a later slot.
    IntentionalCancellation,
    /// Completion won the cancellation race; only slot 7 may continue afterward.
    CancelRacedWithCompletion,
    /// Ordinary failure requiring a persisted halt before further admission.
    Failed,
    /// Wire/response contract failed; remaining sends must be halted.
    ContractViolation,
}

/// Bounded privacy-safe reservation report, never an actual monetary receipt.
pub struct Report {
    /// Number of possibly-sent or terminal reserved slots, at most eight.
    pub reserved_slots: u8,
    /// Budget reservations in USD units of 10^-7; never actual billed fees.
    pub reservation_units: u64,
    /// Whether reopening forbids every mutation and new send permit.
    pub report_only: bool,
    /// Whether a persisted halt or this process's storage failure blocks admission.
    pub halted: bool,
    /// Owned copies of safe local slot classifications/IDs, not actual fee receipts.
    pub slots: [SlotRecord; 8],
}

/// Durable compare-and-commit boundary. An error must never grant permission.
/// The SQLite adapter uses FULL synchronization and exclusive ownership.
pub trait RoundStore {
    /// Load validated, bounded evidence and this owner's access capability.
    /// Reopened rounds are report-only; missing/corrupt/inconsistent state returns
    /// a fixed error and must never be reset or repaired into fresh permission.
    fn load(&self) -> Result<(Access, Snapshot), Denied>;
    /// Atomically compare the current revision and durably store valid `next`.
    /// The caller owns exclusive write access; current and candidate evidence
    /// must both be valid, and `next.revision` must be `previous_revision + 1`.
    /// Success means durable commit before any send permit can be returned.
    /// Failure grants no permit and may follow a completed durable commit: callers
    /// must halt, never refund/retry a send, and reopen only for reporting.
    fn commit(&mut self, previous_revision: u64, next: &Snapshot) -> Result<(), Denied>;
}

/// Capability returned only after reserving the fixed request budget durably.
/// It is not proof that a request actually reached the server.
#[derive(Debug)]
pub struct SendPermit {
    slot: u8,
    call_id: String,
}

/// Actual helper admission service, shared by offline fixtures and future bridge.
pub struct Round<S: RoundStore> {
    store: S,
    state: Snapshot,
    access: Access,
    storage_failed: bool,
}

impl<S: RoundStore> Round<S> {
    /// Load the bounded round through its storage port; reopening is read only.
    pub fn open(store: S) -> Result<Self, Denied> {
        let (access, state) = store.load()?;
        Ok(Self {
            store,
            state,
            access,
            storage_failed: false,
        })
    }

    /// Durably arm this new round before the first possible dispatch.
    pub fn arm(&mut self) -> Result<(), Denied> {
        self.require_write()?;
        if self.state.armed {
            return Err(Denied::Schedule);
        }
        let mut next = self.state.clone();
        next.armed = true;
        self.persist(next)
    }

    /// Validate and reserve one fixed slot before allowing any HTTP polling.
    /// A storage error blocks this instance even if the commit actually finished;
    /// never infer an unsent/refundable slot or retry from that error.
    pub fn admit(
        &mut self,
        slot: u8,
        history_id: &str,
        call_id: &str,
        wire: &WireFacts<'_>,
    ) -> Result<SendPermit, Denied> {
        self.admit_validated(slot, history_id, call_id, validate_wire(wire))
    }

    /// Test-build-only admission for an explicitly owned loopback fixture.
    /// Production helper builds contain neither this entry nor a target override.
    #[cfg(test)]
    pub fn admit_fixture(
        &mut self,
        slot: u8,
        history_id: &str,
        call_id: &str,
        wire: &WireFacts<'_>,
    ) -> Result<SendPermit, Denied> {
        self.admit_validated(
            slot,
            history_id,
            call_id,
            super::policy::validate_fixture_wire(wire),
        )
    }

    fn admit_validated(
        &mut self,
        slot: u8,
        history_id: &str,
        call_id: &str,
        wire_validation: Result<(), Denied>,
    ) -> Result<SendPermit, Denied> {
        self.require_write()?;
        if !self.state.armed {
            return Err(Denied::NotArmed);
        }
        if let Err(reason) = wire_validation {
            self.halt()?;
            return Err(reason);
        }
        let index = slot
            .checked_sub(1)
            .filter(|s| *s < 8)
            .ok_or(Denied::Schedule)? as usize;
        if self.state.slots[..index]
            .iter()
            .any(|s| !matches!(s.state, SlotState::Finished(_) | SlotState::Skipped))
            || self.state.slots[index].state != SlotState::NotRun
        {
            return Err(Denied::Schedule);
        }
        if !valid_id(history_id)
            || !valid_id(call_id)
            || self.state.slots.iter().any(|s| s.call_id == call_id)
        {
            return Err(Denied::Schedule);
        }
        let group = schedule()[index].history_group;
        for (prior_index, prior) in self.state.slots[..index].iter().enumerate() {
            if prior.history_id.is_empty() {
                continue;
            }
            if (schedule()[prior_index].history_group == group) != (prior.history_id == history_id)
            {
                return Err(Denied::Schedule);
            }
        }
        let mut next = self.state.clone();
        next.slots[index] = SlotRecord {
            state: SlotState::PossiblySent,
            history_id: history_id.into(),
            call_id: call_id.into(),
        };
        let count = next
            .slots
            .iter()
            .filter(|s| matches!(s.state, SlotState::PossiblySent | SlotState::Finished(_)))
            .count() as u64;
        if count
            .checked_mul(RESERVATION_UNITS)
            .ok_or(Denied::Schedule)?
            > BUDGET_UNITS
        {
            return Err(Denied::Schedule);
        }
        self.persist(next)?;
        Ok(SendPermit {
            slot,
            call_id: call_id.into(),
        })
    }

    /// Record a terminal attempt without refunding its reservation.
    pub fn finish(&mut self, permit: &SendPermit, outcome: Outcome) -> Result<(), Denied> {
        self.require_owner()?;
        let index = (permit.slot - 1) as usize;
        if self.state.slots[index].state != SlotState::PossiblySent
            || self.state.slots[index].call_id != permit.call_id
        {
            return Err(Denied::Schedule);
        }
        let mut next = self.state.clone();
        next.slots[index].state = SlotState::Finished(outcome);
        if outcome != Outcome::Success
            && !(permit.slot == 7
                && matches!(
                    outcome,
                    Outcome::IntentionalCancellation | Outcome::CancelRacedWithCompletion
                ))
        {
            next.halted = true;
        }
        self.persist(next)
    }

    /// Return reservations explicitly, without estimating actual fees.
    pub fn report(&self) -> Report {
        let reserved_slots = self
            .state
            .slots
            .iter()
            .filter(|s| matches!(s.state, SlotState::PossiblySent | SlotState::Finished(_)))
            .count() as u8;
        Report {
            reserved_slots,
            reservation_units: u64::from(reserved_slots) * RESERVATION_UNITS,
            report_only: self.access == Access::ReportOnly,
            halted: self.state.halted || self.storage_failed,
            slots: self.state.slots.clone(),
        }
    }

    /// Stop every later admission synchronously, including automatic review
    /// calls; the future shared receipt observer uses this on contract errors.
    pub fn halt(&mut self) -> Result<(), Denied> {
        self.require_write()?;
        let mut next = self.state.clone();
        next.halted = true;
        self.persist(next)
    }

    /// Skip only the unsent reverse call when the successful forward result
    /// lacks a usable source language. It cannot create a replacement slot.
    pub fn skip_back_translation(&mut self) -> Result<(), Denied> {
        self.require_write()?;
        if self.state.slots[4].state != SlotState::Finished(Outcome::Success)
            || self.state.slots[5].state != SlotState::NotRun
        {
            return Err(Denied::Schedule);
        }
        let mut next = self.state.clone();
        next.slots[5].state = SlotState::Skipped;
        self.persist(next)
    }

    fn persist(&mut self, mut next: Snapshot) -> Result<(), Denied> {
        next.revision = self.state.revision.checked_add(1).ok_or(Denied::Storage)?;
        if self.store.commit(self.state.revision, &next).is_err() {
            self.storage_failed = true;
            return Err(Denied::Storage);
        }
        self.state = next;
        Ok(())
    }

    fn require_write(&self) -> Result<(), Denied> {
        self.require_owner()?;
        if self.state.halted {
            return Err(Denied::Halted);
        }
        Ok(())
    }

    fn require_owner(&self) -> Result<(), Denied> {
        if self.access == Access::ReportOnly {
            return Err(Denied::ReportOnly);
        }
        if self.storage_failed {
            return Err(Denied::Halted);
        }
        Ok(())
    }
}

/// Local opaque IDs are short ASCII correlation keys, not arbitrary strings.
pub fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}
