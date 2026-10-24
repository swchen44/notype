# 研究方法、證據與驗證邊界

查核日期：2026-10-24。此資料夾交付產品／source-level 研究、決策紀錄及 Windows 接手說明；沒有交付可執行的公司 App 或 npm／pip package。GitHub private repo 與 issue tracking 為使用者後續明確授權的獨立成果。公司交付已固定 amd64／無 installer。

## 1. 原始問題的涵蓋範圍

| 問題 | 本次狀態 | 證據／剩餘邊界 |
| --- | --- | --- |
| 用 REA 研究 macOS Typeless 功能 | 已完成產品功能盤點與發行套件靜態分析 | 本機 2.8.3、官方功能頁、REA summary；沒有聽寫 runtime 驗收 |
| 解釋 Typeless 的邏輯 | 桌面端流程部分回答 | 找到模式、state machine、上下文、音訊編碼、HTTP／串流相關路徑與輸入 binding；native helper 內部與伺服器仍未知 |
| GitHub 是否有相近開源專案 | 已找到 8 個候選並比較 | 固定 main／stable source、release metadata、endpoint 與安裝形式；沒有執行候選 App |
| 公司只有文字 LLM 時如何做 | 已提出具體架構 | 本機 STT＋文字 API adapter＋送字；公司 endpoint／硬體未提供，沒有整合驗證 |
| 效能是否必須用 Rust | 已分析取捨 | 區分 native engine、外層語言、模型與 API 延遲；無公司 benchmark |
| npm／pip Proxy 如何配送 | 已提出發行結構 | 預編譯 App／DLL／模型放入套件，registry／scope 與發布憑證尚未選定；沒有 upload |

## 2. 本機 Typeless 的實際證據

| 證據檔 | 用途 |
| --- | --- |
| [typeless-artifact-identity.json](evidence/typeless-artifact-identity.json) | bundle version、ASAR 與來源／native libraries 的 bytes、SHA-256、ASAR 原始 offset、兩筆 integrity 差異 |
| [typeless-rea-summary.json](evidence/typeless-rea-summary.json) | REA 6.5.0 身分、parent Evidence ID、選讀範圍、解析統計、partial graph 與限制 |
| [typeless-rea-initial-integrity-error.json](evidence/typeless-rea-initial-integrity-error.json) | strict policy 初次失敗的完整回應 |
| [typeless-static-excerpts.json](evidence/typeless-static-excerpts.json) | T01～T17、source hash、原始 UTF-8 byte range、原文與有限替換後的閱讀對照 |
| [GitHub repository metadata](evidence/github/repository-metadata.json) | 8 個 repo 的 main SHA、可用 stable release、資產名稱／URL／API 宣告 digest；沒有下載資產實算 hash |
| [GitHub source identities](evidence/github/source-file-identities.json) | 70 個選讀 connector source snapshot 的 hash；不是 Git blob object ID，也不是 release binary 身分 |
| [Handy source／payload audit](evidence/github/handy-stable-source-audit.json) | 固定 source 成員、Windows x64 發布物實算 digest／靜態解包檔案身分與提取限制；沒有執行 installer |
| [外部第一方來源索引](evidence/primary-sources.json) | 官方政策、API／打包文件與查核日期 |
| [本地查核 receipt](evidence/verification-receipt.json) | 文件／JSON／連結及研究工具的本地查核結果；不代表產品 runtime 通過 |

沒有將完整 Typeless ASAR、完整反混淆程式碼、原生 binary、第三方模型或上游 source tree 放入此資料夾。來源工作副本在 `/private/tmp`；證據檔保存重取身分與選讀摘錄。

## 3. REA 的實際呼叫

本次使用已連線的 `rea-agents`／REA MCP **6.5.0**；沒有執行 setup、安裝 engine 或修改 MCP 設定。先呼叫 `binary_session({})` 確認沒有既有 active target；ASAR JavaScript 分析走 target-free 工具，沒有開啟 native disassembler session。

初次 `analyze_javascript_application` 用預設 strict integrity policy，遇到 better-sqlite3 unpacked member 的 declared／observed hash 差異而失敗。第二次明確改成 `integrity_policy: "record-and-continue"`，保留差異後繼續：

```json
{
  "input_path": "/Applications/Typeless.app/Contents/Resources/app.asar",
  "format": "asar",
  "detail": "summary",
  "integrity_policy": "record-and-continue",
  "source_paths": [
    "package.json",
    "dist/main/index.js",
    "dist/preload/index.mjs",
    "dist/main/worker/chunkSessionWorker.js",
    "dist/main/worker/opusWorker.js",
    "dist/renderer/static/js/juYOhWy9.mjs"
  ],
  "max_semantic_nodes": 80000,
  "max_semantic_nodes_per_file": 40000,
  "analysis_timeout_ms": 60000
}
```

