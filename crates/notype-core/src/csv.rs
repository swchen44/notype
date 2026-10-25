//! User-requested raw research export: one consistent, fixed-schema CSV row per
//! history ID. This module never reads credentials/settings or uploads a file.

use crate::{read_entry, CancellationToken, SessionEntry, SessionService};
use rusqlite::{Connection, OpenFlags};
use serde::Serialize;
use std::{
    fs::{self, OpenOptions},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    time::Duration,
};
use thiserror::Error;

/// Stable errors contain neither the selected path nor an OS/SQL message.
#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum CsvExportError {
    /// The explicit destination is not a CSV file in an existing directory.
    #[error("history-csv-invalid-destination")]
    InvalidDestination,
    /// A file-backed store is required for an independent read snapshot.
    #[error("history-csv-file-store-required")]
    FileStoreRequired,
    /// The canonical history snapshot could not be read safely.
    #[error("history-csv-read-failed")]
    ReadFailed,
    /// Space observation failed or is insufficient for the checked upper bound.
    #[error("history-csv-space-unavailable")]
    SpaceUnavailable,
    /// The operation was explicitly cancelled; no destination was committed.
    #[error("history-csv-cancelled")]
    Cancelled,
    /// A staged write, sync or final replacement failed.
    #[error("history-csv-write-failed")]
    WriteFailed,
    /// The operation's own partial file could not be removed after failure.
    #[error("history-csv-cleanup-failed")]
    CleanupFailed,
}

/// Platform composition supplies an actual free-space observation. Tests may
/// replace this filesystem boundary without replacing snapshot/CSV algorithms.
pub trait CsvSpaceProbe: Send + Sync {
    /// Available bytes on the explicit destination directory's filesystem.
    fn available_bytes(&self, directory: &Path) -> Result<u64, CsvExportError>;
}

/// Successful export metadata; it deliberately does not return a filesystem path.
#[derive(Clone, Copy, Debug)]
pub struct CsvExport {
    /// Number of history IDs represented, independent of their API call counts.
    pub rows: u64,
}

const HEADERS: &[&str] = &[
    "format_version",
    "history_id",
    "created_at_utc",
    "updated_at_utc",
    "scenario",
    "status",
    "recognition_mode",
    "recording_audio_seconds",
    "recording_quality",
    "asr_requested_model",
    "asr_actual_model",
    "asr_model_name",
    "asr_backend",
    "asr_tokens_status",
    "asr_tokens",
    "original_text",
    "original_count",
    "result_text",
    "result_count",
    "back_translation",
    "count_rule_version",
    "initial_stop_to_result_ms",
    "latest_processing_kind",
    "latest_processing_ms",
    "asr_after_stop_ms",
    "api_stage_ms",
    "latest_outcome",
    "api_call_count",
    "api_observed_call_count",
    "input_tokens_status",
    "input_tokens",
    "input_tokens_known_subtotal",
    "input_tokens_reported_calls",
    "output_tokens_status",
    "output_tokens",
    "output_tokens_known_subtotal",
    "output_tokens_reported_calls",
    "actual_fee_status",
    "actual_fee_totals_by_currency",
    "actual_fee_known_subtotals_by_currency",
    "actual_fee_reported_calls",
    "api_requested_models",
    "api_reported_models",
    "api_model_efforts",
    "api_request_options",
];

