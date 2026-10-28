use super::*;

#[test]
fn restoring_dictation_requests_fluent_cleanup_without_touching_other_scenes() {
    let mut config = TextConfiguration::default();
    config
        .set_prompt(notype_core::Scenario::Dictation, "custom".into())
        .unwrap();
    config
        .set_prompt(
            notype_core::Scenario::Translation,
            "translation custom".into(),
        )
        .unwrap();
    config.reset_prompt(notype_core::Scenario::Dictation);
    let prompt = config.prompt(notype_core::Scenario::Dictation);
    assert!(prompt.contains("Actively remove verbal fillers"));
    assert!(prompt.contains("Smooth awkward spoken grammar"));
    assert!(prompt.contains("negation"));
    assert_eq!(
        config.prompt(notype_core::Scenario::Translation),
        "translation custom"
    );
}
