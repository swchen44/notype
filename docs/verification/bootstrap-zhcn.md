# Simplified Chinese bootstrap recovery source verification (#29)

## Merged candidate checkpoint — 2026-10-26

Clean candidate `f8c2dd8b56c47e06926e89ff663fe15cdb3b76f8` has final independent
**Spec0** and **Standards0 / zero actionable advisories** against fixed base
`6dd9ca1165461c842d81d6654086d2e883b730dc`. Both formal reports cover the entire
five-file delta and return their review leases. History-preserving merge
`f81f00f524ffe4b9da938a8bf3936f30a3b38d1d` has the identical candidate tree.

The merger independently checked [frontend40292723964](https://github.com/swchen44/notype/actions/runs/40292723964)
metadata and completed logs at product source
`88f9527448d1f6a4b706aeb9134c67e074faba75`: unit4/rendered95, zero failures/skips,
frontend lint/TypeScript/Vite passed. Its portable/core job was explicitly
skipped. This is **not** a full ALL result at f8 or the later merged source.
Final .github/workflows delta from6dd is empty; temporary test ordering is absent.

All source checkpoint/HANDOFF/TODO/research status documentation is assembled
before a new integration freeze S. Root must obtain exact-new-S full ALL plus
Windows contracts with build_private_artifact=false and both final review delta
repins before native source/input/preflight admission. No later documentation
commit may silently substitute a different S. #29 remains open for actual native/
full ZIP/clean-host/real-model/private provisioning acceptance; UI mocks prove
none of those.

The prior API artifact and closed HALTED live round remain pinned to source6dd;
see [actual safe results](https://github.com/swchen44/notype/issues/25#issuecomment-6411081019).
Six DeepSeek calls/five histories were retained; actual fee remains UNKNOWN.
The last successful CSV has four rows while SQLite has five histories, so a
complete research export remains pending. No API retry/new round/new artifact,
key access or provider request is authorized by this locale correction.
[Native external r3](https://github.com/swchen44/notype/issues/14#issuecomment-6411176335) /
[actual Astra0](https://github.com/swchen44/notype/issues/14#issuecomment-6411205513)
preparation/fixtures remain separate; no compiler, native operation baseline or
App/ZIP acceptance has been created. Worktrees, old receipts/artifacts, native
cache and user data remain preserved.

The earlier implementation evidence below remains pinned to its actual source;
older pending-review wording is superseded by this checkpoint.

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
