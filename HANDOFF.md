# Windows 11 開發接手

更新日期：2026-10-25。Repository：[swchen44/notype](https://github.com/swchen44/notype)，private。

使用者已決定在自己的 **Windows 11** 開發，MVP 只交付 **Windows amd64／x64**；架構預留 **macOS／Linux**，見 [ADR 0003](docs/adr/0003-shared-core-platform-adapters.md)。目前尚未交付 macOS／Linux App。Q1～Q15 為原始決策，後續三場景與點按操作更新以 #8／decision log 為準，不重問已確認項目。

## 目前 checkpoint

正式功能與審查修正已合入 `implement/notype-mvp`。#9 桌面基線、#10 共用核心、#11 文字流程、#17 三場景／提示詞已依各自 receipt 結案；#12／#13 的功能已合入，品質／實際操作仍有未驗收項目。兩個獨立 reviewer 的 Standards／Spec findings 均已修正，最終限定複核各 0 項，見 [完整審查紀錄](docs/verification/local-code-review.md) 及 [修正與 RED→GREEN](docs/verification/local-review-fixes.md)。

正式 Release 建置來源 `239d6f05cdf7883032889b8d25042a7a427d72d9`，App SHA256 `7a70d5abe3558c2b12d2c8a16c0ed583bf0a231d3cbe8ba14fa89300b74257ab`，host helper SHA256 `6cd855cb408ccee695d800ce70de8c94ddd76e4d38e510a50c68976aac5bd38d`。其後 `a2d2328` 只有平台 Rustfmt 換行；[Linux CI 40162614988](https://github.com/swchen44/notype/actions/runs/40162614988) 通過核心 27、桌面 34（debug）、平台 4、Node 23 unit +16 integration、fmt／clippy。Windows Release 桌面契約為 33 項（native token target 的條件編譯數量不同），另有 9 項核心 session 與 4 項平台檢查。繁簡各 570 keys、前端 lint/build 通過。

三個實際私有 tarball 已產生於 `D:/git/notype-local-artifacts/`：`notype-0.1.0-local.1.tgz`、`notype-win32-x64-0.1.0-local.1.tgz`、`notype-model-0.1.0-qwen3-q6k.1.tgz`。實際內容清單／regular bytes／hash 檢查已完成；**離線安裝與正式包啟動仍在驗證**。首次 npm 試裝觸及 D 槽 1 GiB 下限而停止，保留失敗證據；正在以新的 C 槽 task-owned cache 重試。npm 取得本機 tarball metadata 會先解包暫存，必須計入約 1.49 GB 解包＋1.03 GB cache 的峰值；安裝成功及核對後先清除此暫存，再進行 690 MB 模型啟用，不降低 C≥2 GiB／D≥1 GiB 的界線。

Qwen 固定 6 cases／12 次推論與新版 converter 的 3 個 native 保存案例已完成，見 [Qwen](docs/verification/qwen3-quality.md)／[語言](docs/verification/languages.md)；首次 normalization 1176 ms、重用 0 ms 是階段測量，並非完整 session p95。人工原生 suite 的 1 次完整通過／2 次 Pending 斷言失敗與未知精確焦點差異保留於 [原生輸入 receipt](docs/verification/guarded-input.md)；舊 QA 入口已停用，新入口未啟用，不再要求使用者盲目重跑。整體規格仍未結案。

**使用者最新確認的本輪交付**：先完成「錄音 → 繁／簡中英本機 STT → 本機歷史 → 複製」。其他 App 自動輸入下一階段再補。使用者目前不在公司網路，公司 API／Proxy 與三場景真實文字品質驗收延後，現在不需 key；不另選 provider 或假造生成結果。本機 pilot 使用 Qwen3-ASR 0.6B Q6_K，完整 artifact/hash／Apache-2.0 LICENSE／NOTICE 見 Qwen receipt。#14 的正式 Release／完整私有 tarball、離線安裝、冷啟動與升級資料保留正在完成；公開發布及五個 App／人聲／p95 仍未驗收。此里程碑已同步 #8，不代表刪除完整規格的後續驗收。

2026-10-25 最新需求：口述、翻譯、隨便問三場景，提示詞獨立可編輯／保存／還原；使用者只要還原預設提示詞。開始鍵依序 Right Alt、Right Alt + Right Shift、Right Alt + Space；改為點按開始，錄音中以獨立可設定停止鍵結束（預設 Right Alt），Esc 取消，取代舊按住／放開。公司 API 格式已確認為 OpenAI 相容 Chat Completions。隨便問依意圖回答或代寫；**使用者已取消網域擷取，情境只保留 App 名稱，不讀頁面／輸入內容／標題或網址**。回譯可選且預設關閉，啟用時先顯示譯文及回譯，明確確認後才嘗試受保護交付。詳見 [設計紀錄](docs/research/11-scenario-prompt-design.md)。本輪產品選項已回答，不重問；新未定案行為仍須詢問。

2026-10-25 Prompt 補充：使用者提供角色／處理原則／格式／限制及 few-shot 草稿；已加入 #8／#11 的保真文字整理設計。只處理原稿文字，保留數字／否定／英文／code／既有不確定標記，依上下文刪贅詞，不新增講者或臆測音訊；原稿及術語表當作資料，不執行其指令。中英排版保護字面值，提示詞依當次辨識模式選字形，與介面 locale 獨立。真正模型品質仍待驗收，無需為 prompt 設計先提供 key。

2026-10-25 implement-spec 已啟動：整合分支 `implement/notype-mvp`（起點 `be6f0a5`），工作圖 #9～#17，具 GitHub 原生 blocking edges，見 [TODO](TODO.md)。#11 接合後解鎖 #12 語言及 #17 場景／提示詞；#13 需等待 #17 場景介面，再做配送／目標驗收。公司連線／真實 App／Proxy 未就緒時 #15／#16 保持未完成。實作以 TDD、獨立 worktree、整合與最終 code review 推進；只有實際達成驗收才結案。

2026-10-25 語音需求補充：本機 STT 必須支援「繁體中文＋英文」及「簡體中文＋英文」兩種同段中英混合模式，保存模式偏好，預設沿用繁體中文＋英文，並在 LLM 前產生與保存所選中文字形的原稿。介面 locale 與辨識模式獨立；模式切換不改動既有歷史，整理結果預設沿用當次原稿字形。規格 #8 與 STT issue #3 同步此需求，兩種中文品質／效能尚未驗收，英文 tiny.en smoke 不能代替。

2026-10-25 補充：介面納入繁體中文（zh-TW）與簡體中文（zh-CN），可切換並保存偏好，翻譯資源跨平台共用；介面語言獨立於 STT／文字輸出，不自動改寫原稿、整理結果或文字歷史。已更新規格 #8，尚待正式介面實作。使用者要求真正需要 API key 時再通知；目前以本機 STT／HTTP fixture 繼續開發，不要求先申請或提供 key。

2026-10-25：已依 to-spec 發布 [完整規格 #8](https://github.com/swchen44/notype/issues/8)，標記 ready-for-agent；使用者確認主要測試 seam。正式 App 方向為 Handy 的 Rust + Tauri 2 + React／TypeScript + SQLite；現有 Node 實作為 prototype／驗收先例。先建立共用核心與平台 adapter 的 contract，將 Windows 專用資料位置及原生互動隔離，再繼續正式 App 整合。macOS／Linux 目前只規劃架構，不宣稱已可執行。

2026-10-24 Windows 接手成果：產品與 private repo 改名為 **notype**（本機資料夾仍為 notype_study）。GitHub CLI、CMake、Bun、Rust、MSVC／Windows SDK 已安裝，Rust／C++ 編譯及執行驗證通過；Vulkan 安裝被取消、尚未完成。文字核心與固定英文合成 WAV→CPU STT→HTTP fixture→SQLite→Edge 文字框已接通，11 unit／5 integration／1 runtime smoke 通過。麥克風熱鍵、繁中品質、原生跨 App 送字、公司 API 與完整 portable 產品仍待完成。詳見 [preflight](docs/verification/windows11-preflight.md)。使用者已授權安裝開發所需工具，產品交付仍維持 npm portable。以下研究 checkpoint 為接手前背景。

- 研究、選型比較、15 項產品決策、兩份 ADR、測試目錄規範與 GitHub issues 已保存。
- 研究定稿 commit：[bdbcb13](https://github.com/swchen44/notype/commit/bdbcb13bdb2a3f529236b0e690344522351c5e54)。本交接文件屬於其後續更新，請以 clone 後的 `HEAD` 與 live issues 核對最新狀態。
- 尚無產品程式碼、可執行產品測試、Windows runtime／效能結果或已發布 npm 產品。`tests/` 目前只有規範，不能回報測試通過。
- [#1 設計收斂](https://github.com/swchen44/notype/issues/1) 已結案；[#2～#7](TODO.md) 是開放待辦，尚未全部拆成可直接執行的規格切片。

## 先讀哪些檔案

依序讀 [AGENTS.md](AGENTS.md)、[GLOSSARY.md](GLOSSARY.md)、[npm 配送 ADR](docs/adr/0001-portable-npm-distribution.md)、[文字歷史 ADR](docs/adr/0002-text-history-independent-of-audio.md)、[MVP 定稿](docs/research/10-confirmed-windows-mvp.md) 與 [TODO.md](TODO.md)。

再按工作範圍查 [決策來源](docs/research/08-decision-log.md)、[Windows 技術接手](docs/research/07-windows-handoff.md)、[Handy source／payload audit](docs/research/06-source-feasibility-audit.md)、[history source audit](docs/research/09-history-source-audit.md)、[測試規範](tests/README.md) 與 live GitHub issues。原始研究入口為 [docs/research/README.md](docs/research/README.md)。

## 下一個 session 的工作

1. **核對目前狀態。** 讀此 checkpoint、TODO、live #8／#12～#16 與 receipts，核對 remote／branch／HEAD／未提交變更。工具及原生 Release 已建置完成，依現有證據接續，不重做最初選型或要求 key。
2. **完成 #14 的本機交付。** 續做實際三包離線安裝、完整性與 normal-user cold launch、兩模式真實 CPU 推論／fixed WebView／外部資料庫重開。只用自有暫存 profile，不自動錄麥克風或讀使用者原有設定。保留失敗證據；同 facade 版本更新的 proof 不等於 native binary／schema migration／中斷／鎖檔驗收。
3. **提供可直接試用的入口。** 安裝位置採 `%LOCALAPPDATA%/notype/local-app`，使用者試用資料採 `%LOCALAPPDATA%/notype/local-trial`，都在建置快取外。使用者資料不能列入 QA/cache cleanup。由正常使用者從 Explorer 啟動，等自有測試程序全部結束後再交付；尚未完成實際安裝前不可聲稱 Starter 已可用。
4. **按最新範圍接續驗收。** 先交付本機錄音／辨識／歷史／複製，再處理人聲與實體快捷鍵回饋。其他 App 自動輸入與公司 API／Proxy 延後；有新產品歧義才詢問，不重新訪談已確認選項。各項成果連回 issue／commit／receipt。

使用者已確認目前不在公司網路，先完成本機版本。公司 API／Proxy 維持未驗收，現在不需要 key，也不另選公開 provider。

七個已合入的實作／修正 worktrees 已清理，branch commits 與外部證據保留；目前只留下 active packaging worktree。唯一 native target 為 `D:/git/notype-build`，舊 debug／core target 已退役，不能重建第二份來迴避 cache 使用權。獨立 C++ prefix 是 `%LOCALAPPDATA%/notype-study/build-cache/native-prebuilt/transcribe-cpp-0.2.4-win64-cpu-release-E4b0TR`；舊 tcs 是指向已刪 debug 的 junction，不能當作獨立備份。清理只限確認 idle、驗證絕對範圍且不跟隨 reparse point 的 task-owned derived files；不得清除模型、歷史、來源 registry、系統或 pagefile。

## 接手時特別保留的邊界

產品交付只用 npm 取得完整預編譯 portable payload，禁止 setup／MSI／runtime bootstrapper。公司只有文字 LLM API，因此 STT 必須在本機。持久文字歷史及模型應放在 npm 版本化程式目錄之外；不可假設 Handy 已有 `--data-dir` 等未經 source 核對的參數。

Handy 的錯誤 fallback、WAV gate／cleanup 與 installer updater 已按本產品修改，原始差異見兩份 audit。現有 Qwen／固定 WebView／Release 證據與公司 API／認證／CA、完整效能、publisher／Proxy 等未驗收項目分開記錄。短錄音完整 session p95≤5 秒仍是目標。

公司 endpoint、憑證、工作錄音及逐字稿留本機，不提交 GitHub、issues 或通用 npm 包。固定來源、hash、靜態檢查、build、Windows 啟動、五個 App、公司 API 與效能各自記錄；測試替身不代表公司驗收通過。

## 建議使用的 skills

Windows 已有 Codex 與使用者所稱的 `grillme` skill。先查當地實際 skill 名稱；只對研究後仍需使用者決定、且會改變既有方案的新問題使用 `grill-me`／`grilling`。已確認的 Q1～Q15 直接沿用，技術事實由 agent 查核。

若有 `to-spec`／`to-tickets`，可用來補規格與相依順序；沒有時直接在 repo／issues 記錄等效的範圍、驗收、相依與測試方式，繼續工作。不要因 Mac 曾有某個 skill 就假設 Windows 也已安裝。重大新決策更新 decision log／ADR；一般實作與可逆驗證直接執行。

## 直接貼給 Windows Codex

```text
請接手並繼續開發 private repo https://github.com/swchen44/notype。
若目前不是這個 repo，先取得或定位工作副本，保留現有未提交變更。

先讀 HANDOFF.md 的目前 checkpoint、AGENTS.md、TODO.md、GLOSSARY.md、docs/adr/，
再查 live #8、#12～#16 及對應 receipts。本期 Windows amd64，架構預留 macOS／Linux。
產品選項已確認；有新歧義才問我，不能自行猜測，也不要重問已回答的問題。

核對 repo 與現有產物後，接續尚未完成的 #14 實際安裝／啟動與本機交付。
這輪先交付錄音→繁／簡中英本機 STT→本機歷史→複製；其他 App 自動輸入下一階段再補。
我目前不在公司網路，公司 API／Proxy／真實文字品質延後，真正需要 key 時再通知我。
沿用已固定的 Handy source、Qwen pilot、Release 與完整私有 tarball，不重做選型或安裝工具。
只用 npm 配送完整 portable payload，不使用 installer，不把 fixture 或 source checks 當作實際啟動。
依 tests/README.md 在 tests/unittest/ 與 tests/integration/ 加入並執行實際測試。
遵循已定稿的失敗待選、焦點保護、手動 Enter 與錄音清理規則。
持續更新 issue、TODO、commit 和驗證紀錄；公司設定、憑證與工作內容留在本機。
只有 Codex 和 grillme 也要能接續，不要因缺少其他規劃 skills 停工。
```
