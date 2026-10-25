# Physical tap routing and guarded Windows delivery (#13)

2026-10-25 Asia/Taipei. Functional implementation and deterministic contracts are present. **Positive native insertion acceptance is still open:** both bounded owned-fixture manual runs ended without the fixture becoming foreground. The guard refused delivery before any target write. No real five-App, live physical keyboard/IME, live microphone or company API acceptance is claimed.

## Behavior and public seams

- Idle starts are physical `alt_right` (Dictation), `alt_right+shift_right` (Translation) and `alt_right+space` (Ask). Stop is independently configurable, default `alt_right`, only during Recording; Esc cancels Recording/processing. These are toggle gestures. A gesture commits after every participating key is released, so a slow shared prefix cannot start Dictation before Translation/Ask. Repeats and suffix release/repress cannot create a second action. There is no chord deadline.
- `physical_gestures::{Bindings,GestureRouter}` is a portable, std-only interface. `event(vk,down,Phase)` returns one lifecycle intent plus paired OS forwarding/suppression. Busy, cancellation, configuration changes and input-loss reconciliation invalidate partial gestures. Windows `physical_shortcuts` translates physical sided keys, ignores injected input/AltGr's phantom left Ctrl and moves resolved intents off the low-level hook callback. It neither logs ordinary keys nor performs audio/HTTP work in the callback. Other platforms explicitly report Unsupported; their native adapter implementations remain future work.
- General settings edits, validates, saves and resets all three starts and Stop. Windows edits synchronously save the store before reporting success, and Reset uses current product defaults rather than historical stored `default_binding` metadata. Untouched legacy Ctrl+Space migrates; custom bindings remain. Esc is fixed. The old hold/toggle selector is absent from this flow.
- `NativeInput::capture()` returns a process-local opaque target token and optional validated process **basename**. Generic hosts, access failures or invalid names produce general context. No window title, input/page text, omnibox, URL or domain is acquired. `begin_captured_scenario` delegates to the existing profile/context/processing-snapshot-before-audio workflow.
- `DesktopWorkflow::with_delivery(Arc<dyn DeliveryAdapter>)` composes the native guard; portable tests retain the unavailable default. Original and processed selections use the same core delivery/review/revision rules. Review confirmation does not focus a window or bypass the guard.
- The Windows target snapshot contains foreground/focused-control identity, process creation identity, style, selection/length metadata and a focus-event epoch. Tokens are bounded and process-local; persisted or invented tokens cannot resolve to the current foreground after restart. Observed A→B→A focus changes invalidate the capture.
- The native capability allowlist is verified Unicode Edit/RichEdit20W/RICHEDIT50W controls. Readonly/password/disabled/invisible, changed identity/style/selection/length, unsupported controls, NUL/oversized text, single-line newline and native length limits fail closed. Directed `EM_REPLACESEL` avoids global clipboard/keyboard focus routing. There is no clipboard mutation, Ctrl+V, automatic focus or Enter. A timeout or ambiguous post-write observation returns Uncertain, and shared-core deduplication prevents resending that session.
- Foreign-window validation and insertion are guarded pre/post observations, not a Windows transaction that atomically locks another process's selection or focus. Unsupported/unknown targets remain Pending; ambiguous writes remain DeliveryUnknown. Browser/Electron/Terminal/Teams/Outlook support is not inferred from native control class names or from this owned fixture.

## Executed verification

Product code was built after merging integration `3d9b7ed0516bad0e731166e6ab780d4fbf5c214f`; later merge `b33b2ca` adds #12 evidence only. Product fixes are in `079b547` and `29d09cc`.

| Check | Actual result |
| --- | --- |
| Portable desktop Rust harness | 29 passed, 0 failed/ignored: 10 workflow, 12 HTTP/provider, 6 gesture, 1 unknown native-token rejection |
| Actual Windows shortcut IPC/restart/UI reload | 1 passed, 0 failed/skipped; all four edits survive immediate restart, conflicts reject, all four defaults reset, localized fields render |
| Frontend TypeScript/Vite | Passed, 2031 modules; existing large theme chunk warning |
| Frontend ESLint / changed-file Prettier / Rustfmt | Passed |
| CPU-only x64 native no-bundle debug build | Passed in 32.86 s; existing unused `Emitter` warning in `secure_input.rs` |
| Owned native delivery runtime suite | 1 failed at required foreground precondition, before any delivery write; 0 skipped. Positive insertion/selection/timeout cases remain unperformed |
| Actual user keyboard, Windows IME, five applications, microphone, company API | Not performed |

Commands (PowerShell, repository root unless stated):

