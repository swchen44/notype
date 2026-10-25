//! Windows physical-key acquisition. Callback retains key edges only for the
//! gesture router; it never records/exports ordinary key text or performs audio/IPC.
use crate::physical_gestures::{Bindings, Intent, Phase};
use tauri::{AppHandle, Manager};

pub fn configured(app: &AppHandle) -> Result<Bindings, String> {
    let settings = crate::settings::get_settings(app);
    let defaults = crate::settings::get_default_settings();
    let value = |id: &str| {
        settings
            .bindings
            .get(id)
            .or_else(|| defaults.bindings.get(id))
            .map(|b| b.current_binding.clone())
            .unwrap_or_default()
    };
    Bindings::parse(
        &value("transcribe"),
        &value("translation"),
        &value("ask"),
        &value("stop"),
    )
}
pub fn initialize(app: &AppHandle) -> Result<(), String> {
    #[cfg(windows)]
    {
        platform::initialize(app, configured(app)?)
    }
    #[cfg(not(windows))]
    {
        let _ = app;
        Err("physical-shortcuts-unsupported".into())
    }
}
pub fn configure(app: &AppHandle, bindings: Bindings) -> Result<(), String> {
    #[cfg(windows)]
    {
        platform::initialize(app, bindings)
    }
    #[cfg(not(windows))]
    {
        let _ = (app, bindings);
        Err("physical-shortcuts-unsupported".into())
    }
}
pub fn suspend(suspended: bool) {
    #[cfg(windows)]
    {
        platform::suspend(suspended);
    }
    #[cfg(not(windows))]
    {
        let _ = suspended;
    }
}
pub fn processing_started() {
    #[cfg(windows)]
    {
        platform::set_phase(2);
    }
}
pub fn processing_finished(app: &AppHandle) {
    #[cfg(windows)]
    {
        let recording = app
            .try_state::<std::sync::Arc<crate::managers::audio::AudioRecordingManager>>()
            .is_some_and(|rm| rm.is_recording());
        let processing = app
            .try_state::<std::sync::Arc<crate::workflow::DesktopWorkflow>>()
            .is_some_and(|workflow| workflow.active_generation().ok().flatten().is_some());
        platform::set_phase(if recording {
            1
        } else if processing {
            2
        } else {
            0
        });
    }
    #[cfg(not(windows))]
    {
        let _ = app;
    }
}

