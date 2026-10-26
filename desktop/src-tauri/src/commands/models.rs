use crate::managers::model::{ModelInfo, ModelManager};
use crate::managers::model_root_dto::{
    ModelCatalogView, ModelRootCandidate, ModelRootScanProgress, ModelRootView,
};
use crate::managers::model_roots::{RootActivity, RootError};
use crate::managers::model_storage::ModelPathAccess;
use crate::managers::transcription::{ModelStateEvent, TranscriptionManager};
use crate::settings::{get_settings, write_settings, ModelUnloadTimeout};
use log::error;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_opener::OpenerExt;

fn safe_error(error: anyhow::Error) -> String {
    error
        .downcast_ref::<RootError>()
        .map(|e| e.0)
        .unwrap_or("model-operation-failed")
        .to_owned()
}
fn expected(activity: &RootActivity, generation: Option<u64>) -> Result<(), String> {
    if generation.is_some_and(|generation| generation != activity.root.generation) {
        Err("model-root-stale".into())
    } else {
        Ok(())
    }
}
#[tauri::command]
#[specta::specta]
pub fn show_model_root(
    app: AppHandle,
    model_manager: State<'_, Arc<ModelManager>>,
    expected_generation: u64,
) -> Result<(), String> {
    let activity = model_manager.activity().map_err(safe_error)?;
    expected(&activity, Some(expected_generation))?;
    let path = model_manager
        .roots
        .location(&activity)
        .map_err(|e| e.0.to_owned())?;
    app.opener()
        .open_path(
            crate::managers::model_roots_files::display_path(&path),
            None::<&str>,
        )
        .map_err(|_| "model-root-unavailable".to_owned())
}

#[tauri::command]
#[specta::specta]
pub fn get_model_root(
    model_manager: State<'_, Arc<ModelManager>>,
) -> Result<ModelRootView, String> {
    Ok(model_manager.root_view())
}
#[tauri::command]
#[specta::specta]
pub fn get_model_catalog(
    model_manager: State<'_, Arc<ModelManager>>,
) -> Result<ModelCatalogView, String> {
    let _activity = model_manager.activity().map_err(safe_error)?;
    Ok(model_manager.catalog_view())
}
#[tauri::command]
#[specta::specta]
pub async fn validate_model_root(
    app: AppHandle,
    model_manager: State<'_, Arc<ModelManager>>,
    folder: String,
    request_id: String,
) -> Result<ModelRootCandidate, String> {
    let manager = model_manager.inner().clone();
    let work = manager
        .roots
        .prepare_validation(std::path::Path::new(&folder), &request_id)
        .map_err(|e| e.0.to_owned())?;
    let generation = work.generation();
    tokio::task::spawn_blocking(move || {
        manager
            .roots
            .run_validation(work, &|checked, total| {
                let _ = app.emit(
                    "model-root-scan-progress",
                    ModelRootScanProgress {
                        generation,
                        request_id: request_id.clone(),
                        checked,
                        total,
                    },
                );
            })
            .map(|candidate| manager.candidate_view(&candidate))
            .map_err(|error| error.0.to_owned())
    })
    .await
    .map_err(|_| "model-operation-failed".to_owned())?
}
#[tauri::command]
#[specta::specta]
pub fn apply_model_root(
    app: AppHandle,
    model_manager: State<'_, Arc<ModelManager>>,
    candidate_id: String,
    revision: u64,
) -> Result<ModelRootView, String> {
    model_manager
        .roots
        .apply(&candidate_id, revision, || {
            if let Some(transcription_manager) = app.try_state::<Arc<TranscriptionManager>>() {
                transcription_manager.invalidate_idle_root()
            }
        })
        .map_err(|e| e.0.to_owned())?;
    let view = model_manager.root_view();
    let _ = app.emit(
        "model-root-changed",
        serde_json::json!({"generation":view.generation}),
    );
    Ok(view)
}
#[tauri::command]
#[specta::specta]
pub fn cancel_model_root_scan(
    model_manager: State<'_, Arc<ModelManager>>,
    request_id: String,
) -> Result<(), String> {
    model_manager.roots.cancel_scan(&request_id);
    Ok(())
}

