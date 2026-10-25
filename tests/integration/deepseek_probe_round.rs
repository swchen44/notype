#[path = "../../desktop/src-tauri/src/deepseek_probe/policy.rs"]
mod policy;
#[path = "../../desktop/src-tauri/src/deepseek_probe/round.rs"]
mod round;
#[path = "../../desktop/src-tauri/src/deepseek_probe/sqlite_round.rs"]
mod sqlite_round;

use policy::{Denied, WireFacts};
use round::{Round, RoundStore};
use sqlite_round::SqliteRoundStore;
use std::{
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

struct OwnedFixture(PathBuf);
impl OwnedFixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "notype-p23-fixture-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn round(&self) -> PathBuf {
        self.0.join("round")
    }
}
impl Drop for OwnedFixture {
    fn drop(&mut self) {
        let resolved = self.0.canonicalize().unwrap();
        let temp = std::env::temp_dir().canonicalize().unwrap();
        assert_eq!(resolved.parent(), Some(temp.as_path()));
        assert!(resolved
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("notype-p23-fixture-"));
        std::fs::remove_dir_all(resolved).unwrap();
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
fn durable_reservation_survives_restart_and_the_round_is_report_only() {
    let fixture = OwnedFixture::new();
    let mut round = Round::open(SqliteRoundStore::open_owned(&fixture.round()).unwrap()).unwrap();
    round.arm().unwrap();
    let _permit = round.admit(1, "history-1", "call-1", &wire()).unwrap();
    drop(round); // Unknown whether an HTTP future would have been polled.
    let mut reopened =
        Round::open(SqliteRoundStore::open_owned(&fixture.round()).unwrap()).unwrap();
    assert!(reopened.report().report_only);
    assert_eq!(reopened.report().reservation_units, 104_448);
    assert_eq!(reopened.arm(), Err(Denied::ReportOnly));
    assert_eq!(
        reopened
            .admit(1, "history-1", "new-call", &wire())
            .unwrap_err(),
        Denied::ReportOnly
    );
}

#[test]
fn concurrent_owner_is_rejected_and_an_unarmed_restart_is_still_report_only() {
    let fixture = OwnedFixture::new();
    let store = SqliteRoundStore::open_owned(&fixture.round()).unwrap();
    assert!(matches!(
        SqliteRoundStore::open_owned(&fixture.round()),
        Err(Denied::AlreadyOwned)
    ));
    drop(store);
    let mut reopened =
        Round::open(SqliteRoundStore::open_owned(&fixture.round()).unwrap()).unwrap();
    assert_eq!(reopened.arm(), Err(Denied::ReportOnly));
    assert_eq!(reopened.report().reserved_slots, 0);
}

#[test]
fn missing_or_corrupt_artifacts_never_reinitialize_budget() {
    for artifact in ["round.lock", "round.db"] {
        let fixture = OwnedFixture::new();
        let mut round =
            Round::open(SqliteRoundStore::open_owned(&fixture.round()).unwrap()).unwrap();
        round.arm().unwrap();
        round.admit(1, "h1", "c1", &wire()).unwrap();
        drop(round);
        std::fs::remove_file(fixture.round().join(artifact)).unwrap();
        assert!(matches!(
            SqliteRoundStore::open_owned(&fixture.round()),
            Err(Denied::Storage)
        ));
        assert!(!fixture.round().join(artifact).exists());
    }
    let fixture = OwnedFixture::new();
    drop(SqliteRoundStore::open_owned(&fixture.round()).unwrap());
    std::fs::write(fixture.round().join("round.db"), b"corrupt fixture").unwrap();
    assert!(matches!(
        SqliteRoundStore::open_owned(&fixture.round()),
        Err(Denied::Storage)
    ));
    assert_eq!(
        std::fs::read(fixture.round().join("round.db")).unwrap(),
        b"corrupt fixture"
    );
}

#[test]
fn foreign_directory_or_marker_is_rejected_without_mutation() {
    let fixture = OwnedFixture::new();
    std::fs::create_dir(fixture.round()).unwrap();
    std::fs::write(
        fixture.round().join("foreign.txt"),
        b"fixture unrelated data",
    )
    .unwrap();
    assert!(matches!(
        SqliteRoundStore::open_owned(&fixture.round()),
        Err(Denied::Storage)
    ));
    assert_eq!(
        std::fs::read(fixture.round().join("foreign.txt")).unwrap(),
        b"fixture unrelated data"
    );
    assert!(!fixture.round().join("round.lock").exists());
}

#[test]
fn sqlite_retains_all_eight_outcomes_and_aggregate_reservations() {
    let fixture = OwnedFixture::new();
    let mut round = Round::open(SqliteRoundStore::open_owned(&fixture.round()).unwrap()).unwrap();
    round.arm().unwrap();
    for (number, history) in [
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
            .admit(number, history, &format!("c{number}"), &wire())
            .unwrap();
        round
            .finish(
                &permit,
                if number == 7 {
                    round::Outcome::IntentionalCancellation
                } else {
                    round::Outcome::Success
                },
            )
            .unwrap();
    }
    drop(round);
    let reopened = Round::open(SqliteRoundStore::open_owned(&fixture.round()).unwrap()).unwrap();
    let report = reopened.report();
    assert_eq!(report.reserved_slots, 8);
    assert_eq!(report.reservation_units, 835_584);
    assert_eq!(
        report.slots[6].state,
        round::SlotState::Finished(round::Outcome::IntentionalCancellation)
    );
    assert_eq!(report.slots[7].history_id, "h6");
    assert_eq!(report.slots[7].call_id, "c8");
    assert!(report.report_only);
}

#[test]
fn process_exit_without_destructors_cannot_restart_a_paid_round() {
    const CHILD_PATH: &str = "NOTYPE_P23_OWNED_TEST_ROUND";
    if let Some(path) = std::env::var_os(CHILD_PATH) {
        let path = PathBuf::from(path);
        let fixture = path.parent().unwrap().canonicalize().unwrap();
        assert_eq!(
            fixture.parent(),
            Some(std::env::temp_dir().canonicalize().unwrap().as_path())
        );
        assert!(fixture
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("notype-p23-fixture-"));
        assert_eq!(path.file_name().unwrap(), "round");
        let mut round = Round::open(SqliteRoundStore::open_owned(&path).unwrap()).unwrap();
        round.arm().unwrap();
        round.admit(1, "h1", "c1", &wire()).unwrap();
        // Intentionally bypass Drop after a durable reservation. This is only
        // the owned nonsecret test worker; no console mode or HTTP exists.
        std::process::exit(0);
    }
    let fixture = OwnedFixture::new();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "process_exit_without_destructors_cannot_restart_a_paid_round",
        ])
        .env(CHILD_PATH, fixture.round())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "owned worker failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let mut reopened =
        Round::open(SqliteRoundStore::open_owned(&fixture.round()).unwrap()).unwrap();
    assert_eq!(reopened.report().reservation_units, 104_448);
    assert!(reopened.report().report_only);
    assert_eq!(reopened.arm(), Err(Denied::ReportOnly));
}

