# DeepSeek 單輪驗收工具

目前是來源與離線驗證成果，尚未交付可用的 Windows helper。原生 feature 建置、
獨立隱藏 console 驗證、binary hash 與最終審查仍待完成；現在不需要輸入 key。
此工具獨立於 notype App，不能當成麥克風、公司 API 或其他 App 輸入驗收。

原生建置及審查完成後，會提供 `notype-deepseek-probe.exe`、
`Start-deepseek-probe.cmd` 與 build/hash 紀錄。Launcher 建立專屬新 console；
工具拒絕共用／重新導向輸入，不讀取 notype 設定、環境變數或參數中的 key。

工具先檢查固定 round；已存在就只顯示紀錄，不問 key、不續跑、不重設。
全新 round 才在本機隱藏輸入 key，還原 console 模式，再要求明確輸入
`RUN P23-live-v1`。輸入 key 本身不發送請求；中止 key 或確認步驟也不建立 round。
確認後才建立一次性標記，之後即使中斷／失敗／重啟，都只能查看該輪。

固定 `deepseek-flash`、非思考模式、effort none、每次最多 512 output tokens、
完整 request 最多 8KiB／四則訊息。只用固定非機密短句，最多八次，涵蓋三場景、
回譯、取消與一次明確重試；沒有自動重試、模型列表或餘額請求。每個產出先在本機
顯示，核對語意、數字、否定及術語後才繼續；回譯不保證正確。

本輪授權 US$0.10，保守預留最多 US$0.0835584。32,768 input-token ceiling 是已審查的
有限實驗操作假設，並非服務商的帳單硬上限；異常只會停止後續，不能撤銷已發生費用。
取消／逾時／未知用量仍保留預留。開始前仍須核對當時官方
[價格](https://api-docs.deepseek.com/quick_start/pricing/)及
[API 契約](https://api-docs.deepseek.com/api/create-chat-completion/)。

固定 one-shot store 在 OS known-folder LocalAppData 下的
`notype-study/deepseek-probe/P23-live-v1`，獨立結果目錄在同層 `P23-live-v1-results`：

- `sessions.db`：實際 SessionService 歷史及內部 call ledger。
- `history.csv`：BOM UTF-8、每個 history ID 一列；回譯／重試加總 tokens，模型／effort
  保留關聯，含原稿／結果。本工具沒有錄音，秒數及 ASR 模型維持未知。
- `receipt.md`：slot 狀態、來源及預留；binary hash 另見交付的 build receipt。

未回報實收金額時，歷史／CSV 顯示未知；預留及牌價算式不寫入費用欄位。key、
Authorization、endpoint、完整請求／回應及 reasoning 不寫入證據。工具不自動上傳，
原稿／產出是刻意保存的研究文字。重開只讀 round，上次 CSV 可能早於中斷中的呼叫，
不能當完整帳單；沒有刪除／換名／重設來開始另一輪的選項。

見 [bridge receipt](verification/deepseek-probe-bridge.md)及
[offline receipt](verification/deepseek-probe-offline.md)。
