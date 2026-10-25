//! Production support entry points: preview locally, then explicitly save.
//! No command accepts raw log/report text and no command uploads anything.

use crate::diagnostics::{self, LogFileStatus, Platform, Report, ReportContext};
use crate::managers::model::ModelManager;
use crate::settings::{get_settings, LogLevel};
use serde::Serialize;
use specta::Type;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

static PREVIEW: Mutex<Option<Report>> = Mutex::new(None);

/// Public IPC preview of one owned, allowlisted report. The adapter caches this
/// exact snapshot until another preview replaces it or the app exits. Export
/// accepts only its matching ID, never replacement text supplied by the UI.
#[derive(Serialize, Type)]
pub struct DiagnosticPreview {
    /// Opaque snapshot ID for export matching and the safe GitHub summary.
    pub report_id: String,
    /// Exact bounded export text; excludes user content, secrets and paths.
    pub text: String,
}

/// Builds an allowlisted report without opening any user data or raw log file.
#[tauri::command]
#[specta::specta]
pub fn get_diagnostic_report(
    app: AppHandle,
    model_manager: State<'_, Arc<ModelManager>>,
) -> Result<DiagnosticPreview, String> {
    let settings = get_settings(&app);
    let model = model_manager.get_model_info(&settings.selected_model);
    let local_log = crate::portable::app_log_dir(&app)
        .ok()
        .map(|directory| std::fs::metadata(directory.join("notype.log")))
        .map(|metadata| match metadata {
            Ok(metadata) if metadata.len() == 0 => LogFileStatus::Empty,
            Ok(_) => LogFileStatus::Present,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => LogFileStatus::Missing,
            Err(_) => LogFileStatus::Unavailable,
        })
        .unwrap_or(LogFileStatus::Unavailable);
    let platform = match std::env::consts::OS {
        "windows" => Platform::Windows,
        "macos" => Platform::Macos,
        "linux" => Platform::Linux,
        _ => Platform::Other,
    };
    let log_level = match settings.log_level {
        LogLevel::Error => diagnostics::LogLevel::Error,
        LogLevel::Warn => diagnostics::LogLevel::Warn,
        LogLevel::Info => diagnostics::LogLevel::Info,
        LogLevel::Debug => diagnostics::LogLevel::Debug,
        LogLevel::Trace => diagnostics::LogLevel::Trace,
    };
    let report = diagnostics::report(ReportContext {
        version: &app.package_info().version.to_string(),
        platform,
        log_level,
        debug_mode: settings.debug_mode,
        always_on_microphone: settings.always_on_microphone,
        model_selected: !settings.selected_model.is_empty(),
        model_available: model.as_ref().is_some_and(|model| model.is_downloaded),
        model_is_custom: model.as_ref().is_some_and(|model| model.is_custom),
        local_log,
    });
    let preview = DiagnosticPreview {
        report_id: report.id().to_string(),
        text: report.text().to_string(),
    };
    *PREVIEW
        .lock()
        .map_err(|_| "diagnostic-preview-failed".to_string())? = Some(report);
    Ok(preview)
}

/// The native save dialog is the only destination input. Returns false when
/// the user cancels; only the exact previously previewed snapshot is saved.
#[tauri::command]
#[specta::specta]
pub async fn export_diagnostic_report(app: AppHandle, report_id: String) -> Result<bool, String> {
    let report = PREVIEW
        .lock()
        .map_err(|_| "diagnostic-preview-failed".to_string())?
        .as_ref()
        .filter(|report| report.id() == report_id)
        .cloned()
        .ok_or_else(|| "diagnostic-preview-expired".to_string())?;
    let selected = app
        .dialog()
        .file()
        .set_file_name(format!("notype-diagnostics-{}.txt", report.id()))
        .add_filter("Text", &["txt"])
        .blocking_save_file();
    let Some(selected) = selected else {
        return Ok(false);
    };
    let path = selected
        .into_path()
        .map_err(|_| "diagnostic-save-failed".to_string())?;
    report.export_to(&path).map_err(str::to_string)?;
    Ok(true)
}
