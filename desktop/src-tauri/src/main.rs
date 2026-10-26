// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use clap::Parser;
use notype_app_lib::CliArgs;

#[cfg(all(target_os = "windows", not(debug_assertions)))]
fn portable_startup_failure(code: &str) -> ! {
    let text = format!("notype could not verify the local portable package.\nUse the complete package as a normal Windows user.\n{code}");
    eprintln!("{text}");
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};
        let wide: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
        let title: Vec<u16> = "notype".encode_utf16().chain(Some(0)).collect();
        let _ = MessageBoxW(
            None,
            windows::core::PCWSTR(wide.as_ptr()),
            windows::core::PCWSTR(title.as_ptr()),
            MB_ICONERROR | MB_OK,
        );
    }
    std::process::exit(2);
}

fn main() {
    let cli_args = CliArgs::parse();
    // Bundle inputs have one explicit, independently validated CLI boundary.
    // Inherited bootstrap/model overrides can never select a preset or root.
    for (name, _) in std::env::vars_os()
        .filter(|(name, _)| {
            let name = name.to_string_lossy().to_ascii_uppercase();
            [
                "NOTYPE_BOOTSTRAP",
                "NOTYPE_BUNDLE",
                "NOTYPE_OFFLINE",
                "NOTYPE_MODEL",
            ]
            .iter()
            .any(|prefix| name.starts_with(prefix))
        })
        .collect::<Vec<_>>()
    {
        std::env::remove_var(name);
    }

    #[cfg(all(target_os = "windows", not(debug_assertions)))]
    {
        // Refuse a partial/direct package before Tauri can start a hidden host,
        // initialize history/tray or offer an installer. Debug fixtures retain
        // their isolated development runtime contract.
        let configured = std::env::var_os("NOTYPE_DATA_DIR").map(std::path::PathBuf::from);
        let executable = std::env::current_exe()
            .unwrap_or_else(|_| portable_startup_failure("runtime-executable-unavailable"));
        let plan = notype_platform::prepare_packaged_process(&executable, configured.as_deref())
            .unwrap_or_else(|error| portable_startup_failure(error.code()));
        for (name, _) in std::env::vars_os()
            .filter(|(name, _)| {
                name.to_string_lossy()
                    .to_ascii_uppercase()
                    .starts_with("WEBVIEW2_")
            })
            .collect::<Vec<_>>()
        {
            std::env::remove_var(name);
        }
        std::env::set_var("WEBVIEW2_BROWSER_EXECUTABLE_FOLDER", plan.browser_directory);
        std::env::set_var("WEBVIEW2_USER_DATA_FOLDER", plan.user_data_directory);
        // The pinned ggml loader otherwise honors an out-of-tree module path
        // even when transcribe supplied its own verified module directory.
        std::env::remove_var("GGML_BACKEND_PATH");
    }

    #[cfg(target_os = "linux")]
    {
        // DMABUF renderer causes crashes on various GPU/display server configurations
        // See: https://github.com/tauri-apps/tauri/issues/9394
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
    }

    #[cfg(target_os = "windows")]
    {
        // Avoid overlay/capture layer crashes (#2049). Set before backend
        // initialization, preserving user overrides.
        if std::env::var_os("VK_LOADER_LAYERS_DISABLE").is_none()
            && !notype_app_lib::env_flag_enabled("HANDY_KEEP_VULKAN_IMPLICIT_LAYERS")
        {
            std::env::set_var("VK_LOADER_LAYERS_DISABLE", "~implicit~");
        }
    }

    notype_app_lib::run(cli_args)
}
