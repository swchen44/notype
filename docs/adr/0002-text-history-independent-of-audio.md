---
status: accepted
---

# 文字歷史與當次錄音分開保存

使用者需要長期查閱原稿與整理結果，但錄音只供當次處理，因此文字歷史在本機保留到使用者手動刪除，提供搜尋、複製與刪除；音訊清理不應連帶刪除文字。原稿、整理結果、時間與狀態必須在沒有長期錄音的情況下仍可保存及讀取，LLM 重試也應能直接從已保存原稿開始。

Handy stable 的文字寫入受 WAV 保存成功限制，既有 retention 同時清理文字列與錄音，不能靠一個「Never」設定滿足這個需求。修改時需要可選音訊關係、獨立清理、持久化狀態及對應 UI／migration；舊資料相容與 npm 更新後資料保留需在 Windows 驗收。

依據：[Q10、Q11](../research/08-decision-log.md) 與 [Handy history source audit](../research/09-history-source-audit.md)。
