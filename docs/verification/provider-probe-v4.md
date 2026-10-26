# P25-live-v4 source verification

[Exact plan](https://github.com/swchen44/notype/issues/25#issuecomment-6405986527),
[GPT-6-Astra APPROVED / zero unresolved](https://github.com/swchen44/notype/issues/25#issuecomment-6406003416).
Worktree `provider-probe-v4`, branch `ticket/25-provider-probe-v4`, initially
`73c57aac`; integration `943e534` merged. No local Cargo/rustc/cache/native App,
actual home/key read, helper public GET, authenticated request or inference has
run. US$2 remains wholly unspent. P23 source41/artifact/ledger/round are untouched.

## Actual behavioral red/green

| Slice | Runtime RED | Subsequent GREEN |
| --- | --- | --- |
| Final-wire routing controls absent | [40253410122](https://github.com/swchen44/notype/actions/runs/40253410122) | `28bf3e6`, [ALL40253826647](https://github.com/swchen44/notype/actions/runs/40253826647) |
| Missing numeric zero response cost accepted | [40253925629](https://github.com/swchen44/notype/actions/runs/40253925629) | `1d62b76`, [ALL40254326855](https://github.com/swchen44/notype/actions/runs/40254326855) |
|429 header lost behind stalled-body cancellation | [40254445859](https://github.com/swchen44/notype/actions/runs/40254445859) | `e6b9b59`, [ALL40254850660](https://github.com/swchen44/notype/actions/runs/40254850660), including OFF before request |
| Wrong target script accepted | [40266739768](https://github.com/swchen44/notype/actions/runs/40266739768) | `f3a7765`, [ALL40267350224](https://github.com/swchen44/notype/actions/runs/40267350224) |
| Positive1e-999 cost rounded to accepted zero | [Windows40269136565](https://github.com/swchen44/notype/actions/runs/40269136565),14 other integration passes | `d1e39cf`, [ALL40270473220](https://github.com/swchen44/notype/actions/runs/40270473220), [Windows40270472394](https://github.com/swchen44/notype/actions/runs/40270472394) |
| Documented26B comparison model accepted as31B result | [Windows40272529608](https://github.com/swchen44/notype/actions/runs/40272529608),15 other integration passes | `65cae31`, [Windows40273357104](https://github.com/swchen44/notype/actions/runs/40273357104):7 unit/18 integration plus Clippy |
| Completion-winning7 skipped fixed Chinese quality | [Windows40273991825](https://github.com/swchen44/notype/actions/runs/40273991825):18 other integration passes; timer already joined | Minimal outcome-based exception only for explicitly expected cancellation; final source CI pending |

Compile/import/Clippy failures are not behavioral RED. The original duplicate
test-module imports were removed using dedicated targets, without lint suppression.
Local source checks use installed rustfmt and `git diff --check`.

At `1d02971`, [Windows40268422780](https://github.com/swchen44/notype/actions/runs/40268422780)
passed7 unit/SQLite and12 integration tests plus Clippy; its artifact job was
skipped. The complete helper check exercised an owned literal synthetic home
through production HomeFileReader/ConnectionStore, opaque Custom transfer,
12 owned HTTP requests, actual observer/SessionService/SQLite, and production
atomic CSV. Independent readback reconciled9 histories, retry/reverse groups,
calls/usage/options and UNKNOWN actual fees. The test took about35seconds,
including actual10second OpenRouter spacing. Slot7 cancellation/retry retained
one usage receipt across two calls rather than inventing zero. Four Gemini
entries remained sealed. Fake keys were absent from CSV/P20 export.

A separate completion race keeps the old timer pending, lets slot7 finish, then
proves the timer is joined before retry8 HTTP. Actual429 headers survive body
stall/cancel and diagnostics OFF/Clear. New SQL-trigger fixtures test both stores'
send-admission failures and a helper finish-callback failure before automatic
reverse, with diagnostics OFF before processing. CSV write failure retains data
and denies the next request. Bounded table fixtures bind current Flash peak
prices to the correct column despite old-price decoys; the full public HTML
snapshot fetched independently by root is not committed.

The Windows contracts execute the real unpinned ordinary binary with invalid
arguments, hidden/redirected, asserting fixed stderr, empty stdout and no owned
state writes. They exercise real AvailableSpace/SHA256/file-allocation APIs on
owned fixtures. This does not substitute for the future downloaded pinned
same-binary no-key `check` or establish live readiness.

Final helper Spec0/Standards0, exact-source ALL/Windows CI, private artifact and
download/check/resource/fresh-contract gates remain root operations. The private
Windows workflow runs contracts on push; artifact building is manual after
source acceptance. [Registration promotion](../provider-probe-workflow-registration.md)
requires its own reviewed root operation before any `main` mutation.

No downloaded Windows artifact check, actual API language quality/account tier,
human acceptance, microphone, App delivery, enterprise TLS/proxy or p95 is
claimed. Reservations/zero-price metadata never populate production actual fees.

## Approved review fixes

Original independent review at `6c7f7c4`: Spec findings P1 (forward quality checked
after automatic reverse) and P2 (unattached/missing protected negation); Standards0.
The original findings are retained in the outside-repository review reports.
[Reviewfix-v1](https://github.com/swchen44/notype/issues/25#issuecomment-6409094265)
received actual [GPT-6-Astra APPROVED / zero unresolved](https://github.com/swchen44/notype/issues/25#issuecomment-6409112341).

P2 test-only `8cbaca2` produced actual
[Windows40276076285](https://github.com/swchen44/notype/actions/runs/40276076285)
runtime RED:7 existing policy/SQLite tests passed; the new protected-fact test
failed because slot10 accepted affirmative deletion. The narrow matcher uses
only finite supported deletion/slide-prohibition forms and the distinct not-five
contrast. An unrelated negative token is insufficient; stored output is unchanged
and no generic semantic correctness is claimed.

P2 `6ada6e2` passed [Windows40276603301](https://github.com/swchen44/notype/actions/runs/40276603301),
including all supported positive forms and the unchanged full synthetic round.
After integration composition, P1 test-only `ce218bd` produced actual
[Windows40277277505](https://github.com/swchen44/notype/actions/runs/40277277505)
runtime RED:8 policy/SQLite tests and19 existing integration tests passed; the
two new tests observed6 rather than5 and11 rather than10 actual owned POSTs.
Compilation/lint failures are not included as RED evidence.

The fix checks the exact `BackTranslationRequest.text` at reverse entry and
irreversibly halts the same Coordinator before pacing/observer construction.
The forward stays a successful, already-flushed transport call. Both negative
integration cases run with diagnostics OFF before processing and Clear before
the bad forward; they assert no reverse call/link/reservation/POST or later call,
and reopen the actual owned SQLite to verify original/forward text, usage and
UNKNOWN fees. Conservative reservations remain522240 units at slot5 and835584
at slot10; Gemini stays sealed. The existing positive pairs, complete12-call/
9-history smoke, callback faults,429/cancel and completion-winning timer tests
remain required in final exact-source CI.

Latest integration `0a0fc82` was merged, preserving both bootstrap and helper
harness targets, CI steps/filters and test documentation. The private Windows
workflow bytes remain identical to the original reviewed helper candidate.
No core, wire/parser, CLI, credential, schedule, fee or activation change is part
of these review fixes. Exact final CI/source rechecks remain separate gates.

## Fixed deferred-npm coexistence amendment

[P25-coexistence-v1](https://github.com/swchen44/notype/issues/25#issuecomment-6409393018)
has actual [GPT-6-Astra APPROVED / zero unresolved](https://github.com/swchen44/notype/issues/25#issuecomment-6409496675).
It binds admission to root's explicit [P14 npm deferral](https://github.com/swchen44/notype/issues/14#issuecomment-6409002692).
The entire remaining P14 native/ZIP/extraction/QA reservation is retained:
C2049MiB/D2368MiB, including the existing shared32MiB guard once. Runtime
floors remain C2GiB/D1GiB. Own remaining reserves are C128MiB staging,
D32MiB combined smoke/live runtime and D1MiB launcher. Zero-consumption
requirements are C4,430,233,600/D3,591,372,800 bytes. There is no runtime scope
override, new readiness field, native completion credit or cleanup credit.

The pure ResourceBudget ledger is called by the actual QuotaGate checkpoint
with unchanged Windows allocation and availability adapters. Root unit tests
inject owned byte observations through that same public admission boundary;
they do not lower real disk space or substitute availability during execution.
They check exact and one-byte-short boundaries, worked retained-helper totals,
unavailable/shrinking probes, monotonic peaks after failed/cancelled writes and
invalid/extreme marks. C112MiB stop, runtime24MiB stop and launcher1MiB cap
remain unchanged; invalid marks fail closed before subtracting reserves.

Test source `74b432d` retained the original formula behind the extracted
production admission seam. [Windows40278940625](https://github.com/swchen44/notype/actions/runs/40278940625)
was actual runtime RED:8 policy tests and21 existing integration tests passed;
`deferred_npm_resource_admission_accepts_exact_reviewed_space` failed with
`Err(Resource)` instead of `Ok(())` at the exact reviewed initial boundary.
The fix changes only the fixed coexistence arithmetic, using checked operations.
The portable resource tests live in `tests/unittest/provider_probe_resources.rs`
and are loaded by the existing headless production-helper contract harness.

The earlier quality source `8195bb9` passed
[Windows40277905369](https://github.com/swchen44/notype/actions/runs/40277905369).
Its [ALL40277906369](https://github.com/swchen44/notype/actions/runs/40277906369)
passed the Rust/core job but failed the existing About log-level UI
acknowledgement assertion at `about-settings.ui.mjs:176`. This is retained as
a failed full run, not represented as complete quality acceptance. Final exact
amendment Linux/Windows results and independent source rechecks are separate
gates. No local Cargo/native build, artifact operation, key read, public metadata
fetch, provider request or inference is part of this amendment; spend remains0.
