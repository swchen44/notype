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
fn store() -> ConnectionStore {
    ConnectionStore::open(Arc::new(Storage::default())).unwrap()
}
fn draft(dialect: ProviderDialect) -> ConnectionDraft {
    store()
        .create(0, "Connection".into(), ConnectionKind::Provider, dialect)
        .unwrap()
        .connections
        .remove(0)
        .connection
}

#[test]
fn bounded_collection_cas_and_explicit_active_removal_preserve_global_settings() {
    let store = store();
    let first = store
        .create(
            0,
            " duplicate ".into(),
            ConnectionKind::Provider,
            ProviderDialect::Deepseek,
        )
        .unwrap();
    let id = first.connections[0].connection.id.clone();
    assert_eq!(first.connections[0].connection.model, "");
    assert_eq!(
        first.connections[0].connection.thinking_preference,
        ThinkingPreference::Off
    );
    assert_eq!(first.active_connection_id, None);
    assert!(!first.connections[0].app_key_present);
    assert_eq!(
        store
            .create(
                0,
                "stale".into(),
                ConnectionKind::Custom,
                ProviderDialect::Generic
            )
            .err()
            .unwrap()
            .0,
        "configuration-conflict"
    );
    let selected = store.activate(1, Some(id.clone())).unwrap();
    assert_eq!(selected.active_connection_id.as_deref(), Some(id.as_str()));
    let prompted = store
        .prompt(2, notype_core::Scenario::Ask, Some("current ask".into()))
        .unwrap();
    assert_eq!(prompted.active_connection_id.as_deref(), Some(id.as_str()));
    let removed = store.remove(3, &id).unwrap();
    assert_eq!(removed.active_connection_id, None);
    assert_eq!(removed.prompts.ask, "current ask");
    for revision in 4..24 {
        store
            .create(
                revision,
                "duplicate".into(),
                ConnectionKind::Custom,
                ProviderDialect::Generic,
            )
            .unwrap();
    }
    assert_eq!(
        store
            .create(
                24,
                "overflow".into(),
                ConnectionKind::Custom,
                ProviderDialect::Generic
            )
            .err()
            .unwrap()
            .0,
        "configuration-connection-limit"
    );
}

#[test]
fn canonical_routes_keep_prefixes_and_refuse_unbound_secret_routes() {
    let mut draft = draft(ProviderDialect::Generic);
    draft.kind = ConnectionKind::Custom;
    draft.api_address = "https://company.example/api/v1/".into();
    assert_eq!(
        canonical_chat(&draft).unwrap().as_str(),
        "https://company.example/api/v1/chat/completions"
    );
    assert_eq!(
        canonical_models(&draft).unwrap().as_str(),
        "https://company.example/api/v1/models"
    );
    draft.api_address = "https://company.example/api/v1/chat/completions".into();
    assert!(canonical_chat(&draft).is_err());
    draft.address_mode = AddressMode::FullChatEndpoint;
    assert_eq!(
        canonical_models(&draft).err().unwrap().0,
        "configuration-model-list-unavailable"
    );
    draft.models_endpoint = Some("https://company.example/api/v1/models".into());
    assert!(canonical_models(&draft).is_ok());
    draft.models_endpoint = Some("https://other.example/models".into());
    assert!(canonical_models(&draft).is_err());
    for url in [
        "http://company.example/v1",
        "https://user:key@company.example/v1",
        "https://company.example/v1?key=secret",
        "https://company.example/v1#secret",
    ] {
        draft.api_address = url.into();
        assert!(canonical_chat(&draft).is_err());
    }
    draft.kind = ConnectionKind::Local;
    draft.api_address = "http://127.0.0.1:11434/v1".into();
    draft.address_mode = AddressMode::BaseUrl;
    assert!(canonical_chat(&draft).is_ok());
    draft.api_address = "https://company.example/v1".into();
    assert!(canonical_chat(&draft).is_err());
}

