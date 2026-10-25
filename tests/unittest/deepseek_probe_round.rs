#[path = "../../desktop/src-tauri/src/deepseek_probe/policy.rs"]
mod policy;
#[path = "../../desktop/src-tauri/src/deepseek_probe/round.rs"]
mod round;

use policy::{Denied, WireFacts};
use round::{Access, Outcome, Round, RoundStore, Snapshot};
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct FixtureStorage {
    state: Snapshot,
    fail_commit: bool,
    opened: bool,
}
#[derive(Clone, Default)]
struct MemoryStore(Arc<Mutex<FixtureStorage>>);
impl RoundStore for MemoryStore {
    fn load(&self) -> Result<(Access, Snapshot), Denied> {
        let mut fixture = self.0.lock().unwrap();
        let access = if fixture.opened {
            Access::ReportOnly
        } else {
            Access::Fresh
        };
        fixture.opened = true;
        Ok((access, fixture.state.clone()))
    }
    fn commit(&mut self, _revision: u64, next: &Snapshot) -> Result<(), Denied> {
        let mut fixture = self.0.lock().unwrap();
        if fixture.fail_commit {
            return Err(Denied::Storage);
        }
        fixture.state = next.clone();
        Ok(())
    }
}
fn wire() -> WireFacts<'static> {
    WireFacts {
        endpoint: "https://api.deepseek.com/chat/completions",
        model: "deepseek-flash",
        deepseek_profile: true,
        effort: Some("none"),
        thinking: Some(false),
        non_streaming: true,
        max_output_tokens: Some(512),
        wire_bytes: 4096,
        message_count: 3,
        plain_text_only: true,
    }
}

#[test]
fn storage_failure_cannot_grant_a_send_permit() {
    let storage = MemoryStore::default();
    let mut round = Round::open(storage.clone()).unwrap();
    round.arm().unwrap();
    storage.0.lock().unwrap().fail_commit = true;
    assert_eq!(
        round.admit(1, "history-1", "call-1", &wire()).unwrap_err(),
        Denied::Storage
    );
}

#[test]
fn all_eight_calls_include_cancel_retry_and_never_refund_reservations() {
    let mut round = Round::open(MemoryStore::default()).unwrap();
    round.arm().unwrap();
    for (slot, history) in [
        (1, "h1"),
        (2, "h2"),
        (3, "h3"),
        (4, "h4"),
        (5, "h5"),
        (6, "h5"),
        (7, "h6"),
        (8, "h6"),
    ] {
        let permit = round
            .admit(slot, history, &format!("c{slot}"), &wire())
            .unwrap();
        round
            .finish(
                &permit,
                if slot == 7 {
                    Outcome::IntentionalCancellation
                } else {
                    Outcome::Success
                },
            )
            .unwrap();
    }
    let report = round.report();
    assert_eq!(report.reserved_slots, 8);
    assert_eq!(report.reservation_units, 835_584); // Approved US$0.0835584.
    assert!(report.reservation_units <= 1_000_000);
    assert_eq!(
        round.admit(9, "h7", "c9", &wire()).unwrap_err(),
        Denied::Schedule
    );
    assert_eq!(round.report().reservation_units, 835_584);
}

#[test]
fn failed_forward_contract_halts_before_automatic_back_translation() {
    let mut round = Round::open(MemoryStore::default()).unwrap();
    round.arm().unwrap();
    for slot in 1..=5 {
        let permit = round
            .admit(slot, &format!("h{slot}"), &format!("c{slot}"), &wire())
            .unwrap();
        round
            .finish(
                &permit,
                if slot == 5 {
                    Outcome::ContractViolation
                } else {
                    Outcome::Success
                },
            )
            .unwrap();
    }
    assert_eq!(
        round.admit(6, "h5", "c6", &wire()).unwrap_err(),
        Denied::Halted
    );
    assert_eq!(round.report().reservation_units, 522_240);
    assert!(round.report().halted);
}

#[test]
fn only_a_new_armed_round_can_admit_and_reopening_cannot_rearm() {
    let storage = MemoryStore::default();
    let mut round = Round::open(storage.clone()).unwrap();
    assert_eq!(
        round.admit(1, "h1", "c1", &wire()).unwrap_err(),
        Denied::NotArmed
    );
    round.arm().unwrap();
    assert_eq!(round.arm(), Err(Denied::Schedule));
    let permit = round.admit(1, "h1", "c1", &wire()).unwrap();
    drop(round); // May have crashed before actually sending or receiving.
    let mut reopened = Round::open(storage).unwrap();
    assert!(reopened.report().report_only);
    assert_eq!(reopened.report().reservation_units, 104_448);
    assert_eq!(reopened.arm(), Err(Denied::ReportOnly));
    assert_eq!(
        reopened.finish(&permit, Outcome::Success),
        Err(Denied::ReportOnly)
    );
    assert_eq!(
        reopened.admit(2, "h2", "c2", &wire()).unwrap_err(),
        Denied::ReportOnly
    );
}