/// Displays a completed model's actual resolved file/directory in the UI.
/// This read-only lookup never exports a path in diagnostics or returns a
/// guessed destination for a model that has not completed download.
#[tauri::command]
#[specta::specta]
pub fn get_model_path(
    model_manager: State<'_, Arc<ModelManager>>,
    model_id: String,
) -> Result<String, String> {
    model_manager
        .get_model_path_for_display(&model_id)
        .map(|path| crate::managers::model_roots_files::display_path(&path))
        .map_err(|_| "model-path-unavailable".to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn get_available_models(
    model_manager: State<'_, Arc<ModelManager>>,
) -> Result<Vec<ModelInfo>, String> {
    Ok(model_manager.get_available_models())
}

#[tauri::command]
#[specta::specta]
pub async fn get_model_info(
    model_manager: State<'_, Arc<ModelManager>>,
    model_id: String,
) -> Result<Option<ModelInfo>, String> {
    Ok(model_manager.get_model_info(&model_id))
}

/// Re-scan local sources (custom models dir + shared HF cache) for models added
/// since launch
#[tauri::command]
#[specta::specta]
pub async fn rescan_local_models(
    model_manager: State<'_, Arc<ModelManager>>,
) -> Result<ModelRootView, String> {
    let mm = model_manager.inner().clone();
    let work = mm
        .roots
        .prepare_rescan("rescan")
        .map_err(|e| e.0.to_owned())?;
    tokio::task::spawn_blocking(move || {
        mm.roots
            .run_rescan(work, &|_, _| {})
            .map_err(|e| e.0.to_owned())?;
        mm.notify_models_updated();
        Ok(mm.root_view())
    })
    .await
    .map_err(|_| "model-operation-failed".to_owned())?
}

#[tauri::command]
#[specta::specta]
pub async fn download_model(
    app_handle: AppHandle,
    model_manager: State<'_, Arc<ModelManager>>,
    model_id: String,
    expected_generation: Option<u64>,
) -> Result<(), String> {
    let activity = model_manager.activity().map_err(safe_error)?;
    expected(&activity, expected_generation)?;
    let result = model_manager
        .download_model(&model_id)
        .await
        .map_err(safe_error);

    if let Err(ref error) = result {
        // Log as well as emit: the toast is transient, and failed downloads have
        // historically been undiagnosable because logs showed nothing (#1579).
        error!("Model download failed: {}", error);
        let _ = app_handle.emit(
            "model-download-failed",
            serde_json::json!({ "generation": activity.root.generation, "model_id": &model_id, "error": error }),
        );
    }

    result
}

#[tauri::command]
#[specta::specta]
pub async fn delete_model(
    _app_handle: AppHandle,
    model_manager: State<'_, Arc<ModelManager>>,
    transcription_manager: State<'_, Arc<TranscriptionManager>>,
    model_id: String,
    expected_generation: Option<u64>,
) -> Result<(), String> {
    let activity = model_manager.activity().map_err(safe_error)?;
    expected(&activity, expected_generation)?;
    let record = model_manager
        .roots
        .artifacts()
        .remove(&model_id)
        .ok_or("model-artifact-unavailable")?;
    if !record.owned || record.legacy || record.custom {
        return Err("model-root-readonly".into());
    }
    if transcription_manager.get_current_model().as_deref() == Some(model_id.as_str()) {
        transcription_manager.unload_model().map_err(safe_error)?;
    }
    model_manager.delete_model(&model_id).map_err(safe_error)
}

/// Shared logic for switching the active model, used by both the Tauri command
/// and the tray menu handler.
///
/// Validates the model, updates the persisted setting, and loads the model
/// unless the unload timeout is set to "Immediately" (in which case the model
/// will be loaded on-demand during the next transcription).
pub fn switch_active_model(app: &AppHandle, model_id: &str) -> Result<(), String> {
    switch_active_model_for_generation(app, model_id, None)
}
fn switch_active_model_for_generation(
    app: &AppHandle,
    model_id: &str,
    generation: Option<u64>,
) -> Result<(), String> {
    crate::offline_bootstrap::admit()?;
    let model_manager = app.state::<Arc<ModelManager>>();
    let transcription_manager = app.state::<Arc<TranscriptionManager>>();
    let activity = model_manager.activity().map_err(safe_error)?;
    expected(&activity, generation)?;

    // Atomically claim the loading slot — prevents concurrent model loads
    // from tray double-clicks or overlapping commands. The guard resets the
    // flag on drop (including early returns, errors, and panics).
    let _loading_guard = transcription_manager
        .try_start_loading()
        .ok_or_else(|| "Model load already in progress".to_string())?;

    // Check if model exists and is available
    let model_info = model_manager
        .get_model_info(model_id)
        .ok_or_else(|| "model-artifact-unavailable".to_owned())?;

    if !model_info.is_downloaded {
        return Err("model-artifact-unavailable".into());
    }

    let settings = get_settings(app);
    let unload_timeout = settings.model_unload_timeout;
    let old_model = settings.selected_model.clone();
    let old_onboarding_completed = settings.onboarding_completed;

    // Persist the new selection early so the frontend sees the correct model
    // when it reacts to events emitted by load_model.
    let mut settings = settings;
    settings.selected_model = model_id.to_string();
    settings.onboarding_completed = true;

    write_settings(app, settings);

    // Skip eager loading if unload is set to "Immediately" — the model
    // will be loaded on-demand during the next transcription.
    if unload_timeout == ModelUnloadTimeout::Immediately {
        // Notify frontend — load_model won't be called so no events
        // would otherwise be emitted.
        let _ = app.emit(
            "model-state-changed",
            ModelStateEvent {
                generation: activity.root.generation,
                event_type: "selection_changed".to_string(),
                model_id: Some(model_id.to_string()),
                model_name: Some(model_info.name.clone()),
                error: None,
            },
        );
        log::info!(
            "Model selection changed to {} (not loading — unload set to Immediately).",
            model_id
        );
        return Ok(());
    }

    // Load the model. On failure, revert the persisted selection.
    if let Err(e) = transcription_manager.load_model(model_id) {
        let mut settings = get_settings(app);
        settings.selected_model = old_model;
        settings.onboarding_completed = old_onboarding_completed;
        write_settings(app, settings);
        return Err(safe_error(e));
    }

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn set_active_model(
    app_handle: AppHandle,
    _model_manager: State<'_, Arc<ModelManager>>,
    _transcription_manager: State<'_, Arc<TranscriptionManager>>,
    model_id: String,
    expected_generation: Option<u64>,
) -> Result<(), String> {
    switch_active_model_for_generation(&app_handle, &model_id, expected_generation)
}

#[tauri::command]
#[specta::specta]
pub async fn get_current_model(app_handle: AppHandle) -> Result<String, String> {
    let settings = get_settings(&app_handle);
    Ok(settings.selected_model)
}

#[tauri::command]
#[specta::specta]
pub async fn get_transcription_model_status(
    transcription_manager: State<'_, Arc<TranscriptionManager>>,
) -> Result<Option<String>, String> {
    Ok(transcription_manager.get_current_model())
}

#[tauri::command]
#[specta::specta]
pub async fn is_model_loading(
    transcription_manager: State<'_, Arc<TranscriptionManager>>,
) -> Result<bool, String> {
    // Check if transcription manager has a loaded model
    let current_model = transcription_manager.get_current_model();
    Ok(current_model.is_none())
}

#[tauri::command]
#[specta::specta]
pub async fn cancel_download(
    model_manager: State<'_, Arc<ModelManager>>,
    model_id: String,
    expected_generation: Option<u64>,
) -> Result<(), String> {
    let activity = model_manager.activity().map_err(safe_error)?;
    expected(&activity, expected_generation)?;
    model_manager.cancel_download(&model_id).map_err(safe_error)
}
