//! Actual sealed transport, authorization journal and production core stores.
use super::probe;
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
impl notype_core::csv::CsvSpaceProbe for OwnedSpace {
    fn available_bytes(
        &self,
        _: &std::path::Path,
    ) -> Result<u64, notype_core::csv::CsvExportError> {
        Ok(64 * 1024 * 1024)
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
    let destination = root.join("results/history.csv");
    let proof = runner.export(&destination, &OwnedSpace).unwrap();
    assert_eq!(proof.histories, 6);
    assert_eq!(proof.observed_calls, 8);
    assert!(proof.actual_fees_unknown);
    let bytes = std::fs::read(destination).unwrap();
    let text = String::from_utf8(bytes.clone()).unwrap();
    assert!(!text.contains("Bearer "));
    let corrupted = text.replacen(
        "\"unknown\",\"[]\",\"[]\",\"0\"",
        "\"complete\",\"[]\",\"[]\",\"0\"",
        1,
    );
    assert_ne!(corrupted.as_bytes(), bytes);
    assert!(probe::export::verify(
        corrupted.as_bytes(),
        &runner.sessions().search("").unwrap(),
        &state
    )
    .is_err());
}

#[tokio::test]
async fn actual_csv_replace_failure_halts_next_request_and_preserves_core_journal() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = workflow::experiment::OwnedLoopback::from_listener(&listener).unwrap();
    listener.set_nonblocking(true).unwrap();
    let listener = TcpListener::from_std(listener).unwrap();
    let (finished, wait) = oneshot::channel();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        request(&mut stream).await;
        stream.write_all(reply(1).as_bytes()).await.unwrap();
        wait.await.unwrap();
        assert!(
            tokio::time::timeout(Duration::from_millis(100), listener.accept())
                .await
                .is_err()
        );
    });
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("owned");
    let mut runner = ProbeRunner::for_owned_fixture(
        &root,
        endpoint,
        identity(),
        diagnostics::api::TraceContext::global(None),
        Arc::new(probe::runner::RuntimeCancellationTimer),
        Arc::new(OwnedSpace),
    )
    .unwrap();
    let first = runner.next().await.unwrap().unwrap();
    let destination = root.join("results/history.csv");
    std::fs::create_dir(&destination).unwrap();
    assert!(runner.export(&destination, &OwnedSpace).is_err());
    assert!(runner.snapshot().unwrap().halted);
    assert!(runner.next().await.is_err());
    assert!(destination.is_dir());
    assert_eq!(
        runner
            .sessions()
            .get(&first.history.id)
            .unwrap()
            .result
            .as_deref(),
        Some("OK")
    );
    finished.send(()).unwrap();
    server.await.unwrap();
}

