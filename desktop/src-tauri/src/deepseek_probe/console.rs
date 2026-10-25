//! Non-echo entry algorithm behind an owned-console system port. No native
//! adapter or user-facing executable is enabled by this offline module.

/// Fixed errors deliberately exclude secret input and raw platform messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsoleError {
    Unavailable,
    Setup,
    Interrupted,
    InvalidInput,
    InputTooLong,
    Read,
    Restore,
}

/// Unicode key events from the future dedicated-console adapter.
pub enum InputEvent {
    Character(char),
    Backspace,
    Enter,
    Escape,
    ControlC,
    Ignore,
}

/// Console system boundary. Native implementation must reject redirected input,
/// own its console, install idempotent interrupt restoration before changing
/// mode, and honor bounded waits without printing input or logging events.
pub trait ConsolePort {
    fn input_mode(&self) -> Result<u32, ConsoleError>;
    fn set_input_mode(&self, mode: u32) -> Result<(), ConsoleError>;
    fn install_interrupt_restore(&self, original_mode: u32) -> Result<(), ConsoleError>;
    fn remove_interrupt_restore(&self) -> Result<(), ConsoleError>;
    fn interrupted(&self) -> bool;
    fn next_event(&self, timeout_ms: u32) -> Result<Option<InputEvent>, ConsoleError>;
}

/// Secret material deliberately has no Debug/Serialize/Display implementation.
/// This does not promise complete zeroization of process/OS memory copies.
pub struct KeyMaterial(String);
impl KeyMaterial {
    /// Move the key into the actual provider's in-memory configuration later.
    pub fn into_string(self) -> String {
        self.0
    }
}

/// Read locally through the dedicated-console port, restoring mode before
/// returning key material. Production console adapter remains separately gated.
pub fn read_local_key(console: &impl ConsolePort) -> Result<KeyMaterial, ConsoleError> {
    let original_mode = console.input_mode()?;
    console.install_interrupt_restore(original_mode)?;
    let mut guard = ModeGuard {
        console,
        original_mode,
        active: true,
    };
    console.set_input_mode(original_mode & !4)?;
    let result = read_events(console);
    guard.restore()?;
    result
}

struct ModeGuard<'a, C: ConsolePort> {
    console: &'a C,
    original_mode: u32,
    active: bool,
}
impl<C: ConsolePort> ModeGuard<'_, C> {
    fn restore(&mut self) -> Result<(), ConsoleError> {
        if self.active {
            self.console
                .set_input_mode(self.original_mode)
                .map_err(|_| ConsoleError::Restore)?;
            self.console
                .remove_interrupt_restore()
                .map_err(|_| ConsoleError::Restore)?;
            self.active = false;
        }
        Ok(())
    }
}
impl<C: ConsolePort> Drop for ModeGuard<'_, C> {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

fn read_events(console: &impl ConsolePort) -> Result<KeyMaterial, ConsoleError> {
    let mut input = String::new();
    loop {
        if console.interrupted() {
            return Err(ConsoleError::Interrupted);
        }
        let event = console.next_event(50)?;
        if console.interrupted() {
            return Err(ConsoleError::Interrupted);
        }
        if let Some(event) = event {
            match event {
                InputEvent::Character(c) => {
                    if c.is_control() {
                        return Err(ConsoleError::InvalidInput);
                    }
                    if input.len() + c.len_utf8() > 1024 {
                        return Err(ConsoleError::InputTooLong);
                    }
                    input.push(c);
                }
                InputEvent::Backspace => {
                    input.pop();
                }
                InputEvent::Enter => {
                    if input.trim().is_empty() {
                        return Err(ConsoleError::InvalidInput);
                    }
                    return Ok(KeyMaterial(input));
                }
                InputEvent::Escape | InputEvent::ControlC => return Err(ConsoleError::Interrupted),
                InputEvent::Ignore => {}
            }
        }
    }
}
