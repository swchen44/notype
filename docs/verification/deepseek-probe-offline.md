# P23 independent offline preparation receipt

Approved scope: [P23-offline-prep-v1](https://github.com/swchen44/notype/issues/23#issuecomment-6399793622),
[GPT-6-Astra APPROVED / zero unresolved](https://github.com/swchen44/notype/issues/23#issuecomment-6399801208).
Original [P23-live-v1](https://github.com/swchen44/notype/issues/23#issuecomment-6399600403)
and [approval](https://github.com/swchen44/notype/issues/23#issuecomment-6399640205)
remain binding. Base integration: `c07937189d60f782fa4512367f93b35813bb5bb4`.

Implementation is the reusable `deepseek_probe` module: pinned final-wire policy,
fixed eight slots, admission/reservation state machine, actual owned SQLite FULL
store and console mode/lifetime algorithm behind a system port. Public boundaries
and remaining gates are documented in [contract](../deepseek-probe-offline.md).
There is no App registration, native console adapter, production observer bridge,
HTTP invocation, launcher or user binary in this slice.

## RED → GREEN

Tests were added at approved public policy, admission/storage and console seams.
Observed runtime RED examples before implementation:

- Full-wire 8193 bytes returned `Ok` instead of `WireContract`; default/expanded
  capabilities were accepted; fixed schedule was empty. Policy now passes 3/3.
- A storage commit failure still returned a send permit. Reservation totals
  were zero; a failed forward contract still admitted slot 6; an unarmed/reopened
  round could admit/rearm; differing reverse history was accepted. These now
  fail closed, with 9/9 round lifecycle tests passing.
- Actual SQLite store initially could not create/reopen the owned round. The
  adapter now passes 6/6 real-files tests, including a child process exiting
  without destructors after a durable reservation. Restart remains report-only.
- Fixture input was read while echo was enabled; abort/control/length handling
  and Unicode Backspace were missing; a simultaneous interrupt was ignored.
  The guard now passes 6/6 console-port tests, including read/setup error,
  unwind, redirected input and restoration failure.

These are controlled offline failures, not reproduction of a live provider,
native console, hardware or user-latency defect. Additional adapter cases check
missing/corrupt artifacts, exclusive lock, eight persisted outcomes and unchanged
foreign data. No fake provider is used or accepted as API evidence.

## Commands and results

PowerShell from the dedicated `ticket/23-deepseek-probe` worktree. Test executables
use task-owned files under `C:/Users/swche/AppData/Local/notype-study/agent-notes/`.
Each `rustc` must succeed before its executable runs.

```powershell
rustc --edition=2021 -A dead_code --test tests/unittest/deepseek_probe_policy.rs -o C:/Users/swche/AppData/Local/notype-study/agent-notes/p23-policy.exe
& C:/Users/swche/AppData/Local/notype-study/agent-notes/p23-policy.exe
rustc --edition=2021 -A dead_code --test tests/unittest/deepseek_probe_round.rs -o C:/Users/swche/AppData/Local/notype-study/agent-notes/p23-round.exe
& C:/Users/swche/AppData/Local/notype-study/agent-notes/p23-round.exe
rustc --edition=2021 -A dead_code --test tests/unittest/deepseek_probe_console.rs -o C:/Users/swche/AppData/Local/notype-study/agent-notes/p23-console.exe
& C:/Users/swche/AppData/Local/notype-study/agent-notes/p23-console.exe
```

The actual SQLite fixture used only read-only existing Release dependencies,
with the Cargo lease holder informed; no Cargo command, target write or download:

```powershell
rustc --edition=2021 -A dead_code --test tests/integration/deepseek_probe_round.rs -o C:/Users/swche/AppData/Local/notype-study/agent-notes/p23-sqlite.exe --extern rusqlite=D:/git/notype-build/release/deps/librusqlite-90012f4035d3bfdd.rlib -L dependency=D:/git/notype-build/release/deps -L native=D:/git/notype-build/release/build/libsqlite3-sys-b72ad247cb31790d/out
& C:/Users/swche/AppData/Local/notype-study/agent-notes/p23-sqlite.exe
```

Results: policy **3 pass**, round **9 pass**, console port **6 pass**, actual
SQLite integration **6 pass**; **24 total**, zero failed/ignored. Source/test
`rustfmt --edition 2021 --check` and `git diff --check` pass. There are no frontend
changes; frontend lint/build was not repeated. No Cargo/native build/Clippy ran.
The std-only policy/round/console checks are registered in core-contracts CI;
that workflow run is not claimed here. SQLite harness registration is deferred
until the #22 contract gate; no shared manifest/lock was modified.

## Not performed / incomplete

- Full #22 merge gate and actual production observer/history/CSV integration.
- Native Windows console adapter/features compile and dedicated-console fixture:
  pending parent lease/audit, explicitly not a fake-port pass.
- Final-wire/response parser and model/usage halt integration, direct-TLS policy,
  current official price/contract and returned-model allowlist pin.
- Complete live-v1 offline acceptance, user binary/hash, launcher and local key
  step. No actual key was requested, obtained, read or persisted.
- Paid API, live cancellation/retry/translation quality, actual API fee receipt.
  Reserved US$0.0835584 is not an actual fee or provider-enforced dollar cap.
- User App/profile, mic, clipboard, company environment or installation actions.

#23 remains open. Parent #8 retains every unfinished gate and acceptance item.

## Offline source-review fixes (2026-10-26)

The independent reviews pinned `8a273e980854b65ffe074fa8ba31b356111dd7f3`
(same tree as integration `83639c90b79eccc4681d0a90902de03223533149`) and found
one Spec/P2 and one Standards/P3 issue. The exact
[P23-offline-review-fix-v1 plan](https://github.com/swchen44/notype/issues/23#issuecomment-6400259566)
received [GPT-6-Astra APPROVED / zero unresolved](https://github.com/swchen44/notype/issues/23#issuecomment-6400277302)
before this follow-up's implementation/test edits. Original results above remain
the historical preparation receipt.

**P2 original finding:** SQLite accepted individually valid rows which could not
form a Round snapshot, such as slot 1 NotRun followed by slot 2 Finished, or slots
5/6 using different history IDs. This was an evidence acceptance/reporting defect;
review did not establish additional sends or a budget-cap bypass.

A single production snapshot validator now serves actual load and candidate
commit; current data is also read/validated inside the IMMEDIATE transaction
before any UPDATE. Candidate revision advances exactly once and must match the
current compare revision. Invalid current data cannot be washed into valid-looking
evidence. Schema/round identity and eight unique ordered row positions remain
checked at decoding; the fixed array represents the slots without duplicate keys.

The validator checks a continuous occupied prefix, serial completion, allowed
slot-6 skip, slot-7 cancellation/race continuation, fatal terminal/halt consistency,
revision operations, unique local call IDs and exact history groups. The fixed
reservation count/product stays within eight and the reviewed budget; schema 1
has no independent persisted fee/total column. This does not calculate actual fees.

Missing external history/call rows are not guessed or queried. Positive fixtures
use actual Round transitions whose local IDs are already reserved even if another
store never committed or HTTP never started. NotRun/Skipped IDs remain empty;
no new blank-ID reserved state or future bridge ID mapping was invented.
Corrupt/inconsistent state returns only `Denied::Storage`, never resets, and
reopened evidence remains report-only. Invalid compare commits preserve the
prior logical rows.

**Actual RED → GREEN:**

- `sqlite_rejects_individually_valid_rows_with_an_impossible_slot_prefix`: created
  a real schema-1 owned database, then injected individually valid rows with slot
  2 Finished while slot 1 remained NotRun. Before validation, the named case was
  **0 passed / 1 failed** because actual `open_owned` accepted the impossible state.
  After the shared validator, it was **1 passed / 0 failed**.
- `sqlite_rejects_impossible_identity_completion_and_halt_combinations`: the first
  actual corruption (slot 2 reusing slot 1's call ID in an otherwise valid completed
  eight-slot round) was likewise accepted, giving **0 passed / 1 failed**. After
  validation, **1 passed / 0 failed**, including all eight logical corruption
  variants: duplicate call, split 5/6 group, split 7/8 group, merged distinct group,
  unfinished earlier slot, fatal failure followed by later slots, missing fatal
  halt, and incompatible revision.
- These RED cases observed actual successful storage opens, not malformed SQL,
  compile errors, wrong locators or zero tests. Fixture writes touch only verified
  task-owned temporary directories. No fault injection remains in product code.

Additional actual SQLite regressions include **44 valid transition-prefix
fixtures**: every completed prefix, pending crash boundary, explicit halt with
pending receipt, ordinary failure, halt before final receipt, both planned slot-7
cancellation outcomes with skipped slot 6 and slot 8, and halt before arming.
Reopened reservation counts, units, halt and local slot state/IDs are preserved.
The compare-commit case rejects an invalid candidate, skipped/stale revisions and
an invalid current snapshot without changing logical rows; a valid commit positive
control confirms rejected writes did not poison the owner.

**P3 original finding:** new public fields/variants and ConsolePort/RoundStore
methods lacked method-level API documentation. Every public field/variant now
has purpose documentation. Port methods specify owned interactive input, private
buffer ownership, bounded polling, interrupt lifetime/restore/retry and cleanup
errors. RoundStore methods specify exclusive access, complete validation,
revision compare, durability before permit, and the possibility that a failed
commit already completed: halt, never refund/resend, reopen for reporting only.
These comments do not change console, budget or admission semantics.

### Follow-up commands and results

Compiler: `rustc 1.99.1 (58cf8f9c3 2026-10-13)`. From the diagnostics worktree,
all outputs use task-owned TEMP paths, never the leased native/core target.

```powershell
foreach ($part in @('policy','round','console')) {
  $rootTestPath = "tests/unittest/deepseek_probe_$part.rs"
  $testExePath = Join-Path $env:TEMP "notype-p23-review-$part.exe"
  rustc --edition=2021 -A dead_code --test $rootTestPath -o $testExePath
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
  & $testExePath
  if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
}
rustc --edition=2021 -A dead_code --test tests/integration/deepseek_probe_round.rs -o "$env:TEMP/notype-p23-review-sqlite.exe" --extern rusqlite=D:/git/notype-build/release/deps/librusqlite-90012f4035d3bfdd.rlib -L dependency=D:/git/notype-build/release/deps -L native=D:/git/notype-build/release/build/libsqlite3-sys-b72ad247cb31790d/out
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
& "$env:TEMP/notype-p23-review-sqlite.exe"
```

The two RED/GREEN runs used the SQLite compilation above, then the corresponding
full named test with `--exact`. Final results: policy **3**, round **9**, console
port **6**, SQLite **10** passed: **28 total / 0 failed / 0 ignored**, retaining all
original 24 plus four new executable cases and their real-file fixture matrices.

The entire public helper API also compiled with missing docs denied:

```powershell
rustc --edition=2021 --crate-type lib --crate-name notype_p23_review -D missing_docs desktop/src-tauri/src/deepseek_probe/mod.rs -o "$env:TEMP/notype-p23-review-api.rlib" --extern rusqlite=D:/git/notype-build/release/deps/librusqlite-90012f4035d3bfdd.rlib -L dependency=D:/git/notype-build/release/deps -L native=D:/git/notype-build/release/build/libsqlite3-sys-b72ad247cb31790d/out
```

That API check passed; it is not counted as a test. All referenced dependencies
were existing read-only rlibs/native libraries; no Cargo, dependency changes,
target writes or downloads. Required desktop frontend lint/build also passed,
with the existing Vite chunk-size advisory; no frontend source was modified.
Changed Rust `rustfmt --edition 2021 --check`, Markdown Prettier and
`git diff --check` passed. No UI suite was broadened for this helper-only change.

No App registration, native console, HTTP/production observer bridge, user binary,
key, paid API, product history/CSV, profile/microphone/clipboard or live reset/cleanup
operation was added or performed. Full #22 merge and every live-v1/offline-prep-v1
gate remain binding. This fix does not enable the paid round or close #23.
