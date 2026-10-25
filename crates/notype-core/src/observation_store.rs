//! Transactional accounting for the actual SessionService processing boundary.
//! Prepared rows survive cancellation; safe pending receipts never cause resends.

use crate::observations::{
    ApiCallDescriptor, ApiCallObserver, ApiOutcome, ApiReceipt, SessionObservations,
    SummaryAccumulator,
};
use crate::{CoreError, SessionService, TextFailure};
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};

pub(crate) const SCHEMA:&str="CREATE TABLE IF NOT EXISTS api_calls (
 id TEXT PRIMARY KEY, session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
 revision INTEGER NOT NULL, ordinal INTEGER NOT NULL, descriptor TEXT NOT NULL,
 send_state TEXT NOT NULL DEFAULT 'queued', outcome TEXT NOT NULL DEFAULT 'pending',
 receipt TEXT, elapsed_ms INTEGER, started_at TEXT NOT NULL, finished_at TEXT,
 UNIQUE(session_id,ordinal)); CREATE INDEX IF NOT EXISTS api_calls_history ON api_calls(session_id,ordinal);";

#[derive(Clone, Default)]
struct Pending {
    receipt: Option<ApiReceipt>,
    outcome: Option<ApiOutcome>,
    elapsed_ms: Option<u64>,
    sent: bool,
}

