//! One-shot authorization journal. It links production call IDs without storing
//! another usage/fee ledger. Every mutation commits FULL SQLite before returning.
use super::policy::{schedule, BUDGET_UNITS, MAX_CALLS, ROUND_ID};
use rusqlite::{params, Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions, TryLockError},
    io::{Read, Write},
    path::Path,
};

/// Fixed safe failure vocabulary; never format rejected input or an OS error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failure {
    /// Prerequisite unavailable; no selected credential or inference is allowed.
    NotReady,
    /// Retained/partial round may be inspected but never resumed or replaced.
    ReportOnly,
    /// Another process owns the same round lock.
    AlreadyOwned,
    /// Durable write/read/reconciliation failed; preserve retained evidence.
    Storage,
    /// Immutable slot, purpose or revision order was not satisfied.
    Schedule,
    /// A prior monotonic failure blocks all further admission.
    Halted,
    /// Fixed public contract, wire or receipt failed validation.
    Contract,
    /// Fixed-corpus machine check failed; human meaning remains unverified.
    Quality,
    /// Actual quota/space observation failed or crossed a retained allowance.
    Resource,
}
impl Failure {
    /// Stable CLI/receipt code, independent of provider/error text.
    pub fn code(self) -> &'static str {
        match self {
            Self::NotReady => "probe-not-ready",
            Self::ReportOnly => "probe-report-only",
            Self::AlreadyOwned => "probe-already-owned",
            Self::Storage => "probe-storage-failed",
            Self::Schedule => "probe-schedule-denied",
            Self::Halted => "probe-halted",
            Self::Contract => "probe-contract-failed",
            Self::Quality => "probe-quality-failed",
            Self::Resource => "probe-resource-unavailable",
        }
    }
}
/// Durable irreversible lifecycle; reservations never return to unused state.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum State {
    /// Never admitted; no reservation or bill is implied.
    NotRun,
    /// Actual queued core call linked before reservation.
    Linked,
    /// Reservation committed before durable core send admission.
    PossiblySent,
    /// Terminal callback observed; independent core flush is still required.
    Finished,
    /// Independent persisted core receipt/outcome and checked journal agree.
    Reconciled,
    /// Permanent free-tier-unverified skip; never resolves Gemini credentials.
    SealedGemini,
}
/// Terminal local outcome. Expected cancellation is legal only in slot7.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Outcome {
    /// Production call succeeded through the sealed receipt gate.
    Success,
    /// Slot7 explicitly requested local cancellation without adverse evidence.
    ExpectedCancellation,
    /// Actual failure; reservations and unknown fees remain retained.
    Failed,
}
/// Safe immutable linkage to the actual core ledger; no raw response or secret.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Record {
    /// One-based fixed manifest slot.
    pub slot: u8,
    /// Durable reservation lifecycle.
    pub state: State,
    /// Actual SessionService history ID, absent until core admission succeeds.
    pub history: String,
    /// Actual ApiCallObserver begin ID, unique within this round.
    pub call: String,
    /// Actual accepted processing revision, never a guessed attempt number.
    pub processing_revision: u64,
    /// Explicitly requested scheduled cancellation, never inferred from Drop.
    pub cancellation_requested: bool,
    /// Local terminal outcome, absent while pending.
    pub outcome: Option<Outcome>,
    /// Successful sealed OpenRouter zero-cost validation; not an actual fee.
    pub reported_zero_validated: bool,
}
/// Bounded nonsecret source/contract evidence, immutable after fresh creation.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Identity {
    /// Clean compiled source SHA, lowercase full Git object ID.
    pub source: String,
    /// Exact source policy SHA256, not a credential fingerprint.
    pub policy_sha256: String,
    /// Exact fixed corpus/prompt SHA256.
    pub corpus_sha256: String,
    /// Fresh bounded public evidence hashes; maximum six unauthenticated GETs.
    pub contract_sha256: Vec<String>,
}
/// Full validated reservation snapshot. Existing evidence is always report-only.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    /// Fixed schema/round identity; cannot be supplied through CLI arguments.
    pub round: String,
    /// Compare-and-commit monotonic revision.
    pub revision: u64,
    /// Immutable nonsecret provenance.
    pub identity: Identity,
    /// Whether a fresh arm sealed Gemini and authorized the fixed schedule.
    pub armed: bool,
    /// Monotonic admission halt; never cleared, even after callback/storage errors.
    pub halted: bool,
    /// All sixteen immutable slot positions, including sealed Gemini entries.
    pub records: Vec<Record>,
}
impl Snapshot {
    /// Conservative spent reservation, independent of actual unknown bills.
    pub fn reservation_units(&self) -> Result<u64, Failure> {
        self.records
            .iter()
            .filter(|r| {
                matches!(
                    r.state,
                    State::PossiblySent | State::Finished | State::Reconciled
                )
            })
            .try_fold(0u64, |total, r| {
                total
                    .checked_add(
                        schedule()
                            .get(usize::from(r.slot.checked_sub(1).ok_or(Failure::Storage)?))
                            .ok_or(Failure::Storage)?
                            .reservation(),
                    )
                    .filter(|value| *value <= BUDGET_UNITS)
                    .ok_or(Failure::Storage)
            })
    }
}
fn hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}
fn id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
}
fn validate(state: &Snapshot) -> Result<(), Failure> {
    if state.round != ROUND_ID
        || state.revision > 256
        || state.records.len() != 16
        || !hex(&state.identity.source, 40)
        || !hex(&state.identity.policy_sha256, 64)
        || !hex(&state.identity.corpus_sha256, 64)
        || state.identity.contract_sha256.len() > 6
        || state.identity.contract_sha256.iter().any(|v| !hex(v, 64))
    {
        return Err(Failure::Storage);
    }
    let mut tail = false;
    let mut active = false;
    let mut fatal = false;
    for (index, record) in state.records.iter().enumerate() {
        if usize::from(record.slot) != index + 1 {
            return Err(Failure::Storage);
        }
        if index >= MAX_CALLS {
            if record.state
                != if state.armed {
                    State::SealedGemini
                } else {
                    State::NotRun
                }
            {
                return Err(Failure::Storage);
            }
        } else if record.state == State::SealedGemini {
            return Err(Failure::Storage);
        }
        if matches!(record.state, State::NotRun | State::SealedGemini) {
            if !record.history.is_empty()
                || !record.call.is_empty()
                || record.processing_revision != 0
                || record.outcome.is_some()
                || record.cancellation_requested
                || record.reported_zero_validated
            {
                return Err(Failure::Storage);
            }
            if index < MAX_CALLS {
                tail = true;
            }
            continue;
        }
        if !state.armed
            || tail
            || active
            || fatal
            || !id(&record.history)
            || !id(&record.call)
            || record.processing_revision == 0
            || record.processing_revision > i64::MAX as u64
        {
            return Err(Failure::Storage);
        }
        if record.cancellation_requested && record.slot != 7 {
            return Err(Failure::Storage);
        }
        let terminal = matches!(record.state, State::Finished | State::Reconciled);
        if terminal != record.outcome.is_some()
            || record.outcome == Some(Outcome::ExpectedCancellation)
                && (!record.cancellation_requested || record.slot != 7)
        {
            return Err(Failure::Storage);
        }
        if record.reported_zero_validated
            && (!(9..=12).contains(&record.slot) || record.outcome != Some(Outcome::Success))
        {
            return Err(Failure::Storage);
        }
        if record.outcome == Some(Outcome::Success)
            && (9..=12).contains(&record.slot)
            && !record.reported_zero_validated
        {
            return Err(Failure::Storage);
        }
        active = record.state != State::Reconciled;
        fatal = record.outcome == Some(Outcome::Failed);
        for prior in &state.records[..index] {
            if prior.call.is_empty() {
                continue;
            }
            if prior.call == record.call
                || (schedule()[usize::from(prior.slot - 1)].group == schedule()[index].group)
                    != (prior.history == record.history)
            {
                return Err(Failure::Storage);
            }
            if prior.history == record.history {
                let retry = record.slot == 8;
                if retry && record.processing_revision <= prior.processing_revision
                    || !retry && record.processing_revision != prior.processing_revision
                {
                    return Err(Failure::Storage);
                }
            }
        }
    }
    if fatal && !state.halted
        || (!state.armed && state.records.iter().any(|r| r.state != State::NotRun))
    {
        return Err(Failure::Storage);
    }
    state.reservation_units()?;
    Ok(())
}
/// Reject links/reparse points through every existing ancestor before file I/O.
pub fn plain(path: &Path) -> Result<(), Failure> {
    if !path.is_absolute()
        || path.components().any(|c| {
            matches!(
                c,
                std::path::Component::ParentDir | std::path::Component::CurDir
            )
        })
    {
        return Err(Failure::Storage);
    }
    for ancestor in path.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(meta) => {
                if meta.file_type().is_symlink() || ancestor != path && !meta.is_dir() {
                    return Err(Failure::Storage);
                }
                #[cfg(windows)]
                {
                    use std::os::windows::fs::MetadataExt;
                    if meta.file_attributes() & 0x400 != 0 {
                        return Err(Failure::Storage);
                    }
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(Failure::Storage),
        }
    }
    Ok(())
}
struct Owner(File);
impl Drop for Owner {
    fn drop(&mut self) {
        let _ = self.0.unlock();
    }
}
/// Exclusive single-round SQLite journal. The caller supplies only a fixed live
/// child or an explicitly owned fixture. Reopening never authorizes mutations.
pub struct Journal {
    connection: Connection,
    _owner: Owner,
    snapshot: Snapshot,
    report_only: bool,
    failed: bool,
}
impl Journal {
    /// Create once or reopen a valid retained report. Partial/foreign/corrupt
    /// entries fail closed and are retained; there is no reset/recovery API.
    pub fn open(directory: &Path, identity: Identity) -> Result<Self, Failure> {
        Self::open_inner(directory, Some(identity))
    }
    /// Read retained state exclusively without creating any missing directory,
    /// marker, database or recovery identity. Every mutation remains denied.
    pub fn open_report(directory: &Path) -> Result<Self, Failure> {
        Self::open_inner(directory, None)
    }
    fn open_inner(directory: &Path, identity: Option<Identity>) -> Result<Self, Failure> {
        plain(directory)?;
        let fresh = match identity {
            None => {
                if !directory.is_dir() {
                    return Err(Failure::ReportOnly);
                }
                false
            }
            Some(_) => match fs::create_dir(directory) {
                Ok(()) => true,
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => false,
                Err(_) => return Err(Failure::Storage),
            },
        };
        let marker = directory.join("round.lock");
        let database = directory.join("round.sqlite");
        if !fresh {
            let entries = fs::read_dir(directory)
                .map_err(|_| Failure::Storage)?
                .take(4)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| Failure::Storage)?;
            if entries.len() > 3 {
                return Err(Failure::Storage);
            }
            for entry in entries {
                if !["round.lock", "round.sqlite", "round.sqlite-journal"]
                    .iter()
                    .any(|name| entry.file_name() == *name)
                {
                    return Err(Failure::Storage);
                }
                plain(&entry.path())?;
                if !entry.file_type().map_err(|_| Failure::Storage)?.is_file() {
                    return Err(Failure::Storage);
                }
            }
        }
        let mut options = OpenOptions::new();
        options.read(true).write(true);
        if fresh {
            options.create_new(true);
        }
        let mut file = options.open(marker).map_err(|_| Failure::Storage)?;
        file.try_lock().map_err(|error| match error {
            TryLockError::WouldBlock => Failure::AlreadyOwned,
            _ => Failure::Storage,
        })?;
        const MAGIC: &[u8] = b"notype-owned-P25-live-v4\n";
        if fresh {
            file.write_all(MAGIC)
                .and_then(|_| file.sync_all())
                .map_err(|_| Failure::Storage)?;
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&database)
                .and_then(|f| f.sync_all())
                .map_err(|_| Failure::Storage)?;
        } else {
            let mut bytes = vec![];
            (&mut file)
                .take(128)
                .read_to_end(&mut bytes)
                .map_err(|_| Failure::Storage)?;
            let meta = fs::metadata(&database).map_err(|_| Failure::Storage)?;
            if bytes != MAGIC || !meta.is_file() || meta.len() == 0 || meta.len() > 1_048_576 {
                return Err(Failure::Storage);
            }
        }
        let owner = Owner(file);
        let flags = if fresh {
            OpenFlags::SQLITE_OPEN_READ_WRITE
        } else {
            OpenFlags::SQLITE_OPEN_READ_ONLY
        };
        let connection =
            Connection::open_with_flags(&database, flags).map_err(|_| Failure::Storage)?;
        if fresh {
            connection.execute_batch("PRAGMA journal_mode=DELETE;PRAGMA synchronous=FULL;CREATE TABLE reservation(id INTEGER PRIMARY KEY CHECK(id=1),revision INTEGER NOT NULL,payload TEXT NOT NULL);PRAGMA user_version=4;").map_err(|_|Failure::Storage)?;
            let state = Snapshot {
                round: ROUND_ID.into(),
                revision: 0,
                identity: identity.ok_or(Failure::Storage)?,
                armed: false,
                halted: false,
                records: schedule()
                    .iter()
                    .map(|slot| Record {
                        slot: slot.number,
                        state: State::NotRun,
                        history: String::new(),
                        call: String::new(),
                        processing_revision: 0,
                        cancellation_requested: false,
                        outcome: None,
                        reported_zero_validated: false,
                    })
                    .collect(),
            };
            validate(&state)?;
            connection
                .execute(
                    "INSERT INTO reservation VALUES(1,0,?1)",
                    [serde_json::to_string(&state).map_err(|_| Failure::Storage)?],
                )
                .map_err(|_| Failure::Storage)?;
        }
        let snapshot = load(&connection)?;
        Ok(Self {
            connection,
            _owner: owner,
            snapshot,
            report_only: !fresh,
            failed: false,
        })
    }
    /// Owned safe snapshot; no raw business text or credentials are present.
    pub fn snapshot(&self) -> Snapshot {
        self.snapshot.clone()
    }
    /// Existing state never becomes eligible again, including an unarmed crash.
    pub fn report_only(&self) -> bool {
        self.report_only
    }
    fn update(
        &mut self,
        operation: impl FnOnce(&mut Snapshot) -> Result<(), Failure>,
    ) -> Result<(), Failure> {
        if self.report_only {
            return Err(Failure::ReportOnly);
        }
        if self.failed {
            return Err(Failure::Halted);
        }
        let result = (|| {
            let old = load(&self.connection)?;
            if old != self.snapshot {
                return Err(Failure::Storage);
            }
            let mut next = old.clone();
            operation(&mut next)?;
            next.revision = next.revision.checked_add(1).ok_or(Failure::Storage)?;
            if old.identity != next.identity
                || old.armed && !next.armed
                || old.halted && !next.halted
                || old.records.iter().zip(&next.records).any(|(old, new)| {
                    new.state < old.state
                        || !old.call.is_empty()
                            && (old.call != new.call
                                || old.history != new.history
                                || old.processing_revision != new.processing_revision)
                        || old.cancellation_requested && !new.cancellation_requested
                })
            {
                return Err(Failure::Storage);
            }
            validate(&next)?;
            let payload = serde_json::to_string(&next).map_err(|_| Failure::Storage)?;
            if payload.len() > 32_768 {
                return Err(Failure::Storage);
            }
            let transaction = self
                .connection
                .transaction()
                .map_err(|_| Failure::Storage)?;
            let count = transaction
                .execute(
                    "UPDATE reservation SET revision=?1,payload=?2 WHERE id=1 AND revision=?3",
                    params![next.revision, payload, old.revision],
                )
                .map_err(|_| Failure::Storage)?;
            if count != 1 {
                return Err(Failure::Storage);
            }
            transaction.commit().map_err(|_| Failure::Storage)?;
            self.snapshot = next;
            Ok(())
        })();
        if result == Err(Failure::Storage) {
            self.failed = true;
        }
        result
    }
    /// Arm exactly once and durably seal Gemini before any slot admission.
    pub fn arm(&mut self) -> Result<(), Failure> {
        self.update(|state| {
            if state.armed || state.halted {
                return Err(Failure::NotReady);
            }
            state.armed = true;
            for r in &mut state.records[MAX_CALLS..] {
                r.state = State::SealedGemini;
            }
            Ok(())
        })
    }
    /// Monotonic durable halt. A failed write still blocks this process in memory.
    pub fn halt(&mut self) -> Result<(), Failure> {
        if self.snapshot.halted {
            return Ok(());
        }
        let result = self.update(|state| {
            state.halted = true;
            Ok(())
        });
        if result.is_err() {
            self.failed = true;
        }
        result
    }
    /// Link the real core-queued ID only after its begin succeeded. No spending
    /// is authorized yet; the next mark-sent reservation must still commit.
    pub fn link(
        &mut self,
        slot: u8,
        history: &str,
        call: &str,
        revision: u64,
    ) -> Result<(), Failure> {
        self.update(|state| {
            if !state.armed || state.halted || slot == 0 || usize::from(slot) > MAX_CALLS {
                return Err(Failure::Halted);
            }
            let index = usize::from(slot - 1);
            if state.records[index].state != State::NotRun
                || state.records[..index]
                    .iter()
                    .any(|r| r.state != State::Reconciled)
            {
                return Err(Failure::Schedule);
            }
            state.records[index] = Record {
                slot,
                state: State::Linked,
                history: history.into(),
                call: call.into(),
                processing_revision: revision,
                cancellation_requested: false,
                outcome: None,
                reported_zero_validated: false,
            };
            Ok(())
        })
    }
    /// Commit the full conservative reservation before the core possibly-sent
    /// callback. Failed core admission never refunds this authorization.
    pub fn reserve(&mut self, slot: u8, call: &str) -> Result<(), Failure> {
        self.update(|state| {
            if state.halted {
                return Err(Failure::Halted);
            }
            let r = record_mut(state, slot, call)?;
            if r.state != State::Linked {
                return Err(Failure::Schedule);
            }
            r.state = State::PossiblySent;
            Ok(())
        })
    }
    /// Mark only the scheduled current cancellation, before cancelling the core.
    pub fn request_cancellation(&mut self, call: &str) -> Result<(), Failure> {
        self.update(|state| {
            let r = record_mut(state, 7, call)?;
            if !matches!(r.state, State::PossiblySent | State::Finished) {
                return Err(Failure::Schedule);
            }
            r.cancellation_requested = true;
            Ok(())
        })
    }
    /// Commit the callback's local outcome. This alone never proves a core flush;
    /// independent reconciliation is mandatory before every later begin.
    pub fn finish(&mut self, slot: u8, call: &str, outcome: Outcome) -> Result<(), Failure> {
        self.update(|state| {
            let r = record_mut(state, slot, call)?;
            if !matches!(r.state, State::Linked | State::PossiblySent) {
                return Err(Failure::Schedule);
            }
            r.state = State::Finished;
            r.outcome = Some(outcome);
            r.reported_zero_validated = (9..=12).contains(&slot) && outcome == Outcome::Success;
            if outcome == Outcome::Failed {
                state.halted = true;
            }
            Ok(())
        })
    }
    /// Commit externally verified actual-core terminal evidence, checked against
    /// the captured helper revision after the other store's read completed.
    pub fn reconciled(&mut self, revision: u64, slot: u8, call: &str) -> Result<(), Failure> {
        self.update(|state| {
            if state.revision != revision {
                return Err(Failure::Storage);
            }
            let r = record_mut(state, slot, call)?;
            if r.state != State::Finished {
                return Err(Failure::Schedule);
            }
            r.state = State::Reconciled;
            Ok(())
        })
    }
}
fn record_mut<'a>(
    state: &'a mut Snapshot,
    slot: u8,
    call: &str,
) -> Result<&'a mut Record, Failure> {
    state
        .records
        .get_mut(usize::from(slot.checked_sub(1).ok_or(Failure::Schedule)?))
        .filter(|r| r.call == call)
        .ok_or(Failure::Schedule)
}
fn load(connection: &Connection) -> Result<Snapshot, Failure> {
    let schema: i64 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|_| Failure::Storage)?;
    let count: i64 = connection
        .query_row("SELECT COUNT(*) FROM reservation", [], |row| row.get(0))
        .map_err(|_| Failure::Storage)?;
    if schema != 4 || count != 1 {
        return Err(Failure::Storage);
    }
    let (revision, payload): (u64, String) = connection
        .query_row(
            "SELECT revision,payload FROM reservation WHERE id=1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|_| Failure::Storage)?;
    if payload.len() > 32_768 {
        return Err(Failure::Storage);
    }
    let state: Snapshot = serde_json::from_str(&payload).map_err(|_| Failure::Storage)?;
    if revision != state.revision {
        return Err(Failure::Storage);
    }
    validate(&state)?;
    Ok(state)
}
