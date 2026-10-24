# notype 開發待辦

更新日期：2026-10-25。這份文件是接手順序與狀態快照；live [GitHub Issues](https://github.com/swchen44/notype/issues) 是工作追蹤入口。先讀 [HANDOFF.md](HANDOFF.md)。

## 已完成

- [x] Typeless macOS 發行套件與 8 個 GitHub 候選研究；Handy 固定版本 source／payload／文字歷史 audit。
- [x] Q1～Q15 產品決策收斂；[#1](https://github.com/swchen44/notype/issues/1) 已結案。
- [x] AGENTS、glossary、兩份 ADR、unit／integration 測試目錄規範與 GitHub tracker。
- [x] 使用者選定自己的 Windows 11 開發；將交接說明與可貼用 prompt 保存於 repo。
- [x] 依 to-spec 發布 [#8 MVP 與跨平台核心規格](https://github.com/swchen44/notype/issues/8)，使用者已確認主要測試 seam，標記 ready-for-agent；新增 [ADR 0003](docs/adr/0003-shared-core-platform-adapters.md)，先交付 Windows amd64，架構預留 macOS／Linux。
- [x] 將繁體／簡體中文介面、語言偏好保存及與 STT／文字輸出設定分離加入 #8；實作與介面驗收仍待完成。API key 於真正需要時再通知使用者。
- [x] 將「繁體中文＋英文／簡體中文＋英文」兩種本機辨識模式與獨立品質驗收加入 #8、#3；正式實作及模型驗收仍待完成。

已完成 Windows 環境查核、開發工具安裝及 Rust／C++ 編譯驗證。產品名稱及 private repo 已改為 notype。文字核心與固定英文合成 WAV→CPU STT→HTTP fixture→SQLite→Edge 文字框已接通（11 unit、5 integration、1 runtime smoke 通過）。麥克風熱鍵、繁中品質、原生跨 App 送字、公司 API 與完整 portable 產品仍未完成。見 [開發驗證紀錄](docs/verification/windows11-preflight.md)。

## implement-spec 工作圖

整合分支：`implement/notype-mvp`，起點 `be6f0a5`。每個實作 ticket 使用獨立 branch／worktree 與已確認的 public seams 做 TDD，合入整合分支後再啟動被解鎖的工作；最後對起點到整合分支的完整 diff 做 Standards／Spec review。本輪不建立 PR，不把公司／目標環境尚未執行的驗收當作完成。

| Ticket | 阻擋來源 | 可展示／驗收成果 |
| --- | --- | --- |
| [#9 桌面基線](https://github.com/swchen44/notype/issues/9) | 無 | 固定 Handy source、notype 啟動、設定保存、外部資料目錄、CPU-only no-bundle 與停用 installer updater／自動 Enter |
| [#10 共用 Rust 核心](https://github.com/swchen44/notype/issues/10) | 無 | 原稿到 HTTP fixture、SQLite 歷史、重啟／重試／取消與受保護交付；無 Tauri／Windows 相依 |
| [#11 桌面文字流程](https://github.com/swchen44/notype/issues/11) | #9、#10 | 正式錄音／STT 接核心，失敗待選、歷史搜尋／複製／刪除與無 WAV 重試 |
| [#12 語言完整切片](https://github.com/swchen44/notype/issues/12) | #11 | 繁／簡介面與繁／簡中英辨識，獨立設定、偏好保存、固定模型／音訊驗收 |
| [#13 錄音與原生交付](https://github.com/swchen44/notype/issues/13) | #11 | 熱鍵／Esc、當次音訊清理、Windows 原目標保護、去重與手動 Enter |
| [#14 npm portable](https://github.com/swchen44/notype/issues/14) | #12、#13 | 完整預編譯 App／runtime／模型 tarball、cold launch、更新資料保留 |
| [#15 公司 API 驗收](https://github.com/swchen44/notype/issues/15) | #11；公司 contract／連線／憑證 | 真實文字整理與錯誤／取消 receipt，目前 needs-info；真正需要 key 時才詢問 |
| [#16 目標環境驗收](https://github.com/swchen44/notype/issues/16) | #14、#15；目標 App／帳號／Proxy | 五個 App、公司 Proxy、兩模式品質與 p95、更新資料保留；尚缺環境時保持未完成 |

GitHub 已建立 #8 的原生 sub-issue 關係及 blocking edges。原 #2～#7 保留為對應主題的驗收追蹤，不因新增實作切片而提前結案。

## 接續順序

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
