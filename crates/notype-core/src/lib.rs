#![doc = include_str!("../README.md")]

use chrono::{SecondsFormat, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{File, OpenOptions, TryLockError};
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};
use std::{future::Future, pin::Pin};
use thiserror::Error;
pub use tokio_util::sync::CancellationToken;
use uuid::Uuid;
mod http;
pub use http::HttpTextProcessor;
pub mod csv;
pub mod gateway_error;
mod observation_store;
pub mod observations;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecognitionMode {
    #[default]
    TraditionalEnglish,
    SimplifiedEnglish,
}

pub type TargetToken = String;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scenario {
    #[default]
    Dictation,
    Translation,
    Ask,
}

/// Only a platform-verified application name. No website or content acquisition.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceContext {
    pub application: Option<String>,
}
impl SourceContext {
    pub fn is_valid(&self) -> bool {
        self.application.as_ref().is_none_or(|name| {
            !name.trim().is_empty() && name.len() <= 256 && !name.chars().any(char::is_control)
        })
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SessionOptions {
    pub recognition_mode: RecognitionMode,
    pub original_target: Option<TargetToken>,
    pub scenario: Scenario,
    pub translation_target: Option<String>,
    pub source_context: SourceContext,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SessionStatus {
    Recognized,
    Processing,
    Pending,
    Failed,
    Cancelled,
    Delivering,
    Delivered,
    DeliveryUnknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextSelection {
    Polished,
    Original,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TextRequest {
    pub transcript: String,
    pub glossary: Vec<String>,
    pub recognition_mode: Option<RecognitionMode>,
    #[serde(default)]
    pub scenario: Scenario,
    #[serde(default)]
    pub translation_target: Option<String>,
    #[serde(default)]
    pub source_context: SourceContext,
    #[serde(default)]
    pub back_translation: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextFailure {
    /// Safe local snapshot/credential preparation failure, after original saving.
    Preparation(PreparationFailure),
    /// Safe provider category; never contains an upstream body or transport URL.
    Provider(gateway_error::GatewayError),
    ConfigurationRequired,
    InvalidPrompt,
    Failed,
    Timeout,
    InvalidResponse,
    Cancelled,
    /// History accounting could not safely precede or commit the request.
    HistoryStorage,
}

/// Fixed preparation categories; no credential/path/URL or filesystem message.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreparationFailure {
    /// Selected credential missing or selected home file unavailable/invalid.
    Credentials,
    /// Unknown capability needs an explicit declaration before generation.
    Capability,
    /// Invalid or unbound route.
    Route,
    /// Unsupported/invalid request option.
    Options,
    /// Failed to read the fresh settings store.
    Storage,
}
impl PreparationFailure {
    /// Fixed safe history code; UI localizes without formatting rejected input.
    pub fn code(self) -> &'static str {
        match self {
            Self::Credentials => "configuration-credentials-unavailable",
            Self::Capability => "configuration-capability-required",
            Self::Route => "configuration-route-invalid",
            Self::Options => "configuration-invalid-options",
            Self::Storage => "configuration-storage-failed",
        }
    }
}

pub type TextFuture = Pin<Box<dyn Future<Output = Result<String, TextFailure>> + Send>>;
#[derive(Clone, Debug)]
pub struct TextOutput {
    pub text: String,
    pub source_language: Option<String>,
}
pub type GenerationFuture = Pin<Box<dyn Future<Output = Result<TextOutput, TextFailure>> + Send>>;
/// Deliberately cannot contain the original transcript: reverse the generated text independently.
#[derive(Clone, Debug)]
pub struct BackTranslationRequest {
    pub text: String,
    pub source_language: String,
    pub target_language: String,
}

pub trait TextProcessor: Send + Sync {
    fn polish(&self, request: TextRequest, cancellation: CancellationToken) -> TextFuture;
    fn generate(&self, request: TextRequest, cancellation: CancellationToken) -> GenerationFuture {
        let future = self.polish(request, cancellation);
        Box::pin(async move {
            future.await.map(|text| TextOutput {
                text,
                source_language: None,
            })
        })
    }
    fn back_translate(&self, _: BackTranslationRequest, _: CancellationToken) -> TextFuture {
        Box::pin(async { Err(TextFailure::InvalidResponse) })
    }
    /// Actual processing entry point. Defaults are only for uninstrumented
    /// test/nonnetwork adapters and cannot claim complete zero API usage.
    fn generate_observed(
        &self,
        request: TextRequest,
        cancellation: CancellationToken,
        observer: Arc<dyn observations::ApiCallObserver>,
    ) -> GenerationFuture {
        observer.unavailable();
        self.generate(request, cancellation)
    }
    /// Reverse-translation accounting is independent of the forward call.
    fn back_translate_observed(
        &self,
        request: BackTranslationRequest,
        cancellation: CancellationToken,
        observer: Arc<dyn observations::ApiCallObserver>,
    ) -> TextFuture {
        observer.unavailable();
        self.back_translate(request, cancellation)
    }
}

#[derive(Clone, Debug)]
pub struct DeliveryRequest {
    pub session_id: String,
    pub text: String,
    pub original_target: TargetToken,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeliveryOutcome {
    Delivered,
    TargetChanged,
    Unavailable,
    Uncertain,
}

pub type DeliveryFuture = Pin<Box<dyn Future<Output = DeliveryOutcome> + Send>>;

/// Implementations atomically guard the original target and insert text. They
/// must never substitute arbitrary current focus or synthesize Enter.
pub trait DeliveryAdapter: Send + Sync {
    fn deliver(&self, request: DeliveryRequest) -> DeliveryFuture;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionEntry {
    pub id: String,
    pub transcript: String,
    pub result: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub status: SessionStatus,
    pub error: Option<String>,
    pub recognition_mode: Option<RecognitionMode>,
    pub original_target: Option<TargetToken>,
    pub selection: TextSelection,
    #[serde(default)]
    pub scenario: Scenario,
    #[serde(default)]
    pub translation_target: Option<String>,
    pub source_context: SourceContext,
    pub source_language: Option<String>,
    pub back_translation: Option<String>,
    pub review_required: bool,
    pub review_confirmed: bool,
    pub revision: i64,
    /// Typed durable product observations; never a diagnostics snapshot.
    pub observations: observations::SessionObservations,
    /// All observed call usage/actual charges for this history ID, not just retry.
    pub api_summary: observations::ApiSummary,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct ImportReport {
    pub imported: usize,
    pub skipped: usize,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CoreError {
    #[error("invalid-input")]
    InvalidInput,
    #[error("session-not-found")]
    NotFound,
    #[error("session-busy")]
    Busy,
    #[error("history-already-owned")]
    AlreadyOwned,
    #[error("unsupported-history-schema")]
    UnsupportedSchema,
    #[error("history-reset-required")]
    ResetRequired,
    #[error("legacy-history-import-required")]
    LegacyImportRequired,
    #[error("history-storage-failed")]
    Storage,
    #[error("invalid-session-transition")]
    InvalidTransition,
}

impl From<rusqlite::Error> for CoreError {
    fn from(_: rusqlite::Error) -> Self {
        Self::Storage
    }
}
impl From<std::io::Error> for CoreError {
    fn from(_: std::io::Error) -> Self {
        Self::Storage
    }
}

#[derive(Clone)]
pub struct SessionService {
    connection: Arc<Mutex<Connection>>,
    active: Arc<Mutex<HashMap<String, ActiveAttempt>>>,
    _owner: Option<Arc<DatabaseOwner>>,
    database_path: Option<Arc<std::path::PathBuf>>,
}

// Constructed only after successful exclusive acquisition. Sharing the guard
// keeps live service clones exclusive; its final Drop also releases inherited
// open-file descriptions before their child process reaches exec/close.
struct DatabaseOwner(File);
impl Drop for DatabaseOwner {
    fn drop(&mut self) {
        // Drop cannot report failure or panic. Close remains the fallback; if
        // unlock fails, inherited descriptors may still delay the OS release.
        let _ = self.0.unlock();
    }
}

struct ActiveAttempt {
    revision: i64,
    cancellation: CancellationToken,
    started: std::time::Instant,
}

fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

struct AttemptGuard {
    service: SessionService,
    id: String,
    attempt: i64,
    delivery: bool,
    armed: bool,
}
impl Drop for AttemptGuard {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        if let Ok(connection) = self.service.connection() {
            let (expected, status) = if self.delivery {
                ("delivering", "delivery-unknown")
            } else {
                ("processing", "failed")
            };
            let _ = connection.execute("UPDATE sessions SET status=?2,error='interrupted',updated_at=?3 WHERE id=?1 AND attempt=?4 AND status=?5",params![self.id,status,now(),self.attempt,expected]);
            if let Ok(mut active) = self.service.active.lock() {
                if active
                    .get(&self.id)
                    .is_some_and(|active| active.revision == self.attempt)
                {
                    if let Some(active) = active.remove(&self.id) {
                        active.cancellation.cancel();
                    }
                }
            }
        }
    }
}

impl SessionService {
    pub fn open(database_path: impl AsRef<Path>) -> Result<Self, CoreError> {
        let path = database_path.as_ref();
        let file_name = path.file_name().ok_or(CoreError::InvalidInput)?;
        let parent = path
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        std::fs::create_dir_all(parent)?;
        let resolved = if path.exists() {
            path.canonicalize()?
        } else {
            parent.canonicalize()?.join(file_name)
        };
        let mut owner_path = resolved.as_os_str().to_os_string();
        owner_path.push(".notype-lock");
        let owner = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(owner_path)?;
        owner.try_lock().map_err(|error| match error {
            TryLockError::WouldBlock => CoreError::AlreadyOwned,
            TryLockError::Error(_) => CoreError::Storage,
        })?;
        let owner = Arc::new(DatabaseOwner(owner));
        let connection = Connection::open(&resolved)?;
        let mut service = Self::from_connection(connection, Some(owner))?;
        service.database_path = Some(Arc::new(resolved));
        Ok(service)
    }

    pub fn in_memory() -> Result<Self, CoreError> {
        Self::from_connection(Connection::open_in_memory()?, None)
    }

    fn from_connection(
        mut connection: Connection,
        owner: Option<Arc<DatabaseOwner>>,
    ) -> Result<Self, CoreError> {
        let version: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        let has_sessions: bool = connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='sessions')",
            [],
            |row| row.get(0),
        )?;
        let has_handy: bool = connection.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='transcription_history')", [], |row| row.get(0))?;
        if has_handy && !has_sessions {
            return Err(CoreError::LegacyImportRequired);
        }
        if version > 4 {
            return Err(CoreError::UnsupportedSchema);
        }
        let has_user_tables: bool = connection.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%')",[],|row|row.get(0))?;
        if (version != 0 && version != 4) || version == 0 && has_user_tables {
            return Err(CoreError::ResetRequired);
        }
        if has_sessions {
            let mut statement = connection.prepare("PRAGMA table_info(sessions)")?;
            let columns = statement
                .query_map([], |row| row.get::<_, String>(1))?
                .collect::<Result<Vec<_>, _>>()?;
            if [
                "id",
                "transcript",
                "result",
                "created_at",
                "updated_at",
                "status",
                "error",
            ]
            .iter()
            .any(|name| !columns.iter().any(|column| column == name))
            {
                return Err(CoreError::UnsupportedSchema);
            }
        }
        connection.execute_batch(
            "PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; PRAGMA foreign_keys=ON;",
        )?;
        let transaction = connection.transaction()?;
        transaction.execute_batch(
            "
        CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY, transcript TEXT NOT NULL, result TEXT,
            created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
            status TEXT NOT NULL, error TEXT, recognition_mode TEXT,
            original_target TEXT, selection TEXT NOT NULL DEFAULT 'polished',
            attempt INTEGER NOT NULL DEFAULT 0,
            scenario TEXT NOT NULL DEFAULT 'dictation', translation_target TEXT,
            source_context TEXT NOT NULL DEFAULT '{}', source_language TEXT,
            back_translation TEXT, review_required INTEGER NOT NULL DEFAULT 0,
            review_confirmed INTEGER NOT NULL DEFAULT 0, recognition_failed INTEGER NOT NULL DEFAULT 0,
            observations TEXT NOT NULL DEFAULT '{}'
        );",
        )?;
        transaction.execute(
            "UPDATE sessions SET recognition_failed=1 WHERE error='recognition-script-failed'",
            [],
        )?;
        transaction.execute("UPDATE sessions SET status=CASE status WHEN 'processing' THEN 'failed' ELSE 'delivery-unknown' END,error='interrupted',attempt=attempt+1,updated_at=?1 WHERE status IN ('processing','delivering')", [now()])?;
        transaction.execute_batch("CREATE TABLE IF NOT EXISTS history_imports (source_key TEXT NOT NULL,source_id INTEGER NOT NULL,session_id TEXT NOT NULL,PRIMARY KEY(source_key,source_id)); PRAGMA user_version=4;")?;
        transaction.execute_batch(observation_store::SCHEMA)?;
        transaction.execute(
            "UPDATE api_calls SET outcome='interrupted',finished_at=?1 WHERE outcome='pending'",
            [now()],
        )?;
        transaction.commit()?;
        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
            active: Arc::new(Mutex::new(HashMap::new())),
            _owner: owner,
            database_path: None,
        })
    }

    fn connection(&self) -> Result<MutexGuard<'_, Connection>, CoreError> {
        self.connection.lock().map_err(|_| CoreError::Storage)
    }

