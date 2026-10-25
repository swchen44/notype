# History store owner release across child spawn

Approved exact plan: [P22-owner-release-fix-v2](https://github.com/swchen44/notype/issues/22#issuecomment-6400680932),
[GPT-6-Astra APPROVED / zero unresolved](https://github.com/swchen44/notype/issues/22#issuecomment-6400703948).
The previous initial-ready correction remains separate. This work first merged
integration `84f1253b5ed2295e1f7246c366471a33b544fa05`; no failing regression was
merged into integration to obtain the diagnosis.

## Deterministic diagnosis before product changes

Original Linux [run 40214859978](https://github.com/swchen44/notype/actions/runs/40214859978)
failed `malformed_response_is_recoverable_after_real_sqlite_restart_and_retry_uses_saved_raw`
with AlreadyOwned after awaited work and `drop(workflow)`. Windows provider tests
had passed. The later unchanged-root [run 40215819126](https://github.com/swchen44/notype/actions/runs/40215819126)
succeeded, demonstrating that the original CI failure alone was not deterministic
proof of the inferred mechanism.

Only the root public regression was committed first, as
`5fd2a9f790f01972678ec76546c8173762973609` on `ticket/20-diagnostics`. The existing
Shared core contracts workflow was explicitly dispatched on that branch, with
no workflow/toolchain/configuration modification. Actual Linux
[run 40216079671](https://github.com/swchen44/notype/actions/runs/40216079671)
executed
`last_service_owner_releases_store_while_inherited_pre_exec_child_is_held` and failed
its final public reopen assertion at `core_persistence.rs:113`:

```text
final service owner must release before child exec; got Some(AlreadyOwned)
test result: FAILED. 9 passed; 1 failed; 0 ignored
```

Handshake, normal child exit and live-clone exclusion assertions preceding that
final assertion passed. This is the required named runtime RED, not a compile
failure, timeout or unrelated failure. Product ownership was untouched until that
exact result was independently fetched and confirmed.

The Unix-only fixture opens a fresh owned SQLite store, retains a service clone,
and starts only its current executable's selected helper branch on an owned
thread. A precreated UnixStream pair receives read/write deadlines on both ends
before fork. `pre_exec` signals one byte and waits for one release byte using only
bounded read/write operations, stack bytes and allocation-free OS errors. It does
not use SQLite, locks, allocation, logging, formatting, async or environment reads
inside that post-fork closure; the unsafe boundary has an explicit safety comment.
The parent saves the public reopen result while the child is still held, then
releases/shuts down the stream and joins/reaps the owned child before assertions.
RAII repeats that cleanup on an early unwind. No sleeps, reopen retries, lockfile
deletion, fixture serialization or PID guessing hide the failure.

## Small ownership correction

The product fix is
`fb4ab689322cf0e7843c69a80489ed92b983c8b9`. A private `DatabaseOwner(File)` is created
only after successful exclusive lock acquisition, before SQLite initialization,
and shared through Arc. Its final Drop explicitly unlocks before the file closes.
Live service clones therefore still hold the same exclusive owner, while an
inherited descriptor cannot by itself retain the former parent's lock until exec.
Failed acquisition never creates a guard or unlocks another owner's lock.

Drop cannot return an error or panic. Unlock is best effort, followed by normal
close; if the OS unlock fails, inherited descriptors can still delay release.
This limitation is documented instead of adding a retry or data cleanup policy.
No public API, schema, observations, CSV, transport or admission rule changes.

The mechanism follows the primary [Rust File lock/unlock contract](https://doc.rust-lang.org/std/fs/struct.File.html#method.unlock),
which distinguishes explicit unlock from waiting for every duplicated/inherited
descriptor to close. The fixture follows the [Rust pre_exec safety contract](https://doc.rust-lang.org/std/os/unix/process/trait.CommandExt.html#tymethod.pre_exec).

Pinned source SHA-256 (also checked by the independent provisional Spec/Standards
reviews; later documentation edits must preserve both):

- `crates/notype-core/src/lib.rs`: `1863BEDD224697F0A7921A09FD451BB4C614B4998A745F45D51184CC327C10CE`
- `tests/integration/core_persistence.rs`: `3796F0414505C4990BF0BF6398FF7F76600BD7538273BF8616FD5A4000C6A9CB`

## Verification and resource boundary

Windows used the sole existing portable Cargo lease: `D:/git/notype-build`,
Release, offline/locked, jobs=1, incremental=0. All guarded child processes finish
before another command starts. No new dependencies/features/downloads/target or
native App build. Commands:

```powershell
cargo test --release --offline --locked --manifest-path crates/notype-core/Cargo.toml
cargo test --release --offline --locked --manifest-path tests/integration/desktop-contracts/Cargo.toml --test desktop-provider --test desktop-workflow --test history-metrics-dto
cargo clippy --release --offline --locked --manifest-path crates/notype-core/Cargo.toml --all-targets -- -D warnings
```

Local results: **core 43 tests + 1 doc example**, **desktop-provider 20**,
**desktop-workflow 15**, **history-metrics-dto 1** passed; zero failures/ignored.
The Unix fixture is cfg-excluded on Windows and is not counted as executed or
passed locally. Existing live-clone, second-instance and restart negatives remain
in the suites. Release core all-target Clippy with warnings denied passed.
Changed Rust rustfmt, Markdown Prettier and `git diff --check` passed.

The task-local guard checked drives every 250ms and would stop only its owned
Cargo PID tree at C<=2GiB+32MiB, D<=1GiB+32MiB or 128MiB cumulative observed extra
disk consumption. Lease baseline: C2,261,110,784 / D1,564,192,768 bytes. Observed
minima: C**2,252,849,152**, D**1,513,779,200**; peak extra **54,734,848 bytes**.
No floor/cap was crossed and no cache cleanup was performed.

The final product/test source was dispatched through the unchanged normal
parallel full Linux workflow, [run 40216512564](https://github.com/swchen44/notype/actions/runs/40216512564),
head `fb4ab689322cf0e7843c69a80489ed92b983c8b9`. Its exact conclusion and executed
named regression must be checked before integration merge; this local receipt
does not substitute Windows results for that Linux gate. Subsequent documentation
commits preserve the source hashes above; final run/source matching is recorded
in the handoff.

No native App/profile/microphone/clipboard, key or paid request was touched.
The whole #22/#24 source dependency gate, #23 bridge/live gates and remaining
native/deployment acceptance remain separately required.
