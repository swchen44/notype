use crate::{
    portable, settings,
    workflow::{DesktopWorkflow, TextConfiguration, TranslationSettings},
};
use notype_core::{RecognitionMode, Scenario, SessionEntry, SessionStatus};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::{Arc, Mutex};
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
    pub scenario: SceneId,
    pub translation_target: Option<String>,
    pub source_language: Option<String>,
    pub back_translation: Option<String>,
    pub review_required: bool,
    pub review_confirmed: bool,
    pub revision: i64,
}
#[derive(Clone, Copy, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum SceneId {
    #[default]
    Dictation,
    Translation,
    Ask,
}
impl From<SceneId> for Scenario {
    fn from(value: SceneId) -> Self {
        match value {
            SceneId::Dictation => Self::Dictation,
            SceneId::Translation => Self::Translation,
            SceneId::Ask => Self::Ask,
        }
    }
}
impl From<Scenario> for SceneId {
    fn from(value: Scenario) -> Self {
        match value {
            Scenario::Dictation => Self::Dictation,
            Scenario::Translation => Self::Translation,
            Scenario::Ask => Self::Ask,
        }
    }
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
            scenario: entry.scenario.into(),
            translation_target: entry.translation_target,
            source_language: entry.source_language,
            back_translation: entry.back_translation,
            review_required: entry.review_required,
            review_confirmed: entry.review_confirmed,
            revision: entry.revision,
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
    mut configuration: TextConfiguration,
) -> Result<(), String> {
    let _lock = CONFIGURATION_WRITE
        .lock()
        .map_err(|_| "configuration-storage-failed")?;
    // The API form saves only its fields. A stale form cannot overwrite profiles.
    let current = read_configuration(&app)?;
    configuration.prompts = current.prompts;
    configuration.translation = current.translation;
    persist_configuration(&app, configuration)
}
static CONFIGURATION_WRITE: Mutex<()> = Mutex::new(());
fn persist_configuration(app: &AppHandle, configuration: TextConfiguration) -> Result<(), String> {
    let path = portable::app_data_dir(app)
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
pub fn save_scenario_prompt(
    app: AppHandle,
    scenario: SceneId,
    prompt: String,
) -> Result<TextConfiguration, String> {
    let _lock = CONFIGURATION_WRITE
        .lock()
        .map_err(|_| "configuration-storage-failed")?;
    let mut configuration = read_configuration(&app)?;
    configuration.set_prompt(scenario.into(), prompt)?;
    persist_configuration(&app, configuration.clone())?;
    Ok(configuration)
}
#[tauri::command]
#[specta::specta]
pub fn reset_scenario_prompt(
    app: AppHandle,
    scenario: SceneId,
) -> Result<TextConfiguration, String> {
    let _lock = CONFIGURATION_WRITE
        .lock()
        .map_err(|_| "configuration-storage-failed")?;
    let mut configuration = read_configuration(&app)?;
    configuration.reset_prompt(scenario.into());
    persist_configuration(&app, configuration.clone())?;
    Ok(configuration)
}
#[tauri::command]
#[specta::specta]
pub fn save_translation_settings(
    app: AppHandle,
    translation: TranslationSettings,
) -> Result<TextConfiguration, String> {
    if translation.target_language.trim().is_empty()
        || translation.target_language.len() > 100
        || translation.domain.len() > 2000
        || translation.tone.len() > 2000
        || translation.glossary.len() > 1000
        || translation.glossary.iter().any(|term| {
            term.source.trim().is_empty()
                || term.target.trim().is_empty()
                || term.target_language.trim().is_empty()
                || term.source.len() > 1000
                || term.target.len() > 1000
                || term.target_language.len() > 100
        })
    {
        return Err("translation-settings-invalid".into());
    }
    let _lock = CONFIGURATION_WRITE
        .lock()
        .map_err(|_| "configuration-storage-failed")?;
    let mut configuration = read_configuration(&app)?;
    configuration.translation = translation;
    persist_configuration(&app, configuration.clone())?;
    Ok(configuration)
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
        delivery_available: app.state::<crate::native_input::NativeInput>().available(),
        legacy_import_error: notice.0.clone(),
    }
}

pub(crate) fn begin_captured_scenario(app: &AppHandle, scenario: Scenario) -> Result<u64, String> {
    let capture = app.state::<crate::native_input::NativeInput>().capture();
    begin_scenario_recording(
        app,
        scenario,
        capture.target,
        notype_core::SourceContext {
            application: capture.application,
        },
    )
}
pub(crate) fn begin_scenario_recording(
    app: &AppHandle,
    scenario: Scenario,
    target: Option<String>,
    context: notype_core::SourceContext,
) -> Result<u64, String> {
    let preferences = settings::get_settings(app);
    let profile = crate::workflow::recognition::RecognitionProfile::new(
        preferences.recognition_mode.into(),
        preferences.custom_words.clone(),
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
    let workflow = app.state::<Arc<DesktopWorkflow>>();
    let generation = workflow
        .begin_with_profile(profile, target)
        .map_err(|error| error.to_string())?;
    workflow
        .capture_source_context(generation, context)
        .map_err(|error| error.to_string())?;
    workflow
        .capture_processing_snapshot(
            generation,
            scenario,
            read_configuration(app).unwrap_or_default(),
            preferences.custom_words,
        )
        .map_err(|error| error.to_string())?;
    Ok(generation)
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
#[tauri::command]
#[specta::specta]
pub fn confirm_history_review(
    workflow: State<'_, Arc<DesktopWorkflow>>,
    id: String,
    revision: i64,
) -> Result<SessionDto, String> {
    workflow
        .confirm_review(&id, revision)
        .map(Into::into)
        .map_err(|error| error.to_string())
}

#[derive(Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum HistoryTextVersion {
    Original,
    Result,
    BackTranslation,
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
        HistoryTextVersion::BackTranslation => {
            entry.back_translation.ok_or("result-unavailable")?
        }
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
    scenario: Option<SceneId>,
    capture_input: Option<bool>,
) -> Result<u64, String> {
    let capture = if capture_input == Some(true) {
        app.state::<crate::native_input::NativeInput>().capture()
    } else {
        Default::default()
    };
    let generation = workflow
        .begin_with_profile(
            crate::workflow::recognition::RecognitionProfile::new(
                recognition_mode
                    .unwrap_or_else(|| settings::get_settings(&app).recognition_mode.into()),
                settings::get_settings(&app).custom_words,
            ),
            capture.target,
        )
        .map_err(|error| error.to_string())?;
    workflow
        .capture_source_context(
            generation,
            notype_core::SourceContext {
                application: capture.application,
            },
        )
        .map_err(|error| error.to_string())?;
    workflow
        .capture_processing_snapshot(
            generation,
            scenario.unwrap_or_default().into(),
            read_configuration(&app).unwrap_or_default(),
            settings::get_settings(&app).custom_words,
        )
        .map_err(|error| error.to_string())?;
    Ok(generation)
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
