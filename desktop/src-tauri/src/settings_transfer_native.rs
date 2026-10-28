//! Native transfer owns dialogs, secret-capable snapshots and restart handoff.
use crate::{
    commands::history::configuration_store,
    diagnostics,
    managers::{model::ModelManager, model_root_activity::IdleTransition, model_roots_files},
    settings::{self, AppSettings},
    settings_journal::{self, Domain, Storage, Target},
    settings_journal_files::{self as files, Files},
    settings_transfer::{self, Archive},
    settings_yaml,
};
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use specta::Type;
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
};
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_store::StoreExt;

static PENDING: AtomicBool = AtomicBool::new(false);
static SERIAL: Mutex<()> = Mutex::new(());
static NEXT: AtomicU64 = AtomicU64::new(1);
static PREVIEW: Mutex<Option<Preview>> = Mutex::new(None);
// Keeps recording/model admissions closed through normal teardown.
static FROZEN: Mutex<Option<IdleTransition>> = Mutex::new(None);
// All runtime owners, including headless instances, share one OS ownership lock.
static RUNTIME_FILES: Mutex<Option<Files>> = Mutex::new(None);
pub fn pending() -> bool {
    PENDING.load(Ordering::SeqCst)
}
pub fn command_guard() -> std::sync::MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(|e| e.into_inner())
}
pub fn activity(
    app: &AppHandle,
) -> Result<crate::managers::model_root_activity::ActivityLease, String> {
    ensure_ready()?;
    app.state::<Arc<ModelManager>>()
        .roots
        .reserve()
        .map_err(|_| "settings-transfer-busy".into())
}

