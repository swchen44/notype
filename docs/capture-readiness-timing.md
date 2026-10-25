# Capture readiness timing contract (#19)

Approved [P19-latency-v3](https://github.com/swchen44/notype/issues/19#issuecomment-6398258525),
[Astra APPROVED / zero unresolved](https://github.com/swchen44/notype/issues/19#issuecomment-6398268939).
The [hard dependency record](https://github.com/swchen44/notype/issues/19#issuecomment-6398707430)
pins approved P20/P21 and their integrated implementation commits. This change
adds observability and production test seams; it does not diagnose or fix the
reported long wait after an effective Right Alt short tap.

## Privacy and authority

`capture_startup.rs` owns the production open ? capture-command ? processed-ready
handoff and duration math. `RecorderPort` substitutes native operations without
moving the manager's state/mode/generation policy. Its native port retains the
original recorder guard through publication of recording state. The actual
CaptureProcessor delegates its captured-chunk processing closure to the shared
handoff; no VAD/resampling algorithm is duplicated. The real-time CPAL callback
is unchanged and never takes a diagnostic lock.

`AcceptedAttempt` reserves a nonzero app-run u32 ordinal through a lock-free
counter at accepted start intent; exhaustion returns unavailable forever, never
wraps/reuses. The worker captures dequeue before existing logging/action work.
Direct non-keyboard action starts allocate at their own entry and report queue
timing unavailable. No keyboard values, persistent session/history IDs, device
names, model IDs, target tokens, paths, text/audio or credentials enter records.

`CaptureTrace` immutably binds ordinal, backend origin, collector clear epoch and
preference revision. P20 validates epoch/revision/effective ON atomically with
record insertion. Clear and OFF?ON cannot resurrect old traces. Collection state
never controls recording, text history or audio resource ownership. The in-memory
capture-owner-to-trace mapping is not exported or persisted. Attempt terminal
flags close observations only; existing native generation checks still govern
ready UI/cue and cancellation.

## Durations and actual boundaries

All exported durations are integer milliseconds; sub-millisecond fractions are
truncated. Values beyond 86,400,000 ms are capped and explicitly marked
`capped=true`. A frontend value must be finite and safely representable; IPC
accepts only a nonnegative integer u64 and the collector applies the same cap.
`elapsed_ms` remains relative to collector initialization/reset at event insertion.
It is not a physical key timestamp or frontend receipt time.

| Typed span | Origin ? endpoint / meaning |
|---|---|
| dispatch-queue | accepted intent Instant ? actual worker dequeue; unavailable for direct action starts |
| action-cumulative | action trace entry ? open/request return or early failure |
| stream-open | native ensure-stream entry ? return; opened / reused / reopened are finite disposition outcomes |
| command-queue | capture command send ? consumer handling before begin_recording reset; includes consumer scheduling/logging |
| consumer-first-chunk-cumulative | action entry ? first nonempty captured chunk entering the processing wrapper, after ring consumption |
| chunk-processing | wrapper entry ? actual visualizer/resampler/VAD/audio_cb processing closure completion |
| processed-ready-cumulative | action entry ? processed first-chunk observation before readiness send |
| waiter-scheduling | processed-ready observation ? dedicated waiter return; includes observation/send/scheduler overhead |
| ready-queue-cumulative | action entry ? ready main-thread submission; reveals otherwise unmeasured post-wait/debug delay without mixing clock origins |
| main-queue | submission ? validated main-thread emission-adapter entry; includes callback preparation, not one-way IPC |
| frontend-round-trip-upper-bound | backend emission attempt ? received ACK arrives back; includes native submission, delivery, frontend and return IPC |
| frontend-commit-local | same overlay webview's ready receipt ? React layout-effect commit; not paint |
| frontend-settings-local | same overlay webview's show receipt ? current settings Promise resolution |
| model-job | job actually spawned by this attempt: worker entry ? load return; already loaded is reused/no duration; preexisting shared load is unassociated |
| cue-call-including-playback | blocking feedback function entry ? return, including its setup and entire playback; not audible-onset time |

Cumulative and local spans overlap and must not be added as sequential stages.
No frontend absolute performance clock is sent or subtracted from Rust Instant.
Raw hardware callback?ring?consumer subcomponents and actual audible onset remain
unavailable. Existing ShortcutAccepted is worker-observed; its name is not proof
of physical acceptance. Legacy unassociated stage rows remain distinct from the
new `phase=capture-timing` rows.

## Native/frontend seam

`report_capture_observation` is a production diagnostic ACK command, restricted
to the `recording_overlay` webview. Arguments use camelCase:
`captureId: string`, `captureRevision: string`,
`observation: received | committed | settings`, `durationMs: number`.
The command parses decimal u64 envelopes, checks the exact current P21 capturing
presentation, then validates active trace/collection generation and per-kind
deduplication. Stale/duplicate/disabled observations return false, without
changing the overlay or exposing an error popup. Owner IDs stay only in memory.
The received ACK reports a backend round-trip upper bound; local commit/settings
durations originate only in that same frontend webview.

The real React component sends received once on first accepted ready, committed
once after the corresponding layout commit, and settings only after current
request completion. None of these waits for an ACK before showing readiness.
Ready/show duplicates retain the existing P21 display elapsed origin. The display
counter remains display elapsed, not audio sample duration.

## Reports and missing evidence

The same P20 bounded ring retains at most 128 records. `ring_truncated` and
`events_discarded` make eviction visible. Each retained attempt summary requires
its start, terminal and all explicitly started measured spans to complete;
otherwise it is `incomplete=true`. Numeric cumulative point observations are not
pending operations. Missing start after eviction is explicitly truncated; no
adjacent-row subtraction or guessed attempt pairing is performed. Cancelled or
replaced partial attempts may legitimately remain incomplete. Model jobs spawned
by an earlier capture can finish with that original ordinal; shared existing work
never acquires the successor's ordinal. No history metric or billing model is
introduced.

Portable/root seams: `tests/unittest/capture_startup.rs` drives the production
startup/processed-ready/dispatch module with native/scheduler adapters and a
controlled backend clock; `tests/integration/capture_readiness.rs` drives the
actual diagnostic service/report and ACK registry. Rendered overlay tests use
the real component and public Tauri boundary, with a separate frontend clock.
They are synthetic signal-sensitivity evidence, not a reproduction of slow audio
hardware. Native compilation and hardware cold/warm observations remain separate.
