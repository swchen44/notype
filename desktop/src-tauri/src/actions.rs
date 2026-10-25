use crate::audio_feedback::{play_feedback_sound, play_feedback_sound_blocking, SoundType};
use crate::audio_toolkit::{is_microphone_access_denied, is_no_input_device_error, VadPolicy};
use crate::managers::audio::AudioRecordingManager;
use crate::managers::model::ModelManager;
use crate::managers::transcription::StreamWorkKind;
use crate::managers::transcription::TranscriptionManager;
use crate::settings::{get_settings, OverlayStyle};
use crate::shortcut;
use crate::tray::{set_tray_state, TrayIconState};
use crate::utils::{
    self, show_processing_overlay, show_recording_overlay, show_transcribing_overlay,
};
use crate::TranscriptionCoordinator;
use log::{debug, warn};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Arc;
#[cfg(debug_assertions)]
use std::time::Duration;
use std::time::Instant;
use tauri::Manager;
use tauri::{AppHandle, Emitter};

#[derive(Clone, serde::Serialize)]
struct RecordingErrorEvent {
    error_type: String,
    detail: Option<String>,
}

/// Drop guard that finishes the transcription pipeline, including immediate
/// model unloading on early exits.
struct FinishGuard {
    app: AppHandle,
    owner: crate::workflow::GenerationToken,
    _cleanup: crate::workflow::GenerationCleanup,
}
/// Ownership is rechecked when the queued UI operation actually executes.
pub(crate) fn generation_ui(
    app: &AppHandle,
    owner: crate::workflow::GenerationToken,
    operation: impl FnOnce(&AppHandle) + Send + 'static,
) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        owner.run(|| operation(&handle));
    });
}
impl Drop for FinishGuard {
    fn drop(&mut self) {
        generation_ui(&self.app, self.owner.clone(), |app| {
            utils::hide_recording_overlay(app);
            set_tray_state(app, TrayIconState::Idle);
        });
    }
}

// Shortcut Action Trait
pub trait ShortcutAction: Send + Sync {
    fn start(&self, app: &AppHandle, binding_id: &str, shortcut_str: &str);
    fn stop(&self, app: &AppHandle, binding_id: &str, shortcut_str: &str);
}

// Transcribe Action
struct TranscribeAction;

fn should_use_streaming_overlay(style: OverlayStyle, is_streaming: bool) -> bool {
    style == OverlayStyle::Live && is_streaming
}

