# Physical tap routing and guarded Windows delivery (#13)

2026-10-25 Asia/Taipei. A user-started owned native-control suite passed once on `633f5821…`; two other user-started runs failed their assertions with safe Pending outcomes. All three are retained below. QA isolation was subsequently strengthened because the old harness checked foreground ownership only at its initial wait. The new scoped build passes 30 Rust contracts and two hidden IPC/restart tests; its positive human-fixture suite has **not** been rerun. No real five-App, live physical keyboard/IME, live microphone or company API acceptance is claimed.

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

The latest QA scope build includes integration `b1aa22bddc8058560678d19426f5a948bad34ff9`. Initial product changes are in `079b547` and `29d09cc`; the later QA scope fix is documented below.

| Check | Actual result |
| --- | --- |
| Portable desktop Rust harness | 30 passed, 0 failed/ignored: 10 workflow, 12 HTTP/provider, 6 gesture, 2 native token/fixture-identity checks |
| Actual Windows hidden IPC/restart/UI reload | 2 passed, 0 failed/skipped: missing/mismatched QA owners reject without finish/delivery/history; all four shortcut edits survive immediate restart, conflicts reject, all defaults reset, localized fields render |
| Frontend TypeScript/Vite | Passed, 2031 modules; existing large theme chunk warning |
| Frontend ESLint / changed-file Prettier / Rustfmt | Passed |
| CPU-only x64 native no-bundle debug build | Latest scope build passed in 24.53 s; existing unused `Emitter` warning in `secure_input.rs` |
| User-started owned native delivery on old `633f5821…` | Three runs: 1 passed in 19.57 s, 2 failed with Pending; all cleanup 0, 0 skipped. Exact failed native snapshot differences were not recorded |
| New scoped QA positive human suite | Not rerun; old user-start entry retired, replacement prepared inactive |
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
node --test tests/integration/desktop-input-scope.runtime.mjs
$env:NOTYPE_MANUAL_FIXTURE_FOCUS='1'
node --test tests/integration/desktop-input.runtime.mjs
```

Latest scoped executable SHA256: `9fbe2c0ffdd8bf33bda96601f69fe7e3ffd9d81b8cd907dc1cab8832d586f654`. The previous shortcut fix built `8e937419…`. Debug-only CDP/test commands are absent from release invoke handling. The test driver explicitly blanks all six legacy/new bindings, leaves the selected model empty and disables always-on microphone; the settings test alone substitutes rare F20–F23 bindings. Audio is never recorded by these tests. Fixture state reads are restricted to its own synthetic controls.

RED→GREEN evidence: the adapter-boundary test first exposed missing delivery composition; shortcut restart then reproduced a successful edit lost before Store autosave, fixed with synchronous saving. A separate legacy-default Reset test failed with `shortcut-physical-key-required`, fixed by selecting current product defaults. The final runtime suite includes both persistence and legacy Reset and passes. The HTTP/workflow seam uses actual loopback HTTP/SQLite and an explicit delivery adapter substitute, not invented native insertion results.

## Native environment and remaining manual prerequisite

The elevated tool host fails main WebView startup with `0x80070057`/overlay `0x8007139F`. The exact failure also occurred with the pristine #17 binary, establishing an environment limitation. An outside-repository helper creates same-user/session medium-integrity Node suspended through the verified Explorer parent, checks image/token, assigns a kill-on-close job before resume, and runs app plus fixture at the same integrity. It uses fixed WebView2 `156.0.4314.42`, explicit fresh profiles and no arbitrary app/command mode. Actual shortcut and #12's three context/normalization sessions passed through this normal-user route.

Owned fixture observations confirmed visible=true, local first Edit focused=true, foregroundOwned=false; input desktop accessible/receiving input and session active. Allowed own-window UIA/foreground attempts did not satisfy the production precondition. No further activation workarounds were attempted. The second, taskbar-enabled manual run expired after five minutes with no foreground ownership; its `complete.json` proves zero remaining owned processes and no visible owned window. Helper cleanup success is distinct from its test exit 1.

Outside receipt roots under `%LOCALAPPDATA%/notype-study/agent-notes/input-runtime-launch/`:

- `profile-positive-566d7558af5f4751a56a357b45fca21b/`: taskbar-enabled manual precondition failure and cleanup 0.
- `profile-positive-198b6d6b303e482e958f57487a11eb3d/`: actual legacy Reset RED on earlier binary `633f5821…`.
- `profile-positive-ef94a68b5f36489ca6b70fea39c90022/`: final `8e937419…` shortcut/restart GREEN and cleanup 0.
- `profile-positive-525ab3d9648e45fa868b74a81dae2ac1/`: #12 actual three-session normalizer proof on `633f5821…`, cleanup 0; corresponding language receipts are separate.

The former user-startable fixture at `%LOCALAPPDATA%/notype-study/build-cache/input-runtime-app-probe-u3opQ0/manual-input/Start-owned-input-test.cmd` was used in the three runs below. It is now **retired**: its original bytes/hash are preserved beside it, while the active CMD only prints an informational stop and launches nothing. Its old `633f5821…` app snapshot and completed receipts remain unchanged.

## Three completed user-started runs and bounded diagnosis

All three roots below are under `build-cache/input-runtime-app-probe-u3opQ0/manual-input/`. Every `complete.json` records zero owned processes, and every binding-restoration receipt matches the captured original hash.

| Profile suffix | Actual result | What the existing evidence proves |
| --- | --- | --- |
| `b3edc354c3ba4f5584d4d885de5c6fa4` | Failed at frozen source line 177: expected `target-changed`, got `configuration-required`; initial foregroundOwned=true | First Edit/Unicode/dedup and RichEdit cases had passed. A later focus-change case remained Pending; exact loop iteration/capture precondition is unknown |
| `2b4f1ade4cb8498ab9f93721197edbdd` | Full suite passed, 19.570 s test / 24.296 s Node run, initial foregroundOwned=true | Own Edit and RichEdit selection insertion, once-only delivery, changed focus/A→B→A/selection, readonly/password/unsupported/length limits, timeout after write and no resend all met assertions in this run |
| `619f71f0958c4828a7ff4c793f43cb8d` | First delivery failed expected Delivered, actual Pending/`target-changed`, frozen line 150; initial foregroundOwned=true | A target token existed, then native pre-write snapshot/equality validation failed. No replacement message was sent for that attempt. The exact changed field is unknown |

The initial foreground receipt is not a continuous observation: in `619f…` it was written at `02:24:17.113Z`, while the session was created at `02:24:18.584Z`. No per-begin focus/selection/style/epoch diagnostics were retained, and the driver disposed its isolated session SQLite/log directory in `finally`; none exists in these outer receipt roots. The records therefore cannot attribute a specific focus/selection/epoch change or blame user action.

For `b3ed…`, shared core `choose_original` preserves the earlier `configuration-required` error. `deliver` leaves that Pending entry unchanged when `original_target=None`; this is the source-supported missing-capture explanation for the observed error rather than proof of a provider request. The dictation fixture has no review requirement. No target-token/entry snapshot was printed for that assertion, so the exact capture failure remains bounded unknown. Ranked remaining possibilities were delayed focus-event epochs, focus/selection changes between async operations, or a failed native snapshot query. Existing receipts cannot distinguish them. No repeat-until-green run was used to remove the two failures.

## QA confinement correction and separate RED/GREEN

The old `debug_begin_session(captureInput=true)` could capture any supported foreground Edit on later cases, because the JavaScript harness checked owned foreground only once. This was a test-isolation defect even though observed failed attempts remained Pending. One authorized **hidden** old-633f negative IPC run (`agent-notes/input-runtime-launch/profile-positive-1410eddad01c4a15915b2fde82fe614b/`) proved omitted and mismatched owner arguments were both incorrectly accepted. Each accepted generation was immediately cancelled; no finish, history, delivery, fixture window or keyboard injection occurred. The test failed as intended and job cleanup was 0.

The debug command now requires `fixtureOwner:{pid,image}` whenever `captureInput=true`. Native code requires the generated `NativeInputFixture.exe` basename, absolute exact image path and live PID, resolves creation identity, verifies owned foreground before capture, and stores that immutable owner scope in the opaque token. Scoped snapshot checks root and control PID/creation/image before selection metadata messages. Delivery uses the same scope before its write and afterward; there is no unscoped debug fallback. `requireInputTarget` defaults true, so expected positive captures fail immediately if no native token is available. Only the deliberate readonly/password/unsupported cases explicitly allow no token, still within verified owner scope. Production capture behavior is unchanged; this debug argument/type is absent from release.

The harness verifies ready PID equals its spawned child PID, resolves its own regular image, checks own foreground before **every** begin, and supplies the native owner scope. `owned-captures.json` records only case number, owned foreground and required/captured-target booleans. It never records another application's name or content. The new adapter contract rejects PID 0 and the owned test process with an incorrect expected image before foreground capture.

GREEN on the new `9fbe2c0f…` build is in `agent-notes/input-runtime-launch/profile-positive-d021ba50270843cfbe1190ba3b82b898/`: 2 hidden IPC/restart tests pass, missing/mismatched owners reject, no finish/delivery/history from the negative case, cleanup 0 and bindings restored. The portable four-target suite passes all 30 tests. No new user fixture was opened or human repeat requested after this correction.

A newly frozen scoped helper exists at `build-cache/input-scoped-runtime-app-probe-a42N1B/manual-input/`, **inactive and not launched**. Its entry file is `Start-owned-input-test.cmd.disabled.txt`; `prepared.json` records `activeEntryPoint=false`. It pins the new `9fbe2c0f…` app plus 71 regular app/DLL/resource files (115,226,263 bytes), 548 frozen test/Playwright files (13,055,930 bytes), fixed runtime 156 and compiled source bindings. Final helper SHA256 is `f22c3bf64a957de17d84f300f165a3b08794ecb693ca0fcbdbeb8beb8fc10046`. This preserves reviewability without requiring a mutable D-target binary or another human run. Keep the input worktree present if this archived debug helper is ever intentionally enabled, because the app's debug binding export path is compiled there and must pass the guard/restore check.

## Build storage adjustment

The desktop binary consumes the Rust library directly on Windows/macOS/Linux, so `[lib] crate-type=["rlib"]` replaces unused staticlib/cdylib outputs. Before rebuilding, all owned Cargo/compiler/linker processes were stopped. The exact generated `D:/git/notype-build/debug/deps/notype_app_lib.lib` was verified regular/non-reparse, single-link, with archive magic and an exclusive handle; only that file was removed. Its length was 752,900,676 bytes, observed free-space increase 752,902,144 bytes. The external receipt is `agent-notes/input-staticlib-reclaim.json`. C++ dependency cache and app-local DLL staging were retained; no Vulkan/GPU tools or second native cache were introduced. This adjustment does not deliver foreign-host/mobile library artifacts.
