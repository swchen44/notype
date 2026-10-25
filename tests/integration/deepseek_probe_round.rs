#[path = "../../desktop/src-tauri/src/deepseek_probe/policy.rs"]
mod policy;
#[path = "../../desktop/src-tauri/src/deepseek_probe/round.rs"]
mod round;
#[path = "../../desktop/src-tauri/src/deepseek_probe/sqlite_round.rs"]
mod sqlite_round;

use policy::{Denied, WireFacts};
use round::Round;
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
