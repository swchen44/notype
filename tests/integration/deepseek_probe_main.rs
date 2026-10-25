//! Standalone fixed-round helper. This is not the Tauri App or a general API client.
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/capture_startup.rs"]
mod capture_startup;
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/diagnostics.rs"]
mod diagnostics;
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/deepseek_probe/mod.rs"]
mod probe;
#[allow(dead_code, unused_imports)]
#[path = "../../desktop/src-tauri/src/workflow.rs"]
mod workflow;

#[cfg(not(windows))]
fn main() -> std::process::ExitCode {
    eprintln!("probe-platform-unavailable");
    std::process::ExitCode::from(2)
}
#[cfg(windows)]
fn main() -> std::process::ExitCode {
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(_) => {
            eprintln!("probe-runtime-unavailable");
            return std::process::ExitCode::from(1);
        }
    };
    // Every console mode/ownership guard is dropped before returning ExitCode.
    match runtime.block_on(run()) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(code) => {
            eprintln!("{code}");
            std::process::ExitCode::from(1)
        }
    }
}

#[cfg(windows)]
fn confirm(
    console: &probe::native_console::OwnedConsole,
    prompt: &str,
    expected: &str,
) -> Result<bool, &'static str> {
    use std::io::Write;
    console
        .clear_pending_input()
        .map_err(|_| "probe-console-unavailable")?;
    print!("{prompt}");
    std::io::stdout()
        .flush()
        .map_err(|_| "probe-console-unavailable")?;
    let line = console
        .confirmation_line()
        .map_err(|_| "probe-console-unavailable")?;
    Ok(line.trim() == expected)
}

#[cfg(windows)]
fn display(text: &str) -> String {
    let mut output = String::new();
    for character in text.chars() {
        if character.is_control() && !matches!(character, '\n' | '\t') {
            output.push_str(&format!("\\u{{{:x}}}", u32::from(character)));
        } else {
            output.push(character);
        }
    }
    output
}

#[cfg(windows)]
fn save_evidence(
    paths: &probe::native_paths::TrialPaths,
    sessions: &notype_core::SessionService,
    round: &std::sync::Arc<
        std::sync::Mutex<probe::round::Round<probe::sqlite_round::SqliteRoundStore>>,
    >,
    source: &str,
    outcome: &str,
) -> Result<(), &'static str> {
    use std::io::Write;
    let exported = sessions
        .export_csv(
            &paths.results.join("history.csv"),
            &probe::native_paths::AvailableSpace,
            &Default::default(),
        )
        .map_err(|_| "probe-history-export-failed")?;
    let report = round.lock().map_err(|_| "probe-storage-failed")?.report();
    let mut body=format!("# notype DeepSeek fixed-round evidence\n\nSource: `{source}`\n\nOutcome: `{outcome}`\n\nHistory rows: {}\n\nReserved slots: {}\n\nReservation units (USD 1e-7): {}\n\nHalted: {}\n\n",exported.rows,report.reserved_slots,report.reservation_units,report.halted);
    body.push_str("Binary SHA256/offline review: see the separate reviewed build receipt delivered with this helper. This runtime report does not calculate or substitute an actual API fee. Missing billed money stays UNKNOWN.\n\nPinned docs: 2026-10-25 UTC; requested/allowed returned model deepseek-flash; served version documented as DeepSeek-V4.1-Flash-1017. Peak/cache-miss input USD0.30/M, output USD1.20/M. The 32768-input ceiling is an operational assumption, not a provider-enforced token/dollar guarantee. Response detection cannot undo already incurred charges.\n\n");
    body.push_str("| Slot | State | History ID | Call ID |\n| --- | --- | --- | --- |\n");
    for (index, slot) in report.slots.iter().enumerate() {
        body.push_str(&format!(
            "| {} | {:?} | {} | {} |\n",
            index + 1,
            slot.state,
            slot.history_id,
            slot.call_id
        ));
    }
    let partial = paths.results.join("receipt.md.partial");
    let destination = paths.results.join("receipt.md");
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&partial)
        .map_err(|_| "probe-receipt-write-failed")?;
    file.write_all(body.as_bytes())
        .and_then(|_| file.sync_all())
        .map_err(|_| "probe-receipt-write-failed")?;
    drop(file);
    std::fs::rename(partial, destination).map_err(|_| "probe-receipt-write-failed")
}

