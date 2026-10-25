# 語言處理實驗模型選擇

查核日期：2026-10-26（Asia/Taipei）；範圍為 2024-10-26 以後發布的模型。這份文件只選 **實驗模型**，不替 UI 的手動 model ID 設定預設值，也不代表實際 API／免費額度／文字品質已驗收。未讀取使用者 key、未呼叫認證或推論 API、未付費；僅讀取官方公開文件、公開模型目錄與評測。

## 建議的實驗組

| API | 優先實驗 ID | 備援實驗 ID | 發布與用途 | Thinking 預設關閉時 |
| --- | --- | --- | --- | --- |
| Gemini OpenAI 相容 API | `gemini-3.8-flash` | `gemini-3.5-flash-lite` | 主模型 2026-09-02；備援 2026-07-21。前者作主要品質對照，後者作短句整理、翻譯與低延遲對照 | 3.8 Flash 不可真正關閉，最低為 `low`；`minimal` 明確不支援。依使用者已選政策提示後以 `reasoning_effort: low` 執行，歷史區分 requested OFF／effective ON。Flash-Lite 的最低是 `minimal`，也不得標成真正關閉。 |
| OpenRouter | `google/gemma-4-31b-it:free` | `google/gemma-4-26b-a4b-it:free` | 兩者 2026-04-02；31B 作文字品質主要對照，26B A4B MoE 作速度備援 | 公開目錄 `reasoning.mandatory=false`、`default_enabled=false`；以 OpenRouter 的 `reasoning.enabled=false` 表達關閉，不把 `exclude=true` 當關閉。 |

