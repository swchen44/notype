//! One-shot access to a newly owned Windows console. Handler state remains
//! process-static because unregistering does not join callbacks already in flight.

#[cfg(not(windows))]
/// Native key entry is unavailable on this helper's unsupported platforms.
pub struct OwnedConsole;
#[cfg(not(windows))]
impl OwnedConsole {
    /// Return an explicit capability failure without reading stdin.
    pub fn open() -> Result<Self, super::console::ConsoleError> {
        Err(super::console::ConsoleError::Unavailable)
    }
}

#[cfg(windows)]
pub use platform::OwnedConsole;

#[cfg(windows)]
mod platform {
    use super::super::console::{ConsoleError, ConsolePort, InputEvent};
    use std::{
        cell::{Cell, RefCell},
        sync::atomic::{AtomicBool, AtomicIsize, AtomicU32, Ordering},
    };
    use windows_sys::{
        core::BOOL,
        Win32::{
            Foundation::{HANDLE, INVALID_HANDLE_VALUE, WAIT_OBJECT_0, WAIT_TIMEOUT},
            System::{
                Console::*,
                Threading::{GetCurrentProcessId, WaitForSingleObject},
            },
        },
    };

    static TAKEN: AtomicBool = AtomicBool::new(false);
    static ACTIVE: AtomicBool = AtomicBool::new(false);
    static INTERRUPTED: AtomicBool = AtomicBool::new(false);
    static RESTORE_FAILED: AtomicBool = AtomicBool::new(false);
    static INPUT: AtomicIsize = AtomicIsize::new(0);
    static ORIGINAL: AtomicU32 = AtomicU32::new(0);

    // The OS calls this on another thread. It touches only process-static
    // atomics and a borrowed standard handle that is never closed by this module.
    // No stack/closure pointer, allocation, lock, formatting or database is used.
    unsafe extern "system" fn interrupt(kind: u32) -> BOOL {
        if !matches!(
            kind,
            CTRL_C_EVENT
                | CTRL_BREAK_EVENT
                | CTRL_CLOSE_EVENT
                | CTRL_LOGOFF_EVENT
                | CTRL_SHUTDOWN_EVENT
        ) {
            return 0;
        }
        INTERRUPTED.store(true, Ordering::SeqCst);
        if ACTIVE.load(Ordering::SeqCst) {
            let input = INPUT.load(Ordering::SeqCst) as HANDLE;
            let original = ORIGINAL.load(Ordering::SeqCst);
            // SAFETY: registration occurs only after a valid owned input handle
            // and its mode are stored; the borrowed handle lives for the process.
            if !input.is_null() && unsafe { SetConsoleMode(input, original) } == 0 {
                RESTORE_FAILED.store(true, Ordering::SeqCst);
            }
        }
        1
    }

    #[derive(Clone, Copy)]
    enum Repeated {
        Character(char),
        Backspace,
    }
    #[derive(Default)]
    struct Decode {
        high: Option<u16>,
        repeated: Option<(Repeated, u16)>,
    }