#[cfg(windows)]
async fn run() -> Result<(), &'static str> {
    use probe::{
        console::{read_local_key, ConsolePort},
        native_console::OwnedConsole,
        native_paths::TrialPaths,
        round::Round,
        sqlite_round::SqliteRoundStore,
    };
    use std::sync::{Arc, Mutex};
    if std::env::args_os().count() != 1 {
        return Err("probe-arguments-unavailable");
    }
    let paths = TrialPaths::discover().map_err(|_| "probe-path-unavailable")?;
    if paths
        .preflight()
        .map_err(|_| "probe-owned-path-unavailable")?
    {
        let round = Round::open(
            SqliteRoundStore::open_owned(&paths.round).map_err(|_| "probe-round-unavailable")?,
        )
        .map_err(|_| "probe-round-unavailable")?;
        let report = round.report();
        println!("這輪已存在：僅顯示紀錄，不取得 key、不重跑或續跑。\n已預留 {} 次，USD 1e-7 預留單位 {}。\n保留的 CSV 可能早於中斷中的呼叫，不能當完整帳單。",report.reserved_slots,report.reservation_units);
        for (index, slot) in report.slots.iter().enumerate() {
            println!("{} {:?}", index + 1, slot.state);
        }
        return Ok(());
    }
    // Parent builds only a clean reviewed source with this nonsecret compile-time
    // pin. An ordinary unpinned build cannot prompt for a key or start a round.
    let source = option_env!("NOTYPE_PROBE_SOURCE_COMMIT")
        .filter(|value| value.len() == 40 && value.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or("probe-build-source-unpinned")?;
    let console = OwnedConsole::open().map_err(|_| "probe-owned-console-required")?;
    println!("notype：單輪 DeepSeek 實測\n只用固定非機密短句，最多 8 次（含回譯、取消與一次重試），Flash 非思考模式、每次輸出上限 512 tokens。\n本輪授權 US$0.10，操作預留最多 US$0.0835584。32K input 是保守假設，不是服務商帳單硬上限；發現異常會停止後續請求，不能撤銷已發生費用。\n不使用麥克風、剪貼簿、其他 App 或既有 notype 資料。\n請先核對隨附的 binary hash／離線審查與當時官方價格。\nAPI key 只在這個獨立視窗輸入、不顯示、不保存；Esc 可中止。輸入 key 本身不發送請求。\n");
    print!("DeepSeek API key：");
    use std::io::Write;
    std::io::stdout()
        .flush()
        .map_err(|_| "probe-console-unavailable")?;
    let key = read_local_key(&console).map_err(|_| "probe-key-entry-aborted")?;
    println!();
    if console.interrupted() {
        return Err("probe-key-entry-aborted");
    }
    if !confirm(
        &console,
        "要開始此單輪，請輸入 RUN P23-live-v1（其餘中止）：",
        "RUN P23-live-v1",
    )? {
        return Err("probe-round-not-started");
    }
    // No live marker/history is created until both key entry and final local
    // confirmation succeed. Every restart after this creation is report only.
    paths
        .prepare_parent()
        .map_err(|_| "probe-owned-path-unavailable")?;
    let mut round = Round::open(
        SqliteRoundStore::open_owned(&paths.round).map_err(|_| "probe-round-unavailable")?,
    )
    .map_err(|_| "probe-round-unavailable")?;
    if round.report().report_only {
        return Err("probe-round-report-only");
    }
    paths
        .create_results()
        .map_err(|_| "probe-results-unavailable")?;
    let sessions = notype_core::SessionService::open(paths.results.join("sessions.db"))
        .map_err(|_| "probe-history-unavailable")?;
    round.arm().map_err(|_| "probe-round-unavailable")?;
    let round = Arc::new(Mutex::new(round));
    let configuration = workflow::TextConfiguration {
        transport: workflow::TextTransport::OpenAiChat,
        endpoint: "https://api.deepseek.com/chat/completions".into(),
        model: "deepseek-flash".into(),
        authentication: workflow::TextAuthentication::Bearer,
        api_key: key.into_string(),
        timeout_ms: 15000,
        provider_profile: workflow::TextProviderProfile::DeepSeek,
        effort: Some("none".into()),
        max_output_tokens: Some(512),
        ..Default::default()
    };
    let mut runner = probe::runner::ProbeRunner::live(
        sessions.clone(),
        round.clone(),
        workflow::probe_text_processor(configuration),
    )
    .map_err(|_| "probe-round-unavailable")?;
    let mut outcome = "completed";
    loop {
        let step = match runner.next().await {
            Ok(Some(step)) => step,
            Ok(None) => break,
            Err(error) => {
                outcome = error.code();
                break;
            }
        };
        println!(
            "\n步驟 {}–{}：{:?}\n原稿：{}\n結果：{}",
            step.first_slot,
            step.last_slot,
            step.history.status,
            display(&step.history.transcript),
            display(step.history.result.as_deref().unwrap_or("（未提供結果）"))
        );
        if let Some(reverse) = &step.history.back_translation {
            println!(
                "回譯：{}\n回譯僅供人工比較，不保證翻譯正確。",
                display(reverse)
            );
        }
        if save_evidence(&paths, &sessions, &round, source, "awaiting-local-review").is_err() {
            outcome = "probe-evidence-write-failed";
            break;
        }
        let accepted = match confirm(
            &console,
            "請核對語意、數字、否定與術語；符合請輸入 y，否則中止：",
            "y",
        ) {
            Ok(accepted) => accepted,
            Err(code) => {
                outcome = code;
                break;
            }
        };
        if !accepted {
            let _ = runner.accept_output(false);
            outcome = "probe-quality-rejected";
            break;
        }
        if let Err(error) = runner.accept_output(true) {
            outcome = error.code();
            break;
        }
    }
    if outcome != "completed" {
        if let Ok(mut state) = round.lock() {
            if !state.report().halted {
                let _ = state.halt();
            }
        }
    }
    save_evidence(&paths, &sessions, &round, source, outcome)?;
    println!("\n此輪結束：{outcome}。研究資料保存在 {}。\nAPI 未回報實收金額時，費用保持未知；預留不是帳單。",display(&paths.results.display().to_string()));
    if outcome == "completed" {
        Ok(())
    } else {
        Err(outcome)
    }
}
