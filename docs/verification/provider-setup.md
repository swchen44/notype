# Provider setup and real connection checks (#32)

Specification: [issue #32](https://github.com/swchen44/notype/issues/32). Product source `90159a19c6e930abb06959e297f0656b08577a8e`. The user explicitly waived further Astra review. This fixes the setup interface and records actual provider results; it does not turn an upstream limit into a successful generation.

## Reproduction and change

Actual v0.1.1-local.2 Windows UI reproduced both problems: creating a connection left the editor below the saved list without explaining the next step; a Bearer key input appeared only after selecting “Replace” in the key-action menu. Two rendered regressions failed on those exact symptoms before the fix.

The creation form now explains address/key/model setup and later Edit. Creation and Edit reveal the selected editor and focus its name; revisiting the same editor preserves its draft. The Bearer password input appears before model lookup and accepts a key directly. Typing selects replacement only for explicit Save; returning to blank preserves the existing setting. Clearing remains an explicit separate action. Switching no-auth to Bearer enables direct entry. Existing masking, route binding, revision/cancellation protections and App-key priority over eligible home `.env` fallback remain in place. Basic fields use two columns where space permits, with Base URL/full-endpoint examples and explanation. English, Traditional Chinese and Simplified Chinese resources were updated.

Open WebUI tutorial connection/edit screenshots were downloaded and inspected, alongside the user's provider reference. Their URL/key/model organization informed the layout; no chat, RAG, Arena or extra provider features were introduced.

## Executed checks

| Check | Actual result |
| --- | --- |
| RED → GREEN | Both new setup regressions failed before implementation, then passed |
| Local affected rendered checks | 50 passed, 0 skipped, including direct key/save, blank/clear, no-auth→Bearer, settings/model-list errors, races, masking and narrow translated controls |
| Local UI unit | 12 passed, 0 skipped |
| Frontend | ESLint, TypeScript, Vite and changed-file Prettier passed |
| [ALL CI 40620476588](https://github.com/swchen44/notype/actions/runs/40620476588) | Both jobs succeeded at exact product source; UI unit12/rendered116 with 0 skipped; Rust/Node contracts and Clippy passed. Node integration retained 24 passed plus one Windows-CMD-only Linux skip |
| Windows native build | Exact clean source, 321,791 ms, 59,357,184-byte App; SHA-256 `812e2319d7ec11301da649185718e258d5f25789e65230855ec771767f77ffc4`; 26 existing unused-code/import warnings, no errors |
| Complete native local payload | 1,695 declared files verified; application manifest `ea850b199e2a8b2f750189f2bc3b8da334aa20583094c8330b4d8b184aa201d9`; unchanged runtime/model provenance |
| Actual new Windows UI | On a 682×600 window, Add displayed next-step guidance; Edit scrolled to the correct existing connection and focused its name. The API-key input was directly visible before the model controls without selecting Replace. Existing connections/counters/model persisted across normal Quit/restart |

No Rust/backend, dependency lock or runtime resource was changed. Windows clipboard behavior was not re-certified by this frontend change; its earlier source-specific evidence remains separate. Native validation entered no real or synthetic replacement key into the user's saved connections; actual input/save/clear behavior was covered by rendered tests with synthetic credentials and mocked IPC.

## Actual provider observations, 2026-10-28

The user authorized these checks using their local home `.env`. Complete credentials, account labels, raw responses and private transcripts were not printed or attached. Requests used official fixed endpoints with normal TLS; no proxy/TLS bypass or automatic retry was added.

| Provider | Actual result |
| --- | --- |
| DeepSeek model list in App | Success; returned `deepseek-flash` and `deepseek-v4-pro` |
| DeepSeek generation in released App | Success with `deepseek-flash`, thinking off, input200/output1; fee unknown |
| DeepSeek generation after new native restart | Success again, input200/output1; cumulative counters400/2, actual fee still unknown |
| OpenRouter model list in App | Success using home `.env` |
| OpenRouter free generation | `google/gemma-4-31b-it:free` rate-limited. After about ten minutes and a deliberate switch, one `google/gemma-4-26b-a4b-it:free` check also rate-limited. Further generation requests stopped; tokens/fees remain unknown |
| OpenRouter read-only authentication check | Official `GET /api/v1/key` returned HTTP200, authenticated=true, free-tier=true and spending-limit-reached=false. Key validity is established; successful model generation is not |

DeepSeek's existing connection had root `https://api.deepseek.com` selected as a full endpoint. Changing it to Base URL produced `https://api.deepseek.com/chat/completions`, after which the model query and generation succeeded. OpenRouter used Base URL `https://openrouter.ai/api/v1`. These connection edits were saved; the user's master AI switch remained OFF and no active connection was chosen on their behalf.

OpenRouter's public metadata at test time listed prompt/completion prices0 for the two `:free` models. That metadata is not an actual receipt and was not substituted for unknown fees. Authentication success and an unexhausted spending limit do not identify whether the 429 comes from a daily free quota, provider capacity or another upstream limit.

Primary references: [DeepSeek current model/API information](https://api-docs.deepseek.com/updates/), [OpenRouter model-list API](https://openrouter.ai/docs/api/api-reference/models/list-all-models-and-their-properties), [OpenRouter current-key API](https://openrouter.ai/docs/api/api-reference/api-keys/get-current-key), [Open WebUI tutorial](https://github.com/twtrubiks/open-webui-tutorial).

## Delivery boundary

The tested local candidate application package is `0.1.1-local.3`, built from the product SHA above. It reuses the user's existing data/model locations. The published full offline ZIP **v0.1.1-local.2 is unchanged and does not contain this UI fix**; do not attribute the newer UI or source to that older artifact. Later documentation commits do not represent another App build.

Still unverified here: native real-key replacement/clear, company API/TLS/proxy, clean-host/offline installation, real microphone/ASR sessions and all other P31 physical acceptance items. OpenRouter generation remains unavailable under the observed limit; it is not marked passed or retried continuously.
