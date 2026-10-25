# 設計決策與 grilling 紀錄

更新日期：2026-10-25。**Q1～Q15 是原始已確認決策；後續 C16～C22 更新場景、提示詞及錄音操作。** 本檔記錄使用者已回答的選項；環境未知及尚未驗證結果維持分開。GitHub repo／追蹤建立是使用者另行明確授權的工作。

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
| C13 | 本機語音辨識需支援「繁體中文＋英文」及「簡體中文＋英文」，皆可在同段錄音內中英混合；原稿在本機產生所選中文字形，兩模式分別驗收，介面語言獨立 | 2026-10-25 使用者明確新增語音辨識語言需求；延伸 Q4，不以英文 smoke 代表中文已達標 |
| C14 | 文字整理 prompt 採角色／處理原則／格式／限制及公開 few-shot，保留原意、數字／否定／英文／code／不確定資訊，依上下文刪贅詞；原稿及術語表當作資料，不執行指令，中文排版保護字面值 | 2026-10-25 使用者提供草稿並授權參考加強；依文字 LLM／既有 MVP 邊界調整，納入 #8／#11，尚待實作／真實模型品質驗收 |
| C15 | 公司文字 LLM 採 OpenAI 相容的 `chat/completions` 格式；endpoint、model ID、認證方式與實際 TLS／CA／Proxy 仍待公司環境驗證 | 2026-10-25 使用者明確回答「OpenAI 相容（chat/completions）」；現在可依此實作 adapter，不代表真實連線或整理品質已通過，也不要求先提供 key |
| C16 | 三個場景的提示詞可獨立編輯、保存及還原；只還原預設提示詞，不新增一般／全部設定重設 | 使用者新增可改預設提示詞，並明確回答「只需要還原預設提示詞」 |
| C17 | 納入口述、翻譯、隨便問三場景；開始鍵依圖片預設 Right Alt、Right Alt + Right Shift、Right Alt + Space；提示詞各自獨立 | 使用者提供圖片及明確要求；取代 Q2 僅做口述的原始範圍。翻譯角色／任務／保真約束／格式及手動領域／語氣／術語對照依最新草稿加入，見 [設計紀錄](11-scenario-prompt-design.md) |
| C18 | 點按場景鍵開始錄音，錄音中點按獨立可設定的停止鍵開始處理；停止鍵預設 Right Alt，Esc 取消 | 使用者明確回答「按一下開始、再按一下停止」，並補充停止鍵可設定且預設 Right-Alt；取代 Q13 的按住／放開操作 |
| C19 | 隨便問依口述意圖回答問題或代寫內容，採使用者提供的目標／範圍／排除條件／步驟與結構化思考方向 | 使用者明確回答「依口述意圖，回答問題或代寫內容」；原先直接當成純問答的未確認解讀已修正 |
| C20 | 自動語氣／格式可取得 App 名稱與網站網域，不讀頁面或輸入內容；metadata 與可信提示詞分層 | 使用者明確回答「App 名稱＋網站網域，不讀頁面或輸入內容」；延伸原 Q14 的最小上下文邊界，不擴大為頁面／輸入／視窗標題讀取 |
| C21 | 翻譯是否回譯由使用者選擇，提供輔助核對 | 使用者明確回答「使用者可選擇是否回譯」；不能以回譯自我一致性聲稱語意正確 |
| C22 | 回譯預設關閉，需要時開啟；啟用時先顯示譯文與回譯，確認後才嘗試受保護交付，不能略過原目標／去重 | 使用者分別回答「預設關閉，需要時開啟」及「先顯示譯文與回譯，確認後再交付」；後端確認綁定顯示結果版本，重試後舊確認失效 |

## 2. 訪談收斂

使用者已逐輪確認 Q1～Q15，最後以「全部採用建議」確認 Q11～Q15；其後以 C16～C22 擴充三場景、提示詞與可選回譯，更新操作方式。本輪隨便問目的、情境範圍、回譯預設與確認交付均已回答，不重問，也不猜測新的未定案行為。正式更新以 [規格 #8](https://github.com/swchen44/notype/issues/8) 為準；[Windows MVP 定稿](10-confirmed-windows-mvp.md) 保留原始背景。

模型選擇、公司 API 實際連線／認證、真實硬體／runtime 及 registry 技術條件是後續環境與實驗事項，不以偏好答案代替驗證。公司 API 格式已依 C15 確認，不重問。遇到新的需求變更，再更新決策或 ADR。

## 3. 設計樹

```text
Windows amd64／無 installer
├─ 個人pilot → 五個App → Terminal自然語言＋手動Enter → 原目標一致才貼上
├─ 核心dictation → 繁中夾英文忠實整理
│  ├─ CPU baseline／p95≤5秒目標 [待Windows量測]
│  ├─ LLM失敗保留原稿待選 → 可從原稿重試
│  ├─ 長期本機文字歷史 → 搜尋／複製／手動刪除
│  └─ 當次錄音／點按開始與獨立停止鍵／最小LLM上下文
└─ 可修改既有App → Handy source與payload研究
   ├─ portable runtime／模型 [待Windows驗收]
   └─ 公開npm-only → 公司設定本機 → GitHub private
```

## 4. 環境未知與驗證邊界

公司 API wire format／認證／CA、Windows CPU／RAM／GPU、既有 WebView2／開發工具、registry 取得方式與限制仍未觀察。接手 agent 先查環境與現有文件；查不到的項目記成所需輸入，不用偏好答案代替 evidence。

Typeless 與 Handy 的 source／靜態 payload 結果不是公司 Windows acceptance。本輪沒有執行 installer、App、語音辨識、公司 API 或發布 npm／pip package。
