#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/capture_startup.rs"]
mod capture_startup;
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/diagnostics.rs"]
mod diagnostics;
#[allow(dead_code)]
#[path = "../../desktop/src-tauri/src/provider_config.rs"]
mod provider_config;
#[allow(dead_code, unused_imports)]
#[path = "../../desktop/src-tauri/src/workflow.rs"]
mod workflow;
use provider_config::*;
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct Storage(Mutex<Option<Vec<u8>>>);
impl ConfigurationStorage for Storage {
    fn load(&self) -> Result<Option<Vec<u8>>, ConfigError> {
        Ok(self.0.lock().unwrap().clone())
    }
    fn commit(&self, bytes: &[u8]) -> Result<(), ConfigError> {
        *self.0.lock().unwrap() = Some(bytes.to_vec());
        Ok(())
    }
}
fn preset() -> ConnectionPreset {
    serde_json::from_value(serde_json::json!({
        "display_name":"Synthetic connection", "kind":"custom", "dialect":"generic",
        "address_mode":"base_url", "api_address":"https://synthetic.reserved.invalid/v1",
        "models_endpoint":null, "model":"synthetic-model", "authentication":"bearer",
        "timeout_ms":30000, "thinking_preference":"off", "effort":null,
        "capability_declaration":{"kind":"non_reasoning","minimum_effort":null,
          "supported_efforts":[],"temperature_supported":true,"top_p_supported":true},
        "temperature":null,"top_p":null,"max_output_tokens":null
    })).unwrap()
}

#[test]
fn first_use_import_is_one_durable_no_key_selection_and_restart_never_duplicates_it() {
    let storage = Arc::new(Storage::default());
    let store = ConnectionStore::open(storage.clone()).unwrap();
    assert_eq!(store.bootstrap_if_empty(preset(), "synthetic-v1").unwrap(), BootstrapImport::Applied);
    let view = store.view().unwrap();
    assert_eq!(view.revision, 1);
    assert_eq!(view.connections.len(), 1);
    assert_eq!(view.active_connection_id.as_deref(), Some(view.connections[0].connection.id.as_str()));
    assert_eq!(view.connections[0].connection.model, "synthetic-model");
    assert!(!view.connections[0].app_key_present);
    let before = storage.0.lock().unwrap().clone();
    let reopened = ConnectionStore::open(storage.clone()).unwrap();
    assert_eq!(reopened.bootstrap_if_empty(preset(), "synthetic-v2").unwrap(), BootstrapImport::SkippedExisting);
    assert_eq!(*storage.0.lock().unwrap(), before);
    assert_eq!(reopened.view().unwrap().connections.len(), 1);
}
