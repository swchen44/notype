# 測試規範

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
