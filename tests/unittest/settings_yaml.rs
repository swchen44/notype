#[path = "../../desktop/src-tauri/src/settings_yaml.rs"]
mod settings_yaml;
use serde_json::json;
use settings_yaml::{parse, render, Error, MAX_BYTES};

#[test]
fn human_readable_yaml_round_trips_chinese_multiline_and_literal_references() {
    let value = json!({"format":"notype-settings", "schema":1, "enabled":false,
        "app_keys":null, "temperature":0.0, "top_p":0.5, "maximum":18446744073709551615u64,
        "words":["繁體中文", "简体中文", "OpenAI"],
        "prompt":"第一行\nKeep English.\n${GEMINI_API_KEY} stays literal.",
        "home_key_variables":{"fixture":"GEMINI_API_KEY"}});
    let text = render(&value).unwrap();
    assert!(
        !text.trim_start().starts_with('{'),
        "must emit readable YAML, not a JSON-only substitute"
    );
    assert!(text.contains("繁體中文"));
    assert_eq!(parse(text.as_bytes()).unwrap(), value);
    let plain = "format: notype-settings\nschema: 1\nenabled: false\napp_keys: null\nwords:\n  - 繁體中文\n  - English\n";
    assert_eq!(
        parse(plain.as_bytes()).unwrap()["words"],
        json!(["繁體中文", "English"])
    );
    let bom = format!("\u{feff}{plain}");
    assert_eq!(parse(bom.as_bytes()).unwrap()["schema"], 1);
}

#[test]
fn ambiguous_expanding_or_external_yaml_fails_without_echoing_its_contents() {
    let deep = format!("{}0{}", "[".repeat(100), "]".repeat(100));
    let cases = [
        "field: 1\nfield: 2".to_owned(),
        "{field: 1, field: 2}".to_owned(),
        "1: first\n\"1\": second".to_owned(),
        "null: first\n\"null\": second".to_owned(),
        "field: &a SECRET_SENTINEL\nother: *a".to_owned(),
        "field: &a unused_anchor".to_owned(),
        "field: !include C:/SECRET_SENTINEL.env".to_owned(),
        "field: !unknown SECRET_SENTINEL".to_owned(),
        "{<<: {field: 1}}".to_owned(),
        "---\nfield: 1\n---\nfield: 2".to_owned(),
        "? [a, b]\n: SECRET_SENTINEL".to_owned(),
        "field: .nan".to_owned(),
        deep,
    ];
    for text in cases {
        let error =
            parse(text.as_bytes()).expect_err("ambiguous or unsupported syntax must be rejected");
        assert_eq!(error.to_string(), "settings-yaml-invalid");
        assert!(!format!("{error:?}").contains("SECRET_SENTINEL"));
    }
    assert_eq!(parse(&[0xff, 0xfe]).unwrap_err(), Error::Invalid);
}

#[test]
fn byte_and_render_structure_limits_are_bounded() {
    let input = vec![b' '; MAX_BYTES + 1];
    assert_eq!(parse(&input).unwrap_err(), Error::TooLarge);
    let value = json!({"text":"x".repeat(MAX_BYTES + 1)});
    assert_eq!(render(&value).unwrap_err(), Error::TooLarge);
    let mut deep = json!(0);
    for _ in 0..100 {
        deep = json!([deep]);
    }
    assert_eq!(render(&deep).unwrap_err(), Error::Invalid);
    let wide = serde_json::Value::Array(vec![json!(0); 200_001]);
    assert_eq!(render(&wide).unwrap_err(), Error::Invalid);
}
