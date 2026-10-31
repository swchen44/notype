# Handy 功能比較：原始碼查核

查核日期：2026-11-01。Handy 基準為 commit [`7bfb2e2c934b798bcf6e3627653aed2439996f89`](https://github.com/cjpais/Handy/tree/7bfb2e2c934b798bcf6e3627653aed2439996f89)，不是對最新版或發行執行檔的保證；notype 依比較時的 `ffdc227` 原始碼與其 README。以下為靜態原始碼查核，非 GUI、麥克風、跨 App 貼上或辨識品質實測。

| 項目 | Handy 固定版本的證據及範圍 |
| --- | --- |
| 本機辨識與平台 | [README](https://github.com/cjpais/Handy/blob/7bfb2e2c934b798bcf6e3627653aed2439996f89/README.md) 描述本機辨識和 Windows、macOS、Linux；[模型管理](https://github.com/cjpais/Handy/blob/7bfb2e2c934b798bcf6e3627653aed2439996f89/src-tauri/src/managers/model.rs) 包含模型語言能力與下載狀態。 |
| 中文字形 | [設定](https://github.com/cjpais/Handy/blob/7bfb2e2c934b798bcf6e3627653aed2439996f89/src-tauri/src/settings.rs) 含 `ChineseScript`，區分簡體、台灣繁體與香港繁體；[轉換程式](https://github.com/cjpais/Handy/blob/7bfb2e2c934b798bcf6e3627653aed2439996f89/src-tauri/src/chinese_script.rs) 使用 OpenCC 處理字形。 |
| 錄音快捷鍵 | [設定](https://github.com/cjpais/Handy/blob/7bfb2e2c934b798bcf6e3627653aed2439996f89/src-tauri/src/settings.rs) 的 `ShortcutActivation` 包含切換、按住及混合模式；另有 `transcribe_with_post_process` 快捷鍵。 |
| 文字後處理及提示詞 | [設定](https://github.com/cjpais/Handy/blob/7bfb2e2c934b798bcf6e3627653aed2439996f89/src-tauri/src/settings.rs) 包含 provider、base URL、key、model、多筆 prompt 與已選 prompt，並含 OpenAI 相容與 Anthropic API 格式。這些是結構中的能力，不等於各提供者均已實測。 |
| 翻譯 | [設定](https://github.com/cjpais/Handy/blob/7bfb2e2c934b798bcf6e3627653aed2439996f89/src-tauri/src/settings.rs) 有 `translate_to_english`；是否可用受所選語音模型能力限制，不能概括為所有模型都可翻譯。 |
| 詞彙 | [CustomWords](https://github.com/cjpais/Handy/blob/7bfb2e2c934b798bcf6e3627653aed2439996f89/src/components/settings/CustomWords.tsx) 已有詞彙介面，能新增與刪除，不應僅描述成後端設定。未全面查核 CSV 功能。 |
| 歷史 | [歷史管理](https://github.com/cjpais/Handy/blob/7bfb2e2c934b798bcf6e3627653aed2439996f89/src-tauri/src/managers/history.rs) 保存辨識及後處理文字；[設定](https://github.com/cjpais/Handy/blob/7bfb2e2c934b798bcf6e3627653aed2439996f89/src-tauri/src/settings.rs) 包含歷史數量及錄音保留期限。 |
| 目標輸入 | [設定](https://github.com/cjpais/Handy/blob/7bfb2e2c934b798bcf6e3627653aed2439996f89/src-tauri/src/settings.rs) 有多種 `PasteMethod` 與剪貼簿處理選項；特定 App 是否成功仍須實測。 |
| 模型目錄 | [模型管理](https://github.com/cjpais/Handy/blob/7bfb2e2c934b798bcf6e3627653aed2439996f89/src-tauri/src/managers/model.rs) 包含固定模型目錄、手動放入檔案及自訂模型發現機制；這不等同已確認使用者可在 UI 任意指定資料夾。 |

設定匯入／匯出、獨立問答或回譯等項目的 Handy 全部程式碼未做完整否定式查核；未在本次搜尋中確認，不可寫成「Handy 沒有」。