#[test]
fn exact_capability_facts_keep_requested_off_distinct_from_mandatory_minimum() {
    let mut draft = draft(ProviderDialect::Gemini);
    assert!(draft.model.is_empty());
    draft.model = "gemini-3.8-flash".into();
    draft.effort = Some("minimal".into());
    let resolved = capabilities::resolve(&draft).unwrap();
    assert_eq!(resolved.effective_effort.as_deref(), Some("low"));
    assert_eq!(resolved.reason, "mandatory_minimum");
    assert_eq!(resolved.effective_thinking, Some(true));
    assert!(!resolved.supported_efforts.contains(&"minimal".into()));
    draft.thinking_preference = ThinkingPreference::On;
    assert!(capabilities::resolve(&draft).is_err());
    draft.model = "gemini-3.5-flash-lite".into();
    assert_eq!(
        capabilities::resolve(&draft)
            .unwrap()
            .effective_effort
            .as_deref(),
        Some("minimal")
    );
    draft.model = "unknown-custom-model".into();
    draft.effort = None;
    assert_eq!(capabilities::resolve(&draft).unwrap().reason, "unresolved");
    draft.capability_declaration = Some(CapabilityDeclaration {
        kind: "non_reasoning".into(),
        minimum_effort: None,
        supported_efforts: vec![],
        temperature_supported: true,
        top_p_supported: false,
    });
    assert_eq!(
        capabilities::resolve(&draft).unwrap().provenance,
        "user_declared"
    );
    draft.top_p = Some(0.5);
    assert!(capabilities::resolve(&draft).is_err());
}

#[test]
fn sampling_omissions_zero_and_documented_ignored_values_are_distinct() {
    let mut draft = draft(ProviderDialect::Deepseek);
    draft.temperature = Some(0.0);
    draft.top_p = Some(0.5);
    let off = capabilities::resolve(&draft).unwrap();
    assert_eq!(off.temperature_wire, Some(0.0));
    assert_eq!(off.top_p_wire, None);
    assert_eq!(off.top_p_resolution, "ignored_by_provider");
    draft.thinking_preference = ThinkingPreference::On;
    let on = capabilities::resolve(&draft).unwrap();
    assert_eq!(on.temperature_wire, None);
    assert_eq!(on.temperature_resolution, "ignored_by_provider");
    assert_eq!(on.top_p_wire, Some(0.5));
    assert_eq!(on.documented_top_p, Some(0.95));
    for value in [f64::NAN, f64::INFINITY, -0.1, 2.1] {
        draft.temperature = Some(value);
        assert!(capabilities::validate(&draft).is_err());
    }
    draft.temperature = None;
    draft.top_p = Some(0.0);
    assert!(capabilities::validate(&draft).is_err());
}

#[test]
fn dotenv_parser_is_selected_literal_bounded_and_never_expands_other_secrets() {
    use credentials::{parse, PublicKey};
    let bytes=b"\xef\xbb\xbfUNRELATED_SECRET=PRIVATE_OTHER\nOPENROUTER_API_KEY=OTHER_KEY\nexport DEEPSEEK_API_KEY=\"literal-$HOME-$(command)\" # comment\n";
    assert_eq!(
        parse(bytes, PublicKey::DeepSeek).unwrap().as_deref(),
        Some("literal-$HOME-$(command)")
    );
    assert_eq!(parse(bytes, PublicKey::Gemini).unwrap(), None);
    for bytes in [
        b"DEEPSEEK_API_KEY=x\nDEEPSEEK_API_KEY=y".as_slice(),
        b"DEEPSEEK_API_KEY=\"unterminated".as_slice(),
        b"DEEPSEEK_API_KEY=bad\0key".as_slice(),
    ] {
        assert!(parse(bytes, PublicKey::DeepSeek).is_err());
    }
    assert!(parse(&vec![b'x'; 65_537], PublicKey::DeepSeek).is_err());
}