#[test]
fn rejected_wire_halts_the_same_guard_before_another_call_can_be_reserved() {
    let mut round = Round::open(MemoryStore::default()).unwrap();
    round.arm().unwrap();
    let mut invalid = wire();
    invalid.thinking = Some(true);
    assert_eq!(
        round.admit(1, "h1", "c1", &invalid).unwrap_err(),
        Denied::WireContract
    );
    assert_eq!(
        round.admit(1, "h1", "c2", &wire()).unwrap_err(),
        Denied::Halted
    );
    assert_eq!(round.report().reservation_units, 0);
}

#[test]
fn unavailable_source_language_skips_only_the_reverse_call_without_replacement() {
    let mut round = Round::open(MemoryStore::default()).unwrap();
    round.arm().unwrap();
    for slot in 1..=5 {
        let permit = round
            .admit(slot, &format!("h{slot}"), &format!("c{slot}"), &wire())
            .unwrap();
        round.finish(&permit, Outcome::Success).unwrap();
    }
    round.skip_back_translation().unwrap();
    assert_eq!(
        round.admit(6, "h5", "replacement", &wire()).unwrap_err(),
        Denied::Schedule
    );
    let cancelled = round.admit(7, "h6", "c7", &wire()).unwrap();
    round
        .finish(&cancelled, Outcome::CancelRacedWithCompletion)
        .unwrap();
    let retry = round.admit(8, "h6", "c8", &wire()).unwrap();
    round.finish(&retry, Outcome::Success).unwrap();
    assert_eq!(round.report().reserved_slots, 7);
    assert_eq!(round.report().reservation_units, 731_136);
}

#[test]
fn review_and_retry_must_reuse_the_origin_history_and_distinct_call_ids() {
    let mut round = Round::open(MemoryStore::default()).unwrap();
    round.arm().unwrap();
    for slot in 1..=5 {
        let permit = round
            .admit(slot, &format!("h{slot}"), &format!("c{slot}"), &wire())
            .unwrap();
        round.finish(&permit, Outcome::Success).unwrap();
    }
    assert_eq!(
        round
            .admit(6, "different-history", "c6", &wire())
            .unwrap_err(),
        Denied::Schedule
    );
    assert_eq!(
        round.admit(6, "h5", "c5", &wire()).unwrap_err(),
        Denied::Schedule
    );
    assert_eq!(
        round
            .admit(6, "h5", "Bearer credential", &wire())
            .unwrap_err(),
        Denied::Schedule
    );
    let back = round.admit(6, "h5", "c6", &wire()).unwrap();
    round.finish(&back, Outcome::Success).unwrap();
    let cancel = round.admit(7, "h6", "c7", &wire()).unwrap();
    round
        .finish(&cancel, Outcome::IntentionalCancellation)
        .unwrap();
    assert_eq!(
        round.admit(8, "h7", "c8", &wire()).unwrap_err(),
        Denied::Schedule
    );
    assert!(round.admit(8, "h6", "c8", &wire()).is_ok());
}

#[test]
fn storage_failure_after_possible_send_keeps_reserve_and_blocks_all_later_calls() {
    let storage = MemoryStore::default();
    let mut round = Round::open(storage.clone()).unwrap();
    round.arm().unwrap();
    let permit = round.admit(1, "h1", "c1", &wire()).unwrap();
    storage.0.lock().unwrap().fail_commit = true;
    assert_eq!(
        round.finish(&permit, Outcome::Success),
        Err(Denied::Storage)
    );
    storage.0.lock().unwrap().fail_commit = false;
    assert_eq!(
        round.admit(2, "h2", "c2", &wire()).unwrap_err(),
        Denied::Halted
    );
    assert_eq!(round.report().reservation_units, 104_448);
    drop(round);
    let reopened = Round::open(storage).unwrap();
    assert_eq!(reopened.report().reservation_units, 104_448);
    assert!(reopened.report().report_only);
}

#[test]
fn immediate_contract_halt_still_allows_the_origin_call_terminal_receipt() {
    let mut round = Round::open(MemoryStore::default()).unwrap();
    round.arm().unwrap();
    let permit = round.admit(1, "h1", "c1", &wire()).unwrap();
    round.halt().unwrap(); // Shared observer stops any automatic next call now.
    round.finish(&permit, Outcome::ContractViolation).unwrap();
    assert_eq!(
        round.report().slots[0].state,
        round::SlotState::Finished(Outcome::ContractViolation)
    );
    assert_eq!(round.report().reservation_units, 104_448);
    assert_eq!(
        round.admit(2, "h2", "c2", &wire()).unwrap_err(),
        Denied::Halted
    );
}
