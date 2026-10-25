//! App-owned correlation only. Capture owner IDs stay in memory; reports receive
//! bounded attempt ordinals. Collection generation checks remain in P20 service.
use crate::capture_startup::{
    AcceptedAttempt, CaptureEvent, CaptureObserver, CaptureOutcome, CaptureSpan, CaptureTrace,
    EmissionStamp, MonotonicClock,
};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
struct GlobalObserver;
impl CaptureObserver for GlobalObserver {
    fn accepts(&self, token: crate::capture_startup::CollectionToken) -> bool {
        crate::diagnostics::service::global().capture_token() == Some(token)
    }
    fn record(&self, event: CaptureEvent) {
        crate::diagnostics::service::global().record_capture(event);
    }
}
/// Creates a metadata-only trace using the current effective collection generation.
pub fn begin(accepted: AcceptedAttempt) -> CaptureTrace {
    CaptureTrace::begin(
        accepted,
        crate::diagnostics::service::global().capture_token(),
        Arc::new(MonotonicClock),
        Arc::new(GlobalObserver),
    )
}
#[derive(Clone, Copy, Debug)]
/// Closed set of frontend-local observations, without text or absolute clocks.
pub enum FrontendObservation {
    /// Native ACK arrival gives an emission-to-return upper bound.
    Received,
    /// Same-webview receipt-to-layout-effect local difference.
    Committed,
    /// Same-webview show-to-settings-resolution local difference.
    Settings,
}
struct BoundCapture {
    capture: u64,
    trace: CaptureTrace,
    emitted: Option<(u64, EmissionStamp)>,
    observed: u8,
}
/// Same registry used by native commands; it never owns an audio resource.
pub struct CaptureObservations(Mutex<Option<BoundCapture>>);
impl Default for CaptureObservations {
    fn default() -> Self {
        Self::new()
    }
}
impl CaptureObservations {
    /// Starts an empty in-memory mapping with no audio resources or persisted IDs.
    pub const fn new() -> Self {
        Self(Mutex::new(None))
    }
    /// Binds the real workflow capture; replacement closes only old observations.
    pub fn bind(&self, capture: u64, trace: CaptureTrace) {
        let previous = self
            .0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .replace(BoundCapture {
                capture,
                trace,
                emitted: None,
                observed: 0,
            });
        if let Some(previous) = previous {
            previous.trace.finish(CaptureOutcome::Replaced);
        }
    }
    /// Stores the first emission-attempt boundary without resetting duplicates.
    pub fn emitted(&self, capture: u64, revision: u64, stamp: EmissionStamp) {
        let mut current = self.0.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(bound) = current
            .as_mut()
            .filter(|bound| bound.capture == capture && bound.trace.active())
        {
            // Retransmitted ready does not reset the emission origin/dedup state.
            if bound.emitted.is_none() {
                bound.emitted = Some((revision, stamp));
            }
        }
    }
    /// Marks diagnostics terminal; original recording ownership is unchanged.
    pub fn finish(&self, capture: u64, outcome: CaptureOutcome) {
        if let Some(bound) = self
            .0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .filter(|bound| bound.capture == capture)
        {
            bound.trace.finish(outcome);
        }
    }
    /// Native adapter separately validates the exact current P21 presentation
    /// before entering this correlation boundary. Duplicate/late ACKs are inert.
    pub fn frontend(
        &self,
        capture: u64,
        revision: u64,
        observation: FrontendObservation,
        duration_ms: u64,
    ) -> bool {
        let mut current = self.0.lock().unwrap_or_else(|e| e.into_inner());
        let Some(bound) = current.as_mut().filter(|bound| {
            bound.capture == capture && bound.trace.active() && bound.trace.collecting()
        }) else {
            return false;
        };
        let bit = match observation {
            FrontendObservation::Received => 1,
            FrontendObservation::Committed => 2,
            FrontendObservation::Settings => 4,
        };
        if bound.observed & bit != 0 {
            return false;
        }
        match observation {
            FrontendObservation::Received => {
                let Some((emitted_revision, stamp)) = &bound.emitted else {
                    return false;
                };
                if *emitted_revision != revision {
                    return false;
                }
                stamp.trace.record(
                    CaptureSpan::FrontendRoundTrip,
                    CaptureOutcome::Ok,
                    Some(
                        stamp
                            .trace
                            .now()
                            .saturating_duration_since(stamp.emitted_at),
                    ),
                );
            }
            FrontendObservation::Committed => {
                let Some((emitted_revision, _)) = &bound.emitted else {
                    return false;
                };
                if *emitted_revision != revision {
                    return false;
                }
                bound.trace.record(
                    CaptureSpan::FrontendCommit,
                    CaptureOutcome::Ok,
                    Some(Duration::from_millis(duration_ms)),
                );
            }
            FrontendObservation::Settings => bound.trace.record(
                CaptureSpan::FrontendSettings,
                CaptureOutcome::Ok,
                Some(Duration::from_millis(duration_ms)),
            ),
        }
        bound.observed |= bit;
        true
    }
}
static GLOBAL: OnceLock<CaptureObservations> = OnceLock::new();
/// Process-owned correlation registry; never exported in diagnostic report content.
pub fn global() -> &'static CaptureObservations {
    GLOBAL.get_or_init(CaptureObservations::new)
}
