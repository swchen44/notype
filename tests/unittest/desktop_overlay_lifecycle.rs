#[path = "../../desktop/src-tauri/src/overlay_lifecycle.rs"]
mod lifecycle;
use lifecycle::{CaptureId, OverlayLifecycle, Presentation};

#[test]
fn a_queued_ready_or_hide_cannot_mutate_a_successor_capture() {
    let mut lifecycle = OverlayLifecycle::new();
    let ready = lifecycle
        .reserve(CaptureId(1), Presentation::Recording)
        .unwrap();
    let hide = lifecycle
        .reserve(CaptureId(1), Presentation::Hidden)
        .unwrap();
    let successor = lifecycle
        .reserve(CaptureId(2), Presentation::Streaming)
        .unwrap();
    let mut visible = false;
    lifecycle.commit(successor, || visible = true);
    assert_eq!(lifecycle.ready(ready, || "stale-ready"), None);
    lifecycle.commit(hide, || visible = false);
    assert!(visible, "delayed native hide must not hide its successor");
}

#[test]
fn capture_identity_survives_duplicate_show_but_ready_does_not_survive_working() {
    let mut lifecycle = OverlayLifecycle::new();
    let capture = CaptureId(7);
    let shown = lifecycle.reserve(capture, Presentation::Streaming).unwrap();
    assert_eq!(
        lifecycle.reserve(capture, Presentation::Streaming),
        Some(shown)
    );
    assert_eq!(lifecycle.ready(shown, || "ready"), Some("ready"));
    let working = lifecycle
        .reserve(capture, Presentation::StreamTranscribing)
        .unwrap();
    assert_eq!(working.capture, capture);
    assert_ne!(working.revision, shown.revision);
    assert_eq!(lifecycle.ready(shown, || "late-ready"), None);
    assert_eq!(lifecycle.reserve(capture, Presentation::Streaming), None);
    assert_eq!(lifecycle.commit(shown, || "queued-show"), None);
    assert_eq!(lifecycle.text(CaptureId(6), || "stale text"), None);
    assert_eq!(lifecycle.text(capture, || "final text"), Some("final text"));
    let processing = lifecycle
        .reserve(capture, Presentation::StreamProcessing)
        .unwrap();
    assert_eq!(lifecycle.commit(working, || "old phase"), None);
    assert_eq!(
        lifecycle.commit(processing, || "processing"),
        Some("processing")
    );
}

#[test]
fn terminal_capture_cannot_be_revived_and_delayed_hide_is_fenced_by_exact_request() {
    let mut lifecycle = OverlayLifecycle::new();
    let capture = CaptureId(9007199254740993);
    let shown = lifecycle.reserve(capture, Presentation::Recording).unwrap();
    lifecycle
        .reserve(capture, Presentation::Transcribing)
        .unwrap();
    lifecycle
        .reserve(capture, Presentation::Processing)
        .unwrap();
    let hidden = lifecycle.reserve(capture, Presentation::Hidden).unwrap();
    assert_eq!(lifecycle.reserve(capture, Presentation::Recording), None);
    assert_eq!(lifecycle.ready(shown, || ()), None);
    assert_eq!(lifecycle.text(capture, || "late"), None);
    assert_eq!(
        lifecycle.commit(hidden, || "native-hide"),
        Some("native-hide")
    );
    lifecycle
        .reserve(CaptureId(capture.0 + 1), Presentation::Recording)
        .unwrap();
    assert_eq!(lifecycle.commit(hidden, || "stale-native-hide"), None);
}

#[test]
fn observation_requires_exact_capturing_envelope_and_never_runs_rejected_closures() {
    for presentation in [Presentation::Recording, Presentation::Streaming] {
        let mut lifecycle = OverlayLifecycle::new();
        let current = lifecycle.reserve(CaptureId(10), presentation).unwrap();
        let mut calls = 0;
        assert_eq!(
            lifecycle.observe(current.capture, current.revision, || {
                calls += 1;
                "accepted"
            }),
            Some("accepted")
        );
        assert_eq!(calls, 1, "matching envelope is the positive control");
        assert_eq!(
            lifecycle.observe(current.capture, current.revision + 1, || calls += 1),
            None
        );
        assert_eq!(calls, 1, "wrong revision must not execute its closure");
        assert_eq!(
            lifecycle.observe(CaptureId(9), current.revision, || calls += 1),
            None
        );
        assert_eq!(calls, 1, "wrong capture must not execute its closure");
    }
}

#[test]
fn observation_rejects_working_hidden_and_successor_envelopes_without_running_closures() {
    let mut lifecycle = OverlayLifecycle::new();
    let capture = CaptureId(11);
    let old = lifecycle.reserve(capture, Presentation::Recording).unwrap();
    let mut calls = 0;
    for stage in [
        Presentation::Transcribing,
        Presentation::Processing,
        Presentation::Hidden,
    ] {
        let current = lifecycle.reserve(capture, stage).unwrap();
        assert_eq!(
            lifecycle.observe(capture, current.revision, || calls += 1),
            None,
            "even the current revision cannot authorize a noncapturing stage"
        );
        assert_eq!(calls, 0);
    }
    let successor = lifecycle
        .reserve(CaptureId(12), Presentation::Streaming)
        .unwrap();
    assert_eq!(
        lifecycle.observe(old.capture, old.revision, || calls += 1),
        None
    );
    assert_eq!(calls, 0, "old ACK cannot run against a successor capture");
    assert_eq!(
        lifecycle.observe(successor.capture, successor.revision, || calls += 1),
        Some(())
    );
    assert_eq!(calls, 1, "successor's own capturing envelope remains valid");
}
