# 測試規範

## GitHub release draft (#36, #39)

`node --test tests/unittest/release-controller.test.mjs tests/integration/release-controller.test.mjs`
checks the real draft controller with synthetic owned files and a GitHub adapter
double. It covers safe version/main input, tag/release collision, asset checksum,
remote digest mismatch, bounded client timeout, stderr redaction, post-create tag
fences and an incomplete draft after upload failure. These tests
do not invoke GitHub, compile a native App or establish Windows host acceptance;
the manually dispatched Windows action and actual draft verification are separate.
Release assembly tests also exercise actual Git source/lock gates in an owned Git
repository, native provenance and CRT checks, unchanged-output sentinel, a pinned
license fixture, and malformed archive rejection under Python -O. These fixtures
do not prove current GitHub runner CRT compatibility or legal redistribution rights.
`release-smoke.test.mjs` tests the isolated data environment, a real owned child
with a bounded timeout, and the requirement that the requested EXE belongs to the
freshly validated extraction. The action separately runs `--list-models` on that
actual extracted EXE; this does not exercise microphone input or recognition.

## Dictation fluency (#35)

`tests/unittest/dictation_prompt.rs` verifies restoring the dictation prompt
requests meaningful cleanup while leaving the customized translation prompt
untouched. `tests/integration/desktop_provider.rs` exercises the real workflow
with an owned loopback provider: it checks the default guidance reaches the
provider and the returned text is retained alongside the original transcript.
The fixture does not prove model quality. A separate synthetic DeepSeek test
showed filler removal and preserved the meeting time and instruction; real
speech, providers and model outputs can vary.

## YAML settings transfer (#34)

The source now connects Advanced settings to native file selection, a redacted
preview, confirmed restart and startup recovery. `settings-journal` tests the
transaction/space/admission boundary; `settings-journal-files` tests actual owned
files, locks, interrupted write prefixes and external-edit rejection.
`settings-transfer` round-trips Chinese YAML, rejects redacted/unknown inputs,
reopens actual provider/App/root/diagnostics stores and preserves SQLite/model
sentinels. It also distinguishes known App autosave from unrelated disk changes.
`settings-transfer.ui.mjs` mounts the actual component with IPC doubles; six cases
cover default key exclusion, plaintext notice, cancel/stale, audio/model conflicts,
three locales and restart. These mocks do not establish native restart acceptance.

```powershell
cargo test --release --locked --manifest-path tests/integration/desktop-contracts/Cargo.toml --test settings-yaml --test settings-journal --test settings-journal-files --test settings-transfer
cargo clippy --release --locked --manifest-path tests/integration/desktop-contracts/Cargo.toml --test settings-yaml --test settings-journal --test settings-journal-files --test settings-transfer -- -D warnings
node --test --test-concurrency=1 tests/integration/settings-transfer.ui.mjs
```

`tests/unittest/settings_transfer_native.rs` is compiled by the actual native
library: `cargo test --release --manifest-path desktop/src-tauri/Cargo.toml --lib
--features tauri/custom-protocol settings_transfer_native::tests`. It checks real
AppSettings default import and rejects invalid typed/ranged/product settings.
The regression first failed on retained legacy shortcut fields, then passed after
restricting physical-key parsing to the four active scene/stop bindings.

`settings-yaml` exercises the bounded production codec source with owned synthetic
values. It covers real YAML/Chinese/multiline/numeric round-trip, literal variable
references, duplicate keys (including conversion collisions), zero aliases/anchors,
unsupported tags/include/merge, multidocuments, malformed UTF-8, structure/byte
limits and fixed errors without excerpts. It reads no files or credentials and
does not establish domain import, atomic settings application or a working UI.

```powershell
cargo test --locked --manifest-path tests/integration/desktop-contracts/Cargo.toml --test settings-yaml
cargo clippy --locked --manifest-path tests/integration/desktop-contracts/Cargo.toml --test settings-yaml -- -D warnings
```

The pinned codec is now registered in both native and portable manifests. Six
added registry crates and eleven license files have been audited against crate
checksums; inclusion in a newly verified package is a separate acceptance gate.
The running local.5 executable does not contain this source.

