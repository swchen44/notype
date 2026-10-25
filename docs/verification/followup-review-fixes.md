# Follow-up Standards fixes and P20-v3 verification

工作對應 [#18](https://github.com/swchen44/notype/issues/18)／[#20](https://github.com/swchen44/notype/issues/20)。實作依 [P20-v3 確切計畫](https://github.com/swchen44/notype/issues/20#issuecomment-6398232178)，[GPT-6-Astra 核准，0 項未解問題](https://github.com/swchen44/notype/issues/20#issuecomment-6398249625)。保留前版 [P20-v2 的四項修訂要求](https://github.com/swchen44/notype/issues/20#issuecomment-6398043879) 作為下列驗收來源。

來源：首輪整合 `735b571`；pre-gate 修正保留於 `a66efd8`，主要 P20-v3 實作 `5495307`。完成後以 `faee6f33985ff161760f1c8556fc94d6e1bcc422` 合入 parent 的 `35721c029d425fff4d90f16075f2bcc8c7e1c1c6`，保留 P14 和 P21。只在 CI 的 target 列表有文字衝突，已同時保留 diagnostic、model-storage 和 overlay-lifecycle targets。此 receipt 與 README 的後續 commit 只補測試／文件／格式，產品語意不變。

## 原始四項 Standards findings

原唯讀 review：Spec 0；Standards 4。以下為原 finding 與對應修正，不能把計畫核准或本機測試視為最後 source/native review 已通過。

| Finding | 原問題                                                     | 實作與公開邊界證據                                                                                                                                                                                                                                                                                |
| ------- | ---------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| S1 / P2 | ModelCard 新 Qwen receipt `openUrl` 沒有 catch／使用者回饋 | 實際點擊連結、拒絕公開 opener IPC：缺少 alert 時 RED；加入繁簡／英文錯誤與重試清除後 GREEN；沒有 unhandled rejection 或原始錯誤內容。                                                                                                                                                             |
| S2 / P2 | 唯讀 model-path 新行為只有 UI 假路徑，未執行真正 resolver  | 抽出 production `ModelStorage` 與 `ModelPathAccess`。Manager 實作 metadata/cache snapshot 與 reconciliation hooks；native callers 直接用同一 trait 方法，沒有另一個未測試的 display/activation wrapper。5 個自有檔案測試，含可實際觸及的 catalog/selection 狀態與 activation 正向 callback 對照。 |
| S3 / P2 | 診斷 UI 沒有 rendered preview/export/取消／過期／失敗測試  | 真實 DiagnosticsSupport、React、i18next、Vite、headless Edge，僅 mock 公開 Tauri IPC。7 cases 包含精確預覽／ID、取消、過期與保存失敗、重新預覽、opener 失敗／重試、Clear 與晚回覆、初始 unknown disabled、鍵盤 OFF unsaved／重試、舊狀態回覆。                                                    |
| S4 / P3 | 新診斷公開 API 沒有 doc comments                           | enum／labels、metadata fields、report snapshot／ID、constructor／getter、global collector facade、service tickets／status／鎖與生命周期、storage traits、native IPC DTOs／commands 均補目的、所有權與隱私契約；見 `desktop/CONTRIBUTING.md` 的 public API 規則。                                  |

## P20-R1：清除與匯出

共用 service 取代分離的 EVENTS/PREVIEW。`state` 只做短暫的 bounded collector／metadata 操作；獨立 `export_commit` gate 統一 preview publication、Clear 及最終 export。Context gathering 與原生 save dialog 不持此 gate，任何檔案 I/O 都不持 collector lock。

Clear-first：epoch／snapshot ID 失效後，對話框回傳的舊 export 在建立暫存／目的檔前即拒絕。Export-first：最終 identity 驗證與 atomic staging／flush／rename／失敗清理完成前，Clear 不能回覆成功；先完成的手動匯出保留。Clear 只重設結構化 events／discard count／time origin／preview，偏好、原稿、模型與舊手動匯出不刪除。服務測試使用 barrier 明確固定順序、真實檔案 sentinel，以及實際 rename 到非空目錄的失敗與暫存清理；在 writer 被阻塞時，record 與有效 OFF 仍可完成。

前端 Clear 一開始撤下預覽並增加 request generation；晚到的 preview/export/opener 回覆不能恢復舊文字或蓋掉 Clear 訊息。清除完成前顯示 waiting；不以「清除 PREVIEW 變數」誤稱已撤銷進行中的檔案寫入。

## P20-R2：白名單與遮罩的範圍

`record` 仍只接 enum 與有界數字；report renderer 的 credential-free metadata 欄位集合不擴張。`ReportContext.version` 是既有唯一文字輸入，先經固定欄位 sanitizer，非法內容改為 `unavailable`；再建立不可改寫的 snapshot/cache。匯出只接受 snapshot ID，不接受 UI 提供的正文。

防禦 helper 的已辨識 synthetic LiteralOpaque key/token：長度至少 12，只保留頭尾各 2，中間固定 `********`，例如 `abcdefghijkl` → `ab********kl`；0–11 字元完全 `********`。Authorization／Bearer、JWT／query／URL、Unicode、編碼或不明來源、格式不合法一律全遮蔽；未知欄位省略，沒有解碼後保留原值的 fallback；canonical mask 可重複處理而不改變。欄位名不分 ASCII 大小寫並正規化 space／hyphen／underscore。**這些 sensitive synthetic 欄位即使已遮罩也不准進正式 report allowlist**；沒有為了測遮罩新增憑證讀取或任意 log ingestion API。

Helper unit tests 證明部分遮罩契約；公開 service preview＋真實 export tests 證明更強的 credential／原稿／URL／本機路徑完全排除。UI 明示 switch／Clear／mask 僅適用於 structured diagnostics，既有 raw log 獨立且可能含私人資訊，不會匯入或自動分享。

## P20-R3：預設與保存失敗

新 service 起始 Uninitialized／有效 OFF，提前的 startup record 被丟棄。Native setup 先讀專用 `diagnostics-preferences.json`（最多 4 KiB），才保留第一筆 Startup。檔案／key 缺值才 default ON；明確 false 在 startup 前生效；錯誤、損壞、oversized、unsupported schema、非 boolean 都 fail-closed OFF，不走一般 AppSettings salvage/default-true。沒有從 provider settings 讀取新資料。

`preference_update` gate 序列化 load／toggle／explicit atomic save。OFF 在 I/O 前即生效，保存失敗維持 OFF 並回 Unsaved；ON 成功保存後才啟用，失敗保持 OFF。UI 的 checkbox、pending／load_failed／unsaved 訊息與獨立 retry-saving action 不會把有效 OFF 宣稱為已保存 OFF；明示未保存時重新啟動可能用先前偏好。Revision／request sequence 阻止較舊 status 回覆覆蓋新 toggle。Clear 不改偏好或保存狀態。

自有偏好檔測試含舊 profile 缺值、explicit false、壞／讀取失敗／過大、保存失敗、重試、兩種 rapid-toggle 順序及真實 restart 載入結果；沒有讀寫使用者 profile。

## P20-R4：真正 Manager 決策路徑

`ModelManager` 不再另有選模式的 inherent wrapper。`ModelPathAccess` 的 `get_model_path_for_display` 與 `get_model_path` default method 是 native callers 和 fixture host 執行的同一份程式。Host 的 reconcile callback 是 query 真正能觸及的 catalog 邊界；activation missing 正向對照會呼叫一次並改 availability，同時保留 selection。Display missing 不呼叫 callback，不改 catalog/selection；HF mirror display 保留 stale partial，activation 才清理。

保留 HF cache-resolved snapshot 優先／本機 mirror fallback、custom file/directory／partial／missing／downloading cases。這驗證傳入既有 hf-hub adapter 已解析的路徑後，正式 resolver 如何選擇與處理實際檔案；不宣稱已測 hf-hub 的環境變數／revision discovery 或已啟動完整 Tauri Manager。原本與 resolver 無關的 catalog.json/settings.json 不變斷言已移除。

## RED → GREEN 證據

- Receipt opener：有效的已存在按鈕點擊後，缺少 alert 的 timeout 是 RED；補 catch／翻譯／retry 後 GREEN。第一次誤寫的按鈕 locator 不算 regression RED。
- Service／privacy／file adapter 新 seam 的首個測試先因缺 module/API 而 RED，實作後 GREEN；collection checkbox 的實際 rendered 初始 disabled case 在元件沒有控制項時 RED，實作後 GREEN。
- `display_query_never_activates_or_reconciles_catalog`：顯示精確原始碼後，把實際 display trait method 暫時改呼叫 activation；1 case RED（partial 被刪），立即還原，1 case GREEN。
- `clear_first_rejects_dialog_return_without_touching_destination`：暫時移除 Clear 的 epoch／preview 撤銷，1 case RED（舊 export 得到 Ok 並能寫入）；立即還原後 1 case GREEN。
- Rendered `support previews exact text and export cancellation sends only its snapshot ID`：暫時把送出 ID 改為 `wrong-preview-id`，1 case RED（公開 IPC payload 不符）；立即還原後 GREEN。
- 所有 mutation 均用 source 證明命中、`finally` cleanup 還原；沒有 mutation 留在可建置內容。早期 partial test-name 配合 `--exact` 得到 0 tests 的執行不算證據，也未記成通過。

## 執行命令與結果

編譯器：`rustc 1.99.1 (58cf8f9c3 2026-10-13)`。沒有執行 Cargo／native build 或改寫 native target。Standalone binaries 僅寫至 task-owned TEMP 名稱；serde_json 與其 dependency directory 唯讀重用既有 rlib。

| Scope                   | Root test path                                         | Result                                                                 |
| ----------------------- | ------------------------------------------------------ | ---------------------------------------------------------------------- |
| Rust unit               | `tests/unittest/desktop_diagnostics.rs`                | 2 passed                                                               |
| Rust unit               | `tests/unittest/diagnostic_privacy.rs`                 | 2 passed                                                               |
| Rust unit               | `tests/unittest/diagnostic_service.rs`                 | 2 passed                                                               |
| Rust integration        | `tests/integration/desktop_diagnostics.rs`             | 1 passed                                                               |
| Rust integration        | `tests/integration/model_storage.rs`                   | 5 passed                                                               |
| Rust integration        | `tests/integration/diagnostic_control.rs`              | 8 passed                                                               |
| Node unit               | `node --test tests/unittest/diagnostic-issue.test.mjs` | 1 passed                                                               |
| Frontend UI unit        | `npm.cmd run test:ui:unit`                             | 1 passed                                                               |
| Frontend UI integration | `npm.cmd run test:ui:integration`                      | 19 passed after parent merge: 7 diagnostics + 5 model + 7 retained P21 |

Rust 合計 **20 passed / 0 failed**。Clear 另覆蓋 uninitialized／default／saved／load_failed／unsaved 與保存中的 pending 偏好保留；只針對新增斷言重跑 service/control cases，其餘 broad suites 在合入 parent 後通過後未重複執行。每個無外部依賴的 Rust path 用以下格式編譯後，僅在編譯 exit 0 才執行對應 exe：

```powershell
rustc --edition=2021 -A dead_code --test <root-test-path> -o <task-owned-TEMP-exe>
```

實際 production file-adapter/control 測試的依賴命令：

```powershell
rustc --edition=2021 -A dead_code --test tests/integration/diagnostic_control.rs -o "$env:TEMP/notype-diagnostic-control.exe" --extern serde_json=D:/git/notype-build/release/deps/libserde_json-92987db7904ad379.rlib -L dependency=D:/git/notype-build/release/deps
if ($LASTEXITCODE -eq 0) { & "$env:TEMP/notype-diagnostic-control.exe" }
```

既有 desktop-contracts Cargo manifest／Linux CI 註冊所有新 targets；本機 standalone reuse 不表示已跑該次 CI。前端 lint／build 在合入 parent 後通過；Vite chunk-size 和 baseline-browser-mapping 的既有提示不影響 verdict。變更前端／文件用 Prettier，Rust 用 rustfmt；`git diff --check` 無錯。

**未執行／限制**：native Tauri／C++ compile、Clippy、bindings regeneration、原生 save dialog／退出、使用者 App／process／profile／mic／clipboard、真實 STT、公司 API、套件更新與實際交付。這些由 parent 依後續 source review／核准計畫分開驗證，不能用本檔 unit／browser cases 代替。原 0-byte log 原因與 Right Alt 短按後等待 READY 的真機原因均仍未確認，沒有宣稱已修好。

## 補充審查 S5／S6：操作測試與 overlay API 文件

2026-10-25，獨立來源審查在 `ebca453dc58b194c18c692861a0ff1e7a039aeb5`（與 integration `160ea647c3e4dfa9fa3cc5b8179b2c3a08a0a55d` 同 tree）確認原四項 findings 已修復，另列兩項 Standards findings；Spec 為 0。精確補充計畫 [P20/P21-review-v1](https://github.com/swchen44/notype/issues/20#issuecomment-6398809381) 先取得 [GPT-6-Astra APPROVED，0 項未解問題](https://github.com/swchen44/notype/issues/20#issuecomment-6398823972)，才開始本節測試／註解修改。本節不取代前面的 P20-v3 驗證紀錄。

- **S5／P2 原 finding**：新增的 About Quit 和修改過的 log-level 操作缺少可執行的實際互動覆蓋；既有 support suite 雖掛載 selector，沒有操作其 dropdown，也未掛載 AboutSettings。
- **S5 修正**：`tests/integration/about-settings.ui.mjs` 掛載正式 AboutSettings／LogLevelSelector，使用真實 React／i18next／Dropdown／useSettings／settingsStore／process plugin。僅替換公開 Tauri IPC transport，沒有替換內部 hook、初始化、refresh 或儲存方法。Quit 的成功只回覆合成 transport，不關閉任何 App。
- **S6／P3 原 finding**：新 overlay 公開 policy／DTO 不符 CONTRIBUTING 的 doc-comment 規範，外部事件契約文件無法取代 API 註解。
- **S6 修正**：補上 Presentation／helpers／variants、HideReason labels、CaptureId／OverlayTicket fields、OverlayLifecycle constructor／reserve／commit／ready／text、ready／hide DTO 與相關 show helpers 的文件。註解明確區分 capture identity 與 presentation revision；ready 必須 exact current capturing ticket，text 可延續至同 capture 的非 Hidden working 階段。Adapter 對 reserve 使用同一 registry lock，執行時重新取得並持有至 check／synchronous commit 完成。沒有修改任何產品判斷、event shape、binding 或 timing instrumentation。

新增三個具名 rendered cases：

1. `About Quit sends exit zero, reports translated rejection and allows retry`：實際按鈕送出 `plugin:process|exit`／`{code: 0}`；拒絕顯示翻譯後 status，沒有 raw Bearer／URL／私人路徑 sentinel，仍可再次按下重試。
2. `log level waits for settings and save, then displays the refreshed server acknowledgement`：未載入 settings 時 disabled；選 Trace 送 `set_log_level`／`{level: "trace"}`；保存與後續 settings refresh 的兩段等待期間皆 disabled。Server 確認值刻意為 Warn，畫面透過正式 `get_app_settings` refresh 後顯示 Warn，不能以本地 optimistic Trace 冒充成功。
3. `log level rejection is translated, preserves the acknowledged value and allows retry`：拒絕保留 Debug、解除 disabled、顯示翻譯後回饋且不漏原始錯誤；選 Info 重試後顯示已確認 Info，清除錯誤。每個 fixture 皆斷言沒有 browser pageerror／unhandled rejection。

### 補充 named mutation RED → restore GREEN

先確認既有正確行為通過，再依核准計畫對實際產品 source 做暫時 fault injection。每次先保存原始 bytes，顯示 `git diff -- <source>` 證明命中，在 PowerShell `try` 中執行下列精確測試，`finally` 使用 `WriteAllBytes` 還原原始 source；隨即執行同一測試確認 GREEN。沒有把錯 locator、timeout 或 zero tests 當作此輪 mutation 證據。

```diff
--- desktop/src/components/settings/about/AboutSettings.tsx
-      await exit(0);
+      await exit(1);
--- desktop/src/components/settings/debug/LogLevelSelector.tsx
-      await invoke<void>("set_log_level", { level: value as LogLevel });
+      await invoke<void>("set_log_level", { level: "error" as LogLevel });
```

```powershell
node --test --test-name-pattern='About Quit' tests/integration/about-settings.ui.mjs
node --test --test-name-pattern='log level waits' tests/integration/about-settings.ui.mjs
```

第一個 mutation：**1 failed／0 passed**，公開 transport assertion 顯示 actual code 1／expected 0；還原後 **1 passed／0 failed**。第二個 mutation：**1 failed／0 passed**，actual level error／expected trace；還原後 **1 passed／0 failed**。最終這兩個產品 TSX 檔均無 diff。

### 補充執行結果

- `node --test tests/integration/about-settings.ui.mjs`：**3 passed／0 failed**。
- `npm.cmd run test:ui:integration`：**22 passed／0 failed／0 skipped**（新增 3、diagnostics 7、model 5、保留 P21 overlay 7）。
- 從 `desktop/` 執行 `npm.cmd run lint`、`npm.cmd run build`：通過；保留既有 Vite chunk-size／baseline-browser-mapping 提示，沒有新增依賴。
- `node desktop/node_modules/prettier/bin/prettier.cjs --check tests/integration/about-settings.ui.mjs tests/README.md docs/verification/followup-review-fixes.md`、`rustfmt --edition 2021 --check desktop/src-tauri/src/overlay_lifecycle.rs desktop/src-tauri/src/overlay.rs`、`git diff --check`：通過。

這一輪僅測試／API 註解／receipt；沒有 Cargo、native cache、App、原生退出或 logger/runtime 驗收、使用者 profile／mic／clipboard、歷史資料／付費 API／#22／#23 實作。P19 的另行核准 instrumentation 在其他 worktree；本次未修改 diagnostics/service。新增 tests 發現的範圍內沒有需改產品語義的缺陷。
