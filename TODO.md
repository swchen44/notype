# notype 開發待辦

更新日期：2026-10-25。這份文件是接手順序與狀態快照；live [GitHub Issues](https://github.com/swchen44/notype/issues) 是工作追蹤入口。先讀 [HANDOFF.md](HANDOFF.md)。

目前接續本機試用回饋 [#18](https://github.com/swchen44/notype/issues/18)／[#19](https://github.com/swchen44/notype/issues/19)／[#20](https://github.com/swchen44/notype/issues/20)／[#21](https://github.com/swchen44/notype/issues/21)。新實作先走 [GPT-6-Astra 計畫 gate](docs/agents/plan-review.md)；P20-v2／P21-v2／P19-latency-v2 尚有計畫審查問題，須修訂並複審。使用者最新的數字計時、diagnostics 開關／預設開啟／清除／敏感值遮罩已列入待審範圍，未實作或更新目前正式包。

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
