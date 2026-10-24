# 公司內網 Windows 語音輸入 App 研究

查核日期：2026-10-24。

**研究與Q1～Q15決策已收斂，先讀 [Windows MVP 定稿](10-confirmed-windows-mvp.md)。** 產品為Windows amd64 portable App，只用公開通用npm包配送，公司設定留本機；本機STT→公司文字LLM→原輸入目標交付，並保留可搜尋的長期文字歷史。優先評估修改Handy，效能與runtime仍須在公司Windows驗收。

## 已確認的需求

- 公司電腦是 Windows，使用內部網路；現有 LLM API 只有文字輸入。
- 希望功能類似 Typeless，方便同事安裝使用，偏好降低執行時相依與額外下載。
- 公司 pip／npm 有 Proxy；可將自製套件上傳到 Proxy 能取得的來源。
- 尚未指定 software stack；重視效能與可封裝性。
- 只做 amd64／x64；禁止 installer、setup／MSI 及 runtime bootstrapper。
- 產品只做公開通用 npm 配送，公司設定本機保存；需要本機長期文字歷史及搜尋／複製／刪除，錄音只供當次處理。
- Terminal 首版以自然語言 prompt 為主；所有 App 手動 Enter。CPU baseline，短錄音 p95≤5秒是待驗證目標。
- 先供本人使用；輸出繁中夾英文忠實整理，LLM 失敗停下待選。
- 首批目標：瀏覽器文字框、VS Code、Teams、Outlook、Windows Terminal。
- GitHub repo 已建立為 private，保存研究、Git history 與 issues；Q1～Q15 訪談決策已收斂。

完整的已確認決策／驗證邊界以 [決策紀錄](08-decision-log.md) 為準。公司 API 的 wire format、認證／TLS、實際 Windows 硬體、Rust build 工具取得方式、npm publisher 權限與 Proxy 技術條件仍未驗證。本次沒有呼叫公司端點或發布套件。

## 研究文件

| 文件 | 內容 |
| --- | --- |
| [01：Typeless macOS 功能與邏輯](01-typeless-macos-analysis.md) | 使用 REA 分析已安裝的 Typeless 2.8.3；功能、Electron／native 組成、錄音／上下文／雲端處理／送字流程 |
| [02：GitHub 開源專案比較](02-open-source-alternatives.md) | 8 個候選，固定 commit、stable release、Windows 發布形式、STT／內部 LLM 接點、外部依賴 |
| [03：Windows 內網架構與安裝](03-windows-intranet-architecture.md) | 本機 STT、公司 API adapter、stack 比較、模型／runtime 打包及 Windows 驗收順序 |
| [04：方法、證據與驗證邊界](04-method-and-evidence.md) | REA 參數、hash 差異、partial coverage、重現工具、已完成與未執行項目 |
| [05：效能與套件配送](05-performance-and-package-delivery.md) | Rust／Python native engine 的差異；npm／wheel 內含 exe、DLL、模型的 Proxy 配送方案 |
| [06：Handy source 與 payload](06-source-feasibility-audit.md) | 固定版本 portable／模型／LLM／paste 路徑、Windows x64 發布物靜態檢查 |
| [07：Windows 接手](07-windows-handoff.md) | 無 installer 限制下的 preflight、實作／驗收順序與接手 prompt |
| [08：決策紀錄](08-decision-log.md) | Q1～Q15 已確認選項、設計樹與驗證邊界 |
| [09：文字歷史 source audit](09-history-source-audit.md) | WAV／文字保存耦合、UI／重試／retention 與 npm 更新接縫 |
| [10：Windows MVP 定稿](10-confirmed-windows-mvp.md) | 最終行為、必改 source 接縫、測試與 Windows 接手順序 |
| [工作紀錄](HISTORY.md) | 工作里程碑，精確變更以 Git commits 為準 |

## 優先候選

| 候選 | 為什麼先看 | 需接受的前提 |
| --- | --- | --- |
| [Handy](https://github.com/cjpais/Handy) | Rust／Tauri、本機 STT、自訂文字 LLM provider；先行驗證最直接 | 模型、Windows runtime／WebView2、公司 API 相容性需要驗收 |
| [Sotto](https://github.com/stofll/Sotto) | 有 Windows portable ZIP、本機 STT、自訂 LLM | WebView2、模型與 Credential Manager；portable 不等於所有檔案都零相依 |
| [OpenWhispr](https://github.com/OpenWhispr/openwhispr) | Electron／TypeScript 的完整桌面語音產品參考 | 須管理 native helpers、模型及較廣功能範圍 |
| [OpenTypeless](https://github.com/tover0314-w/opentypeless) | 風格、選取文字改寫、Ask、翻譯等功能接近 Typeless | Windows local STT 需另外準備 Whisper-compatible HTTP 服務與模型 |

這是第一方文件及原始碼研究後的建議，詳見 02 的來源；沒有在公司 Windows 實測。其餘候選是 Whispering／Epicenter、Voicetypr、VoiceInk、OmniDictate Windows GUI。

## 本次交付的證據

- Typeless `app.asar` SHA-256、89 個來源與 6 個 native libraries 的身分紀錄。
- REA summary、初次 integrity error、17 筆原始 byte range／有限 literal substitution 對照摘錄。
- 8 個 GitHub repository 的 metadata／發布資產描述，以及 70 個選讀 source snapshot 的 hash；source snapshot 計數含不同 ref／scope。
- Handy history 的13個固定source成員hash，及source／Windows payload靜態追查。
- 兩支只用 Python 標準函式庫的研究工具，用來讀取選定 ASAR 來源與靜態替換字串查表。

Typeless 的兩個 unpacked native 模組有宣告／實際 hash 差異；REA graph 為 partial。本機套件不能揭露完整伺服器模型與 prompt。已另做 Handy x64 發布物 hash／靜態解包研究，沒有執行其 installer。公司版交付不使用 installer。本次未執行聽寫、Windows runtime、效能測試、公司 API 呼叫或 npm／pip package upload；這些邊界均保留在各文件。
