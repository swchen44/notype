# Independent DeepSeek helper contracts

This is the independently approved offline preparation for #23, not an enabled
probe or a completed DeepSeek API acceptance test.

- [P23-live-v1](https://github.com/swchen44/notype/issues/23#issuecomment-6399600403)
  and [Astra approval / zero unresolved](https://github.com/swchen44/notype/issues/23#issuecomment-6399640205).
- [P23-offline-prep-v1](https://github.com/swchen44/notype/issues/23#issuecomment-6399793622)
  and [Astra approval / zero unresolved](https://github.com/swchen44/notype/issues/23#issuecomment-6399801208).

The modules in `desktop/src-tauri/src/deepseek_probe/` are the actual reusable
helper implementation. They are not registered in the App and there is no
user-launchable binary, native console adapter, network client or production
observer bridge in this slice. The root fixtures exercise these modules directly.

## Request policy and fixed schedule

`policy::validate_wire(WireFacts)` accepts only the exact DeepSeek HTTPS target,
`deepseek-flash`, explicit DeepSeek profile / effort none / thinking false,
non-streaming, max output 512, 1–4 plain-text messages and 1–8192 bytes of the
complete actual serialized wire. The full-wire bound is stricter than bounding
only the messages array. The future production adapter must supply facts from
the bytes it actually sends, including production prompts and review envelope;
these tests do not establish that connection yet. No contents or credentials
belong in these facts or in budget storage.

`policy::schedule()` is the fixed eight-call schedule: connection, dictation,
translation, Ask, review forward, reverse, cancel candidate, explicit retry.
Slots 5/6 share history; slots 7/8 share history. Slot 6 has no fixed sample:
production must use only the generated forward text. The source-language
unavailable path may skip only the unsent reverse slot, without replacement.
The schedule carries the exact nonsecret approved samples.

One reservation is 104448 units of USD 10^-7 (US$0.0104448). Eight reserve
US$0.0835584 within the user's US$0.10 authorization. The 32768 input-token
ceiling, peak/cache-miss prices and 512 output bound are the reviewed operational
assumptions in live-v1, not an official framing bound or provider-enforced dollar
quota. Detection cannot undo charges already incurred. This module neither
calculates actual fees nor writes product history fees. Missing monetary receipts
must remain unknown when the future #22 bridge is implemented.

## Admission and durability

`Round<S: RoundStore>` is the admission service. `RoundStore` is a synchronous
durable compare-and-commit system boundary; only the SQLite adapter and failure
fixtures implement it here. Its bounded snapshot holds eight fixed records,
one-shot armed/halted state and revision. Local history/call IDs are opaque
short ASCII correlation keys, not arbitrary text or credentials. The trusted
future bridge must use the actual core IDs.

- A new round must `arm()` durably before `admit()`.
- `admit()` validates the final wire, serial order, distinct call IDs, and exact
  history grouping. It commits a conservative `PossiblySent` reservation before
  returning an opaque `SendPermit`. It cannot start HTTP itself.
- No outcome refunds a reservation. Ordinary failure/contract error halts later
  admissions. Only slot 7's planned cancellation/race can proceed to slot 8.
- A shared receipt observer can `halt()` immediately before a subsequent
  automatic call; `finish()` may still record the origin permit's terminal
  outcome after that halt. Halt never grants another permit.
- Any storage error blocks further mutation in that process, retaining the
  last durable possibly-sent reservation. Recovery never resends for metadata.
- `report()` returns fixed slot states and reservation units, explicitly distinct
  from billed fees. A slot that was reserved but never polled remains possibly
  sent; this is conservative, not evidence of server acceptance.

`SqliteRoundStore::open_owned(absolute_path)` is the actual storage adapter. Its
path parameter exists for the owned fixture and the future fixed-path launcher;
there is no user CLI path/round override. The future binary must resolve exactly
the approved known-folder child, not an App profile or an environment prefix.

Fresh creation uses an exclusive, synchronized `round.lock` ownership marker,
holds its OS file lock, exclusively creates `round.db`, initializes schema 1,
and checks SQLite `synchronous=FULL` with DELETE journal mode. Reservations and
terminal updates use checked revision transactions before permit return. SQLite
may overreserve on a later cross-store failure; the future bridge must ensure
both budget and #22 send-state commits succeed before polling HTTP.

Every valid existing round opens read-only, including an unarmed round or a
process that exited without destructors. It cannot rearm, resume, reset or
replace slots. A second active owner is rejected. Missing/corrupt/foreign marker
or database is rejected without recreating it. Symlinks/reparse ancestors and
foreign directory entries are rejected. There is no reset/delete operation.
These are ordinary crash/restart protections, not protection against an external
actor deleting all files or modifying the binary, nor a power-loss guarantee
beyond the storage/OS durability contract.

## Console lifetime port

`console::read_local_key(ConsolePort)` is the actual platform-independent input
algorithm; tests use nonsecret fake console events only. `KeyMaterial` has no
Debug/Display/Serialize and moves into later in-memory configuration explicitly.

The port must require a real dedicated console and reject redirected input. The
guard snapshots its mode, installs interrupt restoration before disabling echo,
and restores mode before returning any key. Error/unwind cleanup uses RAII;
restoration failure returns an error without releasing key material. Input has
a 1024-byte limit, silent Unicode Backspace, explicit Enter, rejection of empty
and control input, and Escape/Ctrl+C abort. Interrupt is checked on both sides
of each bounded 50ms input wait. Other console modes are preserved.

The future native adapter must implement idempotent, allocation/lock-free control
restoration/signaling, safe handler lifetime and bounded native waits. It must
not print/log events or secrets. No raw Win32 FFI or native adapter is added here.
Actual Windows adapter compile and owned-console restore/interrupt fixture are
**not performed**, pending parent-approved Cargo lease/dependency audit. Fake
port tests do not prove native terminal cleanup. Forced termination/logoff/power
loss cannot guarantee cleanup; use a new dedicated console, never the user's
existing terminal. No complete memory-zeroization claim is made.

## Remaining gates

#22 must be fully merged and its exact commits/verification recorded before
production ChatProcessor/SessionService/CSV integration, a binary for the user,
real local key input or any paid request. At this milestone that gate is pending.
Its provisional observer APIs are deliberately not imported or guessed here.

Later live-v1 acceptance still needs actual final-wire and receipt integration,
shared model/usage/reasoning halt behavior, direct-TLS policy, exact returned-model
allowlist/current-price contract pin, native console/dependency audit, complete
offline tests, reviewed binary/hash and final local user launch. The ordinary
product proxy/provider behavior must remain unchanged. Parent #8 retains all
unfinished work; the independent slice does not close #23.
