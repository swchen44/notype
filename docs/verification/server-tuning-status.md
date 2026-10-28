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

After the user confirmed unlock, the old App was quit normally and this candidate
was launched with the existing profile/model folder. Computer Use verified the AI
server/no-selection gray state with master OFF, both tabs and three independent
tuning scenes defaulting to None. The same previously selected Qwen model was
loaded without opening the microphone: Not loaded/gray -> Loading/amber ->
Loaded/green; normal idle unload later returned to Not loaded/gray. No active AI
connection was chosen and no settings/prompt/key were saved during these checks.

The user then confirmed the new Gemini key was ready in home `.env`. A fresh
read-only official compatibility-models request returned HTTP200 and60 IDs in439ms
at2026-10-28T07:32:17.779Z. The candidate's actual model-list button also succeeded
and displayed home `.env` as the source, without showing its value. This validates
authentication and model listing only. The user then explicitly confirmed a
Free-tier key/project. Native local.5 generation succeeded at 2026-10-28 18:07:53
Taiwan with `gemini-3.1-flash-lite`: requested thinking OFF, explicit mandatory
minimum `minimal` (the previously accepted fallback), received input50/output1.
Actual API fee remains unknown. Master AI stays OFF and no connection is active.
The saved verification profile keeps the home `.env` reference; no real key was
typed or exported. Generation stopped after this success.

Earlier native short/exact-returned `gemini-2.5-flash-lite` tests failed; two owned
one-shot synthetic diagnostics returned HTTP404, one with an array error root.
Read-only native metadata returned200 and did not prove compatibility generation.
No definitive retirement/server cause is claimed. Total five synthetic generation
attempts: three App calls (two failed/one success), two standalone404 diagnostics.
Earlier missing usage remains unknown in cumulative counters; nothing was reset.
Free Standard eligibility and minimum controls were checked against official
[pricing](https://ai.google.dev/gemini-api/docs/pricing) and
[OpenAI compatibility](https://ai.google.dev/gemini-api/docs/openai) documentation.

## Remaining work and delivery boundary

YAML export/import source is implemented; native restart acceptance is pending.
Scope is settings/prompts/vocabulary,
optional keys, excluding histories/model files, with difference/conflict preview.
The user has since confirmed App-stored keys only (home `.env` variable names,
never their values), whole replacement after preview/confirmation, and preservation
of unchanged original bound App keys when omitted. The bounded YAML codec and
provider archive/replacement, fixed-domain journal, startup recovery and Advanced
UI now have focused source tests. None is in the running local.5 candidate binary.
Latest source gates: focused Rust37 plus actual native AppSettings2, rendered
UI125 (including transfer6), frontend lint/build and focused Clippy. The native
default projection regression initially failed on retained legacy shortcut
fields, then passed after limiting physical parsing to four active bindings.
No home secrets, public API, microphone or real user settings are used by these
transfer tests. New parser dependencies add six crates/eleven audited notices.
See the [transfer design](../settings-transfer-design.md).

Native transfer UI/restart acceptance, company networking,
microphone/full ASR sessions and clean-host installation remain unverified here.
Published **v0.1.1-local.2 is unchanged** and does not contain these source/local
candidate changes. This receipt does not represent a new GitHub Release ZIP.
