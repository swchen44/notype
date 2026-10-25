# Input and tray follow-up (#19)

Date: 2026-10-25. Branch: `ticket/19-input-tray`. Base: `ac06224`.

## What is established

The actual public physical gesture router accepts Right Alt down/up as Dictation
while idle, and the same down/up as Stop while recording. Its existing six
contract tests passed before any change. The router has no hold threshold or
clock. This proves the tap contract at that boundary; it does **not** reproduce
or diagnose the reported need to hold Alt / wait for capture. No microphone,
user App, clipboard, user profile, or unrelated process was exercised here.

The close handler previously prevented CloseRequested and unconditionally hid
the window. Tray startup availability depended only on settings booleans. A
missing/failed native tray could therefore remove the return path.

## Red / green feedback loop

Approved public seam: main-window close-to-tray platform policy, used by the
actual native CloseRequested handler. Tests live in
`tests/unittest/desktop_window_lifecycle.rs`. The policy was first extracted with
the existing unconditional-hide behavior, then the failure test was run:

```powershell
rustc --edition=2021 --test tests/unittest/desktop_window_lifecycle.rs -o D:/git/notype-local-artifacts/input-tray-window-tests.exe
& D:/git/notype-local-artifacts/input-tray-window-tests.exe --nocapture
```

RED: `close_keeps_window_visible_when_native_tray_cannot_be_restored` failed:
actual `Ok(())`, expected `Err("tray-unavailable")`; 0 passed / 1 failed.

GREEN after adding the native-return-path gate: 2 passed / 0 failed. The second
case confirms native tray confirmation happens before hiding. This is a fast,
agent-runnable policy regression test, not proof of native shell behavior.

Existing physical gesture loop:

```powershell
rustc --edition=2021 --test tests/unittest/physical_gestures.rs -o D:/git/notype-local-artifacts/input-tray-physical-tests.exe
& D:/git/notype-local-artifacts/input-tray-physical-tests.exe --nocapture
```

Result: 6 passed / 0 failed, including sided modifiers, release-only commit,
common Recording-only Stop, repeats, cancellation and suspended input draining.

## Resulting behavior

- Shared tray factory returns resource/build errors instead of panicking. A
  failed creation leaves the main window available; a missing handle can be
  recreated on the next close request.
- Closing requires the native tray to be available. Startup with start-hidden
  also checks the actual return path. Disabled/unavailable tray leaves the
  window visible and Close shows a localized toast (en/zh-TW/zh-CN).
- Windows verifies registration by updating its existing version tooltip. The
  installed tray-icon Windows implementation forwards `Shell_NotifyIconW`
  `NIM_MODIFY` failure for this operation. In contrast, its constructor and
  `set_visible` can return success even if native `NIM_ADD` fails. A failed probe
  triggers hide/show recovery, then a second registration probe.
- This does not depend on `rect()`: notification icons may be in shell overflow,
  and failure to obtain a rectangle is not proof that registration is absent.
  Windows still opens the app on left click/double click, presents the tray menu
  on right click and explicitly exits through Quit.
- macOS/Linux retain their supported native visibility operation and platform
  menu conventions. Their runtime behavior has not been tested here.

The native registration reasoning uses the pinned local tray-icon source
(`src/platform_impl/windows/mod.rs`, constructor, `set_visible`, `set_tooltip`)
and Microsoft?s documented [Shell_NotifyIconW actions and BOOL return contract](https://learn.microsoft.com/en-us/windows/win32/api/shellapi/nf-shellapi-shell_notifyiconw).

## Capture evidence added for the unresolved input report

The bounded diagnostics ring now receives enum-only phase/outcome/duration
probes from the actual shortcut worker, action start, audio stream opening,
first-sample readiness and recording stop. The native keyboard callback only
queues an intent plus monotonic Instant; it does not log, export key values or
access the diagnostics mutex. Dispatch reports queue duration. RecordingStart
covers pre-capture setup, AudioStreamOpen covers stream preparation, and
MicrophoneReady reports first samples only for the current recording owner.
Started/failed/cancelled states distinguish a stalled stage from a successful
one. No captured trace yet establishes which stage caused the user?s wait.

The diagnostics API/module and removal of the legacy Hold Threshold control
are supplied by the coordinated diagnostics branch. Native compilation requires
that branch to be integrated. Explicit in-window Quit is coordinated there too.

## Validation scope

- Unit: physical gestures 6/6; window lifecycle 2/2.
- Frontend: full ESLint and TypeScript + Vite production build passed; changed
  frontend files formatted with Prettier. Vite reports the existing large chunk
  advisory.
- Rust: changed files formatted with rustfmt; `git diff --check` passed.
- Native Cargo/clippy/build: not performed here, to avoid concurrent access or
  creation of native caches; parent performs the integrated build.
- Real Windows tray registration/overflow, X-hide, restore click, explicit Quit,
  no-tray fail-visible behavior: not performed here; require an owned fixture
  process and must be reported separately before claiming native acceptance.
- Real microphone readiness/short tap latency: not performed; the router pass
  must not be represented as an end-to-end recording fix.
