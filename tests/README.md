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

目前測試 HTTP、SQLite、文字流程及輸入 callback 邊界；callback 並非真實 Windows 輸入框。STT、公司 API、五個 App 與完整 portable 啟動仍須獨立驗收。

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
