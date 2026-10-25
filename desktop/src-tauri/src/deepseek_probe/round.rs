//! One-shot admission and reservation lifecycle. Storage is a synchronous,
//! durable system boundary; it must commit before returning a send permit.
use super::policy::{schedule, validate_wire, Denied, WireFacts, BUDGET_UNITS, RESERVATION_UNITS};

/// Whether this process exclusively created the round or reopened a report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    Fresh,
    ReportOnly,
}

/// Bounded storage snapshot, independent of credentials and product history.
#[derive(Clone, Default)]
pub struct Snapshot {
    pub revision: u64,
    pub armed: bool,
    pub halted: bool,
    pub slots: [SlotRecord; 8],
}

/// Bounded persisted admission state. PossiblySent conservatively includes a
/// permit whose HTTP future was later cancelled before reaching the network.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum SlotState {
    #[default]
    NotRun,
    PossiblySent,
    Finished(Outcome),
    Skipped,
}

/// Safe local correlation identifiers; never provider text or Authorization.
#[derive(Debug, Clone, Default)]
pub struct SlotRecord {
    pub state: SlotState,
    pub history_id: String,
    pub call_id: String,
}

/// Terminal classification; only the deliberately scheduled slot-7 cancellation
/// may continue. A network/storage/contract failure must halt remaining sends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Success,
    IntentionalCancellation,
    CancelRacedWithCompletion,
    Failed,
    ContractViolation,
}

/// Bounded privacy-safe reservation report, never an actual monetary receipt.
pub struct Report {
    /// Number of possibly-sent or terminal reserved slots, at most eight.
    pub reserved_slots: u8,
    /// Budget reservations in USD units of 10^-7; never actual billed fees.
    pub reservation_units: u64,
    pub report_only: bool,
    pub halted: bool,
    pub slots: [SlotRecord; 8],
}

/// Durable compare-and-commit boundary. An error must never grant permission.
/// The SQLite adapter uses FULL synchronization and exclusive ownership.
pub trait RoundStore {
    fn load(&self) -> Result<(Access, Snapshot), Denied>;
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
    pub fn admit(
        &mut self,
        slot: u8,
        history_id: &str,
        call_id: &str,
        wire: &WireFacts<'_>,
    ) -> Result<SendPermit, Denied> {
        self.require_write()?;
        if !self.state.armed {
            return Err(Denied::NotArmed);
        }
        if let Err(reason) = validate_wire(wire) {
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