Gemini 的模型資訊、相容介面及最低 thinking 以 [3.8 Flash](https://ai.google.dev/gemini-api/docs/models/gemini-3.8-flash)、[3.5 Flash-Lite](https://ai.google.dev/gemini-api/docs/models/gemini-3.5-flash-lite)、[OpenAI compatibility](https://ai.google.dev/gemini-api/docs/openai)、[3.5 Flash-Lite thinking](https://docs.cloud.google.com/gemini-enterprise-agent-platform/models/gemini/3-5-flash-lite?hl=en) 為依據。發布日期見 [3.8 model card](https://deepmind.google/models/model-cards/gemini-3-8-flash/) 與 [Gemini release notes](https://ai.google.dev/gemini-api/docs/changelog)。

Gemma 4 的發布與多語言範圍見 [Google 發布文](https://developers.googleblog.com/bring-state-of-the-art-agentic-skills-to-the-edge-with-gemma-4/)。OpenRouter 的精確 ID 見 [31B 免費頁](https://openrouter.ai/google/gemma-4-31b-it:free)、[26B A4B 免費頁](https://openrouter.ai/google/gemma-4-26b-a4b-it:free)，thinking 語意見 [Reasoning tokens](https://openrouter.ai/docs/guides/best-practices/reasoning-tokens)。

## 與 Qwen3.8 27B 的關係及網路評價

`Qwen3.8-27B` 是正式型號，不需要修正為 Qwen3.5：Qwen 官方記錄 2026-08-14 發布，支援 thinking 與 non-thinking。[官方 release README](https://github.com/QwenLM/Qwen3.8/blob/main/README.md)、[官方 model card](https://huggingface.co/Qwen/Qwen3.8-27B)。

評測呈現不同面向，不能用參數量直接推論語言品質：

- **接近使用者的自然語言偏好**：Arena 官方 2026-10-22 文字人類盲評，Gemma 4 31B 為 1452±7、26B A4B 為 1437±7、Qwen3.8-27B 為 1437±5。這支持將 Gemma 作同級文字實驗候選，但不是繁中／中英混排正確率保證。[Arena 原始榜單](https://arena.ai/leaderboard/chat/text?license=open-source)
- **較重推理的綜合指標**：Artificial Analysis 當日 Qwen3.8 27B xhigh 為 34、Gemma 4 31B reasoning 為 15，不能宣稱 Gemma 全面等於 Qwen。Gemini 3.8 Flash low 為 33，適合作主要 API 品質對照；不同 thinking 設定及測試預算仍限制可比性。[Qwen／Gemma 原始比較](https://artificialanalysis.ai/models/comparisons/qwen3-8-27b-vs-gemma-4-31b)、[Gemini low 原始評測](https://artificialanalysis.ai/models/gemini-3-8-flash-low)
- **語言任務的選擇理由**：Google 明列 Gemma 4 涵蓋 140 多種語言；Flash-Lite 的官方用途含翻譯與簡單資料處理。這些是適配線索，不是本產品實測成績。[Gemma 4 說明](https://deepmind.google/models/gemma/gemma-4/)、[Gemini 定價與用途](https://ai.google.dev/gemini-api/docs/pricing)
- **社群使用經驗**：有人回報 Gemma 的自然語言／摘要體驗較好，也有人回報 Qwen 翻譯會服從原文中的指令。這類自選樣本的硬體、prompt 與評分並不一致，只用來增加本產品的失敗測例，不據此宣稱誰一定較準。[摘要討論](https://www.reddit.com/r/LocalLLaMA/comments/1u1ggo9/newer_qwen_models_are_worse_at_summarization/)、[翻譯資料中的指令案例](https://www.reddit.com/r/LocalLLaMA/comments/1w71vg1/even_qwen38_followed_the_instruction_inside_my/)

因此，主組是 Gemini 3.8 Flash low 與 Gemma 4 31B non-thinking；備援模型只在明確實驗選擇後使用，不能在同一次請求失敗時靜默換模型重送。這是依據上述資料作出的設計判斷。

## 免費與實際 route 查核

已無認證讀取 [OpenRouter 公開模型目錄](https://openrouter.ai/api/v1/models) 與三個公開 endpoints metadata（不是推論呼叫）。2026-10-25（UTC）的快照：

| ID | 主目錄 | 精確 ID 的 endpoints | prompt／completion 價格 |
| --- | --- | --- | --- |
| `google/gemma-4-31b-it:free` | 有 | 1 個，Google AI Studio，status 0 | 均為字串 `0` |
| `google/gemma-4-26b-a4b-it:free` | 有 | 1 個，Google AI Studio，status 0 | 均為字串 `0` |
| `qwen/qwen3.8-27b:free` | 未列出 | 0 個 | 沒有可驗證的目前免費 route |

可重查的公開 metadata：[31B](https://openrouter.ai/api/v1/models/google/gemma-4-31b-it:free/endpoints)、[26B A4B](https://openrouter.ai/api/v1/models/google/gemma-4-26b-a4b-it:free/endpoints)、[Qwen](https://openrouter.ai/api/v1/models/qwen/qwen3.8-27b:free/endpoints)。Qwen 的網頁仍標示免費，與即時目錄不同；本輪不以網頁存在當作可呼叫證據，更不能改用無 `:free` 的付費 ID。

Gemini 官方定價表將兩個建議模型的 Standard Free Tier input／output 列為免費；**同一 model ID 在 Paid Tier 仍收費**。因此須先確認該 key 的 project 確實處於 Free Tier／有相應免費額度；僅從公開 pricing 或持有 key 無法證明使用者帳戶零費用。無法確認時不進行 Gemini live send，不能改用其他付費 route。[Gemini pricing](https://ai.google.dev/gemini-api/docs/pricing)、[usage tiers／rate limits](https://ai.google.dev/gemini-api/docs/rate-limits)

兩家都限流，額度與可用性隨帳戶、區域和模型變更：OpenRouter 可由認證的 key metadata 查目前 free-model daily quota，但本研究未呼叫；Gemini 以 AI Studio 顯示的專案限額為準。不要把網路上固定的 RPM／RPD 當成該帳戶保證。遇到 429 保存原稿與 attempt，遵守 Retry-After／明確人工重試，不以換 key、換 route 或付費 fallback 繞過。[OpenRouter limits](https://openrouter.ai/docs/api_reference/limits)、[Gemini limits](https://ai.google.dev/gemini-api/docs/rate-limits)

## 納入規格與後續驗收

這四個 ID 僅供經審查的實驗排程；UI 維持 model ID 自填，模型清單查詢僅作參考。使用者明確保存後才成為當次設定快照。模型價格／route／thinking 能力需在 live plan 前再核對，不把本文快照當成未來保證。

測試不能只檢查「收到文字」：以固定公開短句覆蓋繁中＋英文、簡中＋英文、數字／小數／否定／英文術語／code 保留、口述整理、翻譯與回譯、代寫、原文中的指令注入。人工語意核對與可重複機器檢查分開，另記 time-to-result、API input／output tokens、requested／reported model 與 requested／effective thinking。API 未回傳實際費用時記未知；免費路由 metadata 不是實際帳單 receipt。Unit／integration／真實 API 各保留獨立證據。
