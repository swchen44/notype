//! Same-executable no-key exercise of production selected credentials, sealed
//! transport, observer, core SQLite and atomic CSV. Only synthetic home bytes
//! and an owned loopback listener exist at this boundary.
use super::{
    bridge::ResourceGate,
    configuration::LiveConfigurations,
    journal::{Failure, Identity, State},
    policy,
    runner::{PreparedRoot, ProbeRunner},
};
use crate::{
    capture_startup::MonotonicClock,
    diagnostics::{self, api::*, service::*},
    provider_config,
    workflow::experiment::OwnedLoopback,
};
use serde::Serialize;
use serde_json::{json, Value};
use std::{path::Path, sync::Arc, time::Duration};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
const DEEPSEEK: &str = "SYNTHETIC-P25-DEEPSEEK-NOT-A-REAL-KEY";
const OPENROUTER: &str = "SYNTHETIC-P25-OPENROUTER-NOT-A-REAL-KEY";

/// Space observation is replaceable only at the owned fixture/check boundary;
/// the live runner always uses the fixed native quota adapter.
pub trait SmokeSpace: ResourceGate + notype_core::csv::CsvSpaceProbe {}
impl<T: ResourceGate + notype_core::csv::CsvSpaceProbe> SmokeSpace for T {}
/// Safe successful synthetic proof; never an inference/billing result.
#[derive(Serialize)]
pub struct SmokeProof {
    /// The owned server accepted exactly the12 eligible synthetic POSTs.
    pub accepted_requests: u8,
    /// Actual SessionService/CSV distinct history count, including shared groups.
    pub histories: usize,
    /// Actual durable call count, including cancellation and explicit retry.
    pub observed_calls: u64,
    /// Four immutable ineligible Gemini entries remained sealed.
    pub gemini_sealed: bool,
    /// Synthetic credentials were absent from independently read CSV/trace.
    pub synthetic_keys_excluded: bool,
}
struct Storage;
impl DiagnosticStorage for Storage {
    fn load_preference(&self) -> Result<Option<bool>, &'static str> {
        Ok(Some(true))
    }
    fn save_preference(&self, _: bool) -> Result<(), &'static str> {
        Ok(())
    }
    fn write_report(&self, destination: &Path, text: &str) -> Result<(), &'static str> {
        super::native::text(destination, text).map_err(|_| "probe-trace-write-failed")
    }
}
/// Owned P20 service; no App/global collector or raw logging adapter is used.
pub(super) struct Trace(Arc<DiagnosticService>);
impl Trace {
    pub(super) fn new() -> Self {
        let service = Arc::new(DiagnosticService::new());
        service.initialize(Arc::new(Storage));
        Self(service)
    }
    pub(super) fn context(self: &Arc<Self>) -> TraceContext {
        TraceContext::new(None, Arc::new(MonotonicClock), self.clone())
    }
    pub(super) fn export(&self, destination: &Path) -> Result<(), Failure> {
        let ticket = self.0.begin_preview().map_err(|_| Failure::Storage)?;
        let preview = self
            .0
            .publish_preview(
                ticket,
                diagnostics::ReportContext {
                    version: "4",
                    platform: diagnostics::Platform::Windows,
                    log_level: diagnostics::LogLevel::Info,
                    debug_mode: false,
                    always_on_microphone: false,
                    model_selected: false,
                    model_available: false,
                    model_is_custom: false,
                    local_log: diagnostics::LogFileStatus::Unavailable,
                },
            )
            .map_err(|_| Failure::Storage)?;
        let ticket = self
            .0
            .begin_export(preview.report.id())
            .map_err(|_| Failure::Storage)?;
        self.0
            .commit_export(&ticket, destination)
            .map_err(|_| Failure::Storage)
    }
}
impl ApiObserver for Trace {
    fn begin(&self, scope: Option<ScopeBinding>, revision: Option<u64>) -> Option<TraceIdentity> {
        self.0.begin_api(scope, revision)
    }
    fn record(&self, event: ApiEvent) {
        self.0.record_api(event);
    }
}
async fn request(stream: &mut tokio::net::TcpStream) -> Result<(String, Value), Failure> {
    let mut bytes = vec![];
    let mut buffer = [0u8; 4096];
    let (end, length) = loop {
        let count = stream
            .read(&mut buffer)
            .await
            .map_err(|_| Failure::Contract)?;
        if count == 0 || bytes.len() + count > 24_576 {
            return Err(Failure::Contract);
        }
        bytes.extend_from_slice(&buffer[..count]);
        if let Some(end) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
            let header = std::str::from_utf8(&bytes[..end]).map_err(|_| Failure::Contract)?;
            if !header.starts_with("POST /v1/chat/completions HTTP/1.1\r\n") {
                return Err(Failure::Contract);
            }
            let length = header
                .lines()
                .find_map(|line| {
                    line.to_ascii_lowercase()
                        .strip_prefix("content-length:")
                        .and_then(|value| value.trim().parse::<usize>().ok())
                })
                .filter(|length| *length <= 8192)
                .ok_or(Failure::Contract)?;
            break (end + 4, length);
        }
    };
    while bytes.len() < end + length {
        let count = stream
            .read(&mut buffer)
            .await
            .map_err(|_| Failure::Contract)?;
        if count == 0 || bytes.len() + count > 24_576 {
            return Err(Failure::Contract);
        }
        bytes.extend_from_slice(&buffer[..count]);
    }
    Ok((
        String::from_utf8(bytes[..end].to_vec()).map_err(|_| Failure::Contract)?,
        serde_json::from_slice(&bytes[end..end + length]).map_err(|_| Failure::Contract)?,
    ))
}
fn response(slot: u8) -> String {
    let text=match slot{
        1=>"OK".into(),2|9|11=>policy::DICTATION.into(),
        3=>"Do not delete session_id; keep timeout at 0.5 seconds, not 5, and retain 42 API requests.".into(),
        4|12=>"下週會議改到星期三下午兩點，不用準備投影片。".into(),
        5=>json!({"text":"Do not delete API_42; keep timeout at 0.5 seconds.","source_language":"zh-TW"}).to_string(),
        6=>policy::REVIEW.into(),
        10=>json!({"text":"Do not delete session_id; keep timeout at 0.5 seconds, not 5, and retain 42 API requests.","source_language":"zh-TW"}).to_string(),
        _=>"這是測試資料的短文。保留所有測試。不要引用外部資料。".into(),
    };
    json!({"model":if slot<=8{"deepseek-flash"}else{"google/gemma-4-31b-it"},"usage":{"prompt_tokens":5,"completion_tokens":2,"total_tokens":7,"cost":0},"choices":[{"finish_reason":"stop","message":{"role":"assistant","content":text}}]}).to_string()
}
async fn server(listener: tokio::net::TcpListener) -> Result<u8, Failure> {
    for slot in 1..=12 {
        let (mut stream, _) = tokio::time::timeout(Duration::from_secs(20), listener.accept())
            .await
            .map_err(|_| Failure::Contract)?
            .map_err(|_| Failure::Contract)?;
        let (headers, body) = tokio::time::timeout(Duration::from_secs(3), request(&mut stream))
            .await
            .map_err(|_| Failure::Contract)??;
        let key = if slot <= 8 { DEEPSEEK } else { OPENROUTER };
        if !headers
            .lines()
            .any(|line| line.trim() == format!("authorization: Bearer {key}"))
            || body["model"] != policy::schedule()[usize::from(slot - 1)].model()
        {
            return Err(Failure::Contract);
        }
        if slot >= 9
            && body["provider"]
                != json!({"only":["google-ai-studio"],"allow_fallbacks":false,"require_parameters":true,"max_price":{"prompt":0,"completion":0,"request":0}})
        {
            return Err(Failure::Contract);
        }
        let body = response(slot);
        stream
            .write_all(
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    body.len()
                )
                .as_bytes(),
            )
            .await
            .map_err(|_| Failure::Contract)?;
        if slot == 7 {
            tokio::time::sleep(Duration::from_millis(100)).await;
            let _ = stream.write_all(body.as_bytes()).await;
        } else {
            stream
                .write_all(body.as_bytes())
                .await
                .map_err(|_| Failure::Contract)?;
        }
    }
    if tokio::time::timeout(Duration::from_millis(75), listener.accept())
        .await
        .is_ok()
    {
        return Err(Failure::Contract);
    }
    Ok(12)
}
/// Exercise one fresh owned directory; no global home/environment or public
/// provider destination is accepted. Any failure settles the owned server task.
pub async fn exercise(
    root: &Path,
    identity: Identity,
    space: Arc<dyn SmokeSpace>,
) -> Result<SmokeProof, Failure> {
    space.checkpoint()?;
    let prepared = PreparedRoot::create(root)?;
    let home = root.join("synthetic-home");
    std::fs::create_dir(&home).map_err(|_| Failure::Storage)?;
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(home.join(".env"))
        .map_err(|_| Failure::Storage)?;
    file.write_all(
        format!("DEEPSEEK_API_KEY={DEEPSEEK}\nOPENROUTER_API_KEY={OPENROUTER}\n").as_bytes(),
    )
    .and_then(|_| file.sync_all())
    .map_err(|_| Failure::Storage)?;
    drop(file);
    let configuration =
        LiveConfigurations::resolve(&provider_config::credentials::HomeFileReader::new(home))?;
    let listener = std::net::TcpListener::bind("127.0.0.1:0").map_err(|_| Failure::Contract)?;
    let endpoint = OwnedLoopback::from_listener(&listener).map_err(|_| Failure::Contract)?;
    listener
        .set_nonblocking(true)
        .map_err(|_| Failure::Contract)?;
    let listener = tokio::net::TcpListener::from_std(listener).map_err(|_| Failure::Contract)?;
    let mut server = tokio::spawn(server(listener));
    let trace = Arc::new(Trace::new());
    let result = async {
        let mut runner = ProbeRunner::smoke(
            prepared,
            endpoint,
            configuration,
            identity,
            trace.context(),
            space.clone(),
        )?;
        let mut last = None;
        while runner.next().await?.is_some() {
            space.checkpoint()?;
            last = Some(runner.export(&root.join("results/history.csv"), space.as_ref())?);
        }
        let snapshot = runner.snapshot()?;
        let proof = last.ok_or(Failure::Contract)?;
        if proof.histories != 9
            || proof.observed_calls != 12
            || snapshot.halted
            || snapshot.records[..12]
                .iter()
                .any(|record| record.state != State::Reconciled)
            || snapshot.records[12..]
                .iter()
                .any(|record| record.state != State::SealedGemini)
        {
            return Err(Failure::Contract);
        }
        trace.export(&root.join("results/diagnostics.txt"))?;
        for path in [
            root.join("results/history.csv"),
            root.join("results/diagnostics.txt"),
        ] {
            let bytes = super::native::read(&path, 8 * 1024 * 1024)?;
            if [DEEPSEEK, OPENROUTER]
                .iter()
                .any(|key| bytes.windows(key.len()).any(|part| part == key.as_bytes()))
            {
                return Err(Failure::Contract);
            }
        }
        Ok(SmokeProof {
            accepted_requests: 0,
            histories: proof.histories,
            observed_calls: proof.observed_calls,
            gemini_sealed: true,
            synthetic_keys_excluded: true,
        })
    }
    .await;
    if result.is_err() {
        server.abort();
        let _ = server.await;
        return result;
    }
    let accepted = tokio::time::timeout(Duration::from_secs(2), &mut server).await;
    let accepted = match accepted {
        Ok(Ok(Ok(count))) => count,
        _ => {
            server.abort();
            let _ = server.await;
            return Err(Failure::Contract);
        }
    };
    result.map(|mut proof| {
        proof.accepted_requests = accepted;
        proof
    })
}
