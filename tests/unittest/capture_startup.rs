#[path = "../../desktop/src-tauri/src/capture_startup.rs"]
mod startup;
use startup::*;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
};
use std::time::{Duration, Instant};

struct Clock {
    origin: Instant,
    millis: AtomicU64,
}
impl Clock {
    fn advance(&self, millis: u64) {
        self.millis.fetch_add(millis, Ordering::SeqCst);
    }
}
impl CaptureClock for Clock {
    fn now(&self) -> Instant {
        self.origin + Duration::from_millis(self.millis.load(Ordering::SeqCst))
    }
}
#[derive(Default)]
struct Observer(Mutex<Vec<CaptureEvent>>);
impl CaptureObserver for Observer {
    fn record(&self, event: CaptureEvent) {
        self.0.lock().unwrap().push(event);
    }
}
struct Recorder {
    clock: Arc<Clock>,
    open_ms: u64,
    signal: Option<ReadySignal>,
}
impl RecorderPort for Recorder {
    type Policy = ();
    type Error = &'static str;
    fn ensure_stream(&mut self) -> Result<StreamDisposition, Self::Error> {
        self.clock.advance(self.open_ms);
        Ok(StreamDisposition::Opened)
    }
    fn request_capture(&mut self, _: (), request: ReadyRequest) -> Result<(), Self::Error> {
        request.send(|_, signal| {
            signal.command_received();
            self.signal = Some(signal);
        });
        Ok(())
    }
}
fn fixture(open: u64, processing: u64, waiter: u64, main: u64) -> Vec<CaptureEvent> {
    let clock = Arc::new(Clock {
        origin: Instant::now(),
        millis: AtomicU64::new(0),
    });
    let observer = Arc::new(Observer::default());
    let trace = CaptureTrace::begin(
        AcceptedAttempt {
            ordinal: Some(7),
            accepted_at: clock.now(),
            dequeued_at: None,
        },
        Some(CollectionToken {
            epoch: 2,
            revision: 3,
        }),
        clock.clone(),
        observer.clone(),
    );
    let mut recorder = Recorder {
        clock: clock.clone(),
        open_ms: open,
        signal: None,
    };
    let ready = start_recording(&mut recorder, (), trace.clone()).unwrap();
    let mut samples_processed = 0;
    process_captured_chunk(&mut recorder.signal, &[0.0; 480], |samples| {
        samples_processed += samples.len();
        clock.advance(processing);
    });
    assert_eq!(
        samples_processed, 480,
        "the real processing closure runs before readiness"
    );
    clock.advance(waiter);
    let dispatch = ready.wait().unwrap().queue();
    clock.advance(main);
    let mut emitted = 0;
    dispatch.commit(|_| {
        emitted += 1;
        true
    });
    assert_eq!(emitted, 1);
    trace.finish(CaptureOutcome::Stopped);
    let events = observer.0.lock().unwrap().clone();
    events
}
fn duration(events: &[CaptureEvent], span: CaptureSpan) -> Option<u64> {
    events
        .iter()
        .find(|event| event.span == span && event.outcome != CaptureOutcome::Started)
        .unwrap()
        .duration_ms
}

#[test]
fn independent_native_and_scheduler_delays_are_reported_in_their_actual_production_spans() {
    for (open, processing, waiter, main) in
        [(120, 0, 0, 0), (0, 80, 0, 0), (0, 0, 90, 0), (0, 0, 0, 110)]
    {
        let events = fixture(open, processing, waiter, main);
        assert_eq!(duration(&events, CaptureSpan::StreamOpen), Some(open));
        assert_eq!(
            duration(&events, CaptureSpan::ChunkProcessing),
            Some(processing)
        );
        assert_eq!(
            duration(&events, CaptureSpan::WaiterScheduling),
            Some(waiter)
        );
        assert_eq!(duration(&events, CaptureSpan::MainQueue), Some(main));
        assert!(events.iter().all(|event| event.attempt == Some(7)
            && event.token
                == CollectionToken {
                    epoch: 2,
                    revision: 3
                }));
    }
    let events = fixture(120, 80, 90, 110);
    assert_eq!(
        duration(&events, CaptureSpan::ProcessedCumulative),
        Some(200)
    );
    assert_eq!(events.last().unwrap().duration_ms, Some(400));
}