#[test]
fn every_public_mutation_returns_safe_views_and_key_keep_is_route_bound() {
    let storage = Arc::new(Storage::default());
    let store = ConnectionStore::open(storage.clone()).unwrap();
    let mut draft = store
        .create(
            0,
            "My connection".into(),
            ConnectionKind::Provider,
            ProviderDialect::Deepseek,
        )
        .unwrap()
        .connections
        .remove(0)
        .connection;
    draft.model = "my-model".into();
    let saved = store
        .save(
            1,
            draft.clone(),
            KeyAction::Replace {
                value: "APP_SECRET_SENTINEL".into(),
            },
        )
        .unwrap();
    assert!(!serde_json::to_string(&saved)
        .unwrap()
        .contains("APP_SECRET_SENTINEL"));
    assert!(
        String::from_utf8(storage.0.lock().unwrap().clone().unwrap())
            .unwrap()
            .contains("APP_SECRET_SENTINEL")
    );
    let prompted = store
        .prompt(2, notype_core::Scenario::Ask, Some("current".into()))
        .unwrap();
    assert!(!serde_json::to_string(&prompted)
        .unwrap()
        .contains("APP_SECRET_SENTINEL"));
    let reset = store.prompt(3, notype_core::Scenario::Ask, None).unwrap();
    assert!(!serde_json::to_string(&reset)
        .unwrap()
        .contains("APP_SECRET_SENTINEL"));
    let translated = store
        .translation(4, workflow::TranslationSettings::default())
        .unwrap();
    assert!(!serde_json::to_string(&translated)
        .unwrap()
        .contains("APP_SECRET_SENTINEL"));
    draft.api_address = "https://another.example/v1".into();
    assert_eq!(
        store.save(5, draft, KeyAction::Keep).err().unwrap().0,
        "configuration-key-binding-required"
    );
    assert_eq!(store.view().unwrap().revision, 5);
}

#[test]
fn active_app_priority_and_anonymous_no_active_custom_local_never_read_home() {
    struct Reader(Mutex<Vec<credentials::PublicKey>>);
    impl credentials::CredentialReader for Reader {
        fn read(&self, key: credentials::PublicKey) -> Result<Option<String>, ConfigError> {
            self.0.lock().unwrap().push(key);
            Ok(Some("HOME_SECRET_SENTINEL".into()))
        }
    }
    let reader = Reader(Mutex::new(vec![]));
    let store = store();
    assert!(!store.resolve_active(&reader).unwrap().is_ready());
    assert!(reader.0.lock().unwrap().is_empty());
    let mut draft = store
        .create(
            0,
            "Public".into(),
            ConnectionKind::Provider,
            ProviderDialect::Deepseek,
        )
        .unwrap()
        .connections
        .remove(0)
        .connection;
    draft.model = "my-model".into();
    let id = draft.id.clone();
    store
        .save(
            1,
            draft.clone(),
            KeyAction::Replace {
                value: "APP_SECRET_SENTINEL".into(),
            },
        )
        .unwrap();
    store.activate(2, Some(id)).unwrap();
    let frozen = store.resolve_active(&reader).unwrap();
    assert_eq!(frozen.credential_source.as_deref(), Some("app"));
    assert!(reader.0.lock().unwrap().is_empty());
    assert!(!serde_json::to_string(&frozen)
        .unwrap()
        .contains("APP_SECRET_SENTINEL"));
    store.save(3, draft.clone(), KeyAction::Clear).unwrap();
    let home = store.resolve_active(&reader).unwrap();
    assert_eq!(home.credential_source.as_deref(), Some("home_env"));
    assert_eq!(
        reader.0.lock().unwrap().as_slice(),
        &[credentials::PublicKey::DeepSeek]
    );
    assert!(!serde_json::to_string(&home)
        .unwrap()
        .contains("HOME_SECRET_SENTINEL"));
    draft.authentication = Authentication::None;
    store.save(4, draft.clone(), KeyAction::Keep).unwrap();
    store.resolve_active(&reader).unwrap();
    assert_eq!(reader.0.lock().unwrap().len(), 1);
    draft.authentication = Authentication::Bearer;
    draft.kind = ConnectionKind::Custom;
    store.save(5, draft.clone(), KeyAction::Clear).unwrap();
    assert_eq!(
        store.resolve_active(&reader).err().unwrap().0,
        "configuration-credentials-unavailable"
    );
    assert_eq!(reader.0.lock().unwrap().len(), 1);
    assert_eq!(frozen.credential_source.as_deref(), Some("app"));
}

