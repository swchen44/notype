//! Serialized construction/publication of one native voice runtime.
use std::sync::Mutex;

pub struct RuntimeStartup {
    operation: Mutex<()>,
}
impl RuntimeStartup {
    pub const fn new() -> Self {
        Self {
            operation: Mutex::new(()),
        }
    }
    /// System factories and publication run under the same readiness gate.
    pub fn initialize<R: Clone, A, T, E>(
        &self,
        ready: impl FnOnce() -> bool,
        router: impl FnOnce() -> R,
        audio: impl FnOnce(R) -> Result<A, E>,
        transcription: impl FnOnce(R) -> Result<T, E>,
        publish: impl FnOnce(A, T) -> Result<(), E>,
    ) -> Result<bool, E> {
        let _operation = self.operation.lock().unwrap_or_else(|e| e.into_inner());
        if ready() {
            return Ok(false);
        }
        let router = router();
        let audio = audio(router.clone())?;
        let transcription = transcription(router)?;
        publish(audio, transcription)?;
        Ok(true)
    }
}
impl Default for RuntimeStartup {
    fn default() -> Self {
        Self::new()
    }
}
