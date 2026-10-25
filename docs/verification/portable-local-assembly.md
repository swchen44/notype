# #14 local assembly progress

2026-10-25, following foundation commit `5dcf3f1` and integration QA scope fix `6d5fc9921d4f0fa2e9e088ffd33f204701b8f562`. **Local assembly is in progress; this receipt does not claim a complete portable artifact or #14 completion.** Company API/Proxy/public publication remain deferred under the user's local-first request.

## Verified source contracts

The launcher now composes full package/hash/role verification, normal-token/CPU checks, local model activation, then shell-free native spawn. Source release descriptors remain private/unassembled so this code cannot accidentally launch placeholders. The model is the verified local-pilot Qwen3-ASR 0.6B Q6_K; actual model provenance/quality remain in the separate Qwen receipts, without another full matrix run here.

Default data follows actual Windows RoamingAppData/`io.github.swchen44.notype` and leaves NOTYPE_DATA_DIR absent unless the user supplied it. This retains the previous HF_HOME/global-cache behavior and valid model selections; injecting the same default root through that override would change native cache lookup. UDF is `<data>/webview`, matching existing main/overlay builders. No history/settings store is read or overwritten by model activation.

Copied weights are hashed, synced and exclusively published without replacing a preexisting model; matching models are reused. Cancellation/retry, simultaneous instances, changed source, insufficient copy margin and an actual killed Node copy/restart are covered. Only exact current-content-hash/PID/UUID reserved partial names from a proven-dead process are eligible for cleanup; unknown/live processes, links and unrelated files are retained. Atomic publication requires hard-link support and otherwise fails clearly. This is tested with synthetic bytes, not a claim that a real 690 MB copy/upgrade has already passed.

The independent `notype-platform` adapter and standalone baseline probe use Rust/std, serde/sha2 and Windows system APIs, with no Tauri/ORT/ggml/WebView imports. Actual token query in the elevated agent returns unsupported; CPU reports AVX2/BMI2 and Known Folder reports the correct roaming directory. PE imports include **VCRUNTIME140.dll** in addition to Windows API-set/UCRT/system libraries. Therefore the helper sits alongside the app-local audited CRT, and hostProbeLibraries must declare/verify that dependency before the helper can run. A developer-installed CRT is not portable closure evidence.

Release entry-point source independently validates the regular-byte application/runtime inventory, its token and data placement before Tauri/history/tray setup; directly invoked invalid packages fail with a clear native diagnostic. This glue is pending the true Release compilation/owned cold-launch proof. GUI startup prewarms the existing script converter in a background thread; the earlier headless CLI path retains cold timing behavior. No new latency/p95 result is claimed.

The local package explicitly reports startup registration unavailable: a direct login invocation would bypass facade model activation and lose a shell-only data override. The visible toggle is disabled with a localized explanation; the command refuses before changing settings, and startup skips plugin enable/disable. Existing preferences/OS entries are retained, with no HKCU writes or login acceptance test in this task. A stable verified login-launcher integration remains future work.

Pinned transcribe.cpp derives default dynamic-backend lookup from its own DLL location. However `ggml-backend-reg.cpp:600` additionally honors GGML_BACKEND_PATH even with an explicit scan directory; both launcher and native release guard remove that override. Actual loaded module paths from the final payload still need observation.

## Actual checks before release build

| Check | Result |
| --- | --- |
| `npm.cmd run test:unit` | **23 passed**, including 12 portable contracts |
| `npm.cmd run test:integration` | **16 passed**, including 7 activation, 3 CLI and 1 real small-package tarball contract |
| `cargo test --offline --locked --manifest-path crates/notype-platform/Cargo.toml --test portable-native` | **3 passed** |
| platform adapter `cargo clippy ... --all-targets -- -D warnings` | Passed |
| host-filtered desktop `cargo metadata --offline --locked --filter-platform x86_64-pc-windows-msvc` | Passed; lock adds only the path adapter (10 lines) |

TDD REDs included changed default HF behavior, absent activation module, cancellation, actual killed-copy partial cleanup, insufficient storage, out-of-tree backend override and omitted helper CRT; corresponding public contracts became GREEN. No tests were skipped. The unrestricted metadata query first refused uncached Linux ALSA metadata offline; the Windows-filtered query resolved locally. No dependency download or native build occurred in that query.

Small contract builds use the existing isolated cargo-core home and D:/git/notype-core-build, debug=0/incremental=0/jobs1; public source registry only, no credential/config copies. Node fixture TEMP/TMP/npm cache were redirected into that owned target after C free space dropped; no large staging/model copies were started.

## Native input preservation and authorized cache retirement

Regular-byte transcribe-cpp install subset preserved at:

`C:/Users/swche/AppData/Local/notype-study/build-cache/native-prebuilt/transcribe-cpp-0.2.4-win64-cpu-release-E4b0TR`

Exactly **44 files / 12,055,325 bytes** across bin/lib/include, every source/copy/re-source hash verified. Link manifest, source CMakeCache and real CPU flags are retained. GGML_NATIVE=OFF, conservative x64/all CPU variants/dynamic modules enabled; these do not certify ORT's minimum instruction baseline. C tcs remains an untouched junction into the retired old debug location; the new prefix is independent. ORT/DirectML C cache was retained untouched.

A tiny actual build-script probe proved `cargo --offline` leaves build-script CARGO_NET_OFFLINE **unset** when not explicitly supplied. Explicit CARGO_NET_OFFLINE/ORT_OFFLINE/ORT_SKIP_DOWNLOAD must not trigger the cached ORT route's skip-link behavior. No network/bootstrapper is used to repair a missing cache.

After #13 released all jobs, the parent authorized retiring only the exact derived D:/git/notype-build/debug tree. Preflight inspected **301 process module inventories, 0 inaccessible, 0 target hits**; no Cargo/rustc/link/cmake remained. The independent prefix and the final 71-file regular 9fbe QA app snapshot were rehashed first. The resolved debug root was a normal directory. **10,445 derived files** and **3 reparse entries** were removed; links were unlinked without following targets. Actual D free grew **663,457,792 → 5,845,901,312 bytes**, an increase of **5,182,443,520 bytes**. No core target, worktree, registry cache, ORT, model, user data or C junction was deleted.

Detailed outside receipts: agent-notes/native-debug-retirement.json, the prefix preservation.json/CMakeCache.source.txt, and agent-notes/offline-env-probe source/output. The preserved QA app is the source-isolated 9fbe snapshot, not the pending Release binary.

## Next build/acceptance frontier

One actual desktop Release uses rlib-only, lto="off", codegen-units=16, debug=0, incremental=false, jobs1, the preserved TRANSCRIBE_DIR, existing native Cargo cache/default cached ORT route, --offline/--locked, and D-local compiler/link TEMP. Monitor D>=1 GiB and C>=2 GiB; stop only the owned build if it cannot fit. No second native cache or OS/pagefile/user-model cleanup is authorized.

Still pending: actual release compile, baseline helper build provenance/CRT closure, bounded final package assembly, complete per-file/notices manifests and true npm tarball/install inventory; verified real-model activation/native offline discovery/inference; normal-user fixed-runtime/UDF/module-path/no-download observation; invalid direct/facade startup and two-version interrupted/file-lock update/history restart receipts. App launch requires an owned slot coordinated by the parent. No real company API, public registry publication, microphone or broad human-app interaction has occurred in this slice.
