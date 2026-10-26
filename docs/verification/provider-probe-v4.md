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
