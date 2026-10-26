# Provider configuration review fixes

Scope: the three Spec findings and credential-policy duplication advisory from
the independent reviews of `2977db0`. Exact
[reviewfix-v2 plan](https://github.com/swchen44/notype/issues/25#issuecomment-6405575893)
and [GPT-6-Astra approval/0](https://github.com/swchen44/notype/issues/25#issuecomment-6405614167).
No provider/model/credential-source additions, migration or new dependencies.

## Actual RED and GREEN evidence

- `b494084`, [CI 40250282479](https://github.com/swchen44/notype/actions/runs/40250282479):
  the rendered actual `Error` rejection could not perform its second model-list
  query; the existing string rejection passed. Rust did not reach behavior in
  this run because the new test used `original` instead of `transcript`; that
  compile failure is not claimed as RED.
- `4add1bf`, [CI 40250614882](https://github.com/swchen44/notype/actions/runs/40250614882):
  actual snapshot unit RED, 10 passed/2 failed, both `en-US` versus `ja-JP`;
  actual capture/SQLite integration RED, 9 passed/1 failed/1 ignored controlled
  child role, persisted target `en-US` versus `ja-JP`. The ignored role is
  explicitly executed by the separate accepted proxy test, not skipped acceptance.
  The saved/unsaved public credential-action parity test passed before extraction.
- `8fed165`, [ALL CI 40251021969](https://github.com/swchen44/notype/actions/runs/40251021969):
  completed success. Snapshot globals survive missing credentials or unresolved
  capability, including a settings mutation inside credential resolution. The
  actual workflow persists the requested Japanese target before repair and
  retries the same SQLite history ID using that target despite a later French
  global preference. Shared private key-action policy preserves the two existing
  callers' validation order and unsaved credentials never replace stored keys.
- Local rendered React/IPC checks: 5 passed, covering string and actual `Error`
  rejections, cancellation plus late result, an old finalizer while its successor
  remains pending, and an editor route change. Each explicit retry keeps the
  manual model and performs no save or activation. The raw Error sentinel is absent.
- Cooldown test-only `4130054`, [CI 40251084613](https://github.com/swchen44/notype/actions/runs/40251084613),
  produced actual GET runtime RED: 7 passed/2 failed, both second queries returned
  `ok` instead of `throttled`. Its two
  actual GET regressions distinguish unrelated collection edits from a second
  record sharing the same route/credential. They also retain positive controls
  for a genuinely different credential/endpoint and monotonic 60-second and
  provider Retry-After expiry. The fix removes record ID and collection revision
  from the existing cooldown key and Retry-After update, while preserving both
  collection-revision stale checks. The held-reply regression also covers an
  unrelated prompt mutation; the reply stays stale and its cooldown stays spent.
  Final exact-source ALL CI is dispatched after this receipt; its outcome is not
  claimed here in advance.

## Reproducible checks

Rust runs only in the existing Linux `Shared core contracts` workflow. Its
desktop command uses `--no-fail-fast` to report independent test targets in the
same run; assertions and failures remain enforced. Final acceptance uses default
`scope=all`, including core/platform tests, desktop unit/HTTP/SQLite tests,
Clippy, the complete rendered UI/unit suites, frontend lint and TypeScript/Vite.

```powershell
node D:/git/notype_study/.local/gh.cjs workflow run core-contracts.yml --repo swchen44/notype --ref ticket/26-provider-config
node --test --test-name-pattern='rejected model-list|cancellation|finalizer|late model-list' tests/integration/provider-connections.ui.mjs
node desktop/node_modules/prettier/bin/prettier.cjs --check desktop/src/components/settings/post-processing/ModelIdList.tsx tests/integration/provider-connections.ui.mjs tests/integration/fixtures/provider-connections.mjs
git diff --check
```

Changed Rust sources/tests are formatted with the installed `rustfmt --edition
2021`; no local Rust compiler or Cargo target was run. All HTTP endpoints,
credentials and SQLite/settings files in these regressions are owned synthetic
fixtures. Windows/native/Tauri execution, microphone, clipboard, user settings,
actual home `.env`, real providers and paid calls remain not performed.