pub(crate) struct StoreObserver {
    service: SessionService,
    history: String,
    revision: i64,
    pending: Mutex<HashMap<String, Pending>>,
    unavailable: AtomicBool,
}
impl StoreObserver {
    pub(crate) fn new(service: SessionService, history: String, revision: i64) -> Self {
        Self {
            service,
            history,
            revision,
            pending: Mutex::new(HashMap::new()),
            unavailable: AtomicBool::new(false),
        }
    }
    pub(crate) fn has_sent(&self) -> bool {
        self.pending
            .lock()
            .map(|pending| pending.values().any(|p| p.sent))
            .unwrap_or(false)
    }
    pub(crate) fn missing(&self) -> bool {
        self.unavailable.load(Ordering::Acquire)
    }
    /// Flush the same bounded in-process safe receipts together with the final
    /// result transaction. Failure is propagated, never converted to a resend.
    pub(crate) fn flush(&self, connection: &Connection) -> Result<(), CoreError> {
        let pending = self.pending.lock().map_err(|_| CoreError::Storage)?.clone();
        for (id, value) in pending {
            write_pending(connection, &self.history, &id, &value)?;
        }
        if self.missing() {
            if let Some(json) = connection
                .query_row(
                    "SELECT observations FROM sessions WHERE id=?1",
                    [&self.history],
                    |row| row.get::<_, String>(0),
                )
                .optional()?
            {
                let mut observations: SessionObservations =
                    serde_json::from_str(&json).map_err(|_| CoreError::Storage)?;
                observations.api_metadata_unavailable = true;
                connection.execute(
                    "UPDATE sessions SET observations=?2 WHERE id=?1",
                    params![
                        self.history,
                        serde_json::to_string(&observations).map_err(|_| CoreError::Storage)?
                    ],
                )?;
            }
        }
        Ok(())
    }
    fn write_now(&self, id: &str) {
        let value = self
            .pending
            .lock()
            .ok()
            .and_then(|pending| pending.get(id).cloned());
        if let Some(value) = value {
            // A failed attempt leaves the safe value in pending for flush().
            if let Ok(connection) = self.service.connection() {
                let _ = write_pending(&connection, &self.history, id, &value);
            }
        }
    }
}
impl ApiCallObserver for StoreObserver {
    fn begin(&self, descriptor: ApiCallDescriptor) -> Result<String, TextFailure> {
        if !descriptor.valid() {
            return Err(TextFailure::ConfigurationRequired);
        }
        let id = uuid::Uuid::new_v4().to_string();
        let json = serde_json::to_string(&descriptor).map_err(|_| TextFailure::HistoryStorage)?;
        let connection = self
            .service
            .connection()
            .map_err(|_| TextFailure::HistoryStorage)?;
        let changed=connection.execute("INSERT INTO api_calls(id,session_id,revision,ordinal,descriptor,started_at)
            SELECT ?1,?2,?3,COALESCE((SELECT MAX(ordinal) FROM api_calls WHERE session_id=?2),0)+1,?4,?5
            FROM sessions WHERE id=?2 AND attempt=?3 AND status='processing'",params![id,self.history,self.revision,json,crate::now()]).map_err(|_|TextFailure::HistoryStorage)?;
        if changed == 0 {
            return Err(TextFailure::Cancelled);
        }
        drop(connection);
        self.pending
            .lock()
            .map_err(|_| TextFailure::HistoryStorage)?
            .insert(id.clone(), Pending::default());
        Ok(id)
    }
    fn mark_sent(&self, id: &str) -> Result<(), TextFailure> {
        let connection = self
            .service
            .connection()
            .map_err(|_| TextFailure::HistoryStorage)?;
        let changed=connection.execute("UPDATE api_calls SET send_state='possibly_sent' WHERE id=?1 AND session_id=?2 AND revision=?3 AND send_state='queued'
            AND EXISTS(SELECT 1 FROM sessions WHERE id=?2 AND attempt=?3 AND status='processing')",params![id,self.history,self.revision]).map_err(|_|TextFailure::HistoryStorage)?;
        if changed == 0 {
            return Err(TextFailure::Cancelled);
        }
        drop(connection);
        if let Some(value) = self
            .pending
            .lock()
            .map_err(|_| TextFailure::HistoryStorage)?
            .get_mut(id)
        {
            value.sent = true;
        }
        Ok(())
    }
    fn response(&self, id: &str, receipt: ApiReceipt) {
        if let Ok(mut pending) = self.pending.lock() {
            if let Some(value) = pending.get_mut(id) {
                value.receipt = Some(receipt.sanitized());
            }
        }
        self.write_now(id);
    }
    fn finish(&self, id: &str, outcome: ApiOutcome, elapsed_ms: u64) {
        if let Ok(mut pending) = self.pending.lock() {
            if let Some(value) = pending.get_mut(id) {
                value.outcome = Some(outcome);
                value.elapsed_ms = Some(elapsed_ms.min(i64::MAX as u64));
            }
        }
        self.write_now(id);
    }
    fn unavailable(&self) {
        self.unavailable.store(true, Ordering::Release);
    }
}

fn write_pending(
    connection: &Connection,
    history: &str,
    id: &str,
    value: &Pending,
) -> Result<(), CoreError> {
    let receipt = value
        .receipt
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(|_| CoreError::Storage)?;
    let outcome = value
        .outcome
        .map(|outcome| serde_json::to_string(&outcome))
        .transpose()
        .map_err(|_| CoreError::Storage)?;
    let outcome = outcome.as_deref().map(|s| s.trim_matches('"'));
    connection.execute("UPDATE api_calls SET receipt=COALESCE(?3,receipt),send_state=CASE WHEN ?3 IS NOT NULL AND send_state!='queued' THEN 'response_received' ELSE send_state END,
        outcome=CASE WHEN outcome='pending' THEN COALESCE(?4,outcome) ELSE outcome END,elapsed_ms=COALESCE(elapsed_ms,?5),finished_at=CASE WHEN ?4 IS NOT NULL THEN COALESCE(finished_at,?6) ELSE finished_at END
        WHERE id=?1 AND session_id=?2",params![id,history,receipt,outcome,value.elapsed_ms,crate::now()])?;
    Ok(())
}

pub(crate) fn summary(
    connection: &Connection,
    history: &str,
    unavailable: bool,
) -> Result<crate::observations::ApiSummary, CoreError> {
    let mut accumulator = SummaryAccumulator::new(unavailable);
    let mut statement=connection.prepare("SELECT descriptor,receipt FROM api_calls WHERE session_id=?1 AND send_state!='queued' ORDER BY ordinal")?;
    let mut rows = statement.query([history])?;
    while let Some(row) = rows.next()? {
        let descriptor: ApiCallDescriptor =
            serde_json::from_str(&row.get::<_, String>(0)?).map_err(|_| CoreError::Storage)?;
        let receipt: Option<String> = row.get(1)?;
        let receipt = receipt
            .map(|json| serde_json::from_str::<ApiReceipt>(&json))
            .transpose()
            .map_err(|_| CoreError::Storage)?
            .unwrap_or_default()
            .sanitized();
        accumulator.add(descriptor, receipt);
    }
    Ok(accumulator.finish())
}
