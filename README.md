# notype_study

公司 Windows amd64 免安裝語音輸入 App 的研究、決策與實作追蹤。

目標流程：**熱鍵錄音 → 本機 STT → 公司文字 LLM 整理 → 貼回原本輸入框**。

目前完成 Typeless macOS 發行套件分析、8 個 GitHub 候選比較，以及 Handy 固定版本 source／Windows payload 靜態研究。產品規格正在 grilling 訪談中；公司 App 尚未實作，Windows runtime、效能與公司 API 尚未驗證。

## 已確認的限制

- 只做 Windows amd64／x64，先供本人日常使用，再擴給同事。
- **交付不可使用 installer、setup.exe、MSI 或需要安裝的 runtime bootstrapper。** 採 portable 檔案包，由公司 npm／pip Proxy 取得預編譯 App、DLL 與模型；主要套件渠道仍待確認。
- 公司目前只有文字 LLM API，語音辨識需在本機完成。
- 以修改現有開源 App 為優先，先評估 Handy；Rust 不代表沒有 native DLL 或 WebView runtime。
- 繁中夾英文，以忠實整理為準；LLM 失敗時停下，保留逐字稿供重試或貼原稿。
- 首批目標為瀏覽器文字框、VS Code、Teams、Outlook、Windows Terminal。

## 從這裡開始

| 文件 | 用途 |
| --- | --- |
| [研究索引](docs/research/README.md) | Typeless、開源選型、架構、封裝與證據 |
| [Windows 接手說明](docs/research/07-windows-handoff.md) | 在公司 Windows 開始實作與驗收的順序及接手 prompt |
| [決策紀錄](docs/research/08-decision-log.md) | 已確認選項、未決訪談問題與設計樹 |
| [工作紀錄](docs/research/HISTORY.md) | 研究／決策里程碑；程式碼演進以 Git commits 為準 |
| [測試規範](tests/README.md) | `tests/unittest/` 與 `tests/integration/` 的範圍、執行與回報規則 |
| [Agent skills](AGENTS.md) | Issue tracker、triage labels、domain docs 與測試目錄入口 |
| [GitHub Issues](https://github.com/swchen44/notype_study/issues) | 待辦、驗收條件與後續 evidence receipt |

此 repo 為 private。公司 endpoint、API key、token、實際工作錄音與逐字稿不屬於目前提交內容；未來 npm／pip package 的 registry 與可見性另行確認。此 repo 沒有發行公司 App 或模型包。
