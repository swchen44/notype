#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/provider_probe/policy.rs"]
mod policy;
use policy::*;
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
