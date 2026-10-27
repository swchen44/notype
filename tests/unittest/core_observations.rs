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
fn explicit_openrouter_receipts_preserve_the_exact_returned_usd_decimal() {
    use notype_core::observations::ApiReceipt;
    for (wire, expected) in [
        (r#"{"usage":{"cost":0}}"#, "0"),
        (
            r#"{"usage":{"cost":0.100000000000000001}}"#,
            "0.100000000000000001",
        ),
        (r#"{"usage":{"cost":1.4e-7}}"#, "0.00000014"),
        (r#"{"usage":{"cost":1e+2}}"#, "100"),
        (r#"{"usage":{"cost":1.0000000000000000000}}"#, "1"),
    ] {
        let charge = ApiReceipt::default()
            .with_openrouter_chat_charge(wire.as_bytes())
            .actual_charge
            .unwrap();
        assert_eq!(charge.amount, expected);
        assert_eq!(charge.currency, "USD");
        assert_eq!(charge.provenance, "openrouter_usage_cost");
    }
    for wire in [
        r#"{"usage":{}}"#,
        r#"{"usage":{"cost":null}}"#,
        r#"{"usage":{"cost":"0.2"}}"#,
        r#"{"usage":{"cost":-1}}"#,
        r#"{"usage":{"cost":1e-999}}"#,
        r#"{"usage":{"cost":0.0000000000000000001}}"#,
        r#"{"usage":{"cost":1e999}}"#,
        r#"{"usage":{"cost":0,"cost":1}}"#,
    ] {
        assert!(
            ApiReceipt::default()
                .with_openrouter_chat_charge(wire.as_bytes())
                .actual_charge
                .is_none(),
            "invalid receipt must remain unknown"
        );
    }
    // A generic compatible server's similarly named value has no currency contract.
    assert!(
        ApiReceipt::from_chat_json(&serde_json::json!({"usage":{"cost":0.1}}))
            .actual_charge
            .is_none()
    );
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
