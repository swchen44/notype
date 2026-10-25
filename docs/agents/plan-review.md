# Implementation plan review

使用者指定：所有新的實作計畫先交 **GPT-6-Astra** 審查，確認沒有未解問題後才實作。計畫與審查記錄在對應 GitHub issue／ticket。

## 開始實作前

1. 在對應 issue 發布版本化計畫，列出範圍、行為、資料邊界、測試 seam、驗收條件，以及適用的包裝／更新策略。
2. 交給獨立的 GPT-6-Astra reviewer；模型識別為 `gpt-6-astra`。Reviewer 查核計畫與實際 source，將結論發布到該 issue，註明模型、計畫版本及原計畫 comment URL。
3. 有問題時先修訂計畫，再請同模型複審。取得針對該確切版本的 `APPROVED` 與 **0 項未解問題**後，才開始該範圍的實作。
4. 把計畫與通過審查的連結提供給實作者，依核准範圍執行；語意、範圍、資料邊界或部署策略有變動時，發布新版本並重新審查。

在指定 reviewer 無法使用或尚有未解問題時，保存目前工作並回報具體阻塞；實作者不自行核准或更換審查模型。唯讀調查與計畫修訂可繼續，但不代表產品變更已核准。

## 接手與完成

Issue、HANDOFF／TODO 應能找到目前計畫版本、審查狀態與通過連結。來源審查、測試、原生執行及實際交付依原有規範分開回報；計畫通過不等於驗收通過。

此文件的實作計畫為 [P0-v1](https://github.com/swchen44/notype/issues/8#issuecomment-6398008744)，[GPT-6-Astra 已核准，0 項未解問題](https://github.com/swchen44/notype/issues/8#issuecomment-6398043691)。
