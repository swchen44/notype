# Offline bundle native handoff (#29)

Approved [P29-v1](https://github.com/swchen44/notype/issues/29#issuecomment-6405357391),
[Astra approval](https://github.com/swchen44/notype/issues/29#issuecomment-6405428255).

The facade derives its bundle root from its module location. Its only native
handoff is `--offline-bundle-root <canonical absolute local root>`. It removes
inherited `NOTYPE_BOOTSTRAP*`, `NOTYPE_BUNDLE*`, `NOTYPE_OFFLINE*`, and
`NOTYPE_MODEL*` variables. There are no keys, raw JSON, bootstrap paths or model
paths in argv/environment. Native independently validates that its executable
is `app/node_modules/notype-win32-x64/app/notype.exe` under that root, verifies
the declared application-manifest and optional bootstrap digests, and uses its
compiled catalog/RootService to verify models before initialization.

`bundle.json` schema1 has `mode:"offline-bundle"`, `id`, `version`, 40-hex
`source`, application pin `{name,version,manifest:"payload.json",sha256}`,
`model:{model_id,root:"models",pin:{name,version,manifest:"payload.json",sha256}}`,
and optional `bootstrap:{path:"bootstrap/notype-bootstrap.json",sha256}`.
The Node owner documents additional Node/inventory/provenance fields in
`packages/OFFLINE-BUNDLE.md`. Native reads the narrow subset, with bounded input.

Optional bootstrap JSON has exactly `schema:1`, `bootstrap_id`,
`stt:{model_id,root:"models"}`, and optional `connection`. A connection has
exactly the existing draft fields below, without backend `id` or credentials:

```text
display_name, kind, dialect, address_mode, api_address, models_endpoint,
model, authentication, timeout_ms, thinking_preference, effort,
capability_declaration, temperature, top_p, max_output_tokens
```

Optional draft values use JSON null. The capability declaration uses existing
kind/minimum_effort/supported_efforts/temperature_supported/top_p_supported.
Native validation is authoritative for routes and capabilities. Unknown/secret
fields and environment expansion are rejected. Generic distribution uses the
declared STT model without an invented administrator connection. No bootstrap
operation reads credentials or performs a model listing/API request.

Existing initialized settings/root/provider state always wins and is skipped
without a rewrite. A missing unused bundled default cannot block an existing
valid explicit root/selection. Fresh required model failure opens Recovery;
full bundle --check still fails any missing declared asset. Required fresh
domains are validated together, committed independently with durable applied
markers, then readiness is published. Partial failure blocks capture/API and
preserves History/safe settings/root Recovery until explicit retry.