`analysis_timeout_ms` 是每個來源的限制，沒有當作整體完成時間。parent Evidence：

```text
ev_1c6631fb12d64494f351837ddac8de0927d5144963b5ab894cfc404737fddfa4
```

統計為 905,941 text bytes read、5 個 JS 解析、164,834 visited AST nodes、0 parse failures。application／semantic coverage 都為 partial；未選讀 10,492 個 relevant inventoried files，floating renderer 的部分 semantic expansion 超過 parent result budget。這些不是完整 App 的「測試通過」數字。

保留的 summary view 自己有 Evidence ID，但它只是 parent 的投影。這裡保存的是完整 summary JSON 與選讀摘錄，**沒有匯出完整 parent Evidence bundle**；跨新 REA session 不能保證直接解析這個 retained ID，需重新取得相同 artifact 或使用另行匯出的完整 bundle。

strict 失敗及後續觀察共指向兩個 unpacked native 模組：better-sqlite3、Koffi。其宣告與實算 hash 保存在 artifact identity；原因未判定，未將它們執行或當成可信 native 實作證據。主 JS 的 API binding 名稱不等於已驗證該模組的行為。

## 4. 靜態閱讀與重現

兩支研究工具只用 Python 標準函式庫，本次在 **Python 3.13.2** 實跑；ASAR reader 使用 `hashlib.file_digest`，需 Python 3.11+。它們不會啟動 Typeless、不會執行或 import 目標 JavaScript。

在研究資料夾執行的重現範例，output 必須是新資料夾：

```sh
python3 tools/extract_app_sources.py /Applications/Typeless.app /private/tmp/typeless-research-new
python3 tools/static_literal_decode.py /private/tmp/typeless-research-new
```

第一支讀 ASAR header、列出 package 與 dist 內 JS／MJS／HTML，直接複製選定 bytes，保存 source 身分。它不是通用 ASAR 完整性驗證器，亦不處理所有封裝格式。

第二支只支援此次觀察到的四個 bundle 的特定 literal table／decoder pattern。Python 以受限 AST 算術重現輪轉 checksum，再做 alias／lookup 文字替換。沒有執行 target bootstrap；alias mapping 未做完整 lexical scope 證明，所以替換結果只是閱讀輔助。證據仍以原始 hash 與 byte range 為準。

本次將研究工具重跑於新的 `/private/tmp/notype-reproduce-20261024`：**89 個來源身分全部相同，17 筆證據摘錄與重新擷取的原始 UTF-8 bytes 全部相同。** 四個 string-table 輪轉結果也與初次相同。這驗證研究擷取／閱讀輔助的重現性，沒有驗證 Typeless 功能。

T17 的麥克風呼叫來自另一個 renderer bundle 的直接選讀；該檔未列入上述 REA AST source selection，證據檔已分開標示。

## 5. 網路研究方法

- 功能與資料處理回到 `www.typeless.com` 官方首頁、FAQ、Ask anything、Privacy、Data Controls；沒有把名稱相近的 `typeless.cc` 當成同一官方來源。
- GitHub 候選以 repository metadata、commit、release API、固定 source 為準。main 能力與 stable tag 分開，尤其 Whispering／Epicenter 的 main 與舊 stable 產品範圍不同。
- 第一方 README 的效能、準確率、隱私及無遙測聲明保留為作者描述；設定或 source 分支存在也沒有外推為 Windows runtime acceptance。
- 部分 GitHub 頁面在 web fetch 回報 cache miss，改以 GitHub connector 已讀固定 ref 的 source snapshot 核對。例如 Sotto portable 文件另從已保存的 stable snapshot 閱讀；沒有把 web cache miss 說成 repository 不存在。
- 研究建議、範例 prompt、目錄示意與部署方案均為本研究提出，沒有描述成 Typeless 或上游已實作的公司功能。

## 6. 未執行的驗收

| 項目 | 狀態 |
| --- | --- |
| macOS Typeless 聽寫／選取編輯／翻譯操作 | not-performed |
| native helper 反組譯／實際呼叫 | not-performed |
| 雲端模型、完整 prompt 與 backend pipeline | unavailable |
| Windows 安裝／portable／package 執行 | not-performed |
| 公司 Proxy、TLS、API 認證及 wire format | not-performed |
| STT 準確率、繁中／英文術語與延遲、RAM | not-performed |
| 模型／DLL／WebView2 完整離線包與封包觀測 | not-performed |
| npm／PyPI／公司 registry upload | not-performed |

下一輪的必要成果是公司 Windows 上同一批非機密錄音、同一模型的品質／效能 receipt，以及透過實際 Proxy 取得套件後的冷啟動／內部 API／貼上驗收。具體流程見 03、05。
