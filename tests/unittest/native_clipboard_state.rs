#[path = "../../desktop/src-tauri/src/native_clipboard_state.rs"]
mod state;
use notype_core::DeliveryOutcome;
use state::{completion, Restoration};

#[test]
fn equal_length_metadata_never_confirms_a_different_clipboard_owner() {
    assert_eq!(
        completion(true, true, false, Restoration::Superseded),
        DeliveryOutcome::Uncertain
    );
    assert_eq!(
        completion(true, true, false, Restoration::Restored),
        DeliveryOutcome::Uncertain
    );
    assert_eq!(
        completion(true, true, true, Restoration::Superseded),
        DeliveryOutcome::Uncertain
    );
}

#[test]
fn delivered_requires_handler_target_source_and_complete_restoration() {
    assert_eq!(
        completion(true, true, true, Restoration::Restored),
        DeliveryOutcome::Delivered
    );
    assert_eq!(
        completion(false, true, true, Restoration::Restored),
        DeliveryOutcome::Uncertain
    );
    assert_eq!(
        completion(true, false, true, Restoration::Restored),
        DeliveryOutcome::Uncertain
    );
    assert_eq!(
        completion(true, true, true, Restoration::Failed),
        DeliveryOutcome::Uncertain
    );
}
