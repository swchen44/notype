#[path = "../../desktop/src-tauri/src/runtime_startup.rs"]
mod runtime_startup;
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc, Mutex,
};

#[test]
fn repeated_audio_failure_never_starts_watcher_then_success_shares_router_and_publishes_once() {
    let factory = runtime_startup::RuntimeStartup::new();
    let started = Arc::new(AtomicUsize::new(0));
    let workers = Mutex::new(Vec::new());
    let ready = AtomicBool::new(false);
    let history_and_markers = Arc::new(vec!["owned-history", "root-applied", "provider-applied"]);
    let before = history_and_markers.clone();
    for _ in 0..3 {
        let result = factory.initialize(
            || ready.load(Ordering::SeqCst),
            || Arc::new(()),
            |_| Err::<Arc<()>, _>("audio-unavailable"),
            |router| {
                let started = started.clone();
                let (stop, receive) = std::sync::mpsc::channel();
                let thread = std::thread::spawn(move || {
                    started.fetch_add(1, Ordering::SeqCst);
                    let _ = receive.recv();
                });
                workers.lock().unwrap().push((stop, thread));
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
    for (stop, thread) in workers.lock().unwrap().drain(..) {
        stop.send(()).unwrap();
        thread.join().unwrap();
    }
    assert_eq!(
        started.load(Ordering::SeqCst),
        0,
        "failed audio preparation must start no transcription workers"
    );
    assert!(!ready.load(Ordering::SeqCst));
    assert_eq!(*history_and_markers, *before);
    let result = factory.initialize(
        || ready.load(Ordering::SeqCst),
        || Arc::new(()),
        |router| Ok::<_, &str>(router),
        |router| {
            started.fetch_add(1, Ordering::SeqCst);
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
