# YAML settings transfer design (#34)

Status: design preparation; transfer is not implemented. Accepted decisions are
[recorded in issue #34](https://github.com/swchen44/notype/issues/34#issuecomment-6462790837).
The user has waived further Astra plan reviews. Existing settings/tuning source
and local candidate acceptance remain separate from this planned feature.

## Confirmed behavior

Export contains App settings, saved AI connections/options, shared scenario
prompts, each server's independent optional scene tuning, translation settings,
glossaries/custom words, diagnostics preferences and model-location references.
History, API usage counters, logs, model binaries and caches are excluded. No
model file is copied or moved. The exported document is human-readable UTF-8 YAML
with an explicit notype format/schema marker.

Include API keys defaults OFF. When explicitly selected, only keys already stored
in the App may be exported. Home `.env` contributes the applicable fixed variable
name reference only. Export never opens/resolves home `.env`, even with key
inclusion enabled. A masked display placeholder must never become a real imported
key. The explicit key-inclusive output is plaintext and the UI says so.

Import parses and validates first, displays differences/conflicts, and waits for
explicit confirmation. The accepted apply policy replaces the whole reviewed
configuration; it is not a per-change merge interface. Cancel performs no writes.
Files, revisions and settings changed since preview require a new preview. No
connection query, test, generation, model download or microphone request is sent
by preview or apply.

**Pending data policy:** when incoming YAML omits keys, the user is choosing
between retaining keys only for an unchanged connection identity/canonical route/
authentication binding, or clearing all stored App keys. No implementation may
silently choose either. Imported explicit keys still require valid route/auth
binding, and arbitrary `.env` variable names cannot select new secrets to read.

## Authorities and apply boundary

The current App has distinct authorities: AppSettings in `settings_store.json`
(including custom words), ConnectionStore, model-root preferences and dedicated
diagnostics preferences. A raw directory copy would also copy histories and
runtime state and is not the transfer format. Legacy credential-bearing settings
must be classified explicitly rather than leaking through a whole-struct dump.

A proposed coordinator validates domain values before preparing a transfer. It
pins preview inputs and current settings, admits apply only when session/model
work is idle, and stages a bounded transaction. Confirmation must explicitly say
that settings take effect together after restart. The startup recovery boundary
precedes opening the affected stores, initializing hotkeys/audio or starting model
work. Other domains must not observe partly imported values.

The journal uses fixed App-owned target filenames and atomic staging/commit with
recovery/rollback; YAML cannot supply target paths. History and usage databases
are never transaction targets. File errors, stale previews or interrupted apply
retain/recover the previous complete settings or the complete confirmed settings,
never a silent mixture. Existing settings cache/autosave and single-instance
behavior must be included in integration acceptance.

Missing machine-specific audio/model locations appear as conflicts requiring
resolution in the preview. Do not invent replacement paths or download models.
Model location is a reference; normal model-root validation/recovery remains the
authority. Applied settings must also respect fixed product policies, including
CPU-only operation and disabled automatic submit/update behavior.

## Parsing and error handling

Use a maintained, pinned typed YAML parser with explicit resource limits. Reject
duplicate keys, unsupported schema/fields, extra documents, external inclusion/
property expansion, unsupported tags and excessive input/depth/alias expansion.
No unsafe YAML object construction or arbitrary filesystem/network resolvers.
Validate the parsed domain values, not only YAML syntax.

Return fixed actionable error codes for selection cancellation, invalid format,
unsupported version, invalid configuration, unresolved conflict, stale preview,
busy operation, insufficient space, staging failure and recovery failure.
Diagnostics record phase, result code, schema, counts and bounded timings only;
no YAML excerpts, key values, prompts, vocabulary or file contents. Parser error
snippets must not enter UI/diagnostics as raw exceptions.

## Required tests and acceptance

Unit tests belong in `tests/unittest/`:

- Schema, duplicate/tag/document/depth/size/alias limits and fixed safe errors.
- Export with key inclusion OFF/ON, including legacy secret-bearing fields and
  a home reader that panics if invoked; variable references remain references.
- Independent three-scene tuning, prompts, vocabulary and translation round-trip.
- Key omission/binding policy after the user's answer; redacted credential diffs.
- Difference/conflict classification, stale previews and cancel/no-write.

Integration tests belong in `tests/integration/`:

- Real owned settings/provider/root/diagnostics stores: export, fresh-profile
  import, reopen and exact typed values. Sentinel history/usage/model files remain
  byte-for-byte unchanged. No actual home key, microphone or public provider.
- Stale file/current revision, missing model location, permission/space/rename
  faults and interrupted multi-file transactions with actual restart recovery.
- Cache/autosave ordering, single-instance ownership and idle admission races.
- Actual rendered settings transfer UI with IPC doubles: default key exclusion,
  select file, localized differences/conflicts, cancel, confirm-whole-replace,
  restart notice, busy/stale/failure/retry, and no automatic network requests.

Source gates include focused unit/integration execution, formatting, lint/build,
Clippy and complete composed CI. Native Windows build, file-dialog/preview/restart
Computer Use, actual offline/clean-profile transfer, dependency notices and a new
verified package are separate acceptance gates. The previous immutable package
assembly assumptions must be re-audited if a parser/lockfile dependency changes.
