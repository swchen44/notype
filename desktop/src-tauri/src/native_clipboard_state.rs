//! A paste receipt confirms only the source owned through completion and restoration.
use notype_core::DeliveryOutcome;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Restoration {
    Restored,
    Superseded,
    Failed,
}

/// Ownership loss makes a metadata-compatible write ambiguous; never retry it.
pub fn completion(
    handler_completed: bool,
    target_matches: bool,
    owned_on_completion: bool,
    restoration: Restoration,
) -> DeliveryOutcome {
    if handler_completed
        && target_matches
        && owned_on_completion
        && restoration == Restoration::Restored
    {
        DeliveryOutcome::Delivered
    } else {
        DeliveryOutcome::Uncertain
    }
}
