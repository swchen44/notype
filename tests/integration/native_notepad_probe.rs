//! Manual Windows runtime probe for the production input adapter.
//! The runner supplies the PID of its newly launched Notepad++ instance.
//! It never enumerates document text; use only a synthetic, owned test document.
#[path = "../../desktop/src-tauri/src/native_input.rs"]
mod native_input;
use notype_core::{DeliveryAdapter, DeliveryRequest};
use std::io::{BufRead, Write};

#[cfg(windows)]
fn main() {
    #[link(name = "user32")]
    unsafe extern "system" {
        fn GetForegroundWindow() -> isize;
        fn GetWindowThreadProcessId(window: isize, pid: *mut u32) -> u32;
    }
    let expected = std::env::var("NOTYPE_OWNED_NOTEPAD_PID")
        .expect("owned PID required")
        .parse::<u32>()
        .expect("valid PID");
    assert!(expected != 0);
    let input = native_input::NativeInput::default();
    let mut token = None;
    println!("ready");
    std::io::stdout().flush().unwrap();
    for line in std::io::stdin().lock().lines() {
        let command = line.unwrap();
        if command == "exit" {
            break;
        }
        let mut pid = 0;
        unsafe {
            GetWindowThreadProcessId(GetForegroundWindow(), &mut pid);
        }
        if pid != expected {
            println!("owned-foreground-required");
            continue;
        }
        match command.as_str() {
            "capture" => {
                let capture = input.capture();
                assert_eq!(capture.application.as_deref(), Some("notepad++"));
                token = capture.target;
                println!("captured={}", token.is_some());
            }
            "deliver" | "replay" => {
                let target = token.clone().expect("capture first");
                let mut future = input.deliver(DeliveryRequest {
                    session_id: "owned-notepad-probe".into(),
                    original_target: target,
                    text: "notype 測試 OpenAI 🧪\n第二行。".into(),
                });
                let mut context = std::task::Context::from_waker(std::task::Waker::noop());
                println!("{:?}", future.as_mut().poll(&mut context));
            }
            _ => panic!("unknown owned command"),
        }
        std::io::stdout().flush().unwrap();
    }
}
#[cfg(not(windows))]
fn main() {
    panic!("Windows runtime required");
}