`provider-config` additionally verifies backend-only archive key exclusion/inclusion,
fixed home-variable references and prepared whole-provider replacement. Omitted
keys preserve only the same ID/canonical route/authentication binding; explicit
empty/included key maps clear/replace that exported state, without live writes.
`provider-config-store` uses actual owned FileStorage and SQLite with Chinese YAML
round-trip, prepared target reopen/NeverHome credential resolution, and unchanged
source settings/history bytes. These are foundations, not full multi-store import.

## Per-server tuning and settings tabs (#34)

`provider-config` verifies scene/server independence, None, frozen snapshots,
UTF-8 byte limits, composed prompt overflow and storage-failure rollback.
`provider-config-store` reopens the actual file store and sends three synthetic
scenes through the production workflow to owned loopback HTTP, inspecting the
effective system message. No home credential or public provider is used.

```powershell
cargo test --locked --manifest-path tests/integration/desktop-contracts/Cargo.toml --test provider-config --test provider-config-store
node --test --test-concurrency=1 tests/integration/provider-connections.ui.mjs tests/integration/api-options.ui.mjs tests/integration/footer-status.ui.mjs
```

The rendered tests use the actual components and IPC doubles: tab switching keeps
unsaved key/model/shared/tuning drafts; saving or clearing one scene preserves the
others. They also exercise translated narrow layouts and keyboard tab navigation.
Native App compilation/UI and live provider checks are reported separately.

## Session delivery and usability (#31)

