# Model explorer and editorial theme — #18

2026-10-25. Branch `ticket/18-model-interface`, based on integration commit `ac06224`. This receipt covers the public frontend UI/model presentation seams approved for this implementation. Native runtime, STT quality, and company API acceptance remain separate.

## Delivered behavior

Model cards show the actual catalog name/purpose, explicit state, size, existing relative scores and download actions. Missing localization uses the original `ModelInfo` text; the app missing-key error no longer replaces catalog entries. Advanced disclosure is separate from the download button and lists every unique language code supplied by that model's metadata, current recognition-profile compatibility, streaming/batch behavior, backend-resolved path, filename, engine, format/quantization, and source/revision. The language count is computed from metadata; there is no hardcoded 30-language product list.

Scores are direct existing `accuracy_score` / `speed_score` values on the documented 0–1 catalog scale. Zero, missing/nonfinite, and out-of-range values display as not rated. Accuracy/speed buttons sort highest first within downloaded and available sections, with unscored models last. Catalog order restores the previous default organization. The UI explicitly says these are relative estimates, not accuracy percentages or measurements on the current computer.

The exact Qwen3-ASR 0.6B Q6_K filename also shows a separate summary of the existing [Qwen receipt](qwen3-quality.md), its 3 synthetic bilingual recordings × 2 profiles / 12 decodes, observed 3,229–3,795 ms scope, and limitations. It does not relabel the old observations as current session p95, human speech, or a new benchmark. Streaming capability is taken only from the actual notype `ModelInfo.supports_streaming`; upstream framework capabilities do not override the adapter.

Download UI consumes the existing backend progress events through the model store: percentage, actual downloaded/total bytes in MiB, speed, cancel, integrity verification and extraction states. Onboarding and settings pass the same byte fields. Paths come from the peer-added public `get_model_path` command, which resolves through the new read-only `ModelManager.get_model_path_for_display`; the frontend never guesses a directory. Before download/verification completes, the UI explicitly says the actual path is not yet available and does not query it. An open disclosure refreshes after completion. The original activation resolver keeps its behavior; the display resolver shares the same path checks while skipping stale-partial deletion and all three unavailable-state mutations. Failure is explicit and expandable again to retry. This UI-only fixture test does not verify the native resolver or a real network download.

## Primary design references

Read the official [News](https://www.anthropic.com/news) and [Engineering](https://www.anthropic.com/engineering) pages on 2026-10-25. Fetched the page-linked stylesheet `https://www.anthropic.com/_next/static/chunks/0vuyy3v1anrzc.css` and observed `--color-gray-050:#faf9f5`, `--color-gray-150:#f0eee6`, `--color-gray-950:#141413`, and `--color-clay-primary:#d97757`. Applied warm ivory, dark ink, clay accents, thin rules, generous settings spacing and native Georgia/serif headings. Kept notype identity and dark-mode readability. No fonts, logos, or other assets were downloaded into the repository.

## RED → GREEN evidence

1. `node --test tests/unittest/model-presentation.ui.mjs`: initial assertion failed because actual i18next returned `目前無法顯示這段文字，請稍後重試。` instead of the original catalog name. The public presenter now checks translation existence using the real i18next object. Existing valid translations remain translated. Final: 1 passed, 0 failed, 0 skipped.
2. Browser advanced-information slice initially failed because disclosure was absent. Final UI reveals all metadata languages (synthetic >30-language edge includes Sundanese), revision, quantization and a fixture path, while checking no download callback fired.
3. Score-sort slice initially failed because the speed-sort button was absent. Final rendered order follows 0.90 then 0.60, with an unscored model last; accuracy order reverses the scored pair. Numeric scores and the explicit estimate disclaimer are visible.
4. Download slice initially failed because byte progress was absent. Final rendered UI shows 1.0 MiB / 4.0 MiB and 25%, then verification/extraction, indeterminate-size and path-failure states.

`npm.cmd run test:ui:integration`: final 4 passed, 0 failed, 0 skipped. The fourth check renders the real sidebar + model settings at 440px in Traditional/light and Simplified/dark, confirms current profile labels, no missing-key error, and no horizontal document overflow. Wide 1080px preview is also captured. These are deterministic synthetic IPC fixtures, not invented real-model catalog metadata or a live App screenshot. Screenshot paths are `.local/model-explorer-zh-TW-light-440.png`, `.local/model-explorer-zh-CN-dark-440.png`, `.local/model-explorer-wide.png`; inspected light/dark previews manually. No CSS snapshot mirror tests.

## Checks and limits

- Frontend Prettier: all changed frontend and test files formatted.
- `cd desktop; npm.cmd run lint`: passed.
- `cd desktop; npm.cmd run build`: passed; Vite retains the existing large-chunk advisory. Final build passed after the frontend completion-refresh change; integrated native verification is separate.
- Frontend unit and browser integration suites are explicit root `test:ui:*` commands, so root portable CI does not require desktop packages or a browser and is not claimed as UI coverage.
- Reused the verified `desktop/node_modules` junction targeting `D:/git/notype_study/desktop/node_modules`; no dependency install or native build was performed.
- `rustfmt --edition 2021 desktop/src-tauri/src/managers/model.rs`: passed. No Cargo command or native compilation/test was run here; the parent reserves the single native build cache and validates the integrated command and resolver.
- Native path/IPC integration, real model download, native window, STT, microphone, clipboard and company API: not performed by this worktree. No installed package or user profile was changed.
