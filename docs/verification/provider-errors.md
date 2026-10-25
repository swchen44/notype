# Provider failure classification — phase-one verification

Date: 2026-10-26 Asia/Taipei. Issue [#25](https://github.com/swchen44/notype/issues/25).
Exact reviewed source: `d4315522171a393f01012a934d78b110cd72fb25`.
Integration source merge: `ec37ab6741422c9e7ba5aeeb5f93553cd508f401`.

Approved [P25-errors-v1](https://github.com/swchen44/notype/issues/25#issuecomment-6403957943)
and [GPT-6-Astra APPROVED / zero unresolved findings](https://github.com/swchen44/notype/issues/25#issuecomment-6403993599).
The independent Spec review found one context-classification issue; its
[fix-v2 plan](https://github.com/swchen44/notype/issues/25#issuecomment-6404301764)
received [Astra approval](https://github.com/swchen44/notype/issues/25#issuecomment-6404307928).
Final independent Spec and Standards rechecks both reported **zero findings**.

## Behavior exercised

The existing production ChatProcessor classifies authoritative HTTP statuses and
narrow structural in-band error codes into safe fixed categories. Numeric
validation codes permit exact context-limit refinement without overriding auth,
balance, rate-limit or server precedence. Natural-language messages are not used
for classification and upstream body/URL/key content is not owned by errors.

Actual provider → owned loopback HTTP → SessionService → SQLite/reopen tests cover
HTML401, permission/balance/rate-limit/unavailable/validation/context errors,
HTTP200 error objects, redirects, socket closure, truncated/oversized responses,
whole deadline and cancellation. Each attempt sends at most once; explicit retry
adds a call to the same history. Actual receipts are observed before error result
validation. The saved original remains available, and reverse failure retains
the committed primary translation with review incomplete.

EN/zh-TW/zh-CN HistorySettings rendered tests exercise forward/reverse auth
messages, actionable categories and a safe fallback for unknown persisted text.
Raw malicious/credential/endpoint sentinels do not become rendered history text.
These tests use owned fixtures and synthetic credentials, not current user files.

## Actual RED and GREEN evidence

- Initial auth-only regression source `f77d464c` failed in
  [CI40238961659](https://github.com/swchen44/notype/actions/runs/40238961659), then
  implementation `ffa42fe8` passed
  [CI40239267075](https://github.com/swchen44/notype/actions/runs/40239267075).
- The first full source `46bb5358` failed only the actual provider Clippy target
  for a needless borrow in
  [CI40240188340](https://github.com/swchen44/notype/actions/runs/40240188340).
  The nonsemantic borrow correction is included in final source; that failed run
  is retained rather than relabelled successful.
- Context-refinement tests at `ccab10e` had a real unit plus HTTP/SQLite RED:
  [CI40240688564](https://github.com/swchen44/notype/actions/runs/40240688564)
  recorded two failures / 28 passes in the provider harness. Minimal fix
  `19b1d23` plus the integration merge produced final source `d431552`.
- [Full final CI40240970555](https://github.com/swchen44/notype/actions/runs/40240970555)
  completed **success** at exact `d431552`; job `121312450921` logs were read
  independently for the counts below. The source was then merged without
  conflicts; receipt-only changes do not claim their own separate CI execution.

| Final Linux CI check                                        | Actual result                                                                                      |
| ----------------------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| Core tests                                                  | 49 passed, including five public gateway-error unit tests                                          |
| Core doc example                                            | 1 passed                                                                                           |
| Platform contracts                                          | 4 passed                                                                                           |
| Desktop contract targets                                    | 118 passed across 19 targets; provider30, workflow15 and DTO1 included                             |
| Standalone offline probe policy/round/console               | 3 / 9 / 7 passed                                                                                   |
| Node unit/integration                                       | 24 / 18 passed                                                                                     |
| Core/platform formatting and Clippy; actual provider Clippy | Passed, warnings denied                                                                            |
| Rendered provider-error UI                                  | Six tests passed in the worker's earlier Windows run; UI source unchanged by final Rust-only fixes |
| New local Cargo/Tauri build or native App acceptance        | Not performed; local resource gate unmet                                                           |
| Live provider/key/cost/quality/company acceptance           | Not performed                                                                                      |

No local Cargo execution was used for this phase; RED/GREEN Rust execution was
on Linux CI. Existing headless UI validation does not prove Tauri/native runtime.
No current App/profile was changed, no real key value was printed or copied and
no paid request was made.

## Follow-on scope remains open

This phase does not implement multiple connections, Custom/Local request options,
provider-specific thinking, App-key/home-.env precedence, credential destination
binding, View/Copy model IDs, manual model choice or safe debug controls. Those
are separately planned under #25 and require exact Astra approval before code.
The user has resolved the essential product choices; experimental IDs do not
become UI defaults. DeepSeek Flash4.1's current-round authorization is US$2 total,
while Gemini/OpenRouter are free-only; unresolved real free-billing eligibility
blocks live sends, not owned-fixture work.

The old source41 helper retains its own historical build/fixture proof and is
not activated for the changed model/key/budget design. #25, #23, #22, #14 and #8
remain open for their remaining boundaries. Native App0.1.1, storage handling,
company/five-App input, human language quality/p95 and public distribution are
not inferred from this source slice.
