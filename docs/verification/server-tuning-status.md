# AI server status, tabs and per-scene tuning (#33/#34)

Product source: `6e3e61461b1a4bd58d96bbcf7e449df1d55327e0`.
This contains the status correction from
`32a8c2c2fc7080370fe73703914b41a9a193ba4c`.
Confirmed tuning specification:
[issue #34](https://github.com/swchen44/notype/issues/34#issuecomment-6460276795).

## Behavior

- The lower-left AI server panel shows the selected connection name, model and
  endpoint, master enablement and last observed connection-test result. Green
  requires enabled AI and a successful current test; no selection or AI OFF is
  gray. Untested/read-failed is distinct from a live connectivity guarantee.
- Speech model selection is labelled Selected. The gray/green load indicator and
  Not loaded/Loaded text follow actual engine events rather than mere selection.
- AI server settings and shared scenario prompts occupy separate accessible tabs.
  Hidden panels remain mounted, preserving unsaved key/model/prompt drafts.
- Each server has independent dictation, translation and ask tuning, initially
  None. A supplement is appended to that scene's shared prompt; None retains the
  shared prompt exactly. Effective prompts are frozen with the session snapshot.
- Each supplement allows up to 8,192 UTF-8 bytes; combined instructions allow up
  to 32,768. Blank/NUL/oversized supplements and oversized combinations are rejected
  without publishing a new configuration or truncating text.

## Executed source checks

Before implementation the added backend test failed because the resolved prompt
did not contain its scene supplement; the rendered test failed because no tuning
control existed. After implementation:

| Check | Result |
| --- | --- |
| Windows release provider/config/bootstrap/model contracts | 42 passed across five targets |
| Focused release Clippy for config and store targets | Passed with `-D warnings` |
| Selected UI regression | 32 passed, zero skipped: 30 rendered plus two form units |
| ESLint, TypeScript, Vite | Passed |
| Changed frontend/test Prettier and changed Rust rustfmt | Passed |
| Full composed Linux CI | [Run 40630433546](https://github.com/swchen44/notype/actions/runs/40630433546), both jobs completed successfully on the exact product source |

The full UI job ran 14 units and 119 rendered tests, all passed with no skips.
Portable Node suites ran 33 units and 24 integrations; the separate Windows CMD
integration was explicitly skipped on Linux. This is not Windows CMD acceptance.

Rust command: `cargo test --release --offline --locked --manifest-path tests/integration/desktop-contracts/Cargo.toml --test provider-config --test provider-config-store --test provider-models --test provider-bootstrap --test offline-bootstrap-store`.
The real filesystem permission-failure case is Unix-only and was not executed on
Windows; the portable unit storage-failure case did execute. The three-scene test
uses actual ConnectionStore, DesktopWorkflow, SQLite and owned loopback HTTP and
inspects the outgoing system message; it opens no public endpoint or home key file.

UI command: `node --test --test-concurrency=1 tests/unittest/provider-form.ui.mjs tests/integration/provider-connections.ui.mjs tests/integration/api-options.ui.mjs tests/integration/footer-status.ui.mjs`.
IPC is doubled; components, localization, rendering, tab keyboard navigation and
user interactions are real. Tests cover draft retention, independent tuning saves,
clearing one scene to None, translated 360px layouts, key actions and status events.

## Native build and local package

The exact clean product source compiled successfully with `tauri build --no-bundle`
in 276,717 ms. The existing native build emitted 26 unused-code/import warnings;
no compile errors. No dependencies or lockfiles changed.

- Executable: 59,427,328 bytes.
- Executable SHA-256: `9b20dc44e3bf411ea957f25d4e2d469f2a96dad9047cf6c0b6640f827b35598d`.
- Local candidate: `0.1.1-local.5`, complete 1,695-file application inventory verified.
- Application manifest SHA-256: `64be72d8573e6c663da53fe30d2ced85be135fbb1c27b39794378cc5294cc20c`.

The new candidate has **not been launched or visually accepted**: Computer Use
could not activate the existing old App; a fresh capture established that Windows
was locked. UI input stopped and the user was asked to unlock. The original App,
settings, model folder and master OFF/no-active-server preference were retained.
No real Gemini request was made during this implementation. The user's new Gemini
development key is not ready; an earlier model-list HTTP200 concerned the value
then in home `.env`, not the new key and not successful generation/free-tier proof.

## Remaining work and delivery boundary

YAML export/import is **not implemented**. Scope is settings/prompts/vocabulary,
optional keys, excluding histories/model files, with difference/conflict preview.
The user has since confirmed App-stored keys only (home `.env` variable names,
never their values) and whole replacement after preview/confirmation. One policy
remains pending: preserve unchanged bound App keys or clear all App keys when the
incoming file omits them. See the [transfer design](../settings-transfer-design.md).

New native UI acceptance, the new Gemini key/generation check, company networking,
microphone/full ASR sessions and clean-host installation remain unverified here.
Published **v0.1.1-local.2 is unchanged** and does not contain these source/local
candidate changes. This receipt does not represent a new GitHub Release ZIP.
