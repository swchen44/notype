# notype 開發待辦

2026-10-26 09:30：#28 模型資料夾功能已在 backend／UI 獨立 worktrees 進行，contract `661b968`／`39e9569`／`8ab6274`；service 小型實際檔案與 UI 已有逐步 RED→GREEN，原生組合、最終 composed CI／雙軸0仍待完成，尚未合入／交付。#25 隔離 API helper 依 [live-v4](https://github.com/swchen44/notype/issues/25#issuecomment-6405986527)／[Astra 核准0](https://github.com/swchen44/notype/issues/25#issuecomment-6406003416) 平行做 source／owned-fixture TDD，真實 key／推論／付費仍0；Gemini 本輪因 free-project 未證明封存，不延用舊 source41 helper。#29 等 #28 gate；P14-v5 雖核准，仍待所有 source／CI／leases／共存容量 gate。詳細 owner、paths、race 保護及資源界線見 [HANDOFF](HANDOFF.md) 最新節；下文「下一個／尚未開始」按當時歷史保留。

2026-10-26 08:56 最新 frontier：**#26/#27 source-only 切片完成**，exact `ab9e785` → 保留歷史的 integration merge `c5605f4`。獨立 Spec0／Standards0，[完整 ALL CI40251455558](https://github.com/swchen44/notype/actions/runs/40251455558) exact ab9 completed/success；UI3／66、Node24／18、core49＋doc1、platform4、desktop158 全通過，另1個 controlled child role 由通過的 proxy parent 明確執行。見 [最終 receipt](docs/verification/provider-config.md)／[parent gate](https://github.com/swchen44/notype/issues/25#issuecomment-6405850082)。

下一項 **#28**（[計畫](https://github.com/swchen44/notype/issues/28#issuecomment-6405356803)／[Astra 核准0](https://github.com/swchen44/notype/issues/28#issuecomment-6405400881)）→ **#29**（[計畫](https://github.com/swchen44/notype/issues/29#issuecomment-6405357391)／[核准0](https://github.com/swchen44/notype/issues/29#issuecomment-6405428255)）；本 merge 未包含兩者產品實作。P14-build-v5（[計畫](https://github.com/swchen44/notype/issues/14#issuecomment-6405766076)／[核准0](https://github.com/swchen44/notype/issues/14#issuecomment-6405783543)）及 delivery-v5（[計畫](https://github.com/swchen44/notype/issues/14#issuecomment-6405766320)／[核准0](https://github.com/swchen44/notype/issues/14#issuecomment-6405789439)）已實際核准，依賴 #28/#29；build／新版 App／實際 ZIP 尚未完成。#25/#8／native／live／公司／五 App 仍 open。所有來源 leases 已交還、worktrees 保留；沒有 Cargo／App／home/key／profile／live／paid／交付／清理。最近先前 C約6.85GB／D4.14GB，不沿用舊不足結論，native 前仍須當下預算／資源 gate。以下舊 checkpoint 的時態以本段、[HANDOFF](HANDOFF.md) 與 live issues 為準。

2026-10-26 08:40 最新 frontier：#27 UI 已合入，#26 composed candidate `2977db0` [完整 ALL CI 成功](https://github.com/swchen44/notype/actions/runs/40249253105)，來源審查的 3 項 Spec finding＋1 項 Standards 重複邏輯建議由單一 fixer 依 [reviewfix-v2](https://github.com/swchen44/notype/issues/25#issuecomment-6405575893)／[Astra 核准0](https://github.com/swchen44/notype/issues/25#issuecomment-6405614167) 修正中；最終來源／CI gate 尚未通過。之後接 [#28 模型資料夾選取／重用](https://github.com/swchen44/notype/issues/28)（[計畫核准](https://github.com/swchen44/notype/issues/28#issuecomment-6405400881)）→ [#29 完整離線 ZIP／無 key 預設](https://github.com/swchen44/notype/issues/29)（[核准](https://github.com/swchen44/notype/issues/29#issuecomment-6405428255)），皆未開始產品實作。App 不搬模型，使用者手動換位置後重新指定；基本 App 缺模型可開設定，完整 ZIP 另驗證全部宣告模型。見 [ADR0001](docs/adr/0001-portable-npm-distribution.md)。#14 build/delivery-v3 未核准且須因這兩項修訂；目前沒有本機 Cargo／新原生 App／實際 ZIP／付費 API 驗收。資源最新 C 約6.86GB／D4.15GB，未刪檔，不能沿用下文過時不足結論。詳細來源、保護界線及後續在 [HANDOFF](HANDOFF.md)。

以下保留先前 checkpoint；時態與 frontier 以本段及 live issues 為準。

