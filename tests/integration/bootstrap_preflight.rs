#[allow(dead_code, unused_imports)]
#[path = "../../crates/notype-platform/src/lib.rs"]
mod platform;
use std::{fs, path::Path};
fn inventory(root: &Path) -> Vec<(std::path::PathBuf, Vec<u8>)> {
    let mut result = Vec::new();
    for entry in fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            result.extend(inventory(&path));
        } else {
            result.push((
                path.strip_prefix(root).unwrap().to_path_buf(),
                fs::read(path).unwrap(),
            ));
        }
    }
    result.sort();
    result
}
#[test]
fn full_bundle_placement_is_rejected_before_any_dataset_or_log_mutation() {
    let owned = tempfile::tempdir().unwrap();
    let root = owned.path().join("bundle");
    let exe = root.join("app/node_modules/notype-win32-x64/app/notype.exe");
    fs::create_dir_all(exe.parent().unwrap()).unwrap();
    fs::write(&exe, b"synthetic-native").unwrap();
    for folder in ["models", "runtime"] {
        fs::create_dir(root.join(folder)).unwrap();
    }
    fs::write(root.join("models/sentinel"), b"untouched").unwrap();
    let before = inventory(&root);
    for forbidden in [
        root.join("models/new-data"),
        root.join("runtime/new-data"),
        root.join("app/new-data"),
        owned.path().to_path_buf(),
    ] {
        let result = platform::preflight_offline_placement(&exe, &root, Some(&forbidden), None);
        assert!(
            result.is_err(),
            "entire bundle and its ancestors must be excluded"
        );
        assert_eq!(inventory(&root), before);
        assert!(!root.join("models/new-data").exists());
    }
    assert!(
        platform::preflight_offline_placement(&exe, &root, None, Some(&root.join("models")))
            .is_err()
    );
    assert!(platform::preflight_offline_placement(
        &exe,
        &root,
        Some(&owned.path().join("sibling-data")),
        None
    )
    .is_ok());
    assert_eq!(inventory(&root), before);
}
