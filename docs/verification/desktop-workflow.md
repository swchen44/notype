# Desktop saved-text workflow (#11)

Verified on Windows x64, 2026-10-25, from `ticket/11-desktop-workflow`, based on integration `4422d6f`. This receipt covers the fixed faithful Dictation preset. The later editable/resettable prompts and Dictation/Translation/Ask scenarios belong to #17 and are not implemented by this baseline.

## Result

Both existing recording shortcuts now use the same session workflow. Successful local STT text is saved in the core before text processing; there is no persistent WAV archive or WAV-success gate. Failed, timed-out or cancelled attempts preserve their original and cannot automatically deliver it. Explicit original selection does not itself insert text. Delivery currently reports unavailable/pending until #13 supplies verified target capture and insertion; the production build has no arbitrary-current-focus paste fallback.

The sole active text-history store is `<portable::app_data_dir>/sessions.db`, opened by `notype-core` with its lifetime ownership lock. The old Handy manager/retention path is no longer compiled or initialized. History uses string IDs and exposes search, separate original/result display and copy, text-only retry, cancellation, explicit original selection and manual deletion. It has no playback, recording-folder, star/retention or re-STT controls. Tray latest-copy also reads this core store.

An existing `<data root>/history.db` is offered to the core's read-only pinned Handy importer. Import failures are surfaced without resetting either store. Node v1 data already at the destination `sessions.db` migrates through the core. The importer never reads/deletes source recordings, and import tombstones survive manual destination deletion. No other application's profile is searched automatically.

Text-service configuration is a separate local `text_processing_store.json`. It starts disabled, with empty endpoint/model/key and an unspecified authentication requirement. The user explicitly chooses a compatible Chat Completions endpoint and None/Bearer authentication; model entry is manual and no `/models` discovery occurs. Missing/invalid configuration becomes persisted `configuration-required` with zero requests. Configuration is captured once per processing attempt; recognition mode is captured at recording start. Prompt-at-start snapshots are part of the subsequent #17 work.

The company has confirmed a compatible Chat Completions format. The actual endpoint, model, authentication, TLS/proxy and real service behavior remain unverified. Requests separate fixed role/principles/boundaries, the three adopted public few-shot pairs, and JSON-encoded current original/glossary/captured mode. They do not add history, application text, target tokens or audio. The transport has a bounded deadline, cancellation, redirect rejection, bounded response size and sanitized failures; malformed/non-string/empty/truncated responses fail closed. It does not strip or reinterpret arbitrary tags/code fences and has no speculative schema/prompt retry.

## Executed checks

| Check | Result |
| --- | --- |
| Core unit target `core-session` | 9 passed |
| Core integration targets `core-persistence`, `core-http` | 6 + 3 passed |
| Core documentation example / clippy warnings denied | 1 compiled example; clippy passed |
| Desktop unit target `desktop-workflow` | 2 passed |
| Desktop real HTTP target `desktop-provider` | 5 passed |
| Actual native `desktop-workflow.runtime.mjs` | 4 passed, 0 skipped; final combined run 13.19 s |
| Existing native settings/submission/updater regression | 1 passed, 0 skipped; 3.75 s |
| TypeScript + Vite through `tauri build --no-bundle --debug -- --locked` | Passed; existing chunk-size warning |
| ESLint, changed frontend Prettier, Rust formatting, staged whitespace check | Passed |
| Frontend copied only from Git staged blobs | Passed, 392 desktop files, 2,030 transformed modules |

The native workflow receipt launches the actual executable and WebView2 through its debug-only CDP seam, with synthetic STT output, fresh external data roots and unbound shortcuts. It verifies visible settings and history controls as well as public Tauri commands: save produces no request, real loopback HTTP shape/data, separate copy actions, invalid response, saved-original retry, timeout, row/global cancellation, an unrelated history retry surviving global recording cancellation, search/delete, persistence on restart and read-only legacy import. The Node/Handy migration case verifies source DB bytes and source audio are unchanged and deleted imports do not return. These are synthetic fixtures, not personal data or actual microphone/STT acceptance.

The tracked-source frontend check populated a new temporary tree from Git's staged blobs, then reused only the existing dependency directory through a junction. No ignored product source was copied. This corrects the #9 clean-checkout gap: the two restored model-settings source files are now tracked, with scoped `.gitignore` exceptions that continue to ignore weights. `ModelsSettings.tsx` SHA256 matches the fixed archive: `b6d887f54cefe759259128d270c6790b93c1afc260a08bf365172096407d5fbd`.

## Reproduction

From `desktop/`, preserve the scoped MSVC UTF-8 configuration and use an external target directory:

```powershell
$env:CARGO_TARGET_DIR='D:/git/notype-build'
$env:CARGO_INCREMENTAL='0'
$env:CARGO_PROFILE_DEV_DEBUG='0'
$env:CARGO_BUILD_JOBS='2'
cargo test --offline --locked --features tauri/custom-protocol --manifest-path src-tauri/Cargo.toml --test desktop-workflow
cargo test --offline --locked --features tauri/custom-protocol --manifest-path src-tauri/Cargo.toml --test desktop-provider
bun run tauri build --no-bundle --debug -- --locked
```

Then from the repository root:

```powershell
$env:NOTYPE_DESKTOP_EXE='D:/git/notype-build/debug/notype.exe'
node --test tests/integration/desktop-workflow.runtime.mjs
node --test tests/integration/desktop-settings.runtime.mjs
```

Core commands remain in [notype-core's README](../../crates/notype-core/README.md). Native C++ outputs were reused; no installer or Vulkan setup was performed. The initial desktop test build fetched one missing public `windows-sys` dependency and added the path core dependency to the committed Cargo lock. No API key was needed. Normal successful save/copy feedback is inline: the native receipt caught stacked success toasts intercepting Retry, and the same receipt passed after this fix without forced clicks.

## Red/green and boundaries

Observed failures before implementation/fix include the missing core configuration failure variant; missing desktop workflow/cancel/retry APIs; absent faithful transport fields; a non-Send lock held across an await; an aborted desktop future retaining its active recording generation; missing native commands/search; inaccessible select labels; and the toast interception above. They passed after the corresponding changes. Tests observe public session, Tauri, HTTP and history/UI outcomes rather than internal method order.

New core failure `TextFailure::ConfigurationRequired` persists the sanitized code `configuration-required`; no new history schema is needed. Recognition metadata currently maps the existing desktop ChineseScript setting into the two core modes. #12 owns binding the real STT/script pipeline to that same snapshot and proving both mixed-language modes. #13 owns guarded native insertion and real hotkey/target behavior. Release registration omits the synthetic debug commands.

Eight public references in [faithful-text.json](../../tests/integration/fixtures/faithful-text.json) prepare real-provider quality checks, including meaningful filler, corrections, uncertainty, protected literals and dictated instructions. They are not eight passing model-quality tests. No real-company response quality, microphone recognition, five-application insertion, latency target, release payload or macOS/Linux desktop runtime was accepted by this task. The native process also loaded the mandatory DirectML dependency through its developer-cache link; packaging must dereference it as documented in [payload preparation](portable-inputs-preparation.md).
