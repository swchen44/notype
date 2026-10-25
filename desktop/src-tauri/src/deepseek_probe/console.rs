//! Non-echo entry algorithm behind an owned-console system port. No native
//! adapter or user-facing executable is enabled by this offline module.

/// Fixed errors deliberately exclude secret input and raw platform messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsoleError {
    /// No owned interactive input console is available; redirected input is denied.
    Unavailable,
    /// Console mode or interrupt restoration could not be prepared safely.
    Setup,
    /// Escape, Ctrl-C or an interrupt stopped entry before a key was returned.
    Interrupted,
    /// Input is empty/whitespace-only or contains an unsupported control character.
    InvalidInput,
    /// The private UTF-8 input buffer would exceed its 1024-byte bound.
    InputTooLong,
    /// Reading the owned console failed; no raw platform/input details are exposed.
    Read,
    /// Restoring mode or removing the restoration handler failed; no key is returned.
    Restore,
}

/// Unicode key events from the future dedicated-console adapter.
pub enum InputEvent {
    /// One Unicode scalar from the owned console; the reader buffers it privately.
    Character(char),
    /// Remove the last Unicode scalar from the private input buffer.
    Backspace,
    /// Submit the buffer after validation and successful mode restoration.
    Enter,
    /// Abort entry without returning key material.
    Escape,
    /// Abort entry using the same restoration path as an asynchronous interrupt.
    ControlC,
    /// An unrelated console event with no input-buffer effect.
    Ignore,
}

/// Console system boundary. Native implementation must reject redirected input,
/// own its console, install idempotent interrupt restoration before changing
/// mode, and honor bounded waits without printing input or logging events.
pub trait ConsolePort {
    /// Read the owned interactive input handle's mode without consuming input.
    /// Reject missing/redirected/foreign input with a fixed error before setup.
    fn input_mode(&self) -> Result<u32, ConsoleError>;
    /// Set mode on that same owned handle; never echo, print or log buffered input.
    /// The caller's guard restores the original mode if setup or reading fails.
    fn set_input_mode(&self, mode: u32) -> Result<(), ConsoleError>;
    /// Install idempotent interrupt restoration before echo mode is changed.
    /// Retain `original_mode` for interruption; an error must undo partial setup
    /// because the caller has not yet created its restoration guard.
    fn install_interrupt_restore(&self, original_mode: u32) -> Result<(), ConsoleError>;
    /// Remove the handler after original mode is restored; tolerate retry by Drop.
    /// Failure returns a fixed code and prevents key material from being returned.
    fn remove_interrupt_restore(&self) -> Result<(), ConsoleError>;
    /// Read the interrupt flag without consuming input or changing console mode.
    /// A pending interrupt must remain observable across a bounded event wait.
    fn interrupted(&self) -> bool;
    /// Wait at most `timeout_ms` on the owned handle, returning None on timeout.
    /// Transfer at most one event; leave key buffering to the reader. Never log
    /// events or input, and allow the reader to recheck interruption after polling.
    fn next_event(&self, timeout_ms: u32) -> Result<Option<InputEvent>, ConsoleError>;
}

/// Secret material deliberately has no Debug/Serialize/Display implementation.
/// This does not promise complete zeroization of process/OS memory copies.
pub struct KeyMaterial(String);
impl KeyMaterial {
    /// Move the key into the actual provider's in-memory configuration later.
    /// The receiver owns this secret and must not persist, print or report it.
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
    if console.interrupted() {
        return Err(ConsoleError::Interrupted);
    }
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