#[test]
fn sqlite_rejects_individually_valid_rows_with_an_impossible_slot_prefix() {
    let fixture = OwnedFixture::new();
    drop(SqliteRoundStore::open_owned(&fixture.round()).unwrap());
    let connection = rusqlite::Connection::open(fixture.round().join("round.db")).unwrap();
    connection
        .execute_batch(
            "UPDATE round_meta SET revision=3,armed=1;
         UPDATE slots SET state=2,history_id='h2',call_id='c2' WHERE number=2;",
        )
        .unwrap();
    drop(connection);
    assert!(
        matches!(
            SqliteRoundStore::open_owned(&fixture.round()),
            Err(Denied::Storage)
        ),
        "valid individual rows cannot make slot2 precede slot1"
    );
}

fn complete_prefix(fixture: &OwnedFixture, count: u8) {
    let mut round = Round::open(SqliteRoundStore::open_owned(&fixture.round()).unwrap()).unwrap();
    round.arm().unwrap();
    for number in 1..=count {
        let group = policy::schedule()[usize::from(number - 1)].history_group;
        let permit = round
            .admit(number, &format!("h{group}"), &format!("c{number}"), &wire())
            .unwrap();
        round
            .finish(
                &permit,
                if number == 7 {
                    round::Outcome::IntentionalCancellation
                } else {
                    round::Outcome::Success
                },
            )
            .unwrap();
    }
}

