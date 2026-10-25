//! Owned, one-round SQLite reservation store. No App database or history fee.
use super::{
    policy::{Denied, ROUND_ID},
    round::{validate_snapshot, Access, Outcome, RoundStore, SlotRecord, SlotState, Snapshot},
};
use rusqlite::{params, Connection, OpenFlags, TransactionBehavior};
use std::{
    fs::{self, File, OpenOptions, TryLockError},
    io::{Read, Write},
    path::Path,
};

const MARKER: &[u8] = b"notype-owned-deepseek-P23-live-v1\n";

/// Exclusive lock and SQLite transaction adapter for the helper's owned round.
/// The eventual binary supplies its fixed known-folder child; this API accepts
/// an owned absolute fixture path so offline tests never touch that live root.
pub struct SqliteRoundStore {
    connection: Connection,
    _owner: File,
    access: Access,
}

impl SqliteRoundStore {
    /// Create a fresh round exclusively, or open an existing valid report only.
    /// Missing/corrupt/foreign artifacts fail closed instead of recreating them.
    /// The returned store owns its file lock until Drop. Errors reveal only fixed
    /// codes; partial fresh creation remains evidence and is never auto-reset.
    pub fn open_owned(directory: &Path) -> Result<Self, Denied> {
        if !directory.is_absolute() || directory.file_name().is_none() {
            return Err(Denied::Storage);
        }
        plain_ancestors(directory)?;
        let fresh = match fs::create_dir(directory) {
            Ok(()) => true,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => false,
            Err(_) => return Err(Denied::Storage),
        };
        let marker = directory.join("round.lock");
        let database = directory.join("round.db");
        if !fresh {
            for entry in fs::read_dir(directory).map_err(|_| Denied::Storage)? {
                let entry = entry.map_err(|_| Denied::Storage)?;
                let name = entry.file_name();
                if !["round.lock", "round.db", "round.db-journal"]
                    .iter()
                    .any(|n| name == *n)
                {
                    return Err(Denied::Storage);
                }
                plain_ancestors(&entry.path())?;
            }
        }
        let mut options = OpenOptions::new();
        options.read(true).write(true);
        if fresh {
            options.create_new(true);
        }
        let mut owner = options.open(marker).map_err(|_| Denied::Storage)?;
        owner.try_lock().map_err(|e| match e {
            TryLockError::WouldBlock => Denied::AlreadyOwned,
            _ => Denied::Storage,
        })?;
        if fresh {
            owner
                .write_all(MARKER)
                .and_then(|_| owner.sync_all())
                .map_err(|_| Denied::Storage)?;
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&database)
                .and_then(|f| f.sync_all())
                .map_err(|_| Denied::Storage)?;
        } else {
            let mut marker = Vec::new();
            (&mut owner)
                .take(MARKER.len() as u64 + 1)
                .read_to_end(&mut marker)
                .map_err(|_| Denied::Storage)?;
            let meta = fs::metadata(&database).map_err(|_| Denied::Storage)?;
            if marker != MARKER || !meta.is_file() || meta.len() == 0 || meta.len() > 1_048_576 {
                return Err(Denied::Storage);
            }
        }
        let access = if fresh {
            Access::Fresh
        } else {
            Access::ReportOnly
        };
        let flags = if fresh {
            OpenFlags::SQLITE_OPEN_READ_WRITE
        } else {
            OpenFlags::SQLITE_OPEN_READ_ONLY
        };
        let mut connection =
            Connection::open_with_flags(database, flags).map_err(|_| Denied::Storage)?;
        if fresh {
            connection
                .execute_batch("PRAGMA journal_mode=DELETE; PRAGMA synchronous=FULL;")
                .map_err(|_| Denied::Storage)?;
            let mode: i64 = connection
                .query_row("PRAGMA synchronous", [], |row| row.get(0))
                .map_err(|_| Denied::Storage)?;
            if mode != 2 {
                return Err(Denied::Storage);
            }
            let transaction = connection.transaction().map_err(|_| Denied::Storage)?;
            transaction.execute_batch("CREATE TABLE round_meta (id INTEGER PRIMARY KEY CHECK(id=1), round_id TEXT NOT NULL, revision INTEGER NOT NULL, armed INTEGER NOT NULL CHECK(armed IN(0,1)), halted INTEGER NOT NULL CHECK(halted IN(0,1))); CREATE TABLE slots (number INTEGER PRIMARY KEY CHECK(number BETWEEN 1 AND 8), state INTEGER NOT NULL CHECK(state BETWEEN 0 AND 7), history_id TEXT NOT NULL, call_id TEXT NOT NULL); PRAGMA user_version=1;").map_err(|_| Denied::Storage)?;
            transaction
                .execute("INSERT INTO round_meta VALUES (1,?1,0,0,0)", [ROUND_ID])
                .map_err(|_| Denied::Storage)?;
            for number in 1..=8 {
                transaction
                    .execute("INSERT INTO slots VALUES (?1,0,'','')", [number])
                    .map_err(|_| Denied::Storage)?;
            }
            transaction.commit().map_err(|_| Denied::Storage)?;
        }
        let store = Self {
            connection,
            _owner: owner,
            access,
        };
        store.load()?;
        Ok(store)
    }
}

