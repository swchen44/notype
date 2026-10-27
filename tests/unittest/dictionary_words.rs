#[path = "../../desktop/src-tauri/src/shortcut/dictionary.rs"]
mod dictionary;

#[test]
fn dictionary_normalizes_and_preserves_the_first_spelling_without_case_duplicates() {
    assert_eq!(
        dictionary::validated_words(vec![
            " OpenAI ".into(),
            "openai".into(),
            " 繁體中文 ".into(),
            "API".into(),
            "api".into()
        ])
        .unwrap(),
        ["OpenAI", "繁體中文", "API"]
    );
}

#[test]
fn invalid_or_excessive_input_is_rejected_as_a_whole() {
    for invalid in [
        "".to_owned(),
        "  ".to_owned(),
        "line\nbreak".to_owned(),
        "nul\0word".to_owned(),
        "字".repeat(51),
    ] {
        assert_eq!(
            dictionary::validated_words(vec!["valid".into(), invalid]).unwrap_err(),
            "dictionary-words-invalid"
        );
    }
    assert_eq!(
        dictionary::validated_words(vec!["字".repeat(50)])
            .unwrap()
            .len(),
        1
    );
    assert!(dictionary::validated_words(vec!["word".into(); 50_001]).is_err());
    assert!(dictionary::validated_words(vec![]).unwrap().is_empty());
}