#[test]
fn anonymous_view_distinguishes_stored_key_presence_from_effective_credential_source() {
    let store = store();
    let mut draft = store
        .create(
            0,
            "Anonymous switch".into(),
            ConnectionKind::Provider,
            ProviderDialect::Deepseek,
        )
        .unwrap()
        .connections
        .remove(0)
        .connection;
    store
        .save(
            1,
            draft.clone(),
            KeyAction::Replace {
                value: "DORMANT_KEY_SENTINEL".into(),
            },
        )
        .unwrap();
    draft.authentication = Authentication::None;
    let view = store.save(2, draft, KeyAction::Keep).unwrap();
    assert!(view.connections[0].app_key_present);
    assert_eq!(view.connections[0].credential_source, "none");
    assert_eq!(view.connections[0].credential_present, Some(false));
}

#[test]
fn unknown_model_capability_remains_saved_but_view_explains_generation_unavailability() {
    let store = store();
    let mut draft = store
        .create(
            0,
            "Unknown".into(),
            ConnectionKind::Custom,
            ProviderDialect::Generic,
        )
        .unwrap()
        .connections
        .remove(0)
        .connection;
    draft.api_address = "https://custom.example/v1".into();
    draft.model = "user-model".into();
    draft.authentication = Authentication::None;
    let view = store.save(1, draft, KeyAction::Clear).unwrap();
    assert_eq!(
        view.connections[0].resolution.as_ref().unwrap().reason,
        "unresolved"
    );
    assert_eq!(
        view.connections[0].configuration_error.as_deref(),
        Some("configuration-capability-required")
    );
}

#[test]
fn failed_processing_snapshot_retains_global_target_terms_and_prompts() {
    struct NeverHome;
    impl credentials::CredentialReader for NeverHome {
        fn read(&self, _: credentials::PublicKey) -> Result<Option<String>, ConfigError> {
            panic!("custom selected connection must not read home")
        }
    }
    for (dialect, failure) in [
        (
            ProviderDialect::Deepseek,
            notype_core::PreparationFailure::Credentials,
        ),
        (
            ProviderDialect::Generic,
            notype_core::PreparationFailure::Capability,
        ),
    ] {
        let store = store();
        let mut draft = store
            .create(
                0,
                "Owned missing preparation".into(),
                ConnectionKind::Custom,
                dialect,
            )
            .unwrap()
            .connections
            .remove(0)
            .connection;
        draft.api_address = "https://owned.invalid/v1".into();
        draft.model = "manual-model".into();
        let view = store.save(1, draft.clone(), KeyAction::Clear).unwrap();
        let view = store.activate(view.revision, Some(draft.id)).unwrap();
        let mut translation = view.translation;
        translation.target_language = "ja-JP".into();
        translation.glossary = vec![workflow::TranslationTerm {
            source: "term".into(),
            target: "mapped".into(),
            target_language: "ja-JP".into(),
        }];
        let view = store.translation(view.revision, translation).unwrap();
        store
            .prompt(
                view.revision,
                notype_core::Scenario::Translation,
                Some("captured prompt".into()),
            )
            .unwrap();
        let captured = store.processing_snapshot(&NeverHome);
        assert_eq!(captured.preparation_failure, Some(failure));
        assert_eq!(captured.translation.target_language, "ja-JP");
        assert_eq!(captured.translation.glossary[0].target, "mapped");
        assert_eq!(captured.prompts.translation, "captured prompt");
    }
}

#[test]
fn failed_processing_snapshot_cannot_mix_globals_changed_during_key_resolution() {
    struct MutatingReader(Arc<ConnectionStore>);
    impl credentials::CredentialReader for MutatingReader {
        fn read(&self, _: credentials::PublicKey) -> Result<Option<String>, ConfigError> {
            let view = self.0.view().unwrap();
            let mut translation = view.translation;
            translation.target_language = "fr-FR".into();
            let view = self.0.translation(view.revision, translation).unwrap();
            self.0
                .prompt(
                    view.revision,
                    notype_core::Scenario::Ask,
                    Some("newer prompt".into()),
                )
                .unwrap();
            Ok(None)
        }
    }
    let store = Arc::new(store());
    let mut draft = store
        .create(
            0,
            "Public".into(),
            ConnectionKind::Provider,
            ProviderDialect::Deepseek,
        )
        .unwrap()
        .connections
        .remove(0)
        .connection;
    draft.model = "manual-model".into();
    let view = store.save(1, draft.clone(), KeyAction::Clear).unwrap();
    let view = store.activate(view.revision, Some(draft.id)).unwrap();
    let mut translation = view.translation;
    translation.target_language = "ja-JP".into();
    let view = store.translation(view.revision, translation).unwrap();
    store
        .prompt(
            view.revision,
            notype_core::Scenario::Ask,
            Some("captured prompt".into()),
        )
        .unwrap();
    let captured = store.processing_snapshot(&MutatingReader(store.clone()));
    assert_eq!(
        captured.preparation_failure,
        Some(notype_core::PreparationFailure::Credentials)
    );
    assert_eq!(captured.translation.target_language, "ja-JP");
    assert_eq!(captured.prompts.ask, "captured prompt");
    assert_eq!(store.view().unwrap().translation.target_language, "fr-FR");
    assert_eq!(store.view().unwrap().prompts.ask, "newer prompt");
}

