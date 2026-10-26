#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/managers/model_root_activity.rs"]
mod activity;
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/managers/model_transfer_worker.rs"]
mod worker;
use activity::ActivityGate;

#[test]
fn terminal_callback_is_owned_and_completed_before_the_result_releases_admission() {
    let gate = ActivityGate::new();
    let owner = gate.enter().unwrap();
    let callback_gate = gate.clone();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime
        .block_on(worker::run_owned(
            owner,
            |_| async { "finished" },
            move || {
                assert!(
                    callback_gate.busy(),
                    "terminal callbacks cannot run after releasing root ownership"
                );
            },
        ))
        .unwrap();
    assert!(!gate.busy());
}

#[test]
fn dropping_the_ipc_waiter_keeps_busy_until_an_unabortable_file_job_actually_finishes() {
    use std::sync::{mpsc, Arc};
    struct Owner {
        lease: Option<activity::ActivityLease>,
        ended: mpsc::Sender<()>,
    }
    impl Drop for Owner {
        fn drop(&mut self) {
            drop(self.lease.take());
            let _ = self.ended.send(());
        }
    }
    let parent = std::env::var_os("NOTYPE_TEST_FIXTURE_ROOT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let parent = std::fs::canonicalize(parent).unwrap();
    let folder = parent.join(format!("notype-p28-transfer-{}", std::process::id()));
    std::fs::create_dir(&folder).unwrap();
    let file = folder.join("owned-sentinel");
    let worker_file = file.clone();
    let gate = ActivityGate::new();
    let (ended_tx, ended_rx) = mpsc::channel();
    let owner = Arc::new(Owner {
        lease: Some(gate.enter().unwrap()),
        ended: ended_tx,
    });
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let caller = std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async move {
            let (queued_tx, queued_rx) = tokio::sync::oneshot::channel();
            let waiter = tokio::spawn(worker::run_owned(
                owner,
                move |_| async move {
                    let job = tokio::task::spawn_blocking(move || {
                        started_tx.send(()).unwrap();
                        release_rx.recv().unwrap();
                        std::fs::write(worker_file, b"terminal").unwrap();
                    });
                    drop(job); // An interrupted filesystem await cannot abort this job.
                    let _ = queued_tx.send(());
                },
                || {},
            ));
            queued_rx.await.unwrap();
            waiter.abort();
            let _ = waiter.await;
        });
    });
    started_rx
        .recv_timeout(std::time::Duration::from_secs(3))
        .unwrap();
    caller.join().unwrap(); // The IPC-side waiter is already gone.
    assert!(gate.busy());
    assert!(gate.transition().is_err());
    assert!(!file.exists());
    release_tx.send(()).unwrap();
    ended_rx
        .recv_timeout(std::time::Duration::from_secs(3))
        .unwrap();
    assert_eq!(std::fs::read(&file).unwrap(), b"terminal");
    assert!(!gate.busy());
    assert_eq!(
        std::fs::canonicalize(&folder).unwrap().parent(),
        Some(parent.as_path())
    );
    std::fs::remove_file(file).unwrap();
    std::fs::remove_dir(folder).unwrap();
}
