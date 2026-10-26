//! Runtime observations belong to an exact verified artifact, never an ID alone.
use super::model_roots::{ArtifactKey, RootService};
use std::{collections::BTreeMap, sync::Mutex};

#[derive(Clone)]
pub struct Capabilities {
    pub streaming: bool,
    pub translation: bool,
    pub language_detection: bool,
    pub languages: Vec<String>,
}
#[derive(Default)]
pub struct RuntimeMetadata(Mutex<BTreeMap<ArtifactKey, Capabilities>>);
impl RuntimeMetadata {
    pub fn get(&self, key: &ArtifactKey) -> Option<Capabilities> {
        self.0
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(key)
            .cloned()
    }
    pub fn publish(&self, roots: &RootService, key: &ArtifactKey, value: Capabilities) -> bool {
        roots
            .with_current(key, || {
                let mut values = self.0.lock().unwrap_or_else(|e| e.into_inner());
                values.retain(|old, _| {
                    old.generation == key.generation && old.model_id != key.model_id
                });
                values.insert(key.clone(), value);
            })
            .is_some()
    }
}
