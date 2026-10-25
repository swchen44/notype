#[path = "../../desktop/src-tauri/src/deepseek_probe/console.rs"]
mod console;
use console::{read_local_key, ConsoleError, ConsolePort, InputEvent};
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
};

struct FakeConsole {
    mode: Cell<u32>,
    original: u32,
    handler: Cell<bool>,
    events: RefCell<VecDeque<InputEvent>>,
    interrupt_on_read: Cell<bool>,
    interrupted: Cell<bool>,
    panic_on_read: Cell<bool>,
    fail_read: Cell<bool>,
    redirected: Cell<bool>,
    fail_disable: Cell<bool>,
    fail_restore: Cell<bool>,
}
impl FakeConsole {
    fn new(events: Vec<InputEvent>) -> Self {
        Self {
            mode: Cell::new(0x2ff),
            original: 0x2ff,
            handler: Cell::new(false),
            events: RefCell::new(events.into()),
            interrupt_on_read: Cell::new(false),
            interrupted: Cell::new(false),
            panic_on_read: Cell::new(false),
            fail_read: Cell::new(false),
            redirected: Cell::new(false),
            fail_disable: Cell::new(false),
            fail_restore: Cell::new(false),
        }
    }
}
impl ConsolePort for FakeConsole {
    fn input_mode(&self) -> Result<u32, ConsoleError> {
        if self.redirected.get() {
            Err(ConsoleError::Unavailable)
        } else {
            Ok(self.mode.get())
        }
    }
    fn set_input_mode(&self, mode: u32) -> Result<(), ConsoleError> {
        if mode == self.original && self.fail_restore.get() {
            return Err(ConsoleError::Restore);
        }
        self.mode.set(mode);
        if mode != self.original && self.fail_disable.get() {
            return Err(ConsoleError::Setup);
        }
        Ok(())
    }
    fn install_interrupt_restore(&self, _original: u32) -> Result<(), ConsoleError> {
        self.handler.set(true);
        Ok(())
    }
    fn remove_interrupt_restore(&self) -> Result<(), ConsoleError> {
        self.handler.set(false);
        Ok(())
    }
    fn interrupted(&self) -> bool {
        self.interrupted.get()
    }
    fn next_event(&self, timeout_ms: u32) -> Result<Option<InputEvent>, ConsoleError> {
        assert_eq!(timeout_ms, 50);
        assert_eq!(self.mode.get() & 4, 0, "fixture input must never be echoed");
        assert!(
            self.handler.get(),
            "interrupt restoration must exist before input"
        );
        assert!(!self.panic_on_read.get(), "nonsecret fixture unwind");
        if self.fail_read.get() {
            return Err(ConsoleError::Read);
        }
        if self.interrupt_on_read.get() {
            self.interrupted.set(true);
        }
        Ok(self.events.borrow_mut().pop_front())
    }
}

#[test]
fn input_is_non_echoed_and_original_mode_is_restored_before_key_returns() {
    let fake = FakeConsole::new(vec![
        InputEvent::Character('x'),
        InputEvent::Character('y'),
        InputEvent::Enter,
    ]);
    let key = read_local_key(&fake).unwrap().into_string();
    assert_eq!(key, "xy"); // Nonsecret fixture, never a real credential.
    assert_eq!(fake.mode.get(), fake.original);
    assert!(!fake.handler.get());
}

#[test]
fn abort_invalid_input_and_length_limit_never_return_key_material() {
    for (event, expected) in [
        (InputEvent::Escape, ConsoleError::Interrupted),
        (InputEvent::ControlC, ConsoleError::Interrupted),
        (InputEvent::Character('\0'), ConsoleError::InvalidInput),
        (InputEvent::Character('\n'), ConsoleError::InvalidInput),
    ] {
        let fake = FakeConsole::new(vec![InputEvent::Character('x'), event, InputEvent::Enter]);
        assert!(matches!(read_local_key(&fake), Err(e) if e == expected));
        assert_eq!(fake.mode.get(), fake.original);
        assert!(!fake.handler.get());
    }
    let mut events: Vec<_> = (0..1025).map(|_| InputEvent::Character('x')).collect();
    events.push(InputEvent::Enter);
    let fake = FakeConsole::new(events);
    assert!(matches!(
        read_local_key(&fake),
        Err(ConsoleError::InputTooLong)
    ));
    assert_eq!(fake.mode.get(), fake.original);
    let empty = FakeConsole::new(vec![InputEvent::Enter]);
    assert!(matches!(
        read_local_key(&empty),
        Err(ConsoleError::InvalidInput)
    ));
}

#[test]
fn silent_backspace_edits_one_unicode_character() {
    let fake = FakeConsole::new(vec![
        InputEvent::Character('x'),
        InputEvent::Character('文'),
        InputEvent::Backspace,
        InputEvent::Character('y'),
        InputEvent::Enter,
    ]);
    assert_eq!(read_local_key(&fake).unwrap().into_string(), "xy");
}

#[test]
fn interrupt_wins_when_enter_and_control_signal_arrive_together() {
    let fake = FakeConsole::new(vec![InputEvent::Character('x'), InputEvent::Enter]);
    fake.interrupt_on_read.set(true);
    assert!(matches!(
        read_local_key(&fake),
        Err(ConsoleError::Interrupted)
    ));
    assert_eq!(fake.mode.get(), fake.original);
    assert!(!fake.handler.get());
}

#[test]
fn read_error_setup_error_and_unwind_restore_the_owned_console() {
    let read_error = FakeConsole::new(vec![]);
    read_error.fail_read.set(true);
    assert!(matches!(
        read_local_key(&read_error),
        Err(ConsoleError::Read)
    ));
    assert_eq!(read_error.mode.get(), read_error.original);
    assert!(!read_error.handler.get());
    let setup_error = FakeConsole::new(vec![]);
    setup_error.fail_disable.set(true); // Failure after a partial mode change.
    assert!(matches!(
        read_local_key(&setup_error),
        Err(ConsoleError::Setup)
    ));
    assert_eq!(setup_error.mode.get(), setup_error.original);
    assert!(!setup_error.handler.get());
    let unwind = FakeConsole::new(vec![]);
    unwind.panic_on_read.set(true);
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| read_local_key(&unwind))).is_err()
    );
    assert_eq!(unwind.mode.get(), unwind.original);
    assert!(!unwind.handler.get());
}

#[test]
fn redirected_input_and_failed_restoration_never_release_key_material() {
    let redirected = FakeConsole::new(vec![]);
    redirected.redirected.set(true);
    assert!(matches!(
        read_local_key(&redirected),
        Err(ConsoleError::Unavailable)
    ));
    assert_eq!(redirected.mode.get(), redirected.original);
    assert!(!redirected.handler.get());
    let restore = FakeConsole::new(vec![InputEvent::Character('x'), InputEvent::Enter]);
    restore.fail_restore.set(true);
    assert!(matches!(
        read_local_key(&restore),
        Err(ConsoleError::Restore)
    ));
    // We do not pretend the native mode was restored after a real restore error.
    assert_ne!(restore.mode.get(), restore.original);
}
