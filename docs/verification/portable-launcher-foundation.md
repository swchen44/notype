# #14 portable launcher foundation — partial local evidence

2026-10-25 Asia/Taipei. Branch `ticket/14-portable-package`, based on integration `a774c5ddb174ffb9e21fd440cba90660b95fecb1`. This is the independent Node/package contract slice. **#14 remains incomplete.** No native Rust/Cargo/profile/portable initialization code was changed; no native build, cache cleanup, model/runtime large copy, application launch, recording, company request or publication occurred.

## Result

The separate private `packages/notype`, `packages/notype-win32-x64` and `packages/notype-model` roles establish the small facade/application-specific runtime/model split. Root diagnostic package metadata is unchanged. All role descriptors remain unassembled; no shipping model, publisher identity, exact dependency version or CPU baseline has been invented.

Public inspection verifies exact pinned manifests, per-file hashes/lengths, source revisions/license references/notices, complete declared inventory, regular files and Windows path constraints. Missing fixed-runtime files refuse before any app work, addressing the observed empty-runtime host-alive gap in the earlier probe. The new tests operate on synthetic payload bytes and **do not establish actual native dependency closure**.

Default data resolves to the existing `%APPDATA%/io.github.swchen44.notype` (current Tauri identifier and Windows roaming app data); an absolute local override stays at its existing location. A version-change test preserves the same root and a history byte sentinel. This proves resolution/nonmutation, **not** actual SQLite migration, locked-file or interrupted npm update acceptance. The plan supplies an external `webview2` UDF and exact verified browser folder; inherited WebView overrides are removed.

`--check` performs artifact inspection only. Default CLI first refuses missing/corrupt/unassembled inputs; valid inputs additionally require medium integrity. The bounded Windows inherited-token probe ran in this elevated agent environment and returned `integrity: unsupported`, as expected. An actual ordinary-user launcher acceptance is still pending. The default CLI deliberately refuses with `launch-not-integrated` even after successful package/host checks until local model activation and final native launch are integrated. No fake executable was spawned to claim native success.

## Red/green evidence

Observed RED failures were followed by focused implementation at the approved public package-plan/CLI boundaries:

- Missing launcher module initially; then a valid package plan resolves exact fixed runtime/external data.
- Missing runtime, same-size model corruption and replacement manifest initially returned a plan; each now refuses with its stable error code.
- Unsupported platform initially reached data work; explicit platform/host guards now refuse.
- Undeclared private file/traversal initially passed; exact inventory and path validation now refuse.
- Private settings explicitly added to a hash-pinned resource list initially passed; private artifact exclusion now refuses.
- Windows `\rooted-without-drive` data initially depended on the current drive; a fully qualified local drive path is now required.
- Actual CLI test initially failed because no executable facade existed; after wiring, missing/corrupt/unassembled packages fail without application launch or environment-value disclosure.

Checks on Windows 11 x64, Node **22.15.0**, npm **11.7.0**:

| Command | Actual result |
| --- | --- |
| `npm.cmd run test:unit` | **20 passed**, including 9 new portable contracts |
| `npm.cmd run test:integration` | **9 passed**, including 3 CLI + 1 actual tarball contract |
| touched files Prettier | Passed |
| `git diff --check` | Passed |

No tests were skipped. On non-Windows hosts the real CLI contracts assert explicit unsupported-platform refusal; fixture plan tests still inspect synthetic Windows payload contracts. No Linux/macOS desktop support is claimed.

The tarball test runs local `npm pack --ignore-scripts --offline --json`, decompresses actual gzip/tar bytes, requires regular files and exact role inventories, checks private/unassembled metadata and absence of lifecycle scripts/extra repo/user files. This is a real tarball inspection of **foundation packages only**. Final native tarball/private-content scanning remains separate.

## Retained reviewable artifacts

Three small local npm tarballs and `receipt.json` are retained outside the checkout at:

`C:/Users/swche/AppData/Local/notype-study/agent-notes/portable-foundation-tarballs-AM6TSO/`

| File | Compressed bytes | Unpacked bytes | Observed SHA256 |
| --- | ---: | ---: | --- |
| notype-0.0.0.tgz | 5,821 | 17,913 | `e80c62e03fae633f8050a4188f8edb9fd67e143b816fbbc4a0218130c2395612` |
| notype-win32-x64-0.0.0.tgz | 724 | 1,113 | `70faa12d9f4fcc88e98769ef79a4d84b4f575509cad9f58e76d1d67856823836` |
| notype-model-0.0.0.tgz | 700 | 1,031 | `769020d80cacd654704765a52d0fede96a14d5d6555bf8eaa4e1d9521c15e145` |

These tiny artifacts deliberately contain no App/DLL/WebView/model. They are not installable complete product artifacts, public registry receipts, or model-quality results.

## Remaining local frontier

[Assembly plan and contracts](../../packages/ASSEMBLY.md) describe final native/model selection, explicit file allowlists, provenance/notices, storage/lease gates and model/data activation. #13 owns current native work and its `rlib`-only adjustment. Final release profile/build and immutable payload assembly follow its merge and the #12 proof; current native outputs and caches were left untouched.

Still required: actual required CPU baseline/capability adapter if higher than generic x86-64; release imports/regular DLL closure; full fixed runtime and licensing/notices preservation; accepted pinned model and tokenizer/config; atomic local model activation into current native layout; shell-free native launch; owned normal-user/offline cold launch with actual fixed runtime images and real inference; restart/migration, interruption and file-lock updates at the same data root; final artifact inventory/private-content scan and measured storage.

The user currently has no company API/network access and asked to finish the local version first. The existing configurable OpenAI-compatible adapter is unchanged. Actual company API/Proxy and public npm publication remain unperformed; no credentials/login/key were requested or used.
