# 多 Provider 與 API 失敗處理研究

查核日期：2026-10-26（Asia/Taipei）。這是原始碼／官方文件研究，**不是已核准的實作計畫或完成驗收**。沒有編譯參考專案、使用 key、啟動代理服務或發出付費請求。

## 研究來源與邊界

已 shallow clone `respanai/respan-gateway` 到忽略追蹤的 `.local/references/respan-gateway`；研究固定於 commit `58f05fc5e6deef3bf5f23facc607d6ca95d83fbe`。該 workspace 標示 `MIT OR Apache-2.0`，並附兩份授權。參考架構不等於把整個專案變成 notype 的依賴；若日後複製實質程式碼，須在 notices 保留所選授權要求的聲明。[workspace](https://github.com/respanai/respan-gateway/blob/58f05fc5e6deef3bf5f23facc607d6ca95d83fbe/Cargo.toml)、[MIT](https://github.com/respanai/respan-gateway/blob/58f05fc5e6deef3bf5f23facc607d6ca95d83fbe/LICENSE-MIT)、[Apache-2.0](https://github.com/respanai/respan-gateway/blob/58f05fc5e6deef3bf5f23facc607d6ca95d83fbe/LICENSE-APACHE)。

## 值得採用的設計

| 參考設計 | 原始碼證據 | 對 notype 的建議（尚未實作） |
| --- | --- | --- |
| 與 transport 分離的失敗種類、穩定錯誤碼與附加 metadata | [Fault／FaultKind](https://github.com/respanai/respan-gateway/blob/58f05fc5e6deef3bf5f23facc607d6ca95d83fbe/crates/core/src/fault/mod.rs)、[HTTP faults](https://github.com/respanai/respan-gateway/blob/58f05fc5e6deef3bf5f23facc607d6ca95d83fbe/crates/wires/http-faults/src/lib.rs) | 讓核心辨別設定錯誤、驗證、認證、額度、限流、網路／TLS、逾時、服務異常、格式異常與取消；UI 使用安全的類型與訊息，不直接顯示 raw upstream JSON。 |
| 把格式、路由與 transport 分開 | [routing](https://github.com/respanai/respan-gateway/blob/58f05fc5e6deef3bf5f23facc607d6ca95d83fbe/crates/plugins/routing/src/lib.rs)、[OpenAI request mapping](https://github.com/respanai/respan-gateway/blob/58f05fc5e6deef3bf5f23facc607d6ca95d83fbe/crates/formats/conversation-http/src/openai/request.rs) | 使用者設定固定當次 provider／endpoint／model snapshot；adapter 只做對應 provider 的欄位轉換，不根據輸入文字猜路由。內部共同請求保留既有 session／history seam。 |
| 連線、等候 response head、串流 idle deadline 分開 | [transport](https://github.com/respanai/respan-gateway/blob/58f05fc5e6deef3bf5f23facc607d6ca95d83fbe/crates/wires/http-transport/src/lib.rs)、[call timeout wrapper](https://github.com/respanai/respan-gateway/blob/58f05fc5e6deef3bf5f23facc607d6ca95d83fbe/crates/wires/http/src/call.rs) | 保留總 deadline／取消，再將連線／讀取階段分開，內網失效不能無限卡住。DNS、TLS、拒絕連線與讀取中斷的 UI 建議需不同。不要為了內網問題停用 TLS 憑證驗證。 |
| Retry eligibility 與執行 retry 是兩件事 | [HTTP retryable／Retry-After](https://github.com/respanai/respan-gateway/blob/58f05fc5e6deef3bf5f23facc607d6ca95d83fbe/crates/wires/http-faults/src/lib.rs)、[fallback](https://github.com/respanai/respan-gateway/blob/58f05fc5e6deef3bf5f23facc607d6ca95d83fbe/crates/plugins/fallback/src/lib.rs) | 解析等待時間，但目前沿用明確人工重試。認證／錯誤參數不能自動重送；取消後不再送，可能已送出的請求仍列入用量與預算。未獲授權不做跨 provider fallback 或競速呼叫。 |
| 有界、增量的 SSE parser；輸出開始後不回放整段 | [SSE byte parser](https://github.com/respanai/respan-gateway/blob/58f05fc5e6deef3bf5f23facc607d6ca95d83fbe/crates/wires/sse/src/lib.rs)、[fallback commit boundary](https://github.com/respanai/respan-gateway/blob/58f05fc5e6deef3bf5f23facc607d6ca95d83fbe/crates/plugins/fallback/src/lib.rs) | 日後串流須處理 UTF-8／CRLF／event 跨 chunk、單 event 大小上限、idle timeout、終止標記、途中 error 與取消；partial output 不得成為可自動交付的成功結果。 |
| 明確的 credential 目標限制 | [credentials plugin](https://github.com/respanai/respan-gateway/blob/58f05fc5e6deef3bf5f23facc607d6ca95d83fbe/crates/plugins/credentials/src/lib.rs) | key 與 provider／目的地綁定。使用者改 endpoint 不能沿用上一家 credential 靜默外送；redirect 不可把 Authorization 帶至另一目的地。 |

以上為採用方向，不建議為桌面 App 預設新增 Axum listener。notype 已有 Tauri IPC；獨立 HTTP server 可留作未來有實際外部 client 需求時的 adapter。`GatewayError` 應先是 Rust 類型，`IntoResponse` 只是 HTTP 外層映射。這是對目前部署／資料邊界的設計判斷。

## 三家 API 的實際差異

### DeepSeek

官方 OpenAI 相容 base URL 為 `https://api.deepseek.com`。使用者指定的 `deepseek-v4-flash` 仍被接受，但官方說明已轉由 `DeepSeek-V4.1-Flash-1017` 服務；目前建議名稱是 `deepseek-flash`。不能把使用者填的 alias 改寫後當成原設定；history 應分別保存 requested model 與 API reported model。[官方起始文件](https://api-docs.deepseek.com/)。

thinking 與 effort 是 provider-specific 欄位，不能把省略參數當作已關閉；其 request 定義含 `thinking`、`reasoning_effort`。stream 使用 SSE，最後的 content chunk 可帶 usage，不能硬假設一定有獨立 usage-only chunk。[Chat Completions](https://api-docs.deepseek.com/api/create-chat-completion/)。

官方明列 400／401／402／422／429／500／503，分別涉及格式、認證、餘額、參數、限流及服務故障。402 不應顯示成一般連線失敗。[錯誤碼](https://api-docs.deepseek.com/quick_start/error_codes/)。

### OpenRouter

OpenAI 相容的 Chat Completions endpoint 為 `https://openrouter.ai/api/v1/chat/completions`；model 是平台 model ID。內建 provider routing／fallback 是另一層行為，不能和 notype 自己是否重送混為一談。[API overview](https://openrouter.ai/docs/api_reference/overview)。

`reasoning` 物件依模型能力決定可否關閉；模型 metadata 可以宣告 mandatory。`reasoning.exclude: true` 只是不回傳推理內容，**不等於停用，也不代表不收 reasoning tokens 費用**。因此開關、effort 與「顯示思考內容」不可混成一個設定。[Reasoning tokens](https://openrouter.ai/docs/guides/best-practices/reasoning-tokens)。

失敗可能出現在 HTTP 非成功狀態，也可能出現在已送出 HTTP 200 後的 JSON／SSE。應檢查 top-level `error`、choice error 及 `finish_reason`，不能只用 `status.is_success()`。typed error code 比比對自然語言 message 穩定；`Retry-After` 只提供等待訊號，並非自動 retry 授權。raw metadata 可能含輸入或供應商細節，不能直接進 diagnostics。[Errors and debugging](https://openrouter.ai/docs/api_reference/errors-and-debugging)。

### Gemini OpenAI compatibility

base URL 為 `https://generativelanguage.googleapis.com/v1beta/openai/`，以 Bearer API key 呼叫 `/chat/completions`。官方指出未給 `reasoning_effort` 會採模型預設；可停用的 2.5 模型可用 `none`，但 2.5 Pro／3 類別有不能關閉的模型。不能以統一 `thinking: false` 宣稱適用全部 Gemini。[OpenAI 相容文件](https://ai.google.dev/gemini-api/docs/openai?hl=zh-tw)。

Gemini 的原生 troubleshooting 建議針對可重試的 429／503 使用 backoff；notype 仍需另外遵守本身取消、預算與人工重試決策，不能直接照搬無界自動重試。[Troubleshooting](https://ai.google.dev/gemini-api/docs/troubleshooting)。

目前 OpenRouter／Gemini 的預設 model ID 未由使用者指定。研究不替使用者選型；未知自訂模型能否停用 thinking，也不應自動猜測。這個產品決策待使用者回覆。

## 研究後的使用者決策

使用者已指定金鑰存放於家目錄 `~/.env`，亦可從 App UI 輸入並保存到 App 自己的設定檔，**App 設定優先**。此決策取代下方原先提出的 OS vault 建議；本期不能改以 vault 當成已決定的需求。規格記於 [#25](https://github.com/swchen44/notype/issues/25)。

規劃採非空 App key 優先，未設定時才讀取所選 Provider 的 `DEEPSEEK_API_KEY`、`OPENROUTER_API_KEY` 或 `GEMINI_API_KEY`；兩處都沒有時禁止發送。後端僅解析需要的變數，UI 顯示來源，不把值帶入 diagnostics、CSV、錯誤格式或 GitHub。這是使用者選定的本機檔案儲存方式，不能宣稱為加密金鑰庫。精確目的地綁定與讀寫行為仍須納入新的實作計畫及 Astra 審查；目前未讀寫使用者 `.env` 或取得 key。

## notype 目前已有什麼、缺什麼

以下為目前 repository source 的唯讀查核，不是已安裝 App 的行為保證：

- `faithful_provider` 已有自訂完整 endpoint／model、Bearer／anonymous policy、Generic／DeepSeek profile、effort／輸出上限、每次請求設定 snapshot、redirect 禁止、deadline／取消、1 MiB body bound、response receipt observer。正式請求走既有 provider → SessionService → SQLite／CSV；不是另寫一套展示 transport。
- 目前 `TextFailure` 只有設定、提示詞、generic failure、timeout、invalid response、cancelled、history storage；非 2xx 目前落成 generic failure。缺少使用者可辨識的 auth／quota／429／server 等分類，`Retry-After` 也尚未在這條路徑呈現。
- OpenRouter HTTP200 error body 目前不會被當作成功文字，但會退成 invalid response，無法提供正確可操作的錯誤原因。需提升分類，並保留收到的 usage，再結束該次 attempt。
- 本輪 DeepSeek helper 使用專屬本機 non-echo console，一次性 key 不需寫入設定。一般 `TextConfiguration` 現有 `api_key: String` 可序列化；**這不是 OS credential vault**。不要宣稱 key 已加密保存或已完成跨平台 keychain。

原始碼：[`faithful_provider`](../../desktop/src-tauri/src/faithful_provider.rs)、[`TextFailure／SessionService`](../../crates/notype-core/src/lib.rs)、[`DeepSeek console`](../../desktop/src-tauri/src/deepseek_probe/console.rs)。

## key 建議與驗收重點

目前有限額 DeepSeek 測試沿用既有本機 non-echo 視窗，key 不經聊天／命令列參數／環境變數／檔案。正式 App 若要記住 key，建議新增 CredentialStore adapter：Windows 使用作業系統 credential 管理，設定只放 credential reference；未來 macOS／Linux 實作各平台 adapter。這是建議，尚未實作；Windows API 能提供 credential store 並不代表防得住相同使用者權限下的所有程序。[Microsoft Credential Management](https://learn.microsoft.com/en-us/windows/win32/secauthn/credentials-management)。

先用自有 loopback HTTP 測試 server，不需要真實 key，也不耗 API 額度：401／403、402、429＋合法／非法 Retry-After、5xx＋HTML／空 body、HTTP200 error、慢 head／慢 body、半包 JSON、超大 body、連線中斷、取消及晚到回應、usage 已回但內容驗證失敗。驗證原稿仍可讀、每次 attempt 記錄不漏、沒有隱式重送、UI 安全分類及 diagnostics 不含輸入／key／raw body。真實 provider 品質、公司 TLS／CA／Proxy 與 native App 仍需分開驗收。