#[test]
fn sqlite_rejects_impossible_identity_completion_and_halt_combinations() {
    for mutation in [
        "UPDATE slots SET call_id='c1' WHERE number=2",
        "UPDATE slots SET history_id='wrong-group' WHERE number=6",
        "UPDATE slots SET history_id='wrong-retry-group' WHERE number=8",
        "UPDATE slots SET history_id='h1' WHERE number=2",
        "UPDATE slots SET state=1 WHERE number=1",
        "UPDATE slots SET state=5 WHERE number=2; UPDATE round_meta SET halted=1",
        "UPDATE slots SET state=5 WHERE number=8",
        "UPDATE round_meta SET revision=0",
    ] {
        let fixture = OwnedFixture::new();
        complete_prefix(&fixture, 8);
        let connection = rusqlite::Connection::open(fixture.round().join("round.db")).unwrap();
        connection.execute_batch(mutation).unwrap();
        drop(connection);
        assert!(
            matches!(
                SqliteRoundStore::open_owned(&fixture.round()),
                Err(Denied::Storage)
            ),
            "accepted impossible snapshot: {mutation}"
        );
    }
}

fn finish_next(round: &mut Round<SqliteRoundStore>, number: u8, outcome: round::Outcome) {
    let group = policy::schedule()[usize::from(number - 1)].history_group;
    let permit = round
        .admit(number, &format!("h{group}"), &format!("c{number}"), &wire())
        .unwrap();
    round.finish(&permit, outcome).unwrap();
}

#[test]
fn sqlite_preserves_actual_transition_crash_prefixes_and_cross_store_failure_evidence() {
    for completed in 0..=8 {
        for edge in [
            "completed",
            "pending",
            "halted-pending",
            "failed",
            "halted-receipt",
        ] {
            if completed == 8 && edge != "completed" {
                continue;
            }
            let fixture = OwnedFixture::new();
            let mut round =
                Round::open(SqliteRoundStore::open_owned(&fixture.round()).unwrap()).unwrap();
            round.arm().unwrap();
            for number in 1..=completed {
                finish_next(
                    &mut round,
                    number,
                    if number == 7 {
                        round::Outcome::CancelRacedWithCompletion
                    } else {
                        round::Outcome::Success
                    },
                );
            }
            if edge != "completed" {
                let number = completed + 1;
                let group = policy::schedule()[usize::from(number - 1)].history_group;
                let permit = round
                    .admit(number, &format!("h{group}"), &format!("c{number}"), &wire())
                    .unwrap();
                // No history/call store has been created: a crash or another
                // store's failure does not invalidate this durable reservation.
                if edge.starts_with("halted") {
                    round.halt().unwrap();
                }
                if edge == "failed" {
                    round.finish(&permit, round::Outcome::Failed).unwrap();
                }
                if edge == "halted-receipt" {
                    round
                        .finish(&permit, round::Outcome::ContractViolation)
                        .unwrap();
                }
            }
            let before = round.report();
            drop(round);
            let reopened =
                Round::open(SqliteRoundStore::open_owned(&fixture.round()).unwrap()).unwrap();
            let after = reopened.report();
            assert!(after.report_only);
            assert_eq!(
                after.reserved_slots, before.reserved_slots,
                "{completed}/{edge}"
            );
            assert_eq!(after.reservation_units, before.reservation_units);
            assert_eq!(after.halted, before.halted);
            for (before, after) in before.slots.iter().zip(after.slots.iter()) {
                assert_eq!(after.state, before.state);
                assert_eq!(after.history_id, before.history_id);
                assert_eq!(after.call_id, before.call_id);
            }
        }
    }
    for cancellation in [
        round::Outcome::IntentionalCancellation,
        round::Outcome::CancelRacedWithCompletion,
    ] {
        let fixture = OwnedFixture::new();
        let mut round =
            Round::open(SqliteRoundStore::open_owned(&fixture.round()).unwrap()).unwrap();
        round.arm().unwrap();
        for number in 1..=5 {
            finish_next(&mut round, number, round::Outcome::Success);
        }
        round.skip_back_translation().unwrap();
        finish_next(&mut round, 7, cancellation);
        finish_next(&mut round, 8, round::Outcome::Success);
        drop(round);
        let reopened =
            Round::open(SqliteRoundStore::open_owned(&fixture.round()).unwrap()).unwrap();
        assert_eq!(reopened.report().reserved_slots, 7);
        assert_eq!(reopened.report().reservation_units, 731_136);
        assert_eq!(reopened.report().slots[5].state, round::SlotState::Skipped);
    }
    let fixture = OwnedFixture::new();
    let mut round = Round::open(SqliteRoundStore::open_owned(&fixture.round()).unwrap()).unwrap();
    round.halt().unwrap(); // Legal fail-closed halt before arming or any slots.
    drop(round);
    let reopened = Round::open(SqliteRoundStore::open_owned(&fixture.round()).unwrap()).unwrap();
    assert!(reopened.report().halted);
    assert_eq!(reopened.report().reserved_slots, 0);
}

