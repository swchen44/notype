use crate::{
    portable, settings,
    workflow::{DesktopWorkflow, TextConfiguration},
};
use notype_core::{RecognitionMode, SessionEntry, SessionStatus};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_store::StoreExt;

#[derive(Serialize, Type)]
#[serde(rename_all = "kebab-case")]
pub enum SessionPhase {
    Recognized,
    Processing,
    Pending,
    Failed,
    Cancelled,
    Delivering,
    Delivered,
    DeliveryUnknown,
}
#[derive(Serialize, Type)]
pub struct SessionDto {
    pub id: String,
    pub transcript: String,
    pub result: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub status: SessionPhase,
    pub error: Option<String>,
    pub recognition_mode: Option<String>,
    pub selection: String,
}
impl From<SessionEntry> for SessionDto {
    fn from(entry: SessionEntry) -> Self {
        Self {
            id: entry.id,
            transcript: entry.transcript,
            result: entry.result,
            created_at: entry.created_at,
            updated_at: entry.updated_at,
            status: match entry.status {
                SessionStatus::Recognized => SessionPhase::Recognized,
                SessionStatus::Processing => SessionPhase::Processing,
                SessionStatus::Pending => SessionPhase::Pending,
                SessionStatus::Failed => SessionPhase::Failed,
                SessionStatus::Cancelled => SessionPhase::Cancelled,
                SessionStatus::Delivering => SessionPhase::Delivering,
                SessionStatus::Delivered => SessionPhase::Delivered,
                SessionStatus::DeliveryUnknown => SessionPhase::DeliveryUnknown,
            },
            error: entry.error,
            recognition_mode: entry.recognition_mode.map(|mode| {
                match mode {
                    RecognitionMode::TraditionalEnglish => "traditional_english",
                    RecognitionMode::SimplifiedEnglish => "simplified_english",
                }
                .into()
            }),
            selection: match entry.selection {
                notype_core::TextSelection::Original => "original",
                notype_core::TextSelection::Polished => "polished",
            }
            .into(),
        }
    }
}

pub struct LegacyImportNotice(pub Option<String>);
#[derive(Serialize, Type)]
pub struct WorkflowReadiness {
    pub text_configured: bool,
    pub delivery_available: bool,
    pub legacy_import_error: Option<String>,
}

pub(crate) fn read_configuration(app: &AppHandle) -> Result<TextConfiguration, String> {
    let path = portable::app_data_dir(app)
        .map_err(|_| "configuration-storage-failed")?
        .join("text_processing_store.json");
    let store = app
        .store(path)
        .map_err(|_| "configuration-storage-failed")?;
    store
        .get("configuration")
        .map(serde_json::from_value)
        .transpose()
        .map_err(|_| "configuration-required".to_string())
        .map(|configuration| configuration.unwrap_or_default())
}

#[tauri::command]
#[specta::specta]
pub fn get_text_configuration(app: AppHandle) -> Result<TextConfiguration, String> {
    read_configuration(&app)
}

#[tauri::command]
#[specta::specta]
pub fn set_text_configuration(
    app: AppHandle,
    configuration: TextConfiguration,
) -> Result<(), String> {
    let path = portable::app_data_dir(&app)
        .map_err(|_| "configuration-storage-failed")?
        .join("text_processing_store.json");
    let store = app
        .store(path)
        .map_err(|_| "configuration-storage-failed")?;
    store.set(
        "configuration",
        serde_json::to_value(configuration).map_err(|_| "configuration-required")?,
    );
    store
        .save()
        .map_err(|_| "configuration-storage-failed".to_string())
}

#[tauri::command]
#[specta::specta]
pub fn get_workflow_readiness(
    app: AppHandle,
    notice: State<'_, LegacyImportNotice>,
) -> WorkflowReadiness {
    WorkflowReadiness {
        text_configured: read_configuration(&app)
            .is_ok_and(|configuration| configuration.is_ready()),
        delivery_available: false,
        legacy_import_error: notice.0.clone(),
    }
}

pub(crate) fn begin_recording(app: &AppHandle) -> Result<u64, String> {
    let preferences = settings::get_settings(app);
    let profile = crate::workflow::recognition::RecognitionProfile::new(
        preferences.recognition_mode.into(),
        preferences.custom_words,
    );
    if let Some(model) = app
        .state::<Arc<crate::managers::model::ModelManager>>()
        .get_model_info(&preferences.selected_model)
    {
        // Custom model capabilities can remain unknown until loading; the
        // engine validates its actual capabilities before decoding as well.
        if !model.supported_languages.is_empty() {
            profile
                .validate_model(&model.supported_languages)
                .map_err(str::to_string)?;
        }
    }
    app.state::<Arc<DesktopWorkflow>>()
        .begin_with_profile(profile, None)
        .map_err(|error| error.to_string())
}

