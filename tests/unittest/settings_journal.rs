#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/settings_journal.rs"]
mod settings_journal;
use settings_journal::*;
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
};

#[derive(Default)]
struct Memory {
    files: RefCell<BTreeMap<Domain, String>>,
    journal: RefCell<Option<Vec<u8>>>,
    writes: Cell<usize>,
    fail_at: Cell<Option<usize>>,
    fail_clear: Cell<bool>,
    available: Cell<Option<u64>>,
}
impl Storage for Memory {
    fn available_bytes(&self) -> Result<u64, Error> {
        Ok(self.available.get().unwrap_or(u64::MAX))
    }
    fn read(&self, d: Domain) -> Result<Option<String>, Error> {
        Ok(self.files.borrow().get(&d).cloned())
    }
    fn write(&self, d: Domain, t: &str) -> Result<(), Error> {
        let n = self.writes.get();
        self.writes.set(n + 1);
        if self.fail_at.get() == Some(n) {
            return Err(Error::Io);
        }
        self.files.borrow_mut().insert(d, t.into());
        Ok(())
    }
    fn read_journal(&self) -> Result<Option<Vec<u8>>, Error> {
        Ok(self.journal.borrow().clone())
    }
    fn write_journal(&self, b: &[u8]) -> Result<(), Error> {
        *self.journal.borrow_mut() = Some(b.to_vec());
        Ok(())
    }
    fn clear_journal(&self) -> Result<(), Error> {
        if self.fail_clear.get() {
            return Err(Error::Io);
        }
        *self.journal.borrow_mut() = None;
        Ok(())
    }
}

#[test]
fn insufficient_space_never_stages_or_changes_any_domain() {
    let memory = Memory::default();
    memory.available.set(Some(0));
    assert_eq!(stage(&memory, targets()), Err(Error::NoSpace));
    assert!(memory.files.borrow().is_empty());
    assert!(memory.read_journal().unwrap().is_none());
}
fn targets() -> Vec<Target> {
    [
        Domain::App,
        Domain::Providers,
        Domain::ModelRoot,
        Domain::Diagnostics,
    ]
    .into_iter()
    .map(|domain| Target {
        domain,
        before: None,
        after: format!("{{\"field\":\"{domain:?}\"}}"),
    })
    .collect()
}

#[test]
fn stage_changes_no_domains_and_recovery_applies_the_confirmed_whole_state() {
    let memory = Memory::default();
    let expected = targets();
    stage(&memory, expected.clone()).unwrap();
    assert!(memory.files.borrow().is_empty());
    assert_eq!(recover(&memory).unwrap(), Recovery::Applied);
    for target in expected {
        assert_eq!(
            memory.read(target.domain).unwrap().as_deref(),
            Some(target.after.as_str())
        );
    }
    assert!(memory.read_journal().unwrap().is_none());
    assert_eq!(recover(&memory).unwrap(), Recovery::Nothing);
}

#[test]
fn every_interrupted_write_prefix_recovers_and_unknown_changes_fail_closed() {
    for prefix in 0..4 {
        let memory = Memory::default();
        stage(&memory, targets()).unwrap();
        memory.fail_at.set(Some(prefix));
        assert_eq!(recover(&memory).unwrap_err(), Error::Io);
        assert!(memory.read_journal().unwrap().is_some());
        memory.fail_at.set(None);
        assert_eq!(recover(&memory).unwrap(), Recovery::Applied);
        assert_eq!(memory.files.borrow().len(), 4);
    }
    let memory = Memory::default();
    stage(&memory, targets()).unwrap();
    memory
        .files
        .borrow_mut()
        .insert(Domain::Providers, "{\"external\":true}".into());
    assert_eq!(recover(&memory).unwrap_err(), Error::Conflict);
    assert_eq!(memory.writes.get(), 0);
    assert!(memory.read_journal().unwrap().is_some());
}

#[test]
fn stale_or_invalid_stage_is_no_write_and_committed_cleanup_never_replays() {
    let memory = Memory::default();
    memory
        .files
        .borrow_mut()
        .insert(Domain::App, "{\"changed\":true}".into());
    assert_eq!(stage(&memory, targets()).unwrap_err(), Error::Stale);
    assert!(memory.read_journal().unwrap().is_none());
    assert_eq!(memory.writes.get(), 0);
    let mut duplicate = targets();
    duplicate[1].domain = Domain::App;
    assert_eq!(
        stage(&Memory::default(), duplicate).unwrap_err(),
        Error::Invalid
    );
    let memory = Memory::default();
    stage(&memory, targets()).unwrap();
    memory.fail_clear.set(true);
    assert_eq!(recover(&memory).unwrap(), Recovery::CleanupPending);
    let writes = memory.writes.get();
    memory
        .files
        .borrow_mut()
        .insert(Domain::App, "{\"later_user_edit\":true}".into());
    memory.fail_clear.set(false);
    assert_eq!(recover(&memory).unwrap(), Recovery::Nothing);
    assert_eq!(memory.writes.get(), writes);
    assert_eq!(
        memory.read(Domain::App).unwrap().as_deref(),
        Some("{\"later_user_edit\":true}")
    );
}