最新 #25 第一階段已實作／審查／合入：source `d431552` → integration `ec37ab6`；[P25-errors-v1](https://github.com/swchen44/notype/issues/25#issuecomment-6403957943) 已獲 [Astra APPROVED0](https://github.com/swchen44/notype/issues/25#issuecomment-6403993599)，context 優先序 [fix-v2](https://github.com/swchen44/notype/issues/25#issuecomment-6404301764) 亦已 [核准](https://github.com/swchen44/notype/issues/25#issuecomment-6404307928)。最終獨立 Spec／Standards 皆 0，[完整 Linux CI 40240970555](https://github.com/swchen44/notype/actions/runs/40240970555) 在精確 `d431552` 通過；見 [錯誤處理 receipt](docs/verification/provider-errors.md)。這是安全分類／HTTP／SQLite 歷史與本地化訊息切片，不是多 Provider 設定或真實 API 驗收。

後續決策已確認：多 connection／一份 active（含 Custom／Local），App 非空 key 優先、home `~/.env` 後備；三個預期變數僅 names-only 確認、未輸出值。Gemini／OpenRouter UI model ID 留白手填，View／Copy 後手動貼上；實驗 ID 不當 UI default。Thinking 預設 OFF，mandatory 模型提示後使用已知最小值並分記 requested／effective。DeepSeek Flash4.1 本輪總預算 US$2；其他兩家 free-only。[config-v2](https://github.com/swchen44/notype/issues/25#issuecomment-6404377843) 已獲 [Astra APPROVED0](https://github.com/swchen44/notype/issues/25#issuecomment-6404401509)，從第一階段合入 gate 接續；尚無第二階段完成證據。重要 debug／troubleshooting 日誌與手動 GitHub issue 附檔沿 P20 安全白名單另案規劃，預設 ON／可關閉清除／預覽匯出，保持 secrets／原稿／raw body 不入診斷，歷史計量獨立。source41 舊 helper 不當成新需求的可用入口。沒有付費呼叫。

更新日期：2026-10-26。這份文件是接手順序與狀態快照；live [GitHub Issues](https://github.com/swchen44/notype/issues) 是工作追蹤入口。先讀 [HANDOFF.md](HANDOFF.md)。

先前來源整合 `af4ce80a315de025c0d8001e71081c3794c8fea2` 已包含 #23 完整 source `41cb62d`，最終獨立 Spec／Standards 皆 0；[Linux CI 40224287522](https://github.com/swchen44/notype/actions/runs/40224287522) 於精確 `41cb62d` 通過。bridge 6、shared console 7、core 43＋doc 1 等 portable checks 通過；原生 Windows owned-console direct-rustc driver 2 tests／9 cases 通過，見 [receipt](docs/verification/deepseek-native-console.md)。完整 Windows helper 的精確來源啟用建置、固定參數拒絕、無 key console gate smoke 與 dated 私有 package 證據見 [helper receipt](docs/verification/deepseek-reviewed-helper.md)；這不是新 #25 需求或真實 API／費用／品質／Tauri App 驗收。未輸出 key 值，沒有付費呼叫。

#22 backend／#24 UI 及 readiness／owner-release 修正已合入，frontend-only #24 已結案；Windows core 43＋doc 1、provider 20／workflow 15／DTO 1、UI 36＋1、Clippy 及對應獨立來源雙軸 0。Linux CI 40216512564 驗證產品來源 `fb4ab689`，文件-only receipt `8214d317` 的產品／測試來源相同。#22 的 native bindings／capture fixture／實際匯出與新版 App acceptance 尚未完成，不因來源／fixture 通過而結案。

目前 frontier 是完成 #25 後續多連線設定／thinking／key fallback／model list／參數與安全 debug 切片；依精確核准計畫做 owned-fixture unit／integration，不讀取或發送真實 key，不因使用者離線重問已確認決策。新的模型／key 來源／本輪 US$2 實測仍需新 source／artifact／live gate；舊 source41 入口不直接啟動。Gemini 免費計費資格未證明時不送出 live 請求。實際模型／tokens／文字品質／同 ID CSV 核對仍未做；AvailableSpace 實際 quota 查詢亦未由先前 smoke 證明。這不是公司 API 驗收，也不是新版 App 交付。新版 native App `0.1.1` 尚未建置／安裝；磁碟不足以符合核准起始門檻，沒有新的本機 Cargo 執行。儲存空間處理尚待核准實作；不降低 floor、不刪個人檔案。誤 profile 快取與自動清理拒絕的證據照實保留，零檔案被刪，不重試被拒絕的刪除；累積 baseline／ledger 不重設。

所有未完項目仍保留於 [主規格 #8](https://github.com/swchen44/notype/issues/8)，包含 Tauri build／bindings／Clippy／recording fixture、新版乾淨安裝與重設路徑、右 Alt readiness／tray／diagnostics／秒數 overlay 真機驗收、繁簡中英人聲品質／完整 session p95、公司 API／Proxy、五個 App、完整 npm 配送／公開 notices／發布。公司／五 App 依使用者範圍延後；macOS／Linux 僅架構預留。所有新實作仍須 [GPT-6-Astra 計畫 gate](docs/agents/plan-review.md)。現在不移除使用者現用 App 或資料；舊版相容已由使用者取消，之後使用 fresh schema4 與版本化資料目錄。

## 已完成

- [x] Typeless macOS 發行套件與 8 個 GitHub 候選研究；Handy 固定版本 source／payload／文字歷史 audit。
- [x] Q1～Q15 產品決策收斂；[#1](https://github.com/swchen44/notype/issues/1) 已結案。
- [x] AGENTS、glossary、兩份 ADR、unit／integration 測試目錄規範與 GitHub tracker。
- [x] 使用者選定自己的 Windows 11 開發；將交接說明與可貼用 prompt 保存於 repo。
- [x] 依 to-spec 發布 [#8 MVP 與跨平台核心規格](https://github.com/swchen44/notype/issues/8)，使用者已確認主要測試 seam，標記 ready-for-agent；新增 [ADR 0003](docs/adr/0003-shared-core-platform-adapters.md)，先交付 Windows amd64，架構預留 macOS／Linux。
- [x] 將繁體／簡體中文介面、語言偏好保存及與 STT／文字輸出設定分離加入 #8；實作與介面驗收仍待完成。API key 於真正需要時再通知使用者。
- [x] 將「繁體中文＋英文／簡體中文＋英文」兩種本機辨識模式與獨立品質驗收加入 #8、#3；正式實作及模型驗收仍待完成。

已完成 Windows 環境、開發工具、正式桌面／共用核心／文字歷史與三場景提示詞實作；#9／#10／#11／#17 已依各自驗證結案。繁簡介面、兩種混語辨識與快捷鍵的功能已合入；人聲／實體操作、其他 App、自動配送更新與公司環境仍有未驗收項目。原始 Node 英文 smoke 保留於 [開發驗證紀錄](docs/verification/windows11-preflight.md)，不代替正式 App 的後續結果。

## implement-spec 工作圖

整合分支：`implement/notype-mvp`，起點 `be6f0a5`。每個實作 ticket 使用獨立 branch／worktree 與已確認的 public seams 做 TDD，合入整合分支後再啟動被解鎖的工作；最後對起點到整合分支的完整 diff 做 Standards／Spec review。本輪不建立 PR，不把公司／目標環境尚未執行的驗收當作完成。

正式程式及兩軸 review 修正已整合。Release 來源 `239d6f0`；`a2d2328` 只追加 Rustfmt 換行，其 [Linux CI](https://github.com/swchen44/notype/actions/runs/40162614988) 通過核心 27、桌面 34（debug）、平台 4、Node 23 unit +16 integration、fmt／clippy。Windows Release 桌面契約 33 項通過；繁簡各 570 keys、前端 lint/build 通過。[審查](docs/verification/local-code-review.md) 及 [修正](docs/verification/local-review-fixes.md) 保留原 findings、RED 與兩次限定複核的結果。Qwen 6 cases／12 次推論與 [人工原生 suite 的 1 pass／2 Pending failures](docs/verification/guarded-input.md) 依原 receipt 保留，未聲稱人聲、五個 App 或 p95 通過。

使用者已確認本輪先交付錄音／繁簡中英本機辨識／本機歷史／複製；其他 App 自動輸入後補。目前不在公司網路，公司 API／Proxy／真實文字品質延後，不需要 key。正式 Release／helper 與三個完整私有 tarball 已建置核對；實際離線安裝、兩模式 CPU、固定 runtime GUI、資料庫重開及限定 facade 更新均已通過，[試用入口與說明](docs/local-trial.md) 已備妥。首次安裝的暫存空間失敗與 no-tray fixture 可見視窗失敗依實際結果保留；完整 #8／#14／#16 仍未結案。

| Ticket | 狀態 | 原相依 | 可展示／驗收成果 |
| --- | --- | --- | --- |
| [#9 桌面基線](https://github.com/swchen44/notype/issues/9) | closed | 無 | 固定 source／notype identity、啟動／設定重啟、外部資料目錄及停用 installer updater／自動 Enter |
| [#10 共用 Rust 核心](https://github.com/swchen44/notype/issues/10) | closed | 無 | public session、真實 HTTP／SQLite、migration／owner lock／重啟／取消／交付；Windows／Linux contracts |
| [#11 桌面文字流程](https://github.com/swchen44/notype/issues/11) | closed | #9、#10 | 原稿先存、錯誤待選、可見歷史 UI／搜尋／複製／刪除與原稿重試 |
| [#12 語言完整切片](https://github.com/swchen44/notype/issues/12) | open，功能已合入 | #11 | 繁簡介面／中英辨識／獨立偏好／固定音訊已有證據；人聲、整體品質／效能仍未通過 |
| [#17 三場景與提示詞](https://github.com/swchen44/notype/issues/17) | closed | #11 | 三場景／獨立提示詞／快照／翻譯設定／預設關閉的回譯／精確版本確認；真實公司品質屬 #15 |
| [#13 錄音與原生交付](https://github.com/swchen44/notype/issues/13) | open，功能已合入 | #11、#17 | 點按開始／獨立停止／Esc、原目標保護及 App 名稱；人工 fixture 結果保留，其他 App 自動輸入下階段 |
| [#14 npm portable](https://github.com/swchen44/notype/issues/14) | open，本機包與限定 proof 通過 | #12、#13、#17 | 實際 tarball／安裝／兩模式 CPU／GUI／歷史與 facade 更新已通過；完整 native 更新／中斷／鎖檔及公開／Proxy 尚未驗收 |
| [#15 公司 API 驗收](https://github.com/swchen44/notype/issues/15) | needs-info，使用者延後 | #11、#17；公司環境 | 已知 Chat Completions 格式；真實 API／認證／政策與三場景品質待公司網路可用 |
| [#16 目標環境驗收](https://github.com/swchen44/notype/issues/16) | ready-for-human，後續 | #14、#15；App／Proxy | 五個 App、公司 Proxy、人聲／p95、完整更新驗收 |

GitHub 已建立 #8 的原生 sub-issue 關係及 blocking edges。原 #2～#7 保留為對應主題的驗收追蹤，不因新增實作切片而提前結案。

## 接續順序

本機 Starter 與 [試用說明](docs/local-trial.md) 已交付入口，程式／試用資料位於建置快取外且互相分開；接續實際麥克風／實體快捷鍵與繁簡介面回饋。公司與五個 App 依使用者最新範圍延後。以下 #2～#7 是原始主題驗收對應，不能當作從環境安裝／選型重新開始的指示。

| 順序 | Issue／狀態 | 下一個可核對的產物 |
| --- | --- | --- |
| 0 | [#8 MVP／跨平台架構規格](https://github.com/swchen44/notype/issues/8)，ready-for-agent | 先收斂共用核心與平台 adapter 的 interface／contract tests，隔離 OS 路徑及 native 型別，再接 Windows 正式流程；macOS／Linux 實作不在本期 |
| 1 | [#2 Windows portable 基線](https://github.com/swchen44/notype/issues/2)，open | Windows 11 實際架構／工具／runtime preflight、固定來源 manifest、無 installer 的啟動 receipt |
| 2 | [#3 STT](https://github.com/swchen44/notype/issues/3)，open | 固定非機密音訊及模型，CPU 可用；品質與冷／暖延遲分開量測 |
| 2 | [#4 文字 LLM](https://github.com/swchen44/notype/issues/4)，open | adapter contract、成功／timeout／取消／錯誤待選測試；公司 API 未就緒時先用本機測試 server |
| 2 | [#7 文字歷史](https://github.com/swchen44/notype/issues/7)，open | 原稿／整理結果／時間／狀態持久化，無 WAV 也可保存、重啟與重試 LLM |
| 3 | [#5 五個 App 送字](https://github.com/swchen44/notype/issues/5)，open | Browser、VS Code、Teams、Outlook、Windows Terminal 的焦點、IME、只貼一次與手動 Enter receipt |
| 4 | [#6 npm 配送](https://github.com/swchen44/notype/issues/6)，open | 實際 tarball manifest／hash、完整 App／DLL／runtime／模型、Proxy 冷啟動與升級後資料保留 |

先把 #3／#4／#7 的最小行為串成一條可展示流程，再補各自完整驗收；上表的順序不是已驗證的正式 blocking graph。Native build 工具、runtime 或公司 API 缺少時，記下具體阻塞與能獨立繼續的工作，不把整個專案停在重新訪談。

## 完成一個切片的條件

- [ ] Issue 有明確範圍、驗收、相依及測試方式，成果連回 commit。
- [ ] `tests/unittest/`、`tests/integration/` 有對應可執行測試與分開的命令；回報實際通過／失敗／未執行數。
- [ ] 需要 Windows、模型、公司 API 或真實 App 的驗收有獨立 receipt，列出命令、版本、fixture、結果與限制。
- [ ] 更新此檔與相關 issue；只有實際達成驗收才結案。

環境查核與文字核心測試見 `docs/verification/windows11-preflight.md`；後續 receipt 也放在該資料夾。文件驗證結果見 [research receipt](docs/research/evidence/verification-receipt.json)，它不代表產品測試通過。
