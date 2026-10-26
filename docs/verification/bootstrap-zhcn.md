# Simplified Chinese bootstrap recovery source verification (#29)

Date: 2026-10-26. Fixed base:
`6dd9ca1165461c842d81d6654086d2e883b730dc`.
Authority: [P29-zhcn-recovery-v1](https://github.com/swchen44/notype/issues/29#issuecomment-6411382409)
/ [actual GPT-6-Astra APPROVED, zero unresolved](https://github.com/swchen44/notype/issues/29#issuecomment-6411399601).

The product correction adds only `notype.bootstrap.title`, `description`,
`retry` and `retrying` to the zh-CN resource. Previously these fell back to
Traditional Chinese. Existing English/zh-TW messages and all other locale keys,
including valid Chinese plural categories, are unchanged. Native startup,
bootstrap domains, API behavior, settings and prompts are unchanged.

## Actual runtime RED to GREEN

- Resource RED: tests-only source
  `421b91cd6aa4815f7e35c12341ed0c0dbb2362ca`,
  [hosted frontend run 40291890913](https://github.com/swchen44/notype/actions/runs/40291890913).
  The actual i18next `getResource` lookup returned `undefined` for the zh-CN
  title instead of the expected Simplified Chinese text: 3 units passed,
  1 failed. The direct-resource assertion bypasses fallback; the test also
  checks ordinary translated lookup for all four labels after the fix.
- Rendered RED: tests-only source
  `d45ead0cbd676a77da82a97a9a8e9b53761aea63`,
  [hosted frontend run 40292083129](https://github.com/swchen44/notype/actions/runs/40292083129).
  The actual recovery component could not find the Simplified Chinese heading
  within its status panel. English recovery passed; rendered totals were
  94 passed, 1 failed, 0 skipped. This checkpoint temporarily ran rendered
  tests before units so the first unit failure would not hide rendered RED.
- GREEN product/test source:
  `88f9527448d1f6a4b706aeb9134c67e074faba75`,
  [hosted frontend run 40292723964](https://github.com/swchen44/notype/actions/runs/40292723964),
  completed successfully. All 4 UI units and 95 rendered integrations passed,
  with 0 failures and 0 skips. Frontend ESLint, TypeScript and Vite build passed.
  The original workflow command order is restored; its final diff from the
  fixed base is empty. Existing browser-metadata/chunk-size notices are
  nonblocking and no dependency was changed.

The reused public seams are actual i18next resources and the rendered
`BootstrapRecovery` component/hook with only native IPC mocked. The rendered
zh-CN case checks heading, description, retry and disabled busy labels, one
explicit retry, and acknowledgment before the panel disappears. The existing
English case and full History/safe-view integration coverage remain passing.
Prettier checks passed on the changed frontend/tests; `git diff --check` passed.
Commands are documented in [tests/README](../../tests/README.md).

## Source freeze and evidence limits

This documentation is committed after the GREEN source; that run is not claimed
to cover a later documentation commit or merge. Independent Spec/Standards
review, accepted merged source S, final full ALL CI and exact-S Windows contracts
with `build_private_artifact=false` remain merger gates. Future native source
and input checks must use that newly accepted S rather than the superseded 6dd
source freeze. No native build or acceptance follows from this UI-only check.

Existing API artifacts and closed live-round evidence retain their original 6dd
provenance. This correction performs and authorizes no key read, API retry or
provider request and requires no new private helper artifact. Local formatting
used existing dependencies read-only; tests/build ran on hosted Linux. No local
dependency install, Cargo/native/App execution, user profile/home/key/model read,
clipboard/device operation, large archive operation or cleanup was performed.
Existing worktrees, data, artifacts and native caches remain preserved. Source
writer ownership returns to the merger at branch freeze.
