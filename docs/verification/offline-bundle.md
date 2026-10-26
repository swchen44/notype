# Offline bundle and first-use bootstrap source verification (#29)

Date: 2026-10-26. Final reviewed/CI source:
`d7137beb82cc583b88ae0d93a58de18c7eed55b5`; fixed review base:
`943e53482294d7a06235b37184557d0706181a1c`.
History-preserving integration merge:
`1143f61a09e7bd59da74b0ecc0bfff3060a47def`.
The merge tree equals the reviewed tree. A later documentation commit is not
claimed to have run this exact source's CI.

Authority: [P29-offline-bundle-v1](https://github.com/swchen44/notype/issues/29#issuecomment-6405357391) /
[actual GPT-6-Astra APPROVED, zero unresolved](https://github.com/swchen44/notype/issues/29#issuecomment-6405428255),
and [P29-reviewfix-v1](https://github.com/swchen44/notype/issues/29#issuecomment-6408412474) /
[actual Astra APPROVED0](https://github.com/swchen44/notype/issues/29#issuecomment-6408691967).
[Final source gate](https://github.com/swchen44/notype/issues/29#issuecomment-6409109529)
records the independently verified reviews, CI and merge. **#29 remains open**:
source contracts are complete; actual full private Windows ZIP/native/clean-host
and company acceptance are still unperformed.

## Delivered source boundaries

The [complete ZIP contract](../../packages/OFFLINE-BUNDLE.md) defines bundled
Node22.15, full App/runtime, separate models, typed optional nonsecret bootstrap,
exact inventory/provenance/notices and Start-notype.cmd. The facade derives the
root from its module, uses the adjacent verified runtime and fixed arguments,
and hands native only the canonical bundle root. No PATH/npm/download/model-copy
or arbitrary inherited bootstrap authority is required by that source route.

Production verification audits actual tiny ZIP bytes before extraction, including
entry/Windows-case/path/link safety, bounded expansion, hashes/CRC and exact
inventory. Full --check stays strict for every declared model/runtime/bootstrap
file and notice. Ordinary launch verifies required program/Node/runtime/bootstrap
and host gates; unavailable optional models permit recovery and never certify a
complete bundle or override an existing verified user root. Synthetic 257-file
runtime fixtures do not establish real vendor-payload completeness.

The [native bootstrap contract](../offline-bootstrap.md) uses the existing
ConnectionStore, RootService and AppSettings selected-model authority. Whole-input
validation precedes independently durable per-domain initialization; existing
settings win without rewriting. Domain markers/retry fill only uninitialized
domains. A partial commit exposes Setup Incomplete and blocks capture/API work,
rather than claiming cross-file atomicity. Import generates one no-key connection
and invokes no credential reader, model listing or generation. Private company
addresses/models remain administrator input, never invented generic presets.

Read-only placement preflight runs at the first native run boundary before
portable directories/logging/plugins/managers. It freezes the checked executable,
bundle and explicit override or authoritative Known Folder resolution and
revalidates before domain commits. Recovery preserves History Original/Result
copy through the existing safe command while capture/API/processing retry stay
blocked. Runtime startup serializes readiness, audio preparation, transcription
watcher construction and publication; audio failure starts no watcher, and a
successful pair shares one router.

## Independent final source review

Both reports were received at exact d713 before merge. Each performed the
original full fixed-base review and reviewed every successor delta; neither
consulted the other axis's report. Original findings remain preserved.

- **Spec: zero unresolved source findings.** Early whole-bundle/data placement
  resolves the original startup-write ordering defect; recovery admits safe
  History copy. The approved runtime retry correction preserves the single
  router and serialized publication. The final e298-to-d713 delta changes only
  an equivalent test function reference.
- **Standards: zero unresolved findings.** The original orphan transcription
  watcher on repeated audio failure is resolved through the production
  RuntimeStartup helper and actual failure/retry/concurrency regression seam.
  No further documented-rule breach or actionable advisory smell was found.

Reviewers ran no native builds/App/user-data/provider operations. Their source
verdicts do not substitute for the merger's separate completed CI check or
future real archive/native acceptance.

## Independently checked exact final ALL CI

[Shared core contracts 40274754512](https://github.com/swchen44/notype/actions/runs/40274754512)
was verified **completed/success**, event workflow_dispatch, exact head d713.
Both jobs and every required step succeeded. Completed logs establish:

| Check | Actual result |
| --- | --- |
| New bootstrap targets | 19 passed: provider unit3, real domain-store integration10, placement3, runtime3 |
| Existing desktop contracts, 27 targets | 202 passed; 1 inherited controlled child role ignored in discovery |
| Platform / core | 4 / 49 passed; core doc1 passed |
| Node unit | 33 passed, 0 failed, 0 skipped |
| Node integration | 24 passed, 0 failed, **1 Windows-CMD-only skip on Linux** |
| UI unit / rendered integration | 3 / 94 passed, 0 failed, 0 skipped |
| Existing standalone offline policy / round / console | 3 / 9 / 7 passed |
| Formatting, all required Clippy, frontend lint, TypeScript/Vite | Passed |

Bootstrap19 is separate from existing desktop202. Existing standalone offline
checks overlap harness coverage. The inherited proxy child is explicitly invoked
by its passing parent; it is distinct from the skipped Windows CMD test.
The skipped case is “Windows Start selects its adjacent runtime double with
empty PATH and quoted Unicode paths.” Linux success does not prove that Windows
route, real Node/native startup or a recipient machine without tools. No Windows
execution total is inferred from CI. Existing harness dead-code/browser metadata/
bundle-size notices remain nonblocking.

Separate commands are in [tests/README](../../tests/README.md#offline-defaults-and-complete-zip-contracts-29)
and [the workflow](../../.github/workflows/core-contracts.yml):

```powershell
cargo test --locked --manifest-path tests/integration/desktop-contracts/Cargo.toml --test provider-bootstrap
cargo test --locked --manifest-path tests/integration/desktop-contracts/Cargo.toml --test offline-bootstrap-store
cargo test --locked --manifest-path tests/integration/desktop-contracts/Cargo.toml --test bootstrap-preflight --test runtime-startup
node --test tests/unittest/offline_bundle.test.mjs
node --test tests/integration/offline-bundle.test.mjs
node --test tests/integration/bootstrap-recovery.ui.mjs
```

## Preserved behavioral RED to GREEN

At test source `292215b839c208f8616c2638ffb7a6b238f88d9a`,
[CI40272933792](https://github.com/swchen44/notype/actions/runs/40272933792)
actually reached and failed the intended boundaries: placement0 passed/1 failed;
domain store8 passed/1 failed on safe-copy admission; runtime1 passed/1 failed
because three watchers started instead of zero; rendered suite93 passed/1 failed
on History Original/Result Copy. Other later steps in that failed run were not
claimed to pass.

Fix `5ed0209f97e36c0876b2eba228e4a9ddfc2301f1`,
[ALL CI40273394611](https://github.com/swchen44/notype/actions/runs/40273394611),
completed successfully in both jobs. The e298 verification additions retained
real domain/SQLite history during runtime failure and expanded placement/router/
concurrent-retry coverage. e298's later Clippy failure in
[CI40273944359](https://github.com/swchen44/notype/actions/runs/40273944359)
was a redundant test closure, not behavioral RED. d713 replaces that closure
without changing assertions; final exact CI above is the acceptance run.

## Unperformed acceptance and next gates

No local Cargo/native App build, actual large stage/ZIP/extracted payload,
clean-host/no-tools start, disconnected/packet first-run network observation,
real model relocation/ASR/microphone, private intranet preset/service or user
profile/home/key/paid API operation was performed by this merge. Tiny real
archives, owned sentinel children, factories and IPC-only rendered tests remain
source evidence. All #29/Node source leases returned; worktrees, references,
old App/models/profiles/packages/artifacts remain preserved. No cleanup occurred.

[P14-build-v5](https://github.com/swchen44/notype/issues/14#issuecomment-6405766076) /
[Astra0](https://github.com/swchen44/notype/issues/14#issuecomment-6405783543)
and [delivery-v5](https://github.com/swchen44/notype/issues/14#issuecomment-6405766320) /
[Astra0](https://github.com/swchen44/notype/issues/14#issuecomment-6405789439)
remain future source/artifact/resource/native gates. The
[explicit optional npm deferral](https://github.com/swchen44/notype/issues/14#issuecomment-6409002692)
preserves complete ZIP/extracted/native QA and defers only new optional npm
tarball/install/prefix/cache validation. It lowers no runtime floor, resets no
ledger, removes no files and is not admission for a build or large write.
Fresh coexistence/input counts and per-operation checks remain required.

#25 helper candidate6c7f7c4 is not merged: two source review findings remain under
investigation/fix at this checkpoint. Its
[workflow-registration-v1 plan](https://github.com/swchen44/notype/issues/25#issuecomment-6409029819) /
[actual Astra0](https://github.com/swchen44/notype/issues/25#issuecomment-6409066492)
is approved but execution still requires final helper/composed source0 and exact
ALL/Linux/Windows gates. No workflow registration, artifact activation or live
request is implied here; old source41 stays frozen/unlaunched.
#29/#14/#25/#8 and company/five-App acceptance remain open.
