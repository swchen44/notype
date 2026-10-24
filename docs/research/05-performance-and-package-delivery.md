# 效能、Rust 與 npm／pip 套件配送

查核日期：2026-10-24。本文件納入使用者後續補充：重視效能、希望減少相依，公司有 pip／npm Proxy，且可把自製套件上傳到 Proxy 能取得的來源。已確認只做 Windows amd64，**不可使用 installer、setup／MSI 或 runtime bootstrapper**。產品現已選定 **npm-only 配送**，不另做 Python wheel。下文的 pip／wheel 是選型比較資料，不是首版交付範圍。尚未建立或上傳正式產品套件。

## 1. 效能重要，是否就要用 Rust？

**Rust 是可行選擇，但不能只由語言決定這個 App 的速度。** 對「錄音結束後辨識，再呼叫文字 LLM」的第一版，等待時間可拆成：

```text
放開熱鍵到結果出現
  = 尚未完成的 STT 時間 + LLM API 時間 + 結果處理與輸入時間
```

若錄音期間已做串流辨識，各階段可以重疊，不能直接用同一公式相加。本次沒有實際 Windows 計時；優先調整模型／量化、模型是否常駐、CPU／GPU 路徑、錄音長度與 API 延遲，是依工作分解提出的建議。

Python 可呼叫原生推論引擎。第一方 [faster-whisper](https://github.com/SYSTRAN/faster-whisper) 用 CTranslate2 執行模型；因此 Python App 不代表推論迴圈都在 Python interpreter 中跑。比較時要固定模型、精度、執行緒、硬體、音訊與輸出品質。該 repo 的 benchmark 並非本次公司環境量測，也不能直接套用到 Handy／Rust。

| 目標 | Rust 能幫到什麼 | 還要處理什麼 |
| --- | --- | --- |
| 常駐 App 的啟動與資源控制 | 可採 native core，避免要求終端使用者裝 Python／Node runtime | UI、模型、native libraries 與系統服務仍影響 footprint；需實測 |
| STT 吞吐與延遲 | 方便整合 C／C++／Rust native engine | 使用何種模型與 backend 比外層語言更直接；Python native engine 也可有效率 |
| 呼叫公司 LLM | HTTP、取消與併發控制可用 Rust 實作 | 伺服器處理與內網延遲不會因 client 改寫語言自動消失 |
| 最少安裝步驟 | 可把依賴編進或隨 native binary 發行 | 靜態連結能力需逐個依賴驗證；模型權重與 OS runtime 另算 |

[Rust Reference 的 linkage 文件](https://doc.rust-lang.org/reference/linkage.html) 明確區分靜態與動態連結。Rust 可做單一 executable，卻不能由此保證選用的 C／C++ library、GPU runtime、Windows CRT 都沒有額外需求。

**目前建議：** 先以 Handy 這類既有 Rust／Tauri App 驗證同一套模型及公司 API。若需要自建且長期常駐資源很重要，可採 Rust core；若最先能取得的是 Python 工具鏈，Python＋native STT 仍可做可靠的比較基線。保留這個選擇，不需要先把 Python 路線排除。

## 2. 把「相依性少」分成三件事

| 層次 | 本案可採的目標 |
| --- | --- |
| 開發／建置相依 | 在一台 build 機解決並鎖版；pip／npm 經 Proxy 取得。Rust 的 Cargo、MSVC／Windows SDK、CMake 等能否取得尚未確認 |
| 安裝時下載 | exe、DLL、模型與必要 runtime 實際放進發行套件；由公司 Proxy 取得所有 payload |
| 安裝後執行 | 只讀本機模型，呼叫公司文字 LLM；需要的 runtime 已隨包或由公司環境提供 |

Tauri 使用 Windows WebView2，不能把「Rust＋Tauri」當成沒有瀏覽器 runtime 的 native UI。依 [Tauri 官方文件](https://v2.tauri.app/distribute/windows-installer/)，本案可使用公司映像既有 runtime 或隨包帶固定 runtime，installer／bootstrapper 已排除；選擇時把大小與更新責任一起計入。

若「完全不依賴 WebView2」是硬性要求，可另外研究 Rust 搭配不採 WebView 的 Windows UI。但這會增加 UI、Accessibility 與維護工作，本次沒有選定或驗收該 GUI framework。先查公司映像的 WebView2 是否已存在，可能更省工作。

## 3. npm／pip 可用來配送 Rust App

**套件管理器負責取得與更新檔案；App 的執行核心可以是預編譯的 Rust executable。** 以下兩種包裝均為本研究建議，沒有真的建立對應 package name：

| 配送方式 | 套件內放什麼 | 使用者端前提 |
| --- | --- | --- |
| npm package | 小型 JavaScript launcher、Windows exe、DLL、預設設定；另可搭固定版本模型 package | 安裝／啟動 wrapper 時需要 Node／npm；原生核心另以 process 執行 |
| Python wheel | Python launcher、Windows exe、DLL、預設設定；另可搭模型 package | 安裝／啟動 wrapper 時需要相容 Python／pip；wheel 應正確標示 Windows 平台與架構 |
| 套件給公司 build／配送機使用 | package 中含完整 portable payload，由公司再發給同事 | 終端使用者直接用 App；npm／pip 只需存在於取得套件的那台機器 |

第三種適合「同事電腦也盡量不要 Python／Node」的要求。若同事本來已有其中一套環境，前兩種則較方便。不能把「Proxy 能供應套件」推導為「所有同事電腦都已裝套件管理器」。

npm 的 [`files`、`bin`、`os`、`cpu` 設定](https://docs.npmjs.com/cli/v11/configuring-npm/package-json/) 可描述要打包的檔案、啟動命令與適用平台；Windows exe 可作為 package payload，JavaScript launcher 負責啟動。Python 的 [wheel 格式](https://packaging.python.org/en/latest/specifications/binary-distribution-format/) 與 [entry points](https://packaging.python.org/en/latest/specifications/entry-points/) 則支援分發檔案及產生命令入口；Windows `gui_scripts` 可用於不附帶 console 視窗的 launcher。

本案不使用 installer；若需桌面入口，可另做不需管理員權限的使用者捷徑流程並驗收。npm `bin` 或 pip entry point 本身不等於已有開始功能表、系統列自啟或解除安裝整合。

## 4. Proxy 必須拿到真正的 payload

建議的套件內容：

```text
company-dictation-app（名稱僅為示意）
  launcher/
  native/windows-x64/CompanyDictation.exe
  native/windows-x64/required-libraries/
  config/defaults.json
  manifest.sha256

company-dictation-model-whisper（另行固定版本）
  model-files/
  model-identity.json
```

程式包與模型包分開，可以讓 App 更新時沿用相同模型；是否適合拆包，要看實際模型大小與公司套件來源的單檔／package 上限。本產品只發 Windows amd64／x64，arm64 不在範圍。模型包的 cache 路徑與版本應由 App 明確管理。

要確認 `npm pack`／wheel 裡確實含有 exe、DLL 與權重，而非只放下載 URL。[npm 官方開發文件](https://docs.npmjs.com/cli/v12/using-npm/developers/) 說明可用 `npm pack` 檢查實際發行內容；wheel 也可直接列出 ZIP 成員與核對 manifest。兩種套件均需實測安裝後的檔案位置、執行權限與模型 discovery。

[Electron 安裝文件](https://www.electronjs.org/docs/latest/tutorial/installation) 區分 npm package 與 Electron binary 下載。這說明「npm 能安裝」和「所有 binary 都由 npm registry 提供」不能混為一談。模型亦常由不同 host 下載。若檔案已實際放入自製套件，就能依公司的套件 Proxy 路徑配送；若仍有額外下載，必須逐項納入發行方式。

Python 開發時可用 [`pip download`](https://pip.pypa.io/en/stable/cli/pip_download/) 建立 wheelhouse，固定依賴與 Windows／Python 平台。這能支援集中取得與離線安裝；模型權重不會因安裝 Python library 就必然包含在 wheelhouse。

公司內部 endpoint 預設值可放不含秘密的 config；API key、token、實際逐字稿及使用者歷史不應放進可分發套件。已確認通用產品包以公開 npm 為發布方向；精確 package scope／名稱、publisher 權限及 registry 限制仍須查核。本次沒有建立或發布產品包。

## 5. 對目前選型的影響

1. **Handy 仍是先行驗證基底。** 已有 Rust core、本機 STT 與自訂文字 LLM；可先量測，再決定包裝為公司用 portable 或套件 payload。
2. **若確定要自建，Rust＋native STT＋npm／wheel 配送值得優先設計。** 其理由是控制常駐資源與終端安裝相依；效能仍須以相同模型實測，並先確認 Rust build 工具取得方式。
3. **Python 是比較／研究候選。** pip Proxy 降低相依取得難度，native STT 可避免純 Python 推論假設；若量測已符合需求，沒有證據要求為語言本身重寫。
4. **OpenTypeless 的 Windows local mode 是 HTTP STT client。** 對「相依與安裝步驟少」的要求，額外本機 ASR server 需要計入，而不能只看它的 Rust／Tauri 標籤。[開源比較](02-open-source-alternatives.md)

最終驗收應同時測量 STT／LLM／總延遲、常駐 RAM、啟動時間、下載體積，以及乾淨 Windows 安裝後是否還會取得額外檔案。這些項目本次均未測量。