    pub fn create(
        &self,
        transcript: &str,
        options: SessionOptions,
    ) -> Result<SessionEntry, CoreError> {
        self.create_observed(
            transcript,
            options,
            observations::SessionObservations::default(),
        )
    }

    /// Atomically preserve original text and validated safe recognition metadata,
    /// before any possibly billable processing request is prepared.
    pub fn create_observed(
        &self,
        transcript: &str,
        options: SessionOptions,
        observations: observations::SessionObservations,
    ) -> Result<SessionEntry, CoreError> {
        self.create_entry(transcript, options, "recognized", None, observations)
    }

    /// Preserve available recognition text when the requested local script failed.
    /// This entry cannot accidentally enter automatic text processing or delivery.
    pub fn create_recognition_failure(
        &self,
        transcript: &str,
        options: SessionOptions,
    ) -> Result<SessionEntry, CoreError> {
        self.create_entry(
            transcript,
            options,
            "failed",
            Some("recognition-script-failed"),
            observations::SessionObservations::default(),
        )
    }

    /// Preserve a script-failed original with the same validated capture metadata.
    pub fn create_recognition_failure_observed(
        &self,
        transcript: &str,
        options: SessionOptions,
        observations: observations::SessionObservations,
    ) -> Result<SessionEntry, CoreError> {
        self.create_entry(
            transcript,
            options,
            "failed",
            Some("recognition-script-failed"),
            observations,
        )
    }
    /// Record the same-process stop-to-durable-original endpoint before API work.
    /// Revision zero fences this observation from later retries or cancellations.
    pub fn observe_original_ready(
        &self,
        id: &str,
        stopped_at: std::time::Instant,
    ) -> Result<SessionEntry, CoreError> {
        let connection = self.connection()?;
        let mut entry = read_entry(&connection, id)?;
        if entry.revision != 0 {
            return Err(CoreError::InvalidTransition);
        }
        entry.observations.asr_after_stop_ms = Some(elapsed_ms(stopped_at));
        connection.execute(
            "UPDATE sessions SET observations=?2 WHERE id=?1 AND attempt=0",
            params![
                id,
                serde_json::to_string(&entry.observations).map_err(|_| CoreError::Storage)?
            ],
        )?;
        read_entry(&connection, id)
    }

