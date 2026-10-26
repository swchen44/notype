//! Literal public CLI policy. No mode here reads paths or resolves credentials.
use super::probe::cli::Mode;
#[test]
fn literal_modes_reject_all_overrides_without_echoing_rejected_values() {
    for mode in ["check", "run", "report"] {
        assert!(Mode::parse(&[mode.into()]).is_ok());
    }
    for arguments in [
        vec![],
        vec!["--url".into(), "https://sentinel.invalid/private".into()],
        vec!["run".into(), "Bearer-UNIQUE-SECRET-SENTINEL".into()],
        vec!["CHECK".into()],
        vec!["".into()],
    ] {
        assert_eq!(Mode::parse(&arguments), Err("probe-arguments-unavailable"));
    }
}

#[test]
fn actual_headless_executable_rejects_bad_arguments_before_any_owned_state_write() {
    let directory = tempfile::tempdir().unwrap();
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_notype-provider-probe"));
    command
        .args(["run", "Bearer-UNIQUE-SECRET-SENTINEL"])
        .current_dir(directory.path())
        .env_clear()
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let output = command.output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).unwrap().trim(),
        "probe-arguments-unavailable"
    );
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
}
