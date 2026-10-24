# Handy 文字歷史、錄音保存與 npm 更新接縫

查核日期：2026-10-24。範圍為 Handy stable v0.9.8，固定 commit **`14f6f0d31cb22a4268acfbddba3052820fd0a06e`** 的現有完整 source。這是普通原始碼研究，未使用 REA、執行 App、安裝套件或呼叫公司 API。來源身分見 [history audit metadata](evidence/github/handy-history-source-audit.json)。

已確認需求：Windows x64 個人 pilot、npm-only 配送、自然語言 prompt、繁中夾英文忠實整理、所有 App 手動 Enter，以及長期文字歷史。後續 Q11 已確認本機保存原稿／整理結果／時間／狀態，保留到手動刪除並提供搜尋／複製／刪除；錄音只供當次處理。以下上游事實與工程建議仍分開描述。CPU baseline 與 10～20 秒錄音放開熱鍵到貼上 p95 ≤ 5 秒是待驗證目標，沒有此處的效能實測。

**核心發現：上游有 SQLite 文字歷史，但寫入與 WAV 成功耦合，清理也同時刪文字列與錄音。長期文字＋可選錄音需要修改 persistence／retention 接縫，不能只改現有選單。**

## 1. 已有的 schema 與欄位

[HistoryManager::new][manager] 使用 `portable::app_data_dir()`，建立 `history.db` 及 `recordings/`。`MIGRATIONS` 建立並逐步擴充 `transcription_history`；使用 SQLite `user_version` 記錄 migration，另有舊 tauri-plugin-sql migration 轉換。這是應用自己的資料庫，不需要另裝 SQLite server。

| 欄位 | 原始碼觀察 | 不能據此推導的資訊 |
| --- | --- | --- |
| `id` | INTEGER 自增 primary key | 不是跨裝置或跨資料庫的全域 ID |
| `file_name` | TEXT NOT NULL，通常指向錄音 WAV | schema 尚未表達「沒有錄音」的可選關係 |
| `timestamp` | `save_entry()` 用 `Utc::now().timestamp()`，秒級 | 是保存時刻，沒有獨立錄音開始／結束／LLM 完成／貼上時刻 |
| `saved` | BOOLEAN，預設 false；星號切換後免受自動清理 | 名稱不是 DB 已持久化與否，也不是 LLM／貼上成功狀態 |
| `title` | 由 timestamp 格式化為 title | 不是 App／視窗 title |
| `transcription_text` | STT 原逐字稿，NOT NULL；辨識失敗可保存空字串 | 空值狀況由 UI 推斷，沒有明確錯誤碼／工作狀態 |
| `post_processed_text` | 可選 LLM 整理結果 | None 可能來自未要求、跳過或失敗；不能單憑此欄判定原因 |
| `post_process_prompt` | 可選 prompt template 記錄 | 不是完整 request／模型版本／API receipt；沒有 token 值欄位 |
| `post_process_requested` | BOOLEAN，記錄這次 action 是否要求整理 | 不是整理成功與否 |

上述 9 欄及 `HistoryEntry` struct 都可在 [manager][manager] 核對。此 schema 沒有 App 名稱／process、視窗、模型 ID、語言、ASR／LLM／貼上 status 或 error 欄位。本次沒有把這些缺少欄位自動列為使用者已要求的功能。

`HistoryUpdatePayload` 的 `added/updated/deleted/toggled` 是 UI event action，沒有寫成工作流程 status。若公司版本要保留「LLM 失敗待使用者選擇」紀錄，需要新的明確狀態，而不是把這些 event 名稱當成既有持久化欄位。

## 2. 為什麼目前文字歷史依賴 WAV

[TranscribeAction::stop][actions] 取得非空 samples 後，同時開始 WAV save 與 STT。WAV save 完成後還會 `verify_wav_file()`，得到 `wav_saved`。

- STT 成功：完成可選 LLM 整理後，**只有 `wav_saved` 為 true 才呼叫 `hm.save_entry(...)`**。
- STT 失敗：同樣只有 WAV 成功，才保存空逐字稿列供重新辨識。
- `HistoryManager::save_entry()` 方法本身沒有檢查檔案存在；耦合主要在 action call site，而且 schema 要求 `file_name`。[manager]

因此若公司版本停用落地 WAV，或磁碟 WAV 保存／驗證失敗，現有 action 可能仍可得到 STT／LLM 文字，但不會有對應文字歷史。把錄音 retention 設為「不刪」或修改 count，不能解決這個依賴。

目前也沒有一筆先保存的 pending history row：`save_entry()` 在上述流程較後段才發生。公司要求的失敗待選、LLM 重試與長期歷史若需跨重新啟動恢復，需要另外設計保存／更新時機。這是工程推論，不是上游已具備的能力。