impl SessionService {
    /// Export all history and call summaries from one independent read-only
    /// snapshot. Dialog/space probing/file I/O never hold the main writer mutex.
    /// Text round-trips exactly; formula-like text is not silently rewritten.
    pub fn export_csv(
        &self,
        destination: &Path,
        space: &dyn CsvSpaceProbe,
        cancellation: &CancellationToken,
    ) -> Result<CsvExport, CsvExportError> {
        if cancellation.is_cancelled() {
            return Err(CsvExportError::Cancelled);
        }
        let parent = destination
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .ok_or(CsvExportError::InvalidDestination)?;
        if !parent.is_dir()
            || !destination
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|value| value.eq_ignore_ascii_case("csv"))
        {
            return Err(CsvExportError::InvalidDestination);
        }
        let path = self
            .database_path
            .as_ref()
            .ok_or(CsvExportError::FileStoreRequired)?;
        let mut connection = Connection::open_with_flags(
            path.as_ref(),
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(|_| CsvExportError::ReadFailed)?;
        connection
            .busy_timeout(Duration::from_secs(3))
            .map_err(|_| CsvExportError::ReadFailed)?;
        connection
            .execute_batch("PRAGMA query_only=ON;")
            .map_err(|_| CsvExportError::ReadFailed)?;
        let transaction = connection
            .transaction()
            .map_err(|_| CsvExportError::ReadFailed)?;
        // Force one snapshot before the filesystem boundary; subsequent writers
        // can proceed through WAL without mixing this export's moments in time.
        let (rows,text_bytes):(u64,u64)=transaction.query_row("SELECT COUNT(*),COALESCE(SUM(length(CAST(transcript AS BLOB))+length(CAST(COALESCE(result,'') AS BLOB))+length(CAST(COALESCE(back_translation,'') AS BLOB))+length(CAST(observations AS BLOB))),0) FROM sessions",[],|row|Ok((row.get(0)?,row.get(1)?))).map_err(|_|CsvExportError::ReadFailed)?;
        let call_bytes:u64=transaction.query_row("SELECT COALESCE(SUM(length(CAST(descriptor AS BLOB))+length(CAST(COALESCE(receipt,'') AS BLOB))),0) FROM api_calls",[],|row|row.get(0)).map_err(|_|CsvExportError::ReadFailed)?;
        let bound = text_bytes
            .checked_add(call_bytes)
            .and_then(|n| n.checked_mul(8))
            .and_then(|n| {
                rows.checked_mul(8192)
                    .and_then(|extra| n.checked_add(extra))
            })
            .and_then(|n| n.checked_add(16 * 1024 * 1024))
            .ok_or(CsvExportError::SpaceUnavailable)?;
        if space.available_bytes(parent)? < bound {
            return Err(CsvExportError::SpaceUnavailable);
        }
        if cancellation.is_cancelled() {
            return Err(CsvExportError::Cancelled);
        }
        let temporary = parent.join(format!(".notype-history-{}.partial", uuid::Uuid::new_v4()));
        let file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .map_err(|_| CsvExportError::WriteFailed)?;
        let mut guard = Temporary {
            path: temporary.clone(),
            committed: false,
        };
        let result = (|| {
            let mut writer = BufWriter::with_capacity(64 * 1024, file);
            writer
                .write_all(&[0xef, 0xbb, 0xbf])
                .map_err(|_| CsvExportError::WriteFailed)?;
            write_row(&mut writer, HEADERS.iter().copied())?;
            let mut statement = transaction
                .prepare("SELECT id FROM sessions ORDER BY created_at DESC,id DESC")
                .map_err(|_| CsvExportError::ReadFailed)?;
            let mut ids = statement
                .query([])
                .map_err(|_| CsvExportError::ReadFailed)?;
            let mut written = 0;
            while let Some(row) = ids.next().map_err(|_| CsvExportError::ReadFailed)? {
                if cancellation.is_cancelled() {
                    return Err(CsvExportError::Cancelled);
                }
                let id: String = row.get(0).map_err(|_| CsvExportError::ReadFailed)?;
                let entry =
                    read_entry(&transaction, &id).map_err(|_| CsvExportError::ReadFailed)?;
                let cells = cells(&entry)?;
                write_row(&mut writer, cells.iter().map(String::as_str))?;
                written += 1;
            }
            if cancellation.is_cancelled() {
                return Err(CsvExportError::Cancelled);
            }
            writer.flush().map_err(|_| CsvExportError::WriteFailed)?;
            let file = writer
                .into_inner()
                .map_err(|_| CsvExportError::WriteFailed)?;
            file.sync_all().map_err(|_| CsvExportError::WriteFailed)?;
            drop(file);
            fs::rename(&temporary, destination).map_err(|_| CsvExportError::WriteFailed)?;
            guard.committed = true;
            Ok(CsvExport { rows: written })
        })();
        if result.is_err() {
            guard.cleanup()?;
        }
        result
    }
}

