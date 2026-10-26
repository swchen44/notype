# Provider configuration, model discovery and diagnostics source gate

Date: 2026-10-26. Final reviewed/CI source:
`ab9e785e6b1faac2d70a759ed70b6582245b49ab`; review base:
`901cac33c7d53464d54d7bc008b681fad600ae79`.
History-preserving merge into `implement/notype-mvp`:
`c5605f44ecb50ad36ee16e57f7e26973b8887974`.
The merge tree equals the reviewed source tree. This receipt's later documentation
commit did not run the source CI below.

## Scope and approved plans

This completes the source/owned-fixture acceptances of [#26](https://github.com/swchen44/notype/issues/26)
and [#27](https://github.com/swchen44/notype/issues/27), whose issue bodies explicitly
retain native/build/live acceptance in parent [#25](https://github.com/swchen44/notype/issues/25).
Multiple connections with one active record, route-bound App credential actions
and selected home fallback, capability/request options, explicit reference-only
View/Copy model discovery, safe history/CSV observations and bounded diagnostic
report v2 are implemented. See [IPC contract](../provider-config-contract.md).

- [P25-config-v2](https://github.com/swchen44/notype/issues/25#issuecomment-6404377843) /
  [actual Astra APPROVED/0](https://github.com/swchen44/notype/issues/25#issuecomment-6404401509).
- [P25-debug-v1](https://github.com/swchen44/notype/issues/25#issuecomment-6404440016) /
  [actual Astra APPROVED/0](https://github.com/swchen44/notype/issues/25#issuecomment-6404467374).
- [P25-config-reviewfix-v2](https://github.com/swchen44/notype/issues/25#issuecomment-6405575893) /
  [actual Astra APPROVED/0](https://github.com/swchen44/notype/issues/25#issuecomment-6405614167).

## Independent final source review

The coordinator received both independent final reports for exact ab9/base901
before merge; their results and final CI are recorded in
[the parent gate](https://github.com/swchen44/notype/issues/25#issuecomment-6405850082).
Both reviewers remained read-only, performed no tests/builds/App/provider calls,
and did not consult the other axis's report.

- **Spec: zero unresolved source findings.** The reviewer combined the original
  full audit with committed delta reviews at 2977, 8fed and ab9. All three
  original findings are resolved: failed preparation retains globals from one
  owned snapshot; an Error finalizer releases only its matching query; canonical
  model endpoint plus private resolved authorization identity controls cooldown
  and Retry-After across unrelated edits/records while collection-revision
  admission/finalization checks remain.
- **Standards: zero documented-rule breaches; zero unresolved smell findings.**
  The sole prior duplication advisory is resolved by private
  `resolve_key_action`; public save/discovery error ordering, persistence/CAS
  boundaries and transient query credentials remain distinct. Required root
  test directories, i18next resources and safe credential/report boundaries
  were checked.

These are source verdicts. The reviewers explicitly did not claim final CI or
native acceptance; the merger separately checked the completed run.

## Exact final ALL CI and counts

[Shared core contracts 40251455558](https://github.com/swchen44/notype/actions/runs/40251455558)
was independently verified **completed/success** at exact ab9, event
`workflow_dispatch`, default `scope=all`. Both jobs and every required step
completed successfully. Completed job logs, rather than earlier run totals,
supply these counts:

| Check | Actual final result |
| --- | --- |
| Platform portable-native | 4 passed |
| Core public contracts | 49 passed; doc test 1 passed |
| Desktop contracts (24 targets) | 158 passed, 0 failed, 1 controlled child role ignored in discovery |
| New provider config unit/store/model GET | 12 / 4 / 9 passed (included in desktop total) |
| Diagnostic API unit/actual HTTP | 4 / 10 passed (included in desktop total) |
| Desktop provider/workflow | 30 / 15 passed (included in desktop total) |
| Standalone offline policy/round/console | 3 / 9 / 7 passed; intentionally overlaps existing harness coverage |
| Node unit/integration | 24 / 18 passed, 0 failed, 0 skipped |
| UI unit/rendered integration | 3 / 66 passed, 0 failed, 0 skipped |
| Core/platform formatting and Clippy | Passed |
| Actual HTTP/config/model harness Clippy | Passed with `-D warnings` |
| Frontend lint, TypeScript and Vite production build | Passed |

The ignored `owned_proxy_generation_child` is a controlled child role explicitly
invoked with `--exact ... --ignored` by the passing
`custom_https_and_local_proxy_policy_use_only_owned_routes_and_cleared_child_env`
parent. It is not omitted acceptance or Windows verification. The fake proxy
and stalled-body deadline regressions use actual reqwest/SessionService/owned
SQLite with synthetic credentials. Existing Rust test-harness dead-code,
browser metadata age and large-chunk notices remain nonblocking; no dependency
update or native build was added.

Full commands are in [.github/workflows/core-contracts.yml](../../.github/workflows/core-contracts.yml).
This run executes separate root `test:unit`, `test:integration`,
`test:ui:unit` and `test:ui:integration` scripts, existing locked Rust contracts,
Clippy and desktop lint/build. The workflow's frontend job now includes rendered
UI; older documentation describing portable-only CI is historical.

## Actual RED to GREEN history

[Reviewfix receipt](provider-config-reviewfix.md) preserves actual behavior RED
at b494084 (rendered Error retry), 4add1bf (snapshot 10 pass/2 fail and actual
capture/SQLite 9 pass/1 fail), and 4130054 (model GET 7 pass/2 fail). The Rust
compile error in the first run is explicitly not behavioral RED.
8fed165's full ALL run passed the snapshot/credential-policy corrections;
the exact final ab9 run above passes the cooldown correction and all regressions.
[UI receipt](provider-connections-ui.md) retains the earlier feature-by-feature
browser RED/GREEN evidence and local 64-test baseline. Final CI now has 66
rendered tests, including actual Error/successor-finalizer coverage.

## Unperformed acceptance and next frontier

No native Cargo/Tauri build, native bindings execution, Windows App/home adapter,
microphone/readiness, user settings/profile, actual home `.env`, real provider,
paid API, company TLS/CA/proxy or five-App acceptance was performed by this gate.
The user's real clipboard was not used. Old App, local-trial, Starter, models and
history remain protected. #25/#8/#14 and outstanding native/live/company/five-App
acceptances remain open; source completion does not complete the parent spec.
All source-stage leases were returned; worktrees/review references are preserved.

Next source frontier is **#28**, followed by **#29**; neither product slice is
included here. [#28 plan](https://github.com/swchen44/notype/issues/28#issuecomment-6405356803) /
[Astra approval/0](https://github.com/swchen44/notype/issues/28#issuecomment-6405400881)
covers user-selected/reused model roots; the user moves files manually.
[#29 plan](https://github.com/swchen44/notype/issues/29#issuecomment-6405357391) /
[Astra approval/0](https://github.com/swchen44/notype/issues/29#issuecomment-6405428255)
covers the complete offline ZIP and optional no-key managed defaults.

[P14-build-v5](https://github.com/swchen44/notype/issues/14#issuecomment-6405766076) /
[actual Astra approval/0](https://github.com/swchen44/notype/issues/14#issuecomment-6405783543)
and [delivery-v5](https://github.com/swchen44/notype/issues/14#issuecomment-6405766320) /
[actual Astra approval/0](https://github.com/swchen44/notype/issues/14#issuecomment-6405789439)
are now approved plans, not completed build/delivery. They depend on #28/#29 and
fresh budget/resource gates before any native work. Latest preceding read-only
space observation was approximately C 6.85 GB / D 4.14 GB; old shortage text is
superseded, and no cleanup occurred.
