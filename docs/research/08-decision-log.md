# 設計決策與 grilling 紀錄

更新日期：2026-10-24。**Q1～Q15 已由使用者確認，訪談決策已收斂。** 本檔記錄使用者已回答的選項；技術建議與環境未知維持分開。GitHub repo／追蹤建立是使用者另行明確授權的工作。研究及交接依此定稿，產品實作與 runtime 驗收留在 Windows。

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
| Q7 | Terminal 首版輸入自然語言 prompt；所有 App 手動按 Enter，精確口述 shell 命令後續再做 | 使用者回覆 Q7 照建議 |
| Q8 | CPU 可用、不要求獨顯；10～20秒短錄音以放開到貼上 p95≤5秒為待驗證目標 | 使用者回覆 Q8 照建議；未實測 |
| Q9 | 只維護 npm 配送；不做 Python wheel／pip 發行 | 使用者回覆只要 npm |
| Q10 | 加入長期文字歷史 | 使用者指定 |
| Q11 | 本機保存原稿、整理結果、時間與狀態，直到手動刪除；提供搜尋／複製／刪除；錄音只供當次處理 | 最後一輪全部採用建議 |
| Q12 | 通用產品包採公開 npm，公司 endpoint／model ID／key 留本機；GitHub private | 最後一輪全部採用建議；未發布產品包 |
| Q13 | 按住可自訂熱鍵說話、放開處理、Esc取消；Windows 避開快捷鍵衝突後設定按鍵 | 最後一輪全部採用建議 |
| Q14 | 公司 LLM 只接收當次逐字稿與手動術語表，不自動取得其他App文字、截圖或整份歷史 | 最後一輪全部採用建議 |
| Q15 | 原輸入目標仍一致才自動貼；改變或無法確認時，保留結果待使用者選擇 | 最後一輪全部採用建議 |
| C3 | pip／npm 有 Proxy；可上傳自製套件供 Proxy 取得 | 使用者補充 |
| C4 | 不能使用 installer；需 portable 檔案包及所需 runtime／模型 | 使用者補充「不要installer 無法安裝」 |
| C5 | 實作接點與 runtime 驗收從公司 Windows amd64 開始較合適 | 使用者提出並依目標 OS 給出建議；未開始實作 |
| C6 | 建立 GitHub repo，保存 history 與 issue tracking | 使用者明確授權；repo 採 private |
| C7 | 根目錄指引使用 `AGENTS.md`；GitHub Issues、五個預設 triage labels、single-context domain docs | 使用者確認 setup 草稿與檔名 |
| C8 | 測試分為 `tests/unittest/` 與 `tests/integration/` | 使用者補充；框架隨 Windows 實作選定 |
| C9 | 在使用者自己的 Windows 11 開發，GitHub 保存 handoff／issues／TODO；Windows 已有 Codex 與 grillme | 使用者最後明確選定；是否位於公司內網仍待查核 |
| C10 | MVP 先交付 Windows amd64；架構現在預留 macOS／Linux，以共用核心及平台 adapter 降低移植返工；正式 App 優先沿用 Handy 的 Rust／Tauri 2／React TypeScript 路線 | 2026-10-25 使用者新增跨平台架構需求；見 [ADR 0003](../adr/0003-shared-core-platform-adapters.md)，其他平台尚未實作／驗收 |
| C11 | 介面新增簡體中文（zh-CN），與繁體中文（zh-TW）可切換並保存偏好；介面語言獨立於語音辨識及文字輸出，不自動轉換使用者文字 | 2026-10-25 使用者要求「介面再加入簡體中文」；已加入規格 #8，實作／介面驗收待完成 |
| C12 | 真正需要 API key 時再通知使用者；目前繼續本機 STT、HTTP fixture 與核心開發，不要求先申請 key | 2026-10-25 使用者明確指示「需要 key 再和我講」 |

## 2. 訪談收斂

使用者已逐輪確認 Q1～Q15，最後以「全部採用建議」確認 Q11～Q15；目前產品決策 frontier 為空。完整行為整理在 [Windows MVP 定稿](10-confirmed-windows-mvp.md)。

模型選擇、公司 API wire format、真實硬體／runtime 及 registry 技術條件是後續環境與實驗事項，不以偏好答案代替驗證。遇到新的需求變更，再更新決策或 ADR；不重問已確認項目。

## 3. 設計樹

```text
Windows amd64／無 installer
├─ 個人pilot → 五個App → Terminal自然語言＋手動Enter → 原目標一致才貼上
├─ 核心dictation → 繁中夾英文忠實整理
│  ├─ CPU baseline／p95≤5秒目標 [待Windows量測]
│  ├─ LLM失敗保留原稿待選 → 可從原稿重試
│  ├─ 長期本機文字歷史 → 搜尋／複製／手動刪除
│  └─ 當次錄音／按住說話／最小LLM上下文
└─ 可修改既有App → Handy source與payload研究
   ├─ portable runtime／模型 [待Windows驗收]
   └─ 公開npm-only → 公司設定本機 → GitHub private
```

## 4. 環境未知與驗證邊界

公司 API wire format／認證／CA、Windows CPU／RAM／GPU、既有 WebView2／開發工具、registry 取得方式與限制仍未觀察。接手 agent 先查環境與現有文件；查不到的項目記成所需輸入，不用偏好答案代替 evidence。

Typeless 與 Handy 的 source／靜態 payload 結果不是公司 Windows acceptance。本輪沒有執行 installer、App、語音辨識、公司 API 或發布 npm／pip package。
