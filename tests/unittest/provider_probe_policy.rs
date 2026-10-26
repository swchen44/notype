#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/provider_probe/policy.rs"]
mod policy;
use policy::*;
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/provider_probe/journal.rs"]
mod journal;
fn identity() -> journal::Identity {
    journal::Identity {
        source: "a".repeat(40),
        policy_sha256: "b".repeat(64),
        corpus_sha256: "c".repeat(64),
        contract_sha256: vec!["d".repeat(64); 6],
    }
}
#[test]
fn immutable_round_has_eight_paid_four_free_and_four_sealed_slots() {
    let slots = schedule();
    assert_eq!(
        slots
            .iter()
            .filter(|slot| slot.provider == Provider::DeepSeek)
            .count(),
        8
    );
    assert_eq!(
        slots
            .iter()
            .filter(|slot| slot.provider == Provider::OpenRouter)
            .count(),
        4
    );
    assert!(slots[12..]
        .iter()
        .all(|slot| slot.provider == Provider::GeminiSealed && slot.purpose == Purpose::Sealed));
    assert_eq!(
        slots.iter().map(|slot| slot.reservation()).sum::<u64>(),
        835_584
    );
    assert_eq!(BUDGET_UNITS, 20_000_000);
    assert_eq!(slots[4].group, slots[5].group);
    assert_eq!(slots[6].group, slots[7].group);
    assert_eq!(slots[9].group, slots[10].group);
    assert_eq!(slots[0].model(), "deepseek-flash");
    assert_eq!(slots[8].model(), "google/gemma-4-31b-it:free");
    assert_eq!(
        slots[..12]
            .iter()
            .map(|slot| slot.group)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        9
    );
}
#[test]
fn machine_quality_checks_keep_identifiers_decimal_negation_and_unknown_meaning_limit() {
    assert!(quality(schedule()[0], " \nOK\n"));
    assert!(!quality(schedule()[0], "Here is OK"));
    assert!(quality(
        schedule()[1],
        "不要刪掉 session_id，timeout 0.5，不是 5，保留 42 個 API request。"
    ));
    assert!(!quality(
        schedule()[1],
        "刪掉 session_id，timeout 5，保留 42 個 API request。"
    ));
    assert!(!quality(schedule()[12], "OK"));
    assert!(!quality(schedule()[2], "x".repeat(16_385).as_str()));
}

#[test]
fn machine_quality_rejects_wrong_target_script_and_obvious_wrapped_prose() {
    assert!(
        !quality(schedule()[2], DICTATION),
        "English translation cannot remain Chinese"
    );
    assert!(
        !quality(
            schedule()[1],
            "Do not delete session_id; timeout 0.5, not 5, retain 42 API requests."
        ),
        "Chinese dictation cannot become English"
    );
    assert!(
        !quality(schedule()[3], "Meeting moved to Wednesday at 2; no slides."),
        "Chinese ask retains its requested script"
    );
    assert!(
        !quality(
            schedule()[1],
            &format!("Here is the polished text:\n{DICTATION}")
        ),
        "obvious extra prose is rejected"
    );
    assert!(quality(
        schedule()[2],
        "Do not delete session_id; timeout is 0.5 seconds, not 5; retain 42 API requests."
    ));
}

#[test]
fn actual_sqlite_authorization_seals_gemini_retains_every_reservation_and_reopens_report_only() {
    use journal::*;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("round");
    let mut journal = Journal::open(&path, identity()).unwrap();
    assert!(matches!(
        Journal::open(&path, identity()),
        Err(Failure::AlreadyOwned)
    ));
    journal.arm().unwrap();
    assert!(journal.snapshot().records[12..]
        .iter()
        .all(|r| r.state == State::SealedGemini));
    for spec in schedule().iter().take(12) {
        let history = format!("history-{}", spec.group);
        let call = format!("actual-call-{}", spec.number);
        journal
            .link(
                spec.number,
                &history,
                &call,
                if spec.number == 8 { 2 } else { 1 },
            )
            .unwrap();
        journal.reserve(spec.number, &call).unwrap();
        journal
            .finish(spec.number, &call, Outcome::Success)
            .unwrap();
        let revision = journal.snapshot().revision;
        journal.reconciled(revision, spec.number, &call).unwrap();
    }
    assert_eq!(journal.snapshot().reservation_units().unwrap(), 835_584);
    assert_eq!(journal.reserve(13, "extra"), Err(Failure::Schedule));
    drop(journal);
    let mut reopened = Journal::open(&path, identity()).unwrap();
    assert!(reopened.report_only());
    assert_eq!(reopened.arm(), Err(Failure::ReportOnly));
    assert_eq!(reopened.snapshot().reservation_units().unwrap(), 835_584);
}

