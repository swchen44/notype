# Provider configuration IPC contract (#26)

Approved scope: [config-v2](https://github.com/swchen44/notype/issues/25#issuecomment-6404377843),
[approval](https://github.com/swchen44/notype/issues/25#issuecomment-6404401509),
[debug-v1](https://github.com/swchen44/notype/issues/25#issuecomment-6404440016),
[approval](https://github.com/swchen44/notype/issues/25#issuecomment-6404467374).
This is the backend/UI implementation contract; native execution remains separate.

All Tauri arguments use camelCase, returned DTO fields use snake_case. Commands
return the existing `Result<T, string>` binding envelope; errors are fixed codes.
`expectedRevision` is mandatory for every mutation. Success returns the entire
latest safe settings view; conflict returns `configuration-conflict`, requiring
reload. No command returns a stored/resolved credential. No old settings import.

```ts
type ConnectionKind = "provider" | "custom" | "local";
type ProviderDialect = "generic" | "deepseek" | "openrouter" | "gemini" | "ollama_openai";
type AddressMode = "base_url" | "full_chat_endpoint";
type ThinkingPreference = "off" | "on";
type KeyAction = { action: "keep" } | { action: "clear" } | { action: "replace"; value: string };
type CapabilityDeclaration = {
  kind: "non_reasoning" | "optional" | "mandatory";
  minimum_effort: string | null;
  supported_efforts: string[];
  temperature_supported: boolean;
  top_p_supported: boolean;
};
type ConnectionDraft = {
  id: string; display_name: string; kind: ConnectionKind; dialect: ProviderDialect;
  address_mode: AddressMode; api_address: string; models_endpoint: string | null;
  model: string; authentication: "none" | "bearer"; timeout_ms: number;
  thinking_preference: ThinkingPreference; effort: string | null;
  capability_declaration: CapabilityDeclaration | null;
  temperature: number | null; top_p: number | null; max_output_tokens: number | null;
};
type RequestResolutionView = {
  supported_efforts: string[];
  capability: "non_reasoning" | "optional" | "mandatory" | "unknown";
  provenance: "documented" | "provider_metadata" | "user_declared" | "unknown";
  effective_thinking: boolean | null; effective_effort: string | null;
  reason: "non_reasoning" | "disabled" | "enabled" | "mandatory_minimum" | "unresolved";
  temperature_wire: number | null; top_p_wire: number | null;
  temperature_resolution: "omitted" | "sent" | "ignored_by_provider" | "unsupported";
  top_p_resolution: "omitted" | "sent" | "ignored_by_provider" | "unsupported";
  documented_top_p: number | null;
};
type CredentialSource = "app" | "home_env" | "none" | "unresolved";
type ConnectionView = ConnectionDraft & {
  app_key_present: boolean; credential_present: boolean | null;
  credential_source: CredentialSource;
  effective_endpoint: string | null; resolution: RequestResolutionView | null;
  configuration_error: string | null;
};
type ConnectionSettingsView = {
  schema_version: 2; revision: number; connections: ConnectionView[];
  active_connection_id: string | null;
  prompts: ScenarioPrompts; translation: TranslationSettings;
};
type ModelListRequest = {
  request_id: string; expected_revision: number;
  connection: ConnectionDraft; key_action: KeyAction;
};
type ModelListResult = {
  request_id: string; configuration_revision: number; connection_id: string;
  status: "ok" | "unsupported" | "cancelled" | "error" | "throttled" | "stale";
  models: { id: string }[]; error_code: string | null;
  retry_after_seconds: number | null; credential_source: CredentialSource;
  cache_hit: boolean;
};
```

`ScenarioPrompts`, `SceneId` and `TranslationSettings` retain their existing safe
shapes. `get_text_configuration()` now returns `ConnectionSettingsView`.
`save_scenario_prompt(scenario, prompt, expectedRevision)`,
`reset_scenario_prompt(scenario, expectedRevision)` and
`save_translation_settings(translation, expectedRevision)` return that same safe
view. They preserve records/active selection. The obsolete whole-record
`set_text_configuration` command is retired; use record mutations below.

- `create_text_connection(expectedRevision, displayName, kind, dialect)` returns
  `ConnectionSettingsView`. Backend creates ID, blank model, OFF, no key; no activation.
- `save_text_connection(expectedRevision, connection, keyAction)` returns the view.
  Keep requires unchanged canonical route/dialect binding. Clear removes the App key;
  eligible public presets may subsequently resolve their named home fallback.
- `activate_text_connection(expectedRevision, id: string | null)` returns the view.
- `remove_text_connection(expectedRevision, id)` returns the view; removing active
  leaves no active connection. No automatic replacement.
- `list_text_models(request: ModelListRequest)` returns `ModelListResult`.
  Record must exist; edited form may be unsaved. Query never saves or activates.
  `request_id` is a caller-generated opaque ASCII request ID, max64 characters.
- `cancel_text_models(requestId)` returns null; cancellation is best effort and
  stale responses must also be ignored in the editor.

Credential presence/source in collection views is App/none where directly known;
an eligible home fallback is `unresolved`/null until an explicit operation resolves
it. No collection-wide credential reads. A model-list result can report the actual
resolved source. Missing capability saves successfully with `unresolved` preview;
generation is unavailable until an exact fact or explicit declaration resolves it.
Known mandatory OFF maps to its minimum with `mandatory_minimum`; it never says OFF
was honored. All new records have blank models, including provider presets.

Model results are reference IDs only: Copy never edits, saves, activates or sends
generation. Use a fresh request ID for each explicit query; one in-flight per
connection, 60s route/credential throttle, stale revision rejected. No raw provider
message/description, pagination or arbitrary response fields are returned.

The process-private cooldown identity is the canonical effective models endpoint
plus resolved authorization identity, shared across records. Collection edits do
not reset its 60-second or Retry-After bound; the collection revision independently
fences request admission and late replies. Processing snapshots retain the same
captured prompts/translation even when selected preparation fails, so repair and
retry preserve the saved history target. See the
[review-fix receipt](verification/provider-config-reviewfix.md).

History keeps the existing model/effort association and adds safe request-option
tuples (requested preference, effective thinking/effort, provenance/resolution,
configured vs wire sampling/output limit). No connection name/address/key enters
those tuples. CSV stays one row per history ID with a versioned aggregate tuple
column. Exact tuple binding fields are published with the Rust implementation.
