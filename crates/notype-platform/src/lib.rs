//! Platform launch policy without Tauri, speech engines or native runtime imports.
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
mod host;
pub use host::{host_report, HostReport};

#[derive(Debug)]
pub struct PreflightError(&'static str);
impl PreflightError {
    pub fn code(&self) -> &'static str {
        self.0
    }
}
impl std::fmt::Display for PreflightError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: notype portable files are missing, invalid or unsupported; use the complete verified local package", self.0)
    }
}
impl std::error::Error for PreflightError {}
#[derive(Debug)]
pub struct RuntimePlan {
    pub browser_directory: PathBuf,
    pub cpu: String,
}
#[derive(Deserialize)]
struct Manifest {
    schema: u32,
    role: String,
    platform: String,
    arch: String,
    cpu: String,
    entrypoint: String,
    webview: Webview,
    files: Vec<File>,
}
#[derive(Deserialize)]
struct Webview {
    directory: String,
    version: String,
    entrypoint: String,
}
#[derive(Deserialize)]
struct File {
    path: String,
    role: String,
    bytes: u64,
    sha256: String,
}

fn relative(value: &str) -> Result<(), PreflightError> {
    if value.len() > 240 || value.contains('\\') {
        return Err(PreflightError("runtime-manifest-invalid"));
    }
    for segment in value.split('/') {
        let stem = segment.split('.').next().unwrap_or("").to_ascii_lowercase();
        if segment.is_empty()
            || segment == "."
            || segment == ".."
            || segment.ends_with(['.', ' '])
            || segment
                .chars()
                .any(|c| c.is_control() || "<>:\"|?*".contains(c))
            || ["con", "prn", "aux", "nul"].contains(&stem.as_str())
            || (stem.len() == 4
                && (stem.starts_with("com") || stem.starts_with("lpt"))
                && matches!(stem.as_bytes()[3], b'1'..=b'9'))
        {
            return Err(PreflightError("runtime-manifest-invalid"));
        }
    }
    Ok(())
}
fn reparse(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}
fn regular(root: &Path, value: &str) -> Result<fs::Metadata, PreflightError> {
    relative(value)?;
    let mut current = root.to_path_buf();
    let root_meta =
        fs::symlink_metadata(root).map_err(|_| PreflightError("runtime-file-missing"))?;
    if !root_meta.is_dir() || reparse(&root_meta) {
        return Err(PreflightError("runtime-file-unsafe"));
    }
    let parts: Vec<_> = value.split('/').collect();
    for (index, part) in parts.iter().enumerate() {
        current.push(part);
        let metadata =
            fs::symlink_metadata(&current).map_err(|_| PreflightError("runtime-file-missing"))?;
        if reparse(&metadata)
            || (index + 1 < parts.len() && !metadata.is_dir())
            || (index + 1 == parts.len() && !metadata.is_file())
        {
            return Err(PreflightError("runtime-file-unsafe"));
        }
    }
    fs::metadata(current).map_err(|_| PreflightError("runtime-file-missing"))
}
fn inventory(
    root: &Path,
    current: &Path,
    files: &mut HashSet<String>,
) -> Result<(), PreflightError> {
    for entry in fs::read_dir(current).map_err(|_| PreflightError("runtime-file-missing"))? {
        let entry = entry.map_err(|_| PreflightError("runtime-file-missing"))?;
        let full = entry.path();
        let metadata =
            fs::symlink_metadata(&full).map_err(|_| PreflightError("runtime-file-missing"))?;
        if reparse(&metadata) {
            return Err(PreflightError("runtime-file-unsafe"));
        }
        if metadata.is_dir() {
            inventory(root, &full, files)?;
        } else if metadata.is_file() {
            let value = full
                .strip_prefix(root)
                .map_err(|_| PreflightError("runtime-file-unsafe"))?
                .to_str()
                .ok_or(PreflightError("runtime-file-unsafe"))?
                .replace('\\', "/");
            relative(&value)?;
            files.insert(value);
        } else {
            return Err(PreflightError("runtime-file-unsafe"));
        }
    }
    Ok(())
}