#[cfg(windows)]
mod platform {
    use super::*;
    use crate::{
        native_input::win::{DispatchMessageW, GetMessageW, Msg},
        physical_gestures::GestureRouter,
    };
    use std::sync::{
        atomic::{AtomicBool, AtomicU8, Ordering},
        mpsc::{sync_channel, SyncSender},
        Mutex, OnceLock,
    };
    use std::time::Instant;
    static STATE: OnceLock<Mutex<HookState>> = OnceLock::new();
    static PHASE: AtomicU8 = AtomicU8::new(0);
    static SUSPENDED: AtomicBool = AtomicBool::new(false);
    static READY: AtomicBool = AtomicBool::new(false);
    static INPUT_LOST: AtomicBool = AtomicBool::new(false);
    struct HookState {
        router: GestureRouter,
        sender: SyncSender<(Intent, Instant)>,
    }
    #[repr(C)]
    struct KeyEvent {
        key: u32,
        scan: u32,
        flags: u32,
        time: u32,
        extra: usize,
    }
    #[link(name = "user32")]
    unsafe extern "system" {
        fn SetWindowsHookExW(
            kind: i32,
            callback: Option<unsafe extern "system" fn(i32, usize, isize) -> isize>,
            module: isize,
            thread: u32,
        ) -> isize;
        fn UnhookWindowsHookEx(hook: isize) -> i32;
        fn CallNextHookEx(hook: isize, code: i32, w: usize, l: isize) -> isize;
        fn GetAsyncKeyState(key: i32) -> i16;
        fn SetTimer(
            window: isize,
            id: usize,
            millis: u32,
            callback: Option<unsafe extern "system" fn(isize, u32, usize, u32)>,
        ) -> usize;
        fn KillTimer(window: isize, id: usize) -> i32;
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetModuleHandleW(name: *const u16) -> isize;
    }
    fn phase() -> Phase {
        match PHASE.load(Ordering::SeqCst) {
            0 => Phase::Idle,
            1 => Phase::Recording,
            _ => Phase::Busy,
        }
    }
    pub fn finished() {
        PHASE.store(0, Ordering::SeqCst);
    }
    pub fn set_phase(phase: u8) {
        PHASE.store(phase, Ordering::SeqCst);
    }
    pub fn suspend(value: bool) {
        SUSPENDED.store(value, Ordering::SeqCst);
        if let Some(state) = STATE.get() {
            if let Ok(mut state) = state.lock() {
                state.router.reset();
            }
        }
    }
    pub fn configure(bindings: Bindings) -> Result<(), String> {
        let state = STATE.get().ok_or("physical-shortcuts-unavailable")?;
        let mut state = state.lock().map_err(|_| "physical-shortcuts-unavailable")?;
        state.router.configure(bindings.clone());
        Ok(())
    }
    unsafe extern "system" fn health(_: isize, _: u32, _: usize, _: u32) {
        let Some(state) = STATE.get() else {
            return;
        };
        let Ok(mut state) = state.try_lock() else {
            INPUT_LOST.store(true, Ordering::SeqCst);
            return;
        };
        if INPUT_LOST.swap(false, Ordering::SeqCst) {
            state.router.reset();
        }
        let released: Vec<_> = state
            .router
            .held()
            .filter(|&key| GetAsyncKeyState(key as i32) >= 0)
            .collect();
        if !released.is_empty() {
            state.router.reset();
            for key in released {
                state.router.event(key, false, Phase::Suspended);
            }
        }
    }
    unsafe extern "system" fn hook(code: i32, w: usize, l: isize) -> isize {
        if code < 0 || !matches!(w, 0x100 | 0x101 | 0x104 | 0x105) {
            return CallNextHookEx(0, code, w, l);
        }
        let event = &*(l as *const KeyEvent);
        // Ignore injected input, including synthetic AltGr phantom Ctrl.
        if event.flags & 0x12 != 0 || (event.key == 0xa2 && event.scan == 0x21d) {
            return CallNextHookEx(0, code, w, l);
        }
        let key = match event.key {
            0x10 => {
                if event.scan == 0x36 {
                    0xa1
                } else {
                    0xa0
                }
            }
            0x11 => {
                if event.flags & 1 != 0 {
                    0xa3
                } else {
                    0xa2
                }
            }
            0x12 => {
                if event.flags & 1 != 0 {
                    0xa5
                } else {
                    0xa4
                }
            }
            other => other as u16,
        };
        let down = matches!(w, 0x100 | 0x104);
        let Some(state) = STATE.get() else {
            return CallNextHookEx(0, code, w, l);
        };
        let Ok(mut state) = state.try_lock() else {
            INPUT_LOST.store(true, Ordering::SeqCst);
            return CallNextHookEx(0, code, w, l);
        };
        if INPUT_LOST.swap(false, Ordering::SeqCst) {
            state.router.reset();
        }
        let suspended = SUSPENDED.load(Ordering::SeqCst);
        if suspended {
            state.router.reset();
        }
        let decision = state.router.event(
            key,
            down,
            if suspended { Phase::Suspended } else { phase() },
        );
        if let Some(intent) = decision.intent {
            if !suspended {
                PHASE.store(2, Ordering::SeqCst);
                if state.sender.try_send((intent, Instant::now())).is_err() {
                    state.router.reset();
                }
            }
        }
        drop(state);
        if decision.suppress {
            1
        } else {
            CallNextHookEx(0, code, w, l)
        }
    }
    pub fn initialize(app: &AppHandle, bindings: Bindings) -> Result<(), String> {
        if STATE.get().is_some() {
            return if READY.load(Ordering::SeqCst) {
                configure(bindings)
            } else {
                Err("physical-shortcuts-unavailable".into())
            };
        }
        let (sender, receiver) = sync_channel(8);
        STATE
            .set(Mutex::new(HookState {
                router: GestureRouter::new(bindings.clone()),
                sender,
            }))
            .map_err(|_| "physical-shortcuts-unavailable")?;
        let (ready_tx, ready_rx) = sync_channel(1);
        std::thread::spawn(move || unsafe {
            let hook_handle =
                SetWindowsHookExW(13, Some(hook), GetModuleHandleW(std::ptr::null()), 0);
            let timer = SetTimer(0, 0, 200, Some(health));
            let ready = hook_handle != 0 && timer != 0;
            READY.store(ready, Ordering::SeqCst);
            let _ = ready_tx.send(ready);
            if ready {
                let mut message: Msg = std::mem::zeroed();
                while GetMessageW(&mut message, 0, 0, 0) > 0 {
                    DispatchMessageW(&message);
                }
            }
            READY.store(false, Ordering::SeqCst);
            if timer != 0 {
                KillTimer(0, timer);
            }
            if hook_handle != 0 {
                UnhookWindowsHookEx(hook_handle);
            }
        });
        if ready_rx.recv_timeout(std::time::Duration::from_secs(2)) != Ok(true) {
            return Err("physical-shortcuts-unavailable".into());
        }
        let app = app.clone();
        std::thread::spawn(move || {
            while let Ok((intent, accepted)) = receiver.recv() {
                use crate::actions::ACTION_MAP;
                use crate::diagnostics::{record, Outcome, Phase as DiagnosticPhase};
                record(DiagnosticPhase::ShortcutAccepted, Outcome::Ok, None);
                record(
                    DiagnosticPhase::ShortcutDispatch,
                    Outcome::Started,
                    Some(accepted.elapsed().as_millis() as u64),
                );
                match intent {
                    Intent::Dictation | Intent::Translation | Intent::Ask => {
                        let id = match intent {
                            Intent::Translation => "translation",
                            Intent::Ask => "ask",
                            _ => "transcribe",
                        };
                        if let Some(action) = ACTION_MAP.get(id) {
                            action.start(&app, id, "");
                        }
                        let recording = app
                            .state::<std::sync::Arc<crate::managers::audio::AudioRecordingManager>>(
                            )
                            .is_recording();
                        PHASE.store(if recording { 1 } else { 0 }, Ordering::SeqCst);
                    }
                    Intent::Stop => {
                        let id = app
                            .state::<std::sync::Arc<crate::managers::audio::AudioRecordingManager>>(
                            )
                            .recording_binding();
                        if let Some(id) = id {
                            if let Some(action) = ACTION_MAP.get(&id) {
                                action.stop(&app, &id, "");
                            }
                        } else {
                            finished();
                        }
                    }
                    Intent::Cancel => {
                        crate::utils::cancel_current_operation(&app);
                        finished();
                    }
                }
            }
        });
        Ok(())
    }
}
