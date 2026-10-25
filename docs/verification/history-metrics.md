# History observations, API receipts and CSV verification

Date: 2026-10-26. Ticket [#22](https://github.com/swchen44/notype/issues/22),
parent [#8](https://github.com/swchen44/notype/issues/8).
Approved [P22-v1](https://github.com/swchen44/notype/issues/22#issuecomment-6399288240),
[GPT-6-Astra APPROVED / zero unresolved plan findings](https://github.com/swchen44/notype/issues/22#issuecomment-6399333180).

Source checkpoints: `081fc368` (observations/ledger), `a6d0f3c` (CSV and frozen
native DTO/options), `655e2f2` (complete backend source). Merge `7fc56dd` brings in
latest integration `de28d33`, including the P19 handoff fix and separate #23
offline preparation, approved candidate-version metadata and current HANDOFF/TODO.
CI/test-document merge conflicts retain both the history DTO target and #23 steps.
This is a backend verification receipt, not completed native/App/live acceptance.

## Behavior exercised

The real SessionService store uses fresh schema 4 and refuses older nonempty
schemas without changing their data. Original text, received native-frame duration,
requested/producer model and both mixed counts persist. API attempts append across
retry/back-translation, with safe durable pre-send state and bounded receipts
captured before output validation. No credential-bearing settings/request dump is
stored. Missing usage/fees are unknown/partial; returned zero remains valid.
Money sums use exact decimals by currency. Overflow keeps truthful reported-call
counts while totals remain unknown.

Actual ChatProcessor loopback requests exercise default omission, DeepSeek none/
thinking enabled controls, explicit custom Generic effort and output limits.
Invalid finish reason/envelope preserves reported usage. Owned SQLite triggers
prove pre-send failure prevents network and post-send storage failure never
reissues a request. A final storage failure retains the original and prepared
possibly-sent call; reopening exposes unknown usage instead of invented totals.
Cancellation/retry and independent reverse requests preserve existing ownership.

The public workflow test captures stop before synthetic scheduling delay and
separates received 500ms audio from decoder input. Requested/actual ASR identities
remain separate. API-disabled original availability is original_only. Actual
diagnostics OFF, Clear and ON operations leave saved observations unchanged.

The main integration seam is actual workflow → ChatProcessor loopback → owned
SQLite → actual UTF-8 BOM CSV. Installed Python 3 standard-library csv parses the
file independently and recovers exact Unicode/multiline text, ID, model/effort,
tokens and unknown actual fees. The export excludes the fixture bearer key,
endpoint, Authorization and messages. No API fee is derived from arbitrary cost
fields, a token rate or a trial reservation. ASR tokens are explicitly not_reported
with a blank count because current engine contracts return text only.

Additional exports cover formula-leading text without rewriting, quotes/CRLF,
concurrent delete/insert during the independent snapshot, low space/cancellation,
failed replacement preserving an existing destination and removing only the
operation's partial, plus 1,200 owned Unicode rows. Export streams rows/call
observations with a 64KiB writer; memory includes one history row and its distinct
summary values, not all history or a raw call vector.

## Results and limits

Windows PowerShell, existing Rust/MSVC/Python installations. Exclusive portable
Cargo lease used only `D:/git/notype-build`, Release, offline/locked, jobs=1 and
incremental=0. No RUSTFLAGS/ORT environment overrides, new dependencies/features,
new target, native App launch, user DB/profile, mic, clipboard, key entry or paid
requests. Temporary fixtures contain synthetic/nonsecret data only.

| Check | Result |
| --- | --- |
| Core unit/public session tests | 12 passed (mixed-count/receipt/duration 3 + session 9) |
| Core real HTTP/SQLite/review/export integration | 28 passed (HTTP 3 + observations/store 11 + persistence 9 + review 5) |
| Core doc example | 1 passed |
| Native provider imported into portable real HTTP harness | 20 passed |
| Portable public desktop workflow | 15 passed |
| Pure native metrics DTO projection | 1 passed |
| Core Release Clippy all-targets, warnings denied | Passed |
| Core formatting, changed Rust rustfmt and git diff whitespace | Passed |
| Native App/library compilation, native Clippy and recorder fixture execution | Not performed; parent-owned native build gate |
| Rendered frontend/settings/export controls | Separate #24 worker/receipt; no frontend changed in this branch |
| Real mic/ASR quality, company/DeepSeek paid API | Not performed |

Reproduction commands (resource environment/guards must follow the active lease):

```powershell
cargo test --release --offline --locked --manifest-path crates/notype-core/Cargo.toml
cargo test --release --offline --locked --manifest-path tests/integration/desktop-contracts/Cargo.toml --test desktop-provider --test desktop-workflow --test history-metrics-dto
cargo clippy --release --offline --locked --manifest-path crates/notype-core/Cargo.toml --all-targets -- -D warnings
```

All behavioral slices had focused regression checks; observed RED→GREEN examples
include missing observation/DTO contracts, absent real HTTP call accounting,
effort fields missing from the actual wire and incorrect overflow reported counts.
Full core verification passed 40 executed tests plus its doc example. The final
same-factory public re-export cleanup changes no transport/parser behavior; native
build and independent review still apply to the handed-off source.

Runtime guards retained C>=2GiB+32MiB and D>=1GiB+32MiB. Initial lease headroom
was recorded with both drives>=2GiB. With dependencies already cached, parent
authorized a 128MiB source-only/check-metadata working budget; observed peak extra
writes were 63,025,152 bytes for Clippy, and <=12,668,928 bytes for the recorded
test batches. No floor/cap was crossed and no cache cleanup occurred. Final observed
free space was approximately C2.294GB and D1.565GB (decimal units).

Two root native tests in `tests/unittest/recording_metrics.rs` are included by the
actual recorder module: public stop through the real ring consumer/resampler/VAD
and decoder-only padding, with an owned hardware-channel fixture and no device.
They require the separately budgeted native library build and are not counted
above. Actual engine/stream identity transport is likewise only formatted/source
checked here. Parent must complete native checks, UI integration and independent
review before declaring #22 complete or opening the #23 production/live gate.
