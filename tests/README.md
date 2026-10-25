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
- npm portable foundation：`node --test tests/unittest/portable-package.test.mjs tests/integration/portable-cli.test.mjs tests/integration/portable-tarball.test.mjs`。使用合成檔案與真正本機 npm tarball；不啟動 App、不含模型／runtime 大檔，詳見 [封裝契約](../packages/ASSEMBLY.md)。
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

- 每個產品行為隨實作加入可執行測試，先驗證失敗，再確認修正通過；驗證公開行為，不鎖死內部呼叫順序或複製實作。
- unit tests 與 integration tests 應有可分開執行的命令。框架與命令在 Windows 實作選定 stack 後，於本檔及對應套件設定中補上。
- deterministic tests 使用合成／非機密 fixture、本機測試 HTTP server 或測試替身。公司真實 LLM、實際模型及 Windows App 驗收另有明確環境條件。
- 缺少 Windows、模型、runtime 或公司網路時，記錄 `unavailable`／`not-performed`，不能以 skip 或 zero tests 代表完成驗收。
- 真實工作錄音、逐字稿、API key、token 與內網設定留在本機。可追蹤 fixture 使用非機密資料，模型與大型音訊依 manifest 另外取得。

## 回報

每次變更列出執行命令、commit／版本、通過／失敗／未執行數量與原因。unit、integration、STT 品質、效能、Windows UI、公司 API 分開回報。

無 installer、僅 Windows amd64 的限制也適用於 integration 測試的部署程序。

詳見 [unit tests](unittest/README.md) 與 [integration tests](integration/README.md)。
