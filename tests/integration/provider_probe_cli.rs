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