```powershell
$env:CARGO_TARGET_DIR='D:/git/notype-build'
$env:CARGO_PROFILE_DEV_DEBUG='0'
$env:CARGO_PROFILE_TEST_DEBUG='0'
$env:CARGO_INCREMENTAL='0'
$env:CARGO_BUILD_JOBS='2'
cargo test --locked --manifest-path tests/integration/desktop-contracts/Cargo.toml --test desktop-workflow --test desktop-provider --test physical-gestures --test native-delivery
# From desktop/:
bun run lint
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
bun run tauri build --no-bundle --debug -- --locked
# With a normal-user interactive desktop/fixed WebView2 and explicit isolated test profile:
$env:NOTYPE_DESKTOP_EXE='D:/git/notype-build/debug/notype.exe'
node --test tests/integration/desktop-shortcuts.runtime.mjs
$env:NOTYPE_MANUAL_FIXTURE_FOCUS='1'
node --test tests/integration/desktop-input.runtime.mjs
```

Final executable SHA256: `8e93741935ddddb318f513de467d7375bbb82dfdb98889aabde3a84261eb3d6b`. Debug-only CDP/test commands are absent from release invoke handling. The test driver explicitly blanks all six legacy/new bindings, leaves the selected model empty and disables always-on microphone; the settings test alone substitutes rare F20–F23 bindings. Audio is never recorded by these tests. Fixture state reads are restricted to its own synthetic controls.

RED→GREEN evidence: the adapter-boundary test first exposed missing delivery composition; shortcut restart then reproduced a successful edit lost before Store autosave, fixed with synchronous saving. A separate legacy-default Reset test failed with `shortcut-physical-key-required`, fixed by selecting current product defaults. The final runtime suite includes both persistence and legacy Reset and passes. The HTTP/workflow seam uses actual loopback HTTP/SQLite and an explicit delivery adapter substitute, not invented native insertion results.

## Native environment and remaining manual prerequisite

The elevated tool host fails main WebView startup with `0x80070057`/overlay `0x8007139F`. The exact failure also occurred with the pristine #17 binary, establishing an environment limitation. An outside-repository helper creates same-user/session medium-integrity Node suspended through the verified Explorer parent, checks image/token, assigns a kill-on-close job before resume, and runs app plus fixture at the same integrity. It uses fixed WebView2 `156.0.4314.42`, explicit fresh profiles and no arbitrary app/command mode. Actual shortcut and #12's three context/normalization sessions passed through this normal-user route.

Owned fixture observations confirmed visible=true, local first Edit focused=true, foregroundOwned=false; input desktop accessible/receiving input and session active. Allowed own-window UIA/foreground attempts did not satisfy the production precondition. No further activation workarounds were attempted. The second, taskbar-enabled manual run expired after five minutes with no foreground ownership; its `complete.json` proves zero remaining owned processes and no visible owned window. Helper cleanup success is distinct from its test exit 1.

Outside receipt roots under `%LOCALAPPDATA%/notype-study/agent-notes/input-runtime-launch/`:

- `profile-positive-566d7558af5f4751a56a357b45fca21b/`: taskbar-enabled manual precondition failure and cleanup 0.
- `profile-positive-198b6d6b303e482e958f57487a11eb3d/`: actual legacy Reset RED on earlier binary `633f5821…`.
- `profile-positive-ef94a68b5f36489ca6b70fea39c90022/`: final `8e937419…` shortcut/restart GREEN and cleanup 0.
- `profile-positive-525ab3d9648e45fa868b74a81dae2ac1/`: #12 actual three-session normalizer proof on `633f5821…`, cleanup 0; corresponding language receipts are separate.

A user-startable fixture is prepared, **not executed**, at `%LOCALAPPDATA%/notype-study/build-cache/input-runtime-app-probe-u3opQ0/manual-input/Start-owned-input-test.cmd`. Starting it creates a fresh five-minute wait only when the user is ready; click the first field of `notype OWNED input fixture` and keep it foreground for about 20 seconds. It uses a verified regular snapshot of `633f5821671edd4e13a634ed21acecda18a11cc763769bbccb9f2c189be34b93` (guard implementation identical; before the Reset-only fix), 71 regular app/DLL/resource files, 115,205,271 bytes, plus 13,054,404 bytes of frozen test/Playwright files. It pins app, manifests, script and runtime, disables all starts and microphone, preserves the source binding file only if semantically unchanged, and refuses launch if compiled source bindings changed. Keep the input worktree present until this optional manual acceptance is completed. `prepared.json` records hashes; no mutable/deleted D-target binary or runtime package install is needed.

## Build storage adjustment

The desktop binary consumes the Rust library directly on Windows/macOS/Linux, so `[lib] crate-type=["rlib"]` replaces unused staticlib/cdylib outputs. Before rebuilding, all owned Cargo/compiler/linker processes were stopped. The exact generated `D:/git/notype-build/debug/deps/notype_app_lib.lib` was verified regular/non-reparse, single-link, with archive magic and an exclusive handle; only that file was removed. Its length was 752,900,676 bytes, observed free-space increase 752,902,144 bytes. The external receipt is `agent-notes/input-staticlib-reclaim.json`. C++ dependency cache and app-local DLL staging were retained; no Vulkan/GPU tools or second native cache were introduced. This adjustment does not deliver foreign-host/mobile library artifacts.
