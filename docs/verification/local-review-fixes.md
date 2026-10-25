# Local milestone review corrections

2026-10-25 Asia/Taipei. This batch addresses all seven findings in [the independent two-axis review](local-code-review.md), based on `37f5bdae8cbd8974ae99fb837908716ef9bbad24`, in isolated branch `ticket/local-review-fixes`.

| Finding | Correction and evidence |
| --- | --- |
| P1: cancelled STT cancels a new recording | `GenerationToken` atomically fences short platform operations and cancellation against a new begin. The native finish guard, engine return, stream emissions, queued UI application and coordinator notifications carry that owner. A cancelled A cannot remove B's workflow, stop/readiness state, stream route or cleanup. The public workflow regression gates old stop/commit and decoder completion, runs delayed UI/cleanup after B starts, and saves B through real SQLite. It also verifies B's own cleanup still runs once |
| P2: native data root overlaps sibling npm roles | Native preflight calls `validate_external_data_directory`, rejecting overlap in either direction with application, façade and model packages. Existing aliases and missing descendants are resolved, including known role directories at npm ancestor levels. No environment, manifest or model-activation schema changes. A real temporary three-role layout rejects all three and permits external storage |
| P2: unquoted paths changed by script conversion | Protect UNC, token-boundary POSIX/home/dot-relative paths and token-delimited relative filenames with extensions before lexical conversion. Ambiguous slash prose converts normally. Both script profiles save exact Chinese path segments while adjacent prose converts. Existing code/URL/drive-path/manual-term tests still pass |
| Standards: copy rejection has no feedback | The existing copy handler catches rejected IPC promises and shows the existing translated `copyFailed` toast. No testing-only UI abstraction was added |
| Standards: public API documentation | Workflow/provider types and APIs now document generation ownership, capture order, recording snapshots versus retry preferences, cancellation, review and delivery effects. Coordinator notification APIs also document selecting the cancelled/completed owner and rechecking it when queued messages are consumed |
| Standards: modified inline product tests | Chinese tray-locale regression lives in `tests/unittest/desktop_tray_locales.rs`, compiling through the real native module/generated translations. The initial two tests of a private tray text helper were removed during follow-up; public session tests already cover processed output, explicit original selection and missing-result recovery without a clipboard. No private helper was exported for testing. Unchanged imported tray tests remain in place |
| Heuristic: duplicated shortcut defaults | The concern was applicable. Canonical strings live in `physical_gestures`; its router parses them and settings/Reset/migration reuse them. The UI reads backend bindings and no longer invents a second fallback default table. No new JSON dependency or broad shortcut rewrite |

The two introduced Release warnings were also corrected: debug-only `Duration` import in actions and the release-unused native scope PID argument. The inherited `secure_input::Emitter` and other unchanged native-test warnings remain.

## Cancellation ownership details

The ownership lock is held only for short state/resource handoffs, never across native decoding, HTTP, stream-finalization waits or the configurable extra-audio buffer wait. Esc atomically cancels workflow and platform resources; it can discard a Stopping capture so B starts while A's decoder is still running. Audio stop checks ownership/cancellation before invalidating readiness, then rechecks before draining the recorder, setting Idle or closing the microphone. Delayed A stop cannot consume a same-binding B buffer.

The stream sender/reply is detached under ownership before its background wait, so a late task never takes B's global stream route. Engine return/panic bookkeeping and finish-time unloading are owner-scoped; headless one-shot recognition retains its separate unload preference. UI ownership is checked when the main-thread callback applies, and the existing overlay show-generation check still protects its delayed fade hide. Queued coordinator cancellation/finished transitions also carry the generation owner. Platform cleanup still runs if persistence cancellation reports an error, preserving Esc's best-effort microphone shutdown.

The regression uses the same public owner/callback boundary as native actions with controlled old-stop/decoder gates and actual session persistence, not a private-field assertion or live microphone. It does not certify real microphone hardware, OS audio timing or model inference.

## RED and GREEN

- At the extracted public ownership boundary retaining the previous unconditional cancel/drop behavior, the restart regression failed with `active_generation=None` instead of B (`Some(2)`). It now passes for delayed initial stop and delayed stop commit/decoder cleanup, verifies old queued UI is ignored, saves B's original, and verifies B cleanup once.
- Path regression initially saved `頭髮` inside exact `\\server\头发\log.txt`, `/tmp/头发/log.txt` and relative paths. A first expression also caught a surrounding prose prefix; that failure was retained and corrected before the final pass. Both Traditional and Simplified expectations now pass, with surrounding prose converted.
- Native data-policy regression initially returned Ok for the sibling façade/model directories. It now rejects all three versioned roles without creating data.
- An initial overly narrow exact-name test filter selected zero tests; it was not accepted as verification. Subsequent full-target executions produced the actual RED/GREEN results above.

Initial executed checks, before the focused follow-up below:

