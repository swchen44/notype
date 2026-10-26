use crate::provider_config::{
    credentials::CurrentUserReader, ConnectionDraft, ConnectionKind, ConnectionSettingsView,
    ConnectionStore, KeyAction, ProviderDialect,
};
use crate::provider_models::{ModelDiscovery, ModelListRequest, ModelListResult};
use crate::{
    portable, settings,
    workflow::{DesktopWorkflow, TextConfiguration, TranslationSettings},
};
use notype_core::{RecognitionMode, Scenario, SessionEntry, SessionStatus};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::sync::OnceLock;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_clipboard_manager::ClipboardExt;

#[path = "history_metrics.rs"]
mod history_metrics;
pub use history_metrics::{
    HistoryApiSummaryDto, HistoryCsvExportResult, HistoryCsvExportStatus, HistoryObservationsDto,
};

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
    /// Safe durable recording/model/count/time observations.
    pub observations: HistoryObservationsDto,
    /// All actual API calls, including retry/back-translation and unknown fees.
    pub api_summary: HistoryApiSummaryDto,
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
            observations: entry.observations.into(),
            api_summary: entry.api_summary.into(),
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

static CONFIGURATION_STORE: Mutex<Option<Arc<ConnectionStore>>> = Mutex::new(None);
pub(crate) fn configuration_store(app: &AppHandle) -> Result<Arc<ConnectionStore>, String> {
    let mut current = CONFIGURATION_STORE
        .lock()
        .map_err(|_| "configuration-storage-failed".to_owned())?;
    if let Some(store) = current.as_ref() {
        return Ok(Arc::clone(store));
    }
    let path = portable::app_data_dir(app)
        .map_err(|_| "configuration-storage-failed")?
        .join("provider_connections_v2.json");
    let store = Arc::new(
        ConnectionStore::open(Arc::new(crate::provider_config::files::FileStorage::new(
            path,
        )))
        .map_err(|error| error.to_string())?,
    );
    *current = Some(Arc::clone(&store));
    Ok(store)
}
pub(crate) fn read_configuration(app: &AppHandle) -> Result<TextConfiguration, String> {
    crate::offline_bootstrap::admit()?;
    let configuration = match configuration_store(app) {
        Ok(store) => store.processing_snapshot(&CurrentUserReader),
        Err(_) => crate::provider_config::failed_preparation(crate::provider_config::ConfigError(
            "configuration-storage-failed",
        )),
    };
    Ok(configuration)
}
/// Return only safe fields; no key resolution is performed while listing records.
#[tauri::command]
#[specta::specta]
pub fn get_text_configuration(app: AppHandle) -> Result<ConnectionSettingsView, String> {
    configuration_store(&app)?
        .view()
        .map_err(|error| error.to_string())
}
/// Add a blank-model/OFF/no-key record without activation or network access.
#[tauri::command]
#[specta::specta]
pub fn create_text_connection(
    app: AppHandle,
    expected_revision: u64,
    display_name: String,
    kind: ConnectionKind,
    dialect: ProviderDialect,
) -> Result<ConnectionSettingsView, String> {
    configuration_store(&app)?
        .create(expected_revision, display_name, kind, dialect)
        .map_err(|error| error.to_string())
}
/// Atomically save one connection with explicit bound credential policy.
#[tauri::command]
#[specta::specta]
pub fn save_text_connection(
    app: AppHandle,
    expected_revision: u64,
    connection: ConnectionDraft,
    key_action: KeyAction,
) -> Result<ConnectionSettingsView, String> {
    configuration_store(&app)?
        .save(expected_revision, connection, key_action)
        .map_err(|error| error.to_string())
}
/// Persist deliberate active selection; selecting an editor row never calls this.
#[tauri::command]
#[specta::specta]
pub fn activate_text_connection(
    app: AppHandle,
    expected_revision: u64,
    id: Option<String>,
) -> Result<ConnectionSettingsView, String> {
    configuration_store(&app)?
        .activate(expected_revision, id)
        .map_err(|error| error.to_string())
}
/// Remove one record and leave active unset when its ID was selected.
#[tauri::command]
#[specta::specta]
pub fn remove_text_connection(
    app: AppHandle,
    expected_revision: u64,
    id: String,
) -> Result<ConnectionSettingsView, String> {
    configuration_store(&app)?
        .remove(expected_revision, &id)
        .map_err(|error| error.to_string())
}
fn model_discovery() -> &'static ModelDiscovery {
    static SERVICE: OnceLock<ModelDiscovery> = OnceLock::new();
    SERVICE.get_or_init(ModelDiscovery::new)
}
/// Explicit reference GET only; no automatic field change, save or activation.
#[tauri::command]
#[specta::specta]
pub async fn list_text_models(
    app: AppHandle,
    request: ModelListRequest,
) -> Result<ModelListResult, String> {
    crate::offline_bootstrap::admit()?;
    model_discovery()
        .query(configuration_store(&app)?, request, &CurrentUserReader)
        .await
        .map_err(|error| error.to_string())
}
/// Cancel an explicit model-list request; late replies are also editor-fenced.
#[tauri::command]
#[specta::specta]
pub fn cancel_text_models(request_id: String) -> Result<(), String> {
    model_discovery().cancel(&request_id);
    Ok(())
}