/// Validate a directly invoked portable application's fixed runtime before
/// initializing Tauri, persistent data, tray, hotkeys or any WebView.
pub fn validate_application(root: &Path, executable: &Path) -> Result<RuntimePlan, PreflightError> {
    let metadata = regular(root, "payload.json")?;
    if metadata.len() > 4 * 1024 * 1024 {
        return Err(PreflightError("runtime-manifest-invalid"));
    }
    let manifest: Manifest = serde_json::from_slice(
        &fs::read(root.join("payload.json")).map_err(|_| PreflightError("runtime-file-missing"))?,
    )
    .map_err(|_| PreflightError("runtime-manifest-invalid"))?;
    if manifest.schema != 1
        || manifest.role != "application"
        || manifest.platform != "win32"
        || manifest.arch != "x64"
        || !["x86-64", "avx2-bmi2"].contains(&manifest.cpu.as_str())
        || manifest.files.is_empty()
        || manifest.files.len() > 10000
    {
        return Err(PreflightError("runtime-manifest-invalid"));
    }
    relative(&manifest.entrypoint)?;
    relative(&manifest.webview.directory)?;
    if manifest.webview.entrypoint != format!("{}/msedgewebview2.exe", manifest.webview.directory)
        || manifest.webview.version.split('.').count() != 4
        || manifest
            .webview
            .version
            .split('.')
            .any(|v| v.is_empty() || !v.chars().all(|c| c.is_ascii_digit()))
    {
        return Err(PreflightError("runtime-manifest-invalid"));
    }
    if fs::canonicalize(executable).ok() != fs::canonicalize(root.join(&manifest.entrypoint)).ok() {
        return Err(PreflightError("runtime-executable-mismatch"));
    }
    let mut declared = HashSet::new();
    let mut expected = HashSet::from(["payload.json".to_string(), "package.json".to_string()]);
    let mut app = false;
    let mut browser = false;
    for file in &manifest.files {
        relative(&file.path)?;
        if !declared.insert(file.path.to_lowercase())
            || file.bytes == 0
            || file.sha256.len() != 64
            || !file
                .sha256
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
            || ["payload.json", "package.json"].contains(&file.path.as_str())
        {
            return Err(PreflightError("runtime-manifest-invalid"));
        }
        let metadata = regular(root, &file.path)?;
        if metadata.len() != file.bytes {
            return Err(PreflightError("runtime-file-corrupt"));
        }
        let mut reader = fs::File::open(root.join(&file.path))
            .map_err(|_| PreflightError("runtime-file-missing"))?;
        let mut hash = Sha256::new();
        let mut buffer = [0u8; 65536];
        loop {
            let count = reader
                .read(&mut buffer)
                .map_err(|_| PreflightError("runtime-file-missing"))?;
            if count == 0 {
                break;
            }
            hash.update(&buffer[..count]);
        }
        if format!("{:x}", hash.finalize()) != file.sha256 {
            return Err(PreflightError("runtime-file-corrupt"));
        }
        app |= file.path == manifest.entrypoint && file.role == "entrypoint";
        browser |= file.path == manifest.webview.entrypoint && file.role == "webview-entry";
        expected.insert(file.path.clone());
    }
    if !app || !browser {
        return Err(PreflightError("runtime-manifest-invalid"));
    }
    let mut actual = HashSet::new();
    inventory(root, root, &mut actual)?;
    if !actual.is_subset(&expected) {
        return Err(PreflightError("runtime-file-unexpected"));
    }
    Ok(RuntimePlan {
        browser_directory: root.join(manifest.webview.directory),
        cpu: manifest.cpu,
    })
}

#[derive(Debug)]
pub struct PackagedRuntime {
    pub browser_directory: PathBuf,
    pub user_data_directory: PathBuf,
}

/// Native release entry-point guard. This returns a plan without initializing
/// any persistent data or app services. The executable always lives in app/.
pub fn prepare_packaged_process(
    executable: &Path,
    data_override: Option<&Path>,
) -> Result<PackagedRuntime, PreflightError> {
    let root = executable
        .parent()
        .and_then(Path::parent)
        .ok_or(PreflightError("runtime-package-missing"))?;
    let runtime = validate_application(root, executable)?;
    let host = host_report();
    if host.platform != "win32" || host.arch != "x64" {
        return Err(PreflightError("unsupported-platform"));
    }
    if host.integrity != "medium" {
        return Err(PreflightError("normal-user-required"));
    }
    if runtime.cpu == "avx2-bmi2" && (!host.cpu.avx2 || !host.cpu.bmi2) {
        return Err(PreflightError("unsupported-cpu"));
    }
    let data = match data_override {
        Some(path) => path.to_path_buf(),
        None => host
            .roaming_app_data
            .ok_or(PreflightError("data-directory-unavailable"))?
            .join("io.github.swchen44.notype"),
    };
    validate_external_data_directory(root, &data)?;
    Ok(PackagedRuntime {
        browser_directory: runtime.browser_directory,
        user_data_directory: data.join("webview"),
    })
}

/// Validate persistent storage against versioned npm package directories without creating it.
/// Existing ancestors are canonicalized so missing descendants and directory aliases cannot bypass policy.
pub fn validate_external_data_directory(root: &Path, data: &Path) -> Result<(), PreflightError> {
    if !data.is_absolute() || data.to_string_lossy().starts_with("\\\\") {
        return Err(PreflightError("invalid-data-directory"));
    }
    let mut ancestor = data;
    let mut missing = Vec::new();
    let resolved = loop {
        match fs::canonicalize(ancestor) {
            Ok(mut path) => {
                for part in missing.iter().rev() {
                    path.push(part);
                }
                break path;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                missing.push(
                    ancestor
                        .file_name()
                        .ok_or(PreflightError("invalid-data-directory"))?
                        .to_os_string(),
                );
                ancestor = ancestor
                    .parent()
                    .ok_or(PreflightError("invalid-data-directory"))?;
            }
            Err(_) => return Err(PreflightError("invalid-data-directory")),
        }
    };
    let package = fs::canonicalize(root).map_err(|_| PreflightError("runtime-package-missing"))?;
    let mut package_roots = vec![package.clone()];
    let mut containers = vec![package
        .parent()
        .ok_or(PreflightError("runtime-package-missing"))?];
    containers.extend(package.ancestors().filter(|ancestor| {
        ancestor
            .file_name()
            .is_some_and(|name| name == "node_modules")
    }));
    for container in containers {
        for role in ["notype", "notype-model", "notype-win32-x64"] {
            let candidate = container.join(role);
            let path = match fs::canonicalize(&candidate) {
                Ok(path) => path,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => candidate,
                Err(_) => return Err(PreflightError("invalid-data-directory")),
            };
            package_roots.push(path);
        }
    }
    for package in package_roots {
        #[cfg(windows)]
        let (resolved, package) = (
            PathBuf::from(resolved.to_string_lossy().to_lowercase()),
            PathBuf::from(package.to_string_lossy().to_lowercase()),
        );
        if resolved.starts_with(&package) || package.starts_with(&resolved) {
            return Err(PreflightError("invalid-data-directory"));
        }
    }
    Ok(())
}
