# History measurements and research CSV

P22-v1 adds product observations to SessionService. These are independent of diagnostics
ON/OFF/Clear and survive restart. Fresh schema 4 is the first-release format; other nonempty
schemas return `history-reset-required` without a wipe. Follow the separately reviewed clean
installation guide; this feature never reads or removes the old user profile.

The file-backed store keeps exclusive ownership until the last service clone
drops. Its shared private owner explicitly unlocks before closing the file, so a
forked child waiting before exec cannot extend that parent's ownership solely by
retaining an inherited descriptor. Failed lock admission never creates an owner
guard. Drop does not panic; if OS unlock fails, close remains a best-effort fallback
and inherited descriptors can still delay release. No lockfile deletion or retry
loop replaces exclusive admission.

## Sources and units

- `recording.audio_ms` is `native-rate accepted mono frames * 1000 / input rate`, floored to
  milliseconds. It includes received silence before VAD/resampling/decoder padding. Known
  dropped frames mark incomplete; unavailable loss/boundary information marks unknown.
  This is neither microphone wall time nor overlay elapsed or padded decoder input length.
- Requested ASR model is captured at recording start. Actual ID/name/backend moves with the
  loaded engine/stream lease producing the text. Streaming fallback uses its batch producer.
  Current engine contracts return text, not ASR token usage: CSV `asr_tokens_status` is
  `not_reported` and `asr_tokens` is blank (unknown), never copied from API usage or word count.
- `zh-en-v1` counts Han including non-BMP individually and other alphanumeric word runs once.
  Internal apostrophes join a run; hyphens/underscores split; numeric runs count once.
  Whitespace, punctuation, emoji and combining marks add nothing. Other alphabetic runs use
  the same fallback boundary. `我們用 API 測試。` = 6; `don't stop` = 2;
  `session_id` = 2; `API-v2 42` = 3. Both original and current result have separate counts.
- Accepted stop uses a backend monotonic Instant before sound/UI/trailing buffer/scheduling.
  ASR ends after normalized original and capture metadata commit; initial total ends after
  selected-result commit, including optional reverse translation, before human review/delivery.
  Timings are recorded after the respective durable availability through fenced metadata
  updates. Original-only uses original availability and does not fabricate an API result.
  A retained forward translation does not complete review when reverse translation fails,
  times out or is cancelled: initial readiness remains unknown, while its text/count and
  terminal processing/API observations remain available. Retry never replaces the first
  initial-readiness measurement or invents one for an incomplete initial result.
  Latest processing starts at acceptance; retry excludes time since the old stop/human waiting.
  API stage spans first possibly-sent dispatch through its last observed terminal boundary;
  pre-stop streaming work overlaps capture and is not added serially. Unknown endpoints stay
  null. Persisted values are milliseconds/UTC timestamps, never Instant or diagnostic events.

## Calls, options and missing data

Every production SessionService network adapter durably queues a safe descriptor and marks
possibly-sent before HTTP polling. Safe receipts precede output validation. Outcome/drop,
back-translation and explicit retries retain their separate call IDs. No DB lock crosses await.
Response/outcome writes cache bounded safe receipts for the final transaction if an earlier
write fails. Final storage failure is surfaced; original and prepared call remain durable,
no resend occurs. Cancellation/timeout can charge. Late work cannot replace newer text or
recreate deleted history; deletion cascades its calls.

Input/output usage is independently nullable, bounded and checked. Cache/reasoning breakdowns
are never added again. No API is `not_applicable`; no reported field is `unknown`; some missing
fields are `partial`, with a named known subtotal and reported-call count. Active processing
summaries remain incomplete. Returned zero is valid and distinct from unknown.

Actual fees accept only supported explicit amount+currency receipts. Exact decimal strings
(up to 18 fractional digits) use checked fixed arithmetic and sum by currency, with no FX.
The compatible chat/DeepSeek response adapter currently has no documented monetary receipt;
its actual fee is unknown even if an arbitrary response `cost`/`price` field exists. A trial
reservation is a spending guard, never an actual history charge.

Generic profile defaults omit effort/max_tokens. Explicit effort is 1..32 ASCII lowercase,
digits, underscore/hyphen, sent unchanged; support depends on the server. DeepSeek offers
omitted default / none / low / high / max. None sends disabled thinking; low/high/max enable
thinking, with the same reasoning_effort value. Requested/reported effort remain separate;
missing reported effort is unknown. Optional max_tokens is 1..393216 for DeepSeek and
1..1000000 for Generic; omitted means provider default. No automatic retries are added.

## Export contract

`SessionService::export_csv` exports all history through an independent read-only transaction,
regardless of UI search, with one fixed row per history ID and all possibly-sent calls summed.
Fixed headers include UTC times/scenario/status, received audio/quality, ASR models and token
availability, exact original/current result/back-translation text, both counts/rule, durations,
input/output totals/completeness/subtotals, actual money by currency and distinct requested/
reported model-effort associations. Aggregate multivalues are deterministic sorted JSON arrays
inside fixed cells; there are no per-call rows, dynamic call columns or per-call JSON export.

UTF-8 BOM, CRLF records and RFC4180 quote doubling preserve exact Unicode, quotes, commas and
embedded newlines, including formula-like raw text. Export does not silently prefix apostrophes
or promise spreadsheet formula neutralization. Open raw research text with appropriate import
settings when using a spreadsheet. Credentials, endpoint, prompts, headers, raw requests/
responses and reasoning are outside this schema. This is an intentional local text export.

Rows stream through bounded writer buffers. Space is checked with an overflow-safe bound;
an exclusive sibling partial is flushed/synced, then atomically replaces the chosen file.
Failure/cancel preserves an existing destination and cleans only this operation's own partial.
Native IPC returns only saved/cancelled and row count; errors contain fixed codes, no raw paths.
No history mutation, upload or directory cleanup is performed by export.
