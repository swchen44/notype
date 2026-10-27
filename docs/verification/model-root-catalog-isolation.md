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

## Native build and actual GUI acceptance

The user subsequently requested compilation/computer-use and explicitly waived further Astra review. Build/acceptance records: [initial scope](https://github.com/swchen44/notype/issues/28#issuecomment-6443709618), [ONNX cache correction](https://github.com/swchen44/notype/issues/28#issuecomment-6444082023), [frontend staging correction](https://github.com/swchen44/notype/issues/28#issuecomment-6444508663).

- R1 failed linking because `CARGO_NET_OFFLINE=true` makes ort-sys skip its binary-link setup, despite an existing local ONNX distribution. R2 restored normal cache resolution and compiled, but the temporary Windows absolute `frontendDist` override was parsed as an external URL and displayed a drive index. These were local build-configuration failures; their logs/results remain preserved. Production source was not changed to work around either issue.
- R3 used the normal `../dist` layout with the nine previously built frontend assets. `tauri build --no-bundle` completed in 4m44s, source **f200d5dd21426caab1f8c8ac336dc128c94aaf55**, clean before/after. Existing Rust unused-code warnings remain; this is not a clippy run. The executable is **59,283,968 bytes**, SHA-256 **de9e655bea57bb91111ca113b22e54d8ec24c011a4a9f20674d1a6b732d3b94b**.
- A separate full local portable payload contains **1,694 declared files / 805,265,575 bytes**, with the new executable and unchanged hash-checked runtime/resources from the existing release. Native integrity and normal-user checks remain enabled. A bare executable directory was correctly rejected; an elevated launch was correctly rejected; normal Explorer launch of the full payload succeeded. The published release extraction was never modified.
- Actual Windows UI Automation and own-window captures selected the existing folder in the native chooser, confirmed a Ready candidate with **1 available model**, invoked Apply, and observed the applied external path and **Qwen3-ASR 0.6B (Q6_K)** marked downloaded/verified. Invoking **Use model** showed the loading state, then completed onboarding and opened the main App with a green Qwen footer. Invoking Rescan kept the same root/model available. This is the original GUI flow, not a fixture or direct command injection.
- The production preference file persisted schema1/revision1 and the external model root. The existing 690MB speech model was not moved, copied, deleted or downloaded. No microphone recording, API call or credential access was part of this acceptance. Captures stay local; only these nonsecret results are published.

## Remaining acceptance and known finding

The already-published ZIP/tag/executable still contain the original bug; the successful new build is currently a separate local test payload, not a replacement GitHub release. Physical microphone transcription, quality, actual API operation, cross-App input, clean official host and restart persistence were not exercised in this run. The folder preference's durable write was checked, but it is not a new restart test.

The GUI also retains a busy hint (“請完成或取消目前工作…”) while the App is idle and the folder/model are Ready. Apply, loading and Rescan succeeded despite it. This stale hint is a separate observed finding, not silently declared fixed by the catalog change.