#[tokio::test]
async fn actual_helper_reservation_and_core_send_commit_faults_prevent_http_without_refunding() {
    for core_fault in [false, true] {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = workflow::experiment::OwnedLoopback::from_listener(&listener).unwrap();
        listener.set_nonblocking(true).unwrap();
        let listener = TcpListener::from_std(listener).unwrap();
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("owned");
        let mut runner = ProbeRunner::for_owned_fixture(
            &root,
            endpoint,
            identity(),
            diagnostics::api::TraceContext::global(None),
            Arc::new(probe::runner::RuntimeCancellationTimer),
            Arc::new(OwnedSpace),
        )
        .unwrap();
        let core = rusqlite::Connection::open(root.join("results/sessions.db")).unwrap();
        let helper = rusqlite::Connection::open(root.join("round/round.sqlite")).unwrap();
        if core_fault {
            core.execute_batch("CREATE TRIGGER fail_sent BEFORE UPDATE OF send_state ON api_calls WHEN NEW.send_state='possibly_sent' BEGIN SELECT RAISE(FAIL,'owned core fault');END;").unwrap();
        } else {
            helper.execute_batch("CREATE TRIGGER fail_reserve BEFORE UPDATE ON reservation WHEN json_extract(NEW.payload,'$.records[0].state')='PossiblySent' BEGIN SELECT RAISE(FAIL,'owned helper fault');END;").unwrap();
        }
        assert!(runner.next().await.is_err());
        assert!(runner.next().await.is_err());
        assert!(
            tokio::time::timeout(Duration::from_millis(100), listener.accept())
                .await
                .is_err()
        );
        let (calls, sent): (u64, u64) = core
            .query_row(
                "SELECT COUNT(*),COALESCE(SUM(send_state!='queued'),0) FROM api_calls",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!((calls, sent), (1, 0));
        assert_eq!(
            runner.snapshot().unwrap().reservation_units().unwrap(),
            if core_fault { 104_448 } else { 0 }
        );
    }
}

#[tokio::test]
async fn diagnostics_off_helper_finish_failure_blocks_automatic_reverse_after_real_forward_flush() {
    use diagnostics::{api::*, service::*};
    struct Off;
    impl DiagnosticStorage for Off {
        fn load_preference(&self) -> Result<Option<bool>, &'static str> {
            Ok(Some(false))
        }
        fn save_preference(&self, _: bool) -> Result<(), &'static str> {
            Ok(())
        }
        fn write_report(&self, _: &std::path::Path, _: &str) -> Result<(), &'static str> {
            Err("unused")
        }
    }
    struct Observer(Arc<DiagnosticService>);
    impl ApiObserver for Observer {
        fn begin(
            &self,
            scope: Option<ScopeBinding>,
            revision: Option<u64>,
        ) -> Option<TraceIdentity> {
            self.0.begin_api(scope, revision)
        }
        fn record(&self, event: ApiEvent) {
            self.0.record_api(event);
        }
    }
    let service = Arc::new(DiagnosticService::new());
    service.initialize(Arc::new(Off));
    assert!(!service.status().effective_enabled);
    let tracing = TraceContext::new(
        None,
        Arc::new(capture_startup::MonotonicClock),
        Arc::new(Observer(service.clone())),
    );
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = workflow::experiment::OwnedLoopback::from_listener(&listener).unwrap();
    listener.set_nonblocking(true).unwrap();
    let listener = TcpListener::from_std(listener).unwrap();
    let (finished, wait) = oneshot::channel();
    let server = tokio::spawn(async move {
        for slot in 1..=5 {
            let (mut stream, _) = listener.accept().await.unwrap();
            request(&mut stream).await;
            stream.write_all(reply(slot).as_bytes()).await.unwrap();
        }
        wait.await.unwrap();
        assert!(
            tokio::time::timeout(Duration::from_millis(100), listener.accept())
                .await
                .is_err()
        );
    });
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("owned");
    let mut runner = ProbeRunner::for_owned_fixture(
        &root,
        endpoint,
        identity(),
        tracing,
        Arc::new(probe::runner::RuntimeCancellationTimer),
        Arc::new(OwnedSpace),
    )
    .unwrap();
    for _ in 0..4 {
        runner.next().await.unwrap().unwrap();
    }
    let helper = rusqlite::Connection::open(root.join("round/round.sqlite")).unwrap();
    helper.execute_batch("CREATE TRIGGER fail_finish BEFORE UPDATE ON reservation WHEN json_extract(NEW.payload,'$.records[4].state')='Finished' BEGIN SELECT RAISE(FAIL,'owned callback fault');END;").unwrap();
    assert!(runner.next().await.is_err());
    assert!(runner.next().await.is_err());
    let entries = runner.sessions().search("").unwrap();
    let forward = entries
        .iter()
        .find(|entry| entry.transcript == REVIEW)
        .unwrap();
    assert_eq!(
        forward.result.as_deref(),
        Some("Do not delete API_42; keep timeout at 0.5 seconds.")
    );
    assert!(forward.back_translation.is_none());
    let core = rusqlite::Connection::open(root.join("results/sessions.db")).unwrap();
    assert_eq!(
        core.query_row("SELECT COUNT(*) FROM api_calls", [], |row| row
            .get::<_, u64>(0))
            .unwrap(),
        5
    );
    assert!(!service.status().effective_enabled);
    finished.send(()).unwrap();
    server.await.unwrap();
}

