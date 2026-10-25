# Production API probe bridge — offline checkpoints

Date: 2026-10-26. Issue [#23](https://github.com/swchen44/notype/issues/23).
Approved plans: [P23-live-v1](https://github.com/swchen44/notype/issues/23#issuecomment-6399600403)
and [review](https://github.com/swchen44/notype/issues/23#issuecomment-6399640205),
[portable source gate](https://github.com/swchen44/notype/issues/23#issuecomment-6400592627)
and [review](https://github.com/swchen44/notype/issues/23#issuecomment-6400608206).
Dependency gate is recorded in [6400857513](https://github.com/swchen44/notype/issues/23#issuecomment-6400857513).

This is source/offline evidence, not a completed user binary or paid acceptance.
No App, microphone, clipboard, current profile, real key or paid API was used.

## Actual production seam

The existing ChatProcessor factory/parser constructs the actual request once.
Its new probe-only factory disables proxy inheritance, limits connect wait to five
seconds and explicitly serializes nonstreaming requests. Ordinary App construction
keeps its prior policy. A safe endpoint enum is derived from the actual URL; no URL
or credential enters the ledger. Safe message-count/plain-text facts accompany
actual serialized byte count. Safe receipts retain total tokens, a nonempty
reasoning flag and malformed-usage flag without archiving reasoning or bodies.
Individually valid history counts survive invalid optional provider metadata.

BudgetProcessor composes the provider observer with the existing SessionService
observer. Durable reservation precedes the core possibly-sent update and HTTP
polling. Response contract failures persist a halt in the common observer before
automatic reverse admission. Fees remain unknown; reservation is not actual money.

The formal target is the exact public DeepSeek HTTPS endpoint. Explicit owned
loopback admission exists only under `cfg(test)`, shares the controls/state/store
implementation and preserves the actual loopback target/class. Tests never label
localhost as a verified DeepSeek endpoint or prove public TLS. Formal helper build
and concrete native console acceptance remain pending.

## RED to GREEN and observed results

- First real tracer compiled against the previous source: missing bridge/probe
  factory caused compiler RED. After implementation, unexpected forward model
  halted before the automatic reverse while the actual history retained its
  returned usage. Initial source checkpoint: `bfecf99d086db4dd5021b4955ccabf97178737ef`.
- A malformed negative reasoning-token response initially passed the guard:
  **one failed / zero passed**. The safe malformed-usage flag fixes that loss of
  provenance; the three then-current integration tests passed.
- The fixed schedule had compiler RED before ProbeRunner existed. It now executes
  eight real loopback requests through SessionService and the actual provider:
  forward/reverse share one history, cancellation is requested 25ms after the
  durable sent boundary, and the single scheduled retry shares cancellation's
  original history. Processing runs in its own task during the cancellation wait.
- Current bridge suite: **six tests passed**, no failure/ignore. The error test
  independently covers 12 fixtures: 401, 429, malformed JSON, oversized response,
  inconsistent totals, input/output limit violations, forbidden reasoning,
  malformed cache usage, truncated result, missing total and a local timeout.
  All stop after one reserved request without automatic retry.
- Live target versus actual loopback, and test fixture versus actual HTTPS
  loopback, each prove zero TCP sends. Unarmed round and actual SQLite dispatch
  failure likewise prove no HTTP polling. A core dispatch failure conservatively
  retains its already committed reservation.
- The successful eight-slot fixture retains 835584 reservation units, six history
  IDs and seven reviewable steps. Forward/reverse totals are 34 input / 10 output;
  cancelled/retried history has two calls, unknown full input and known subtotal
  17. Those are fixture tokens, not DeepSeek measurements.
- Python's independent standard-library CSV reader verifies BOM UTF-8, exactly
  six rows, pair aggregates/back-translation, cancelled-call partial totals and
  unknown fees, with no fixture credential in the exported bytes. An unexpected
  model fixture independently exports and parses five history rows.

Command under the reviewed sole resource lease:

```powershell
cargo test --release --offline --locked -vv --manifest-path tests/integration/desktop-contracts/Cargo.toml --test deepseek-probe-bridge
```

Root test source: `tests/integration/deepseek_probe_bridge.rs`. Fixtures own their
temporary SQLite stores, listeners and exports; no actual user history is opened.
Affected full core/provider/workflow checks, final independent source reviews,
native console/helper build and one-shot user launch remain pending.

## Resource evidence and limits

The first attempt incorrectly inherited App release-profile overrides and rebuilt
dependencies; the resource guard stopped that owned Cargo tree. This was a
resource failure, not test RED. Its ledger and all 372 identified derived cache
paths remain; an exact-file rollback was automatically rejected before execution,
and no cleanup occurred.

Work resumed only under separately approved no-delete resource plan
[6401040948](https://github.com/swchen44/notype/issues/23#issuecomment-6401040948) /
[review](https://github.com/swchen44/notype/issues/23#issuecomment-6401056228),
and cumulative remaining-space supplement
[6401113135](https://github.com/swchen44/notype/issues/23#issuecomment-6401113135) /
[review](https://github.com/swchen44/notype/issues/23#issuecomment-6401126474).
One persistent baseline tracks the greater of volume consumption and unique-inode
allocated growth; baseline-file removal cannot credit new allocations. A separate
worker scans allocations, so its approximately 1.2–1.8s scans do not stall the
active volume checks (observed maximum 216ms). The cumulative observed high-water
mark through the six-test matrix is 32542720 bytes; the 192MiB cap, proactive
160MiB stop and drive floors remain in force. No native App, new target,
dependency download, package version change or cleanup is authorized by this
receipt. Console feature unification still requires a matching cache/cost audit.

## Native/helper source checkpoint — not compiled

Fixed helper, native console/known-folder/space adapters, source launcher and
nine-case hidden owned-console fixture are now written and formatted only.
Native port's missing-module compiler RED was observed first. The portable console
algorithm additionally had a real late-interrupt RED after Enter/restoration;
its final interrupt check now passes **seven unit tests**. Those port tests do not
prove Windows APIs or the new helper/fixture executables compile or run.

Source adds only locked windows-sys0.61.2 features Foundation, Console, Threading,
Com, Shell, Security and Storage_FileSystem. Security's exact
[plan](https://github.com/swchen44/notype/issues/23#issuecomment-6401238944) /
[approval](https://github.com/swchen44/notype/issues/23#issuecomment-6401254722)
supports CreateProcessW's type gate; storage's
[plan](https://github.com/swchen44/notype/issues/23#issuecomment-6401375965) /
[approval](https://github.com/swchen44/notype/issues/23#issuecomment-6401385514)
supports quota-aware GetDiskFreeSpaceExW. No crate version changes/downloads.
The actual cache reverse-closure audit could not bound feature-unification/new
binaries/link peaks within the remaining budget. **No new union build was tried.**
Full helper/Windows fixtures, actual binary/hash, pre-live docs and user launch
remain pending; source launcher is not a ready key-entry handoff.

Native source verifies singleton PID using a valid buffer, rejects redirected
input, decodes UTF-16/repeats, and keeps callback state process-static. Borrowed
stdin is never closed, and unregister is not treated as joining callbacks. Exact
restoration precedes key handoff and bounded nonsecret ReadConsoleW confirmation;
pending input is cleared between phases. Existing rounds are report-only before
any key prompt. A marker is created only after final local confirmation. An
unpinned build cannot ask for a key: NOTYPE_PROBE_SOURCE_COMMIT is a nonsecret
compile-time 40-hex source pin, not a credential/config override.

Fixture source requests CREATE_NEW_CONSOLE and STARTF_USESHOWWINDOW/SW_HIDE0,
checks actual startup info/singleton ownership, injects only nonsecret records and
observes actual modes. Nine cases cover Unicode/repeats/Backspace, Escape,
Control-C key, read error, unwind, real Control-Break, late restore interruption,
disable failure and restore retry. Parent reaps only returned owned process
handles and never broadcasts from its own console. These are **not performed**,
not skipped/passed. Close/power-loss cleanup is best effort; console isolation
avoids modifying the user's existing terminal.

The independent core manifest does not enable the blocked desktop Windows feature
union. Its affected full tests passed **43 plus one doc example** after the safe
receipt additions (same exact locked dependencies). That command observed a
cumulative high-water mark of **37019648 bytes**, maximum active volume interval
215ms, and no guard stop. New Windows/helper targets remain not performed.
