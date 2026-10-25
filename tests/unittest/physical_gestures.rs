#[path = "../../desktop/src-tauri/src/physical_gestures.rs"]
mod gestures;
use gestures::{Bindings, GestureRouter, Intent, Phase};

#[test]
fn tap_commits_after_release_and_slow_shared_prefix_selects_only_translation() {
    let mut router = GestureRouter::new(Bindings::default());
    assert_eq!(router.edge(0xa5, true, Phase::Idle), None);
    // No wall-clock cutoff: a suffix can arrive arbitrarily later.
    assert_eq!(router.edge(0xa1, true, Phase::Idle), None);
    assert_eq!(router.edge(0xa5, false, Phase::Idle), None);
    assert_eq!(
        router.edge(0xa1, false, Phase::Idle),
        Some(Intent::Translation)
    );
}

#[test]
fn cancel_drains_partial_chord_and_busy_gestures_cannot_be_replayed_after_processing() {
    let mut router = GestureRouter::new(Bindings::default());
    router.edge(0xa5, true, Phase::Recording);
    assert_eq!(
        router.edge(0x1b, true, Phase::Recording),
        Some(Intent::Cancel)
    );
    assert_eq!(router.edge(0x1b, false, Phase::Idle), None);
    assert_eq!(router.edge(0xa5, false, Phase::Idle), None);
    router.edge(0xa5, true, Phase::Busy);
    assert_eq!(router.edge(0xa5, false, Phase::Idle), None);
    router.edge(0xa5, true, Phase::Idle);
    router.reset();
    assert_eq!(router.edge(0xa5, false, Phase::Idle), None);
    router.edge(0xa5, true, Phase::Idle);
    assert_eq!(
        router.edge(0xa5, false, Phase::Idle),
        Some(Intent::Dictation)
    );
}

fn gesture(router: &mut GestureRouter, keys: &[(u16, bool)], phase: Phase) -> Vec<Intent> {
    keys.iter()
        .filter_map(|&(key, down)| router.edge(key, down, phase))
        .collect()
}
#[test]
fn physical_chords_preserve_sides_repeats_and_common_stop_is_state_dependent() {
    let cases = [
        (
            vec![(0xa5, true), (0xa5, false)],
            Phase::Idle,
            vec![Intent::Dictation],
        ),
        (
            vec![(0xa1, true), (0xa5, true), (0xa1, false), (0xa5, false)],
            Phase::Idle,
            vec![Intent::Translation],
        ),
        (
            vec![
                (0xa5, true),
                (0x20, true),
                (0x20, true),
                (0x20, false),
                (0xa5, false),
            ],
            Phase::Idle,
            vec![Intent::Ask],
        ),
        (
            vec![(0xa5, true), (0x20, true), (0xa5, false), (0x20, false)],
            Phase::Idle,
            vec![Intent::Ask],
        ),
        (
            vec![
                (0xa5, true),
                (0xa1, true),
                (0xa1, false),
                (0xa1, true),
                (0xa1, false),
                (0xa5, false),
            ],
            Phase::Idle,
            vec![Intent::Translation],
        ),
        (
            vec![
                (0xa5, true),
                (0xa1, true),
                (0x20, true),
                (0xa1, false),
                (0xa5, false),
                (0x20, false),
            ],
            Phase::Idle,
            vec![],
        ),
        (vec![(0xa4, true), (0xa4, false)], Phase::Idle, vec![]),
        (
            vec![(0xa5, true), (0xa0, true), (0xa0, false), (0xa5, false)],
            Phase::Idle,
            vec![],
        ),
        (
            vec![(0xa5, true), (0xa2, true), (0xa2, false), (0xa5, false)],
            Phase::Idle,
            vec![],
        ),
        (
            vec![(0xa5, true), (0xa5, false)],
            Phase::Recording,
            vec![Intent::Stop],
        ),
        (
            vec![(0xa5, true), (0xa1, true), (0xa1, false), (0xa5, false)],
            Phase::Recording,
            vec![],
        ),
        (
            vec![(0xa5, true), (0x20, true), (0x20, false), (0xa5, false)],
            Phase::Recording,
            vec![],
        ),
        (vec![(0xa5, true), (0xa5, false)], Phase::Busy, vec![]),
    ];
    for (keys, phase, want) in cases {
        assert_eq!(
            gesture(&mut GestureRouter::new(Bindings::default()), &keys, phase),
            want,
            "{keys:?} {phase:?}"
        );
    }
}

#[test]
fn independent_stop_binding_and_conflict_validation_do_not_change_start_scenarios() {
    let bindings = Bindings::parse(
        "alt_right",
        "alt_right+shift_right",
        "alt_right+space",
        "f9",
    )
    .unwrap();
    let mut router = GestureRouter::new(bindings);
    assert!(gesture(
        &mut router,
        &[(0xa5, true), (0xa5, false)],
        Phase::Recording
    )
    .is_empty());
    assert_eq!(
        gesture(
            &mut router,
            &[(0x78, true), (0x78, false)],
            Phase::Recording
        ),
        vec![Intent::Stop]
    );
    assert_eq!(
        gesture(&mut router, &[(0xa5, true), (0xa5, false)], Phase::Idle),
        vec![Intent::Dictation]
    );
    assert!(Bindings::parse("alt_right", "alt_right", "alt_right+space", "alt_right").is_err());
    assert!(Bindings::parse("escape", "f8", "f9", "f10").is_err());
    assert!(Bindings::parse("ctrl+space", "f8", "f9", "f10").is_err());
    assert!(Bindings::parse("ctrl_left+f", "f8", "f9", "f10").is_ok());
}

#[test]
fn stop_only_key_passes_through_when_idle_and_chord_suppression_pairs_each_edge() {
    let mut router = GestureRouter::new(
        Bindings::parse(
            "alt_right",
            "alt_right+shift_right",
            "alt_right+space",
            "f9",
        )
        .unwrap(),
    );
    assert!(!router.event(0x78, true, Phase::Idle).suppress);
    assert!(!router.event(0x78, false, Phase::Idle).suppress);
    assert!(!router.event(0xa1, true, Phase::Idle).suppress); // ordinary right Shift
    assert!(router.event(0xa5, true, Phase::Idle).suppress);
    assert!(!router.event(0xa1, false, Phase::Idle).suppress); // paired with passed down
    let committed = router.event(0xa5, false, Phase::Idle);
    assert!(committed.suppress);
    assert_eq!(committed.intent, Some(Intent::Translation));
    assert!(router.event(0xa5, true, Phase::Recording).suppress);
    assert!(router.event(0x20, true, Phase::Recording).suppress);
    assert!(router.event(0xa5, false, Phase::Recording).suppress);
    assert_eq!(router.event(0x20, false, Phase::Recording).intent, None);
}

#[test]
fn shortcut_editing_forwards_new_keys_but_drains_a_previously_suppressed_modifier() {
    let mut router = GestureRouter::new(Bindings::default());
    assert!(router.event(0xa5, true, Phase::Idle).suppress);
    router.reset();
    assert!(router.event(0xa5, false, Phase::Suspended).suppress);
    assert!(!router.event(0xa5, true, Phase::Suspended).suppress);
    assert_eq!(router.event(0xa5, false, Phase::Idle).intent, None);
}
