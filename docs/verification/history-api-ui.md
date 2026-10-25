# History metrics and API request controls UI (#24)

This frontend slice follows [P22-UI-v1](https://github.com/swchen44/notype/issues/24#issuecomment-6400242556)
and [GPT-6-Astra APPROVED / zero unresolved](https://github.com/swchen44/notype/issues/24#issuecomment-6400277019),
under [P22-v1](https://github.com/swchen44/notype/issues/22#issuecomment-6399288240)
and its [approval](https://github.com/swchen44/notype/issues/22#issuecomment-6399333180).

Branch: `ticket/24-history-ui`. The frozen native contract is checked backend
checkpoint `a6d0f3c`; this branch merges integration `83639c9`. That checkpoint is
partial backend evidence, not acceptance of the complete #22 feature. The final
native binding regeneration and native build belong to the parent integration.

## Changes

- `HistorySettings` places independent backend counts beside the original and
  processed text. `HistoryMetrics` presents received-audio seconds and capture
  quality, count-rule version, requested and actual producer identity, initial
  stop/result and latest ASR/API/retry observations. Milliseconds convert only to
  audio display seconds; the frontend does not derive counts or elapsed stages.
- Expandable all-call usage distinguishes complete totals, partial known
  subtotals, unknown and not applicable. Input/output completeness is independent.
  Actual reported fees retain the exact amount string and explicit currency;
  requested/reported model and effort remain associated. History never consults
  current provider/model settings to replace missing observations.
- All-history CSV export invokes `export_history_csv` with no user arguments
  (Tauri transport supplies `{}`). It remains available when search is empty,
  disables repeated pending export, displays the native saved-row count and
  separately reports cancellation or a fixed translated failure.
- API settings use native `provider_profile`, nullable `effort` and nullable
  `max_output_tokens` fields. Generic supports standard and explicit custom
  bounded values; DeepSeek supports default/none/low/high/max. Provider changes
  preserve endpoint/model/key and the existing effort/limit; invalid values show
  translated errors and prevent persistence. Save failure retains the draft and
  supports retry. Pending API controls are disabled. Existing scenario prompts
  and translation controls remain composed in the actual settings component.
- EN / Traditional Chinese / Simplified Chinese translations cover the added
  controls and feedback. Bindings are manually synchronized to the actual
  `commands/history_metrics.rs`, `SessionDto` and `TextConfiguration` contract.

## Public verification

The tests mount actual production React components, i18next, CSS and Vite in a
fresh headless installed Edge instance. Only the public Tauri IPC transport is
replaced with synthetic, credential-free DTO/configuration fixtures. The shared
fixture accepts optional virtual entry/cache settings; these tests use separate
ignored caches for history and settings, and reserve ephemeral ports. Existing
fixture callers keep their original defaults.

TDD observations before implementing each behavior:

- Main history test failed waiting for missing `Original count: 91`.
- Export test failed waiting for missing all-history export button.
- API profile test failed waiting for missing Provider profile select.
- A later custom effort test exposed an input disappearing while typing through
  the standard `low` prefix; retaining custom-edit mode fixed that real failure.

New tests: 13 rendered integration cases (7 history, 6 API settings), including
backend count values intentionally different from JS text length, real zero,
missing observations, partial tokens and exact currency strings, model/effort
associations, all-history export pending/saved/cancelled/error/payload behavior,
provider-switch preservation, Generic custom bounds and null defaults, DeepSeek
limits, rejected saves and retry, persistence across component remount, and all
three locales at a 360px viewport without horizontal overflow.

Final checks on 2026-10-26 (Asia/Taipei), after the custom-edit fix and formatting:

| Command                                                  | Result                                                 |
| -------------------------------------------------------- | ------------------------------------------------------ |
| `npm.cmd run test:ui:unit`                               | 1 passed, 0 failed/skipped                             |
| `npm.cmd run test:ui:integration`                        | 36 passed, 0 failed/skipped; includes all 13 new cases |
| `npm.cmd run lint` in `desktop/`                         | Passed                                                 |
| `npm.cmd run build` in `desktop/`                        | TypeScript and Vite production build passed            |
| Changed TSX / locales / root UI tests / receipt Prettier | Passed                                                 |
| `git diff --check`                                       | Passed                                                 |

The serial composed integration run included the existing About/log-level,
diagnostics, model and recording-overlay suites, and took 142.655 seconds.
Bindings remain excluded by the existing `.prettierignore`; their final native
regeneration is explicitly pending parent integration. Build output retains the
existing large-chunk advisory, and the browser tooling reports old baseline
metadata; neither produced a failure or required dependency installation.

## Limits

This verifies frontend behavior at public transport boundaries. It does not
establish native save dialogs, actual CSV bytes/atomic replacement, SQLite or
provider behavior, capture hardware, Windows App operation or end-to-end native
binding compatibility; those are the parent/backend checks. No App, microphone,
clipboard, actual profile/database, user credential, model download or paid/public
API was used. No Rust, Cargo, database/schema or dependency file was changed.
No issue was modified or closed by this UI worker.
