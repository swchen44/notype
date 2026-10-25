# Readiness observability / P19-latency-v3

Approved plan: [P19-latency-v3](https://github.com/swchen44/notype/issues/19#issuecomment-6398258525).
Approval: [GPT-6-Astra APPROVED, zero unresolved](https://github.com/swchen44/notype/issues/19#issuecomment-6398268939).
Hard gate: [P20/P21 exact implementation record](https://github.com/swchen44/notype/issues/19#issuecomment-6398707430).
Initial implementation base was clean `160ea647c3e4dfa9fa3cc5b8179b2c3a08a0a55d`.
The #19 branch fast-forwarded to that base before changes. The older clean P21
branch was preserved. This receipt concerns instrumentation and synthetic seams,
not an observed hardware latency fix or a packaging/update operation.

## Production changes

- A public std-only production startup service delegates the existing native
  ensure/open ? request_capture operations through RecorderPort. The manager
  retains its original state/mode/generation and recorder-lock publication
  lifetime; collecting diagnostics OFF cannot prevent capture.
- The real CaptureProcessor wraps its existing nonempty captured-chunk processing
  closure in the shared processed-ready handoff. Visualizer/resampler/VAD/audio
  callback code is not duplicated. The real-time CPAL callback is unchanged.
- The actual waiter carries the processed observation Instant; the actual main
  queue carries its immutable trace and original P21 presentation ticket. Worker
  dequeue is captured before logging. New model jobs retain their initiating
  trace; preexisting shared jobs are unassociated.
- Bounded app-run attempt ordinals, collector epoch and preference revision
  reject stale records after Clear or OFF?ON. Native capture/owner IDs stay only
  in memory. Reports contain explicit span origins, pending-stage incomplete
  summaries, ring truncation, capped duration flags and unavailable boundaries.
- The real overlay sends received/local React-commit/settings observations via
  a typed, nonblocking production command. Native command validates the overlay
  webview and current P21 envelope. Received is a backend round-trip upper bound;
  local commit is not paint. No frontend absolute clock crosses IPC.

Public API, exact origins and privacy limits:
[capture-readiness-timing](../capture-readiness-timing.md). New native command is
`report_capture_observation(captureId, captureRevision, observation, durationMs)`;
observation is `received | committed | settings`, result boolean. Parent owns
final integrated Tauri binding generation and native compilation.

## Feedback loop and RED/GREEN evidence

Agent-runnable portable commands (no Cargo/native cache):

```powershell
rustc --edition=2021 -A dead_code --test tests/unittest/capture_startup.rs -o D:/git/notype-local-artifacts/capture-startup-tests.exe
& D:/git/notype-local-artifacts/capture-startup-tests.exe --nocapture
rustc --edition=2021 -A dead_code --test tests/integration/capture_readiness.rs -o D:/git/notype-local-artifacts/capture-readiness-tests.exe
& D:/git/notype-local-artifacts/capture-readiness-tests.exe --nocapture
```

The unused-code allowance is for standalone harness imports of the whole portable
production module, whose native-only entry points are not used by that harness.
It is not a native build/clippy result.

Baseline controlled startup/processing/waiter/main-delivery fixture passed. It
substitutes only clocks, hardware operations and scheduling, while using the
production handoff and time math. Independent delays 120ms stream open, 80ms chunk
processing, 90ms waiter scheduling and 110ms main delivery appeared in their
respective local spans. Combined processed cumulative=200ms and total=400ms were
checked against independent literals. Dequeue=30ms despite later action entry
and separate command queue=70ms are covered.

A deliberate one-line mutation of the **production** chunk-processing origin
changed subtraction to processed_at?processed_at. Its diff against the pristine
source was printed before compilation. Named sensitivity test
`independent_native_and_scheduler_delays_are_reported_in_their_actual_production_spans`
went RED: actual `Some(0)` vs expected `Some(80)` (0 passed / 1 failed). The
mutation was restored immediately and the complete suite passed again. This
proves sensitivity to incorrect instrumentation; it is not a reproduction of
slow real hardware.

A separate real report regression, written before its fix, exposed a stopped
attempt incorrectly marked complete while its model stage still had Started
without completion. `stopped_attempt_with_an_unfinished_measured_stage_is_reported_incomplete`
was RED (0/1), then GREEN after pending measured stages were included in summary
completeness. Numeric cumulative boundary points are not pending operations.

Rendered command:

```powershell
node --test tests/integration/recording-overlay.ui.mjs
```

The actual React component saw independently delayed ready delivery (110ms), a
local commit delay (75ms), and settings resolution (500ms). Ready digits appeared
while settings was still pending. Typed emitted observations matched those local
origins; duplicates did not reset or add observations. Backend ACK registry tests
separately verify round-trip origin/dedup/epoch and oversized local durations
marked capped, without cross-clock subtraction.

## Results and limits

- Startup public contract: 4/4 passed; no-wrap ordinals, independent delays,
  dequeue/command origins, empty/cancelled readiness and no replay.
- Capture report/ACK integration: 7/7 passed; epoch/disable, old/successor ACK,
  shared model job correlation, missing completion, capped local durations,
  truncated ring and precise local/round-trip fields.
- Existing physical tap contract: 6/6 passed; existing overlay policy: 3/3 passed.
- Existing diagnostic report/service std contracts: 2/2 + 2/2 passed.
- Actual rendered overlay suite: 8/8 passed. Full frontend ESLint, TypeScript +
  Vite build passed; the existing Vite large-chunk advisory remains.
- Changed frontend/Rust formatting and diff checks are performed before handoff.
- Native Cargo/clippy/build/binding generation, native App/runtime, real mic,
  clipboard, live user profile/processes and cold/warm hardware observation were
  not performed. No artifact/profile replacement or automatic upload occurred.
- These passing synthetic loops establish observation sensitivity. The user's
  original wait after an effective Right Alt short tap is still not reproduced;
  no driver/model/IPC root cause, latency semantic fix, p95 or hardware acceptance
  is claimed. #19 latency remains open pending safe user baseline and a separately
  reviewed concrete fix if the evidence calls for one. #22/#23 are not included.
