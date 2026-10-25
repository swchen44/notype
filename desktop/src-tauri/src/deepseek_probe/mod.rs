//! Independent offline modules for the approved, one-round DeepSeek helper.
//!
//! These are not registered in the App. Production ChatProcessor/SessionService
//! integration, native console adapter, user binary and paid enablement remain
//! behind the complete merged #22 and P23-live-v1 acceptance gates.
pub mod bridge;
pub mod console;
pub mod policy;
pub mod round;
pub mod sqlite_round;