fn read_snapshot(connection: &Connection) -> Result<Snapshot, Denied> {
    let schema: i64 = connection
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .map_err(|_| Denied::Storage)?;
    if schema != 1 {
        return Err(Denied::Storage);
    }
    let (revision, armed, halted): (i64, i64, i64) = connection
        .query_row(
            "SELECT revision,armed,halted FROM round_meta WHERE id=1 AND round_id=?1",
            [ROUND_ID],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .map_err(|_| Denied::Storage)?;
    if revision < 0 || !(0..=1).contains(&armed) || !(0..=1).contains(&halted) {
        return Err(Denied::Storage);
    }
    let mut state = Snapshot {
        revision: revision as u64,
        armed: armed == 1,
        halted: halted == 1,
        ..Snapshot::default()
    };
    let mut statement = connection.prepare("SELECT number,state,history_id,call_id FROM slots WHERE length(history_id)<=128 AND length(call_id)<=128 ORDER BY number").map_err(|_| Denied::Storage)?;
    let rows = statement
        .query_map([], |r| {
            Ok((
                r.get::<_, u8>(0)?,
                r.get::<_, u8>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
            ))
        })
        .map_err(|_| Denied::Storage)?;
    let mut count = 0;
    for row in rows {
        let (number, code, history_id, call_id) = row.map_err(|_| Denied::Storage)?;
        if count >= 8 || number as usize != count + 1 {
            return Err(Denied::Storage);
        }
        let slot_state = decode(code)?;
        state.slots[count] = SlotRecord {
            state: slot_state,
            history_id,
            call_id,
        };
        count += 1;
    }
    if count != 8 {
        return Err(Denied::Storage);
    }
    validate_snapshot(&state)?;
    Ok(state)
}
impl RoundStore for SqliteRoundStore {
    fn load(&self) -> Result<(Access, Snapshot), Denied> {
        Ok((self.access, read_snapshot(&self.connection)?))
    }
    fn commit(&mut self, previous_revision: u64, next: &Snapshot) -> Result<(), Denied> {
        if self.access != Access::Fresh {
            return Err(Denied::ReportOnly);
        }
        validate_snapshot(next)?;
        let previous = i64::try_from(previous_revision).map_err(|_| Denied::Storage)?;
        let revision = i64::try_from(next.revision).map_err(|_| Denied::Storage)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| Denied::Storage)?;
        let current = read_snapshot(&transaction)?;
        if current.revision != previous_revision
            || previous_revision.checked_add(1) != Some(next.revision)
        {
            return Err(Denied::Storage);
        }
        if transaction
            .execute(
                "UPDATE round_meta SET revision=?1,armed=?2,halted=?3 WHERE id=1 AND revision=?4",
                params![revision, next.armed, next.halted, previous],
            )
            .map_err(|_| Denied::Storage)?
            != 1
        {
            return Err(Denied::Storage);
        }
        for (index, slot) in next.slots.iter().enumerate() {
            if transaction
                .execute(
                    "UPDATE slots SET state=?1,history_id=?2,call_id=?3 WHERE number=?4",
                    params![
                        encode(&slot.state),
                        slot.history_id,
                        slot.call_id,
                        index + 1
                    ],
                )
                .map_err(|_| Denied::Storage)?
                != 1
            {
                return Err(Denied::Storage);
            }
        }
        transaction.commit().map_err(|_| Denied::Storage)
    }
}

fn encode(state: &SlotState) -> u8 {
    match state {
        SlotState::NotRun => 0,
        SlotState::PossiblySent => 1,
        SlotState::Finished(Outcome::Success) => 2,
        SlotState::Finished(Outcome::IntentionalCancellation) => 3,
        SlotState::Finished(Outcome::CancelRacedWithCompletion) => 4,
        SlotState::Finished(Outcome::Failed) => 5,
        SlotState::Finished(Outcome::ContractViolation) => 6,
        SlotState::Skipped => 7,
    }
}
fn decode(code: u8) -> Result<SlotState, Denied> {
    Ok(match code {
        0 => SlotState::NotRun,
        1 => SlotState::PossiblySent,
        2 => SlotState::Finished(Outcome::Success),
        3 => SlotState::Finished(Outcome::IntentionalCancellation),
        4 => SlotState::Finished(Outcome::CancelRacedWithCompletion),
        5 => SlotState::Finished(Outcome::Failed),
        6 => SlotState::Finished(Outcome::ContractViolation),
        7 => SlotState::Skipped,
        _ => return Err(Denied::Storage),
    })
}

fn plain_ancestors(path: &Path) -> Result<(), Denied> {
    for ancestor in path.ancestors() {
        let metadata = match fs::symlink_metadata(ancestor) {
            Ok(m) => m,
            Err(e) if ancestor == path && e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(_) => return Err(Denied::Storage),
        };
        if metadata.file_type().is_symlink() {
            return Err(Denied::Storage);
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if metadata.file_attributes() & 0x400 != 0 {
                return Err(Denied::Storage);
            }
        }
    }
    Ok(())
}
