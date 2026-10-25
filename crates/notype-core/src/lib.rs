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

/// Only platform-verified, minimized metadata. Never page titles, input values or URLs.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceContext {
    pub application: Option<String>,
    pub website_domain: Option<String>,
}
impl SourceContext {
    pub fn is_valid(&self) -> bool {
        self.application.as_ref().is_none_or(|name| {
            !name.trim().is_empty() && name.len() <= 256 && !name.chars().any(char::is_control)
        }) && self.website_domain.as_ref().is_none_or(|domain| {
            domain.len() <= 253
                && !domain.is_empty()
                && domain.split('.').all(|label| {
                    !label.is_empty()
                        && label.len() <= 63
                        && !label.starts_with('-')
                        && !label.ends_with('-')
                        && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
                })
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
    ConfigurationRequired,
    InvalidPrompt,
    Failed,
    Timeout,
    InvalidResponse,
    Cancelled,
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
    active: Arc<Mutex<HashMap<String, (i64, CancellationToken)>>>,
    _owner: Option<Arc<File>>,
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
                    .is_some_and(|(attempt, _)| *attempt == self.attempt)
                {
                    if let Some((_, cancellation)) = active.remove(&self.id) {
                        cancellation.cancel();
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
        let connection = Connection::open(&resolved)?;
        Self::from_connection(connection, Some(Arc::new(owner)))
    }

    pub fn in_memory() -> Result<Self, CoreError> {
        Self::from_connection(Connection::open_in_memory()?, None)
    }

    fn from_connection(
        mut connection: Connection,
        owner: Option<Arc<File>>,
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
        if version > 3 {
            return Err(CoreError::UnsupportedSchema);
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
        connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;")?;
        let transaction = connection.transaction()?;
        transaction.execute_batch(
            "
        CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY, transcript TEXT NOT NULL, result TEXT,
            created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
            status TEXT NOT NULL, error TEXT, recognition_mode TEXT,
            original_target TEXT, selection TEXT NOT NULL DEFAULT 'polished',
            attempt INTEGER NOT NULL DEFAULT 0
        );",
        )?;
        let columns = {
            let mut statement = transaction.prepare("PRAGMA table_info(sessions)")?;
            let columns = statement
                .query_map([], |row| row.get::<_, String>(1))?
                .collect::<Result<Vec<_>, _>>()?;
            columns
        };
        for (name, definition) in [
            ("recognition_mode", "TEXT"),
            ("original_target", "TEXT"),
            ("selection", "TEXT NOT NULL DEFAULT 'polished'"),
            ("attempt", "INTEGER NOT NULL DEFAULT 0"),
            ("scenario", "TEXT NOT NULL DEFAULT 'dictation'"),
            ("translation_target", "TEXT"),
            ("source_context", "TEXT NOT NULL DEFAULT '{}'"),
            ("source_language", "TEXT"),
            ("back_translation", "TEXT"),
            ("review_required", "INTEGER NOT NULL DEFAULT 0"),
            ("review_confirmed", "INTEGER NOT NULL DEFAULT 0"),
            ("recognition_failed", "INTEGER NOT NULL DEFAULT 0"),
        ] {
            if !columns.iter().any(|column| column == name) {
                transaction.execute_batch(&format!(
                    "ALTER TABLE sessions ADD COLUMN {name} {definition}"
                ))?;
            }
        }
        transaction.execute(
            "UPDATE sessions SET recognition_failed=1 WHERE error='recognition-script-failed'",
            [],
        )?;
        transaction.execute("UPDATE sessions SET status=CASE status WHEN 'processing' THEN 'failed' ELSE 'delivery-unknown' END,error='interrupted',attempt=attempt+1,updated_at=?1 WHERE status IN ('processing','delivering')", [now()])?;
        transaction.execute_batch("CREATE TABLE IF NOT EXISTS history_imports (source_key TEXT NOT NULL,source_id INTEGER NOT NULL,session_id TEXT NOT NULL,PRIMARY KEY(source_key,source_id)); PRAGMA user_version=3;")?;
        transaction.commit()?;
        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
            active: Arc::new(Mutex::new(HashMap::new())),
            _owner: owner,
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
        self.create_entry(transcript, options, "recognized", None)
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
        )
    }

    fn create_entry(
        &self,
        transcript: &str,
        options: SessionOptions,
        status: &str,
        error: Option<&str>,
    ) -> Result<SessionEntry, CoreError> {
        if transcript.trim().is_empty() || !options.source_context.is_valid() {
            return Err(CoreError::InvalidInput);
        }
        let id = Uuid::new_v4().to_string();
        let timestamp = now();
        self.connection()?.execute(
            "INSERT INTO sessions
            (id, transcript, created_at, updated_at, status, recognition_mode, original_target,scenario,translation_target,source_context,error,recognition_failed)
            VALUES (?1,?2,?3,?3,?9,?4,?5,?6,?7,?8,?10,?11)",
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
                status,error,error==Some("recognition-script-failed")
            ],
        )?;
        self.get(&id)
    }

    pub fn get(&self, id: &str) -> Result<SessionEntry, CoreError> {
        read_entry(&*self.connection()?, id)
    }

    /// Literal substring search across originals and results, newest first.
    pub fn search(&self, query: &str) -> Result<Vec<SessionEntry>, CoreError> {
        let connection = self.connection()?;
        let mut statement = connection.prepare("SELECT id FROM sessions WHERE instr(lower(transcript),lower(?1))>0 OR instr(lower(coalesce(result,'')),lower(?1))>0 ORDER BY created_at DESC,id DESC")?;
        let ids = statement
            .query_map([query], |row| row.get::<_, String>(0))?
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
        if let Some((_, cancellation)) = self
            .active
            .lock()
            .map_err(|_| CoreError::Storage)?
            .remove(id)
        {
            cancellation.cancel();
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
            connection.execute("UPDATE sessions SET status=?2,error='cancelled',attempt=attempt+1,updated_at=?3 WHERE id=?1", params![id,status,now()])?;
            if let Some((_, cancellation)) = self
                .active
                .lock()
                .map_err(|_| CoreError::Storage)?
                .remove(id)
            {
                cancellation.cancel();
            }
        }
        self.get(id)
    }

    pub async fn deliver(
        &self,
        id: &str,
        adapter: Arc<dyn DeliveryAdapter>,
    ) -> Result<SessionEntry, CoreError> {
        let (request, attempt) = {
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
            DeliveryOutcome::Delivered => ("delivered", None),
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
        let cancellation = CancellationToken::new();
        let (entry, attempt) = {
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
            connection.execute("UPDATE sessions SET status='processing',result=NULL,back_translation=NULL,source_language=NULL,review_required=?4,review_confirmed=0,error=NULL,selection='polished',attempt=?3,updated_at=?2 WHERE id=?1", params![id, now(), attempt,review&&entry.scenario==Scenario::Translation])?;
            self.active
                .lock()
                .map_err(|_| CoreError::Storage)?
                .insert(id.to_owned(), (attempt, cancellation.clone()));
            (entry, attempt)
        };
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
            response = processor.generate(TextRequest { transcript: entry.transcript, glossary, recognition_mode: entry.recognition_mode,scenario:entry.scenario,translation_target:entry.translation_target.clone(),source_context:entry.source_context,back_translation:review }, cancellation.clone()) => response,
        };
        let mut source_language = None;
        let mut back_translation = None;
        let (status, result, error) = match response {
            Err(TextFailure::ConfigurationRequired) => {
                ("failed", None, Some("configuration-required"))
            }
            Ok(output) if !output.text.trim().is_empty() => {
                source_language = output.source_language;
                if review {
                    // This committed write precedes construction/polling of the second request.
                    let changed=self.connection()?.execute("UPDATE sessions SET result=?2,source_language=?3,updated_at=?4 WHERE id=?1 AND attempt=?5 AND status='processing'",params![id,output.text,source_language,now(),attempt])?;
                    if changed == 0 {
                        guard.armed = false;
                        return self.get(id);
                    }
                    let back = if let (Some(source), Some(target)) = (
                        source_language.clone().filter(|s| !s.trim().is_empty()),
                        entry.translation_target,
                    ) {
                        tokio::select! {biased; _=cancellation.cancelled()=>Err(TextFailure::Cancelled),response=processor.back_translate(BackTranslationRequest{text:output.text.clone(),source_language:target,target_language:source},cancellation.clone())=>response}
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
            Err(TextFailure::Timeout) => ("failed", None, Some("llm-timeout")),
            Err(TextFailure::Cancelled) => ("cancelled", None, Some("cancelled")),
            Err(TextFailure::InvalidResponse) => ("failed", None, Some("llm-invalid-response")),
            Err(TextFailure::Failed) => ("failed", None, Some("llm-failed")),
            Err(TextFailure::InvalidPrompt) => ("failed", None, Some("prompt-invalid")),
        };
        let connection = self.connection()?;
        let mut active = self.active.lock().map_err(|_| CoreError::Storage)?;
        if active
            .get(id)
            .is_some_and(|(generation, _)| *generation == attempt)
        {
            active.remove(id);
        }
        connection.execute("UPDATE sessions SET status=?2,result=?3,error=?4,updated_at=?5,source_language=?7,back_translation=?8 WHERE id=?1 AND attempt=?6 AND status='processing'", params![id,status,result,error,now(),attempt,source_language,back_translation])?;
        guard.armed = false;
        read_entry(&connection, id)
    }
}

fn read_entry(connection: &Connection, id: &str) -> Result<SessionEntry, CoreError> {
    connection
        .query_row(
            "SELECT id,transcript,result,created_at,updated_at,status,error,
        recognition_mode,original_target,selection,scenario,translation_target,source_context,source_language,back_translation,review_required,review_confirmed,attempt FROM sessions WHERE id=?1",
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
                })
            },
        )
        .optional()?
        .ok_or(CoreError::NotFound)
}
