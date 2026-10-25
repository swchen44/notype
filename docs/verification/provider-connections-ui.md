# Provider connections UI verification (#27)

Date: 2026-10-26. Product source: `a8aacbef2a46e2ce1dd7b9712bf631e92d6036d0` on
`ticket/27-provider-ui`, based on integration `901cac33`. The integration branch
was merged before freeze and was already up to date. This receipt covers UI
source and synthetic public Tauri-boundary behavior, not native App execution or
paid provider acceptance.

## Approved scope

- [P25-config-v2](https://github.com/swchen44/notype/issues/25#issuecomment-6404377843)
  and [GPT-6-Astra approval](https://github.com/swchen44/notype/issues/25#issuecomment-6404401509).
- [P25-debug-v1](https://github.com/swchen44/notype/issues/25#issuecomment-6404440016)
  and [GPT-6-Astra approval](https://github.com/swchen44/notype/issues/25#issuecomment-6404467374).
- IPC shapes come from [the shared backend contract](../provider-config-contract.md).
  The backend owner supplied bindings unchanged in isolated commits. UI does not
  resolve home `.env` variables or receive stored/resolved secret values.

## Observable behavior

The text settings page supports multiple named Provider/Custom/Local connections
with separate dialect, route, authentication, manually entered model and request
options. Only explicit activation changes the active connection; removing the
active connection leaves none active. New model IDs are blank. Prompt saving,
single-prompt reset and translation settings use revision checks and preserve
the connection collection and unsaved model edits.

Keep/Clear/Replace key actions never refill a saved key. URL changes with a saved
App key require an explicit key action. Zero temperature remains zero and blank
numeric options remain omitted. Thinking starts Off; mandatory models retain a
visible capability limitation during edits, while resolved effective effort is
shown only for the matching saved request configuration. Unknown capabilities
can be explicitly declared as unverified advanced settings.

Model discovery runs only after the user presses its button. The read-only list
offers Copy; users manually paste an ID. Discovery never selects, saves or
activates a model. Late results are fenced after route/revision changes or
cancellation. Authentication, unsupported, empty and invocation-failure cases
leave manual input usable; an explicit retry can recover. Raw rejections are
not displayed. Clipboard tests intercept the public plugin call and do not use
the user's clipboard.

History renders request-option data with its associated requested/reported
model tuple. Preferences, actual wire fields and documented top-p normalization
have distinct labels. Missing observations stay unknown; documented values do
not become response-reported values. English, Traditional Chinese and Simplified
Chinese are exercised in real React/i18next browser views.

Existing diagnostics controls retain their public snapshot contract: preview,
export, OFF/ON, Clear and manual GitHub issue attachment. Its full existing UI
regressions pass; the backend owns report-v2 content and privacy filtering.

## RED to GREEN evidence

Each product slice followed the preceding actual browser failure, using the
existing locked Linux CI harness while local free space was below its limit.

| Slice                                                     | Actual RED run                                                                      | GREEN run / check                                                                                    |
| --------------------------------------------------------- | ----------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| Add connection                                            | [40242306065](https://github.com/swchen44/notype/actions/runs/40242306065)          | [40242824744](https://github.com/swchen44/notype/actions/runs/40242824744)                           |
| Edit/manual model/save                                    | [40243076801](https://github.com/swchen44/notype/actions/runs/40243076801)          | [40243424915](https://github.com/swchen44/notype/actions/runs/40243424915)                           |
| Key action/numeric options                                | [40243497947](https://github.com/swchen44/notype/actions/runs/40243497947)          | [40243895578](https://github.com/swchen44/notype/actions/runs/40243895578)                           |
| Explicit query/Copy                                       | [40243980093](https://github.com/swchen44/notype/actions/runs/40243980093)          | [40244317638](https://github.com/swchen44/notype/actions/runs/40244317638)                           |
| Reload/actionable discovery errors                        | [40244784514](https://github.com/swchen44/notype/actions/runs/40244784514)          | Connection suite 17/17 in [40245355580](https://github.com/swchen44/notype/actions/runs/40245355580) |
| Truthful history request options                          | [40245355580](https://github.com/swchen44/notype/actions/runs/40245355580)          | Local real-browser English, Traditional Chinese and Simplified Chinese tests pass at `0806eeb`       |
| Persistent mandatory-model limitation during option edits | Local targeted browser RED: missing persistent limitation after editing temperature | Same named test and API-option suite 6/6 pass at `a8aacbe`                                           |

Run 40245355580 is an explicit frontend-only run and failed at the intended new
history RED; its connection pass does not imply the whole run or Rust passed.
One earlier assertion incorrectly rejected the legitimate UI label “Bearer API
key”; that test was corrected to reject unique synthetic secret sentinels. It
was not evidence of a credential leak or a product RED.

## Latest executed checks

Local checks used existing dependencies and headless Edge with fresh synthetic
contexts. No package install, browser download, real App profile, microphone,
home key, provider account, paid request or native build was used. Before these
commands, free space was C: 6.89 GB and D: 4.19 GB. Worktree artifacts stayed on
D: except the browser's normal temporary synthetic profile.

| Command                                            | Result                                                                                                                 |
| -------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| `npm.cmd run test:ui:unit`                         | 3 passed, 0 failed, 0 skipped at `0806eeb`                                                                             |
| `npm.cmd run test:ui:integration`                  | 64 passed, 0 failed, 0 skipped at `0806eeb`; includes existing diagnostics, models, history, errors and overlay suites |
| `node --test tests/integration/api-options.ui.mjs` | 6 passed, 0 failed, 0 skipped after the final mandatory-notice fix at `a8aacbe`                                        |
| `npm.cmd run lint` in `desktop/`                   | Passed after final fix                                                                                                 |
| `npm.cmd run build` in `desktop/`                  | TypeScript and Vite passed after final fix; existing large-chunk warning remains                                       |
| Prettier on changed UI source/locales/tests        | Passed                                                                                                                 |
| `git diff --check`                                 | Passed                                                                                                                 |

The baseline-browser-mapping age notice did not affect browser execution. The
production bundle warning reports the existing large theme chunk; no dependency
or code-splitting change was included in this feature.

The composed backend/UI candidate still requires the parent agent's full
default-`all` CI, independent Spec/Standards review and later authorized native
and provider acceptance gates. This receipt does not claim those gates passed.
