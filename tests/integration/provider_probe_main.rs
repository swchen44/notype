//! Ordinary headless Windows helper; no Tauri, console prompt or runtime key arg.
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/capture_startup.rs"]
mod capture_startup;
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/diagnostics.rs"]
mod diagnostics;
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/provider_probe/mod.rs"]
mod probe;
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/provider_config.rs"]
mod provider_config;
#[allow(dead_code, unused_imports)]
#[path = "../../desktop/src-tauri/src/workflow.rs"]
mod workflow;
fn main() -> std::process::ExitCode {
    let arguments = std::env::args_os().skip(1).take(3).collect::<Vec<_>>();
    let mode = match probe::cli::Mode::parse(&arguments) {
        Ok(mode) => mode,
        Err(code) => {
            eprintln!("{code}");
            return std::process::ExitCode::from(1);
        }
    };
    if !cfg!(windows) {
        eprintln!("probe-platform-unavailable");
        return std::process::ExitCode::from(2);
    }
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
    match runtime.block_on(probe::cli::execute(mode)) {
        Ok(summary) => match serde_json::to_string(&summary) {
            Ok(text) => {
                println!("{text}");
                if matches!(
                    summary.state,
                    probe::cli::ExitState::Halted | probe::cli::ExitState::NotReady
                ) {
                    std::process::ExitCode::from(1)
                } else {
                    std::process::ExitCode::SUCCESS
                }
            }
            Err(_) => {
                eprintln!("probe-report-failed");
                std::process::ExitCode::from(1)
            }
        },
        Err(error) => {
            eprintln!("{}", error.code());
            std::process::ExitCode::from(1)
        }
    }
}
