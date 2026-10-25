# #14 private local artifact verification

2026-10-25, Windows 11 x64. **The copy-first local pilot has actual artifacts, an offline installation and bounded native runtime proof. This does not close #14 or claim full portable/public deployment acceptance.**

## Source and builds

Actual App/helper source is `239d6f05cdf7883032889b8d25042a7a427d72d9`, including independently accepted review fixes `dd40397`. CI source `a2d23286643b7eb36da97685ce759ef43994c4b8` differs only by two expression line wraps in the platform library; whitespace-normalized source equality was checked. [CI40162614988](https://github.com/swchen44/notype/actions/runs/40162614988) passed core27, desktop34, platform4, Node23 unit +16 integration, formatting and clippy. These counts are separate from runtime cases below.

The true Release App passed: Cargo6m42s, total runner about7m16s including frontend TypeScript/Vite. It reused `D:/git/notype-build`, global Cargo, the independently preserved C transcribe-cpp prefix, `--offline --locked`, jobs1, rlib-only, LTO off, codegen16, debug0, incremental off. No new C++ build/download. Only the existing `secure_input.rs` unused `Emitter` warning remains.

| Actual binary | Bytes | SHA256 |
| --- | ---: | --- |
| App | 57,063,936 | `7a70d5abe3558c2b12d2c8a16c0ed583bf0a231d3cbe8ba14fa89300b74257ab` |
| Baseline helper | 146,944 | `6cd855cb408ccee695d800ce70de8c94ddd76e4d38e510a50c68976aac5bd38d` |

The separate helper passed23.02s. Only it used `target-cpu=x86-64` and `-avx,-avx2,-bmi2`; App/ORT/native flags were unchanged. Its actual imports include app-local `VCRUNTIME140.dll` and Windows inbox API/UCRT libraries, without speech/WebView imports. The previous pre-review App/hash was archived separately and is not delivered. Build logs/disk samples are under `D:/git/notype-build/release-receipts/review-fixed/`.

## Actual private tarballs

Under `D:/git/notype-local-artifacts/`, all three have `private:true`, no lifecycle scripts, and actual streamed tar entry/size/SHA256 comparisons against their allowlists. No cache symlinks, settings, history databases, recordings or keys are included.

| Artifact | Compressed bytes | Unpacked bytes | Regular files | SHA256 |
| --- | ---: | ---: | ---: | --- |
| `notype-0.1.0-local.1.tgz` | 9,887 | 29,285 | 8 | `abb25ede159633df0aab078b30479c6f79c3d10bc7339991c8db2eb671247462` |
| `notype-win32-x64-0.1.0-local.1.tgz` | 356,789,619 | 803,455,282 | 1,696 | `818c11c7094e9a3195b4dad06e27aa774d9f163d7cff44aad2ee59b532ac1241` |
| `notype-model-0.1.0-qwen3-q6k.1.tgz` | 668,899,885 | 690,434,160 | 8 | `6e4595abca952f2c11d3c91189ecd9c4efa392766ec813c7ce84078136b7be0d` |

Application manifest SHA256 is `c7bcd4f53e1d7bb29ab360d6f5572bad9f285d9bafb1a23987c2ef4676597b52`; model manifest is `36a988d8a0bb3fec912a8655e69a7977a40c8a9e648c26d1e8d37900c25a0556`.

The complete257-file Fixed WebView2 `156.0.4314.42` tree, twelve CPP DLLs, mandatory regular-byte DirectML, four app-local VC CRT DLLs, resources and notices are retained. The source DirectML cache link is dereferenced. PE audits covered22 regular App/helper/CRT/CPP/Fixed entries; helper dependencies are verified before execution. Initial manifest validation correctly rejected three vendor LICENSE entries classified as runtime; correcting their notice roles changed no vendor bytes.

Qwen3-ASR0.6B Q6_K is690,417,824 bytes, SHA256 `3b051f108f03c0c91bbe1a3b2c1ee15e3ed51e4caec2a48751b01f2a21441cc3`, with Apache2 LICENSE/NOTICE and immutable [provenance](qwen3-provenance.json). Its actual catalog ID is `handy-computer/Qwen3-ASR-0.6B-gguf/Qwen3-ASR-0.6B-Q6_K.gguf`; no selected-model setting is injected. See separate [quality limits](qwen3-quality.md).

Primary binary/model terms and available dependency notices are included. A conservative951-component Rust/frontend/build/helper inventory retains SPDX/source/notices;35 component root-notice gaps remain transparent, mostly build/dev/workspace entries. This is not public redistribution/license closure.

## Offline install and storage guard

Installed Node22.15/npm installed all three actual local tgzs together, `--offline --ignore-scripts --no-audit --no-fund`, into fresh stable `C:/Users/swche/AppData/Local/notype/local-app`. The installed facade's `--check`, with explicit owned test data, passed complete role/inventory/hash verification. It correctly distinguishes integrity checking from host/launch proof.

The first attempt stopped its own npm tree at the D1GiB guard before prefix extraction/App launch. Installed pacote's `FileFetcher.manifest()` temporarily unpacks local archives while writing compressed cache streams. Only the exact idle failed cache (1,574,554,801 bytes, zero reparse entries) and empty owned prefix were retired; sources/models/history were preserved. A fresh empty C cache retry passed: measured minimum C2,661,081,088 bytes and D2,749,550,592 bytes. After installed inventory verification, that exact idle C cache (7 files,1,025,704,197 bytes) was retired **before**690MB activation. Floors stayed C2GiB/D1GiB; no OS/pagefile/user-model cleanup or floor reduction.

No npm login/publication, bootstrapper, installer or native compilation occurred during install. System networking was not disconnected and packets were not monitored. `npm --offline`, actual local CPU inference and unconfigured text service are not a whole-system zero-network test.

## Installed runtime and history

A pinned external launcher used the same-user/session medium Explorer parent, suspended child assignment to a kill-on-close Job, isolated owned data, six blank shortcuts and `always_on_microphone=false`. No release debug/CDP hooks were added. It did not acquire clipboard contents, record microphone audio, control user apps or use real API configuration.

Two actual production batch CLI runs used the same13.344s synthetic technical-name WAV and installed model on CPU4:

| Mode | ASR ms | Normalization ms | Combined ms | Load ms | Whole wrapper case ms |
| --- | ---: | ---: | ---: | ---: | ---: |
| Traditional Chinese+English | 3,271 | 378 | 3,649 | 1,140 | 41,912 |
| Simplified Chinese+English | 3,122 | 325 | 3,448 | 884 | 10,356 |

Both outputs retained API, Visual Studio Code, GitHub and English, with the expected Chinese script. Whole-wrapper time includes verification/activation/startup, separate from ASR. First launch may take about a minute; prelinked user-trial timing may differ. These two cases do not estimate p95 or certify human speech quality.

Two corrected60s hidden GUI runs, before/after update, passed. App/broker tokens were medium; expected sandboxed renderers were lower. Actual CPP/DirectML/CRT modules resolved to installed app files or Windows inbox paths. `EmbeddedBrowserWebView.dll` and all owned browser images resolved to the exact installed Fixed156 tree/version. No observed module came from developer ORT/transcribe caches. This is observed local closure, not fresh-VM/all-hardware certification.

An earlier fixture requested `--no-tray`, which intentionally forces main visible despite `--start-hidden`; the visibility guard aborted and cleaned it to zero. The corrected fixture enabled tray/removes `--no-tray`; no product change/rebuild. Its failed receipt is retained. The audit also correctly permits the bundled runtime subfolder for the embedded WebView DLL.

The owned SQLite row was created/read through public `SessionService`, then reopened after native GUI termination/restart. Exact ID, timestamp, Unicode text, `API_42`, numbers and negation survived. This proves existing-history preservation, not physical microphone-to-history/copy end-to-end acceptance.

## Update and user entry

An actual audited private facade `.2` tgz updated the same installation `.1`→`.2` offline. App/model pins stayed identical; history ID/time/text and activated model inode/mtime/size stayed unchanged. Another60s hidden GUI reopen passed. This is **facade-only**, not native binary/schema, locked-file or interrupted-update acceptance.

All five owned runtime Jobs, including the failed no-tray fixture, ended with zero processes. No automated App remained when user entry was prepared. Actual user-facing files:

- `D:/git/notype-local-artifacts/Start-notype.cmd`, Traditional Chinese, SHA256 `b644d7108146a3aa04d7acf0db002cc0d8e62120bbae7239eb12cc88c8e42fba`. It invokes exact quoted installed Node/facade paths, with no UAC/de-elevation/PowerShell-policy step.
- Adjacent `README.md`, `安裝說明.md`, `SHA256SUMS.txt` and `release-pins.json`.
- Permanent explicit trial data `C:/Users/swche/AppData/Local/notype/local-trial`. At preparation it had only the verified regular same-volume model hardlink, no settings, so normal default shortcuts/microphone-off apply. The blank-binding automated profile is separate. **Never retire this user directory as cache/QA data.** No existing `%APPDATA%/io.github.swchen44.notype` settings/history were read or overwritten.

The user App was not automatically launched. The guide covers installed-Qwen onboarding, RightAlt start/stop, history「原稿」→「複製原稿」, missing text-service recovery and X-to-tray/explicit tray quit. Manual user trial is separate evidence.

Detailed receipts are under `%LOCALAPPDATA%/notype-study/build-cache/local-package-pilot-db9qAw/proof/`: tar/install/failed-attempt audits, installed inventory, `local-runtime-audit.json`, facade update, Job cleanup and history fixture. Reversible machine-specific staging/probe scripts remain outside Git, not a portable CI release builder.

Remaining gates include clean-machine/CPU closure, native/locked/interrupted updates, physical microphone/copy, human quality/p95, company network/API/Proxy and public publication. Source CI and synthetic fixtures do not substitute for them.