/// Explicit all-history save. Native selection/space probing occur outside the
/// store writer lock; only the completed snapshot file replaces a destination.
#[tauri::command]
#[specta::specta]
pub async fn export_history_csv(
    app: AppHandle,
    workflow: State<'_, Arc<DesktopWorkflow>>,
) -> Result<HistoryCsvExportResult, String> {
    use tauri_plugin_dialog::DialogExt;
    let selected = app
        .dialog()
        .file()
        .set_file_name("notype-history.csv")
        .add_filter("CSV", &["csv"])
        .blocking_save_file();
    let Some(selected) = selected else {
        return Ok(HistoryCsvExportResult {
            status: HistoryCsvExportStatus::Cancelled,
            rows: None,
        });
    };
    let destination = selected
        .into_path()
        .map_err(|_| "history-csv-invalid-destination".to_owned())?;
    let sessions = workflow.sessions().clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        sessions.export_csv(
            &destination,
            &NativeCsvSpace,
            &notype_core::CancellationToken::new(),
        )
    })
    .await
    .map_err(|_| "history-csv-write-failed".to_owned())?;
    match result {
        Ok(exported) => Ok(HistoryCsvExportResult {
            status: HistoryCsvExportStatus::Saved,
            rows: Some(exported.rows),
        }),
        Err(notype_core::csv::CsvExportError::Cancelled) => Ok(HistoryCsvExportResult {
            status: HistoryCsvExportStatus::Cancelled,
            rows: None,
        }),
        Err(error) => Err(error.to_string()),
    }
}

struct NativeCsvSpace;
impl notype_core::csv::CsvSpaceProbe for NativeCsvSpace {
    fn available_bytes(
        &self,
        directory: &std::path::Path,
    ) -> Result<u64, notype_core::csv::CsvExportError> {
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            #[link(name = "kernel32")]
            unsafe extern "system" {
                fn GetDiskFreeSpaceExW(
                    directory: *const u16,
                    available: *mut u64,
                    total: *mut u64,
                    free: *mut u64,
                ) -> i32;
            }
            let wide: Vec<u16> = directory.as_os_str().encode_wide().chain(Some(0)).collect();
            let mut available = 0;
            // Existing chosen directory; only this process's available quota is read.
            if unsafe {
                GetDiskFreeSpaceExW(
                    wide.as_ptr(),
                    &mut available,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            } != 0
            {
                return Ok(available);
            }
        }
        #[cfg(not(windows))]
        let _ = directory;
        Err(notype_core::csv::CsvExportError::SpaceUnavailable)
    }
}
/// Save exactly one prompt under CAS and return the same safe collection view.
#[tauri::command]
#[specta::specta]
pub fn save_scenario_prompt(
    app: AppHandle,
    scenario: SceneId,
    prompt: String,
    expected_revision: u64,
) -> Result<ConnectionSettingsView, String> {
    configuration_store(&app)?
        .prompt(expected_revision, scenario.into(), Some(prompt))
        .map_err(|error| error.to_string())
}
/// Reset exactly one prompt, preserving current connection records and globals.
#[tauri::command]
#[specta::specta]
pub fn reset_scenario_prompt(
    app: AppHandle,
    scenario: SceneId,
    expected_revision: u64,
) -> Result<ConnectionSettingsView, String> {
    configuration_store(&app)?
        .prompt(expected_revision, scenario.into(), None)
        .map_err(|error| error.to_string())
}
/// Save translation settings independently of connection edits; never return keys.
#[tauri::command]
#[specta::specta]
pub fn save_translation_settings(
    app: AppHandle,
    translation: TranslationSettings,
    expected_revision: u64,
) -> Result<ConnectionSettingsView, String> {
    configuration_store(&app)?
        .translation(expected_revision, translation)
        .map_err(|error| error.to_string())
}

#[tauri::command]
#[specta::specta]
pub fn get_workflow_readiness(
    app: AppHandle,
    notice: State<'_, LegacyImportNotice>,
) -> WorkflowReadiness {
    if crate::offline_bootstrap::blocked() {
        return WorkflowReadiness {
            text_configured: false,
            delivery_available: false,
            legacy_import_error: notice.0.clone(),
        };
    }
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
    crate::offline_bootstrap::admit()?;
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
        .capture_requested_model(generation, preferences.selected_model.clone())
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
    crate::offline_bootstrap::admit()?;
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
    fixture_owner: Option<crate::native_input::DebugFixtureOwner>,
    require_input_target: Option<bool>,
) -> Result<u64, String> {
    crate::offline_bootstrap::admit()?;
    let capture = if capture_input == Some(true) {
        app.state::<crate::native_input::NativeInput>()
            .capture_owned_fixture(
                fixture_owner.ok_or("owned-fixture-identity-required")?,
                require_input_target.unwrap_or(true),
            )?
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