impl ShortcutAction for TranscribeAction {
    fn start(&self, app: &AppHandle, binding_id: &str, _shortcut_str: &str) {
        let start_time = Instant::now();
        debug!("TranscribeAction::start called for binding: {}", binding_id);

        // Load model in the background
        let tm = app.state::<Arc<TranscriptionManager>>();
        let rm = app.state::<Arc<AudioRecordingManager>>();

        // Load ASR model and VAD model in parallel
        let kickoff_started = Instant::now();
        tm.initiate_model_load();
        let rm_clone = Arc::clone(&rm);
        std::thread::spawn(move || {
            if let Err(e) = rm_clone.preload_vad() {
                debug!("VAD pre-load failed: {}", e);
            }
        });
        let kickoff_elapsed = kickoff_started.elapsed();

        // Don't open the mic if nothing can transcribe the recording; the load
        // kicked off above fails and reports why.
        if !tm.is_model_loaded() {
            let selected_model = get_settings(app).selected_model;
            if let Err(e) = app
                .state::<Arc<ModelManager>>()
                .get_model_path(&selected_model)
            {
                warn!("Not starting recording: no model can transcribe it ({})", e);
                return;
            }
        }

        let scenario = match binding_id {
            "translation" => notype_core::Scenario::Translation,
            "ask" => notype_core::Scenario::Ask,
            _ => notype_core::Scenario::Dictation,
        };
        let generation = match crate::commands::history::begin_captured_scenario(app, scenario) {
            Ok(generation) => generation,
            Err(code) => {
                let _ = app.emit("session-error", code);
                return;
            }
        };
        let workflow = app.state::<Arc<crate::workflow::DesktopWorkflow>>();
        let owner = workflow.token(generation);
        let binding_id = binding_id.to_string();
        let tray_started = Instant::now();
        generation_ui(app, owner.clone(), |app| {
            set_tray_state(app, TrayIconState::Recording)
        });
        let tray_elapsed = tray_started.elapsed();

        // Get the microphone mode to determine audio feedback timing
        let plan_started = Instant::now();
        let settings = get_settings(app);
        let is_always_on = settings.always_on_microphone;

        let selected_model_info = app
            .state::<Arc<ModelManager>>()
            .get_model_info(&settings.selected_model);

        // Use the app-facing model capability as the single pre-recording source
        // for live streaming decisions. Unknown support is represented as false
        // until the model registry is updated by discovery or runtime load.
        let model_supports_streaming = selected_model_info
            .as_ref()
            .map(|m| m.supports_streaming)
            .unwrap_or(false);
        let vad_policy = if !settings.vad_enabled {
            VadPolicy::Disabled
        } else if model_supports_streaming {
            VadPolicy::Streaming
        } else {
            VadPolicy::Offline
        };
        if model_supports_streaming {
            if let Ok(profile) = workflow.recognition_profile(generation) {
                owner.run(|| tm.start_stream_with_profile(profile, owner.clone()));
            }
        }
        let plan_elapsed = plan_started.elapsed();

        // Sizing the overlay follows the same advertised capability. A model that
        // doesn't stream (or whose capability is not known yet) gets the compact
        // pill instead of an oversized transparent live window.
        let overlay_started = Instant::now();
        generation_ui(app, owner.clone(), move |app| {
            match settings.overlay_style {
                OverlayStyle::Live if model_supports_streaming => {
                    utils::show_streaming_overlay(app)
                }
                OverlayStyle::Live | OverlayStyle::Minimal => show_recording_overlay(app),
                OverlayStyle::None => {} // show_overlay_state no-ops on None anyway
            }
        });
        // Everything above runs before capture can begin, so each span here is
        // added keypress->capture latency.
        debug!(
            "start-path pre-recording steps: model_kickoff={:?} tray={:?} settings+stream_plan={:?} overlay={:?}",
            kickoff_elapsed,
            tray_elapsed,
            plan_elapsed,
            overlay_started.elapsed()
        );
        debug!("Microphone mode - always_on: {}", is_always_on);

        let mut recording_error: Option<String> = None;
        let recording_start_time = Instant::now();
        let Some(started) = owner.run(|| rm.try_start_recording(&binding_id, vad_policy)) else {
            return;
        };
        match started {
            Ok(readiness) => {
                debug!(
                    "Recording request accepted in {:?}; waiting for first microphone samples",
                    recording_start_time.elapsed()
                );
                let generation = readiness.generation();
                let app_clone = app.clone();
                let rm_clone = Arc::clone(&rm);
                let ready_owner = owner.clone();
                std::thread::spawn(move || {
                    if !readiness.wait() {
                        debug!("Microphone readiness wait ended without receiving samples");
                        return;
                    }

                    // Development-only preview hook for evaluating the brief
                    // arming animation on hardware that normally starts too fast
                    // to make it visible.
                    #[cfg(debug_assertions)]
                    if let Ok(delay_ms) = std::env::var("HANDY_DEBUG_MIC_READY_DELAY_MS")
                        .unwrap_or_default()
                        .parse::<u64>()
                    {
                        let delay_ms = delay_ms.min(10_000);
                        if delay_ms > 0 {
                            debug!("Delaying microphone-ready cue by {delay_ms}ms for UI preview");
                            std::thread::sleep(Duration::from_millis(delay_ms));
                        }
                    }

                    if !rm_clone.is_recording_readiness_current(generation) {
                        debug!("Microphone became ready for an inactive recording");
                        return;
                    }

                    debug!("Microphone is receiving samples; recording is ready");
                    ready_owner.run(|| utils::emit_recording_ready(&app_clone));

                    // The start chime is a readiness cue, so it must follow the
                    // first real input callback rather than Stream::play() or a
                    // fixed delay. The helper returns immediately when feedback
                    // is disabled; mute still follows the same readiness point.
                    ready_owner.readiness_feedback(
                        || rm_clone.is_recording_readiness_current(generation),
                        || play_feedback_sound_blocking(&app_clone, SoundType::Start),
                        || rm_clone.apply_mute(),
                    );
                });
            }
            Err(e) => {
                debug!("Failed to start recording: {}", e);
                recording_error = Some(e);
            }
        }

        if recording_error.is_none() {
            // Dynamically register the cancel shortcut in a separate task to avoid deadlock
            owner.run(|| shortcut::register_cancel_shortcut(app));
        } else {
            let _ = owner.cancel();
            // Starting failed (for example due to blocked microphone permissions).
            // Revert UI state so we don't stay stuck in the recording overlay.
            owner.run(|| tm.cancel_stream());
            generation_ui(app, owner.clone(), |app| {
                utils::hide_recording_overlay(app);
                set_tray_state(app, TrayIconState::Idle);
            });
            if let Some(err) = recording_error {
                let error_type = if is_microphone_access_denied(&err) {
                    "microphone_permission_denied"
                } else if is_no_input_device_error(&err) {
                    "no_input_device"
                } else {
                    "unknown"
                };
                let _ = app.emit(
                    "recording-error",
                    RecordingErrorEvent {
                        error_type: error_type.to_string(),
                        detail: Some(err),
                    },
                );
            }
        }

        owner.run(|| crate::physical_shortcuts::processing_finished(app));
        debug!(
            "TranscribeAction::start completed in {:?}",
            start_time.elapsed()
        );
    }

