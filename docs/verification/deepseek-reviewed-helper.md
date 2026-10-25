# Reviewed DeepSeek helper build and no-key checks

Date: 2026-10-26 Asia/Taipei. Exact product source:
`41cb62da8482592bde02cc0b2b52460e361e75a9`, kept clean on
`ticket/23-production-helper`. Integration merge:
`af4ce80a315de025c0d8001e71081c3794c8fea2`.

The [reviewed activated-helper plan](https://github.com/swchen44/notype/issues/23#issuecomment-6402345307)
received [GPT-6-Astra APPROVED, zero unresolved findings](https://github.com/swchen44/notype/issues/23#issuecomment-6402369558).
Both final independent product source reviews reported zero findings.
[Full Linux CI 40224287522](https://github.com/swchen44/notype/actions/runs/40224287522)
passed on exact source `41cb62d`; that CI is not native App or paid evidence.

## Actual local results

The complete Windows helper first passed an unactivated type/link compile and
fixed-argument rejection. It then passed the reviewed source-pinned activated
type/link build, argument rejection, no-key console-gate smoke and private dated
package stage. The activated compile ran 2026-10-25 20:07:21–20:07:26 UTC and exited
zero. The safe operation record ended at `stage: packaged`.
The [actual artifact issue receipt](https://github.com/swchen44/notype/issues/23#issuecomment-6402561075)
records the same boundary and hashes.

| Check                                                             | Observed result                                                                                        |
| ----------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------ |
| Exact source activation and full helper link                      | Passed; compile-time pin equals `41cb62d`; activated link proof retained                               |
| Caller-supplied argument rejection                                | Expected raw exit 1, empty stdout, exact stderr `probe-arguments-unavailable`                          |
| Redirected/no-owned-console launch without key                    | Expected raw exit 1, empty stdout, exact stderr `probe-owned-console-required`                         |
| Before/after no-key state                                         | Fixed parent/round/results paths absent both times; non-reparse ancestors; zero owned helper processes |
| Launcher and README package                                       | Created and hash recorded beside exact verified binary                                                 |
| AvailableSpace runtime quota query                                | Not performed by this smoke; linked only                                                               |
| Real key, HTTPS request, billed fee or text quality               | Not performed                                                                                          |
| Tauri App build/startup, microphone, company API, other-App input | Not performed                                                                                          |

The OS-current-caller LocalApplicationData resolver returned
`C:/Users/swche/AppData/Local`. The three absent paths checked before and after the
no-key smoke were `notype-study/deepseek-probe`, its `P23-live-v1` round directory,
and `P23-live-v1-results`. This refusal happens before creating a paid round; no
credential was entered, no round was reset, and no HTTP request was made.

The native console acceptance is separate: the actual direct-rustc driver passed
two tests with nine owned-console cases; see
[native console verification](deepseek-native-console.md). It does not turn the
no-key gate smoke into an interactive key/live test.

## Exact private artifacts

Directory: `D:/git/notype-build/compiler-temp/p23-reviewed-helper-41cb62da`.

| File                        |   Bytes | SHA-256                                                            |
| --------------------------- | ------: | ------------------------------------------------------------------ |
| `notype-deepseek-probe.exe` | 6448128 | `43d3a2c26c65dfcfdde8d51d7487b4782acd6e6829073447cff49e997363925d` |
| `Start-deepseek-probe.cmd`  |     240 | `d41b623a3e12704c9dc2cd28112f0886f100a794d2d70ff68b772da835187d48` |
| `README.txt`                |    2618 | `7daf03fa87d8e1b1a7ba15d5677b66c3e05cf2753740a569fbd8dde90585359d` |

These are a local private helper and instructions, not an npm App release or a
fresh-machine runtime guarantee. Imports include installed Windows system/CRT
libraries and `VCRUNTIME140.dll`; no installer was used.

The external safe operation record is
`C:/Users/swche/AppData/Local/notype-study/agent-notes/p23-reviewed-helper-build-operation.json`.
It records compiler hash `ba03e3ba03a73d9fef19ab3659140f94b43615ec1d587fe9befc9aed3bd5aa72`,
manifest hash `6e5495e258450c8a589a3fe5374014f4398a29f800e597b815a303d698036742`,
activation pin, artifact hashes, imports and exact nonsecret smoke results.

## Resource evidence and next gate

The reviewed cumulative guard observed high-water allocation/volume consumption
67,284,992 bytes, minima C 2,236,219,392 / D 1,299,587,072 bytes and maximum active
volume interval 215ms. No drive floor, 192MiB cap or 160MiB stop was crossed. The
baseline/ledger was retained; no model, user profile, existing App or cache
artifact was deleted. The source-pinned worktree remains unchanged.

The user has been asked whether their local key is ready; the reply and paid
round remain pending. Only the user should open the verified launcher, enter the
key in its dedicated non-echo console and explicitly type `RUN P23-live-v1`.
The agent does not launch that paid round, scrape existing credentials or ask for
the key in chat. The existing one-round authorization is US$0.10, at most eight
fixed request slots including cancellation/retry/back-translation. The reviewed
US$0.0835584 reservation is an operational assumption, not actual fees or a
provider-enforced billing cap. API money remains unknown unless actually returned.

The packaged README pins the official contract/pricing snapshot and tells the
user to request a fresh check if using it later. A completed helper build does
not close #23: actual responses, usage, semantic quality and six-history CSV
reconciliation still need evidence. #22/#14 native App acceptance and the
remaining #8 scope also remain open.
