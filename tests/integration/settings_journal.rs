#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/settings_journal.rs"]
mod settings_journal;
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/settings_journal_files.rs"]
mod settings_journal_files;
use settings_journal::*;
use settings_journal_files::Files;
use std::{cell::Cell, fs};

struct Interrupted<'a> {
    files: &'a Files,
    remaining: Cell<usize>,
}
impl Storage for Interrupted<'_> {
    fn read(&self, d: Domain) -> Result<Option<String>, Error> {
        self.files.read(d)
    }
    fn write(&self, d: Domain, text: &str) -> Result<(), Error> {
        if self.remaining.get() == 0 {
            return Err(Error::Io);
        }
        self.remaining.set(self.remaining.get() - 1);
        self.files.write(d, text)
    }
    fn read_journal(&self) -> Result<Option<Vec<u8>>, Error> {
        self.files.read_journal()
    }
    fn write_journal(&self, bytes: &[u8]) -> Result<(), Error> {
        self.files.write_journal(bytes)
    }
    fn clear_journal(&self) -> Result<(), Error> {
        self.files.clear_journal()
    }
}
#[test]
fn actual_owned_files_resume_each_crash_prefix_without_touching_other_data() {
    for prefix in 0..4 {
        let directory = tempfile::tempdir().unwrap();
        for sentinel in [
            "history.db",
            "connection-activity.db",
            "weights.gguf",
            ".env",
        ] {
            fs::write(directory.path().join(sentinel), b"unchanged").unwrap();
        }
        let files = Files::open(directory.path()).unwrap();
        assert!(matches!(Files::open(directory.path()), Err(Error::Locked)));
        let targets = [
            Domain::App,
            Domain::Providers,
            Domain::ModelRoot,
            Domain::Diagnostics,
        ]
        .into_iter()
        .map(|domain| Target {
            domain,
            before: None,
            after: "{\"文字\":\"繁體中文 English\"}".into(),
        })
        .collect();
        stage(&files, targets).unwrap();
        assert!(!directory.path().join(Domain::App.filename()).exists());
        assert_eq!(
            recover(&Interrupted {
                files: &files,
                remaining: Cell::new(prefix)
            }),
            Err(Error::Io)
        );
        drop(files);
        let reopened = Files::open(directory.path()).unwrap();
        assert_eq!(recover(&reopened).unwrap(), Recovery::Applied);
        for domain in [
            Domain::App,
            Domain::Providers,
            Domain::ModelRoot,
            Domain::Diagnostics,
        ] {
            assert_eq!(
                reopened.read(domain).unwrap().as_deref(),
                Some("{\"文字\":\"繁體中文 English\"}")
            );
        }
        for sentinel in [
            "history.db",
            "connection-activity.db",
            "weights.gguf",
            ".env",
        ] {
            assert_eq!(
                fs::read(directory.path().join(sentinel)).unwrap(),
                b"unchanged"
            );
        }
        assert!(reopened.read_journal().unwrap().is_none());
    }
}
#[test]
fn external_edit_and_publication_failure_keep_owned_previous_content() {
    let directory = tempfile::tempdir().unwrap();
    let files = Files::open(directory.path()).unwrap();
    let targets = [
        Domain::App,
        Domain::Providers,
        Domain::ModelRoot,
        Domain::Diagnostics,
    ]
    .into_iter()
    .map(|domain| Target {
        domain,
        before: None,
        after: "{}".into(),
    })
    .collect();
    stage(&files, targets).unwrap();
    fs::write(
        directory.path().join(Domain::Providers.filename()),
        b"{\"external\":true}",
    )
    .unwrap();
    assert_eq!(recover(&files), Err(Error::Conflict));
    assert!(!directory.path().join(Domain::App.filename()).exists());
    let output = directory.path().join("export.yaml");
    fs::write(&output, b"old").unwrap();
    assert_eq!(
        settings_journal_files::atomic_write(&output, b"too big", 1),
        Err(Error::Invalid)
    );
    assert_eq!(fs::read(output).unwrap(), b"old");
    assert!(files.read_journal().unwrap().is_some());
}
