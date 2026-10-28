# Session delivery and settings usability (#31)

Specification: [issue #31](https://github.com/swchen44/notype/issues/31). The user confirmed automatic raw delivery after API failure/timeout, case-insensitive vocabulary deduplication, inclusion of connection-test usage, AI-off applying to all three scenarios, and temporary clipboard paste with restoration. Further Astra plan review was explicitly waived for this work.

## Behavior and boundaries

- AI disabled: dictation, translation and ask all deliver the saved raw transcript without resolving a key or making a text request.
- AI enabled: successful output follows normal review/delivery rules. Processing failure or timeout selects raw text for guarded delivery and preserves the failure for troubleshooting. Cancellation does not become raw fallback.
- History displays time and processed text, falling back to raw text only when no processed text exists. Search uses that same text, literal matching and highlighted ranges. Details/actions/metrics are expanded explicitly.
- Dictionary has a separate sidebar/table, CRUD, search, multiple deletion and UTF-8 BOM CSV. English case and leading/trailing whitespace do not create duplicates. Invalid imports fail as a whole; exports escape formulas for spreadsheet opening while preserving normalized round trips.
- One saved AI connection is active; the master toggle controls all scenarios. Keys remain local, with only masked head/tail previews. Every row has an explicit test and independent usage reset. Last-test status is tied to the configuration revision, not inferred from valid settings.
- Connection accounting persists separately from speech history and diagnostics. It includes actual generation, retry, back-translation and explicit test calls since tracking began or the last reset. Unknown tokens/cost remain unknown or partial. OpenRouter returned `usage.cost` is parsed exactly as USD credits; other providers do not inherit that interpretation. No fee is calculated from prices.

## Native input design

The existing Edit/RichEdit adapter retains directed Unicode insertion. Notepad++ Scintilla uses directed `WM_PASTE` to the captured control, without Ctrl+V routing or foreground activation. The adapter checks original process creation identity, document pointer, foreground/focus epoch, selection, length, encoding, newline conversion and read-only/multiple-selection/virtual-space conditions. Scintilla positions are byte offsets, not UTF-16 positions.

Clipboard formats are duplicated opaquely before replacement; unsupported legacy ownership formats or more than 64 MiB fail before paste. A fresh hidden window owns each temporary clipboard value. Windows may synthesize clipboard formats and change its sequence, so ownership identifies our value; a newer user copy takes precedence. The Windows history/cloud exclusion format accompanies transient text. A worker waits for the directed paste handler and restoration; the caller has a two-second deadline. After a committed/uncertain write the session is never resent. A blocked clipboard/target can keep one cleanup worker alive until the target returns, is destroyed, or the process exits.

Primary references: [Microsoft clipboard formats](https://learn.microsoft.com/en-us/windows/win32/dataxchg/clipboard-formats), [Scintilla API](https://www.scintilla.org/ScintillaDoc.html), [Scintilla message IDs](https://raw.githubusercontent.com/mirror/scintilla/master/include/Scintilla.h), [OpenRouter usage accounting](https://openrouter.ai/docs/guides/guides/usage-accounting). Open WebUI [tutorial screenshots](https://github.com/twtrubiks/open-webui-tutorial) were downloaded and inspected as layout references; no chat/RAG/Arena functionality was added.

## Executed native adapter checks

Native slice `72acb16` was compiled with `rustc` against the cached core release library. This is an adapter probe, not a new Tauri App build or microphone test.

| Check | Actual result |
| --- | --- |
| Native target policy / UTF-8 and newline conversion unit | 2 passed |
| Existing unknown-token native contract before final adapter fixes | 1 passed; final composed contract still required |
| Installed Notepad++, own fresh instance/document, Chinese + English + Emoji + CRLF | `Delivered`; exact saved UTF-8 document 39 bytes |
| Replace selection | `Delivered`; rendered and saved document matched expected text |
| Synthetic clipboard baseline before/after paste | Exact match, restored |
| Reuse the consumed target token | `Unavailable`, no second write |
| Capture then switch to a new blank Notepad++ tab | `TargetChanged`; blank tab remained length 0; replay unavailable |

The native probe is `tests/integration/native_notepad_probe.rs`; it requires a PID supplied by the harness that launched the owned instance and refuses another foreground PID. Computer-use actions used `@oai/sky` on that selected window. No user document, microphone, model download or paid API was used.

Initial native feedback exposed two prototype errors before shipping: Windows clipboard format synthesis invalidated sequence comparison, and a Scintilla selection-anchor message was mistaken for a virtual-space message. The final runtime checks above cover a nonzero caret and real clipboard restoration after those fixes. Temporary debug instrumentation was removed.

## Final source and executed checks (2026-10-28)

Product source: `d7d28645ec275c0b64b8590cc9013ffcc21f422c`. Independent Spec and Standards reviews both finished with **0 unresolved must-fix findings**, including the last connection-test copy change. Earlier findings about stale readiness after enabling AI, preservation of processing errors after failed fallback delivery, public API documentation and clipboard ownership uncertainty were fixed and rechecked. Documentation commits after this source do not represent another binary build.

| Gate | Actual result |
| --- | --- |
| [Shared ALL CI 40607807011](https://github.com/swchen44/notype/actions/runs/40607807011) | Both jobs succeeded at the exact product source; UI unit 12/12, rendered integration 112/112, 0 skipped; frontend lint, TypeScript and Vite build passed |
| Core/desktop/Node checks in that run | Rust core, platform, bootstrap, workflow, HTTP, SQLite/activity, dictionary, metrics and probe contracts plus Clippy passed. Node unit 33/33; integration 24 passed and 1 explicitly Windows-CMD-only test skipped on Linux. That skip is not counted as Windows acceptance |
| [Windows CI 40607811545](https://github.com/swchen44/notype/actions/runs/40607811545) | Contracts and Clippy succeeded; actual isolated native clipboard fixture 1 test / 4 cases passed, 0 ignored; completion-state unit 2 passed; standalone provider helper runtime 25 passed. Artifact-build job intentionally skipped; this is not the App build |
| Local native App build | Tauri Release completed in 298,858 ms; source remained exact/clean. 59,353,088-byte executable; SHA-256 `e456219eb59d625680c452bdf6a1f987bbe6d1d3262ca1610b95eea063a33419`. The 26 existing unused-code/import warnings remain; no native compile errors |
| Complete local native payload | 1,695 declared files verified, including complete Fixed WebView2/runtime and build provenance; actual final App launched using an isolated owned profile |

The first older CI attempt exposed obsolete standalone helper expectations for actual returned USD receipts. Its exporter and expectations were corrected without weakening transport/budget guards. Final Linux and Windows runs above passed. Local private-window-station creation returned AccessDenied before clipboard access; it was not elevated or retried on the user's interactive clipboard. The isolated native fixture subsequently passed on Windows CI.

### Actual Windows App and adapter observations

Computer Use used `@oai/sky` on selected owned windows. The profile contained only synthetic histories, words and an auth-free loopback connection; all recording shortcuts were disabled. No OS-home key reader, paid request, microphone, user document or user history was used.

- The final Notepad++ adapter was rechecked with an owned fresh document: Chinese/English/Emoji/CRLF and selection replacement delivered; the synthetic clipboard baseline was restored exactly. Consumed tokens could not replay. A changed tab caused `TargetChanged` with no write. This adapter source is unchanged between that check and the final product source.
- Native dictionary table displayed owned words. Its actual WebView2 download produced a UTF-8 BOM CSV with exactly the three expected words. Native history export used the real Save As dialog; the CSV retained both owned rows even when search displayed only one, and included processing and delivery error columns.
- History default displayed timestamp plus primary text; searching `OpenAI` hid the other row and highlighted the match.
- An owned localhost server observed exactly **2 POSTs + 1 GET**: one successful connection test, one HTTP 429, and an explicit model-list query. There was no automatic retry. Successful usage was input 3 / output 1; after the unknown 429 receipt, UI showed partial `3 + unknown` / `1 + unknown`, fee unknown, and failed last-test status. These values persisted after a full App restart. Final UI uses dedicated test failure text and no longer claims a speech draft was saved by a connection test.
- The final 682×600 native window displayed the complete lower-left AI status, selected endpoint/model and last-test result. Main-window X hid it while its owned process stayed alive; relaunch restored the same window. The prior candidate's About → Quit also exited its process normally.
- The actual folder chooser scanned the user's existing model folder, found **1 available model**, applied it, and loaded **Qwen3-ASR 0.6B Q6_K** to “in use” with a green footer. No model was moved, copied or downloaded. This establishes reuse/loading, not speech recognition quality or relocation/reinstallation.
- [Issue #30](https://github.com/swchen44/notype/issues/30) remains: the generic busy hint can persist despite Ready / successful Apply / successful model load. It is not hidden by weakening runtime admission checks.

### Complete offline package

Private prerelease: [v0.1.1-local.2](https://github.com/swchen44/notype/releases/tag/v0.1.1-local.2). ZIP **1,053,413,929 bytes**, **1,722 files / 1,581,582,522 expanded bytes**, SHA-256 `6a2a6d1875dcc5e2f5013e32b508e4738cf8afe045674e7c3cb6fb01843343b5`.

Executed separately from native UI: complete staged inventory/hashes, actual bundled Node `--offline-bundle --check`, strict ZIP32 path/inventory/size/CRC/SHA audit, fresh actual extraction and complete extracted-bundle verification. Application manifest SHA-256 `7cf0dab99d6a57297f89587db73cb406fb9b30e9607122060a374c6773f327e6`; independent model manifest remains `336ec3a01380569b85b39d56ee555d1c0932bfa5196a15977e397ccfca4b84a9`. Runtime/model reuse came from the verified immutable previous bundle, not the user's model folder. Older release assets remain unchanged.

## Remaining acceptance

Still unexecuted: real microphone/hotkey-to-ASR-to-App session; right-Alt latency and recording seconds; mixed Chinese/English quality/noise/performance; device removal and sleep/wake; actual model relocation/reinstallation and OS permission/path combinations; native diagnostics OFF/Clear/export privacy; Excel CSV opening; actual cloud/company APIs, TLS/CA/proxy and unstable internal networking; full clean Windows 11 offline launch without development tools. Previous API experiment failures remain recorded in their own receipts and are not converted into success by these loopback tests.

Browser/VS Code/Teams/Outlook/Terminal automatic input, rare clipboard formats, clipboard contention and blocked-target physical cases are not established by the Notepad++ probe. Portable Linux CI does not certify a Linux/macOS desktop application. Full-package integrity and native payload launch are separate evidence; no clean-host claim is made. User testing should start in a blank owned document, with the previous App fully exited.
