# 共用核心 Windows／Linux contract 驗證

日期：2026-10-25。整合分支 `implement/notype-mvp`，本次 Linux run 的 commit 為 `c010b7b778e59c867809a64cc755343920eca716`，基於桌面／核心合併點 `036c08cc9259e91d14fc775e36b27056db1ed095`。

## 實際結果

| 環境／命令 | 結果 |
| --- | --- |
| Windows：共用核心 public session | 8 passed、0 failed |
| Windows：SQLite 保存／migration／owner lock／唯讀匯入 | 6 passed、0 failed |
| Windows：實際 loopback HTTP | 3 passed、0 failed |
| Windows：Rust 文件範例 | 1 passed、0 failed |
| Windows：核心 fmt／clippy | passed |
| Linux：相同 8 + 6 + 3 核心 tests 與 1 文件範例 | 18 passed、0 failed、0 ignored |
| Linux：既有 Node diagnostic unit／integration | 11 + 5 passed、0 failed、0 skipped |
| Linux：核心 fmt／clippy | passed |

Windows 結果由 #10 implementer 與 integration merger 取得；桌面基線另有 [原生 runtime receipt](windows-desktop-baseline.md)，不與核心數量混算。Rust library 本身沒有 inline unit tests 的 0-test harness 不作為通過證據，以上數量來自實際外部 test targets 與文件範例。

Linux 結果：[Shared core contracts run 40137924066](https://github.com/swchen44/notype/actions/runs/40137924066)，conclusion `success`。Runner 為 `ubuntu-latest`，Rust 1.99.1、Node 22.15.0；workflow 有 15 分鐘上限、read-only contents 權限，使用固定 commit 的官方 checkout／setup-node actions。

## 命令與 seam

共用核心：`cargo test --locked --manifest-path crates/notype-core/Cargo.toml`；格式與 lint 分別是同一 manifest 的 `cargo fmt -- --check`、`cargo clippy --all-targets -- -D warnings`。CI 明確使用 `+1.99.1` 工具鏈。

測試從 public session／保存／HTTP seams 驗證原稿先保存、模式快照、處理錯誤／timeout／取消、遲到結果、明確選原稿、目標變更／交付去重／結果不明，以及真實 SQLite 重啟、Node v1 migration、互斥擁有權、唯讀 Handy import 與手動刪除。資料路徑由呼叫端提供，核心沒有 Tauri 或 Windows UI 型別；Linux 執行不需要 Windows SDK、桌面 App 或語音模型。

本次驗證證明共用核心能在 Windows 與 Linux 執行相同 contracts。它不代表 macOS／Linux 桌面 adapter、真實 STT 品質、原生五個 App、公司 API、完整 portable payload 或 p95 已驗收；後續核心改動以對應 commit 的新 run 為準。

## #11 合併後補驗

`5ad169845c49881ec7873b694051117eb4cc7372` 新增需要設定的可復原錯誤分支後，Windows merger 再次通過核心 9 unit、6 persistence、3 HTTP 與 1 doc，共 19 項；Node unit／integration 仍為 11 + 5。Linux [run 40143094184](https://github.com/swchen44/notype/actions/runs/40143094184) 同樣全部通過，0 failed／0 ignored，Node 0 skipped；fmt／clippy 亦通過。已檢查實際 logs 的各 test target 數量，空的 inline harness 不計入 19。

此 run 尚不包含後續 #12 辨識模式及 #17 三場景的新修改。桌面 #11 的 7 項 Rust checks 與 5 項 Windows native tests 另記於 [desktop workflow receipt](desktop-workflow.md)，不與此 Linux 核心數量混算。

## 三場景與 App 名稱限定合併後

`af947359b875584751447e5b532a0f630605d86e` 的 Linux [run 40148240213](https://github.com/swchen44/notype/actions/runs/40148240213) 已完成，conclusion `success`。實際 logs 為 9 public unit、17 integration（3 HTTP + 9 persistence + 5 review）及 1 doc，共 27 passed、0 failed、0 ignored；Node 仍為 11 unit + 5 integration，0 failed／0 skipped。核心 fmt／clippy 通過。

後續 workflow 擴充為直接執行 test-only desktop harness 的 `desktop-workflow` 及 `desktop-provider` targets，並加入對應來源／manifest／lock 的觸發路徑。這個擴充的 Linux 結果須以新 run 為準；上述 40148240213 只證明核心與 Node checks。