    fn create_entry(
        &self,
        transcript: &str,
        options: SessionOptions,
        status: &str,
        error: Option<&str>,
        mut observations: observations::SessionObservations,
    ) -> Result<SessionEntry, CoreError> {
        if transcript.trim().is_empty() || !options.source_context.is_valid() {
            return Err(CoreError::InvalidInput);
        }
        observations.validate()?;
        observations.original_count = Some(observations::mixed_count(transcript));
        observations.result_count = None;
        observations.count_rule = observations::COUNT_RULE_VERSION.into();
        let id = Uuid::new_v4().to_string();
        let timestamp = now();
        self.connection()?.execute(
            "INSERT INTO sessions
            (id, transcript, created_at, updated_at, status, recognition_mode, original_target,scenario,translation_target,source_context,error,recognition_failed,observations)
            VALUES (?1,?2,?3,?3,?9,?4,?5,?6,?7,?8,?10,?11,?12)",
            params![
                id,
                transcript,
                timestamp,
                serde_json::to_string(&options.recognition_mode)
                    .unwrap()
                    .trim_matches('"'),
                options.original_target,
                serde_json::to_string(&options.scenario).unwrap().trim_matches('"'),
                options.translation_target,
                serde_json::to_string(&options.source_context).map_err(|_|CoreError::InvalidInput)?,
                status,error,error==Some("recognition-script-failed"),
                serde_json::to_string(&observations).map_err(|_|CoreError::InvalidInput)?
            ],
        )?;
        self.get(&id)
    }

