# 工作紀錄

## 2026-10-24

- 使用 REA 分析本機 Typeless 2.8.3，保存 ASAR 身分、partial coverage、兩筆 native integrity 差異與17筆選讀摘錄；沒有執行聽寫。
- 比較8個 GitHub 候選；區分 main／stable、文字 LLM／STT、安裝形式與外部路徑。選讀來源保存固定ref及hash。
- 下載 Handy v0.9.8 固定source，追查 portable、模型預載、自訂 LLM、錯誤fallback及貼上路徑。
- 對 Handy x64 發布物做下載hash及靜態解包研究；沒有執行installer／payload。這是研究證據，不是公司部署方式。
- 使用者確認個人pilot、dictation MVP、可修改開源App、Windows amd64 only、繁中夾英文忠實整理、LLM失敗待選，以及五個目標App。
- 使用者新增硬限制：**不能使用 installer**。公司版以portable檔案包、預備runtime／模型與npm／pip Proxy配送為方向。
- 使用者要求建立GitHub repo與issue tracking；建立 private `swchen44/notype_study`。本次提交研究、Windows接手說明及待決紀錄；產品訪談與實作仍未完成。

後續每個issue用commit／可重現receipt連回結果。精確變更順序以Git history為準，這份文件不把未執行工作寫成完成。

- 已建立 GitHub issues #1～#6，分別追蹤規格、portable 啟動、STT、LLM、送字與配送。
- 開始 repo engineering skills setup：GitHub Issues 已確定；使用者確認五個預設 triage labels；根目錄指引檔名與設定草稿仍待確認。
- 本地 `.git` 寫入受工作區限制，改用 GitHub API 建立遠端 history；不宣稱本地 Git 初始化完成。
- 使用者選定 `AGENTS.md` 並採用設定草稿；已寫入 issue tracker、triage labels、domain docs，GitHub 五個 labels 已核對存在。
- 使用者指定 `tests/unittest/` 與 `tests/integration/`；建立目錄與測試規範。產品測試尚未實作，Windows runtime 尚未執行。

- Q7／Q8 採建議：Terminal 自然語言 prompt、所有 App 手動 Enter；CPU baseline 與短錄音放開到貼上 p95≤5秒為待驗證目標。
- Q9 固定 npm-only，不做 Python wheel；Q10 增加長期文字歷史，具體欄位／期限／錄音政策待 Q11。

- Q11～Q15 全部採建議：長期本機文字歷史與當次錄音分離、公開通用npm與本機公司設定、按住熱鍵／Esc、最小LLM上下文、原輸入目標一致才自動貼。
- 15項訪談決策收斂；補glossary、兩份ADR與Windows MVP定稿。History source audit確認需要拆開WAV gate／cleanup、加入文字LLM重試與歷史搜尋UI；新增追蹤issue #7。
- 研究定稿 commit `bdbcb13bdb2a3f529236b0e690344522351c5e54` 已保存，設計 issue #1 結案；#2～#7 留待 Windows 實作。
- 使用者明確選定自己的 Windows 11 開發，已有 Codex 與 grillme。新增根目錄 HANDOFF.md、TODO.md 與接手 prompt；不重問 Q1～Q15，其他 skills 未安裝時仍可依文件及 issues 接續。
- Windows 開發機與公司內網驗收條件分開記錄；本次僅更新交接文件及追蹤，沒有新增產品程式、產品測試、Windows runtime 或 npm 發布結果。
