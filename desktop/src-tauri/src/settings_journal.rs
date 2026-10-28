//! Confirmed fixed-domain settings transactions, recovered before runtime stores open.
//! Journal payloads may contain credentials; no payload type implements Debug.
use serde::{Deserialize, Serialize};

pub const MAX_FILE_BYTES: usize = 4_194_304;
pub const MAX_JOURNAL_BYTES: usize = 67_108_864;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Domain {
    App,
    Providers,
    ModelRoot,
    Diagnostics,
}
impl Domain {
    /// Closed App-owned destinations; the input YAML never supplies filenames.
    pub fn filename(self) -> &'static str {
        match self {
            Self::App => "settings_store.json",
            Self::Providers => "provider_connections_v2.json",
            Self::ModelRoot => "model_storage_v1.json",
            Self::Diagnostics => "diagnostics-preferences.json",
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    pub domain: Domain,
    pub before: Option<String>,
    pub after: String,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Phase {
    Prepared,
    Applying,
    Committed,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Journal {
    schema: u8,
    phase: Phase,
    targets: Vec<Target>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Invalid,
    Busy,
    Stale,
    Conflict,
    Io,
    Locked,
    NoSpace,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Invalid => "settings-transfer-journal-invalid",
            Self::Busy => "settings-transfer-busy",
            Self::Stale => "settings-transfer-stale",
            Self::Conflict => "settings-transfer-recovery-conflict",
            Self::Io => "settings-transfer-storage-failed",
            Self::Locked => "settings-transfer-locked",
            Self::NoSpace => "settings-transfer-space-unavailable",
        })
    }
}
impl std::error::Error for Error {}

/// Implementations provide atomic bounded writes under one process/OS owner.
pub trait Storage {
    /// Adapter preflight; I/O still handles competing disk consumption.
    fn available_bytes(&self) -> Result<u64, Error> {
        Ok(u64::MAX)
    }
    fn read(&self, domain: Domain) -> Result<Option<String>, Error>;
    fn write(&self, domain: Domain, text: &str) -> Result<(), Error>;
    fn read_journal(&self) -> Result<Option<Vec<u8>>, Error>;
    fn write_journal(&self, bytes: &[u8]) -> Result<(), Error>;
    fn clear_journal(&self) -> Result<(), Error>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Recovery {
    Nothing,
    Applied,
    CleanupPending,
}

/// Persist only confirmed intent; no live domain file changes here.
pub fn stage(storage: &dyn Storage, mut targets: Vec<Target>) -> Result<(), Error> {
    if storage.read_journal()?.is_some() {
        return Err(Error::Busy);
    }
    targets.sort_by_key(|target| target.domain);
    validate(&targets)?;
    for target in &targets {
        if !equivalent(
            storage.read(target.domain)?.as_deref(),
            target.before.as_deref(),
        )? {
            return Err(Error::Stale);
        }
    }
    persist(
        storage,
        &Journal {
            schema: 1,
            phase: Phase::Prepared,
            targets,
        },
    )
}

/// Execute only during startup, before any settings reader/writer is initialized.
pub fn recover(storage: &dyn Storage) -> Result<Recovery, Error> {
    let Some(bytes) = storage.read_journal()? else {
        return Ok(Recovery::Nothing);
    };
    if bytes.len() > MAX_JOURNAL_BYTES {
        return Err(Error::Invalid);
    }
    let mut journal: Journal = serde_json::from_slice(&bytes).map_err(|_| Error::Invalid)?;
    if journal.schema != 1 {
        return Err(Error::Invalid);
    }
    validate(&journal.targets)?;
    if matches!(journal.phase, Phase::Committed) {
        // A leftover cleanup marker must never overwrite later user edits.
        return Ok(if storage.clear_journal().is_ok() {
            Recovery::Nothing
        } else {
            Recovery::CleanupPending
        });
    }
    // Check every domain before writing any. Crash prefixes can contain old/new
    // values, but unrelated external edits require explicit recovery intervention.
    for target in &journal.targets {
        let current = storage.read(target.domain)?;
        if !equivalent(current.as_deref(), target.before.as_deref())?
            && !equivalent(current.as_deref(), Some(&target.after))?
        {
            return Err(Error::Conflict);
        }
    }
    journal.phase = Phase::Applying;
    persist(storage, &journal)?;
    for target in &journal.targets {
        if !equivalent(storage.read(target.domain)?.as_deref(), Some(&target.after))? {
            storage.write(target.domain, &target.after)?;
        }
    }
    for target in &journal.targets {
        if !equivalent(storage.read(target.domain)?.as_deref(), Some(&target.after))? {
            return Err(Error::Conflict);
        }
    }
    journal.phase = Phase::Committed;
    persist(storage, &journal)?;
    Ok(if storage.clear_journal().is_ok() {
        Recovery::Applied
    } else {
        Recovery::CleanupPending
    })
}

fn validate(targets: &[Target]) -> Result<(), Error> {
    let domains: Vec<_> = targets.iter().map(|t| t.domain).collect();
    if domains
        != [
            Domain::App,
            Domain::Providers,
            Domain::ModelRoot,
            Domain::Diagnostics,
        ]
    {
        return Err(Error::Invalid);
    }
    for target in targets {
        json(&target.after)?;
        if let Some(before) = &target.before {
            json(before)?;
        }
    }
    Ok(())
}
fn json(text: &str) -> Result<serde_json::Value, Error> {
    if text.len() > MAX_FILE_BYTES {
        return Err(Error::Invalid);
    }
    let value: serde_json::Value = serde_json::from_str(text).map_err(|_| Error::Invalid)?;
    if !value.is_object() {
        return Err(Error::Invalid);
    }
    Ok(value)
}
fn equivalent(left: Option<&str>, right: Option<&str>) -> Result<bool, Error> {
    match (left, right) {
        (None, None) => Ok(true),
        (Some(a), Some(b)) => Ok(json(a)? == json(b)?),
        _ => Ok(false),
    }
}
fn persist(storage: &dyn Storage, journal: &Journal) -> Result<(), Error> {
    let bytes = serde_json::to_vec(journal).map_err(|_| Error::Invalid)?;
    if bytes.len() > MAX_JOURNAL_BYTES {
        return Err(Error::Invalid);
    }
    let replacement_bytes: usize = journal
        .targets
        .iter()
        .map(|target| target.after.len())
        .sum();
    let required = bytes.len() as u64 * 2 + replacement_bytes as u64 + MAX_FILE_BYTES as u64;
    if storage.available_bytes()? < required {
        return Err(Error::NoSpace);
    }
    storage.write_journal(&bytes)
}