    fn stop(&self, app: &AppHandle, binding_id: &str, _shortcut_str: &str) {
        let ah = app.clone();
        let rm = Arc::clone(&app.state::<Arc<AudioRecordingManager>>());
        let tm = Arc::clone(&app.state::<Arc<TranscriptionManager>>());
        let workflow = Arc::clone(&app.state::<Arc<crate::workflow::DesktopWorkflow>>());
        let Some(generation) = workflow.active_generation().ok().flatten() else {
            return;
        };
        let owner = workflow.token(generation);
        let Ok(profile) = workflow.recognition_profile(generation) else {
            return;
        };
        let streaming =
            should_use_streaming_overlay(get_settings(app).overlay_style, tm.is_streaming());
        let Some(cancel_generation) = owner.run(|| {
            crate::physical_shortcuts::processing_started();
            rm.invalidate_recording_readiness();
            rm.remove_mute();
            play_feedback_sound(app, SoundType::Stop);
            rm.cancel_generation()
        }) else {
            return;
        };
        generation_ui(app, owner.clone(), move |app| {
            set_tray_state(app, TrayIconState::Transcribing);
            if streaming {
                app.state::<Arc<TranscriptionManager>>()
                    .emit_stream_working(StreamWorkKind::Transcribing);
            } else {
                show_transcribing_overlay(app);
            }
        });
        let binding_id = binding_id.to_string();
        // Esc remains registered through STT and HTTP, until FinishGuard drops.
        tauri::async_runtime::spawn(async move {
            let cleanup_app = ah.clone();
            let cleanup_tm = Arc::clone(&tm);
            let cleanup_owner = owner.clone();
            let _guard = FinishGuard {
                app: ah.clone(),
                owner: owner.clone(),
                _cleanup: owner.cleanup(move || {
                    shortcut::unregister_cancel_shortcut(&cleanup_app);
                    cleanup_tm.maybe_unload_immediately("transcription session");
                    if let Some(c) = cleanup_app.try_state::<TranscriptionCoordinator>() {
                        c.notify_processing_finished(cleanup_owner);
                    }
                    crate::physical_shortcuts::processing_finished(&cleanup_app);
                    crate::memory::trim_freed_memory();
                }),
            };
            let Some(samples) = rm.stop_recording(&binding_id, cancel_generation, &owner) else {
                owner.run(|| tm.cancel_stream());
                let _ = owner.cancel();
                return;
            };
            if rm.was_cancelled_since(cancel_generation) {
                owner.run(|| tm.cancel_stream());
                let _ = owner.cancel();
                return;
            }
            if samples.is_empty() {
                owner.run(|| {
                    tm.cancel_stream();
                    let _ = ah.emit("session-error", "no-speech");
                });
                let _ = owner.cancel();
                return;
            }
            // PCM belongs only to this attempt. No persistent WAV or audio gate.
            let transcriber = Arc::clone(&tm);
            // Detach this stream under the owner gate; the background wait can
            // never take a newer recording's global stream route.
            let Some(finalization) = owner.run(|| tm.take_stream_finalization()) else {
                return;
            };
            let decoder_owner = owner.clone();
            let transcription =
                tauri::async_runtime::spawn_blocking(move || match finalization.wait() {
                    Ok(Some(text)) if !text.trim().is_empty() => Ok(text),
                    Ok(_) => {
                        transcriber.transcribe_for_generation(samples, &profile, decoder_owner)
                    }
                    Err(error) => Err(error),
                })
                .await;
            if rm.was_cancelled_since(cancel_generation) {
                let _ = owner.cancel();
                return;
            }
            match transcription {
                Ok(Ok(text)) if !text.trim().is_empty() => {
                    generation_ui(&ah, owner.clone(), move |app| {
                        if streaming {
                            app.state::<Arc<TranscriptionManager>>()
                                .emit_stream_working(StreamWorkKind::Polishing);
                        } else {
                            show_processing_overlay(app);
                        }
                    });
                    match crate::commands::history::finish_recognized(&ah, generation, &text).await
                    {
                        Ok(entry) => {
                            owner.run(|| {
                                let _ = ah.emit(
                                    "session-outcome",
                                    serde_json::json!({"status":entry.status,"error":entry.error}),
                                );
                            });
                        }
                        Err(error) if !rm.was_cancelled_since(cancel_generation) => {
                            owner.run(|| {
                                let _ = ah.emit("session-error", error);
                            });
                        }
                        Err(_) => {}
                    }
                }
                Ok(Ok(_)) => {
                    let _ = owner.cancel();
                    owner.run(|| {
                        let _ = ah.emit("session-error", "no-speech");
                    });
                }
                Ok(Err(error)) if error.to_string() == "recognition-model-incompatible" => {
                    let _ = owner.cancel();
                    owner.run(|| {
                        let _ = ah.emit("session-error", "recognition-model-incompatible");
                    });
                }
                _ => {
                    let _ = owner.cancel();
                    owner.run(|| {
                        let _ = ah.emit("session-error", "transcription-failed");
                    });
                }
            }
        });
    }
}

