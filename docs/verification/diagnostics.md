# 安全診斷與回報 #20

實作範圍：About → 診斷與問題回報；production release 也註冊預覽與匯出命令。對應 [GitHub #20](https://github.com/swchen44/notype/issues/20)。本記錄使用合成資料，沒有讀取使用者原稿、API key、endpoint、錄音或原始 log 內容。

## 空 log 的判讀

本次試用檔案的既有 metadata 為 0 bytes；可安全確認的設定子集為 debug mode 關閉、log level Debug、always-on microphone 關閉。這些資訊不足以重現或斷言原本的錯誤原因。

原始碼檢查結果：Cargo.lock 實際鎖定 tauri-plugin-log 2.8.0；Folder/LogDir target 在初始化時就會 create/append `notype.log`，因此空檔僅能證明 sink 曾被建立。檔案 target 依 `FILE_LOG_LEVEL` atomic 篩選，初始為 Debug，setup 從持久化設定同步；數字與 `log::LevelFilter` 的對映一致。console 的 `RUST_LOG` filter 與 webview 的 debug-mode gate 是獨立 targets，不能單靠「未開 debug mode」解釋檔案為空。檔案大小上限 500,000 bytes、KeepOne rotation；RotatingFile 在 flush 時才寫出暫存資料，fern writer 的每筆記錄會 flush。

仍需在自有 profile 的新版原生 App 分辨三個假設：logger/level 未接受記錄、觀察的 sink 與目前執行實例不同或曾 rotation、App 未走到正常 setup。沒有對使用者正在執行的 App 注入事件或重新啟動，故原始空檔原因記為 **unresolved**，不能解讀成沒有錯誤。本次新增無私人資料的 Info 級固定啟動標記 `notype-startup logger-ready` 與 flush；遵守已同步的 log level。Warn/Error 設定下看不到此 Info 標記是正常情況。

## 使用方式與階段判讀

1. 保持目前資料與設定，在 App 重現一次問題；回到 About 的診斷入口。
2. 按「預覽診斷報告」。事件只保留這次 App 執行的最近 128 筆，`elapsed_ms` 從診斷初始化起算；duration 以 ms 計。重新啟動後不保留舊的事件。
3. 比較 shortcut-dispatch（worker 排隊）、recording-start、audio-stream-open、microphone-ready、model-load、transcription、history-save、text-processing、delivery。Started 之後尚未出現結束事件，只能表示該步驟尚未完成或執行被中斷；不能自行認定是硬體或權限故障。streaming 路徑的 transcription 耗時是停止後 finalize 的耗時，batch 路徑是 batch 辨識耗時。
4. 預覽後，匯出該快照成 .txt 並自行選擇保存位置。之後新事件不會改變已預覽的快照。取消保存不建立報告；過期預覽、無法寫入或副檔名錯誤以翻譯後訊息回報，不呈現 OS 錯誤與完整路徑。
5. 按「開啟 GitHub issue」，自行描述重現步驟，檢查報告後附加 .txt。URL 僅放固定模板、經驗證的版本與診斷 ID；沒有整份報告、log 或設定。App 不保存 GitHub token、不呼叫上傳 API。GitHub 按附件或拖放時會立即上傳，可能要求登入；見 [GitHub 官方附件說明](https://docs.github.com/en/get-started/writing-on-github/working-with-advanced-formatting/attaching-files)。

入口也提供本機 log 等級，方便本機調查；Debug/Trace 的既有原始 log 可能含有私人資料，請留在本機。安全報告不會以 regexp 修改整份原始 log，也不讀取原始 log；它只以型別化 enum、布林值、數字、限制字元的版本及生成 ID 建立白名單報告。沒有 API endpoint/key、模型私有檔名、裝置識別、使用者名稱、絕對路徑、語音、原稿或 LLM 內容的輸入欄位。

About 另提供「結束 notype」，在系統匣失效但視窗保留時仍有可見的退出入口；使用既有 `process:default`（含 `allow-exit`）與翻譯後失敗訊息。

## RED → GREEN 與實際驗證

預先確認的 seam 是公開報告預覽／檔案匯出／GitHub 草稿 URL，測試放根目錄。先執行 Rust report test：缺少 `diagnostics.rs` 而 RED；加入型別化報告後該 privacy test GREEN。接著匯出快照測試因缺少公開 `Report::export_to`、`text`、`id` 介面而 RED；實作不可修改的報告快照後 GREEN。URL test 先因缺少公開 helper 而 RED，實作後 GREEN。沒有把無法重現的使用者空 log 宣稱為已重現的 RED。

在 diagnostics worktree 執行：

```powershell
rustc --edition=2021 -A dead_code --test tests/unittest/desktop_diagnostics.rs -o "$env:TEMP/notype-diagnostics-unit.exe"
& "$env:TEMP/notype-diagnostics-unit.exe"
rustc --edition=2021 -A dead_code --test tests/integration/desktop_diagnostics.rs -o "$env:TEMP/notype-diagnostics-export.exe"
& "$env:TEMP/notype-diagnostics-export.exe"
node --test tests/unittest/diagnostic-issue.test.mjs
```

- Rust unit：2 passed / 0 failed。惡意 Bearer/key、URL、Windows/Unix path、使用者名稱、speech／原稿 sentinel 不出現在報告；事件上限 128，超出丟棄計數正確，duration 上限 86,400,000 ms、最壞輸出小於 32 KiB。
- Rust integration：1 passed / 0 failed。實際在自有暫存目錄保存、讀回報告；內容與預覽 byte-for-byte 一致、後續事件不混入、惡意字串與保存路徑不出現在檔案、失敗只回傳固定 code。測試只移除自己建立的單檔與空目錄。
- Node unit：1 passed / 0 failed。公開草稿 URL 帶版本／ID，惡意值與報告內容不進入 URL。
- 前端 `npm.cmd run lint`、`npm.cmd run build` passed；Vite 既有 chunk-size warning 不影響 build。變更前端檔案 Prettier；變更 Rust 檔案 rustfmt。
- Cargo/native compile、Clippy、Windows App save dialog／原生退出、真實快捷鍵／麥克風、STT、公司 API：**此 worktree 未執行**；由 parent 統一使用 native cache 驗證，不能以 std-only tests 代替。

desktop-contracts 註冊 `desktop-diagnostics` 與 `diagnostics-export`，CI 與後續維護可執行；Node test 由既有 root unit command 執行，不依賴 frontend node_modules。input/tray peer 會在整合分支補上實際 shortcut／mic／stop 階段探針；本分支實作模型、batch／stream-finalize 辨識、保存、文字處理與交付探針。
