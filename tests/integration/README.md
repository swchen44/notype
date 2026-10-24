# Integration tests

放置模組接合與目標環境的完整行為測試。每項測試先列明 Windows、runtime、模型、App、API 或網路需求，分開 deterministic 與真實環境結果。

| 路徑 | 要驗證的行為 | 追蹤 |
| --- | --- | --- |
| Portable 啟動 | 完整 x64 payload、DLL／runtime、本機模型與可寫 data 路徑；無 installer／bootstrapper | [#2](https://github.com/swchen44/notype_study/issues/2) |
| 本機 STT | 固定音訊轉文字；繁中夾英文、數字、否定詞與術語；模型／backend 固定 | [#3](https://github.com/swchen44/notype_study/issues/3) |
| STT → LLM → 結果 | 先用本機 HTTP fixture 驗證，再以公司 API 驗收真實 contract | [#4](https://github.com/swchen44/notype_study/issues/4) |
| 錯誤與取消 | timeout／錯誤不遺失原稿、不自動貼原稿；重試／取消不重複插入 | [#4](https://github.com/swchen44/notype_study/issues/4) |
| 跨 App 送字 | Browser、VS Code、Teams、Outlook、Windows Terminal；IME、換行、剪貼簿與焦點 | [#5](https://github.com/swchen44/notype_study/issues/5) |
| 長期文字歷史 | App 重啟後文字仍可讀、錯誤原稿可復原；npm 更新時持久資料保留；文字保留到手動刪除、錄音僅當次處理 | [#7](https://github.com/swchen44/notype_study/issues/7) |
| Proxy 配送 | 實際取得包、核對內容／digest、預載模型後冷啟動；記錄額外下載與外部連線 | [#6](https://github.com/swchen44/notype_study/issues/6) |

效能測試記錄 STT、LLM、貼上及放開熱鍵到文字出現的 p50／p95、樣本數與硬體。門檻以 [決策紀錄](../../docs/research/08-decision-log.md) 已確認的內容為準。

GUI 驗收使用非機密文字、測試文件或草稿；所有 App 手動 Enter，Terminal 首版以自然語言 prompt 為主；原輸入目標改變或無法確認時保留待選結果。

目前沒有產品測試檔，也未在公司 Windows 執行。實作時把可自動化測試加入本目錄，並列出仍需真實 App 驗收的項目。
