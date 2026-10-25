//! Production structured-diagnostics IPC. Native adapters gather only fixed
//! metadata and own the dialog; the shared service owns all state and commits.

use crate::diagnostics::{
    self,
    service::{self, Status},
    LogFileStatus, Platform, ReportContext,
};
use crate::managers::model::ModelManager;
use crate::settings::{get_settings, LogLevel};
use serde::Serialize;
use specta::Type;
use std::sync::Arc;
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

/// Acknowledged collection/persistence status. Unsaved runtime OFF must never
/// be displayed as saved OFF; epoch/revision reject late UI responses.
#[derive(Serialize, Type)]
pub struct DiagnosticStatus {
    /// Initial preference load completed (including fail-closed read failure).
    pub initialized: bool,
    /// Effective structured collection state, independent of legacy raw logs.
    pub effective_enabled: bool,
    /// Last known saved/default preference, absent when unreadable.
    pub saved_preference: Option<bool>,
    /// Fixed persistence state: uninitialized/default/saved/pending/unsaved/load_failed.
    pub persistence: String,
    /// Current requested value while pending or unsaved.
    pub desired_enabled: Option<bool>,
    /// Serialized preference-operation revision.
    pub revision: u64,
    /// Clear generation, independent of preference changes.
    pub epoch: u64,
}
impl From<Status> for DiagnosticStatus {
    fn from(status: Status) -> Self {
        Self {
            initialized: status.initialized,
            effective_enabled: status.effective_enabled,
            saved_preference: status.saved_preference,
            persistence: status.persistence.into(),
            desired_enabled: status.desired_enabled,
            revision: status.revision,
            epoch: status.epoch,
        }
    }
}

/// Exact immutable allowlisted snapshot. A later preview or Clear revokes its
/// export identity; client generations suppress late IPC responses after Clear.
#[derive(Serialize, Type)]
pub struct DiagnosticPreview {
    /// Opaque snapshot identity for export and safe issue summaries.
    pub report_id: String,
    /// Exact sanitized bounded export content, without credentials/user content.
    pub text: String,
    /// Clear generation captured before native context gathering.
    pub epoch: u64,
}
/// Explicit native-dialog outcome; contains no chosen destination or OS error.
#[derive(Serialize, Type)]
pub struct DiagnosticExport {
    /// Fixed outcome: saved or cancelled.
    pub status: String,
    /// Snapshot identity, never replacement text supplied by the UI.
    pub report_id: String,
    /// Snapshot generation for late-response suppression.
    pub epoch: u64,
}

/// Reads service status without loading a general settings/credential document.
#[tauri::command]
#[specta::specta]
pub fn get_diagnostic_status() -> DiagnosticStatus {
    service::global().status().into()
}

/// OFF stops before saving; failed persistence never resumes collection. ON
/// enables only after the dedicated preference is successfully saved.
#[tauri::command]
#[specta::specta]
pub async fn set_diagnostics_enabled(enabled: bool) -> DiagnosticStatus {
    service::global().set_enabled(enabled).into()
}

/// Clears structured in-memory data only. Final export commit serialization
/// ensures no older export can write after this command acknowledges success.
#[tauri::command]
#[specta::specta]
pub async fn clear_diagnostics() -> u64 {
    service::global().clear()
}

/// Captures epoch before context I/O, then publishes only if no Clear won.
#[tauri::command]
#[specta::specta]
pub async fn get_diagnostic_report(
    app: AppHandle,
    model_manager: State<'_, Arc<ModelManager>>,
) -> Result<DiagnosticPreview, String> {
    let ticket = service::global().begin_preview().map_err(str::to_string)?;
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
    let preview = service::global()
        .publish_preview(
            ticket,
            ReportContext {
                version: &app.package_info().version.to_string(),
                platform,
                log_level,
                debug_mode: settings.debug_mode,
                always_on_microphone: settings.always_on_microphone,
                model_selected: !settings.selected_model.is_empty(),
                model_available: model.as_ref().is_some_and(|model| model.is_downloaded),
                model_is_custom: model.as_ref().is_some_and(|model| model.is_custom),
                local_log,
            },
        )
        .map_err(str::to_string)?;
    Ok(DiagnosticPreview {
        report_id: preview.report.id().into(),
        text: preview.report.text().into(),
        epoch: preview.epoch,
    })
}

/// Dialog runs without service locks. The service revalidates after selection,
/// before any destination staging/creation/write under its separate commit gate.
#[tauri::command]
#[specta::specta]
pub async fn export_diagnostic_report(
    app: AppHandle,
    report_id: String,
) -> Result<DiagnosticExport, String> {
    let ticket = service::global()
        .begin_export(&report_id)
        .map_err(str::to_string)?;
    let selected = app
        .dialog()
        .file()
        .set_file_name(format!("notype-diagnostics-{}.txt", ticket.id()))
        .add_filter("Text", &["txt"])
        .blocking_save_file();
    let mut outcome = DiagnosticExport {
        status: "cancelled".into(),
        report_id: ticket.id().into(),
        epoch: ticket.epoch(),
    };
    if let Some(selected) = selected {
        let path = selected
            .into_path()
            .map_err(|_| "diagnostic-save-failed".to_string())?;
        service::global()
            .commit_export(&ticket, &path)
            .map_err(str::to_string)?;
        outcome.status = "saved".into();
    }
    Ok(outcome)
}

/// Fixed overlay telemetry event kinds. Values contain no text or absolute clock.
#[derive(serde::Deserialize, Serialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum CaptureObservation {
    /// First accepted ready receipt, without an absolute frontend clock.
    Received,
    /// First corresponding React commit; not a paint observation.
    Committed,
    /// Current noncritical settings resolution.
    Settings,
}

/// Production overlay timing ACK. This accepts only the overlay webview and the
/// current capture envelope; local durations are bounded, never absolute clocks.
#[tauri::command]
#[specta::specta]
pub fn report_capture_observation(
    window: tauri::WebviewWindow,
    capture_id: String,
    capture_revision: String,
    observation: CaptureObservation,
    duration_ms: u64,
) -> bool {
    if window.label() != "recording_overlay" {
        return false;
    }
    let (Ok(capture), Ok(revision)) = (capture_id.parse::<u64>(), capture_revision.parse::<u64>())
    else {
        return false;
    };
    if capture == 0 || revision == 0 {
        return false;
    }
    let observation = match observation {
        CaptureObservation::Received => crate::capture_observations::FrontendObservation::Received,
        CaptureObservation::Committed => {
            crate::capture_observations::FrontendObservation::Committed
        }
        CaptureObservation::Settings => crate::capture_observations::FrontendObservation::Settings,
    };
    crate::overlay::record_capture_observation(capture, revision, observation, duration_ms)
}