pub(crate) async fn finish_recognized(
    app: &AppHandle,
    generation: u64,
    transcript: &str,
) -> Result<SessionEntry, String> {
    let workflow = Arc::clone(&app.state::<Arc<DesktopWorkflow>>());
    let configuration = read_configuration(app).unwrap_or_default();
    let glossary = settings::get_settings(app).custom_words;
    workflow
        .finish(generation, transcript, configuration, glossary)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
#[specta::specta]
pub fn get_history_entries(
    workflow: State<'_, Arc<DesktopWorkflow>>,
    query: Option<String>,
) -> Result<Vec<SessionDto>, String> {
    workflow
        .sessions()
        .search(query.as_deref().unwrap_or(""))
        .map(|entries| entries.into_iter().map(Into::into).collect())
        .map_err(|error| error.to_string())
}

#[tauri::command]
#[specta::specta]
pub fn delete_history_entry(
    workflow: State<'_, Arc<DesktopWorkflow>>,
    id: String,
) -> Result<(), String> {
    workflow.delete(&id).map_err(|error| error.to_string())
}

#[tauri::command]
#[specta::specta]
pub fn cancel_history_entry(
    workflow: State<'_, Arc<DesktopWorkflow>>,
    id: String,
) -> Result<SessionDto, String> {
    workflow
        .cancel(&id)
        .map(Into::into)
        .map_err(|error| error.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn retry_history_entry_processing(
    app: AppHandle,
    workflow: State<'_, Arc<DesktopWorkflow>>,
    id: String,
) -> Result<SessionDto, String> {
    let configuration = read_configuration(&app).unwrap_or_default();
    let glossary = settings::get_settings(&app).custom_words;
    workflow
        .retry(&id, configuration, glossary)
        .await
        .map(Into::into)
        .map_err(|error| error.to_string())
}

#[tauri::command]
#[specta::specta]
pub fn choose_history_original(
    workflow: State<'_, Arc<DesktopWorkflow>>,
    id: String,
) -> Result<SessionDto, String> {
    workflow
        .choose_original(&id)
        .map(Into::into)
        .map_err(|error| error.to_string())
}

#[tauri::command]
#[specta::specta]
pub async fn deliver_history_entry(
    workflow: State<'_, Arc<DesktopWorkflow>>,
    id: String,
) -> Result<SessionDto, String> {
    workflow
        .deliver(&id)
        .await
        .map(Into::into)
        .map_err(|error| error.to_string())
}

#[derive(Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum HistoryTextVersion {
    Original,
    Result,
}
#[tauri::command]
#[specta::specta]
pub fn copy_history_text(
    app: AppHandle,
    workflow: State<'_, Arc<DesktopWorkflow>>,
    id: String,
    version: HistoryTextVersion,
) -> Result<(), String> {
    let entry = workflow
        .sessions()
        .get(&id)
        .map_err(|error| error.to_string())?;
    let text = match version {
        HistoryTextVersion::Original => entry.transcript,
        HistoryTextVersion::Result => entry.result.ok_or("result-unavailable")?,
    };
    app.clipboard()
        .write_text(text)
        .map_err(|_| "clipboard-unavailable".to_string())
}

// Compatibility commands cannot enable automatic text deletion.
#[tauri::command]
#[specta::specta]
pub fn update_history_limit(_limit: usize) -> Result<(), String> {
    Err("manual-history-retention".into())
}
#[tauri::command]
#[specta::specta]
pub fn update_recording_retention_period(_period: String) -> Result<(), String> {
    Err("session-audio-only".into())
}

#[cfg(debug_assertions)]
#[tauri::command]
pub fn debug_begin_session(
    app: AppHandle,
    workflow: State<'_, Arc<DesktopWorkflow>>,
    recognition_mode: Option<RecognitionMode>,
) -> Result<u64, String> {
    match recognition_mode {
        Some(mode) => workflow
            .begin_with_profile(
                crate::workflow::recognition::RecognitionProfile::new(
                    mode,
                    settings::get_settings(&app).custom_words,
                ),
                None,
            )
            .map_err(|error| error.to_string()),
        None => begin_recording(&app),
    }
}
#[cfg(debug_assertions)]
#[tauri::command]
pub async fn debug_finish_session(
    app: AppHandle,
    generation: u64,
    transcript: String,
) -> Result<SessionDto, String> {
    finish_recognized(&app, generation, &transcript)
        .await
        .map(Into::into)
}