## 3. 錄音與文字 retention 尚未分離

[cleanup_old_entries()][manager] 讀 `recording_retention_period`，實際會操作整筆歷史：

| 設定 | 原始碼實際語意 |
| --- | --- |
| `Never`／`never` | **永不自動清理**，不是「不保存錄音」 |
| `PreserveLimit` | 用 `history_limit`，保留最新指定數量的未加星號 entries |
| `Days3`／`Weeks2`／`Months3` | 按 timestamp 清理超過時間的未加星號 entries；Months3 以 90 天計 |
| `saved=true` | count／time cleanup 的 SQL 都只選 `saved=0`，加星號列受保護 |

`delete_entries_and_files()` 會先刪 SQLite row，再嘗試刪其 WAV；不是只刪音訊。`delete_entry()` 的手動刪除則先嘗試 WAV，再刪 row；音訊刪除失敗會記 log 並繼續刪 row。兩條路徑都沒有獨立的文字保留政策。[manager]

[settings.rs][settings] 預設 `history_limit=5`、retention 為 `PreserveLimit`，所以預設是只留最新 5 筆未加星號 entries，並非長期歷史。`save_entry()` 後及更新 limit／retention 的 [commands][commands] 都會呼叫 cleanup；改設定可以立即影響既有資料。

只把 `never` 設好可讓這條 cleanup 不刪 DB／錄音，但仍不能達成「文字長期、音訊不存或短期」的獨立政策。兩者需要拆開；本產品已確認錄音只供當次處理。

## 4. 現有查詢、畫面與操作

| 操作 | Source-level 結果 | 範圍與差異 |
| --- | --- | --- |
| 歷史列表 | 已有 `get_history_entries(cursor,limit)`，依 `id DESC` 分頁 | UI `PAGE_SIZE=30`、infinite scroll；有 limit 時後端上限 100，每次多讀一筆判斷 has_more |
| 依 ID 讀取 | `get_entry_by_id()` 已有 | 不是全文查詢 |
| 原稿顯示／複製 | HistorySettings 使用 `entry.transcription_text` | **這個歷史畫面沒有把 post_processed_text 作為展示／複製內容** |
| 最後一次結果複製 | tray 的 `copy_last_transcript()` 優先 post_processed_text | 與歷史畫面的複製語意不同，不應混為同一功能 |
| 星號保存 | `toggle_saved_status()` 已有 | 保護整筆歷史／錄音免自動清理 |
| 刪除一筆 | `delete_history_entry()` 已有 | 同時處理文字列與對應音訊；UI 有 optimistic update |
| 播放音訊 | `get_audio_file_path()`＋AudioPlayer | 目前每筆 history component 都建立 audio player；text-only 要調整 UI |
| 重新辨識 | `retry_history_entry_transcription()` 已有 | **先讀歷史 WAV**，重新 STT 後再依 post_process_requested 整理；沒有錄音時無法完成 |
| 更新原稿／整理結果 | `update_transcription()` 已有 | 覆寫同一 row 的三個文字欄位，未保存每次重試的版本鏈，也沒有更新 timestamp |
| 全文搜尋 | 本次限定檢查未找到 | manager SQL 沒 search query／LIKE／FTS，commands 無搜尋參數，HistorySettings 無搜尋欄位 |
| 批次文字匯出 | 本次限定檢查未找到 | 所讀 commands／UI 沒 JSON／CSV／Markdown export；開啟 recordings folder 是另一件事 |

來源：[manager][manager]、[commands][commands]、[HistorySettings][ui]、[clipboard helper][clipboard]、[tray][tray]。另對 `src`、`src-tauri/src` 的 `.ts/.tsx/.rs` 搜尋 `history.*export/export.*history/history.*search/search.*history`，未找到產品實作。這是有範圍的負面搜尋，不是對所有未讀程式碼或未來版本的絕對斷言。

## 5. 長期 text-only 歷史需要修改的接縫

以下是依上述 source 提出的工程建議，**沒有在本次實作，也不代表使用者已同意每個新增欄位或功能**。

1. **文字 persistence 與 WAV 分離。** 修改 `actions.rs` 的 `wav_saved` gate，讓有效文字即使不保存錄音也會落地。schema 讓音訊關係可為空，或移到獨立 audio record；避免以虛構檔名表示音訊一定存在。
2. **文字與音訊獨立 cleanup。** 保留文字長期政策；當次處理後清理音訊。`cleanup_old_entries()`、`delete_entries_and_files()`、settings commands 與 selector 要同步修改，避免沿用「清音訊時刪整筆」的行為。
3. **LLM 重試從已存原稿開始。** 新增只做文字整理的 command／狀態更新，直接使用 transcription_text；原有重新辨識按鈕仍代表 WAV→STT，不能在無錄音狀態冒充 LLM retry。
4. **History UI 表達文字版本與可用操作。** 讓原稿、整理結果可區分；audio 不存在時不顯示可播放／重新 STT 的操作。本產品需可區分地查閱／複製原稿與整理結果。
5. **失敗與待選的保存時機。** 若要重新啟動後仍能找回 LLM 失敗原稿與待選結果，需在適當階段先落地，再更新狀態。時間與狀態及搜尋已由 Q11 確認；App／模型／耗時等額外 metadata、批次匯出並未列為首版要求。