#[test]
fn saved_and_unsaved_key_actions_agree_without_rebinding_or_persisting_query_credentials() {
    struct NeverHome;
    impl credentials::CredentialReader for NeverHome {
        fn read(&self, _: credentials::PublicKey) -> Result<Option<String>, ConfigError> {
            panic!("selected App credential must not read home")
        }
    }
    let store = store();
    let mut draft = store
        .create(
            0,
            "Owned key policy".into(),
            ConnectionKind::Local,
            ProviderDialect::Deepseek,
        )
        .unwrap()
        .connections
        .remove(0)
        .connection;
    draft.api_address = "http://127.0.0.1:11434/v1".into();
    draft.model = "manual-model".into();
    draft.authentication = Authentication::Bearer;
    let view = store
        .save(
            1,
            draft.clone(),
            KeyAction::Replace {
                value: "  ORIGINAL_SYNTHETIC_KEY  ".into(),
            },
        )
        .unwrap();
    let view = store
        .activate(view.revision, Some(draft.id.clone()))
        .unwrap();
    for value in [
        "  ".to_owned(),
        "bad\0key".into(),
        "bad\nkey".into(),
        "x".repeat(8193),
    ] {
        assert_eq!(
            store
                .save(
                    view.revision,
                    draft.clone(),
                    KeyAction::Replace {
                        value: value.clone()
                    }
                )
                .err()
                .unwrap()
                .0,
            "configuration-key-invalid"
        );
        assert_eq!(
            store
                .resolve_models(
                    view.revision,
                    draft.clone(),
                    KeyAction::Replace { value },
                    &NeverHome
                )
                .err()
                .unwrap()
                .0,
            "configuration-key-invalid"
        );
        assert_eq!(store.view().unwrap().revision, view.revision);
    }
    let transient = store
        .resolve_models(
            view.revision,
            draft.clone(),
            KeyAction::Replace {
                value: "  TRANSIENT_SYNTHETIC_KEY  ".into(),
            },
            &NeverHome,
        )
        .unwrap();
    assert_eq!(
        transient
            .authorization_header()
            .unwrap()
            .unwrap()
            .to_str()
            .unwrap(),
        "Bearer TRANSIENT_SYNTHETIC_KEY"
    );
    assert_eq!(
        store
            .resolve_active(&NeverHome)
            .unwrap()
            .authorization_header()
            .unwrap()
            .unwrap()
            .to_str()
            .unwrap(),
        "Bearer ORIGINAL_SYNTHETIC_KEY"
    );
    let mut changed = draft.clone();
    changed.api_address = "http://127.0.0.1:11435/v1".into();
    assert_eq!(
        store
            .save(view.revision, changed.clone(), KeyAction::Keep)
            .err()
            .unwrap()
            .0,
        "configuration-key-binding-required"
    );
    assert_eq!(
        store
            .resolve_models(view.revision, changed, KeyAction::Keep, &NeverHome)
            .err()
            .unwrap()
            .0,
        "configuration-key-binding-required"
    );
    assert_eq!(
        store
            .resolve_models(view.revision, draft.clone(), KeyAction::Clear, &NeverHome)
            .err()
            .unwrap()
            .0,
        "configuration-credentials-unavailable"
    );
    let cleared = store.save(view.revision, draft, KeyAction::Clear).unwrap();
    assert!(!cleared.connections[0].app_key_present);
    assert_eq!(
        store.resolve_active(&NeverHome).err().unwrap().0,
        "configuration-credentials-unavailable"
    );
}
