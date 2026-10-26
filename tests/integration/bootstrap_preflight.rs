#[allow(dead_code, unused_imports)]
#[path = "../../crates/notype-platform/src/lib.rs"]
mod platform;
use std::{fs, path::Path};
fn inventory(root: &Path) -> Vec<(std::path::PathBuf, Option<Vec<u8>>)> {
    fn walk(root: &Path, current: &Path, result: &mut Vec<(std::path::PathBuf, Option<Vec<u8>>)>) {
        for entry in fs::read_dir(current).unwrap() {
            let path = entry.unwrap().path();
            let relative = path.strip_prefix(root).unwrap().to_path_buf();
            if path.is_dir() {
                result.push((relative, None));
                walk(root, &path, result);
            } else {
                result.push((relative, Some(fs::read(path).unwrap())));
            }
        }
    }
    let mut result = Vec::new();
    walk(root, root, &mut result);
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

#[test]
fn captured_known_folder_context_rejects_relative_and_wrong_executable_inputs() {
    let owned = tempfile::tempdir().unwrap();
    let root = owned.path().join("bundle");
    let exe = root.join("app/node_modules/notype-win32-x64/app/notype.exe");
    fs::create_dir_all(exe.parent().unwrap()).unwrap();
    fs::write(&exe, b"synthetic-native").unwrap();
    let known = owned.path().join("known-folder");
    let checked = platform::preflight_offline_placement(&exe, &root, None, Some(&known)).unwrap();
    assert_eq!(checked.bundle_root(), root.as_path());
    assert_eq!(
        checked.data_directory(),
        known.join("io.github.swchen44.notype").as_path()
    );
    assert!(!known.exists());
    let before = inventory(&root);
    assert!(platform::preflight_offline_placement(
        &exe,
        Path::new("relative-bundle"),
        Some(&known),
        None
    )
    .is_err());
    assert!(platform::preflight_offline_placement(
        &exe,
        &root,
        Some(Path::new("relative-data")),
        None
    )
    .is_err());
    let other = owned.path().join("other.exe");
    fs::write(&other, b"other-native").unwrap();
    assert!(platform::preflight_offline_placement(&other, &root, Some(&known), None).is_err());
    assert_eq!(inventory(&root), before);
    assert!(!known.exists());
}

#[cfg(unix)]
#[test]
fn original_link_ancestry_is_rejected_before_canonicalization_hides_it() {
    use std::os::unix::fs::symlink;
    let owned = tempfile::tempdir().unwrap();
    let root = owned.path().join("bundle");
    let exe = root.join("app/node_modules/notype-win32-x64/app/notype.exe");
    fs::create_dir_all(exe.parent().unwrap()).unwrap();
    fs::write(&exe, b"synthetic-native").unwrap();
    let outside = owned.path().join("outside");
    fs::create_dir(&outside).unwrap();
    let data_alias = owned.path().join("data-alias");
    symlink(&outside, &data_alias).unwrap();
    let bundle_alias = owned.path().join("bundle-alias");
    symlink(&root, &bundle_alias).unwrap();
    let before = inventory(&root);
    assert!(platform::preflight_offline_placement(
        &exe,
        &root,
        Some(&data_alias.join("new-data")),
        None
    )
    .is_err());
    assert!(platform::preflight_offline_placement(
        &bundle_alias.join("app/node_modules/notype-win32-x64/app/notype.exe"),
        &bundle_alias,
        Some(&outside),
        None
    )
    .is_err());
    assert!(!outside.join("new-data").exists());
    assert_eq!(inventory(&root), before);
}
