# Existing model folder rejected by the shipped catalog (#28)

2026-10-27. Plan [P28-catalog-isolation-v2](https://github.com/swchen44/notype/issues/28#issuecomment-6442487484), independent [GPT-6-Astra approval, 0 unresolved](https://github.com/swchen44/notype/issues/28#issuecomment-6442501643).

## Actual Windows GUI reproduction

The released v0.1.1-local.1 App was already running on the user's desktop. Windows UI Automation and a foreground-only window capture drove General → Choose model folder → the existing `local-trial/models` directory → Select Folder. The native picker showed the correct directory. Validation then returned “請選擇可讀取且包含有效模型的本機資料夾。” before an Apply candidate appeared. Rescan did not recover the default root. No credentials, recording, API calls, downloads or model-file writes were involved. Screenshots remain local and were not uploaded.

## Isolated cause and change

A read-only harness included the unmodified production root service/filesystem/activity modules. Canonicalizing the real existing folder succeeded. Scanning with all 367 shipped catalog descriptors returned `model-root-invalid`; removing only six `bundle/*.gguf` descriptors returned Ready and SHA-verified the existing 690,417,824-byte `Qwen3-ASR-0.6B-Q6_K.gguf`. The App's flat-root scanner treated a nested catalog filename as a failure of the whole selected folder.

The scanner now retains unsupported catalog anchors as Unavailable, without resolving their filenames or attaching a backing descriptor. Valid flat entries still require the existing size/hash/identity checks. One shared download capability predicate disables their download UI and rejects download admission before preparation, worker registration or network work. This does not add nested model support or change ownership/deletion rules.

## Source verification

- Two real-filesystem regressions parsing the actual shipped catalog failed on the pristine source with `model-root-invalid`, then passed after the fix.
- Windows direct-rustc contract runs: **35 integration-target tests and 7 unit-target tests passed**, none skipped. These compile the checked-out production modules using cached dependency rlibs; they are not a new full Cargo/native App build. The integration target includes module-local probe tests in its count.
- The new integration paths validate/apply, persist/reopen and rescan a tiny hash-verified model alongside all 367 real catalog anchors. All six unsupported entries remain non-present, non-loadable, non-downloadable, unowned and undeletable through the root service. The selected directory remains unchanged. An empty safe root remains selectable with NoModels. Existing corruption, ownership, cancellation, stale generation, read-only and Windows identity tests also pass.
- A separate read-only run against the actual existing Qwen file with the fixed production modules and the full, unfiltered 367-entry catalog returned Ready and verified the model hash. This is filesystem reuse evidence, not speech recognition or GUI acceptance.

Reproducible Cargo targets for normal development/CI:

```powershell
cargo test --locked --manifest-path tests/integration/desktop-contracts/Cargo.toml --test model-roots-policy --test model-roots
```

## Remaining acceptance

The already-published ZIP, tag and executable are unchanged and still contain the reproduced bug. A newly built App must repeat the original Choose → Apply → Use model GUI flow before this is claimed fixed for the user. Native build/clippy, packaged GUI operation, actual microphone transcription and model quality are separate checks; source contracts and hash verification do not establish them. A new native build/delivery requires its own bounded plan and Astra approval.