struct Temporary {
    path: PathBuf,
    committed: bool,
}
impl Temporary {
    fn cleanup(&mut self) -> Result<(), CsvExportError> {
        if !self.committed {
            match fs::remove_file(&self.path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(_) => return Err(CsvExportError::CleanupFailed),
            }
            self.committed = true;
        }
        Ok(())
    }
}
impl Drop for Temporary {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

fn write_row<'a>(
    writer: &mut impl Write,
    cells: impl Iterator<Item = &'a str>,
) -> Result<(), CsvExportError> {
    for (index, cell) in cells.enumerate() {
        if index > 0 {
            writer
                .write_all(b",")
                .map_err(|_| CsvExportError::WriteFailed)?;
        }
        writer
            .write_all(b"\"")
            .map_err(|_| CsvExportError::WriteFailed)?;
        for chunk in cell.as_bytes().split_inclusive(|byte| *byte == b'"') {
            writer
                .write_all(chunk)
                .map_err(|_| CsvExportError::WriteFailed)?;
            if chunk.last() == Some(&b'"') {
                writer
                    .write_all(b"\"")
                    .map_err(|_| CsvExportError::WriteFailed)?;
            }
        }
        writer
            .write_all(b"\"")
            .map_err(|_| CsvExportError::WriteFailed)?;
    }
    writer
        .write_all(b"\r\n")
        .map_err(|_| CsvExportError::WriteFailed)
}
fn json(value: &impl Serialize) -> Result<String, CsvExportError> {
    serde_json::to_string(value).map_err(|_| CsvExportError::ReadFailed)
}
fn label(value: &impl Serialize) -> Result<String, CsvExportError> {
    serde_json::to_value(value)
        .map_err(|_| CsvExportError::ReadFailed)?
        .as_str()
        .map(str::to_owned)
        .ok_or(CsvExportError::ReadFailed)
}
fn number(value: Option<u64>) -> String {
    value.map(|n| n.to_string()).unwrap_or_default()
}
fn cells(entry: &SessionEntry) -> Result<Vec<String>, CsvExportError> {
    let observations = &entry.observations;
    let api = &entry.api_summary;
    let model = observations.actual_asr_model.as_ref();
    let audio = observations
        .recording
        .as_ref()
        .map(|recording| {
            let ms = recording.audio_ms();
            format!("{}.{:03}", ms / 1000, ms % 1000)
        })
        .unwrap_or_default();
    Ok(vec![
        "2".into(),
        entry.id.clone(),
        entry.created_at.clone(),
        entry.updated_at.clone(),
        label(&entry.scenario)?,
        label(&entry.status)?,
        entry
            .recognition_mode
            .as_ref()
            .map(label)
            .transpose()?
            .unwrap_or_default(),
        audio,
        observations
            .recording
            .as_ref()
            .map(|r| r.quality())
            .unwrap_or("unknown")
            .into(),
        observations.requested_asr_model.clone().unwrap_or_default(),
        model.map(|m| m.id.clone()).unwrap_or_default(),
        model.and_then(|m| m.name.clone()).unwrap_or_default(),
        model.and_then(|m| m.backend.clone()).unwrap_or_default(),
        // Current engine contracts return text only; never copy API usage or counts.
        "not_reported".into(),
        String::new(),
        entry.transcript.clone(),
        number(observations.original_count),
        entry.result.clone().unwrap_or_default(),
        number(observations.result_count),
        entry.back_translation.clone().unwrap_or_default(),
        observations.count_rule.clone(),
        number(observations.initial_stop_to_result_ms),
        observations
            .latest_processing_kind
            .clone()
            .unwrap_or_default(),
        number(observations.latest_processing_ms),
        number(observations.asr_after_stop_ms),
        number(observations.latest_api_ms),
        observations.latest_outcome.clone().unwrap_or_default(),
        number(api.call_count),
        api.observed_call_count.to_string(),
        label(&api.input.status)?,
        number(api.input.total),
        number(api.input.known_subtotal),
        api.input.reported_calls.to_string(),
        label(&api.output.status)?,
        number(api.output.total),
        number(api.output.known_subtotal),
        api.output.reported_calls.to_string(),
        label(&api.fees.status)?,
        json(&api.fees.totals)?,
        json(&api.fees.known_subtotals)?,
        api.fees.reported_calls.to_string(),
        json(&api.requested_models)?,
        json(&api.reported_models)?,
        json(&api.model_efforts)?,
        json(
            &api.model_efforts
                .iter()
                .filter(|tuple| tuple.parameters.is_some())
                .collect::<Vec<_>>(),
        )?,
    ])
}