// Cancel Action
struct CancelAction;

impl ShortcutAction for CancelAction {
    fn start(&self, app: &AppHandle, _binding_id: &str, _shortcut_str: &str) {
        utils::cancel_current_operation(app);
    }

    fn stop(&self, _app: &AppHandle, _binding_id: &str, _shortcut_str: &str) {
        // Nothing to do on stop for cancel
    }
}

// Test Action
struct TestAction;

impl ShortcutAction for TestAction {
    fn start(&self, app: &AppHandle, binding_id: &str, shortcut_str: &str) {
        log::info!(
            "Shortcut ID '{}': Started - {} (App: {})", // Changed "Pressed" to "Started" for consistency
            binding_id,
            shortcut_str,
            app.package_info().name
        );
    }

    fn stop(&self, app: &AppHandle, binding_id: &str, shortcut_str: &str) {
        log::info!(
            "Shortcut ID '{}': Stopped - {} (App: {})", // Changed "Released" to "Stopped" for consistency
            binding_id,
            shortcut_str,
            app.package_info().name
        );
    }
}

// Static Action Map
pub static ACTION_MAP: Lazy<HashMap<String, Arc<dyn ShortcutAction>>> = Lazy::new(|| {
    let mut map = HashMap::new();
    map.insert(
        "translation".to_string(),
        Arc::new(TranscribeAction) as Arc<dyn ShortcutAction>,
    );
    map.insert(
        "ask".to_string(),
        Arc::new(TranscribeAction) as Arc<dyn ShortcutAction>,
    );
    map.insert(
        "transcribe".to_string(),
        Arc::new(TranscribeAction) as Arc<dyn ShortcutAction>,
    );
    map.insert(
        "transcribe_with_post_process".to_string(),
        Arc::new(TranscribeAction) as Arc<dyn ShortcutAction>,
    );
    map.insert(
        "cancel".to_string(),
        Arc::new(CancelAction) as Arc<dyn ShortcutAction>,
    );
    map.insert(
        "test".to_string(),
        Arc::new(TestAction) as Arc<dyn ShortcutAction>,
    );
    map
});
