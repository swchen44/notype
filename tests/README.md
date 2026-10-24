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
- 真實 CPU／Edge smoke：`npm.cmd run test:runtime`，需先準備固定模型、CLI、WAV 與 Edge；見 [runtime receipt](../docs/verification/windows11-preflight.md)。缺少必要檔案會失敗，不會 skip。

Deterministic tests 測試 HTTP、SQLite、文字流程與 callback 邊界。獨立 runtime smoke 使用真實 CPU STT 與全新 headless Edge 文字框；它不證明原生 Windows 貼上、公司 API、五個 App 或完整 portable 啟動。

正式共用核心使用 Rust，可獨立執行 `cargo test --locked --manifest-path crates/notype-core/Cargo.toml`，包含 public session、真實 SQLite／HTTP 與文件範例；這個命令不需要 Tauri 或 Windows SDK。根目錄 `Shared core contracts` CI 在 Linux 執行同一套核心及既有 Node deterministic tests；是否通過以該次 workflow run 為準，不代表 macOS／Linux 桌面 App 已交付。

## 執行原則

- 每個產品行為隨實作加入可執行測試，先驗證失敗，再確認修正通過；驗證公開行為，不鎖死內部呼叫順序或複製實作。
- unit tests 與 integration tests 應有可分開執行的命令。框架與命令在 Windows 實作選定 stack 後，於本檔及對應套件設定中補上。
- deterministic tests 使用合成／非機密 fixture、本機測試 HTTP server 或測試替身。公司真實 LLM、實際模型及 Windows App 驗收另有明確環境條件。
- 缺少 Windows、模型、runtime 或公司網路時，記錄 `unavailable`／`not-performed`，不能以 skip 或 zero tests 代表完成驗收。
- 真實工作錄音、逐字稿、API key、token 與內網設定留在本機。可追蹤 fixture 使用非機密資料，模型與大型音訊依 manifest 另外取得。

## 回報

每次變更列出執行命令、commit／版本、通過／失敗／未執行數量與原因。unit、integration、STT 品質、效能、Windows UI、公司 API 分開回報。

無 installer、僅 Windows amd64 的限制也適用於 integration 測試的部署程序。

詳見 [unit tests](unittest/README.md) 與 [integration tests](integration/README.md)。
