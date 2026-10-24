#[path = "../../desktop/src-tauri/src/data_policy.rs"]
mod data_policy;

use std::path::Path;

#[test]
fn settings_use_the_stable_per_user_root() {
    let root = std::env::temp_dir().join("notype-profile");
    assert_eq!(data_policy::resolve_data_dir(&root, None).unwrap(), root);
}

#[test]
fn a_clean_data_directory_can_be_selected_for_a_runtime_receipt() {
    let default = std::env::temp_dir().join("notype-profile");
    let selected = std::env::temp_dir().join("notype-isolated-receipt");
    assert_eq!(
        data_policy::resolve_data_dir(&default, Some(&selected)).unwrap(),
        selected
    );
}

#[test]
fn relative_override_cannot_put_settings_into_the_program_directory() {
    let default = std::env::temp_dir().join("notype-profile");
    assert!(data_policy::resolve_data_dir(&default, Some(Path::new("Data"))).is_err());
}
