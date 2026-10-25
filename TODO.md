# notype 開發待辦

更新日期：2026-10-26。這份文件是接手順序與狀態快照；live [GitHub Issues](https://github.com/swchen44/notype/issues) 是工作追蹤入口。先讀 [HANDOFF.md](HANDOFF.md)。

最新來源整合為 `af4ce80a315de025c0d8001e71081c3794c8fea2`。#23 完整 source `41cb62d` 已合入，最終獨立 Spec／Standards 皆 0；[Linux CI 40224287522](https://github.com/swchen44/notype/actions/runs/40224287522) 於精確 `41cb62d` 通過。bridge 6、shared console 7、core 43＋doc 1 等 portable checks 通過；原生 Windows owned-console direct-rustc driver 2 tests／9 cases 通過，見 [receipt](docs/verification/deepseek-native-console.md)。完整 Windows helper 已完成精確來源啟用建置、固定參數拒絕、無 key console gate smoke 與 dated 私有 package，見 [helper receipt](docs/verification/deepseek-reviewed-helper.md)；這不代表真實 API／費用／品質或 Tauri App 驗收。**沒有取得 key，沒有付費呼叫。**

#22 backend／#24 UI 及 readiness／owner-release 修正已合入，frontend-only #24 已結案；Windows core 43＋doc 1、provider 20／workflow 15／DTO 1、UI 36＋1、Clippy 及對應獨立來源雙軸 0。Linux CI 40216512564 驗證產品來源 `fb4ab689`，文件-only receipt `8214d317` 的產品／測試來源相同。#22 的 native bindings／capture fixture／實際匯出與新版 App acceptance 尚未完成，不因來源／fixture 通過而結案。

目前 frontier 是等使用者回覆本機 key 已就緒，再由使用者自行啟動 `D:/git/notype-build/compiler-temp/p23-reviewed-helper-41cb62da/Start-deepseek-probe.cmd`，於專屬非回顯視窗輸入 key，最後輸入 `RUN P23-live-v1` 執行本輪 US$0.10 有限驗收；agent 不替使用者啟動付費輪次。實際模型／tokens／文字品質／同 ID CSV 核對仍未做；AvailableSpace 實際 quota 查詢亦未由本輪 smoke 證明。這不是公司 API 驗收，也不是新版 App 交付。新版 native App `0.1.1` 尚未建置／安裝；D 低於 2 GiB 起始門檻。使用者空間問題（C 至少 500 MB、D 至少 2 GB）待回覆；不降低 floor、不刪個人檔案。誤 profile 快取與自動清理拒絕的證據照實保留，零檔案被刪，不重試被拒絕的刪除；累積 baseline／ledger 不重設。

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