#[test]
fn empty_or_cancelled_capture_does_not_fabricate_ready_and_terminal_observations_are_not_replayed()
{
    let clock = Arc::new(Clock {
        origin: Instant::now(),
        millis: AtomicU64::new(0),
    });
    let observer = Arc::new(Observer::default());
    let trace = CaptureTrace::begin(
        AcceptedAttempt {
            ordinal: Some(9),
            accepted_at: clock.now(),
            dequeued_at: None,
        },
        Some(CollectionToken {
            epoch: 0,
            revision: 0,
        }),
        clock.clone(),
        observer.clone(),
    );
    let mut recorder = Recorder {
        clock,
        open_ms: 0,
        signal: None,
    };
    let waiter = start_recording(&mut recorder, (), trace.clone()).unwrap();
    process_captured_chunk(&mut recorder.signal, &[], |_| {});
    assert!(recorder.signal.is_some());
    recorder.signal.take();
    assert!(waiter.wait().is_none());
    trace.cumulative(CaptureSpan::ProcessedCumulative, CaptureOutcome::Ok);
    assert!(!observer
        .0
        .lock()
        .unwrap()
        .iter()
        .any(|event| event.span == CaptureSpan::ProcessedCumulative));
}

#[test]
fn bounded_attempt_ordinals_do_not_wrap_or_reuse_after_exhaustion() {
    let sequence = AttemptSequence::<2>::new();
    assert_eq!(sequence.next(), Some(1));
    assert_eq!(sequence.next(), Some(2));
    assert_eq!(sequence.next(), None);
    assert_eq!(sequence.next(), None);
}

#[test]
fn dispatch_uses_dequeue_boundary_and_command_queue_wait_is_separate_from_waiter_scheduling() {
    let clock = Arc::new(Clock {
        origin: Instant::now(),
        millis: AtomicU64::new(40),
    });
    let observer = Arc::new(Observer::default());
    let trace = CaptureTrace::begin(
        AcceptedAttempt {
            ordinal: Some(12),
            accepted_at: clock.origin,
            dequeued_at: Some(clock.origin + Duration::from_millis(30)),
        },
        Some(CollectionToken {
            epoch: 0,
            revision: 0,
        }),
        clock.clone(),
        observer.clone(),
    );
    assert_eq!(
        duration(&observer.0.lock().unwrap(), CaptureSpan::DispatchQueue),
        Some(30)
    );
    let mut recorder = Recorder {
        clock: clock.clone(),
        open_ms: 0,
        signal: None,
    };
    let waiter = start_recording(&mut recorder, (), trace).unwrap();
    clock.advance(70);
    recorder.signal.as_ref().unwrap().command_received();
    process_captured_chunk(&mut recorder.signal, &[0.0; 480], |_| {});
    assert!(waiter.wait().is_some());
    let events = observer.0.lock().unwrap();
    assert!(events
        .iter()
        .any(|event| event.span == CaptureSpan::CommandQueue && event.duration_ms == Some(70)));
    assert_eq!(duration(&events, CaptureSpan::WaiterScheduling), Some(0));
}

#[test]
fn command_queue_excludes_pre_send_recorder_lock_wait_but_readiness_cumulative_retains_it() {
    struct DelayedRecorder {
        clock: Arc<Clock>,
        signal: Option<ReadySignal>,
    }
    impl RecorderPort for DelayedRecorder {
        type Policy = ();
        type Error = ();
        fn ensure_stream(&mut self) -> Result<StreamDisposition, ()> {
            Ok(StreamDisposition::Reused)
        }
        fn request_capture(&mut self, _: (), ready: ReadyRequest) -> Result<(), ()> {
            // Native adapter waits for its existing recorder guard before send.
            self.clock.advance(120);
            ready.send(|sent_at, signal| {
                assert_eq!(sent_at, self.clock.now());
                self.signal = Some(signal);
            });
            Ok(())
        }
    }
    let clock = Arc::new(Clock {
        origin: Instant::now(),
        millis: AtomicU64::new(0),
    });
    let observer = Arc::new(Observer::default());
    let trace = CaptureTrace::begin(
        AcceptedAttempt {
            ordinal: Some(13),
            accepted_at: clock.now(),
            dequeued_at: None,
        },
        Some(CollectionToken {
            epoch: 0,
            revision: 0,
        }),
        clock.clone(),
        observer.clone(),
    );
    let mut recorder = DelayedRecorder {
        clock: clock.clone(),
        signal: None,
    };
    let waiter = start_recording(&mut recorder, (), trace).unwrap();
    // Consumer scheduling is independent from the adapter's earlier lock wait.
    clock.advance(80);
    recorder.signal.as_ref().unwrap().command_received();
    process_captured_chunk(&mut recorder.signal, &[0.0; 480], |_| {});
    assert!(waiter.wait().is_some());
    let events = observer.0.lock().unwrap();
    assert_eq!(duration(&events, CaptureSpan::CommandQueue), Some(80));
    assert_eq!(duration(&events, CaptureSpan::ActionCumulative), Some(120));
    assert_eq!(
        duration(&events, CaptureSpan::FirstChunkCumulative),
        Some(200)
    );
    assert_eq!(
        duration(&events, CaptureSpan::ProcessedCumulative),
        Some(200)
    );
}