#[tokio::test]
async fn completion_winning_slot7_must_pass_quality_before_retry8_can_be_admitted() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = workflow::experiment::OwnedLoopback::from_listener(&listener).unwrap();
    listener.set_nonblocking(true).unwrap();
    let listener = TcpListener::from_std(listener).unwrap();
    let (entered, timer_entered) = oneshot::channel();
    let (_release, timer_release) = oneshot::channel();
    let terminal = Arc::new(AtomicBool::new(false));
    let timer = Arc::new(Timer {
        entered: Mutex::new(Some(entered)),
        release: Mutex::new(Some(timer_release)),
        terminal: terminal.clone(),
    });
    let (finished, wait) = oneshot::channel();
    let server = tokio::spawn(async move {
        let mut entered = Some(timer_entered);
        for slot in 1..=7 {
            let (mut stream, _) = listener.accept().await.unwrap();
            request(&mut stream).await;
            if slot == 7 {
                entered.take().unwrap().await.unwrap();
                let body=json!({"model":"deepseek-flash","usage":{"prompt_tokens":5,"completion_tokens":2,"total_tokens":7},"choices":[{"finish_reason":"stop","message":{"role":"assistant","content":"The task is done."}}]}).to_string();
                stream.write_all(format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
            } else {
                stream.write_all(reply(slot).as_bytes()).await.unwrap();
            }
        }
        wait.await.unwrap();
        assert!(
            tokio::time::timeout(Duration::from_millis(100), listener.accept())
                .await
                .is_err()
        );
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
    for _ in 0..5 {
        runner.next().await.unwrap().unwrap();
    }
    assert!(
        runner.next().await.is_err(),
        "completed English result cannot satisfy the fixed Chinese slot7 request"
    );
    assert!(
        terminal.load(Ordering::SeqCst),
        "timer still joins before quality rejection"
    );
    assert!(runner.snapshot().unwrap().halted);
    assert!(runner.next().await.is_err());
    finished.send(()).unwrap();
    server.await.unwrap();
}

fn disabled_quality_trace() -> (
    Arc<diagnostics::service::DiagnosticService>,
    diagnostics::api::TraceContext,
) {
    use diagnostics::{api::*, service::*};
    struct Off;
    impl DiagnosticStorage for Off {
        fn load_preference(&self) -> Result<Option<bool>, &'static str> {
            Ok(Some(false))
        }
        fn save_preference(&self, _: bool) -> Result<(), &'static str> {
            Ok(())
        }
        fn write_report(&self, _: &std::path::Path, _: &str) -> Result<(), &'static str> {
            Err("unused")
        }
    }
    struct Observer(Arc<DiagnosticService>);
    impl ApiObserver for Observer {
        fn begin(
            &self,
            scope: Option<ScopeBinding>,
            revision: Option<u64>,
        ) -> Option<TraceIdentity> {
            self.0.begin_api(scope, revision)
        }
        fn record(&self, event: ApiEvent) {
            self.0.record_api(event);
        }
    }
    let service = Arc::new(DiagnosticService::new());
    service.initialize(Arc::new(Off));
    let tracing = TraceContext::new(
        None,
        Arc::new(capture_startup::MonotonicClock),
        Arc::new(Observer(service.clone())),
    );
    (service, tracing)
}
struct CompletionWins;
impl CancellationTimer for CompletionWins {
    fn wait(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> {
        Box::pin(std::future::pending())
    }
}
fn quality_reply(slot: u8, bad: u8, text: &str) -> String {
    if slot != bad && slot <= 8 {
        return reply(usize::from(slot));
    }
    let content = if slot == bad {
        json!({"text":text,"source_language":"zh-TW"}).to_string()
    } else {
        DICTATION.into()
    };
    let body=json!({"model":if slot<=8{"deepseek-flash"}else{"google/gemma-4-31b-it"},"usage":{"prompt_tokens":5,"completion_tokens":2,"total_tokens":7,"cost":0},"choices":[{"finish_reason":"stop","message":{"role":"assistant","content":content}}]}).to_string();
    format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}
async fn invalid_forward_stops_reverse(bad: u8) {
    let bad_text = if bad == 5 {
        "wrong text"
    } else {
        "Delete session_id; keep timeout at 0.5 seconds and retain 42 API requests."
    };
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = workflow::experiment::OwnedLoopback::from_listener(&listener).unwrap();
    listener.set_nonblocking(true).unwrap();
    let listener = TcpListener::from_std(listener).unwrap();
    let (finished, mut settled) = oneshot::channel();
    let server = tokio::spawn(async move {
        let mut accepted = 0u8;
        for slot in 1..=bad {
            let (mut stream, _) = tokio::time::timeout(Duration::from_secs(20), listener.accept())
                .await
                .unwrap()
                .unwrap();
            request(&mut stream).await;
            accepted += 1;
            stream
                .write_all(quality_reply(slot, bad, bad_text).as_bytes())
                .await
                .unwrap();
        }
        loop {
            tokio::select! {
                result=listener.accept()=>{
                    let(mut stream,_)=result.unwrap();request(&mut stream).await;accepted+=1;
                    stream.write_all(quality_reply(bad+1,bad,bad_text).as_bytes()).await.unwrap();
                },
                result=&mut settled=>{result.unwrap();break;},
                _=tokio::time::sleep(Duration::from_secs(20))=>panic!("owned quality driver did not settle"),
            }
        }
        assert!(
            tokio::time::timeout(Duration::from_millis(100), listener.accept())
                .await
                .is_err()
        );
        accepted
    });
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("owned");
    let (service, tracing) = disabled_quality_trace();
    assert!(!service.status().effective_enabled);
    let mut runner = ProbeRunner::for_owned_fixture(
        &root,
        endpoint,
        identity(),
        tracing,
        Arc::new(CompletionWins),
        Arc::new(OwnedSpace),
    )
    .unwrap();
    let preceding = if bad == 5 { 4 } else { 8 };
    for _ in 0..preceding {
        runner.next().await.unwrap().unwrap();
    }
    service.clear();
    assert!(!service.status().effective_enabled);
    assert!(tokio::time::timeout(Duration::from_secs(40), runner.next())
        .await
        .unwrap()
        .is_err());
    assert!(runner.next().await.is_err());
    finished.send(()).unwrap();
    let accepted = server.await.unwrap();
    assert_eq!(
        accepted, bad,
        "invalid forward must not spend its reverse HTTP request"
    );
    let original = probe::policy::schedule()[usize::from(bad - 1)].corpus;
    let state = runner.snapshot().unwrap();
    let forward = runner
        .sessions()
        .get(&state.records[usize::from(bad - 1)].history)
        .unwrap();
    assert_eq!(forward.result.as_deref(), Some(bad_text));
    assert!(forward.back_translation.is_none());
    assert_eq!(forward.api_summary.call_count, Some(1));
    assert_eq!(forward.api_summary.input.total, Some(5));
    assert_eq!(forward.api_summary.output.total, Some(2));
    super::provider_probe_transport::assert_actual_fee(
        &forward,
        if bad == 10 { Some("0") } else { None },
    );
    let core = rusqlite::Connection::open(root.join("results/sessions.db")).unwrap();
    assert_eq!(
        core.query_row("SELECT COUNT(*) FROM api_calls", [], |row| row
            .get::<_, u64>(0))
            .unwrap(),
        u64::from(bad)
    );
    let (outcome, receipt): (String, String) = core
        .query_row(
            "SELECT outcome,receipt FROM api_calls WHERE session_id=?1",
            [&forward.id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(outcome, "success");
    assert!(!receipt.is_empty());
    assert!(state.halted);
    assert_eq!(state.records[usize::from(bad - 1)].state, State::Reconciled);
    assert_eq!(state.records[usize::from(bad)].state, State::NotRun);
    assert!(state.records[usize::from(bad)].call.is_empty());
    assert_eq!(
        state.reservation_units().unwrap(),
        if bad == 5 { 522_240 } else { 835_584 }
    );
    let retained_id = forward.id.clone();
    drop(runner);
    let reopened = SessionService::open(root.join("results/sessions.db")).unwrap();
    let retained = reopened.get(&retained_id).unwrap();
    assert_eq!(retained.transcript, original);
    assert_eq!(retained.result.as_deref(), Some(bad_text));
    assert_eq!(retained.api_summary.input.total, Some(5));
    super::provider_probe_transport::assert_actual_fee(
        &retained,
        if bad == 10 { Some("0") } else { None },
    );
    assert!(
        probe::journal::Journal::open_report(&root.join("round"))
            .unwrap()
            .snapshot()
            .halted
    );
}
#[tokio::test]
async fn bad_deepseek_forward5_is_durable_but_never_admits_reverse6() {
    invalid_forward_stops_reverse(5).await;
}
#[tokio::test]
async fn bad_openrouter_forward10_is_durable_but_never_admits_reverse11() {
    invalid_forward_stops_reverse(10).await;
}
