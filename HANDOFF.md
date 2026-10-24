# Windows 11 開發接手

更新日期：2026-10-24。Repository：[swchen44/notype_study](https://github.com/swchen44/notype_study)，private。

使用者已決定在自己的 **Windows 11** 開發，只做 **amd64／x64**。下一個 session 應接續實作與目標機驗證；Q1～Q15 已確認，不必重新訪談。

## 目前 checkpoint

- 研究、選型比較、15 項產品決策、兩份 ADR、測試目錄規範與 GitHub issues 已保存。
- 研究定稿 commit：[bdbcb13](https://github.com/swchen44/notype_study/commit/bdbcb13bdb2a3f529236b0e690344522351c5e54)。本交接文件屬於其後續更新，請以 clone 後的 `HEAD` 與 live issues 核對最新狀態。
- 尚無產品程式碼、可執行產品測試、Windows runtime／效能結果或已發布 npm 產品。`tests/` 目前只有規範，不能回報測試通過。
- [#1 設計收斂](https://github.com/swchen44/notype_study/issues/1) 已結案；[#2～#7](TODO.md) 是開放待辦，尚未全部拆成可直接執行的規格切片。

## 先讀哪些檔案

依序讀 [AGENTS.md](AGENTS.md)、[GLOSSARY.md](GLOSSARY.md)、[npm 配送 ADR](docs/adr/0001-portable-npm-distribution.md)、[文字歷史 ADR](docs/adr/0002-text-history-independent-of-audio.md)、[MVP 定稿](docs/research/10-confirmed-windows-mvp.md) 與 [TODO.md](TODO.md)。

再按工作範圍查 [決策來源](docs/research/08-decision-log.md)、[Windows 技術接手](docs/research/07-windows-handoff.md)、[Handy source／payload audit](docs/research/06-source-feasibility-audit.md)、[history source audit](docs/research/09-history-source-audit.md)、[測試規範](tests/README.md) 與 live GitHub issues。原始研究入口為 [docs/research/README.md](docs/research/README.md)。

## 下一個 session 的工作

1. **唯讀接手。** 核對 repo remote、分支、HEAD 與未提交變更，不覆蓋既有工作。確認 Windows 11 實際 OS／process 架構與 CPU／RAM、現有 Node／npm、WebView2、Rust／MSVC／CMake 等工具及可寫路徑；保留版本與查核命令。不能只因 OS 名稱是 Windows 11 就推定 amd64 或 runtime 齊備。
2. **先完成 #2 的環境判斷。** 優先評估 Handy v0.9.8，固定 commit `14f6f0d31cb22a4268acfbddba3052820fd0a06e`。依 audit 的固定來源重新取得 source／payload，核對身分；Mac 的 `/private/tmp`、模型與解包目錄不會隨 repo 移轉。先查可行路徑，不執行 installer。缺 native build tools 時先查現有資源與其他 Windows build 環境，不把公司機器安裝工具當成預設方案。
3. **實作第一個可驗證的完整切片。** 將工作範圍與驗收記入 issue，再串接非機密錄音 → 本機 STT → 文字 LLM adapter → 文字歷史 → 測試輸入框。先建立可分別執行的 unit／integration 命令，隨行為加入測試。尚缺公司 API contract 時可用本機測試 HTTP server 驗證流程；公司相容性維持 `not-performed`，並繼續不依賴它的工作。
4. **依 TODO 擴大驗收。** 加入錯誤待選、取消、無長期 WAV 的持久化與重試、焦點保護、五個 App，以及 npm 更新後資料保留。Teams／Outlook 使用測試草稿；所有 App 由使用者手動 Enter。每項工作連回 issue、commit 與 receipt。

Windows 11 開發機是否位於公司內網尚未查核。若不在內網，先做本機開發與測試，真實公司 API、Proxy 取得及公司 Windows 冷啟動另外留下明確驗收結果。

## 接手時特別保留的邊界

產品交付只用 npm 取得完整預編譯 portable payload，禁止 setup／MSI／runtime bootstrapper。公司只有文字 LLM API，因此 STT 必須在本機。持久文字歷史及模型應放在 npm 版本化程式目錄之外；不可假設 Handy 已有 `--data-dir` 等未經 source 核對的參數。

Handy 既有錯誤 fallback、WAV gate／cleanup 與 updater 路徑都與本產品需求有差異，詳見兩份 audit；不能只改 retention 設定。實際模型、API contract／認證／CA、硬體效能、WebView runtime 與 npm publisher 尚缺證據。短錄音 p95≤5 秒是目標，尚未量測。

公司 endpoint、憑證、工作錄音及逐字稿留本機，不提交 GitHub、issues 或通用 npm 包。固定來源、hash、靜態檢查、build、Windows 啟動、五個 App、公司 API 與效能各自記錄；測試替身不代表公司驗收通過。

## 建議使用的 skills

Windows 已有 Codex 與使用者所稱的 `grillme` skill。先查當地實際 skill 名稱；只對研究後仍需使用者決定、且會改變既有方案的新問題使用 `grill-me`／`grilling`。已確認的 Q1～Q15 直接沿用，技術事實由 agent 查核。

若有 `to-spec`／`to-tickets`，可用來補規格與相依順序；沒有時直接在 repo／issues 記錄等效的範圍、驗收、相依與測試方式，繼續工作。不要因 Mac 曾有某個 skill 就假設 Windows 也已安裝。重大新決策更新 decision log／ADR；一般實作與可逆驗證直接執行。

## 直接貼給 Windows Codex

```text
請接手並繼續開發 private repo https://github.com/swchen44/notype_study。
若目前不是這個 repo，先取得或定位工作副本，保留現有未提交變更。

我已決定在這台 Windows 11 開發，只支援 amd64。先讀 HANDOFF.md、AGENTS.md、
TODO.md、GLOSSARY.md、docs/adr/、docs/research/10-confirmed-windows-mvp.md，
再查 GitHub issues #2～#7。Q1～Q15 已確認，不要重新訪談；grillme 只用於新的重大決策。

先唯讀核對環境與 repo 狀態，將非機密結果記入 docs/verification/，接著從 #2 開始，
完成第一個可驗證切片：非機密錄音→本機 STT→文字 LLM adapter→長期文字歷史→測試文字框。
優先評估已固定版本的 Handy。公司 API 尚缺 contract 時可用本機測試替身繼續，
清楚標示尚未驗證公司相容性。只用 npm 配送完整 portable payload，不使用 installer。
依 tests/README.md 在 tests/unittest/ 與 tests/integration/ 加入並執行實際測試。
遵循已定稿的失敗待選、焦點保護、手動 Enter 與錄音清理規則。
持續更新 issue、TODO、commit 和驗證紀錄；公司設定、憑證與工作內容留在本機。
只有 Codex 和 grillme 也要能接續，不要因缺少其他規劃 skills 停工。
```
