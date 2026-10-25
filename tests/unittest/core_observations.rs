use notype_core::observations::{mixed_count, COUNT_RULE_VERSION};

#[test]
fn mixed_counts_follow_the_published_chinese_english_rule() {
    assert_eq!(COUNT_RULE_VERSION, "zh-en-v1");
    assert_eq!(mixed_count("我們用 API 測試。"), 6);
    assert_eq!(mixed_count("don't stop"), 2);
    assert_eq!(mixed_count("session_id"), 2);
    assert_eq!(mixed_count("API-v2 42"), 3);
    assert_eq!(mixed_count("𠀀 A\u{301} 👩‍💻\n"), 2);
}

#[test]
fn exact_actual_receipts_reject_estimate_notations_and_invalid_units() {
    use notype_core::observations::ActualCharge;
    let charge = ActualCharge::new("000.100000000000000001", "usd", "fixture-returned").unwrap();
    assert_eq!(charge.amount, "0.100000000000000001");
    assert_eq!(charge.currency, "USD");
    for amount in ["-1", "1e-3", "0.0000000000000000001", "NaN"] {
        assert!(ActualCharge::new(amount, "USD", "fixture").is_err());
    }
    assert!(ActualCharge::new("1", "US$", "fixture").is_err());
}

#[test]
fn received_audio_rejects_unrepresentable_duration_and_zero_rate() {
    use notype_core::observations::RecordingObservation;
    assert!(RecordingObservation::new(1, 0, Some(0)).is_err());
    assert!(RecordingObservation::new(u64::MAX, 1, Some(0)).is_err());
    assert_eq!(
        RecordingObservation::new(1, 48_000, None)
            .unwrap()
            .audio_ms(),
        0
    );
    assert_eq!(
        RecordingObservation::new(1, 48_000, None)
            .unwrap()
            .quality(),
        "unknown"
    );
}