The confirmed session/settings seams cover AI off in all three scenarios, automatic
raw fallback after API error/timeout, cancellation/no-resend, primary-text history
search, dictionary CRUD/UTF-8 CSV and independent connection accounting. See the
[specification](https://github.com/swchen44/notype/issues/31) and
[verification receipt](../docs/verification/usability.md).

```powershell
cargo test --locked --manifest-path crates/notype-core/Cargo.toml
cargo test --locked --manifest-path tests/integration/desktop-contracts/Cargo.toml --test desktop-workflow --test desktop-provider --test provider-config --test provider-config-store --test connection-activity --test dictionary-words --test native-target-policy --test native-delivery
node --test tests/unittest/dictionary.ui.mjs tests/unittest/history-presentation.ui.mjs
node --test --test-concurrency=1 tests/integration/usability.ui.mjs tests/integration/history-metrics.ui.mjs tests/integration/provider-connections.ui.mjs
```

Workflow/provider tests exercise public session behavior with synthetic data,
loopback HTTP and real owned SQLite. Connection activity uses the production
observer, SQLite reopen/reset/late receipts and actual loopback probe transport;
no provider key or paid endpoint is used. Rendered browser fixtures mock IPC,
exercise actual components, and verify downloaded CSV bytes/BOM and import results.
The separate Windows Notepad++ probe requires an owned PID and a synthetic document;
it is not part of portable CI and cannot establish microphone/full-App acceptance.

## Offline defaults and complete ZIP contracts (#29)

Approved [P29-v1](https://github.com/swchen44/notype/issues/29#issuecomment-6405357391),
[Astra approval](https://github.com/swchen44/notype/issues/29#issuecomment-6405428255).
The production ConnectionStore import, original settings-store envelope and
RootService coordinator run without Tauri/speech engines:

```powershell
cargo test --locked --manifest-path tests/integration/desktop-contracts/Cargo.toml --test provider-bootstrap
cargo test --locked --manifest-path tests/integration/desktop-contracts/Cargo.toml --test offline-bootstrap-store
cargo test --locked --manifest-path tests/integration/desktop-contracts/Cargo.toml --test bootstrap-preflight --test runtime-startup
node --test tests/integration/bootstrap-recovery.ui.mjs
node --test tests/unittest/offline_bundle.test.mjs tests/integration/offline-bundle.test.mjs
```

The first Rust target is unit behavior; the second uses actual owned domain files,
tiny trusted artifacts, persistence failure/reopen, partial-domain retry and
unchanged existing credential/settings bytes. The rendered recovery test mounts
the actual component/hook and mocks only IPC. ZIP tests use real Python-standard-
library tiny ZIP bytes/extraction, synthetic vendor inventory, the public facade
and owned sentinel/runtime doubles. Windows CMD/empty-PATH evidence is separate
from Linux contracts. Full ZIP verification checks all declared model assets;
normal launch can recover from invalid/missing optional model assets while
program/Node/runtime/bootstrap gates remain mandatory. None of these tests reads
user settings/home.env/keys/weights or establishes native App/clean-host/company
API/physical capture acceptance. Actual commands/counts/source pins belong in the
verification receipt after execution.

The [approved Simplified Chinese recovery correction](https://github.com/swchen44/notype/issues/29#issuecomment-6411382409)
([Astra0](https://github.com/swchen44/notype/issues/29#issuecomment-6411399601))
adds direct zh-CN resource lookup assertions to
`tests/unittest/model-presentation.ui.mjs` and a zh-CN case to the existing
rendered recovery test. They check all four labels without fallback, the status
panel, disabled retry progress and explicit IPC acknowledgment. Run the unit with
`node --test tests/unittest/model-presentation.ui.mjs`; the rendered command is
listed above. See the [runtime RED/GREEN receipt](../docs/verification/bootstrap-zhcn.md)
for exact source pins and counts. These checks establish UI source behavior only.

The approved [P29 review fixes](https://github.com/swchen44/notype/issues/29#issuecomment-6408412474)
([Astra0](https://github.com/swchen44/notype/issues/29#issuecomment-6408691967)) add
the actual read-only native placement policy with owned-tree/sentinel inventories,
including rejected missing descendants, original alias ancestry and injected Known
Folder inputs. The production runtime factory is exercised with owned audio/worker
ports: repeated audio failure starts zero workers, successful retry shares one
router and one owned worker, and duplicate/in-flight retries cannot publish twice.
Existing SQLite history and applied real domain stores survive these failures.
The actual History UI Copy Original/Result seam uses IPC doubles during incomplete
setup; no real clipboard or speech device is touched. These are source contracts;
Windows native compilation/launch remains separately reported.

使用者已指定以下目錄，後續實作沿用此結構：

```text
tests/
  unittest/       # 模組的外部行為與可重現失敗分支
  integration/    # 模組接合、Windows、API 與 portable payload
```

文字流程核心使用 Node 22.15+ 內建 test runner 與 SQLite。Windows PowerShell 使用 `npm.cmd`，不需要變更 execution policy。

- unit：`npm.cmd run test:unit`
- integration：`npm.cmd run test:integration`
- 安全診斷報告：`node --test tests/unittest/diagnostic-issue.test.mjs`；Rust 公開報告與實際檔案匯出邊界使用 `rustc --edition=2021 -A dead_code --test tests/unittest/desktop_diagnostics.rs -o "$env:TEMP/notype-diagnostics-unit.exe"`、`rustc --edition=2021 -A dead_code --test tests/integration/desktop_diagnostics.rs -o "$env:TEMP/notype-diagnostics-export.exe"`，各編譯成功後執行對應 exe。亦註冊為 desktop-contracts 的 `desktop-diagnostics`／`diagnostics-export` targets；只讀寫自有暫存 fixture，不啟動 App。驗證與操作說明見 [診斷 receipt](../docs/verification/diagnostics.md)。
- npm portable foundation：`node --test tests/unittest/portable-package.test.mjs tests/integration/portable-cli.test.mjs tests/integration/portable-tarball.test.mjs`。使用合成檔案與真正本機 npm tarball；不啟動 App、不含模型／runtime 大檔，詳見 [封裝契約](../packages/ASSEMBLY.md)。
- 保留模型的 npm 交付 gate：`node --test tests/integration/portable-retained-model-install.test.mjs`。真正 tiny npm tarballs／fresh cache／不可用 model tgz，驗證兩個新角色可保留同版模型 metadata/lock；版本不符必須失敗。只在 npm 結束後才補齊 tiny weight，再以公開完整性檢查確認。沒有真實 App／模型／使用者 prefix 操作，見 [Section A receipt](../docs/verification/retained-model-install.md)。
- local model activation：`node --test tests/integration/portable-model.test.mjs`，實際檔案、取消、並行與程序中斷後重啟；不讀寫使用者設定或選取模型。
- native portable preflight：`cargo test --locked --manifest-path crates/notype-platform/Cargo.toml`。獨立 Rust adapter/probe，不需 Tauri、ORT／C++；實際 App Release 啟動另做驗收。
- 真實 CPU／Edge smoke：`npm.cmd run test:runtime`，需先準備固定模型、CLI、WAV 與 Edge；見 [runtime receipt](../docs/verification/windows11-preflight.md)。缺少必要檔案會失敗，不會 skip。

Deterministic tests 測試 HTTP、SQLite、文字流程與 callback 邊界。獨立 runtime smoke 使用真實 CPU STT 與全新 headless Edge 文字框；它不證明原生 Windows 貼上、公司 API、五個 App 或完整 portable 啟動。

正式共用核心使用 Rust，可獨立執行 `cargo test --locked --manifest-path crates/notype-core/Cargo.toml`，包含 public session、真實 SQLite／HTTP 與文件範例；這個命令不需要 Tauri 或 Windows SDK。根目錄 `Shared core contracts` CI 在 Linux 執行同一套核心及既有 Node deterministic tests；是否通過以該次 workflow run 為準，不代表 macOS／Linux 桌面 App 已交付。

## 執行原則

正式 desktop 的平台中立 orchestration／OpenAI-compatible adapter 可用 test-only harness 驗證，不需要編譯 Tauri／C++：

```powershell
cargo test --locked --manifest-path tests/integration/desktop-contracts/Cargo.toml --test desktop-workflow
cargo test --locked --manifest-path tests/integration/desktop-contracts/Cargo.toml --test desktop-provider
cargo test --locked --manifest-path tests/integration/desktop-contracts/Cargo.toml --test physical-gestures --test native-delivery
```

第一個 target 為 `tests/unittest/desktop_workflow.rs`，第二個為 `tests/integration/desktop_provider.rs`；皆透過公開 session／真實 loopback HTTP／SQLite 邊界，不代替原生 UI、STT 或公司品質驗收。三場景與回譯的新原生驗證見 [receipt](../docs/verification/scenarios-prompts.md)。

`Shared core contracts` CI 也執行以上兩個 target，並在 workflow、文字 provider、辨識正規化或測試 harness 變動時觸發；這部分在 Linux 驗證平台中立的桌面流程，不載入 Windows adapter。

實體快捷鍵另有 std-only `physical-gestures` target；`native-delivery` 在 Windows 驗證未知 token 與不符的 QA fixture 身分不能成為目前前景目標，在其他平台驗證 unavailable。CI 也執行這兩個 target。`desktop-shortcuts.runtime.mjs`、`desktop-input-scope.runtime.mjs` 與 `desktop-input.runtime.mjs` 是需明確 `NOTYPE_DESKTOP_EXE` 的獨立 Windows 原生驗證。scope 測試只開始後立即取消，不 finish 或交付；人工 input 測試每次捕捉都必須確認自有 fixture 真正取得前景，並提供 PID／建立身分／完整執行檔限制，缺少條件會失敗，不會假稱通過或 skip。命令、三次歷史人工結果及實際限制見 [guarded input receipt](../docs/verification/guarded-input.md)。

修改過的 tray 語系回歸位於 `tests/unittest/desktop_tray_locales.rs`，由真實 native lib 模組載入，使用 build.rs 生成的語系。從 `desktop/` 執行 `cargo test --release --manifest-path src-tauri/Cargo.toml --lib --features tauri/custom-protocol tray`；只跑合成資料的 library tests，不啟動 App 或使用剪貼簿。原稿、處理結果與失敗後原稿選擇由 `tests/unittest/core_session.rs` 的公開 session 介面驗證；不直接測試 tray 私有文字選擇函式。原生依賴／快取條件與完整 review 修正證據見 [receipt](../docs/verification/local-review-fixes.md)。

- 每個產品行為隨實作加入可執行測試，先驗證失敗，再確認修正通過；驗證公開行為，不鎖死內部呼叫順序或複製實作。
- unit tests 與 integration tests 應有可分開執行的命令。框架與命令在 Windows 實作選定 stack 後，於本檔及對應套件設定中補上。
- deterministic tests 使用合成／非機密 fixture、本機測試 HTTP server 或測試替身。公司真實 LLM、實際模型及 Windows App 驗收另有明確環境條件。
- 缺少 Windows、模型、runtime 或公司網路時，記錄 `unavailable`／`not-performed`，不能以 skip 或 zero tests 代表完成驗收。
- 真實工作錄音、逐字稿、API key、token 與內網設定留在本機。可追蹤 fixture 使用非機密資料，模型與大型音訊依 manifest 另外取得。

## 回報

每次變更列出執行命令、commit／版本、通過／失敗／未執行數量與原因。unit、integration、STT 品質、效能、Windows UI、公司 API 分開回報。

無 installer、僅 Windows amd64 的限制也適用於 integration 測試的部署程序。

詳見 [unit tests](unittest/README.md) 與 [integration tests](integration/README.md)。

## Desktop UI 契約

沿用已確認的公開 UI／模型呈現 seam。前端測試使用 `*.ui.mjs`，與不依賴 desktop 套件／瀏覽器的 portable suite 分開執行：

- `npm.cmd run test:ui:unit`：真正 i18next instance 與公開模型名稱／說明 presenter；驗證缺少翻譯時仍呈現 metadata，而非 missing-key 錯誤文字。
- `npm.cmd run test:ui:integration`：真實 React／i18next／Vite 加上全新 headless 瀏覽器，透過呈現的 UI 操作驗證完整語言、相對評分排序、下載狀態、進階資訊與窄視窗。只在 Tauri 公開 IPC 邊界提供合成資料，不啟動原生 App、不觸碰麥克風／剪貼簿／使用者設定。

需要已準備好的 `desktop/node_modules`。整合測試在 Windows 預設使用已安裝的 Edge；可用 `NOTYPE_TEST_BROWSER` 指定瀏覽器路徑，其他平台需 Playwright Chromium。缺少必要依賴或瀏覽器會失敗，不會 skip。畫面檢查產物寫入忽略追蹤的 `.local/model-explorer-*.png`；內容是明確命名的合成 metadata fixture，不能視為真實 STT 或目前 App 狀態。

此 UI suite 沒有加入 `Shared core contracts` 的 portable CI；該 CI 只安裝根目錄套件，不能宣稱已驗證需要 desktop 依賴與瀏覽器的 UI。驗證紀錄見 [模型介面 receipt](../docs/verification/model-interface.md)。

## Structured diagnostics control and model storage contracts

[P20-v3](https://github.com/swchen44/notype/issues/20#issuecomment-6398232178) 的 [GPT-6-Astra 核准](https://github.com/swchen44/notype/issues/20#issuecomment-6398249625) 涵蓋以下 production seams：

- `desktop-diagnostics`：既有報告白名單、bounded events 與私密 sentinel。
- `diagnostic-privacy`：固定欄位 sanitizer，長值 cutoff 12／頭尾各 2 字元／中段 8 個星號；短值、Unicode、Bearer／JWT／URL／編碼／模糊來源全遮罩。只測合成資料；正式 report 白名單仍不接受憑證欄位。
- `diagnostic-service`：初始未初始化即 OFF、既有 persisted OFF、保存 OFF 失敗仍停止收集，Clear 保留偏好狀態。
- `diagnostics-export`：透過 production service 保存不可變預覽，後續事件不混入，失敗僅回固定 code。
- `diagnostic-control`：實際專用 JSON 偏好檔與原子寫檔；default ON、損壞／讀取失敗 OFF、保存失敗與重啟、快速切換、Clear／匯出兩種屏障順序、目的檔 sentinel、實際 rename 失敗清理與公開 preview/export 隱私邊界。
- `model-storage`：正式 Manager 使用的 `ModelPathAccess` 方法與真實自有檔案；display 不清 partial／不呼叫 reconciliation，activation 正向對照會清理或改 catalog，保留 selection。不是以無關 JSON 檔未變當成 Manager 證據。

以上 targets 均註冊在 `tests/integration/desktop-contracts/Cargo.toml`，由 portable CI 執行，不載入 Tauri／音訊後端。可用 `rustc --edition=2021 -A dead_code --test <root-test-path> -o <task-owned-test.exe>` 分開執行；`diagnostic-control` 另需既有 serde_json 依賴（獨立 rustc 的確切唯讀 rlib 命令見 [receipt](../docs/verification/followup-review-fixes.md)）。不要把 root native target/cache 當作平行任務的測試輸出位置。

`npm.cmd run test:ui:integration` 另以真實 React／i18next 操作 DiagnosticsSupport 與 ModelCard，僅替換公開 IPC/opener。涵蓋 checkbox 的初始 disabled、鍵盤操作、OFF unsaved／重試、取消／過期／保存失敗、Clear 後晚到回覆與舊狀態、opener 拒絕。UI suite 使用 `--test-concurrency=1`，避免共用 Vite optimizer 的測試檔互相重建依賴；overlay fixture 仍保留自己的 cache/port。沒有啟動原生 App 或操作使用者 profile。Native 綁定／編譯／實際收音仍另行驗證。

`node --test tests/integration/about-settings.ui.mjs` 可獨立操作正式 AboutSettings／LogLevelSelector，涵蓋 Quit 的 exit(0)／拒絕／翻譯回饋／重試，以及 log-level 初始載入、保存與 refresh 期間 disabled、正確 IPC payload、實際 settings refresh 後的 acknowledged 值、拒絕與重試。只替換公開 Tauri transport，退出成功不會真正關閉任何 App；原生退出與 logger 接受設定仍需另行驗收。完整 UI script 亦會發現此檔。

## Main window lifecycle contract

`tests/unittest/desktop_window_lifecycle.rs` exercises the public `close_to_tray`
platform-policy seam used by the actual native close handler. It verifies that a
failed native tray recovery keeps the window visible, and that successful tray
confirmation precedes hiding. Native tray registration, shell overflow, restore
clicks and explicit Quit still require the separately reported Windows fixture.

```powershell
cargo test --locked --manifest-path tests/integration/desktop-contracts/Cargo.toml --test window-lifecycle
```

The same std-only contract can run without a native Cargo build/cache:
`rustc --edition=2021 --test tests/unittest/desktop_window_lifecycle.rs -o <task-owned-test.exe>`.

## Recording overlay contract (#21)

Approved plan: [P21-v3](https://github.com/swchen44/notype/issues/21#issuecomment-6398232459),
[GPT-6-Astra APPROVED / zero unresolved](https://github.com/swchen44/notype/issues/21#issuecomment-6398249848).

- Unit: `tests/unittest/desktop_overlay_lifecycle.rs` runs the production overlay
  request policy through its public reserve/commit/ready/text boundary, with
  queued work executed after successor or terminal reservations. It is a std-only
  `overlay-lifecycle` target in the existing desktop-contracts harness; it can also
  run with `rustc --edition=2021 --test tests/unittest/desktop_overlay_lifecycle.rs -o <task-owned-test.exe>`.
- Rendered integration: `node --test tests/integration/recording-overlay.ui.mjs`
  mounts the actual React component via Vite in a new headless browser, replacing
  only Tauri event/settings transport and the frontend clock/scheduler. It covers
  digit-only display elapsed, ready ordering, stale events, terminal phases,
  settings completion and listener/timer cleanup. The fixture uses its own local
  port and ignored Vite cache so other UI tasks can run independently.
- The existing `test:ui:integration` script also discovers this test. Native App,
  mic readiness latency, Windows geometry/permissions and audio duration are not
  established by this suite. See [event contract](../docs/desktop-overlay-events.md).

## Capture readiness observation seams (#19)

Approved [P19-latency-v3](https://github.com/swchen44/notype/issues/19#issuecomment-6398258525),
[Astra approval](https://github.com/swchen44/notype/issues/19#issuecomment-6398268939),
[completed P20/P21 dependency gate](https://github.com/swchen44/notype/issues/19#issuecomment-6398707430).

- `tests/unittest/capture_startup.rs`: production `RecorderPort` startup service,
  captured-chunk readiness wrapper, one-shot waiter and queued dispatch. Hardware
  operations and clocks are adapters; the timing/handoff algorithm is production.
- `tests/integration/capture_readiness.rs`: actual P20 service/report and capture
  observation registry; ordinal/epoch/revision correlation, incomplete and
  truncated/capped reports, local frontend vs backend round-trip origins.
- `tests/integration/recording-overlay.ui.mjs`: actual rendered overlay additionally
  verifies independent delivery/commit/settings observations and an effective
  ready display despite a delayed settings Promise. No native App or mic.

The Rust contracts are std-only and may run with `rustc --edition=2021 --test
<root-test.rs> -o <task-owned-test.exe>` and that executable. They are also targets
`capture-startup` / `capture-readiness` in the existing desktop-contracts harness
and Linux CI. CLI/unit legacy recorder handoffs preserve their prior semantics.
See [exact timing/privacy contract](../docs/capture-readiness-timing.md).

The approved [P19 review fixes](https://github.com/swchen44/notype/issues/19#issuecomment-6399305016)
add a 120ms pre-send recorder-wait / 80ms consumer-delay regression at the actual
`ReadyRequest::send` handoff, plus direct `OverlayLifecycle::observe` positive and
rejected-closure cases in the existing `overlay-lifecycle` target. The report/ACK
integration also verifies Settings uses that outer revision gate before dedup.
These are synthetic attribution tests; native hardware delay is not reproduced.

## History metrics, API observations and CSV (#22)

Approved [P22-v1](https://github.com/swchen44/notype/issues/22#issuecomment-6399288240),
[GPT-6-Astra APPROVED / zero unresolved](https://github.com/swchen44/notype/issues/22#issuecomment-6399333180).

- `core-observations` runs the public `mixed_count` function under `zh-en-v1`.
- `core-observation-store` uses public SessionService with owned real SQLite, observed
  calls and actual CSV files. It covers received duration/model snapshots, both counts,
  retries, independent partial/zero usage, exact money by currency, deletion fencing,
  independent export snapshot during concurrent writes, destination preservation,
  cancellation/low space/rename failure and a 1,200-row fixture. CSV is parsed by the
  existing Python 3 standard-library `csv` module, independently of the Rust encoder;
  `NOTYPE_TEST_PYTHON` may select an installed interpreter. No package install is needed.
- `desktop-provider` runs the actual ChatProcessor over loopback HTTP with SessionService.
  It checks effort/limits/defaults, returned model/usage before invalid-output validation,
  retry/back-translation calls, known disabled API, cancellation and owned SQLite faults
  before/after send. SQLite fault setup uses installed Python's standard-library sqlite3.
- `desktop-workflow` verifies source observations, producer versus requested model,
  accepted-stop scheduling and original-only availability through the actual public workflow.
- `history-metrics-dto` checks the pure native IPC projections without Tauri/engines.
- `tests/unittest/recording_metrics.rs` is included by the native recorder module. It
  exercises public `AudioRecorder::stop_observed` and decoder padding over the production
  ring consumer/resampler/VAD with an owned hardware-channel fixture. It opens no device.
  This requires the separately budgeted native library build; portable tests do not prove it.

```powershell
cargo test --locked --manifest-path crates/notype-core/Cargo.toml
cargo test --locked --manifest-path tests/integration/desktop-contracts/Cargo.toml --test desktop-workflow --test desktop-provider --test history-metrics-dto
```

These are deterministic owned fixtures, not native App/microphone or paid API acceptance.
Native compilation, rendered UI and live DeepSeek have separate receipts/gates.

On Unix, `core-persistence` also executes
`last_service_owner_releases_store_while_inherited_pre_exec_child_is_held`: a
bounded owned-child handshake holds inherited descriptors before exec while the
public SessionService verifies live-clone exclusion and final-owner release.
Windows cannot run this cfg-Unix fixture; its named Linux RED and final workflow
gate are recorded in the [owner-release receipt](../docs/verification/history-owner-release.md).

## Independent DeepSeek probe preparation (#23)

Approved [offline addendum](https://github.com/swchen44/notype/issues/23#issuecomment-6399793622)
and [Astra approval / zero unresolved](https://github.com/swchen44/notype/issues/23#issuecomment-6399801208).
The actual reusable helper modules have no App/network/user-key entry point.

- `tests/unittest/deepseek_probe_policy.rs`: pinned complete-wire bound/profile
  and exact eight-slot schedule.
- `tests/unittest/deepseek_probe_round.rs`: public durable admission service,
  storage failure, no refund/reuse, serial history grouping and terminal halt.
- `tests/unittest/deepseek_probe_console.rs`: actual input/RAII algorithm through
  a fake console system port; no real terminal mode or credential operations.
- `tests/integration/deepseek_probe_round.rs`: actual SQLite FULL reservation
  store, owned files/OS lock and child-process termination/reopen; logical
  corruption rejection, actual transition/crash-prefix positive controls and
  compare-commit preserving invalid/current evidence. No HTTP.

The first three run with `rustc --edition=2021 -A dead_code --test <root-test.rs>
-o <task-owned-test.exe>` and the resulting executable; they are in portable CI.
SQLite tests currently link existing Release rlibs read-only, documented in the
[receipt](../docs/verification/deepseek-probe-offline.md). Do not create a target
or run Cargo concurrently with its lease holder. Shared harness registration,
native console adapter and full production/live acceptance remain pending the
complete #22 gate. See [public contract](../docs/deepseek-probe-offline.md).

## Selectable model roots (#28)

Approved [P28-v1](https://github.com/swchen44/notype/issues/28#issuecomment-6405356803)
and [Astra approval / zero unresolved](https://github.com/swchen44/notype/issues/28#issuecomment-6405400881).

- `model-roots-policy` runs the production activity gate and single-owner engine
  policy, including cancellation versus actual worker exit and delayed same-model
  returns/load results/panics after a successor takes ownership.
- `model-roots` runs the actual root authority, atomic preference adapter, bounded
  scanner, SHA-256 and bounded GGUF prober against tiny owned files/directories.
  It covers manual whole-folder move/reinstall/restart, read-only reuse, corrupt
  preferences, storage failure, invalid bytes, legacy provenance, imported
  directories, HF snapshots and cancellation/publication fences. Unix also
  exercises the one permitted relative HF blob link and an escaping link.
- `model-transfer-worker` drops an actual IPC waiter while a Tokio blocking file
  job is held, proving root admission remains busy until owned runtime shutdown
  drains the worker. Its terminal callback also runs before admission release.
- `model-storage` retains existing P20 behavior and checks selected-root queries
  cannot perform legacy cleanup or ID-only reconciliation of a successor.
- `model-roots.ui.mjs` renders the actual store/components/App/onboarding with
  IPC-only mocks. It checks chooser/recovery, explicit Busy retry, provenance,
  availability and generation/request ownership for late actions/events.
- `model-roots-launcher.test.mjs` and existing portable package tests exercise the
  actual schema-2 launcher with tiny owned App/runtime/model package fixtures.
  Base startup reaches the launch seam without copying or requiring ASR weights;
  explicit full inspection remains strict for a declared optional model pin.

These targets are registered in `core-contracts.yml`. Native Tauri compilation,
actual Windows race/permission/App/Explorer behavior and real model inference
remain separate P14 acceptance gates; portable checks do not certify them.

## P25-live-v4 standalone helper

Approved [plan](https://github.com/swchen44/notype/issues/25#issuecomment-6405986527)
and [Astra0](https://github.com/swchen44/notype/issues/25#issuecomment-6406003416).
Tests use owned HTTP/files/SQLite and fake keys, with no actual OS-home reader,
provider/account request, App, microphone or paid inference.

```powershell
cargo test --locked --manifest-path tests/integration/desktop-contracts/Cargo.toml --test provider-probe-policy
cargo test --locked --manifest-path tests/integration/desktop-contracts/Cargo.toml --test provider-probe-runtime
cargo clippy --locked --manifest-path tests/integration/desktop-contracts/Cargo.toml --test provider-probe-policy --test provider-probe-runtime --bin notype-provider-probe -- -D warnings
```

These are hosted CI commands while the local native-cache lease is unavailable.
The policy target uses root `tests/unittest/provider_probe_policy.rs`; runtime
uses root `tests/integration/provider_probe_contracts.rs` and its separate fixture
modules. It exercises the same smoke path as the future ordinary Windows EXE,
including real public core accounting/export and independent CSV readback.
The private Windows workflow adds actual Windows APIs/headless argument checks.
No source test creates a live activation receipt or authorizes a real key read.
See [actual evidence/limits](../docs/verification/provider-probe-v4.md).
