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

## Remaining acceptance

Composed final unit/integration counts, review results and exact Tauri binary/payload pins will be recorded after execution. Native WebView2 dictionary CSV download, full App UI, microphone/hotkey end-to-end delivery and a clean Windows 11 host remain distinct from rendered browser tests. Browser/VS Code/Teams/Outlook/Terminal automatic input and rare clipboard-format/blocked-target physical cases are not established by the Notepad++ probe.
