//! Owned transfer execution. Dropping an IPC waiter does not drop its work.
//! The owner survives runtime shutdown, including filesystem blocking jobs that
//! cannot be aborted when their calling async future is cancelled.
use std::future::Future;

#[derive(Debug)]
pub struct WorkerFailed;

struct Terminal<W, Done: FnOnce()> {
    owner: W,
    done: Option<Done>,
}
impl<W, Done: FnOnce()> Drop for Terminal<W, Done> {
    fn drop(&mut self) {
        if let Some(done) = self.done.take() {
            done();
        }
    }
}

pub async fn run_owned<W, T, F, Fut, Done>(
    owner: W,
    operation: F,
    done: Done,
) -> Result<T, WorkerFailed>
where
    W: Clone + Send + 'static,
    T: Send + 'static,
    F: FnOnce(W) -> Fut + Send + 'static,
    Fut: Future<Output = T> + 'static,
    Done: FnOnce() + Send + 'static,
{
    let (sender, receiver) = tokio::sync::oneshot::channel();
    // Also cleans registration if starting the OS worker fails. The callback
    // always runs before the owner field is dropped, preventing a successor
    // from being admitted while its predecessor is still removing flags.
    let terminal = Terminal {
        owner,
        done: Some(done),
    };
    std::thread::Builder::new()
        .name("notype-model-transfer".into())
        .spawn(move || {
            let result = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .max_blocking_threads(1)
                .build()
            {
                Ok(runtime) => {
                    let worker_owner = terminal.owner.clone();
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        runtime.block_on(operation(worker_owner))
                    }))
                    .map_err(|_| WorkerFailed);
                    // Runtime Drop waits for actual blocking jobs, unlike dropping
                    // their JoinHandle or an interrupted tokio::fs future.
                    drop(runtime);
                    result
                }
                Err(_) => Err(WorkerFailed),
            };
            drop(terminal);
            let _ = sender.send(result);
        })
        .map_err(|_| WorkerFailed)?;
    receiver.await.map_err(|_| WorkerFailed)?
}
