//! Private nonsecret fixture executable, never the live helper or its launcher.
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/deepseek_probe/console.rs"]
mod console;
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/deepseek_probe/native_console.rs"]
mod native_console;

#[cfg(not(windows))]
fn main() -> std::process::ExitCode {
    std::process::ExitCode::from(2)
}

#[cfg(windows)]
fn main() -> std::process::ExitCode {
    // run returns and drops every mode/registration guard before main terminates.
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(()) => std::process::ExitCode::from(1),
    }
}

#[cfg(windows)]
fn run() -> Result<(), ()> {
    use console::{read_local_key, ConsoleError, ConsolePort, InputEvent};
    use std::{
        cell::Cell,
        io::Write,
        os::windows::fs::MetadataExt,
        path::PathBuf,
        time::{Duration, Instant},
    };
    use windows_sys::Win32::System::{
        Console::*,
        Threading::{GetStartupInfoW, STARTF_USESHOWWINDOW, STARTUPINFOW},
    };
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    if arguments.len() != 2 {
        return Err(());
    }
    let case = arguments[0].to_str().ok_or(())?;
    if ![
        "unicode",
        "escape",
        "control-key",
        "read-error",
        "unwind",
        "signal",
        "late",
        "disable-error",
        "restore-error",
    ]
    .contains(&case)
    {
        return Err(());
    }
    let destination = PathBuf::from(&arguments[1]);
    let parent = destination.parent().ok_or(())?;
    if !destination.is_absolute()
        || !parent
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("notype-probe-console-owned-"))
        || destination.file_name() != Some(std::ffi::OsStr::new(&format!("{case}.txt")))
    {
        return Err(());
    }
    for ancestor in parent.ancestors() {
        let meta = std::fs::symlink_metadata(ancestor).map_err(|_| ())?;
        if !meta.is_dir() || meta.file_attributes() & 0x400 != 0 {
            return Err(());
        }
    }
    let mut startup = STARTUPINFOW::default();
    // SAFETY: startup points to a writable correctly sized Windows structure.
    unsafe {
        GetStartupInfoW(&mut startup);
    }
    if startup.dwFlags & STARTF_USESHOWWINDOW == 0 || startup.wShowWindow != 0 {
        return Err(());
    }
    let owned = native_console::OwnedConsole::open().map_err(|_| ())?;
    let original = owned.input_mode().map_err(|_| ())?;
    // SAFETY: the owned-console check succeeded; this borrowed input handle is
    // used only to enqueue nonsecret records, never closed or read by a second consumer.
    let input = unsafe { GetStdHandle(STD_INPUT_HANDLE) };
    // SAFETY: clears only this newly owned fixture's input, before entry starts.
    if unsafe { FlushConsoleInputBuffer(input) } == 0 {
        return Err(());
    }
    fn key(unit: u16, virtual_key: u16, repeats: u16, control: u32) -> INPUT_RECORD {
        INPUT_RECORD {
            EventType: KEY_EVENT as u16,
            Event: INPUT_RECORD_0 {
                KeyEvent: KEY_EVENT_RECORD {
                    bKeyDown: 1,
                    wRepeatCount: repeats,
                    wVirtualKeyCode: virtual_key,
                    wVirtualScanCode: 0,
                    uChar: KEY_EVENT_RECORD_0 { UnicodeChar: unit },
                    dwControlKeyState: control,
                },
            },
        }
    }
    let events = match case {
        "unicode" => vec![
            key('x' as u16, 0, 2, 0),
            key(0, 8, 1, 0),
            key(0xd83d, 0, 1, 0),
            key(0xde42, 0, 1, 0),
            key(13, 13, 2, 0),
        ],
        "escape" => vec![key('x' as u16, 0, 1, 0), key(27, 27, 1, 0)],
        "control-key" => vec![key(3, 0x43, 1, LEFT_CTRL_PRESSED)],
        "signal" | "read-error" | "unwind" => vec![],
        _ => vec![key('x' as u16, 0, 1, 0), key(13, 13, 1, 0)],
    };
    if !events.is_empty() {
        let mut written = 0;
        // SAFETY: the complete initialized array lives across the call; the
        // borrowed handle belongs solely to this child and written is valid.
        if unsafe { WriteConsoleInputW(input, events.as_ptr(), events.len() as u32, &mut written) }
            == 0
            || written as usize != events.len()
        {
            return Err(());
        }
    }
    struct Port<'a> {
        inner: &'a native_console::OwnedConsole,
        case: &'a str,
        original: u32,
        observed: Cell<bool>,
        injected: Cell<bool>,
        restore_failed: Cell<bool>,
    }
    impl Port<'_> {
        fn signal(&self) -> Result<(), ConsoleError> {
            // SAFETY: the child verified that its new console contains only this
            // process. Broadcast group0 cannot reach the parent/user's console.
            if unsafe { GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, 0) } == 0 {
                return Err(ConsoleError::Read);
            }
            let deadline = Instant::now() + Duration::from_secs(1);
            while !self.inner.interrupted() {
                if Instant::now() >= deadline {
                    return Err(ConsoleError::Read);
                }
                std::thread::sleep(Duration::from_millis(1));
            }
            Ok(())
        }
    }
    impl ConsolePort for Port<'_> {
        fn input_mode(&self) -> Result<u32, ConsoleError> {
            self.inner.input_mode()
        }
        fn set_input_mode(&self, mode: u32) -> Result<(), ConsoleError> {
            if self.case == "restore-error"
                && mode == self.original
                && !self.restore_failed.replace(true)
            {
                return Err(ConsoleError::Restore);
            }
            self.inner.set_input_mode(mode)?;
            if mode & ENABLE_ECHO_INPUT == 0 {
                self.observed.set(true);
                if self.case == "disable-error" {
                    return Err(ConsoleError::Setup);
                }
            }
            Ok(())
        }
        fn install_interrupt_restore(&self, mode: u32) -> Result<(), ConsoleError> {
            self.inner.install_interrupt_restore(mode)
        }
        fn remove_interrupt_restore(&self) -> Result<(), ConsoleError> {
            if self.case == "late" && !self.injected.replace(true) {
                self.signal()?;
            }
            self.inner.remove_interrupt_restore()
        }
        fn interrupted(&self) -> bool {
            self.inner.interrupted()
        }
        fn next_event(&self, timeout: u32) -> Result<Option<InputEvent>, ConsoleError> {
            assert_eq!(self.inner.input_mode()? & ENABLE_ECHO_INPUT, 0);
            self.observed.set(true);
            match self.case {
                "read-error" => return Err(ConsoleError::Read),
                "unwind" => panic!("owned nonsecret fixture unwind"),
                "signal" if !self.injected.replace(true) => {
                    self.signal()?;
                    return Ok(Some(InputEvent::Ignore));
                }
                _ => {}
            }
            self.inner.next_event(timeout)
        }
    }
    let port = Port {
        inner: &owned,
        case,
        original,
        observed: Cell::new(false),
        injected: Cell::new(false),
        restore_failed: Cell::new(false),
    };
    std::panic::set_hook(Box::new(|_| {}));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| read_local_key(&port)));
    let valid = match (case, result) {
        ("unicode", Ok(Ok(material))) => material.into_string() == "x🙂",
        ("escape" | "control-key" | "signal" | "late", Ok(Err(ConsoleError::Interrupted))) => true,
        ("read-error", Ok(Err(ConsoleError::Read))) => true,
        ("unwind", Err(_)) => true,
        ("disable-error", Ok(Err(ConsoleError::Setup))) => true,
        ("restore-error", Ok(Err(ConsoleError::Restore))) => true,
        _ => false,
    };
    if !valid || !port.observed.get() || owned.input_mode().map_err(|_| ())? != original {
        return Err(());
    }
    let mut output = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .map_err(|_| ())?;
    write!(output,"result=pass\nowned=single-process\nhidden_startup=true\nnon_echo_observed=true\nmode_restored=true\ncredential_output=none\n").map_err(|_|())?;
    output.sync_all().map_err(|_| ())
}
