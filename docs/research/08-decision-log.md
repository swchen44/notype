# 設計決策與 grilling 紀錄

更新日期：2026-10-24。**訪談進行中，尚未完成最終共同理解確認。** 本檔記錄使用者已回答的選項；技術建議與環境未知維持分開。GitHub repo／追蹤建立是使用者另行明確授權的工作。

## 1. 已確認

| ID | 決策 | 來源 |
| --- | --- | --- |
| C1 | 公司內網 Windows；現有 API 只有文字 LLM | 使用者原始需求及 API 澄清 |
| C2 | 只做 amd64／x64 | 使用者補充「只需要作 amd64版本」 |
| Q1 | 先供本人日常使用，驗收後再擴同事 | 第一輪回答「是」 |
| Q2 | MVP 為熱鍵錄音→本機辨識→公司 LLM 整理→貼回；翻譯、選取改寫、Ask 後續 | 第一輪採建議 |
| Q3 | 接受修改現有開源 App，優先評估 Handy | 第一輪採建議 |
| Q4 | 繁中夾英文，忠實整理；保留原意、數字、否定詞、code 與英文名稱 | 第二輪「其它照建議」 |
| Q5 | LLM timeout／錯誤時保留原稿、停止自動貼上；由使用者選重試或貼原稿 | 第二輪「其它照建議」 |
| Q6 | 首批同時驗收瀏覽器文字框、VS Code、Teams、Outlook、Windows Terminal | 使用者指定五個目標 |
| C3 | pip／npm 有 Proxy；可上傳自製套件供 Proxy 取得 | 使用者補充 |
| C4 | 不能使用 installer；需 portable 檔案包及所需 runtime／模型 | 使用者補充「不要installer 無法安裝」 |
| C5 | 實作接點與 runtime 驗收從公司 Windows amd64 開始較合適 | 使用者提出並依目標 OS 給出建議；未開始實作 |
| C6 | 建立 GitHub repo，保存 history 與 issue tracking | 使用者明確授權；repo 採 private |

## 2. 目前問題 frontier

| ID | 待決事項 | 已提出的建議，尚未當成決策 |
| --- | --- | --- |
| Q7 | Windows Terminal 是自然語言 prompt，還需要精確口述 shell 命令？是否皆手動 Enter？ | MVP 先做自然語言 prompt；所有 App 手動送出 |
| Q8 | CPU／GPU 基線與速度／品質優先順序 | CPU 可用、不要求獨顯；10～20秒短錄音，放開到貼上 p95≤5秒作待驗證目標 |
| Q9 | 第一條套件配送渠道 | 先 npm；Python wheel 後續；portable 與無 installer 已是硬限制 |
| Q10 | 本機錄音／文字歷史保存 | 預設僅保留當次原稿與整理結果，供重試／複製 |

後續分支包含 package registry／可見性、錄音熱鍵操作、上下文輸入範圍與焦點切換處理；依本輪答案展開。不能把「其它照建議」套用到尚未問過的問題。

## 3. 設計樹

```text
Windows amd64／無 installer
├─ 個人pilot → 五個目標App → Terminal用途／焦點與貼上行為 [待決]
├─ 核心dictation → 繁中夾英文忠實整理 [已定]
│  ├─ LLM失敗停下待選 [已定]
│  ├─ STT品質／效能與CPU基線 [待決]
│  └─ 錄音熱鍵／上下文／歷史 [待決]
└─ 可修改既有App → Handy固定source與payload研究 [已取得]
   ├─ portable runtime／模型 → Windows驗收 [未執行]
   └─ npm／pip配送渠道／registry與可見性 [待決]
```

## 4. 環境未知與驗證邊界

公司 API wire format／認證／CA、Windows CPU／RAM／GPU、既有 WebView2／開發工具、registry 取得方式與限制仍未觀察。接手 agent 先查環境與現有文件；查不到的項目記成所需輸入，不用偏好答案代替 evidence。

Typeless 與 Handy 的 source／靜態 payload 結果不是公司 Windows acceptance。本輪沒有執行 installer、App、語音辨識、公司 API 或發布 npm／pip package。
