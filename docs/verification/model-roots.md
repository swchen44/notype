# Selectable model roots source verification (#28)

Date: 2026-10-26. Final reviewed/CI source:
`b3a75543baf78b22cdf0dac409ef384ad02d2285`; fixed source-review base:
`73c57aac08cafb135fcac3b32757f93f2739a6f6`.
History-preserving integration merge:
`540d1ee44ac78dd95a381c7a7ed854fe42db79bc`.
The merge tree equals the reviewed source tree. The later documentation commit
is not claimed to have run the source CI.

Approved [P28-model-roots-v1](https://github.com/swchen44/notype/issues/28#issuecomment-6405356803) /
[actual GPT-6-Astra APPROVED, zero unresolved](https://github.com/swchen44/notype/issues/28#issuecomment-6405400881).
The canonical issue and exact plan separate source/owned-fixture completion from
native Windows picker, real model reuse/ASR and delivery acceptance.
[Final issue gate](https://github.com/swchen44/notype/issues/28#issuecomment-6407779453)
records the independently verified source reviews, CI and merge.

## Delivered source behavior

One RootService owns durable root preference/revision/generation, candidate
validation, immutable catalog projection, admission and artifact ownership.
AppSettings remains the single requested selected-model authority; Apply does
not change that preference. Invalid root preferences produce recoverable
Unavailable state. Selecting, applying, rescanning and loading never move/copy,
repair/download or delete existing model bytes.

Root-scoped file identity, catalog size/SHA-256 and bounded scanning precede
availability. Flat catalog artifacts use all 367 compiled hash/size anchors.
Compatible custom GGUF remains explicitly unverified; directory models without
a member-hash contract remain visible/unavailable. Explicit roots do not fall
back to ambient HF cache. Default legacy HF sources are read-only. Narrow
contained HF snapshot links are validated; other links remain unsupported.
Paths stay in local folder/settings presentation, outside logs/history/reports.

Admission covers accepted capture/queue/load/decode/scan/download workers until
actual completion. Cancel ACK does not release it prematurely; owned transfer
runtime shutdown drains blocking work. Engine identity includes root/artifact
identity and a load-instance ticket; runtime capability metadata is keyed by
ArtifactKey. Owned partials use exclusive creation, verified handles,
no-overwrite publication and exact-identity cleanup. Imported/shared artifacts
do not acquire destructive ownership from their model ID or matching bytes.

Actual App/onboarding/Settings/store source provides native-picker transport,
validation/Apply/Rescan/Show-folder, Busy/retry and recovery without a network
request. Four mutations pass expectedGeneration; backend admission is the
authority. A stale terminal refresh cannot erase a successor candidate or
selection. Existing users retain History/Settings if their former model vanishes.

The schema-2 base facade requires App/runtime rather than a model dependency.
Missing optional weights can reach recovery without activation/copying. Runtime,
helper, token, CPU and complete Fixed WebView checks remain hard launch gates.
Explicit full inventory inspection remains strict for a declared optional model.
#29 offline ZIP/bootstrap/provider initialization is outside this merge; only the
approved initialize-default-if-unset root hook is present.

## Independent final review

Both independent reports were received at exact b3a before merge. Spec reviewed
the original full source then all intervening deltas; Standards reviewed the
full base-to-8eff958 source and exact 8eff958-to-b3a final delta. Neither reviewer
consulted the other axis's report or ran builds/tests/App/live acceptance.

- **Spec: zero unresolved source findings.** The four original findings are
  resolved: coordinator receives the root service after manager registration;
  dangling partial links are rejected and transfers retain owned/pinned handles;
  capability publication uses full ArtifactKey; mirror success omits raw paths.
  Final b3a docs/checked-parent delta preserves those source boundaries.
- **Standards: zero unresolved findings.** Ownership/locking/publication
  preconditions now document the new public interfaces; HF link resolution
  reuses its fallibly checked parent instead of production unwrap. Original
  findings remain preserved in the review record.
- A separate bounded supplemental Standards inspection found no additional
  issues. It is not claimed as another completed full-source review.

Original and final reports remain preserved in agent-notes. No scope extension,
dependency upgrade or public API change was introduced by the final two-finding
Standards correction.

## Exact final FULL ALL CI

[Shared core contracts 40265243964](https://github.com/swchen44/notype/actions/runs/40265243964)
was independently checked **completed/success**, event workflow_dispatch,
exact head b3a, with both full jobs and every required step successful.
Counts below were independently derived from the completed job logs:

| Check | Actual final result |
| --- | --- |
| Platform portable-native | 4 passed |
| Core public contracts | 49 passed; doc test 1 passed |
| Desktop contracts, 27 targets | 202 passed, 0 failed, 1 inherited controlled child role ignored in discovery |
| Root filesystem/service integration | 35 passed, including 2 Unix link cases |
| Root policy / transfer worker / P20 model storage | 5 / 2 / 7 passed |
| Node unit / integration | 24 / 19 passed; 0 failed, 0 skipped |
| UI unit / actual rendered integration | 3 / 92 passed; 0 failed, 0 skipped |
| Standalone existing offline policy / round / console | 3 / 9 / 7 passed |
| Core/platform formatting and Clippy; root/HTTP harness Clippy | Passed |
| Frontend lint, TypeScript and Vite production build | Passed |

Root, worker and storage counts are included in desktop202, not added to it.
Standalone offline checks overlap existing harness coverage. The inherited
owned_proxy_generation_child direct role is invoked explicitly by its passing
proxy parent; it is not omitted acceptance or Windows QA. There are no new skips.

Commands and separate unit/integration entry points are recorded in
[tests/README](../../tests/README.md#selectable-model-roots-28) and
[the workflow](../../.github/workflows/core-contracts.yml). Relevant Rust commands:

```powershell
cargo test --locked --manifest-path tests/integration/desktop-contracts/Cargo.toml --test model-roots-policy --test model-transfer-worker
cargo test --locked --manifest-path tests/integration/desktop-contracts/Cargo.toml --test model-roots --test model-storage
node --test tests/integration/model-roots-launcher.test.mjs
node --test tests/integration/model-roots.ui.mjs
```

## Local evidence, RED to GREEN and limits

The source owner separately executed actual production modules with standalone
rustc 1.99.1 (58cf8f9c3, 2026-10-13), without Cargo/native target writes:
Windows root integration33 (includes 8 existing GGUF/parser/capability tests),
policy5, worker2, storage7; affected launcher/package Node25. The Linux root35
above includes two additional Unix-only link cases. Local UI owner also
reported full rendered92/unit3 and lint/build/Prettier; exact final CI independently
repeats those suites. Local counts are not added to CI totals.

Recorded actual RED-to-GREEN observations cover the missing onboarding chooser,
queued generation selection, schema1 model-free launch, queued scan ordering,
Busy candidate loss on terminal refresh, obsolete P20 ID reconciliation,
changed-folder Apply, same-byte external replacement inheriting Delete ownership,
and terminal callbacks releasing admission before worker completion. These were
corrected through actual production seams and tiny owned fixtures, without
xfail/skip acceptance. The merger did not rerun local tests.
CI40263265412's new harness Clippy failure was a manual_repeat_n lint in an
existing GGUF inline test; it is not behavioral RED. The corrected c8f0338 and
8eff958 full runs passed before exact final b3a success.

Compatible Release rlibs were read-only standalone compiler inputs:

| File under D:/git/notype-build/release/deps | Recorded SHA-256 |
| --- | --- |
| libserde-00822b0709db48d4.rlib | F5CC4B71A8534414DFE636D7D50F7C0E0A642566E0B29610F6D73480DA300CBB |
| libserde_json-09abb9c207ed1115.rlib | F45195B27783DB6716EEB39E072D6394DDD194764F1FAC914AF359230A693F12 |
| libsha2-213a01a83e744533.rlib | C8F7B68F51066711CE2DD032F6408E90F7E12AAEAD950B901E93F5A1EF1C1575 |
| libspecta-4fdd6aac1d90a261.rlib | 5BEA27D74A24A9353C544D96C5224F078A0DEF987F0EB1BC4B6D9733269EFB41 |
| libtokio-027c17c4de0cf09f.rlib | 7849F896C11385677006F516FE671C74E1370C29EC481B338BEF9BB49900B6AF |

Owned output/fixture directory:
C:/Users/swche/AppData/Local/notype-study/build-cache/p28-contracts-20261026-model-roots.
Source owner recorded total18,485,568 B under its32MiB cap; preceding measured
C free6,755,360,768 B / D free4,049,059,840 B. These are historical observations,
not a fresh native budget. Fixture teardown was confined to verified task-owned
roots. No user-model/native-cache/PDB cleanup occurred.

Native-only inherited direct-download fixture tests were adapted and parsed by
rustfmt, but were **not executed as a native test build**. No local Cargo/native
build or native Clippy, Windows App/Explorer/picker/microphone, real model
relocation/reuse/ASR, user profile/home/key, real provider/paid API or shipping ZIP
acceptance occurred. UI transport mocks do not establish those native behaviors.
All #28 source leases are returned; worktree/review directories remain preserved.

## Remaining frontier

#28 closes only its completed source/owned-fixture scope; #8/#14 retain native
picker, real-model reuse/ASR, extracted ZIP and delivery acceptance. Next source
frontier is [#29](https://github.com/swchen44/notype/issues/29), under its
[plan](https://github.com/swchen44/notype/issues/29#issuecomment-6405357391) /
[actual Astra approval0](https://github.com/swchen44/notype/issues/29#issuecomment-6405428255).
Its implementation is not included in this merge.

[P14-build-v5](https://github.com/swchen44/notype/issues/14#issuecomment-6405766076) /
[approval0](https://github.com/swchen44/notype/issues/14#issuecomment-6405783543)
and [delivery-v5](https://github.com/swchen44/notype/issues/14#issuecomment-6405766320) /
[approval0](https://github.com/swchen44/notype/issues/14#issuecomment-6405789439)
remain future build/delivery gates after required source and fresh coexistence
budget/resource checks. No native Cargo lease was claimed here.

#25's [P25-live-v4](https://github.com/swchen44/notype/issues/25#issuecomment-6405986527) /
[actual Astra approval0](https://github.com/swchen44/notype/issues/25#issuecomment-6406003416)
helper source remains unfinished on ticket/25-provider-probe-v4. No real keys,
live calls or spending; the old source41 probe remains frozen/unlaunched.
#29/#14/#25/#8 and company/five-App acceptances remain open.