struct Preview {
    id: String,
    path: PathBuf,
    file_hash: String,
    state_hash: String,
    archive: Archive,
    disk: settings_transfer::DiskStamp,
}
#[derive(Serialize, Type)]
pub struct TransferPreview {
    pub id: String,
    pub before_yaml: String,
    pub after_yaml: String,
    pub changed_fields: Vec<String>,
    pub key_effects: Vec<KeyEffect>,
    pub conflicts: Vec<String>,
}
#[derive(Serialize, Type)]
pub struct KeyEffect {
    pub name: String,
    pub effect: String,
}
#[derive(Serialize, Type)]
pub struct TransferOutcome {
    pub status: String,
}
fn outcome(status: &str) -> TransferOutcome {
    TransferOutcome {
        status: status.into(),
    }
}
fn root(app: &AppHandle) -> Result<PathBuf, String> {
    crate::portable::app_data_dir(app).map_err(|_| "settings-transfer-storage-failed".into())
}
fn failure(error: settings_journal::Error) -> String {
    error.to_string()
}
fn ensure_ready() -> Result<(), String> {
    crate::offline_bootstrap::admit()?;
    if pending() {
        Err("settings-transfer-busy".into())
    } else {
        Ok(())
    }
}
fn snapshot(app: &AppHandle, keys: bool) -> Result<Archive, String> {
    let status = diagnostics::service::global().status();
    if !status.initialized || !matches!(status.persistence, "saved" | "default") {
        return Err("settings-transfer-busy".into());
    }
    let archive = Archive {
        notype_settings: 1,
        settings: settings_transfer::project_settings(
            serde_json::to_value(settings::get_settings(app))
                .map_err(|_| "settings-transfer-invalid")?,
        )?,
        ai: configuration_store(app)?
            .archive(keys)
            .map_err(|_| "settings-transfer-invalid")?,
        model_folder: model_roots_files::display_path(
            &app.state::<Arc<ModelManager>>().roots.snapshot().root,
        ),
        diagnostics_enabled: status.saved_preference.ok_or("settings-transfer-busy")?,
    };
    archive.validate()?;
    Ok(archive)
}
fn state_hash(app: &AppHandle) -> Result<String, String> {
    Ok(settings_transfer::digest(&json!({
        "archive":serde_json::to_value(snapshot(app,true)?).map_err(|_|"settings-transfer-invalid")?,
        "provider_revision":configuration_store(app)?.view().map_err(|_|"settings-transfer-invalid")?.revision,
        "root_revision":app.state::<Arc<ModelManager>>().roots.snapshot().revision,
        "diagnostics_revision":diagnostics::service::global().status().revision,
    })))
}
fn validate_settings(value: &Value) -> Result<AppSettings, String> {
    let template = settings_transfer::project_settings(
        serde_json::to_value(settings::get_default_settings())
            .map_err(|_| "settings-transfer-invalid")?,
    )?;
    if value.as_object().map(|o| o.keys().collect::<Vec<_>>())
        != template.as_object().map(|o| o.keys().collect::<Vec<_>>())
    {
        return Err("settings-transfer-invalid".into());
    }
    let parsed: AppSettings =
        serde_json::from_value(value.clone()).map_err(|_| "settings-transfer-invalid")?;
    let normalized = settings_transfer::project_settings(
        serde_json::to_value(&parsed).map_err(|_| "settings-transfer-invalid")?,
    )?;
    if normalized != *value
        || parsed.settings_schema_version != 2
        || parsed.auto_submit
        || parsed.update_checks_enabled
        || !(0.0..=1.0).contains(&parsed.audio_feedback_volume)
        || !(0.0..=1.0).contains(&parsed.word_correction_threshold)
        || parsed.hold_threshold_ms > 10_000
        || parsed.paste_delay_ms > 60_000
        || parsed.paste_delay_after_ms > 60_000
        || parsed.extra_recording_buffer_ms > 60_000
        || parsed.history_limit > 1_000_000
        || !matches!(parsed.app_language.as_str(), "en" | "zh-TW" | "zh-CN")
        || parsed.selected_model.len() > 1024
        || crate::shortcut::dictionary::validated_words(parsed.custom_words.clone())
            .map_err(|_| "settings-transfer-invalid")?
            != parsed.custom_words
    {
        return Err("settings-transfer-invalid".into());
    }
    let defaults = settings::get_default_settings();
    if parsed.bindings.len() != defaults.bindings.len() {
        return Err("settings-transfer-invalid".into());
    }
    for (key, binding) in &parsed.bindings {
        if !defaults.bindings.contains_key(key)
            || binding.id != *key
            || binding.current_binding.len() > 256
            || binding.current_binding.chars().any(char::is_control)
        {
            return Err("settings-transfer-invalid".into());
        }
    }
    let binding = |id: &str| {
        parsed
            .bindings
            .get(id)
            .map(|b| b.current_binding.as_str())
            .unwrap_or("")
    };
    crate::physical_gestures::Bindings::parse(
        binding("transcribe"),
        binding("translation"),
        binding("ask"),
        binding("stop"),
    )
    .map_err(|_| "settings-transfer-invalid")?;
    Ok(parsed)
}
fn model_folder(app: &AppHandle, folder: &str) -> Result<PathBuf, String> {
    let files = model_roots_files::Filesystem::new(|_| false);
    let canonical = files
        .canonical_root(Path::new(folder))
        .map_err(|_| "settings-transfer-model-folder-unavailable")?;
    let data = std::fs::canonicalize(root(app)?).map_err(|_| "settings-transfer-storage-failed")?;
    if data.starts_with(&canonical) {
        return Err("settings-transfer-model-folder-unavailable".into());
    }
    Ok(canonical)
}
fn audio_conflict(settings: &AppSettings) -> bool {
    use crate::audio_toolkit::audio::{list_input_devices, list_output_devices};
    let inputs = list_input_devices().ok();
    let outputs = list_output_devices().ok();
    let missing_input = |name: &Option<String>| {
        name.as_deref().is_some_and(|name| {
            name != "default"
                && !inputs
                    .as_ref()
                    .is_some_and(|devices| devices.iter().any(|d| d.name == name))
        })
    };
    missing_input(&settings.selected_microphone)
        || missing_input(&settings.clamshell_microphone)
        || settings
            .selected_output_device
            .as_deref()
            .is_some_and(|name| {
                name != "default"
                    && !outputs
                        .as_ref()
                        .is_some_and(|devices| devices.iter().any(|d| d.name == name))
            })
}
fn file_hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn read_import(path: &Path) -> Result<Vec<u8>, String> {
    files::read(path, settings_yaml::MAX_BYTES)
        .map_err(failure)?
        .ok_or("settings-transfer-file-unavailable".into())
}
fn disk_state() -> Result<std::collections::BTreeMap<Domain, Option<Value>>, String> {
    let owner = RUNTIME_FILES.lock().unwrap_or_else(|e| e.into_inner());
    let files = owner.as_ref().ok_or("settings-transfer-storage-failed")?;
    read_disk(files)
}
fn read_disk(files: &Files) -> Result<std::collections::BTreeMap<Domain, Option<Value>>, String> {
    [
        Domain::App,
        Domain::Providers,
        Domain::ModelRoot,
        Domain::Diagnostics,
    ]
    .into_iter()
    .map(|domain| {
        Ok((
            domain,
            files
                .read(domain)
                .map_err(failure)?
                .map(|text| {
                    serde_json::from_str(&text).map_err(|_| "settings-transfer-invalid".to_string())
                })
                .transpose()?,
        ))
    })
    .collect()
}
fn build_preview(
    app: &AppHandle,
    path: PathBuf,
    bytes: Vec<u8>,
    mut archive: Archive,
    folder: Option<String>,
) -> Result<TransferPreview, String> {
    if let Some(folder) = folder {
        archive.model_folder = folder;
    }
    let baseline = state_hash(app)?;
    let disk = settings_transfer::DiskStamp::new(
        disk_state()?,
        serde_json::to_value(settings::get_settings(app))
            .map_err(|_| "settings-transfer-invalid")?,
    );
    let settings = validate_settings(&archive.settings)?;
    let before = snapshot(app, true)?;
    // Compare actual keys privately; identical masks must not conceal changes.
    let prepared = archive.prepared_providers(configuration_store(app)?.as_ref())?;
    let next: Value = serde_json::from_str(&prepared).map_err(|_| "settings-transfer-invalid")?;
    let old = before.ai.app_keys.as_ref();
    let mut effects = Vec::new();
    for connection in &archive.ai.connections {
        let previous = old.and_then(|keys| keys.get(&connection.id));
        let key = next
            .get("connections")
            .and_then(Value::as_array)
            .and_then(|connections| {
                connections.iter().find(|c| {
                    c.get("draft")
                        .and_then(|d| d.get("id"))
                        .and_then(Value::as_str)
                        == Some(connection.id.as_str())
                })
            })
            .and_then(|c| c.get("key"))
            .and_then(|k| k.get("value"))
            .and_then(Value::as_str);
        let effect = match (previous, key) {
            (Some(a), Some(b)) if a == b => "kept",
            (Some(_), Some(_)) => "replaced",
            (None, Some(_)) => "added",
            (Some(_), None) => "cleared",
            (None, None) => "missing",
        };
        effects.push(KeyEffect {
            name: connection.display_name.clone(),
            effect: effect.into(),
        });
    }
    for connection in &before.ai.connections {
        if !archive.ai.connections.iter().any(|c| c.id == connection.id) {
            effects.push(KeyEffect {
                name: connection.display_name.clone(),
                effect: "removed".into(),
            });
        }
    }
    let mut conflicts = if model_folder(app, &archive.model_folder).is_err() {
        vec!["model-folder-unavailable".into()]
    } else {
        vec![]
    };
    if audio_conflict(&settings) {
        conflicts.push("audio-device-unavailable".into());
    }
    let id = format!("transfer-{}", NEXT.fetch_add(1, Ordering::Relaxed));
    let before_yaml = before.preview()?;
    let after_yaml = archive.preview()?;
    let changed_fields = settings_transfer::changed_fields(
        &settings_yaml::parse(before_yaml.as_bytes()).map_err(|_| "settings-transfer-invalid")?,
        &settings_yaml::parse(after_yaml.as_bytes()).map_err(|_| "settings-transfer-invalid")?,
    );
    let view = TransferPreview {
        id: id.clone(),
        before_yaml,
        after_yaml,
        changed_fields,
        key_effects: effects,
        conflicts,
    };
    if state_hash(app)? != baseline {
        return Err("settings-transfer-stale".into());
    }
    *PREVIEW.lock().unwrap_or_else(|e| e.into_inner()) = Some(Preview {
        id,
        path,
        file_hash: file_hash(&bytes),
        state_hash: baseline,
        archive,
        disk,
    });
    Ok(view)
}
#[tauri::command]
#[specta::specta]
pub async fn export_settings_yaml(
    app: AppHandle,
    include_keys: bool,
) -> Result<TransferOutcome, String> {
    let _activity = activity(&app)?;
    let archive = snapshot(&app, include_keys)?;
    let text = archive.encode()?;
    let selected = app
        .dialog()
        .file()
        .set_file_name("notype-settings.yaml")
        .add_filter("YAML", &["yaml", "yml"])
        .blocking_save_file();
    let Some(selected) = selected else {
        return Ok(outcome("cancelled"));
    };
    ensure_ready()?;
    let path = selected
        .into_path()
        .map_err(|_| "settings-transfer-file-unavailable")?;
    // An export must never replace an App-owned runtime file or import source.
    let data =
        std::fs::canonicalize(root(&app)?).map_err(|_| "settings-transfer-storage-failed")?;
    let parent = path
        .parent()
        .and_then(|p| std::fs::canonicalize(p).ok())
        .ok_or("settings-transfer-file-unavailable")?;
    if parent.starts_with(data) {
        return Err("settings-transfer-file-unavailable".into());
    }
    files::atomic_write(&path, text.as_bytes(), settings_yaml::MAX_BYTES).map_err(failure)?;
    diagnostics::record(
        diagnostics::Phase::SettingsExport,
        diagnostics::Outcome::Ok,
        None,
    );
    Ok(outcome("saved"))
}
#[tauri::command]
#[specta::specta]
pub async fn preview_settings_import(app: AppHandle) -> Result<Option<TransferPreview>, String> {
    ensure_ready()?;
    *PREVIEW.lock().unwrap_or_else(|e| e.into_inner()) = None;
    diagnostics::record(
        diagnostics::Phase::SettingsImport,
        diagnostics::Outcome::Started,
        None,
    );
    let selected = app
        .dialog()
        .file()
        .add_filter("YAML", &["yaml", "yml"])
        .blocking_pick_file();
    let Some(selected) = selected else {
        return Ok(None);
    };
    ensure_ready()?;
    let path = selected
        .into_path()
        .map_err(|_| "settings-transfer-file-unavailable")?;
    let bytes = read_import(&path)?;
    let archive = Archive::decode(&bytes)?;
    build_preview(&app, path, bytes, archive, None).map(Some)
}
#[tauri::command]
#[specta::specta]
pub async fn resolve_settings_import_folder(
    app: AppHandle,
    id: String,
) -> Result<Option<TransferPreview>, String> {
    ensure_ready()?;
    let (path, archive, hash) = {
        let preview = PREVIEW.lock().unwrap_or_else(|e| e.into_inner());
        let p = preview
            .as_ref()
            .filter(|p| p.id == id)
            .ok_or("settings-transfer-stale")?;
        (p.path.clone(), p.archive.clone(), p.file_hash.clone())
    };
    let selected = app.dialog().file().blocking_pick_folder();
    let Some(selected) = selected else {
        return Ok(None);
    };
    ensure_ready()?;
    let bytes = read_import(&path)?;
    if file_hash(&bytes) != hash {
        return Err("settings-transfer-stale".into());
    }
    let folder = selected
        .into_path()
        .map_err(|_| "settings-transfer-model-folder-unavailable")?;
    model_folder(&app, &folder.to_string_lossy())?;
    build_preview(
        &app,
        path,
        bytes,
        archive,
        Some(folder.to_string_lossy().into()),
    )
    .map(Some)
}
#[tauri::command]
#[specta::specta]
pub fn cancel_settings_import(id: String) {
    let mut preview = PREVIEW.lock().unwrap_or_else(|e| e.into_inner());
    if preview.as_ref().is_some_and(|p| p.id == id) {
        *preview = None;
        diagnostics::record(
            diagnostics::Phase::SettingsImport,
            diagnostics::Outcome::Cancelled,
            None,
        );
    }
}
#[tauri::command]
#[specta::specta]
pub async fn resolve_settings_import_audio(
    app: AppHandle,
    id: String,
) -> Result<TransferPreview, String> {
    ensure_ready()?;
    let (path, mut archive, hash) = {
        let preview = PREVIEW.lock().unwrap_or_else(|e| e.into_inner());
        let p = preview
            .as_ref()
            .filter(|p| p.id == id)
            .ok_or("settings-transfer-stale")?;
        (p.path.clone(), p.archive.clone(), p.file_hash.clone())
    };
    let bytes = read_import(&path)?;
    if file_hash(&bytes) != hash {
        return Err("settings-transfer-stale".into());
    }
    for key in [
        "selected_microphone",
        "clamshell_microphone",
        "selected_output_device",
        "selected_channel",
    ] {
        archive.settings[key] = Value::Null;
    }
    build_preview(&app, path, bytes, archive, None)
}
#[tauri::command]
#[specta::specta]
pub async fn apply_settings_import(app: AppHandle, id: String) -> Result<TransferOutcome, String> {
    ensure_ready()?;
    let _serial = command_guard();
    let mut slot = PREVIEW.lock().unwrap_or_else(|e| e.into_inner());
    let preview = slot
        .as_ref()
        .filter(|p| p.id == id)
        .ok_or("settings-transfer-stale")?;
    if file_hash(&read_import(&preview.path)?) != preview.file_hash
        || state_hash(&app)? != preview.state_hash
        || !preview.disk.matches(&disk_state()?)
    {
        return Err("settings-transfer-stale".into());
    }
    let folder = model_folder(&app, &preview.archive.model_folder)?;
    let imported = validate_settings(&preview.archive.settings)?;
    if audio_conflict(&imported) {
        return Err("settings-transfer-audio-unavailable".into());
    }
    let manager = app.state::<Arc<ModelManager>>();
    let freeze = manager
        .roots
        .freeze_for_settings()
        .map_err(|_| "settings-transfer-busy")?;
    PENDING.store(true, Ordering::SeqCst);
    let staged = (|| {
        let store = app
            .store(crate::portable::store_path(settings::SETTINGS_STORE_PATH))
            .map_err(|_| "settings-transfer-storage-failed")?;
        store
            .save()
            .map_err(|_| "settings-transfer-storage-failed")?;
        let file_owner = RUNTIME_FILES.lock().unwrap_or_else(|e| e.into_inner());
        let files = file_owner
            .as_ref()
            .ok_or("settings-transfer-storage-failed")?;
        if !preview.disk.matches(&read_disk(files)?) {
            return Err("settings-transfer-stale".into());
        }
        let mut targets = Vec::new();
        for domain in [
            Domain::App,
            Domain::Providers,
            Domain::ModelRoot,
            Domain::Diagnostics,
        ] {
            let before = files.read(domain).map_err(failure)?;
            let after=match domain{
                Domain::App=>{
                    let mut envelope:Value=before.as_deref().map(serde_json::from_str).transpose().map_err(|_|"settings-transfer-invalid")?.unwrap_or_else(||json!({}));
                    envelope.as_object_mut().ok_or("settings-transfer-invalid")?.insert("settings".into(),serde_json::to_value(&imported).map_err(|_|"settings-transfer-invalid")?);
                    serde_json::to_string(&envelope).map_err(|_|"settings-transfer-invalid")?
                }
                Domain::Providers=>preview.archive.prepared_providers(configuration_store(&app)?.as_ref())?,
                Domain::ModelRoot=>{
                    let marker:Option<String>=before.as_deref().and_then(|s|serde_json::from_str::<Value>(s).ok()).and_then(|v|v.get("bootstrap_applied").and_then(Value::as_str).map(str::to_owned));
                    String::from_utf8(model_roots_files::encode_bootstrap(&folder,manager.roots.snapshot().revision.checked_add(1).ok_or("settings-transfer-invalid")?,marker.as_deref()).map_err(|_|"settings-transfer-invalid")?).map_err(|_|"settings-transfer-invalid")?
                }
                Domain::Diagnostics=>json!({"schema_version":1,"diagnostics_enabled":preview.archive.diagnostics_enabled}).to_string(),
            };
            targets.push(Target {
                domain,
                before,
                after,
            });
        }
        if state_hash(&app)? != preview.state_hash {
            return Err("settings-transfer-stale".into());
        }
        settings_journal::stage(files, targets).map_err(failure)
    })();
    if let Err(error) = staged {
        PENDING.store(false, Ordering::SeqCst);
        diagnostics::record(
            diagnostics::Phase::SettingsImport,
            diagnostics::Outcome::Failed,
            None,
        );
        return Err(error);
    }
    diagnostics::record(
        diagnostics::Phase::SettingsImport,
        diagnostics::Outcome::Ok,
        None,
    );
    *FROZEN.lock().unwrap_or_else(|e| e.into_inner()) = Some(freeze);
    *slot = None;
    // Reply before normal Tauri exit so plugin caches flush their OLD state.
    let handle = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(350));
        handle.restart();
    });
    Ok(outcome("restarting"))
}
/// Must run before diagnostics/bootstrap/plugin settings and every runtime owner.
pub fn recover_startup(app: &AppHandle) -> Result<(), String> {
    let directory = root(app)?;
    std::fs::create_dir_all(&directory).map_err(|_| "settings-transfer-storage-failed")?;
    // Tauri spawns the successor immediately before the old process exits.
    // Wait briefly for that process's OS handles; never initialize stores early.
    let started = std::time::Instant::now();
    let files = loop {
        match Files::open(&directory) {
            Ok(files) => break files,
            Err(settings_journal::Error::Locked)
                if started.elapsed() < std::time::Duration::from_secs(5) =>
            {
                std::thread::sleep(std::time::Duration::from_millis(50))
            }
            Err(error) => return Err(failure(error)),
        }
    };
    settings_journal::recover(&files).map_err(failure)?;
    *RUNTIME_FILES.lock().unwrap_or_else(|e| e.into_inner()) = Some(files);
    Ok(())
}
pub fn recovery_failure_notice() {
    #[cfg(windows)]
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};
        let text:Vec<u16>="notype settings import recovery could not finish. Your import journal was kept. Restart to retry; if it repeats, report settings-transfer-recovery-failed.\0".encode_utf16().collect();
        let title: Vec<u16> = "notype\0".encode_utf16().collect();
        let _ = MessageBoxW(
            None,
            windows::core::PCWSTR(text.as_ptr()),
            windows::core::PCWSTR(title.as_ptr()),
            MB_ICONERROR | MB_OK,
        );
    }
}

#[cfg(test)]
#[path = "../../../tests/unittest/settings_transfer_native.rs"]
mod tests;