    /// Borrows stdin in a console containing exactly this process. It cannot be
    /// reopened in the same process, preventing stale handler callbacks from
    /// restoring an earlier entry's mode during a second secret read.
    pub struct OwnedConsole {
        input: HANDLE,
        original: u32,
        registered: Cell<bool>,
        decode: RefCell<Decode>,
    }
    impl OwnedConsole {
        /// Verify a real input console and sole-process ownership before secret
        /// entry. Redirected or shared consoles are rejected without mutation.
        pub fn open() -> Result<Self, ConsoleError> {
            let mut process = 0u32;
            // SAFETY: the one-element buffer is valid. Its contents are read only
            // when the reported count is exactly one; a larger count is not data.
            let count = unsafe { GetConsoleProcessList(&mut process, 1) };
            // SAFETY: retrieving this process's ID has no ownership requirements.
            if count != 1 || process != unsafe { GetCurrentProcessId() } {
                return Err(ConsoleError::Unavailable);
            }
            // SAFETY: standard input is borrowed, never closed here.
            let input = unsafe { GetStdHandle(STD_INPUT_HANDLE) };
            if input.is_null() || input == INVALID_HANDLE_VALUE {
                return Err(ConsoleError::Unavailable);
            }
            let mut original = 0;
            // SAFETY: input is a non-null borrowed standard handle and output is
            // a live writable u32. Failure is the redirected/unavailable boundary.
            if unsafe { GetConsoleMode(input, &mut original) } == 0 {
                return Err(ConsoleError::Unavailable);
            }
            if TAKEN
                .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                .is_err()
            {
                return Err(ConsoleError::Unavailable);
            }
            Ok(Self {
                input,
                original,
                registered: Cell::new(false),
                decode: RefCell::new(Decode::default()),
            })
        }
        /// Discard queued input only after mode restoration, before a separate
        /// nonsecret confirmation. Pasted/repeated Enter cannot approve a round.
        pub fn clear_pending_input(&self) -> Result<(), ConsoleError> {
            if self.registered.get() || self.input_mode()? != self.original {
                return Err(ConsoleError::Restore);
            }
            // SAFETY: exclusively owned borrowed input; secret reading is over
            // and no second reader is active while this queue is cleared.
            if unsafe { FlushConsoleInputBuffer(self.input) } == 0 {
                return Err(ConsoleError::Read);
            }
            *self.decode.borrow_mut() = Decode::default();
            Ok(())
        }
        /// Read one bounded nonsecret confirmation after exact mode restoration.
        /// ReadConsoleW avoids a separate Rust stdin buffer retaining pasted
        /// future approvals that FlushConsoleInputBuffer could not discard.
        pub fn confirmation_line(&self) -> Result<String, ConsoleError> {
            if self.registered.get()
                || self.input_mode()? != self.original
                || self.original & ENABLE_LINE_INPUT == 0
            {
                return Err(ConsoleError::Unavailable);
            }
            let mut buffer = [0u16; 256];
            let mut read = 0;
            // SAFETY: the owned borrowed handle and fixed writable UTF-16 buffer
            // live for this blocking line read. No secret entry/second reader is
            // active, and the optional wakeup control is null.
            if unsafe {
                ReadConsoleW(
                    self.input,
                    buffer.as_mut_ptr().cast(),
                    buffer.len() as u32,
                    &mut read,
                    std::ptr::null(),
                )
            } == 0
                || read == 0
                || read as usize >= buffer.len()
            {
                return Err(ConsoleError::Read);
            }
            String::from_utf16(&buffer[..read as usize]).map_err(|_| ConsoleError::InvalidInput)
        }
        fn repeated(&self) -> Option<InputEvent> {
            let mut decode = self.decode.borrow_mut();
            let (event, left) = decode.repeated?;
            decode.repeated = if left > 1 {
                Some((event, left - 1))
            } else {
                None
            };
            Some(match event {
                Repeated::Character(c) => InputEvent::Character(c),
                Repeated::Backspace => InputEvent::Backspace,
            })
        }
        fn key(&self, key: KEY_EVENT_RECORD) -> Result<InputEvent, ConsoleError> {
            if key.bKeyDown == 0 {
                return Ok(InputEvent::Ignore);
            }
            if key.wVirtualKeyCode == 0x43
                && key.dwControlKeyState & (LEFT_CTRL_PRESSED | RIGHT_CTRL_PRESSED) != 0
            {
                return Ok(InputEvent::ControlC);
            }
            match key.wVirtualKeyCode {
                0x0d => {
                    if self.decode.borrow().high.is_some() {
                        return Err(ConsoleError::InvalidInput);
                    }
                    return Ok(InputEvent::Enter);
                }
                0x1b => return Ok(InputEvent::Escape),
                0x08 => {
                    self.decode.borrow_mut().high = None;
                    if key.wRepeatCount > 1 {
                        self.decode.borrow_mut().repeated =
                            Some((Repeated::Backspace, key.wRepeatCount - 1));
                    }
                    return Ok(InputEvent::Backspace);
                }
                _ => {}
            }
            // SAFETY: EventType was KEY_EVENT before this typed record was copied;
            // the Unicode member is the documented W API representation.
            let unit = unsafe { key.uChar.UnicodeChar };
            if unit == 0 {
                return Ok(InputEvent::Ignore);
            }
            let mut decode = self.decode.borrow_mut();
            if (0xd800..=0xdbff).contains(&unit) {
                if decode.high.replace(unit).is_some() {
                    return Err(ConsoleError::InvalidInput);
                }
                return Ok(InputEvent::Ignore);
            }
            let character = if let Some(high) = decode.high.take() {
                if !(0xdc00..=0xdfff).contains(&unit) {
                    return Err(ConsoleError::InvalidInput);
                }
                char::decode_utf16([high, unit])
                    .next()
                    .and_then(Result::ok)
                    .ok_or(ConsoleError::InvalidInput)?
            } else {
                char::from_u32(u32::from(unit)).ok_or(ConsoleError::InvalidInput)?
            };
            if key.wRepeatCount > 1 {
                decode.repeated = Some((Repeated::Character(character), key.wRepeatCount - 1));
            }
            Ok(InputEvent::Character(character))
        }
    }
    impl ConsolePort for OwnedConsole {
        fn input_mode(&self) -> Result<u32, ConsoleError> {
            let mut mode = 0;
            // SAFETY: the borrowed standard handle and writable mode out-pointer
            // remain valid for this object's whole process lifetime.
            if unsafe { GetConsoleMode(self.input, &mut mode) } == 0 {
                Err(ConsoleError::Unavailable)
            } else {
                Ok(mode)
            }
        }
        fn set_input_mode(&self, mode: u32) -> Result<(), ConsoleError> {
            let restoring = mode == ORIGINAL.load(Ordering::SeqCst);
            if !restoring && self.interrupted() {
                return Err(ConsoleError::Interrupted);
            }
            // SAFETY: this object exclusively borrows the owned console; mode is
            // the saved original or its echo-disabled form, no foreign handle.
            if unsafe { SetConsoleMode(self.input, mode) } == 0 {
                return Err(if restoring {
                    ConsoleError::Restore
                } else {
                    ConsoleError::Setup
                });
            }
            if !restoring && self.interrupted() {
                // SAFETY: same valid handle; close the race where the handler
                // restored first and this thread then disabled echo again.
                if unsafe { SetConsoleMode(self.input, ORIGINAL.load(Ordering::SeqCst)) } == 0 {
                    RESTORE_FAILED.store(true, Ordering::SeqCst);
                }
                return Err(ConsoleError::Interrupted);
            }
            Ok(())
        }
        fn install_interrupt_restore(&self, original_mode: u32) -> Result<(), ConsoleError> {
            if self.registered.get() {
                return Err(ConsoleError::Setup);
            }
            INPUT.store(self.input as isize, Ordering::SeqCst);
            ORIGINAL.store(original_mode, Ordering::SeqCst);
            INTERRUPTED.store(false, Ordering::SeqCst);
            RESTORE_FAILED.store(false, Ordering::SeqCst);
            // SAFETY: callback has the exact system ABI and references only
            // process-static state. No borrowed callback data can outlive entry.
            if unsafe { SetConsoleCtrlHandler(Some(interrupt), 1) } == 0 {
                return Err(ConsoleError::Setup);
            }
            self.registered.set(true);
            ACTIVE.store(true, Ordering::SeqCst);
            Ok(())
        }
        fn remove_interrupt_restore(&self) -> Result<(), ConsoleError> {
            if self.registered.get() {
                // SAFETY: removes exactly the previously registered static
                // function. Static state and borrowed handle are not freed.
                if unsafe { SetConsoleCtrlHandler(Some(interrupt), 0) } == 0 {
                    return Err(ConsoleError::Restore);
                }
                self.registered.set(false);
                ACTIVE.store(false, Ordering::SeqCst);
            }
            if RESTORE_FAILED.load(Ordering::SeqCst) {
                return Err(ConsoleError::Restore);
            }
            Ok(())
        }
        fn interrupted(&self) -> bool {
            INTERRUPTED.load(Ordering::SeqCst)
        }
        fn next_event(&self, timeout_ms: u32) -> Result<Option<InputEvent>, ConsoleError> {
            if let Some(event) = self.repeated() {
                return Ok(Some(event));
            }
            // SAFETY: waiting on the owned input handle does not consume it;
            // this is the single consumer, so the following one-record read
            // cannot race another reader that drains the signaled input queue.
            match unsafe { WaitForSingleObject(self.input, timeout_ms) } {
                WAIT_TIMEOUT => return Ok(None),
                WAIT_OBJECT_0 => {}
                _ => return Err(ConsoleError::Read),
            }
            let mut record = INPUT_RECORD::default();
            let mut count = 0;
            // SAFETY: one initialized record and one writable count are valid;
            // ReadConsoleInputW receives nLength1 only after signaled readiness.
            if unsafe { ReadConsoleInputW(self.input, &mut record, 1, &mut count) } == 0
                || count != 1
            {
                return Err(ConsoleError::Read);
            }
            if record.EventType != KEY_EVENT as u16 {
                return Ok(Some(InputEvent::Ignore));
            }
            // SAFETY: the checked tag identifies the KeyEvent union member.
            self.key(unsafe { record.Event.KeyEvent }).map(Some)
        }
    }
    impl Drop for OwnedConsole {
        fn drop(&mut self) {
            if self.registered.get() {
                // SAFETY: best-effort restoration on the same borrowed input;
                // owned-console isolation, not cleanup during power loss, is
                // the guarantee. No handle or static callback state is freed.
                unsafe {
                    SetConsoleMode(self.input, self.original);
                }
                let _ = self.remove_interrupt_restore();
            }
        }
    }
}