#[test]
fn explicit_report_does_not_create_missing_state_and_cannot_rearm_retained_round() {
    use journal::*;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("absent");
    assert!(matches!(
        Journal::open_report(&path),
        Err(Failure::ReportOnly)
    ));
    assert!(!path.exists());
    let mut created = Journal::open(&path, identity()).unwrap();
    created.arm().unwrap();
    drop(created);
    let before = std::fs::read(path.join("round.sqlite")).unwrap();
    let mut report = Journal::open_report(&path).unwrap();
    assert_eq!(report.arm(), Err(Failure::ReportOnly));
    assert_eq!(report.halt(), Err(Failure::ReportOnly));
    drop(report);
    assert_eq!(std::fs::read(path.join("round.sqlite")).unwrap(), before);
}

#[test]
fn actual_sqlite_commit_failure_blocks_further_admission_without_reset_or_refund() {
    use journal::*;
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("round");
    let mut journal = Journal::open(&path, identity()).unwrap();
    journal.arm().unwrap();
    journal.link(1, "history-1", "call-1", 1).unwrap();
    let database = rusqlite::Connection::open(path.join("round.sqlite")).unwrap();
    database.execute_batch("CREATE TRIGGER fail_reserve BEFORE UPDATE ON reservation BEGIN SELECT RAISE(FAIL,'owned storage fault');END;").unwrap();
    assert_eq!(journal.reserve(1, "call-1"), Err(Failure::Storage));
    assert_eq!(journal.reserve(1, "call-1"), Err(Failure::Halted));
    drop(database);
    drop(journal);
    let reopened = Journal::open(&path, identity()).unwrap();
    assert!(reopened.report_only());
    assert_eq!(reopened.snapshot().records[0].state, State::Linked);
}

#[test]
fn actual_sqlite_logical_corruption_cannot_reopen_impossible_schedule_or_grouping() {
    use journal::*;
    for corruption in [
        "future-slot",
        "gemini-open",
        "duplicate-call",
        "wrong-group",
        "failed-unhalted",
        "unknown-field",
    ] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("round");
        let mut journal = Journal::open(&path, identity()).unwrap();
        journal.arm().unwrap();
        for spec in schedule().iter().take(2) {
            let call = format!("call-{}", spec.number);
            journal
                .link(spec.number, &format!("history-{}", spec.group), &call, 1)
                .unwrap();
            journal.reserve(spec.number, &call).unwrap();
            journal
                .finish(spec.number, &call, Outcome::Success)
                .unwrap();
            journal
                .reconciled(journal.snapshot().revision, spec.number, &call)
                .unwrap();
        }
        drop(journal);
        let connection = rusqlite::Connection::open(path.join("round.sqlite")).unwrap();
        let payload: String = connection
            .query_row("SELECT payload FROM reservation", [], |row| row.get(0))
            .unwrap();
        let mut payload: serde_json::Value = serde_json::from_str(&payload).unwrap();
        match corruption {
            "future-slot" => {
                payload["records"][4] = payload["records"][1].clone();
                payload["records"][4]["slot"] = serde_json::json!(5);
            }
            "gemini-open" => payload["records"][12]["state"] = serde_json::json!("NotRun"),
            "duplicate-call" => payload["records"][1]["call"] = serde_json::json!("call-1"),
            "wrong-group" => payload["records"][1]["history"] = serde_json::json!("history-1"),
            "failed-unhalted" => payload["records"][1]["outcome"] = serde_json::json!("Failed"),
            _ => payload["foreign"] = serde_json::json!(true),
        }
        connection
            .execute("UPDATE reservation SET payload=?1", [payload.to_string()])
            .unwrap();
        drop(connection);
        assert!(
            matches!(Journal::open(&path, identity()), Err(Failure::Storage)),
            "{corruption}"
        );
    }
}
