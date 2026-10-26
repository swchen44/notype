//! Actual sealed transport, authorization journal and production core stores.
use super::probe;
use super::provider_config;
use super::*;
use probe::{
    journal::{Identity, State},
    policy::{DICTATION, REVIEW},
    runner::{CancellationTimer, ProbeRunner},
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};
struct OwnedSpace;
impl probe::bridge::ResourceGate for OwnedSpace {
    fn checkpoint(&self) -> Result<(), probe::journal::Failure> {
        Ok(())
    }
}
fn identity() -> Identity {
    Identity {
        source: "a".repeat(40),
        policy_sha256: "b".repeat(64),
        corpus_sha256: "c".repeat(64),
        contract_sha256: vec!["d".repeat(64); 6],
    }
}
struct Timer {
    entered: Mutex<Option<oneshot::Sender<()>>>,
    release: Mutex<Option<oneshot::Receiver<()>>>,
    terminal: Arc<AtomicBool>,
}
impl CancellationTimer for Timer {
    fn wait(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
        let entered = self.entered.lock().unwrap().take().unwrap();
        let release = self.release.lock().unwrap().take().unwrap();
        let terminal = self.terminal.clone();
        Box::pin(async move {
            struct Completed(Arc<AtomicBool>);
            impl Drop for Completed {
                fn drop(&mut self) {
                    self.0.store(true, Ordering::SeqCst);
                }
            }
            let _completed = Completed(terminal);
            entered.send(()).unwrap();
            let _ = release.await;
        })
    }
}
async fn request(stream: &mut tokio::net::TcpStream) -> Value {
    let mut bytes = vec![];
    let mut buffer = [0; 8192];
    let (end, length) = loop {
        let n = stream.read(&mut buffer).await.unwrap();
        assert!(n > 0);
        bytes.extend_from_slice(&buffer[..n]);
        if let Some(end) = bytes.windows(4).position(|p| p == b"\r\n\r\n") {
            let headers = std::str::from_utf8(&bytes[..end]).unwrap();
            let length = headers
                .lines()
                .find_map(|line| {
                    line.to_ascii_lowercase()
                        .strip_prefix("content-length:")
                        .map(|v| v.trim().parse::<usize>().unwrap())
                })
                .unwrap();
            break (end + 4, length);
        }
    };
    while bytes.len() < end + length {
        let n = stream.read(&mut buffer).await.unwrap();
        assert!(n > 0);
        bytes.extend_from_slice(&buffer[..n]);
    }
    serde_json::from_slice(&bytes[end..end + length]).unwrap()
}
fn reply(slot: usize) -> String {
    let text=match slot {
        1=>"OK".into(),2=>DICTATION.into(),
        3=>"Do not delete session_id; keep timeout at 0.5 seconds, not 5, and retain 42 API requests.".into(),
        4=>"下週會議改到星期三下午兩點，不用準備投影片。".into(),
        5=>json!({"text":"Do not delete API_42; keep timeout at 0.5 seconds.","source_language":"zh-TW"}).to_string(),
        6=>REVIEW.into(),_=>"這是測試資料的短文。保留所有測試。不要引用外部資料。".into(),
    };
    let value=json!({"model":"deepseek-flash","usage":{"prompt_tokens":5,"completion_tokens":2,"total_tokens":7},"choices":[{"finish_reason":"stop","message":{"role":"assistant","content":text}}]}).to_string();
    format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{value}",
        value.len()
    )
}
#[tokio::test]
async fn completion_winning_slot7_fully_joins_its_pending_timer_before_same_history_retry8() {
    use workflow::experiment::OwnedLoopback;
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = OwnedLoopback::from_listener(&listener).unwrap();
    listener.set_nonblocking(true).unwrap();
    let listener = TcpListener::from_std(listener).unwrap();
    let (entered, timer_entered) = oneshot::channel();
    let (release, timer_release) = oneshot::channel();
    let terminal = Arc::new(AtomicBool::new(false));
    let observed_terminal = terminal.clone();
    let timer = Arc::new(Timer {
        entered: Mutex::new(Some(entered)),
        release: Mutex::new(Some(timer_release)),
        terminal: terminal.clone(),
    });
    let server = tokio::spawn(async move {
        let mut entered = Some(timer_entered);
        let mut release = Some(release);
        let mut bodies = vec![];
        for slot in 1..=8 {
            let (mut stream, _) = tokio::time::timeout(Duration::from_secs(4), listener.accept())
                .await
                .unwrap()
                .unwrap();
            bodies.push(request(&mut stream).await);
            if slot == 7 {
                tokio::time::timeout(Duration::from_secs(2), entered.take().unwrap())
                    .await
                    .unwrap()
                    .unwrap();
                assert!(!observed_terminal.load(Ordering::SeqCst));
            }
            if slot == 8 {
                assert!(
                    observed_terminal.load(Ordering::SeqCst),
                    "owned timer must already be terminal before retry HTTP"
                );
                assert!(
                    release.take().unwrap().send(()).is_err(),
                    "joined timer owns no pending receiver"
                );
            }
            stream.write_all(reply(slot).as_bytes()).await.unwrap();
        }
        assert!(
            tokio::time::timeout(Duration::from_millis(75), listener.accept())
                .await
                .is_err()
        );
        bodies
    });
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("owned");
    let mut runner = ProbeRunner::for_owned_fixture(
        &root,
        endpoint,
        identity(),
        diagnostics::api::TraceContext::global(None),
        timer,
        Arc::new(OwnedSpace),
    )
    .unwrap();
    let mut histories = vec![];
    while histories.len() < 7 {
        histories.push(runner.next().await.unwrap().unwrap());
    }
    let retry = &histories[6];
    let cancelled = &histories[5];
    assert_eq!(cancelled.first_slot, 7);
    assert_eq!(retry.first_slot, 8);
    assert_eq!(cancelled.history.id, retry.history.id);
    assert_eq!(retry.history.error, None);
    assert_eq!(retry.history.api_summary.call_count, Some(2));
    assert!(terminal.load(Ordering::SeqCst));
    let state = runner.snapshot().unwrap();
    assert!(state.records[..8]
        .iter()
        .all(|r| r.state == State::Reconciled));
    assert!(!state.records[6].cancellation_requested);
    assert!(state.records[7].processing_revision > state.records[6].processing_revision);
    assert_eq!(state.reservation_units().unwrap(), 835_584);
    assert!(state.records[12..]
        .iter()
        .all(|r| r.state == State::SealedGemini));
    let bodies = server.await.unwrap();
    assert_eq!(bodies.len(), 8);
    assert_eq!(bodies[0]["thinking"]["type"], "disabled");
    assert_eq!(bodies[0]["temperature"].as_f64(), Some(0.0));
    assert!(bodies[0].get("top_p").is_none());
    assert_eq!(histories[4].history.api_summary.call_count, Some(2));
    assert_eq!(
        histories[4].history.back_translation.as_deref(),
        Some(REVIEW)
    );
}
