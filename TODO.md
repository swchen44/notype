# Windows 開發待辦

更新日期：2026-10-24。這份文件是接手順序與狀態快照；live [GitHub Issues](https://github.com/swchen44/notype_study/issues) 是工作追蹤入口。先讀 [HANDOFF.md](HANDOFF.md)。

## 已完成

- [x] Typeless macOS 發行套件與 8 個 GitHub 候選研究；Handy 固定版本 source／payload／文字歷史 audit。
- [x] Q1～Q15 產品決策收斂；[#1](https://github.com/swchen44/notype_study/issues/1) 已結案。
- [x] AGENTS、glossary、兩份 ADR、unit／integration 測試目錄規範與 GitHub tracker。
- [x] 使用者選定自己的 Windows 11 開發；將交接說明與可貼用 prompt 保存於 repo。

以上完成的是研究與交接。產品程式、可執行產品測試及 Windows 實測尚未開始。

## 接續順序

| 順序 | Issue／狀態 | 下一個可核對的產物 |
| --- | --- | --- |
| 1 | [#2 Windows portable 基線](https://github.com/swchen44/notype_study/issues/2)，open | Windows 11 實際架構／工具／runtime preflight、固定來源 manifest、無 installer 的啟動 receipt |
| 2 | [#3 STT](https://github.com/swchen44/notype_study/issues/3)，open | 固定非機密音訊及模型，CPU 可用；品質與冷／暖延遲分開量測 |
| 2 | [#4 文字 LLM](https://github.com/swchen44/notype_study/issues/4)，open | adapter contract、成功／timeout／取消／錯誤待選測試；公司 API 未就緒時先用本機測試 server |
| 2 | [#7 文字歷史](https://github.com/swchen44/notype_study/issues/7)，open | 原稿／整理結果／時間／狀態持久化，無 WAV 也可保存、重啟與重試 LLM |
| 3 | [#5 五個 App 送字](https://github.com/swchen44/notype_study/issues/5)，open | Browser、VS Code、Teams、Outlook、Windows Terminal 的焦點、IME、只貼一次與手動 Enter receipt |
| 4 | [#6 npm 配送](https://github.com/swchen44/notype_study/issues/6)，open | 實際 tarball manifest／hash、完整 App／DLL／runtime／模型、Proxy 冷啟動與升級後資料保留 |

先把 #3／#4／#7 的最小行為串成一條可展示流程，再補各自完整驗收；上表的順序不是已驗證的正式 blocking graph。Native build 工具、runtime 或公司 API 缺少時，記下具體阻塞與能獨立繼續的工作，不把整個專案停在重新訪談。

## 完成一個切片的條件

- [ ] Issue 有明確範圍、驗收、相依及測試方式，成果連回 commit。
- [ ] `tests/unittest/`、`tests/integration/` 有對應可執行測試與分開的命令；回報實際通過／失敗／未執行數。
- [ ] 需要 Windows、模型、公司 API 或真實 App 的驗收有獨立 receipt，列出命令、版本、fixture、結果與限制。
- [ ] 更新此檔與相關 issue；只有實際達成驗收才結案。

第一份環境查核可存為 `docs/verification/windows11-preflight.md`，後續 receipt 也放在該資料夾；目前尚未建立，因為還沒有 Windows 量測結果。文件驗證結果見 [research receipt](docs/research/evidence/verification-receipt.json)，它不代表產品測試通過。
