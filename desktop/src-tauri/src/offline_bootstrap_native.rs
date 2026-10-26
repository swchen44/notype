//! Native composition for the nonsecret bundle coordinator.
use crate::{
    bootstrap_selection::SelectionStore,
    managers::model::ModelManager,
    offline_bootstrap::{self, BootstrapInput, BootstrapStatus, SttPreset},
    provider_config::{self, ConfigError, ConfigurationStorage},
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use tauri::{AppHandle, Manager};

pub struct NativeBootstrap {
    root: Option<PathBuf>,
    operation: Mutex<()>,
}
#[derive(Deserialize)]
struct Pin {
    name: String,
    version: String,
    manifest: String,
    sha256: String,
}
#[derive(Deserialize)]
struct Model {
    model_id: String,
    root: String,
}
#[derive(Deserialize)]
struct BootstrapReference {
    path: String,
    sha256: String,
}
#[derive(Deserialize)]
struct Bundle {
    schema: u8,
    mode: String,
    id: String,
    version: String,
    source: String,
    application: Pin,
    model: Model,
    bootstrap: Option<BootstrapReference>,
}

fn read(path: &Path) -> Result<Vec<u8>, ConfigError> {
    provider_config::files::FileStorage::new(path.into())
        .load()?
        .ok_or(ConfigError("bootstrap-package-invalid"))
}
fn digest(path: &Path, expected: &str) -> Result<Vec<u8>, ConfigError> {
    if expected.len() != 64
        || !expected
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err(ConfigError("bootstrap-package-invalid"));
    }
    let bytes = read(path)?;
    if format!("{:x}", Sha256::digest(&bytes)) != expected {
        return Err(ConfigError("bootstrap-package-invalid"));
    }
    Ok(bytes)
}
fn input(root: &Path) -> Result<BootstrapInput, ConfigError> {
    if !root.is_absolute() {
        return Err(ConfigError("bootstrap-package-invalid"));
    }
    #[cfg(windows)]
    {
        let text = root.to_string_lossy();
        let bytes = text.as_bytes();
        if bytes.len() < 3
            || !bytes[0].is_ascii_alphabetic()
            || bytes[1] != b':'
            || !matches!(bytes[2], b'/' | b'\\')
        {
            return Err(ConfigError("bootstrap-package-invalid"));
        }
    }
    provider_config::files::plain(root)?;
    let root = std::fs::canonicalize(root).map_err(|_| ConfigError("bootstrap-package-invalid"))?;
    let expected = root.join("app/node_modules/notype-win32-x64/app/notype.exe");
    provider_config::files::plain(&expected)?;
    let expected =
        std::fs::canonicalize(expected).map_err(|_| ConfigError("bootstrap-package-invalid"))?;
    let actual = std::env::current_exe()
        .and_then(std::fs::canonicalize)
        .map_err(|_| ConfigError("bootstrap-package-invalid"))?;
    if expected != actual {
        return Err(ConfigError("bootstrap-package-invalid"));
    }
    let bundle: Bundle = serde_json::from_slice(&read(&root.join("bundle.json"))?)
        .map_err(|_| ConfigError("bootstrap-package-invalid"))?;
    if bundle.schema != 1
        || bundle.mode != "offline-bundle"
        || bundle.model.root != "models"
        || bundle.source.len() != 40
        || !bundle.source.bytes().all(|b| b.is_ascii_hexdigit())
        || bundle.application.name != "notype-win32-x64"
        || bundle.application.manifest != "payload.json"
        || bundle.application.version != bundle.version
    {
        return Err(ConfigError("bootstrap-package-invalid"));
    }
    digest(
        &root.join("app/node_modules/notype-win32-x64/payload.json"),
        &bundle.application.sha256,
    )?;
    let input = match bundle.bootstrap {
        Some(reference) => {
            if reference.path != "bootstrap/notype-bootstrap.json" {
                return Err(ConfigError("bootstrap-package-invalid"));
            }
            BootstrapInput::parse(&digest(&root.join(reference.path), &reference.sha256)?)?
        }
        None => BootstrapInput {
            schema: 1,
            bootstrap_id: bundle.id,
            stt: SttPreset {
                model_id: bundle.model.model_id.clone(),
                root: bundle.model.root.clone(),
            },
            connection: None,
        },
    };
    if input.stt.model_id != bundle.model.model_id || input.stt.root != bundle.model.root {
        return Err(ConfigError("bootstrap-package-invalid"));
    }
    input.validate_envelope()?;
    Ok(input)
}
pub fn selection_store(app: &AppHandle) -> Result<SelectionStore, ConfigError> {
    let path = crate::portable::app_data_dir(app)
        .map_err(|_| ConfigError("bootstrap-settings-invalid"))?
        .join(crate::settings::SETTINGS_STORE_PATH);
    Ok(SelectionStore::new(Arc::new(
        provider_config::files::FileStorage::new(path),
    )))
}
/// Must be called before WebView, headless managers, settings reads or workers.
pub fn prepare(app: &AppHandle, root: Option<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    if root.is_some() {
        offline_bootstrap::begin();
    }
    let manager = Arc::new(ModelManager::new(app)?);
    app.manage(manager);
    app.manage(NativeBootstrap {
        root,
        operation: Mutex::new(()),
    });
    retry(app);
    Ok(())
}
pub fn retry(app: &AppHandle) -> BootstrapStatus {
    if !offline_bootstrap::blocked() {
        return offline_bootstrap::status();
    }
    let native = app.state::<NativeBootstrap>();
    let Ok(_operation) = native.operation.try_lock() else {
        return offline_bootstrap::status();
    };
    let Some(root) = native.root.as_ref() else {
        return offline_bootstrap::finish(Ok(()));
    };
    offline_bootstrap::begin();
    let result = (|| {
        let input = input(root)?;
        let data = crate::portable::app_data_dir(app)
            .map_err(|_| ConfigError("bootstrap-settings-invalid"))?;
        notype_platform::validate_external_data_directory(root, &data)
            .map_err(|_| ConfigError("bootstrap-package-invalid"))?;
        let selection = selection_store(app)?;
        if let Some(value) = selection.read()? {
            serde_json::from_value::<crate::settings::AppSettings>(value)
                .map_err(|_| ConfigError("bootstrap-settings-invalid"))?;
        }
        let manager = app.state::<Arc<ModelManager>>();
        if selection.pending()?
            && manager.roots.initialized()
            && manager.roots.projection().root_identity.is_none()
        {
            manager
                .roots
                .rescan("bootstrap_recovery", &|_, _| {})
                .map_err(|_| ConfigError("bootstrap-model-unavailable"))?;
        }
        let provider = crate::commands::history::configuration_store(app)
            .map_err(|_| ConfigError("configuration-storage-failed"))?;
        offline_bootstrap::initialize(
            &input,
            &root.join("models"),
            &manager.roots,
            &provider,
            &selection,
            serde_json::to_value(crate::settings::get_default_settings())
                .map_err(|_| ConfigError("bootstrap-settings-invalid"))?,
        )
    })();
    let outcome = if result.is_ok() {
        crate::diagnostics::Outcome::Ok
    } else {
        crate::diagnostics::Outcome::Failed
    };
    crate::diagnostics::record(crate::diagnostics::Phase::Startup, outcome, None);
    offline_bootstrap::finish(result)
}
#[tauri::command]
#[specta::specta]
pub fn get_bootstrap_status() -> BootstrapStatus {
    offline_bootstrap::status()
}
#[tauri::command]
#[specta::specta]
pub fn retry_offline_bootstrap(app: AppHandle) -> Result<BootstrapStatus, String> {
    let status = retry(&app);
    if status.state == "ready" {
        crate::initialize_core_logic(&app)
            .map_err(|_| "bootstrap-runtime-unavailable".to_owned())?;
    }
    Ok(offline_bootstrap::status())
}