fn logical_rows(path: &std::path::Path) -> ((i64, i64, i64), Vec<(u8, u8, String, String)>) {
    let connection = rusqlite::Connection::open(path).unwrap();
    let meta = connection
        .query_row("SELECT revision,armed,halted FROM round_meta", [], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })
        .unwrap();
    let mut statement = connection
        .prepare("SELECT number,state,history_id,call_id FROM slots ORDER BY number")
        .unwrap();
    let slots = statement
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    (meta, slots)
}

#[test]
fn sqlite_compare_commit_rejects_invalid_candidates_and_cannot_wash_corrupt_current_state() {
    let fixture = OwnedFixture::new();
    let database = fixture.round().join("round.db");
    let mut store = SqliteRoundStore::open_owned(&fixture.round()).unwrap();
    let (_, mut armed) = store.load().unwrap();
    armed.armed = true;
    armed.revision = 1;
    store.commit(0, &armed).unwrap();
    let before = logical_rows(&database);
    let mut invalid = armed.clone();
    invalid.revision = 2;
    invalid.slots[1] = round::SlotRecord {
        state: round::SlotState::PossiblySent,
        history_id: "h2".into(),
        call_id: "c2".into(),
    };
    assert_eq!(store.commit(1, &invalid), Err(Denied::Storage));
    assert_eq!(
        logical_rows(&database),
        before,
        "invalid candidate must not change logical state"
    );
    let mut skipped_revision = armed.clone();
    skipped_revision.halted = true;
    skipped_revision.revision = 3;
    assert_eq!(store.commit(1, &skipped_revision), Err(Denied::Storage));
    assert_eq!(logical_rows(&database), before);
    let mut halted = armed.clone();
    halted.halted = true;
    halted.revision = 2;
    assert_eq!(store.commit(0, &halted), Err(Denied::Storage));
    assert_eq!(
        logical_rows(&database),
        before,
        "stale revision does not mutate a valid current state"
    );
    store.commit(1, &halted).unwrap(); // Rejected writes did not poison the owner.
    let connection = rusqlite::Connection::open(&database).unwrap();
    connection.execute_batch("UPDATE round_meta SET revision=2; UPDATE slots SET state=1,history_id='h2',call_id='c2' WHERE number=2").unwrap();
    drop(connection);
    let corrupt = logical_rows(&database);
    assert!(matches!(store.load(), Err(Denied::Storage)));
    let mut washed = armed;
    washed.revision = 3;
    washed.slots[0] = round::SlotRecord {
        state: round::SlotState::Finished(round::Outcome::Success),
        history_id: "h1".into(),
        call_id: "c1".into(),
    };
    assert_eq!(store.commit(2, &washed), Err(Denied::Storage));
    assert_eq!(
        logical_rows(&database),
        corrupt,
        "invalid current state cannot be overwritten by a valid-looking candidate"
    );
}
