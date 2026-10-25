#[path = "../../desktop/src-tauri/src/deepseek_probe/policy.rs"]
mod policy;

use policy::{validate_wire, Denied, WireFacts};

fn approved_wire() -> WireFacts<'static> {
    WireFacts {
        endpoint: "https://api.deepseek.com/chat/completions",
        model: "deepseek-flash",
        deepseek_profile: true,
        effort: Some("none"),
        thinking: Some(false),
        non_streaming: true,
        max_output_tokens: Some(512),
        wire_bytes: 8192,
        message_count: 4,
        plain_text_only: true,
    }
}

#[test]
fn complete_wire_over_limit_is_denied_even_with_a_short_sample() {
    let mut wire = approved_wire();
    assert_eq!(validate_wire(&wire), Ok(()));
    // This includes all production prompts and the review envelope, not only
    // the fixed short sample. The 8192-byte boundary comes from approved v1.
    wire.wire_bytes = 8193;
    assert_eq!(validate_wire(&wire), Err(Denied::WireContract));
}

#[test]
fn defaults_and_expanded_request_capabilities_never_enter_this_round() {
    let mutations: [fn(&mut WireFacts<'_>); 11] = [
        |w| w.endpoint = "https://api.deepseek.com/beta/chat/completions",
        |w| w.model = "deepseek-v4-pro",
        |w| w.deepseek_profile = false,
        |w| w.effort = None,
        |w| w.effort = Some("high"),
        |w| w.thinking = None,
        |w| w.thinking = Some(true),
        |w| w.non_streaming = false,
        |w| w.max_output_tokens = Some(513),
        |w| w.message_count = 5,
        |w| w.plain_text_only = false,
    ];
    for mutation in mutations {
        let mut wire = approved_wire();
        mutation(&mut wire);
        assert_eq!(validate_wire(&wire), Err(Denied::WireContract));
    }
    let mut empty = approved_wire();
    empty.wire_bytes = 0;
    assert_eq!(validate_wire(&empty), Err(Denied::WireContract));
    let mut no_messages = approved_wire();
    no_messages.message_count = 0;
    assert_eq!(validate_wire(&no_messages), Err(Denied::WireContract));
}

#[test]
fn eight_slots_include_review_and_retry_without_inventing_new_histories() {
    use policy::{schedule, Purpose};
    let schedule = schedule();
    assert_eq!(schedule.len(), 8);
    assert_eq!(
        schedule.iter().map(|s| s.history_group).collect::<Vec<_>>(),
        [1, 2, 3, 4, 5, 5, 6, 6]
    );
    assert_eq!(schedule[0].sample, "只回答 OK。");
    assert_eq!(schedule[5].purpose, Purpose::BackTranslation);
    assert_eq!(schedule[5].sample, ""); // Must use generated forward text.
    assert!(schedule[4].review);
    assert_eq!(schedule[6].purpose, Purpose::Cancellation);
    assert_eq!(schedule[7].purpose, Purpose::Retry);
    assert_eq!(schedule[7].sample, schedule[6].sample);
}
