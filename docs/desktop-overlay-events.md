# Recording overlay events

Approved [P21-v3](https://github.com/swchen44/notype/issues/21#issuecomment-6398232459)
and [GPT-6-Astra approval](https://github.com/swchen44/notype/issues/21#issuecomment-6398249848).

The recording overlay is a projection of the current process-local capture owner.
`CaptureId` is the workflow generation returned at begin; it is not a saved text
row ID, target token or device identifier. The native `OverlayLifecycle` public
policy accepts `reserve`, then `commit` / `ready` / `text`. The native adapter
serializes reserve and actual window/event commits with the same registry lock.
Callbacks capture their immutable ticket before main-thread or worker dispatch;
delayed hide never replaces the captured identity with the current one. Settings
snapshots are read before entering this registry critical section.

`OverlayRevision` is a distinct monotonic presentation request ordinal. Duplicate
show of the same capture/presentation reuses its revision. Working and hidden
presentations close capture; lower-stage requests cannot reopen it, and hidden
captures cannot be revived. A newer capture invalidates all older queued work.

| Event | Payload |
|---|---|
| `show-overlay` | `{ captureId, revision, state }`, state recording / streaming / transcribing / processing |
| `recording-ready` | `{ captureId, captureRevision }`, matching the original capturing show ticket |
| `hide-overlay` | `{ captureId, revision, reason }`, reason closed / finished / cancelled / failed |
| `stream-phase-event` | `{ capture_id, revision, phase, kind? }`, working closes the capture clock |
| `stream-text-event` | `{ capture_id, committed, tentative }`, only the matching nonhidden capture can display it |

Every identity/revision field is a decimal u64 **string**, preserving JavaScript
precision. Rust serializable DTOs are in overlay.rs; frontend show/ready/hide
contracts are in `desktop/src/overlay/contracts.ts`. Streaming DTOs remain in the
Tauri bindings. The existing streaming event names and snake_case field convention
are retained. No extra diagnostic data or public test command is introduced.

The frontend accepts the current identity synchronously before nonessential
settings I/O. Old events, lower revisions and ready-after-stop cannot reopen a
capture. At most one identity-scoped early ready is retained until its matching
show. Terminal hide is a tombstone, including when received before show.

The digits are **display elapsed** from the first accepted ready receipt in this
overlay webview. They use `floor((performance.now() - origin)/1000)`, refreshed
approximately every 250 ms; two-digit minimum minutes allow `00:59`, `01:00` and
`100:00`. Repeated ready/show does not reset the origin. No backend Instant or
other webview clock is subtracted. This display neither measures audio sample
duration nor diagnoses the user's waiting before ready. Working/terminal/hide
stops and clears it; arming has no running timer. Its accessible name is static
and `aria-live=off`; visible capture time contains digits and colon only.

Each mounted React effect synchronously returns cleanup. Completed registrations
are disposed once, pending registrations are disposed once if they finish after
cleanup, and rejected registration/settings Promises are handled. Event handlers
and settings completion check the mount/capture/request fence. Overlay-local
language/position presentation cannot be revived by an old Promise.
