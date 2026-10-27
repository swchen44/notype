# notype

notype 免安裝語音輸入 App 的研究、決策與實作追蹤；先交付 Windows amd64，架構預留 macOS／Linux。

目標流程：**點按場景鍵錄音 → 獨立停止鍵 → 本機 STT → 公司文字 LLM 依場景處理 → 交付至原輸入目標**。場景為口述、翻譯、隨便問，各有可編輯與還原的提示詞。

**本輪先交付本機里程碑：錄音 → 繁／簡中英本機辨識 → 本機文字歷史 → 複製。** 使用者已確認其他 App 自動輸入後補；目前不在公司網路，公司 API／Proxy 與真正文字整理、翻譯、問答品質延後驗收，現在不需要 API key。

## Windows 11 測試版與驗證狀態

目前尚未發布本輪完整離線 ZIP。R10 的 App 驗證／獨立封存、平台 helper、原生測試列舉及兩項錄音計量測試已通過，見 [R10 原生紀錄](https://github.com/swchen44/notype/issues/14#issuecomment-6431337535)。d2 的 count／stage／archive 已通過並留存候選 ZIP；原 extract 因 independent-monitor-stale 失敗，沒有成功的原 audit。V8 準備、完整來源審查與 root admission 已完成，但 [d3 唯一一次實際執行](https://github.com/swchen44/notype/issues/14#issuecomment-6433977441)於 2026-10-27 04:41 UTC 啟動後約4.4秒，在 predecessor-proof 以 v7-session-worker-rejected 失敗；未接受前置證明，未到達 ZIP／既有副本驗證或 bundled-Node 公開檢查。[唯讀診斷已證實](https://github.com/swchen44/notype/issues/14#issuecomment-6434068783)：替換中介函式遺失原有預設參數值，導致固定任務拒絕有效呼叫；14個路徑逐位元組相同，無需路徑正規化。尚未修正或重試；新計畫待 Astra 審查，尚未核准。R9、d1、d2 及這次 d3 失敗均保留，現在沒有已驗收成品下載。

先前累計 C 用量的已知下限至少為 2,245,115,904 bytes，超過原 1,985 MiB 上限 163,692,544 bytes，尚未加入完整稽核預留；可用空間回升不能抵銷此歷史超限。[V8 計畫](https://github.com/swchen44/notype/issues/14#issuecomment-6433597289)已獲[Astra 核准0](https://github.com/swchen44/notype/issues/14#issuecomment-6433619965)，明確增加有限 C512MiB（post-native 1985→2497MiB、aggregate 2241→2753MiB）；[root carry／當下 admission 重播](https://github.com/swchen44/notype/issues/14#issuecomment-6433907847)已通過。這不等於實際驗收：目前阻塞是上述 d3 前置證明失敗，完整準備、失敗與來源邊界見[驗證狀態](docs/verification/delivery-v8-preparation.md)。

App 實際編譯來源為 `a180a2360009448cdf6e2f4f8e7363e0eb2dbf30`（A），在 R7 編譯，由 R10 驗證並獨立封存；原生測試與組裝來源為 `a7fc1d79822544e6f4858cb32bcd809067367cc3`（T）。A→T 僅根目錄 README 與既有 cfg(test) 模組的一行 ReadySignal 匯入不同，不宣稱 App 由 T 重新編譯；文件 commit 亦不代表重新建置。

預定完整包適用 Windows 11 x64／AVX2＋BMI2，內含 Node、Fixed WebView2、原生執行環境及 Qwen3-ASR 0.6B Q6_K。本機辨識不需 API key。只有完整 ZIP 實際驗證通過後，才提供 GitHub Release 下載位置、雜湊與對應紀錄；目前不提供未驗證下載連結。

完整包實際交付後，先結束所有舊版 notype，完整解壓到新資料夾，再使用 `Start-notype.cmd`。已有實例時，第二次啟動會喚回既有程式，不能據此確認新版或新的資料目錄已啟動。若用 `NOTYPE_DATA_DIR` 隔離試用資料，每次都須用相同命令與同一絕對路徑啟動；直接雙擊不會記住前次命令視窗的環境變數。完整操作與下載位置須待實際最終驗收後發布。

| 範圍 | 實際狀態 | 證據與限制 |
| --- | --- | --- |
| Rust 核心、桌面流程、診斷、Provider、模型資料夾與 bootstrap 契約 | T來源CI已通過 | [ALL40427656120](https://github.com/swchen44/notype/actions/runs/40427656120)兩個Linux jobs成功；使用自有檔案、SQLite、本機HTTP及受控裝置替身，不是實體錄音或公司服務 |
| 前端介面、語系與設定契約 | T來源CI已通過 | 受控IPC rendered tests，不等於新版原生視窗與裝置驗收 |
| Windows平台中立contracts | 已通過 | [Windows40427656156](https://github.com/swchen44/notype/actions/runs/40427656156)；artifact job skipped，不能作為App建置證據 |
| Windows Release App | R7/A編譯／R10攜帶驗證通過 | 59,009,536 bytes；SHA-256 `1432e7fa897a4235a5fc2a0f492c45a0c6ce4b503cfb93eb72135c98121b5ef0`。不代表新版GUI／麥克風已啟動驗收 |
| Windows平台helper | R10/T編譯／封存通過 | 146,944 bytes；SHA-256 `121bdc97e1c8d8e0b5fb633be4c5c998ab10eba0f088d477e67c46a4a70ac31b`。不等於全部native測試通過 |
| 原生錄音計量library fixtures | R10列舉及執行兩項均通過 | 2 passed／0 failed／0 ignored；驗證實收靜音與decoder padding計量，不開啟實體麥克風；R9失敗保留 |
| 交付輸入清單 | T清單已審查；R10 native前置通過 | 原始模型／runtime／notices保留；輸入清單與fixtures不代替整包驗收 |
| 本輪完整ZIP、checksum、獨立解壓與逐檔核對 | d2候選ZIP留存；原extract失敗，V8實際d3前置證明失敗 | 未到達ZIP／既有副本驗證及公開check；原extract／audit不補造成功，目前不發布下載 |
| V7／V8準備與實際結果 | [V7準備121/121、完整Spec0／Standards0](https://github.com/swchen44/notype/issues/14#issuecomment-6433705942)；V8程式、86項current／64項retained證據與完整雙軸0、[root admission](https://github.com/swchen44/notype/issues/14#issuecomment-6433907847)已完成 | V7的55個fixture Workers及owned children已terminal；[實際d3 FAILED](https://github.com/swchen44/notype/issues/14#issuecomment-6433977441)，完整failure／state留存、accountingComplete及全部owned children／Workers terminal均true；fixtures與admission不代替產品驗收 |
| 新版GUI、快捷鍵、秒數、系統匣及設定持久化 | 本輪未完成實機驗收 | 舊pilot及受控契約只作各自範圍的歷史證據 |
| 介面語系與模型目錄 | 本輪原生GUI待測 | 繁中／簡中保存、卡片基本／進階資訊、相對評分排序、下載進度／失敗恢復、模型metadata宣告的全部語音語言；不宣稱固定30種語音或介面語言 |
| 三場景提示詞、回譯確認與交付 | 本輪原生GUI與實際API待測 | 編輯／還原／保存、確認顯示版本後另行交付或複製；API失敗保留原稿 |
| 已保存API連線、模型清單與thinking／effort | 本輪實際服務待測 | 啟用／切換連線、有效設定與實際行為；受控provider contracts不代替真實服務 |
| 實體麥克風權限、裝置拔除、睡眠／喚醒 | 本輪未執行；建議人工驗收 | 裝置替身測試不代表實際OS／硬體事件通過，也不表示這些案例已失敗 |
| 繁中＋英文／簡中＋英文品質、麥克風／噪音、session p95 | 尚未完成實機品質／效能驗收 | [既有模型測量](docs/verification/qwen3-quality.md)與10～20秒錄音停止至交付p95≤5秒目標分開，不以相對分數宣稱達標 |
| 真實大模型搬移、重裝沿用、使用中切換 | 本輪實體操作待測 | 自有小檔案／worker契約已測；真實權重與檔案鎖另驗收，App不替使用者搬移／複製已有權重 |
| 中文／空白／長路徑、唯讀資料夾與Windows權限 | 本輪實機未執行；建議人工驗收 | 既有路徑／檔案fixtures不代表實際OS組合已測過；不是已知失敗清單 |
| 歷史CSV原生GUI匯出與完整API案例 | 本輪待實測 | SQLite／UTF-8 BOM契約已測；原生對話框、取消／重試、回譯、筆數與部分／未知tokens／費用；UTF-8 BOM CSV須由Excel／研究工具實際開啟核對中文及欄位，不把未知當零，費用按幣別分開 |
| DeepSeek真實API | 歷史專用helper回合未完整通過 | [#25實際紀錄](https://github.com/swchen44/notype/issues/25#issuecomment-6411081019)：6次請求、DB5筆／CSV4筆，回譯用詞檢查失敗後停止；取消／重試後續未執行，費用UNKNOWN。不是新版App API驗收 |
| OpenRouter／Gemini真實請求 | 未執行 | 本機HTTP fixtures或免費模型標籤不代表供應商實測 |
| 真實429／斷線、公司Custom API TLS／CA／Proxy | 本輪未執行；建議人工驗收 | 受控限流／錯誤／逾時／取消契約不代表公司環境已測，也不表示實機案例已失敗 |
| 診斷OFF／Clear／匯出隱私的實際GUI操作 | 本輪未執行；建議人工驗收 | 含重啟偏好及使用者在本機檢查實際key保護；合成秘密／安全DTO契約不代替GUI與匯出人工檢視，不附私人key或設定回報 |
| 無Node/npm/開發工具的乾淨Windows11、首次離線／網路行為 | 未完成獨立環境驗收 | 含SmartScreen、Defender及公司執行政策的實際啟動；未證明通過或被擋，未取得斷線／封包證據，不宣稱clean-host或零網路通過 |
| 瀏覽器、VS Code、Teams、Outlook、Windows Terminal自動輸入 | 未完成新版五App驗收 | 歷史標準Win32欄位不代表五App通過；新版錄音／辨識／歷史／複製需各自實機核對 |
| macOS／Linux桌面App | 未交付／未測 | portable CI不等於桌面成品 |

### 完整包交付後的建議人工檢查

1. 首次啟動、麥克風權限、繁中／英文及簡中／英文口述；另測裝置拔除及睡眠／喚醒。
2. 開始／停止／取消、秒數、快捷鍵、系統匣還原／結束及重啟後設定保留。
3. 原稿／結果字數、耗時、模型與原生GUI CSV匯出，核對實際筆數及內容。
4. 中文／空白／長路徑、唯讀模型資料夾、自行搬移整個模型資料夾後重新指定及重裝沿用。
5. 明確設定自己的文字API後，測翻譯／回譯、429、取消／重試、斷線及公司TLS／Proxy。
6. 在GUI關閉診斷、Clear、重新開啟／匯出，核對偏好保存，並只在本機檢視實際key保護及預期紀錄。 Excel／研究工具另實際開啟UTF-8 BOM CSV核對中文、欄位及未知／部分費用。
7. 介面語系保存、模型目錄基本／進階資訊、排序與下載進度；核對該模型宣告的全部語音語言。
8. 三場景提示詞編輯／還原／保存、回譯顯示版本確認與另行交付／複製；已保存API連線切換、模型清單及thinking／effort有效設定。
9. 在無開發工具的乾淨Windows11實測首次離線啟動、SmartScreen／Defender與公司執行政策；這些是待驗收項目，不是已知失敗。

以上是待執行建議，不是測試結果或已知失敗清單。初次請在自有空白測試輸入框驗證；回譯「確認此顯示版本」不會自動交付，需要時另按「交付至原目標」或複製。回報問題時手動檢視並附上診斷檔、版本及重現步驟；勿將個人key、錄音、歷史或私人設定加入通用安裝包。

先前 pilot 的歷史操作與證據見 [本機試用說明](docs/local-trial.md)及[兩軸審查與修正](docs/verification/local-code-review.md)，不作為本輪新版原生／封裝驗收。完整 #8、#14 尚未結案，見 [HANDOFF](HANDOFF.md)／[TODO](TODO.md)。

正式 App 採 Rust + Tauri 2 + React／TypeScript + SQLite，共用核心可獨立執行 `cargo test --locked --manifest-path crates/notype-core/Cargo.toml`。`sessions.db` 與設定位於 Tauri 提供的 notype 每使用者資料目錄，可用絕對 `NOTYPE_DATA_DIR` 隔離；模型資料夾可獨立指定及保存，與程式安裝位置分開。資料庫具擁有權保護。公司 API 已確認採 OpenAI 相容 Chat Completions，真實連線／認證仍待驗證。

根目錄 `src/` 的 Node 22.15+ 文字流程保留作診斷 prototype，`{transcript, glossary} → {text}` 為 loopback 開發 contract，資料目錄與正式 App 分開。`packages/` 另為正式 npm 啟動器／Windows payload／模型套件角色；根目錄 `npm.cmd run test:unit`、`npm.cmd run test:integration` 同時驗證 prototype 與套件契約，正式 session 核心仍為 Rust。

## 已確認的限制

- MVP 只交付 Windows amd64／x64，先供本人日常使用，再擴給同事；共用核心與平台 adapter 預留 macOS／Linux，見 [跨平台架構 ADR](docs/adr/0003-shared-core-platform-adapters.md)。
- **交付不可使用 installer、setup.exe、MSI 或需要安裝的 runtime bootstrapper。** 保留 npm／公司 npm Proxy 配送，另提供預定的完整離線 ZIP；基本 App 與模型取得分開，不另做 Python wheel，見 [配送 ADR](docs/adr/0001-portable-npm-distribution.md)。
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

此 repo 為 private。公司 endpoint、API key、token、實際工作錄音與逐字稿不屬於目前提交內容。通用產品包保留未來公開 npm 的方向；本輪ZIP候選已留存，但V8實際d3失敗，未完成最終驗收，不提供Release下載或宣稱公司環境通過。文件更新反映當前狀態，不代表重新編譯App；公司設定仍留本機。
