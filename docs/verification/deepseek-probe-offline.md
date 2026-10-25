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