    pub fn get(&self, id: &str) -> Result<SessionEntry, CoreError> {
        read_entry(&*self.connection()?, id)
    }

    /// Literal substring search over the displayed result, or original without a result.
    pub fn search(&self, query: &str) -> Result<Vec<SessionEntry>, CoreError> {
        let connection = self.connection()?;
        // SQLite's built-in lower() only folds ASCII. Use the same Unicode
        // lowercase substring semantics as the displayed-text/highlight UI.
        let query = query.trim().to_lowercase();
        let mut statement = connection.prepare("SELECT id,coalesce(nullif(result,''),transcript) FROM sessions ORDER BY created_at DESC,id DESC")?;
        let ids = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .filter_map(|row| match row {
                Ok((id, text)) => text.to_lowercase().contains(&query).then_some(Ok(id)),
                Err(error) => Some(Err(error)),
            })
            .collect::<Result<Vec<_>, _>>()?;
        ids.iter().map(|id| read_entry(&connection, id)).collect()
    }

    pub fn delete(&self, id: &str) -> Result<(), CoreError> {
        let connection = self.connection()?;
        let entry = read_entry(&connection, id)?;
        if entry.status == SessionStatus::Delivering {
            return Err(CoreError::Busy);
        }
        connection.execute("DELETE FROM sessions WHERE id=?1", [id])?;
        if let Some(active) = self
            .active
            .lock()
            .map_err(|_| CoreError::Storage)?
            .remove(id)
        {
            active.cancellation.cancel();
        }
        Ok(())
    }

