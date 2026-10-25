# 已確認的 Windows MVP 與研究結論

原始定稿日期：2026-10-24；2026-10-25 更新三場景、可編輯提示詞及點按／獨立停止鍵。Q1～Q15 的原始回答與後續修改分別記錄於 decision log；實作與驗收現況見 HANDOFF／Issues／receipts，本文不代表所有功能已交付。

2026-10-25 後續補充：正式更新以 [規格 #8](https://github.com/swchen44/notype/issues/8) 與 [HANDOFF](../../HANDOFF.md) 為準，已有 prototype／開發機測試。MVP 仍先交付 Windows amd64，架構預留 macOS／Linux；介面支援繁／簡中文，語音辨識支援「繁體中文＋英文」及「簡體中文＋英文」兩種中英混合模式，兩種設定獨立，中文模型品質／效能仍待驗收。

## 1. 最終方向

先為使用者本人建立 **Windows amd64 portable 語音輸入 App**，優先從 Handy 固定版本評估修改。主路徑為：

```text
點按場景快捷鍵開始錄音 → 點按獨立停止鍵 → 本機 STT → 保存原稿
  → 公司文字 LLM 依口述／翻譯／隨便問場景處理 → 保存結果與狀態
  → 原輸入目標仍一致時貼上；否則保留為待選結果
```

**只用 npm 配送，不使用 installer／MSI／runtime bootstrapper。** 通用產品包以公開 npm 為發布方向，公司 endpoint、model ID、key 留在本機，GitHub repo 維持 private。Npm 是取得與更新通道，預編譯核心可用 Rust；語音引擎、DLL、模型與 WebView runtime 仍需完整準備，不能由 Rust 名稱推導零相依。

## 2. 已確認的產品行為

| 面向 | 已確認內容 |
| --- | --- |
| 使用者／平台 | 本人 pilot，Windows amd64／x64；驗收後再擴同事 |
| 操作 | 點按場景鍵開始錄音，獨立可設定停止鍵結束／處理，停止鍵預設 Right Alt；Esc取消 |
| 場景／提示詞 | 口述、翻譯、隨便問；開始鍵依序 Right Alt／Right Alt + Right Shift／Right Alt + Space，可改；各自提示詞可編輯／保存／單獨還原。隨便問依意圖回答或代寫；情境僅 App 名稱，網站網域擷取已取消；回譯可選、預設關閉，啟用後顯示譯文及回譯，確認後才嘗試受保護交付，見 #8 |
| 辨識／輸出 | 繁體中文＋英文、簡體中文＋英文兩種同段中英混合模式，保存選擇；原稿由本機流程產生所選字形，整理結果預設沿用。整理贅詞、標點、口頭修正與清單，保留原意、數字、否定詞、code與英文名稱 |
| 上下文 | 公司 LLM 只收到當次逐字稿與手動術語表；不自動取得其他 App 文字、截圖或整份文字歷史 |
| 目標 | Browser文字框、VS Code、Teams、Outlook、Windows Terminal |
| 送出 | 所有 App 由使用者手動按 Enter；Terminal 首版以自然語言 prompt 為主 |
| 焦點 | 原輸入目標仍一致才自動貼；已改變或無法確認時，保存待選結果 |
| LLM失敗 | 保留原稿、停止自動貼上，由使用者選重試或貼原稿 |
| 文字歷史 | 本機長期保存原稿、整理結果、時間與狀態；提供搜尋／複製／刪除，手動刪除前保留 |
| 錄音 | 只供當次處理，不做長期錄音歷史；與文字保存／清理分開 |
| 效能 | CPU可用、不要求獨顯；口述 10～20秒短錄音，停止到交付 p95≤5秒是待驗證目標；翻譯／隨便問另列量測 |
| 測試 | `tests/unittest/` 與 `tests/integration/`，隨產品行為加入可執行測試 |

上述是使用者已確認的需求。[完整訪談紀錄](08-decision-log.md) 是決策來源；機器效能、API 相容性、貼上可靠度仍須用實測證明。

## 3. 開源基底與必改之處

Handy v0.9.8／`14f6f0d31cb22a4268acfbddba3052820fd0a06e` 已有本機 STT、自訂文字 LLM、全域熱鍵、送字與 portable source 機制，適合先做目標 Windows 可行性驗收。這是研究建議，不是對公司 runtime 的驗收結果。

| 上游觀察 | 公司版工作 |
| --- | --- |
| LLM 失敗後可能直接貼原稿 | 改成持久保留原稿與待選狀態，明確重試／貼原稿 |
| `wav_saved` 成功才寫文字歷史 | 拆開文字 persistence 與錄音保存 |
| 清理會同時刪 DB row 與 WAV；預設只留5筆未加星號紀錄 | 改成文字長期保存、音訊僅當次；手動文字刪除另處理 |
| DB 有原稿與整理結果，缺明確 workflow status | 增加所需狀態及 migration，驗證重新啟動後的復原 |
| History UI 主要顯示／複製原稿 | 提供可區分的原稿與整理結果操作、搜尋與刪除 |
| 既有重新辨識必須讀 WAV | 另提供從已保存原稿重試 LLM 的路徑 |
| Portable updater 可能導向 NSIS installer | 改為 npm-only 更新路徑；維持無 installer限制 |
| Portable `Data/` 隨 executable 路徑 | 將持久資料固定在 npm package tree 之外，驗證升級／中斷／migration |

來源與 bounded search 範圍見 [06 source／payload audit](06-source-feasibility-audit.md) 及 [09 history audit](09-history-source-audit.md)。尚未做產品修改、資料遷移或 runtime 驗證。

## 4. Windows 實作優先順序

1. **唯讀 preflight。** 查 Windows硬體、現有runtime／build工具、npm Proxy、公司API contract與可寫路徑。缺少工具時評估另一個 Windows build環境，不在公司機器執行 installer。
2. **最小完整路徑。** 固定模型與預編譯payload，讓一段非機密錄音經本機STT、公司文字API，落地原稿／結果並交付至測試輸入框。先用此路徑量測延遲。
3. **失敗與歷史。** 加入timeout／取消／待選、無錄音的文字持久化、重啟復原、從原稿重試LLM，以及搜尋／複製／刪除。
4. **五個App與焦點。** 驗證IME、換行、剪貼簿、只插入一次、手動Enter、目標切換待選。
5. **npm與更新驗收。** 檢查實際tarball內容，從公司Proxy取得後冷啟動，驗證模型／runtime齊備及更新後文字歷史保留。

這是研究後的工程建議順序；後續正式 `to-spec`／`to-tickets` 可將其拆成帶blocking edges的可展示切片。既有 GitHub issues 是研究與待辦追蹤，不代表全部已達 `ready-for-agent`。

## 5. 驗收與尚缺的證據

- unit tests：session狀態、取消、LLM錯誤待選、API adapter、送字去重、上下文邊界、history保存／清理規則。
- integration tests：固定音訊→STT→測試HTTP／公司LLM→文字歷史→輸入；history重啟／migration／npm更新；五個App真實送字；完整portable冷啟動。
- 效能：固定硬體、音訊、模型／精度／backend，分開STT、LLM、貼上與總延遲，報樣本數、p50／p95及常駐RAM。

公司API wire format／認證／CA、Windows硬體與runtime、實際模型及package payload、npm publisher與registry限制仍需在後續環境查核。公開npm發布方式已同意，但產品包與發布憑證尚未就緒，本次沒有publish。

接手入口：[Windows說明與prompt](07-windows-handoff.md)。Domain用語與重要設計分別見 [GLOSSARY](../../GLOSSARY.md)、[portable npm ADR](../adr/0001-portable-npm-distribution.md)、[文字／音訊保存 ADR](../adr/0002-text-history-independent-of-audio.md)。
