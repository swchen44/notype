# notype

notype 免安裝語音輸入 App 的研究、決策與實作追蹤；先交付 Windows amd64，架構預留 macOS／Linux。

目標流程：**點按場景鍵錄音 → 獨立停止鍵 → 本機 STT → 公司文字 LLM 依場景處理 → 交付至原輸入目標**。場景為口述、翻譯、隨便問，各有可編輯與還原的提示詞。

**本輪先交付本機里程碑：錄音 → 繁／簡中英本機辨識 → 本機文字歷史 → 複製。** 使用者已確認其他 App 自動輸入後補；目前不在公司網路，公司 API／Proxy 與真正文字整理、翻譯、問答品質延後驗收，現在不需要 API key。

#9～#13 功能、#17 三場景／提示詞及 #14 啟動器基礎已合入整合分支。Qwen3-ASR 0.6B Q6_K 選作本機 pilot 模型，固定合成語音與階段時間見 [Qwen receipt](docs/verification/qwen3-quality.md)；不等於人聲或完整 session p95 通過。完整 Release／本機套件正在組裝驗證，狀態以 [HANDOFF](HANDOFF.md)／[TODO](TODO.md) 及各項 receipts 為準，不把開發檢查當作完整規格驗收。

正式 App 採 Rust + Tauri 2 + React／TypeScript + SQLite，共用核心可獨立執行 `cargo test --locked --manifest-path crates/notype-core/Cargo.toml`。`sessions.db`、設定與模型位於 Tauri 提供的 notype 每使用者資料目錄，可用絕對 `NOTYPE_DATA_DIR` 隔離，位於 npm 版本目錄外；資料庫具擁有權保護。公司 API 已確認採 OpenAI 相容 Chat Completions，真實連線／認證仍待驗證。

根目錄 `src/` 的 Node 22.15+ 文字流程保留作診斷 prototype，`{transcript, glossary} → {text}` 為 loopback 開發 contract，資料目錄與正式 App 分開。`packages/` 另為正式 npm 啟動器／Windows payload／模型套件角色；根目錄 `npm.cmd run test:unit`、`npm.cmd run test:integration` 同時驗證 prototype 與套件契約，正式 session 核心仍為 Rust。

## 已確認的限制

- MVP 只交付 Windows amd64／x64，先供本人日常使用，再擴給同事；共用核心與平台 adapter 預留 macOS／Linux，見 [跨平台架構 ADR](docs/adr/0003-shared-core-platform-adapters.md)。
- **交付不可使用 installer、setup.exe、MSI 或需要安裝的 runtime bootstrapper。** 採 portable 檔案包，由公司 npm Proxy 取得預編譯 App、DLL 與模型；只做 npm 配送，不另做 Python wheel。
- 公司目前只有文字 LLM API，語音辨識需在本機完成。
- 正式桌面沿用固定 Handy source fork；Rust App 仍需要已稽核的 native DLL 與 WebView runtime，完整包另行驗證。
- 本機語音辨識需支援「繁體中文＋英文」及「簡體中文＋英文」兩種中英混合模式，可切換並保存選擇，預設繁體中文＋英文；口述整理結果預設沿用當次原稿字形。LLM 失敗時保留原稿待選。兩種中文模式的模型品質與效能仍待驗收。
- 介面語言支援繁體中文（zh-TW）與簡體中文（zh-CN），可切換並保存偏好；介面語言獨立於語音辨識與文字輸出語言，完整性及兩種介面驗收進行中。
- 後續自動輸入目標為瀏覽器文字框、VS Code、Teams、Outlook、Windows Terminal；本輪先完成本機錄音／辨識／歷史／複製。所有 App 手動按 Enter，Terminal 首版以自然語言 prompt 為主。
- 本機長期保存場景、原稿、處理結果、時間與狀態，提供搜尋／複製／刪除；錄音只供當次處理。
- 點按開始，獨立可設定停止鍵預設 Right Alt；口述／翻譯／隨便問開始鍵預設 Right Alt／Right Alt + Right Shift／Right Alt + Space，Esc 取消。
- CPU 可用為基線，口述 10～20 秒短錄音以停止到交付 p95≤5秒為待驗證目標。

## 從這裡開始

| 文件 | 用途 |
| --- | --- |
| [MVP 與跨平台核心規格 #8](https://github.com/swchen44/notype/issues/8) | to-spec 規格、69 條 user stories、三場景／獨立提示詞、App 名稱情境／可選回譯與確認交付、介面／辨識語言、共用核心與平台 adapter、驗收與範圍 |
| [Windows 11 HANDOFF](HANDOFF.md) | 下一個 Codex session 的首讀文件、現況與可直接貼用的 prompt |
| [TODO](TODO.md) | 開放 issues、建議接續順序與完成條件 |
| [Windows MVP 定稿](docs/research/10-confirmed-windows-mvp.md) | 已確認產品行為、上游必改點與 Windows 實作／驗收順序 |
| [研究索引](docs/research/README.md) | Typeless、開源選型、架構、封裝與證據 |
| [Windows 接手說明](docs/research/07-windows-handoff.md) | Windows 開發與公司內網驗收的技術順序 |
| [決策紀錄](docs/research/08-decision-log.md) | 已確認選項、未決訪談問題與設計樹 |
| [工作紀錄](docs/research/HISTORY.md) | 研究／決策里程碑；程式碼演進以 Git commits 為準 |
| [測試規範](tests/README.md) | `tests/unittest/` 與 `tests/integration/` 的範圍、執行與回報規則 |
| [Agent skills](AGENTS.md) | Issue tracker、triage labels、domain docs 與測試目錄入口 |
| [GitHub Issues](https://github.com/swchen44/notype/issues) | 待辦、驗收條件與後續 evidence receipt |

此 repo 為 private。公司 endpoint、API key、token、實際工作錄音與逐字稿不屬於目前提交內容。通用產品包保留未來公開 npm 的方向；本輪只產出私有本機試用包，公司設定留本機，尚未公開發布或宣稱公司環境通過。