    /// Reads a Handy database without modifying it or accessing its recordings.
    /// Import tombstones survive destination deletion so repeated imports cannot
    /// silently restore text that a user deleted from notype.
    pub fn import_handy_history(
        &self,
        source_path: impl AsRef<Path>,
    ) -> Result<ImportReport, CoreError> {
        let source_path = source_path.as_ref().canonicalize()?;
        let source_key = source_path.to_str().ok_or(CoreError::InvalidInput)?;
        let source =
            Connection::open_with_flags(&source_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        let mut statement = source.prepare("SELECT id,timestamp,transcription_text,post_processed_text FROM transcription_history ORDER BY id").map_err(|_|CoreError::UnsupportedSchema)?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let mut report = ImportReport {
            imported: 0,
            skipped: 0,
        };
        for (source_id, timestamp, transcript, result) in rows {
            let imported: bool = transaction.query_row(
                "SELECT EXISTS(SELECT 1 FROM history_imports WHERE source_key=?1 AND source_id=?2)",
                params![source_key, source_id],
                |row| row.get(0),
            )?;
            if imported || transcript.trim().is_empty() {
                report.skipped += 1;
                continue;
            }
            let timestamp = chrono::DateTime::from_timestamp(timestamp, 0)
                .ok_or(CoreError::InvalidInput)?
                .to_rfc3339_opts(SecondsFormat::Millis, true);
            let id = Uuid::new_v4().to_string();
            let result = result.filter(|text| !text.trim().is_empty());
            let status = if result.is_some() {
                "pending"
            } else {
                "recognized"
            };
            transaction.execute("INSERT INTO sessions (id,transcript,result,created_at,updated_at,status) VALUES (?1,?2,?3,?4,?4,?5)",params![id,transcript,result,timestamp,status])?;
            transaction.execute(
                "INSERT INTO history_imports VALUES (?1,?2,?3)",
                params![source_key, source_id, id],
            )?;
            report.imported += 1;
        }
        transaction.commit()?;
        Ok(report)
    }

    pub fn choose_original(&self, id: &str) -> Result<SessionEntry, CoreError> {
        {
            let connection = self.connection()?;
            let entry = read_entry(&connection, id)?;
            if !matches!(
                entry.status,
                SessionStatus::Recognized
                    | SessionStatus::Failed
                    | SessionStatus::Pending
                    | SessionStatus::Cancelled
            ) {
                return Err(CoreError::InvalidTransition);
            }
            connection.execute("UPDATE sessions SET status='pending',selection='original',review_confirmed=0,attempt=attempt+1,updated_at=?2 WHERE id=?1", params![id,now()])?;
        }
        self.get(id)
    }

    /// User-authorized automatic raw fallback. Cancellation and completed delivery are never revived.
    pub fn original_for_delivery(&self, id: &str) -> Result<SessionEntry, CoreError> {
        self.original_for_delivery_with_stop(id, None)
    }

    /// Record availability against this capture's accepted stop; retries pass no old clock.
    pub fn original_for_delivery_with_stop(
        &self,
        id: &str,
        stopped_at: Option<std::time::Instant>,
    ) -> Result<SessionEntry, CoreError> {
        {
            let connection = self.connection()?;
            let entry = read_entry(&connection, id)?;
            if !matches!(
                entry.status,
                SessionStatus::Recognized | SessionStatus::Failed | SessionStatus::Pending
            ) || entry.transcript.trim().is_empty()
            {
                return Err(CoreError::InvalidTransition);
            }
            let failed: bool = connection.query_row(
                "SELECT recognition_failed FROM sessions WHERE id=?1",
                [id],
                |row| row.get(0),
            )?;
            if failed {
                return Err(CoreError::InvalidTransition);
            }
            let mut observations = entry.observations;
            if let Some(stopped_at) = stopped_at {
                observations.initial_stop_to_result_ms = Some(elapsed_ms(stopped_at));
            }
            if observations.latest_processing_kind.is_none() {
                observations.latest_outcome = Some("original_only".into());
            }
            connection.execute("UPDATE sessions SET status='pending',selection='original',review_required=0,review_confirmed=0,attempt=attempt+1,updated_at=?2,observations=?3 WHERE id=?1",params![id,now(),serde_json::to_string(&observations).map_err(|_|CoreError::Storage)?])?;
        }
        self.get(id)
    }

    pub fn cancel(&self, id: &str) -> Result<SessionEntry, CoreError> {
        {
            let connection = self.connection()?;
            let entry = read_entry(&connection, id)?;
            if matches!(
                entry.status,
                SessionStatus::Delivered | SessionStatus::DeliveryUnknown
            ) {
                return Ok(entry);
            }
            let status = if entry.status == SessionStatus::Delivering {
                "delivery-unknown"
            } else {
                "cancelled"
            };
            let mut observations = entry.observations.clone();
            if entry.status == SessionStatus::Processing {
                if let Some(active) = self.active.lock().map_err(|_| CoreError::Storage)?.get(id) {
                    if active.revision == entry.revision {
                        observations.latest_processing_ms = Some(elapsed_ms(active.started));
                        observations.latest_outcome = Some("cancelled".into());
                    }
                }
            }
            connection.execute("UPDATE sessions SET status=?2,error='cancelled',attempt=attempt+1,updated_at=?3,observations=?4 WHERE id=?1", params![id,status,now(),serde_json::to_string(&observations).map_err(|_|CoreError::Storage)?])?;
            if let Some(active) = self
                .active
                .lock()
                .map_err(|_| CoreError::Storage)?
                .remove(id)
            {
                active.cancellation.cancel();
            }
        }
        self.get(id)
    }

    pub async fn deliver(
        &self,
        id: &str,
        adapter: Arc<dyn DeliveryAdapter>,
    ) -> Result<SessionEntry, CoreError> {
        let (request, attempt, processing_error) = {
            let connection = self.connection()?;
            let entry = read_entry(&connection, id)?;
            if entry.status != SessionStatus::Pending {
                return Ok(entry);
            }
            if entry.review_required && !entry.review_confirmed {
                return Ok(entry);
            }
            let Some(original_target) = entry.original_target else {
                return Ok(entry);
            };
            let processing_error = (entry.selection == TextSelection::Original)
                .then_some(entry.error)
                .flatten()
                .filter(|code| {
                    !matches!(
                        code.as_str(),
                        "cancelled"
                            | "target-changed"
                            | "delivery-unavailable"
                            | "delivery-unknown"
                    )
                });
            let text = match entry.selection {
                TextSelection::Original => entry.transcript,
                TextSelection::Polished => entry.result.ok_or(CoreError::InvalidTransition)?,
            };
            let attempt: i64 = connection.query_row(
                "SELECT attempt+1 FROM sessions WHERE id=?1",
                [id],
                |row| row.get(0),
            )?;
            connection.execute("UPDATE sessions SET status='delivering',attempt=?2,error=NULL,updated_at=?3 WHERE id=?1", params![id,attempt,now()])?;
            (
                DeliveryRequest {
                    session_id: id.to_owned(),
                    text,
                    original_target,
                },
                attempt,
                processing_error,
            )
        };
        let mut guard = AttemptGuard {
            service: self.clone(),
            id: id.to_owned(),
            attempt,
            delivery: true,
            armed: true,
        };
        let outcome = adapter.deliver(request).await;
        let (status, error) = match outcome {
            // Raw fallback is a successful input operation even when AI failed.
            // Keep its safe processing code so the delivered history explains why.
            DeliveryOutcome::Delivered => ("delivered", processing_error.as_deref()),
            DeliveryOutcome::TargetChanged => ("pending", Some("target-changed")),
            DeliveryOutcome::Unavailable => ("pending", Some("delivery-unavailable")),
            DeliveryOutcome::Uncertain => ("delivery-unknown", Some("delivery-unknown")),
        };
        let connection = self.connection()?;
        connection.execute("UPDATE sessions SET status=?2,error=?3,updated_at=?4 WHERE id=?1 AND attempt=?5 AND status='delivering'", params![id,status,error,now(),attempt])?;
        guard.armed = false;
        read_entry(&connection, id)
    }

    pub async fn process(
        &self,
        id: &str,
        glossary: Vec<String>,
        processor: Arc<dyn TextProcessor>,
    ) -> Result<SessionEntry, CoreError> {
        self.process_with_review(id, glossary, processor, false)
            .await
    }

    /// Approves only the exact displayed revision. It does not deliver or change focus.
    pub fn confirm_review(&self, id: &str, revision: i64) -> Result<SessionEntry, CoreError> {
        let connection = self.connection()?;
        let entry = read_entry(&connection, id)?;
        if entry.status != SessionStatus::Pending
            || !entry.review_required
            || entry.revision != revision
            || (entry.selection == TextSelection::Polished
                && (entry.result.is_none() || entry.back_translation.is_none()))
        {
            return Err(CoreError::InvalidTransition);
        }
        connection.execute(
            "UPDATE sessions SET review_confirmed=1,updated_at=?2 WHERE id=?1",
            params![id, now()],
        )?;
        read_entry(&connection, id)
    }

    pub async fn process_with_review(
        &self,
        id: &str,
        glossary: Vec<String>,
        processor: Arc<dyn TextProcessor>,
        review: bool,
    ) -> Result<SessionEntry, CoreError> {
        self.process_with_stop(id, glossary, processor, review, None)
            .await
    }
    /// Process one revision with an optional same-process accepted-stop origin.
    /// Retry callers omit it; durations never span human waiting or restarts.
    pub async fn process_with_stop(
        &self,
        id: &str,
        glossary: Vec<String>,
        processor: Arc<dyn TextProcessor>,
        review: bool,
        stopped_at: Option<std::time::Instant>,
    ) -> Result<SessionEntry, CoreError> {
        let cancellation = CancellationToken::new();
        let processing_started = std::time::Instant::now();
        let (entry, attempt, mut observations) = {
            let connection = self.connection()?;
            let entry = read_entry(&connection, id)?;
            let recognition_failed: bool = connection.query_row(
                "SELECT recognition_failed FROM sessions WHERE id=?1",
                [id],
                |row| row.get(0),
            )?;
            if recognition_failed {
                return Err(CoreError::InvalidTransition);
            }
            if matches!(
                entry.status,
                SessionStatus::Processing | SessionStatus::Delivering
            ) {
                return Err(CoreError::Busy);
            }
            if matches!(
                entry.status,
                SessionStatus::Delivered | SessionStatus::DeliveryUnknown
            ) {
                return Err(CoreError::InvalidTransition);
            }
            let attempt: i64 = connection.query_row(
                "SELECT attempt+1 FROM sessions WHERE id=?1",
                [id],
                |row| row.get(0),
            )?;
            let mut observations = entry.observations.clone();
            observations.result_count = None;
            observations.latest_processing_kind = Some(
                if observations.latest_processing_kind.is_none() {
                    "initial"
                } else {
                    "retry"
                }
                .into(),
            );
            observations.latest_processing_ms = None;
            observations.latest_api_ms = None;
            observations.latest_outcome = None;
            connection.execute("UPDATE sessions SET status='processing',result=NULL,back_translation=NULL,source_language=NULL,review_required=?4,review_confirmed=0,error=NULL,selection='polished',attempt=?3,updated_at=?2,observations=?5 WHERE id=?1", params![id, now(), attempt,review&&entry.scenario==Scenario::Translation,serde_json::to_string(&observations).map_err(|_|CoreError::Storage)?])?;
            self.active.lock().map_err(|_| CoreError::Storage)?.insert(
                id.to_owned(),
                ActiveAttempt {
                    revision: attempt,
                    cancellation: cancellation.clone(),
                    started: processing_started,
                },
            );
            (entry, attempt, observations)
        };
        let api_observer = Arc::new(observation_store::StoreObserver::new(
            self.clone(),
            id.to_owned(),
            attempt,
        ));
        let mut guard = AttemptGuard {
            service: self.clone(),
            id: id.to_owned(),
            attempt,
            delivery: false,
            armed: true,
        };
        let review = review && entry.scenario == Scenario::Translation;
        let response = tokio::select! {
            biased;
            _ = cancellation.cancelled() => Err(TextFailure::Cancelled),
            response = processor.generate_observed(TextRequest { transcript: entry.transcript, glossary, recognition_mode: entry.recognition_mode,scenario:entry.scenario,translation_target:entry.translation_target.clone(),source_context:entry.source_context,back_translation:review }, cancellation.clone(),api_observer.clone()) => response,
        };
        let mut source_language = None;
        let mut back_translation = None;
        let (status, result, error) = match response {
            Err(TextFailure::Preparation(error)) => ("failed", None, Some(error.code())),
            Err(TextFailure::ConfigurationRequired) => {
                ("failed", None, Some("configuration-required"))
            }
            Ok(output) if !output.text.trim().is_empty() => {
                source_language = output.source_language;
                if review {
                    // This committed write precedes construction/polling of the second request.
                    observations.result_count = Some(observations::mixed_count(&output.text));
                    let changed = {
                        let mut connection = self.connection()?;
                        let transaction = connection.transaction()?;
                        api_observer.flush(&transaction)?;
                        let changed=transaction.execute("UPDATE sessions SET result=?2,source_language=?3,updated_at=?4,observations=?6 WHERE id=?1 AND attempt=?5 AND status='processing'",params![id,output.text,source_language,now(),attempt,serde_json::to_string(&observations).map_err(|_|CoreError::Storage)?])?;
                        transaction.commit()?;
                        changed
                    };
                    if changed == 0 {
                        guard.armed = false;
                        return self.get(id);
                    }
                    let back = if let (Some(source), Some(target)) = (
                        source_language.clone().filter(|s| !s.trim().is_empty()),
                        entry.translation_target,
                    ) {
                        tokio::select! {biased; _=cancellation.cancelled()=>Err(TextFailure::Cancelled),response=processor.back_translate_observed(BackTranslationRequest{text:output.text.clone(),source_language:target,target_language:source},cancellation.clone(),api_observer.clone())=>response}
                    } else {
                        Err(TextFailure::InvalidResponse)
                    };
                    match back {
                        Ok(text) if !text.trim().is_empty() => {
                            back_translation = Some(text);
                            ("pending", Some(output.text), None)
                        }
                        Err(TextFailure::Cancelled) => {
                            ("cancelled", Some(output.text), Some("cancelled"))
                        }
                        Err(TextFailure::Provider(error)) => {
                            ("pending", Some(output.text), Some(error.history_code(true)))
                        }
                        Err(TextFailure::Timeout) => (
                            "pending",
                            Some(output.text),
                            Some("back-translation-timeout"),
                        ),
                        _ => (
                            "pending",
                            Some(output.text),
                            Some("back-translation-failed"),
                        ),
                    }
                } else {
                    ("pending", Some(output.text), None)
                }
            }
            Ok(_) => ("failed", None, Some("llm-invalid-response")),
            Err(TextFailure::Provider(error)) => ("failed", None, Some(error.history_code(false))),
            Err(TextFailure::Timeout) => ("failed", None, Some("llm-timeout")),
            Err(TextFailure::Cancelled) => ("cancelled", None, Some("cancelled")),
            Err(TextFailure::InvalidResponse) => ("failed", None, Some("llm-invalid-response")),
            Err(TextFailure::Failed) => ("failed", None, Some("llm-failed")),
            Err(TextFailure::InvalidPrompt) => ("failed", None, Some("prompt-invalid")),
            Err(TextFailure::HistoryStorage) => ("failed", None, Some("history-storage-failed")),
        };
        let mut connection = self.connection()?;
        let mut active = self.active.lock().map_err(|_| CoreError::Storage)?;
        if active
            .get(id)
            .is_some_and(|active| active.revision == attempt)
        {
            active.remove(id);
        }
        observations.result_count = result.as_ref().map(|text| observations::mixed_count(text));
        let original_only =
            result.is_none() && !api_observer.has_sent() && error == Some("configuration-required");
        observations.latest_outcome = Some(
            if original_only {
                "original_only"
            } else {
                status
            }
            .into(),
        );
        observations.api_metadata_unavailable |= api_observer.missing();
        let transaction = connection.transaction()?;
        api_observer.flush(&transaction)?;
        transaction.execute("UPDATE sessions SET status=?2,result=?3,error=?4,updated_at=?5,source_language=?7,back_translation=?8,observations=?9 WHERE id=?1 AND attempt=?6 AND status='processing'", params![id,status,result,error,now(),attempt,source_language,back_translation,serde_json::to_string(&observations).map_err(|_|CoreError::Storage)?])?;
        transaction.commit()?;
        // Measure after durable result/status availability, before human review or delivery.
        observations.latest_processing_ms = Some(elapsed_ms(processing_started));
        observations.latest_api_ms = api_observer.stage_ms();
        if observations.latest_processing_kind.as_deref() == Some("initial") {
            if result.is_some() && error.is_none() && (!review || back_translation.is_some()) {
                observations.initial_stop_to_result_ms = stopped_at.map(elapsed_ms);
            } else if original_only {
                observations.initial_stop_to_result_ms = observations.asr_after_stop_ms;
            }
        }
        connection.execute(
            "UPDATE sessions SET observations=?3 WHERE id=?1 AND attempt=?2 AND status=?4",
            params![
                id,
                attempt,
                serde_json::to_string(&observations).map_err(|_| CoreError::Storage)?,
                status
            ],
        )?;
        guard.armed = false;
        read_entry(&connection, id)
    }
}
fn elapsed_ms(started: std::time::Instant) -> u64 {
    started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
}

fn read_entry(connection: &Connection, id: &str) -> Result<SessionEntry, CoreError> {
    let mut entry=connection
        .query_row(
            "SELECT id,transcript,result,created_at,updated_at,status,error,
        recognition_mode,original_target,selection,scenario,translation_target,source_context,source_language,back_translation,review_required,review_confirmed,attempt,observations FROM sessions WHERE id=?1",
            [id],
            |row| {
                let status: String = row.get(5)?;
                let mode: Option<String> = row.get(7)?;
                let selection: String = row.get(9)?;
                let scenario:String=row.get(10)?;
                let decode = |value: &str| format!("\"{value}\"");
                Ok(SessionEntry {
                    id: row.get(0)?,
                    transcript: row.get(1)?,
                    result: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                    status: serde_json::from_str(&decode(&status))
                        .map_err(|_| rusqlite::Error::InvalidQuery)?,
                    error: row.get(6)?,
                    recognition_mode: mode
                        .map(|value| serde_json::from_str(&decode(&value)))
                        .transpose()
                        .map_err(|_| rusqlite::Error::InvalidQuery)?,
                    original_target: row.get(8)?,
                    selection: serde_json::from_str(&decode(&selection))
                        .map_err(|_| rusqlite::Error::InvalidQuery)?,
                    scenario:serde_json::from_str(&decode(&scenario)).map_err(|_|rusqlite::Error::InvalidQuery)?,
                    translation_target:row.get(11)?,
                    source_context:serde_json::from_str(&row.get::<_,String>(12)?).map_err(|_|rusqlite::Error::InvalidQuery)?,
                    source_language:row.get(13)?,
                    back_translation:row.get(14)?,
                    review_required:row.get(15)?,
                    review_confirmed:row.get(16)?,
                    revision:row.get(17)?,
                    observations:serde_json::from_str(&row.get::<_,String>(18)?).map_err(|_|rusqlite::Error::InvalidQuery)?,
                    api_summary:Default::default(),
                })
            },
        )
        .optional()?
        .ok_or(CoreError::NotFound)?;
    entry.api_summary = observation_store::summary(
        connection,
        id,
        entry.observations.api_metadata_unavailable || entry.status == SessionStatus::Processing,
    )?;
    Ok(entry)
}
