use super::*;

#[test]
fn actual_app_settings_projection_is_importable_without_legacy_keys() {
    let settings = settings::get_default_settings();
    let projected =
        settings_transfer::project_settings(serde_json::to_value(settings).unwrap()).unwrap();
    assert!(validate_settings(&projected).is_ok());
    assert!(projected.get("post_process_api_keys").is_none());
}
#[test]
fn typed_settings_import_rejects_typos_invalid_ranges_duplicates_and_unsafe_product_modes() {
    let projected = settings_transfer::project_settings(
        serde_json::to_value(settings::get_default_settings()).unwrap(),
    )
    .unwrap();
    for (key, value) in [
        ("typo", json!(true)),
        ("auto_submit", json!(true)),
        ("update_checks_enabled", json!(true)),
        ("hold_threshold_ms", json!(10001)),
        ("audio_feedback_volume", json!(1.1)),
        ("custom_words", json!(["OpenAI", "openai"])),
        ("theme", json!("not-a-theme")),
        ("post_process_api_keys", json!({"provider":"synthetic"})),
    ] {
        let mut invalid = projected.clone();
        invalid[key] = value;
        assert!(validate_settings(&invalid).is_err(), "invalid field: {key}");
    }
    let mut invalid = projected;
    invalid["bindings"]["ask"]["current_binding"] = json!("unsupported-key");
    assert!(validate_settings(&invalid).is_err());
}
