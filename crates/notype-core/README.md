# notype-core

Portable session and text-history rules, independent of Tauri and operating-system APIs. The desktop adapter supplies a resolved database path, a recognition-mode snapshot, an opaque original-target token and the external text/delivery implementations.

```rust,no_run
use notype_core::{RecognitionMode, SessionOptions, SessionService};
# fn example() -> Result<(), notype_core::CoreError> {
let sessions = SessionService::open("/chosen/persistent/directory/history.db")?;
let original = sessions.create("不要刪除 API 42", SessionOptions {
    recognition_mode: RecognitionMode::TraditionalEnglish,
    original_target: Some("adapter-owned-target-token".into()),
    ..SessionOptions::default()
})?;
assert_eq!(sessions.get(&original.id)?.transcript, "不要刪除 API 42");
# Ok(()) }
```

`create` durably saves the original before `process` can call a provider. `process` also retries a saved entry without audio, using its saved recognition mode, scenario, translation target and minimized source context. Its `TextRequest` adds only the supplied manual glossary and whether independent translation review is enabled; the adapter owns editable prompt snapshots. `SourceContext` accepts only an optional app name and domain, rejecting full URL/path/query syntax. The core never discovers application metadata or reads UI locale, application text, recordings, credentials or unrelated history to construct a request.

For Translation with review enabled, `process_with_review` commits the primary text and detected source language before calling `back_translate`. The reverse request cannot contain the original transcript. Missing source metadata or reverse failure leaves the primary recoverable, without delivery. Cancellation/interruption also retains an already committed primary. Review defaults off at the desktop boundary. `confirm_review(id, revision)` approves only the displayed pending revision, without activating a window or delivering. Retrying or choosing the original changes the revision and clears approval. Delivery refuses an unconfirmed review even when called directly; after approval it still uses the original-target guard. This is a human comparison aid, not a semantic-quality certification.

Processing success creates a Pending result. Failure preserves the original and cannot deliver it automatically. `choose_original` is the explicit action that makes the original eligible for delivery. `get` and literal `search` return original and result separately. `delete` cancels processing and removes the entry; deletion during an active delivery returns Busy.

`TextProcessor` and `DeliveryAdapter` return owned Send futures. Each processing attempt receives its own cancellation token. `cancel` and `delete` signal it; the core also drops a cancelled provider future and checks the attempt generation before accepting output. Dropping or aborting a processing task records failed/interrupted and permits retry. Adapters must configure a finite timeout and map failures to `TextFailure`, without exposing private diagnostics.

Delivery is persisted as Delivering before the adapter runs. The adapter must guard the captured target and insertion together and must never synthesize Enter. A changed/unavailable target remains Pending. A possibly inserted result becomes DeliveryUnknown and cannot be retried, reprocessed or selected for automatic delivery. Aborted delivery futures and cancellation after delivery starts also become uncertain. Successful delivery cannot run twice. The core cannot prove native target identity; Windows/macOS/Linux adapters and their real environment checks own that guarantee.

`open` acquires a lifetime sidecar file lock before schema changes or interruption recovery. Clones share ownership; another cooperating core instance cannot recover the first instance's active rows. Interrupted Processing rows recover to Failed; Delivering rows recover to DeliveryUnknown. SQLite uses WAL and FULL synchronous writes. Schema v1 from the Node diagnostic prototype migrates transactionally without replacing IDs, originals, results or creation times. Future schemas are rejected. Stop the old diagnostic prototype before migrating: its older process does not honor this new owner lock. Keep the sidecar alongside the database; deleting it while an owner is active can defeat cooperative locking on some platforms.

`import_handy_history` reads the pinned Handy `transcription_history` schema through a read-only connection and transactionally imports text into the caller's destination. Source database and recordings remain untouched. Unknown recognition modes/targets remain None. Import tombstones prevent repeat imports from restoring manually deleted destination text. Importing the same source under a different path is a distinct import; the caller should retain one canonical source path. An interrupted import rolls back; there is no automatic source migration or deletion. Directly opening a Handy database returns LegacyImportRequired.

`HttpTextProcessor` implements only the development fixture protocol POST `{transcript, glossary}` → `{text}`. It deliberately excludes recognition-mode metadata and target/history data from HTTP. It accepts HTTPS or explicit loopback HTTP, uses a finite caller-selected timeout, rejects redirects, ignores environment proxy settings, limits response size and returns sanitized errors. It has no credential or company-specific protocol support. Company compatibility still requires its actual contract and an external adapter.

Run from the repository root (the crate is a standalone workspace):

```powershell
cargo test --locked --manifest-path crates/notype-core/Cargo.toml --test core-session
cargo test --locked --manifest-path crates/notype-core/Cargo.toml --test core-persistence --test core-http
cargo test --locked --manifest-path crates/notype-core/Cargo.toml --doc
cargo clippy --locked --manifest-path crates/notype-core/Cargo.toml --all-targets -- -D warnings
```

Session tests are in `tests/unittest/`; real SQLite/restart/migration and loopback HTTP tests are in `tests/integration/`. These tests need neither an API key nor a real speech model. They do not claim native microphone, STT quality, target application or company API acceptance.
