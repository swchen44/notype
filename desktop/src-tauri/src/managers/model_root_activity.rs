//! Shared root-change admission. Every accepted operation keeps an owned lease;
//! cancellation is a request, and only the last worker owner releases admission.
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Busy;

#[derive(Default)]
struct State {
    active: usize,
    transitioning: bool,
}

#[derive(Clone, Default)]
pub struct ActivityGate(Arc<Mutex<State>>);
impl ActivityGate {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn enter(&self) -> Result<ActivityLease, Busy> {
        let mut state = self.0.lock().unwrap_or_else(|error| error.into_inner());
        if state.transitioning {
            return Err(Busy);
        }
        state.active += 1;
        Ok(ActivityLease(Arc::new(ActivityOwner(self.clone()))))
    }
    pub fn transition(&self) -> Result<IdleTransition, Busy> {
        let mut state = self.0.lock().unwrap_or_else(|error| error.into_inner());
        if state.transitioning || state.active != 0 {
            return Err(Busy);
        }
        state.transitioning = true;
        Ok(IdleTransition(self.clone()))
    }
    pub fn busy(&self) -> bool {
        let state = self.0.lock().unwrap_or_else(|error| error.into_inner());
        state.transitioning || state.active != 0
    }
}

struct ActivityOwner(ActivityGate);
impl Drop for ActivityOwner {
    fn drop(&mut self) {
        let mut state = self.0 .0.lock().unwrap_or_else(|error| error.into_inner());
        state.active -= 1;
    }
}
#[derive(Clone)]
pub struct ActivityLease(Arc<ActivityOwner>);
impl ActivityLease {
    /// A child keeps the same admitted operation alive, including while queued.
    pub fn worker(&self) -> Self {
        self.clone()
    }
    pub fn is_held(&self) -> bool {
        Arc::strong_count(&self.0) != 0
    }
}

pub struct IdleTransition(ActivityGate);
impl Drop for IdleTransition {
    fn drop(&mut self) {
        self.0
             .0
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .transitioning = false;
    }
}