| Check | Result |
| --- | --- |
| Desktop Release contracts | 31 passed, 0 failed/ignored: 12 workflow, 12 HTTP/provider, 6 physical gestures, 1 native unknown-token test. The separate debug-only owner test is not compiled in Release |
| Native platform contracts | 4 passed, 0 failed/ignored; library/bin/doc targets each contain zero tests and are not counted |
| Native Release library, tray filter | 7 passed, 0 failed/ignored, 250 filtered; actual generated i18n and copy/session helpers. Native library compiled in 1m41s; inherited test dead-code/import warnings remain |
| Strengthened B-cleanup regression | 1 passed after adding explicit B cleanup-once and old queued-UI/commit assertions |
| Frontend | ESLint, TypeScript/Vite (2031 modules), changed-file Prettier passed; existing large theme-chunk warning |
| Formatting | Rustfmt and `git diff --check` passed |

Commands ran from `desktop/` with shared `CARGO_TARGET_DIR=D:/git/notype-build`, global Cargo home, Release debug=0, LTO=off, codegen-units=16, incremental=0, jobs=1 and task-local compiler TEMP/TMP. No second target or retired core cache was recreated:

```powershell
cargo test --offline --locked --release --manifest-path ../tests/integration/desktop-contracts/Cargo.toml --test desktop-workflow --test desktop-provider --test physical-gestures --test native-delivery
cargo test --offline --locked --release --manifest-path ../crates/notype-platform/Cargo.toml
# Existing verified CPU prebuilt prefix, no C++ rebuild:
$env:TRANSCRIBE_DIR='C:/Users/swche/AppData/Local/notype-study/build-cache/native-prebuilt/transcribe-cpp-0.2.4-win64-cpu-release-E4b0TR'
cargo test --offline --locked --release --manifest-path src-tauri/Cargo.toml --lib --features tauri/custom-protocol tray -- --nocapture
bun run lint
bun run build
```

The crate itself has no `custom-protocol` feature; the real successful native test used the dependency feature `tauri/custom-protocol`. The filtered test executable SHA256 was `d8b28f2c527a154dc964d34f05ce7de8badb7b65eb3f275fa709563053a5b964`. The existing Release App remained unchanged at `8e7847d123f39a8126c18a704730a743b0eedc14c18154d22f9dbf28a9e829dd`; final App/helper relinking and package cold-launch validation belong to #14 after integration.

No App/GUI, user helper, microphone, clipboard, registry operation, speech/model inference or publication was performed in this batch. Previous failed native-fixture and model-quality evidence was not changed. Company API/Proxy and other-App automatic delivery remain deferred under the approved local recording → bilingual local STT → history → copy milestone.

## Focused follow-up

The read-only follow-up of `e3e2746` found two behavioral regressions and two standards residuals. All four were corrected in the same isolated branch; the original RED evidence and historical counts above remain recorded.

- **Slash prose:** the new public save-boundary test first failed with `请启用/关闭功能` left unconverted and the suffix of `启动/停止录音` partially protected. Both script directions now convert ordinary slash prose while preserving the genuine `/tmp/头发/log.txt`, `src/头发.rs`, explicit `./`/`../` paths and adjacent UNC paths. Rooted paths require a token boundary, so a mid-prose slash cannot start an absolute path match. Bare unquoted relative files require a filename extension; ambiguous unprefixed directory-like slash expressions convert as prose. Quoting, an explicit `./` prefix or protected terms remain available for such literals.
- **Blocking start feedback:** a controlled playback callback at the production `GenerationToken::readiness_feedback` boundary reproduced the old ownership lock: stop could not finish until the callback was released. Playback now runs outside the lifecycle mutex, between two short readiness handoffs. The regression proves both stop and Esc→begin B complete before sound release, rejects late mute after readiness invalidation or generation change, and persists B's original through the public workflow. The two-second timeout only bounds a blocked-test failure; it is not a product timer or audio-latency benchmark. No actual audio device was opened. A cue already in flight can finish, but it cannot retain capture ownership or mute the successor.
- **Notification docs:** `notify_cancel` and `notify_processing_finished` document choosing the cancelled/completed owner's token and rechecking queued ownership when consumed.
- **Public tests:** removed the two private `last_transcript_text` tests and their test-only module import. Existing public `core-session` tests verify processed output delivery, explicit original selection after failure, and recoverable missing-result original persistence without clipboard access. The private tray helper remains private; no product API was added just for those tests. The locale regression still uses the actual public translation interface.

Follow-up checks passed:

| Check | Result |
| --- | --- |
| Full desktop Release contracts | **33 passed**, 0 failed/ignored: 14 workflow, 12 provider, 6 gestures, 1 native token; includes both new regressions |
| Public core-session contracts | **9 passed**, 0 failed/ignored, including the three selection/recovery cases above |
| Formatting | Rustfmt on changed Rust files and `git diff --check` passed |

The desktop command above was repeated with all four targets; public session verification used `cargo test --offline --locked --release --manifest-path ../crates/notype-core/Cargo.toml --test core-session` from `desktop/`, with the same existing Release target and jobs=1 settings. The standalone core lockfile needed a small Rust-only dependency rebuild; no C++ feature/build, second cache or retired target was used. The native tray filter was not rerun after removing the two redundant tests; its earlier 7-pass count is historical, and five tests now remain in that filter. Frontend sources did not change in this follow-up, so the prior frontend checks remain applicable. Final App/helper relinking is still delegated to #14 after integration; no App was built or launched during this follow-up.