SQLite 可以繼續使用；目前 schema／query／UI 不構成大規模長期歷史的效能驗證。若後續選擇全文搜尋，需另外設計查詢與索引並用實際資料量驗收，搜尋需求已確認，FTS 或其他實作仍須依資料量與查詢測試選擇。

## 6. npm 更新如何維持固定資料位置

[portable.rs][portable] 的一般模式使用 Tauri app data；portable 模式將資料根固定成 executable 旁的 `Data/`。`HistoryManager` 與 model／settings 等使用這個接縫；[tauri.conf.json][config] 的 identifier 是 `com.pais.handy`。

可採的方案有兩種，均為建議而未實作：

- **一般 app-data 模式。** npm wrapper 只管理版本化 executable／DLL，執行時沿用固定 app identifier 對應的每使用者資料目錄。npm 更新 package 時不把 history.db 放在 package tree；fork 若改 identifier，須明確處理資料遷移。
- **固定 portable root。** wrapper 將程式 payload staging 到固定、每使用者可寫的 App 根目錄，更新時保留 `Data/`，替換程式檔與 DLL。若每版 exe 都放不同版本目錄而沿用相對 Data，就會讀到不同資料庫；要嘛固定 exe root，要嘛新增明確 data-dir override。

目前所讀 `portable::app_data_dir()` 只有 portable Data／Tauri app-data 兩條路徑，沒有任意模型／歷史目錄參數。不要把 source 尚無的 `--data-dir` 當成現成能力。整個自製 npm wrapper 尚未存在，也未驗證更新中斷、rollback、Windows lock、SQLite migration 或 npm uninstall 後資料保留。

既有 [portableInstaller.ts][portableupdate] 描述 portable 不能原地自動更新，改導向官方 NSIS installer URL。它不是公司 npm-only 更新機制；公司版本需另整合或停用這條 upstream 更新路徑。

## 7. 本次狀態與驗證邊界

已確認的 source facts 是：原稿／整理結果皆有 DB 欄位、WAV gate、共同 cleanup、預設 5 筆、UI 原稿複製、WAV-based retry、可沿用 app-data／portable data 路徑。這些事實不需要再向使用者詢問。

後續產品決策已確認文字長期、錄音當次、原稿／整理結果／時間／狀態，以及搜尋／複製／刪除。額外 App／模型 metadata、批次匯出與重試版本鏈未列為首版要求。未做 Windows runtime、資料遷移或長期資料量測試；p95 5 秒目標也沒有在這份 source 研究中驗證。

[manager]: https://github.com/cjpais/Handy/blob/14f6f0d31cb22a4268acfbddba3052820fd0a06e/src-tauri/src/managers/history.rs
[commands]: https://github.com/cjpais/Handy/blob/14f6f0d31cb22a4268acfbddba3052820fd0a06e/src-tauri/src/commands/history.rs
[actions]: https://github.com/cjpais/Handy/blob/14f6f0d31cb22a4268acfbddba3052820fd0a06e/src-tauri/src/actions.rs
[settings]: https://github.com/cjpais/Handy/blob/14f6f0d31cb22a4268acfbddba3052820fd0a06e/src-tauri/src/settings.rs
[ui]: https://github.com/cjpais/Handy/blob/14f6f0d31cb22a4268acfbddba3052820fd0a06e/src/components/settings/history/HistorySettings.tsx
[clipboard]: https://github.com/cjpais/Handy/blob/14f6f0d31cb22a4268acfbddba3052820fd0a06e/src/components/settings/history/clipboard.ts
[tray]: https://github.com/cjpais/Handy/blob/14f6f0d31cb22a4268acfbddba3052820fd0a06e/src-tauri/src/tray.rs
[portable]: https://github.com/cjpais/Handy/blob/14f6f0d31cb22a4268acfbddba3052820fd0a06e/src-tauri/src/portable.rs
[config]: https://github.com/cjpais/Handy/blob/14f6f0d31cb22a4268acfbddba3052820fd0a06e/src-tauri/tauri.conf.json
[portableupdate]: https://github.com/cjpais/Handy/blob/14f6f0d31cb22a4268acfbddba3052820fd0a06e/src/components/update-checker/portableInstaller.ts
