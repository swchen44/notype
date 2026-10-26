#[path = "../../desktop/src-tauri/src/runtime_startup.rs"]
mod runtime_startup;
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc, Mutex,
};
type OwnedWorker = (std::sync::mpsc::Sender<()>, std::thread::JoinHandle<()>);
#[derive(Default)]
struct OwnedWorkers(Mutex<Vec<OwnedWorker>>);
impl OwnedWorkers {
    fn start(&self, started: Arc<AtomicUsize>, live: Arc<AtomicUsize>) {
        let (stop, receive) = std::sync::mpsc::channel();
        let (ready, started_ack) = std::sync::mpsc::channel();
        let thread = std::thread::spawn(move || {
            started.fetch_add(1, Ordering::SeqCst);
            live.fetch_add(1, Ordering::SeqCst);
            ready.send(()).unwrap();
            let _ = receive.recv();
            live.fetch_sub(1, Ordering::SeqCst);
        });
        self.0.lock().unwrap().push((stop, thread));
        started_ack.recv().unwrap();
    }
    fn drain(&self) {
        for (stop, thread) in self.0.lock().unwrap().drain(..) {
            let _ = stop.send(());
            thread.join().unwrap();
        }
    }
}
impl Drop for OwnedWorkers {
    fn drop(&mut self) {
        self.drain();
    }
}

#[test]
fn repeated_audio_failure_never_starts_watcher_then_success_shares_router_and_publishes_once() {
    let factory = runtime_startup::RuntimeStartup::new();
    let started = Arc::new(AtomicUsize::new(0));
    let workers = OwnedWorkers::default();
    let live = Arc::new(AtomicUsize::new(0));
    let ready = AtomicBool::new(false);
    for _ in 0..3 {
        let result = factory.initialize(
            || ready.load(Ordering::SeqCst),
            || Arc::new(()),
            |_| Err::<Arc<()>, _>("audio-unavailable"),
            |router| {
                workers.start(started.clone(), live.clone());
                Ok::<_, &str>(router)
            },
            |_, _| {
                ready.store(true, Ordering::SeqCst);
                Ok(())
            },
        );
        assert_eq!(result, Err("audio-unavailable"));
    }
    // Drain owned worker doubles even on RED; no orphan threads survive the test.
    workers.drain();
    assert_eq!(
        started.load(Ordering::SeqCst),
        0,
        "failed audio preparation must start no transcription workers"
    );
    assert!(!ready.load(Ordering::SeqCst));
    assert_eq!(live.load(Ordering::SeqCst), 0);
    let result = factory.initialize(
        || ready.load(Ordering::SeqCst),
        || Arc::new(()),
        Ok::<_, &str>,
        |router| {
            workers.start(started.clone(), live.clone());
            Ok(router)
        },
        |audio, transcription| {
            assert!(Arc::ptr_eq(&audio, &transcription));
            ready.store(true, Ordering::SeqCst);
            Ok(())
        },
    );
    assert_eq!(result, Ok(true));
    assert_eq!(started.load(Ordering::SeqCst), 1);
    assert_eq!(live.load(Ordering::SeqCst), 1);
    assert_eq!(
        factory.initialize(
            || ready.load(Ordering::SeqCst),
            || panic!("duplicate router"),
            |_: ()| Ok::<_, &str>(()),
            |_| Ok(()),
            |_, _| Ok(())
        ),
        Ok(false)
    );
    workers.drain();
    assert_eq!(live.load(Ordering::SeqCst), 0);
}

#[test]
fn concurrent_retries_share_one_complete_publication() {
    let factory = Arc::new(runtime_startup::RuntimeStartup::new());
    let ready = Arc::new(AtomicBool::new(false));
    let prepared = Arc::new(AtomicUsize::new(0));
    let published = Arc::new(AtomicUsize::new(0));
    let handles = (0..8)
        .map(|_| {
            let (factory, ready, prepared, published) = (
                factory.clone(),
                ready.clone(),
                prepared.clone(),
                published.clone(),
            );
            std::thread::spawn(move || {
                factory
                    .initialize(
                        || ready.load(Ordering::SeqCst),
                        || Arc::new(()),
                        |router| {
                            prepared.fetch_add(1, Ordering::SeqCst);
                            Ok::<_, &str>(router)
                        },
                        Ok,
                        |audio, transcription| {
                            assert!(Arc::ptr_eq(&audio, &transcription));
                            published.fetch_add(1, Ordering::SeqCst);
                            ready.store(true, Ordering::SeqCst);
                            Ok(())
                        },
                    )
                    .unwrap()
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(
        handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .filter(|created| *created)
            .count(),
        1
    );
    assert_eq!(prepared.load(Ordering::SeqCst), 1);
    assert_eq!(published.load(Ordering::SeqCst), 1);
}

#[test]
fn duplicate_retry_cannot_enter_readiness_check_while_audio_is_preparing() {
    use std::{sync::mpsc, time::Duration};
    let factory = Arc::new(runtime_startup::RuntimeStartup::new());
    let ready = Arc::new(AtomicBool::new(false));
    let (audio_started, started) = mpsc::channel();
    let (release, audio_release) = mpsc::channel();
    let first = {
        let factory = factory.clone();
        let ready = ready.clone();
        std::thread::spawn(move || {
            factory
                .initialize(
                    || ready.load(Ordering::SeqCst),
                    || Arc::new(()),
                    |router| {
                        audio_started.send(()).unwrap();
                        audio_release.recv().unwrap();
                        Ok::<_, &str>(router)
                    },
                    Ok,
                    |audio, transcription| {
                        assert!(Arc::ptr_eq(&audio, &transcription));
                        ready.store(true, Ordering::SeqCst);
                        Ok(())
                    },
                )
                .unwrap()
        })
    };
    started.recv().unwrap();
    let (attempting, attempted) = mpsc::channel();
    let (checked, readiness_checked) = mpsc::channel();
    let second = {
        let factory = factory.clone();
        let ready = ready.clone();
        std::thread::spawn(move || {
            attempting.send(()).unwrap();
            factory
                .initialize(
                    || {
                        let value = ready.load(Ordering::SeqCst);
                        checked.send(value).unwrap();
                        value
                    },
                    || panic!("duplicate router"),
                    |_: ()| Ok::<_, &str>(()),
                    |_| Ok(()),
                    |_, _| Ok(()),
                )
                .unwrap()
        })
    };
    attempted.recv().unwrap();
    let entered_while_preparing = readiness_checked
        .recv_timeout(Duration::from_millis(30))
        .ok();
    release.send(()).unwrap();
    let results = (first.join().unwrap(), second.join().unwrap());
    assert_eq!(entered_while_preparing, None);
    assert_eq!(results, (true, false));
    assert!(readiness_checked.recv().unwrap());
}
