# 三場景與可編輯提示詞驗證

日期：2026-10-25。Ticket [#17](https://github.com/swchen44/notype/issues/17)，parent [#8](https://github.com/swchen44/notype/issues/8)。實作分支 `ticket/17-scenarios-prompts`，首個產品切片 `64af113`，合併 #12 功能／規格 `fd79093`（整合來源 `9d30ff0`），後續同分支補上最終驗證與收據。這不是公司模型翻譯或回答品質認證。

## 可觀察行為

口述、翻譯、隨便問各有實際使用且可編輯的提示詞。預設包含忠實整理／自然保真翻譯／依口述意圖回答或代寫；公開口述例子放在可見的預設提示詞中。第一個 system 訊息只設 App 文字／工具及資料信任邊界，第二個 system 為所選提示詞；自訂內容不被隱藏預設文風或例子覆蓋。原稿、手動術語、辨識模式、場景、目標語言及允許的 App 名稱以結構化資料提供。#13 負責取得 App 名稱；未知時為空，不讀其他 App 內容。依使用者最新縮減範圍，網站網域及各網站整合不在 MVP。

開始錄音時捕捉設定，後續修改不改變進行中的 session。歷史保存場景及翻譯目標，明確重試採該 entry 的場景／目標／context 與重試當下提示詞。UI 提供單一場景提示詞保存／還原及翻譯目標、手動領域／語氣／目標範圍詞彙對照。還原不觸及其他提示詞、API endpoint／model／認證／key、歷史、辨識偏好或模型。API 表單保存只更新自己的欄位，防止過期表單覆寫提示詞設定。

回譯預設關閉，開啟時第一個 response 在同次請求中回傳主翻譯與來源語言的 transport envelope；UI／交付只有解析後文字。主翻譯在第二次 request 開始前已寫入 SQLite。第二次 request 只含主翻譯、反向語言及對照資料，沒有原稿正文。來源語言缺少／不合法／und 或回譯失敗時保留主翻譯，維持人工檢視待選，不猜來源或另發偵測請求。

啟用回譯後沒有自動交付。後端 `confirm_review(id, revision)` 只確認當次顯示版本，與交付分開；重試／改選原稿會遞增 revision 並撤銷舊確認。舊畫面不能批准新結果。確認後仍須原目標及去重檢查；目前交付 adapter 依 #11 保持 Unavailable，#13 才接入原生實作。主畫面確認不會啟用任意前景輸入位置。取消、task abort／重新開啟、刪除及第二階段遲到結果保留既定保護。辨識字形失敗也不能透過新的場景處理繞過重試限制。

## RED → GREEN 證據

- Core 新 review 公開契約先因缺少 `generate`／`back_translate`／review API 而編譯失敗；完成後通過，含精確 revision、target-changed、防止刪除後復活、第二階段取消／中斷與 SQLite 重開保留主翻譯。
- 三場景／設定快照 HTTP 切片先缺少 helper／欄位而失敗，之後驗證精確自訂提示詞、目標範圍映射及設定保存隔離。
- `und` source metadata 先錯誤啟動反向請求而逾時；修正後保留主翻譯並回報可恢復的 back-translation-failed。缺少及 URL-shaped metadata 也不猜測。
- Recognition failure 公開 core 測試先錯誤產生 Pending 譯文；改用穩定的 recognition_failed 欄位，不能因改選原稿、嘗試交付後 error 變成 target-changed 而繞過限制。
- 兩項新原生測試在既有 #12 binary 實際 RED：找不到場景 UI／`save_translation_settings` command。實作後實際 native binary 兩項 GREEN，包含可見欄位操作、保存、單獨還原、API 參數保留、重新啟動及歷史結果比較／精確確認。
- 合併回歸曾因舊 native fixture 尚未列出新的 scene／target／context request 欄位失敗；更新契約期望後該測試單獨通過，沒有改動交付或取消實作來迎合測試。
- 最終完整原生矩陣 8 passed／1 failed：#12 locale fallback 測試在 React 畫面尚未呈現時立即讀取 count；改為等待公開 combobox 可見後，同一目標測試單獨 1 passed。這是觀察端等待修正，沒有更動產品語言行為。以下列出各項最後結果，不聲稱完整矩陣曾單次 9 passed。

## 執行環境與結果

Windows 11 amd64，既有 MSVC／Rust stable、Bun、WebView2；無管理員權限、真實麥克風或使用者工作資料。只使用暫存 SQLite、合成口述、非機密 fixture key、loopback HTTP 與獨立 App profile。沿用 D:/git/notype-build 原生快取，debug=0、incremental=false、jobs=2；沒有複製第二套原生 C++ 快取。核心與純 Rust harness 使用獨立公開 registry cache `C:/Users/swche/AppData/Local/notype-study/build-cache/cargo-core`、target `D:/git/notype-core-build`；沒有複製憑證或改動 RUSTUP_HOME。

| 檢查                                                         | 結果                                                              |
| ------------------------------------------------------------ | ----------------------------------------------------------------- |
| Core public unit                                             | 9 passed                                                          |
| Core real HTTP／SQLite／review integration                   | 16 passed（3 + 8 + 5）                                            |
| Core doc example                                             | 1 passed                                                          |
| Portable desktop workflow unit                               | 5 passed（含 #12）                                                |
| Portable desktop HTTP integration                            | 11 passed（含 #12）                                               |
| 新三場景 native UI／HTTP／restart                            | 2 passed                                                          |
| 既有 native workflow／history／settings／migration           | 4 passed（最終完整矩陣）                                          |
| #12 native recognition／UI locales regression                | 2 passed（最終完整矩陣）＋1 targeted passed（修正 render 等待後） |
| Frontend TypeScript／Vite build、ESLint                      | passed                                                            |
| Rust fmt、Core clippy -D warnings、touched frontend Prettier | passed                                                            |

命令（repo root）：

```powershell
cargo test --offline --locked --manifest-path crates/notype-core/Cargo.toml
cargo clippy --offline --locked --manifest-path crates/notype-core/Cargo.toml --all-targets -- -D warnings
cargo test --offline --locked --manifest-path tests/integration/desktop-contracts/Cargo.toml --test desktop-workflow --test desktop-provider
$env:NOTYPE_DESKTOP_EXE='D:/git/notype-build/debug/notype.exe'
node --test --test-concurrency=1 tests/integration/desktop-scenarios.runtime.mjs tests/integration/desktop-workflow.runtime.mjs tests/integration/desktop-languages.runtime.mjs
node --test --test-concurrency=1 --test-name-pattern='visible native text settings' tests/integration/desktop-workflow.runtime.mjs
node --test --test-concurrency=1 --test-name-pattern='Simplified interface' tests/integration/desktop-languages.runtime.mjs
```

原生建置必須從 `desktop/` 讀取 scoped MSVC UTF-8 config：`bun run tauri build --no-bundle --debug -- --locked`；設定 `CARGO_TARGET_DIR=D:/git/notype-build`、`CARGO_PROFILE_DEV_DEBUG=0`、`CARGO_INCREMENTAL=0`、`CARGO_BUILD_JOBS=2`。重新啟動 debug App 產生實際 Specta bindings。`bun run build`／`bun run lint` 從 desktop 執行。既有 upstream `secure_input.rs` unused Emitter 及 Vite chunk-size 警告仍在，未作不相關修整。

最後實際 native debug binary SHA256：`f66a35f9b9b761f9f24f5fb780a3e7413154a82473f8fc1feaf2ee24672aa5a6`。純 Rust harness 重用的 module 出現未使用 public export／helper 警告，非產品 build 失敗；core clippy 使用 -D warnings 通過。Specta 產生的 bindings 只移除行尾空白，型別／命令內容採實際 native 產出。

## App 名稱限定 delta

使用者在 c45d595 驗證完成後明確取消 MVP 的網站網域取得。這個後續 delta 移除 SourceContext 的網域欄位、網站語法驗證、request／提示與 UI 中的網域預期；只保留可選的 App 名稱。較早的 v3 context JSON 可讀取，但舊 metadata 不會序列化進新的 request。翻譯的手動「領域」是使用者自行輸入的專業範圍，與網站資訊分開，保持原有功能。

等 Breeze CPU 測量完成並釋放後，先實測兩個 RED：舊實作仍驗證已取消的網域 metadata，HTTP session 因 InvalidInput 失敗；較早 v3 SQLite 讀回的 context 也仍暴露舊欄位。移除欄位後兩者 GREEN。重新執行 core 為 9 unit +17 integration +1 doc，portable workflow 5 +HTTP 11 全數通過；core clippy -D warnings 通過。新增的 SQLite restart fixture 保留原稿／時間／App 名稱，舊 metadata 只作相容讀取，不再輸出。這些結果取代上方縮減前的 core integration 16 數量。

增量 native build 27.34 秒完成，包含 TypeScript／Vite build。針對性原生 UI／HTTP 3 passed：可見說明只列 App 名稱、提示詞保存／還原／重啟、回譯與 revision 確認，以及 request 僅有 application 的完整錯誤／重試／取消回歸。本次沒有宣稱重跑整套原生矩陣。命令：

```powershell
node --test --test-concurrency=1 --test-name-pattern='visible scenario prompts|native translation history|visible native text settings' tests/integration/desktop-scenarios.runtime.mjs tests/integration/desktop-workflow.runtime.mjs
```

App-only native debug binary SHA256：`c9792ffa68fd72b283db6920d041fec053e4e47c1debcf340ffe8f8153ecb66a`。沒有新增瀏覽器／各網站整合、網域取得計畫或 API key 需求；原有回譯預設關閉、人工確認及原目標保護規則保持不變。

## 未涵蓋

尚未驗證真實公司 endpoint／model／auth／TLS／網路及實際翻譯／回答品質（#15）；沒有聲稱回譯保證語意正確。三場景鍵盤路由、原輸入目標擷取及原生交付留給 #13。#12 真實混語音質量的已知失敗仍按其 receipt 處理，本 ticket 的 fixture 不修飾或取代該驗收。未新增 installer、公開發布或送出任何使用者工作內容。
