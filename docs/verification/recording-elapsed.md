# Recording elapsed / overlay ownership (#21)

Plan: [P21-v3](https://github.com/swchen44/notype/issues/21#issuecomment-6398232459).
Approval: [GPT-6-Astra APPROVED, zero unresolved](https://github.com/swchen44/notype/issues/21#issuecomment-6398249848).
Product baseline: `735b571`; mandatory review-policy docs merged from `40ff293`
before implementation. Branch: `ticket/21-recording-elapsed`.

## Observable result

Compact and streaming capture views, including streaming with no transcript,
show only elapsed digits such as `00:12` next to the capture waveform after
ready. The static accessible timer name uses i18next; `aria-live=off` avoids
announcing each tick. Native overlay window dimensions and placement are
unchanged. The capture pill grows to 208 logical CSS pixels within the existing
256-pixel compact native window so waveform, digits and Cancel do not overlap.

This is frontend **display elapsed**, starting at this overlay webview's first
accepted ready receipt. It uses a local monotonic difference, not interval tick
count, Rust Instant subtraction, another webview clock or audio sample duration.
Ready transport delay is not added back. Working/terminal/hide clears the clock.
The change does not diagnose or claim to fix #19 readiness latency.

Capture identity and request revision are separate decimal u64 strings. Native
requests reserve an immutable ticket before dispatch; queued show, ready, phase
and delayed hide validate that ticket under the same registry lock used for new
reservations. Stream text has capture identity. Stop closes capture before the
queued phase transition. No callback reads a successor's current ID to relabel
its old work. Native settings snapshots are read before the registry critical
section. The exact public contract is in [desktop-overlay-events](../desktop-overlay-events.md).

React effect cleanup is synchronous. Completed listeners are disposed once;
registrations resolving after cleanup are disposed once immediately. Callback
and settings completion have mount/capture/request fences; rejected Promises
are handled. Overlay-local language presentation prevents stale async language
work from changing a new capture's presentation.

## RED / GREEN evidence

Actual rendered production component, with only Tauri public transport and
frontend clock/scheduler replaced:

```powershell
node --test tests/integration/recording-overlay.ui.mjs
```

- Initial compact-ready test RED: no `.stimer` appeared (0 passed / 1 failed).
  After wiring display elapsed it passed.
- A stronger real CSS check then failed: computed timer opacity was `0`, not
  `1`. Setting the timer's normal visible style passed it.
- A real geometry assertion then failed: waveform right=674 exceeded timer
  left=655.515625. Reserving digit space in the capture row and using 208-pixel
  capture pills passed the nonoverlap assertion.
- The cleanup sensitivity check deliberately disabled the actual effect cleanup
  (a one-line early return), printed its diff against a pristine green copy,
  and ran `node --test --test-name-pattern='pending registration and settings'
  tests/integration/recording-overlay.ui.mjs`. RED: the listener-count condition
  timed out, 0/1 passed. The mutation was restored immediately; the named test
  then passed 1/1. No mutation remains in source.
- An intermediate fixture-only port collision was not counted as behavioral
  RED evidence. The overlay fixture now reserves its own port and uses an
  ignored task-local Vite cache and virtual entry, avoiding shared optimizer
  interference with the other UI task.

Production native lifecycle policy, without native dependencies/cache:

```powershell
rustc --edition=2021 --test tests/unittest/desktop_overlay_lifecycle.rs -o D:/git/notype-local-artifacts/recording-elapsed-lifecycle-tests.exe
& D:/git/notype-local-artifacts/recording-elapsed-lifecycle-tests.exe --nocapture
```

The initial unguarded extracted policy returned `Some("stale-ready")` where the
successor regression required `None` (RED 0/1). The production adapter now uses
the guarded policy for the actual native/event commits. Final policy suite:
3 passed / 0 failed. It covers stale queued ready/hide, duplicate capture show,
working revisions, ready after stop, final text ownership, terminal tombstones
and exact delayed-hide fencing, including IDs above JavaScript safe integers.
This policy test is not a native shell or microphone acceptance test.

## Validation and limitations

- Rendered overlay integration: 7 passed / 0 failed. Cases include compact and
  empty/text streaming, monotonic clock jumps, 00:59?01:00 and 100:00, duplicate
  ready/show without reset, ready before matching show, old ready/hide/phase/text,
  all terminal reasons, compact processing, delayed/rejected settings, pending
  and partially failed registration, StrictMode unmount/remount, one active
  listener per event, exactly-once disposal and zero residual timers.
- Frontend full ESLint and TypeScript + Vite production build passed. The Vite
  large-chunk advisory remains; it is unrelated to the elapsed contract.
- Changed frontend files pass Prettier; Rust files pass rustfmt; diff check
  passed. Streaming binding DTO fields were updated locally; parent owns final
  integrated binding generation.
- No native Cargo build/clippy/cache, native App/GUI, microphone, clipboard,
  user profile or user-process operation was performed by this implementation.
  Parent must compile the integrated native adapter and report native runtime
  positioning/permissions and real readiness separately.
- No P19 instrumentation, new diagnostic collection, package rebuild/update,
  GitHub write or issue closure is included.
