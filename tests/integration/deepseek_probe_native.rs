#[path = "../../desktop/src-tauri/src/deepseek_probe/console.rs"]
mod console;
#[path = "../../desktop/src-tauri/src/deepseek_probe/native_console.rs"]
mod native_console;

#[cfg(windows)]
#[test]
fn ordinary_test_process_cannot_read_a_key_from_redirected_or_foreign_console() {
    assert!(matches!(
        native_console::OwnedConsole::open(),
        Err(console::ConsoleError::Unavailable)
    ));
}

#[cfg(not(windows))]
#[test]
fn non_windows_console_entry_is_explicitly_unavailable() {
    assert!(matches!(
        native_console::OwnedConsole::open(),
        Err(console::ConsoleError::Unavailable)
    ));
}

#[cfg(windows)]
#[test]
fn hidden_owned_console_restores_modes_for_success_abort_fault_unwind_and_real_signal_races() {
    use std::{os::windows::ffi::OsStrExt, path::Path};
    use windows_sys::Win32::{
        Foundation::{CloseHandle, WAIT_OBJECT_0},
        System::{
            Console::{GetConsoleMode, GetStdHandle, STD_INPUT_HANDLE},
            Threading::{
                CreateProcessW, GetExitCodeProcess, TerminateProcess, WaitForSingleObject,
                CREATE_NEW_CONSOLE, PROCESS_INFORMATION, STARTF_USESHOWWINDOW, STARTUPINFOW,
            },
        },
    };
    fn mode() -> Option<u32> {
        let mut result = 0;
        // SAFETY: borrows this parent's standard input, observes mode only.
        (unsafe { GetConsoleMode(GetStdHandle(STD_INPUT_HANDLE), &mut result) } != 0)
            .then_some(result)
    }
    fn wide(path: &Path) -> Vec<u16> {
        let mut value: Vec<_> = path.as_os_str().encode_wide().collect();
        assert!(!value.contains(&0));
        value.push(0);
        value
    }
    struct Child(windows_sys::Win32::Foundation::HANDLE);
    impl Child {
        fn finish(mut self, timeout: u32) -> Result<u32, &'static str> {
            // SAFETY: this exact returned process handle is owned by the guard.
            let ready = unsafe { WaitForSingleObject(self.0, timeout) } == WAIT_OBJECT_0;
            if !ready {
                // SAFETY: terminate only this fixture's handle and verify its
                // exit; a failed cleanup is never reported as a passed fixture.
                let killed = unsafe { TerminateProcess(self.0, 97) } != 0;
                let settled = unsafe { WaitForSingleObject(self.0, 2000) } == WAIT_OBJECT_0;
                if !killed || !settled {
                    return Err("probe-owned-child-cleanup-failed");
                }
            }
            let mut exit = 0;
            // SAFETY: the now-terminated process and writable exit-code output
            // remain valid until this guard closes the handle below.
            let observed = unsafe { GetExitCodeProcess(self.0, &mut exit) } != 0;
            if unsafe { CloseHandle(self.0) } == 0 {
                return Err("probe-owned-handle-close-failed");
            }
            self.0 = std::ptr::null_mut();
            if !ready {
                return Err("probe-owned-child-timeout");
            }
            if !observed {
                return Err("probe-owned-exit-unavailable");
            }
            Ok(exit)
        }
    }
    impl Drop for Child {
        fn drop(&mut self) {
            if self.0.is_null() {
                return;
            }
            // SAFETY: owns this exact CreateProcessW handle, not a reusable PID.
            unsafe {
                if WaitForSingleObject(self.0, 0) != WAIT_OBJECT_0 {
                    let killed = TerminateProcess(self.0, 97) != 0;
                    let settled = WaitForSingleObject(self.0, 2000) == WAIT_OBJECT_0;
                    if !killed || !settled {
                        eprintln!("probe-owned-child-cleanup-failed");
                    }
                }
                if CloseHandle(self.0) == 0 {
                    eprintln!("probe-owned-handle-close-failed");
                }
            }
        }
    }
    let before = mode();
    let owned = tempfile::Builder::new()
        .prefix("notype-probe-console-owned-")
        .tempdir()
        .unwrap();
    let executable = Path::new(env!("CARGO_BIN_EXE_notype-probe-console-fixture"));
    for case in [
        "unicode",
        "escape",
        "control-key",
        "read-error",
        "unwind",
        "signal",
        "late",
        "disable-error",
        "restore-error",
    ] {
        let receipt = owned.path().join(format!("{case}.txt"));
        let application = wide(executable);
        // Windows file names cannot contain a quote; both quoted path arguments
        // end in ordinary file names, so no terminal backslash escapes a quote.
        let mut command: Vec<_> = format!(
            "\"{}\" {} \"{}\"",
            executable.display(),
            case,
            receipt.display()
        )
        .encode_utf16()
        .chain(Some(0))
        .collect();
        let mut startup = STARTUPINFOW {
            cb: std::mem::size_of::<STARTUPINFOW>() as u32,
            dwFlags: STARTF_USESHOWWINDOW,
            wShowWindow: 0,
            ..Default::default()
        };
        let mut process = PROCESS_INFORMATION::default();
        // SAFETY: absolute executable, mutable NUL command, valid structures;
        // no handle inheritance/redirected stdin and SW_HIDE0 requests a hidden
        // new console containing only the owned fixture child.
        assert_ne!(
            unsafe {
                CreateProcessW(
                    application.as_ptr(),
                    command.as_mut_ptr(),
                    std::ptr::null(),
                    std::ptr::null(),
                    0,
                    CREATE_NEW_CONSOLE,
                    std::ptr::null(),
                    std::ptr::null(),
                    &mut startup,
                    &mut process,
                )
            },
            0,
            "launch {case}"
        );
        let child = Child(process.hProcess);
        // SAFETY: thread handle is owned and not otherwise needed.
        assert_ne!(
            unsafe { CloseHandle(process.hThread) },
            0,
            "thread-handle cleanup {case}"
        );
        let exit = child
            .finish(5000)
            .unwrap_or_else(|code| panic!("{code}: {case}"));
        assert_eq!(exit, 0, "fixture {case}");
        let evidence = std::fs::read_to_string(receipt).unwrap();
        assert!(evidence.contains("mode_restored=true"));
        assert!(evidence.contains("non_echo_observed=true"));
        assert!(!evidence.contains("x🙂"));
        assert_eq!(mode(), before, "parent console mode must remain untouched");
    }
}
