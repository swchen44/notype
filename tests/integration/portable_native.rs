use notype_platform::validate_application;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::fs;
use tempfile::TempDir;

fn fixture() -> TempDir {
    let root = TempDir::new().unwrap();
    fs::create_dir(root.path().join("app")).unwrap();
    fs::create_dir(root.path().join("runtime")).unwrap();
    fs::write(root.path().join("app/notype.exe"), b"fixture-app").unwrap();
    fs::write(
        root.path().join("runtime/msedgewebview2.exe"),
        b"fixture-browser",
    )
    .unwrap();
    let files = [("app/notype.exe", "entrypoint", b"fixture-app".as_slice()), ("runtime/msedgewebview2.exe", "webview-entry", b"fixture-browser".as_slice())].iter().map(|(path, role, bytes)| json!({"path":path,"role":role,"bytes":bytes.len(),"sha256":format!("{:x}",Sha256::digest(bytes))})).collect::<Vec<_>>();
    fs::write(root.path().join("payload.json"), serde_json::to_vec(&json!({"schema":1,"role":"application","platform":"win32","arch":"x64","cpu":"x86-64","entrypoint":"app/notype.exe","webview":{"directory":"runtime","version":"156.0.4314.42","entrypoint":"runtime/msedgewebview2.exe"},"files":files})).unwrap()).unwrap();
    root
}

#[test]
fn direct_native_preflight_selects_only_complete_verified_fixed_folder_without_creating_data() {
    let root = fixture();
    let result = validate_application(root.path(), &root.path().join("app/notype.exe")).unwrap();
    assert_eq!(result.browser_directory, root.path().join("runtime"));
    assert!(!root.path().join("sessions.db").exists());
    fs::remove_file(root.path().join("runtime/msedgewebview2.exe")).unwrap();
    assert_eq!(
        validate_application(root.path(), &root.path().join("app/notype.exe"))
            .unwrap_err()
            .code(),
        "runtime-file-missing"
    );
}

#[test]
fn standalone_baseline_probe_reports_actual_capabilities_without_runtime_or_data_initialization() {
    let directory = TempDir::new().unwrap();
    let result = std::process::Command::new(env!("CARGO_BIN_EXE_notype-platform-check"))
        .current_dir(directory.path())
        .output()
        .unwrap();
    assert!(result.status.success());
    let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["schema"], 1);
    assert!(report["cpu"]["avx2"].is_boolean());
    assert!(report["cpu"]["bmi2"].is_boolean());
    assert!(["medium", "unsupported"].contains(&report["integrity"].as_str().unwrap()));
    assert!(fs::read_dir(directory.path()).unwrap().next().is_none());
}

#[test]
fn directly_invoked_native_manifest_refuses_corrupt_runtime_and_unlisted_files() {
    let root = fixture();
    fs::write(
        root.path().join("runtime/msedgewebview2.exe"),
        b"fixture-BROWSER",
    )
    .unwrap();
    assert_eq!(
        validate_application(root.path(), &root.path().join("app/notype.exe"))
            .unwrap_err()
            .code(),
        "runtime-file-corrupt"
    );
    fs::write(
        root.path().join("runtime/msedgewebview2.exe"),
        b"fixture-browser",
    )
    .unwrap();
    fs::write(root.path().join("unexpected.dll"), b"not approved").unwrap();
    assert_eq!(
        validate_application(root.path(), &root.path().join("app/notype.exe"))
            .unwrap_err()
            .code(),
        "runtime-file-unexpected"
    );
}
