# GitHub 類 Typeless 開源專案比較

查核日期：2026-10-24。研究目的：公司內網、Windows、接公司內部 LLM API、容易安裝；目前沒有指定開發語言。 使用者補充：公司 pip／npm 有 Proxy，可把自製發行包上傳到能被 Proxy 取得的套件來源；偏好安裝後減少額外下載及 runtime 相依。本文只使用第一方 repository、原始碼、release API 與專案文件。授權欄只記錄名稱，沒有判定公司的具體使用情境。

## 先看結論

**目前已確認只做 Windows amd64 且禁止 installer。優先評估 Handy 的 portable source 機制與公司重包；Sotto 是有既有 portable ZIP 的候選；Electron／TypeScript 可參考 OpenWhispr。** 這是本研究的選型建議，依據下列 source-level 證據，尚未在公司 Windows 電腦驗收。

**使用者已確認公司 API 只有文字輸入的 LLM，沒有公司音訊辨識 API。** 因此本次主路徑採本機 STT 語音辨識，組合是「本機 STT → 公司 LLM 整理文字 → 貼回原本的應用程式」。Handy 與 Sotto 已有這三個環節及自訂 LLM endpoint 的程式碼，不必先從空專案重做全域熱鍵、麥克風、模型管理與跨 App 貼上。[H-settings] [H-llm] [S-provider] [S-clipboard]

三個優先候選的「自訂 API」均不等於任意企業 API 都能直接接。已確認的主要格式是 OpenAI-compatible Chat Completions、model 字串及 Bearer key；若公司使用不同 JSON、SSO、客製 header、mTLS 或其他音訊格式，需要 adapter 或修改，這次沒有企業端點可驗證。

## 版本與證據範圍

| 專案 | 查核 main HEAD，短 SHA／commit 日期 UTC | GitHub latest stable／發布日期 UTC | Windows 發布檔 |
| --- | --- | --- | --- |
| [Handy](https://github.com/cjpais/Handy) | 5a9451e70aed／2026-10-21 | [v0.9.8](https://github.com/cjpais/Handy/releases/tag/v0.9.8)，2026-10-03 | x64、arm64 的 setup.exe、MSI |
| [OpenWhispr](https://github.com/OpenWhispr/openwhispr) | 7a6ae826de9e／2026-10-23 | [v1.10.4](https://github.com/OpenWhispr/openwhispr/releases/tag/v1.10.4)，2026-10-22 | Setup.exe |
| [Sotto](https://github.com/stofll/Sotto) | c8d6d54a4837／2026-10-22 | [v0.3.5](https://github.com/stofll/Sotto/releases/tag/v0.3.5)，2026-10-10 | setup.exe、setup-ui.exe、portable.zip |
| [OpenTypeless](https://github.com/tover0314-w/opentypeless) | f5ad60298f53／2026-10-13 | [v1.1.61](https://github.com/tover0314-w/opentypeless/releases/tag/v1.1.61)，2026-09-29 | x64 setup.exe、MSI |
| [Whispering／Epicenter](https://github.com/EpicenterHQ/epicenter) | 7cc869b4a129／2026-10-17 | [v7.11.0](https://github.com/EpicenterHQ/epicenter/releases/tag/v7.11.0)，2025-12-27 | Whispering x64 setup.exe、MSI |
| [Voicetypr](https://github.com/ideaplexa/voicetypr) | f86e571df5bd／2026-10-15 | [v2.0.9](https://github.com/ideaplexa/voicetypr/releases/tag/v2.0.9)，2026-10-09 | x64 setup.exe |
| [VoiceInk](https://github.com/Beingpax/VoiceInk) | 0019d7a37d80／2026-10-24 | [v2.21](https://github.com/Beingpax/VoiceInk/releases/tag/v2.21)，2026-09-28 | 未提供 Windows 版；macOS 專案 |
| [OmniDictate Windows GUI](https://github.com/shaneholloman/omni-dictate-windows-gui) | e861670bc8b8／2026-07-08 | latest release API 未取得可用 release object | README 說明 Setup.exe 流程；本次未確認對應已發布 asset |

資料由 GitHub connector 的 repository metadata、commits 與 releases/latest GET 取得，精簡快照在 [repository-metadata.json](evidence/github/repository-metadata.json)。main 與 stable source 是分開觀察；本文優先候選的核心 endpoint 能力另外核對了 stable tag 對應 SHA。這些 source SHA 不是執行檔內容驗證或可重現建置證明。後續僅對 Handy v0.9.8 x64 發布物補做實算 hash／靜態解包，見 [06](06-source-feasibility-audit.md)；installer 沒有執行，也不是公司交付方式。

| Stable tag | 核對的 source commit |
| --- | --- |
| Handy v0.9.8 | 14f6f0d31cb22a4268acfbddba3052820fd0a06e |
| OpenWhispr v1.10.4 | ba6d61820b4c56d7b08012c61c2ae6b43f3152f3 |
| Sotto v0.3.5 | 645effd86d31e76f7a9ca9f534874eb80983d41c |
| Whispering v7.11.0 | a6b15c28db9076742d7ae7014f8ec9f97aa615d1 |
| Voicetypr v2.0.9 | 082285c05fa4f1ed3c523073de48f982ceb3731a |
| OpenTypeless v1.1.61 | 72914e8db005d1811f46f2dc50bfd91d1660ae01 |

八個 repository 的 metadata 均為 archived:false；這只表示沒有封存，不能推導 issue 回應速度、維護品質或 Windows 穩定度。

## 功能與導入比較

| 專案 | 開發 stack／授權 | STT | LLM 整理文字／內部 API | 適合這個需求的程度 |
| --- | --- | --- | --- | --- |
| Handy | Tauri、Rust、React／TypeScript；MIT | 本機 Whisper、Parakeet 等模型 | Custom provider，可設 base URL、model、key；POST /chat/completions | **首選基底**：範圍集中，離線模型可手動佈署 |
| Sotto | Tauri 2、Rust、React／TypeScript；MIT | 本機多種引擎，亦可設定 cloud STT | OpenAI-compatible provider、base URL、model、key；可自訂處理 profiles | **免安裝候選**：有 portable.zip，須準備 WebView2 與模型 |
| OpenWhispr | Electron、React、TypeScript、whisper.cpp、sherpa-onnx；MIT | 本機或 cloud／self-hosted | LAN cleanup 與 Custom LLM；一般 Self-Hosted STT 的認證另有邊界 | **Electron 參考首選**：功能廣，需縮小公司的產品範圍 |
| OpenTypeless | Tauri、Rust、React／TypeScript；MIT | Windows 的 Local/Custom Whisper 接本機／內網 HTTP STT server；另有 cloud STT，Apple Speech 限 macOS | 可設 OpenAI-compatible base URL、model、key | **重要功能參考**；Windows 需另外準備 STT 服務與模型 |
| Whispering v7.11.0 | Tauri、Rust、Svelte 5／TypeScript；AGPLv3 系列 | 本機 Whisper C++、Speaches 或 cloud | Custom OpenAI-compatible transformations | **舊穩定版本可研究**；目前 main 已改成不同產品範圍 |
| Voicetypr | Tauri 2、Rust、React；AGPL-3.0 | Windows 本機 Whisper；macOS 另有 Parakeet；可 cloud／LAN | Custom OpenAI-compatible formatting | 有參考價值；預編譯產品有 trial／lifetime license 與外部授權服務 |
| VoiceInk | Swift 原生 macOS；GPL-3.0 | 本機及 cloud STT | Custom AI provider、model、key、base URL | **功能設計參考**；Windows 需要重新實作 OS 整合 |
| OmniDictate Windows GUI | Python、PySide6、faster-whisper／PyTorch、PyInstaller；CC BY-NC 4.0 | 本機 STT | 這次未確認通用內部文字 LLM endpoint 設定 | Python 封裝參考；授權與 API 能力不符合優先條件 |

表格依據各節連結。語言支援、速度、辨識率、企業 TLS／proxy、目標 App 貼上成功率均沒有實測；不能以專案自述的離線或準確度宣傳當作公司驗收。

## 1. Handy：最適合先接公司文字 LLM

Handy 的主要互動是按住熱鍵錄音、放開後在本機辨識，再把結果送進目前輸入位置；README 列出 Windows、macOS、Linux。[H-readme]

stable v0.9.8 的 settings.rs 已有 post_process_enabled、providers、models、prompts、selected prompt 與 Custom provider。Custom 預設 base URL 是 localhost 的 /v1，但 allow_base_url_edit:true；因此它是可配置能力，接相容內部 API 不必先 fork。[H-settings]

llm_client.rs 用設定值組成「base URL + /chat/completions」，有非空 key 時加入 Authorization: Bearer；模型和 prompt 進 request，並另有 /models 查詢。這能支援公司相容的文字 API；本次沒有確認 UI 能設定任意 header、企業 SSO 或 client certificate。[H-llm]

跨 App 送字有剪貼簿保存／寫入／貼上／恢復流程，也支援不同貼上方法與直接鍵入。這些 OS 細節可直接作為公司的實作參考，仍須逐個目標程式驗收。[H-clipboard]

內網準備：README 明列 Windows app-data 路徑、models 資料夾及手動放入 .bin／.gguf 等模型方式。應先在可連外環境取得安裝包、確定模型與相依 runtime，再建立公司佈署包。程式含 GitHub updater endpoint 與可控制的 update_checks_enabled；「本機 STT」不能推導「完全不連外」。[H-models] [H-config] [H-settings]

本次對 main 的 src、src-tauri/src、package.json 搜尋 telemetry／posthog／sentry／analytics 沒有命中；這只是一個有範圍的負面搜尋，沒有做封包觀測，不能宣稱所有 runtime 路徑都零遙測。

## 2. Sotto：portable.zip 對受限 Windows 有吸引力

Sotto 支援全域熱鍵、push-to-talk／toggle、overlay、本機 STT、字典與替換、歷史紀錄，及可選的 LLM profiles。[S-readme]

stable v0.3.5 的 OpenAI-compatible provider 可以接指定 base_url、model 與 API key，實際 POST /chat/completions 並帶 Bearer header。這是已存在的原始碼能力；仍需核對公司模型對 request 參數與 response 的相容性。[S-provider]

Windows 的剪貼簿管線特別處理 focus restoration；Ctrl+V 用 raw virtual-key SendInput，避免受非拉丁鍵盤配置影響。程式也有失敗處理與不同策略，不能把原始碼中的 fallback 當作所有 App 都成功。[S-clipboard]

portable.zip 解壓後直接啟動 Sotto.exe；旁邊的 portable.flag 啟用 data 資料夾模式，保存設定、模型、history、logs。**API key 仍存在 Windows Credential Manager，換機要重新設定；portable 仍需要 Microsoft Edge WebView2 Runtime。** portable 不註冊 autostart，停用 installer updater。[S-portable]

公司內網建議先評估 portable 發布物加預先準備的模型與 WebView2。模型目錄及外部行為見 privacy 文件：可連模型主機、配置的 API、GitHub 更新與 opt-in PostHog EU。穩定版文件說明 telemetry 在同意前關閉、portable/debug 不檢查可安裝更新；本次沒有 runtime 網路驗證。[S-privacy]

主介面目前以俄文／英文為主；繁中、中英夾雜、專有名詞與模型選擇需要另外量測。不要把 Parakeet、Whisper 或其他模型名稱視為同等語言支援。

## 3. OpenWhispr：Electron／TypeScript 的完整參考

第一方 README 的範圍已包含語音輸入、notes、meeting transcription 與 agents，桌面支援 Windows、macOS、Linux。這比單純 Typeless 類 MVP 廣；若採用，建議先保留錄音、STT、cleanup、貼上四個環節。[O-readme]

stable v1.10.4 的 lanProvider 會讀 cleanupRemoteUrl、model 與 custom API key，組成 /chat/completions 後呼叫共用處理；因此公司相容文字 LLM 有現成接點。另有 Custom OpenAI provider 的 endpoint 選擇與 fallback，實際格式仍須核對。[O-lan] [O-openai]

**Self-Hosted STT 與文字 LLM 的 key 設定不能混為一談。** SelfHostedPanel 主要提供 server URL 與 model；transcriptionAuth 對一般 self-hosted STT skip key，Orukeet 是例外。官方 custom-ASR shim 文件也說明一般路徑送 multipart 音訊、期待 JSON text，非相容 API 可加本機 shim，讓 shim 保存上游 key。這不代表其他 BYOK Custom 路徑一定不能帶 key；本次結論限定此 Self-Hosted 路徑。[O-selfhost] [O-auth] [O-shim]

Windows 貼上包含 native fast-paste helper 與其他 fallback；Electron 打包安裝後不要求一般使用者另裝 npm。公司自行建置時，則需要 Node／package manager、native helpers 與相依建置環境。[O-clipboard] [O-readme]

離線邊界：local Whisper 文件說明模型首次下載與可選 GPU runtime 的按需下載；音訊選 local 不會自動解決模型、更新、雲端／account 功能等所有外部依賴。README 的「no telemetry」是作者聲明，本次沒有網路封包驗證。[O-local] [O-readme]

## 4. Whispering／Epicenter：必須分開舊 release 與新 main

stable v7.11.0 的 README 明確描述「錄音 → transcribe → transform → copy/paste at cursor」，列出 Windows、macOS、Linux、Whisper C++、Speaches 與 Custom OpenAI-compatible transformation endpoint。對理解可配置的多步文字處理很有參考價值。[W-stable] [W-custom]

但 main 7cc869b... 的 README 已描述另一個 development milestone：獨立的 voice notes app，錄音與結果以檔案放進 Git 資料夾，STT 和文字處理各有 endpoint。它明列未支援 system-wide dictation、global shortcut、push-to-talk、自動貼上、on-device STT；Windows／Linux 未測試，也沒有 release pipeline。[W-main]

因此搜尋結果、網站簡介、main 和 v7.11.0 exe 不能混用。要重現舊語音輸入產品，應固定 v7.11.0 的 source／文件／assets 再驗收；不要直接從最新 main 開始，期待得到舊版功能。這也是本研究不把它列為第一個 fork 的原因。

## 5. Voicetypr：能力接近，但預編譯產品有授權服務

stable v2.0.9 README 列出 Windows 10／11、Tauri／Rust／React、本機 Whisper、選配 cloud STT、自訂 OpenAI-compatible formatting、LAN Network Sharing 與 transcript history；預編譯 Windows exe 已發布。[V-stable]

README 同時寫明 trial 及 lifetime license，不能把「source 開放」解讀成「官方預編譯產品完全沒有啟用要求」。main license API client 的 release 路徑固定到 api.voicetypr.com，包含 trial/check、license/validate、activate 等呼叫；公司隔離網路上的初次啟用與離線容忍期限這次未驗證。[V-license-api]

其 source 為 AGPL-3.0，適合參考 Rust audio／insertion、GPU sidecar 與 LAN 設計；是否採用預編譯版或自行 fork，需要先把產品啟用和公司佈署方式釐清。[V-license]

## 6. VoiceInk：macOS 功能參考，不是 Windows 現成基底

VoiceInk 是 Swift 原生 macOS 專案，README 要求 macOS 15+，列有 app／URL modes、context awareness、字典、熱鍵、push-to-talk 與 AI assistant。其準確率及隱私描述是作者宣傳，這次沒有實測。[I-readme]

原始碼的 CustomAIProviderManager 確實保存 baseURL、model 與 key；AIChatCompletionService 的 custom branch 會使用該組配置，適合研究 app-context 與模型設定的產品設計。[I-custom] [I-chat]

公司 Windows 需要重新做熱鍵、音訊、輸入及權限整合。此專案為 GPL-3.0；README 說明官方下載有 trial／購買方案、可自行從 source build，並表示目前不收 PR。不要把它當作跨平台 npm／Rust App。[I-readme] [I-license]

## 7. OmniDictate Windows GUI：Python 打包參考，非優先採用

這個 repository 的 README 描述 Windows 本機 dictation、VAD／push-to-talk 與直接輸入；requirements 列出 PySide6、faster-whisper、torch、pynput、pywinauto、PyInstaller，可研究 Python 音訊應用如何封成 Windows 安裝包。[D-readme] [D-requirements]

README 明列首次使用下載模型、安裝包不含模型與 NVIDIA driver，部分 Windows 需要 VC++ Redistributable。因此「一個 exe」與「冷啟動完全不連外、不需其他 runtime」是不同驗收項目。本次也未確認通用內部文字 LLM API 的設定入口。[D-readme]

LICENSE 名稱為 Creative Commons Attribution-NonCommercial 4.0 International；與 MIT 候選不同，本文將它列為公開原始碼參考，不把它當作同等的公司採用優先候選。本次 releases/latest 未取得可用 release object，沒有確認下載資產，不應按 README 文件就宣稱已驗證安裝。[D-license]

## 8. OpenTypeless：功能最直接相似，Windows 本機 STT 需另備服務

OpenTypeless 是 MIT 專案，使用 Tauri、Rust、React／TypeScript。README 列出 dictation、app-aware style、selected-text editing、Ask Anything、翻譯、字典、scenes 與 Windows SendInput，與這次研究的 Typeless 互動相近；這些產品描述仍需 runtime 驗收。[T-readme] [T-license]

stable v1.1.61 有 Windows x64 setup.exe 與 MSI，release metadata 記錄的大小分別為 4,964,818 與 6,868,992 bytes。小安裝包不能代表已包含 STT 推論引擎、模型或 WebView2。已核對 stable source：LLM config 有 base_url 與 model；protocol.rs 會組成 /chat/completions，帶配置的 model、messages 和選配 streaming，並使用 Bearer key。因此公司相容文字 LLM 是可配置入口，不必只為改 URL 而 fork。[T-llm] [T-protocol]

**Windows 的 Local/Custom Whisper 需要另外部署 STT server。** stable stt/config.rs 預設為 http://localhost:8000/v1、模型 Systran/faster-whisper-large-v3；whisper_compat.rs 將錄音轉成 WAV，以 multipart HTTP 送到 /audio/transcriptions。這是本機／內網服務的 client 路徑，與 Handy／Sotto 在 App 內載入模型的安裝方式不同。非空 STT key 會作為 Bearer header；Apple Speech 的本機 OS 路徑限 macOS。[T-stt-config] [T-stt-http] [T-stt-mod]

已知公司只有文字 LLM，所以採此專案時，還要把本機 Whisper-compatible 服務、Python／容器或其他 server runtime、模型和啟動管理一起包好。此處的服務佈署與套件選擇是本研究建議，這次未建立或驗證該服務。這個額外成本讓前三推薦仍維持 Handy、Sotto、OpenWhispr；OpenTypeless 則是 app-context／語音改寫互動的重要參考。

帳號與付費 cloud 是選配；README 表示 BYOK 核心流程不需要 OpenTypeless 帳號，provider keys 直接交給配置的服務，不能據此推導整個 App 沒有外部連線。stable 的 UpdatePrompt 在元件啟用時呼叫 updater check，tauri.conf.json 的 endpoint 是 GitHub Releases。選讀的 package.json、lib.rs、api.ts、authStore.ts 未命中 telemetry／analytics／PostHog／Sentry，搜尋邊界有限，也沒有封包觀測，因此遙測全貌仍是 unknown。預先準備時須涵蓋 STT server／模型、Windows runtime 和更新策略；BYOK 不等於所有外部路徑已停用。[T-readme] [T-updater] [T-config]

## 依公司條件採取的下一步

| 公司條件 | 建議優先項目 | 實際需要核對 |
| --- | --- | --- |
| **目前已知：只有文字 LLM API**；希望最快看到成果 | Handy stable + 本機 STT + Custom post-processing | API 格式、Bearer／header、模型 ID、繁中 STT、貼上成功率 |
| 不方便安裝；允許執行解壓後程式 | Sotto stable portable + 預先準備模型 | WebView2、公司允許執行的檔案位置、Credential Manager、模型與 DLL |
| 公司已有 npm／TypeScript／Electron 經驗 | OpenWhispr 的 dictation subset | native helper、安裝大小、關閉無關外部路徑、內部 API adapter |
| 沒有開發 stack，只希望同事安裝可用 | 先驗收 portable 完整payload，之後才決定 fork 語言 | 使用者不應為每台機器另裝 Rust／Python／npm；runtime 與模型需由發行包處理 |
| 打算從零開發小型研究 PoC | Python 適合先驗證錄音→STT→LLM；正式發行再比較 Tauri／Electron | PyInstaller 封裝、DLL／模型大小、熱鍵、剪貼簿、更新與可重現建置 |

建議測試順序：先用 10～20 段非機密繁中／中英夾雜樣本，比較本機 STT，再接公司文字 LLM；最後量測從放開熱鍵到文字出現的時間、被 LLM 改錯的內容，以及在實際目標 App 的貼上結果。這是建議的 acceptance 工作，這次尚未執行。

## 本次完成與未驗證

- 已完成：8 個 GitHub 候選的 live repository 查詢、main SHA、可用 stable release 與 Windows assets metadata；三個優先候選及 OpenTypeless 的 stable 核心 API source；Whispering 版本差異；安裝／模型／授權服務／外部網路文件邊界。
- 未執行：任何 Windows 發布檔的執行／安裝、CPU／GPU效能測試、語音辨識準確度、企業 proxy／TLS／SSO、內部 API 呼叫、斷網冷啟動與封包觀測。只有 Handy x64 資產後續補做下載實算 digest／靜態解包，其他資產仍只有 metadata。
- 不確定：Windows CPU／RAM／GPU、允許的安裝與執行方式、公司 endpoint 的精確 wire format。公司 API 只有文字輸入、pip／npm Proxy 與可自行發套件已確認；尚未確認的是 API wire format、可用硬體、Cargo／native build 工具取得方式及具體套件來源。

## 第一方來源

下列 URL 固定到查核的 commit；release 與 metadata 快照另見前表。只引用程式碼與文件，不把專案內的 plan／spec 當成已實作功能。

[H-readme]: https://github.com/cjpais/Handy/blob/5a9451e70aedc90648ce2befd9d7e0304746f9c1/README.md
[H-models]: https://github.com/cjpais/Handy/blob/5a9451e70aedc90648ce2befd9d7e0304746f9c1/README.md#manual-model-installation-for-proxy-users-or-network-restrictions
[H-settings]: https://github.com/cjpais/Handy/blob/14f6f0d31cb22a4268acfbddba3052820fd0a06e/src-tauri/src/settings.rs
[H-llm]: https://github.com/cjpais/Handy/blob/14f6f0d31cb22a4268acfbddba3052820fd0a06e/src-tauri/src/llm_client.rs
[H-clipboard]: https://github.com/cjpais/Handy/blob/5a9451e70aedc90648ce2befd9d7e0304746f9c1/src-tauri/src/clipboard.rs
[H-config]: https://github.com/cjpais/Handy/blob/14f6f0d31cb22a4268acfbddba3052820fd0a06e/src-tauri/tauri.conf.json
[S-readme]: https://github.com/stofll/Sotto/blob/c8d6d54a4837a7b4fccefd45e70ad8e6d5e6dd5b/README.md
[S-provider]: https://github.com/stofll/Sotto/blob/645effd86d31e76f7a9ca9f534874eb80983d41c/desktop/src-tauri/src/ai/providers.rs
[S-clipboard]: https://github.com/stofll/Sotto/blob/c8d6d54a4837a7b4fccefd45e70ad8e6d5e6dd5b/desktop/src-tauri/src/clipboard.rs
[S-portable]: https://github.com/stofll/Sotto/blob/645effd86d31e76f7a9ca9f534874eb80983d41c/docs/portable.md
[S-privacy]: https://github.com/stofll/Sotto/blob/645effd86d31e76f7a9ca9f534874eb80983d41c/docs/privacy.md
[O-readme]: https://github.com/OpenWhispr/openwhispr/blob/7a6ae826de9ec1caf65f3a6e769475dab2ae829f/README.md
[O-lan]: https://github.com/OpenWhispr/openwhispr/blob/ba6d61820b4c56d7b08012c61c2ae6b43f3152f3/src/services/ai/inferenceProviders/lan.ts
[O-openai]: https://github.com/OpenWhispr/openwhispr/blob/7a6ae826de9ec1caf65f3a6e769475dab2ae829f/src/services/ai/inferenceProviders/openai.ts
[O-selfhost]: https://github.com/OpenWhispr/openwhispr/blob/ba6d61820b4c56d7b08012c61c2ae6b43f3152f3/src/components/SelfHostedPanel.tsx
[O-auth]: https://github.com/OpenWhispr/openwhispr/blob/ba6d61820b4c56d7b08012c61c2ae6b43f3152f3/src/helpers/transcriptionAuth.js
[O-shim]: https://github.com/OpenWhispr/openwhispr/blob/7a6ae826de9ec1caf65f3a6e769475dab2ae829f/examples/custom-asr-shim/README.md
[O-clipboard]: https://github.com/OpenWhispr/openwhispr/blob/7a6ae826de9ec1caf65f3a6e769475dab2ae829f/src/helpers/clipboard.js
[O-local]: https://github.com/OpenWhispr/openwhispr/blob/7a6ae826de9ec1caf65f3a6e769475dab2ae829f/LOCAL_WHISPER_SETUP.md
[W-stable]: https://github.com/EpicenterHQ/epicenter/blob/a6b15c28db9076742d7ae7014f8ec9f97aa615d1/apps/whispering/README.md
[W-custom]: https://github.com/EpicenterHQ/epicenter/blob/a6b15c28db9076742d7ae7014f8ec9f97aa615d1/docs/guides/custom-endpoint-transformations.md
[W-main]: https://github.com/EpicenterHQ/epicenter/blob/7cc869b4a1295e2566b61e1a4d5967a89f413b9b/apps/whispering/README.md
[V-stable]: https://github.com/ideaplexa/voicetypr/blob/082285c05fa4f1ed3c523073de48f982ceb3731a/README.md
[V-license-api]: https://github.com/ideaplexa/voicetypr/blob/f86e571df5bd5c0a4d8950095167b45185fa59d4/src-tauri/src/license/api_client.rs
[V-license]: https://github.com/ideaplexa/voicetypr/blob/f86e571df5bd5c0a4d8950095167b45185fa59d4/LICENSE
[I-readme]: https://github.com/Beingpax/VoiceInk/blob/0019d7a37d803e2dc95c122df63cdf7567979828/README.md
[I-custom]: https://github.com/Beingpax/VoiceInk/blob/0019d7a37d803e2dc95c122df63cdf7567979828/VoiceInk/Features/ModelLibrary/State/CustomAIProviderManager.swift
[I-chat]: https://github.com/Beingpax/VoiceInk/blob/0019d7a37d803e2dc95c122df63cdf7567979828/VoiceInk/Infrastructure/Providers/Enhancement/Chat/AIChatCompletionService.swift
[I-license]: https://github.com/Beingpax/VoiceInk/blob/0019d7a37d803e2dc95c122df63cdf7567979828/LICENSE
[D-readme]: https://github.com/shaneholloman/omni-dictate-windows-gui/blob/e861670bc8b8d693743b9300d25f6a24b1ad13dd/README.md
[D-requirements]: https://github.com/shaneholloman/omni-dictate-windows-gui/blob/e861670bc8b8d693743b9300d25f6a24b1ad13dd/requirements.txt
[D-license]: https://github.com/shaneholloman/omni-dictate-windows-gui/blob/e861670bc8b8d693743b9300d25f6a24b1ad13dd/LICENSE


[T-readme]: https://github.com/tover0314-w/opentypeless/blob/72914e8db005d1811f46f2dc50bfd91d1660ae01/README.md
[T-license]: https://github.com/tover0314-w/opentypeless/blob/72914e8db005d1811f46f2dc50bfd91d1660ae01/LICENSE
[T-llm]: https://github.com/tover0314-w/opentypeless/blob/72914e8db005d1811f46f2dc50bfd91d1660ae01/src-tauri/src/llm/openai.rs
[T-protocol]: https://github.com/tover0314-w/opentypeless/blob/72914e8db005d1811f46f2dc50bfd91d1660ae01/src-tauri/src/llm/protocol.rs
[T-stt-config]: https://github.com/tover0314-w/opentypeless/blob/72914e8db005d1811f46f2dc50bfd91d1660ae01/src-tauri/src/stt/config.rs
[T-stt-http]: https://github.com/tover0314-w/opentypeless/blob/72914e8db005d1811f46f2dc50bfd91d1660ae01/src-tauri/src/stt/whisper_compat.rs
[T-stt-mod]: https://github.com/tover0314-w/opentypeless/blob/72914e8db005d1811f46f2dc50bfd91d1660ae01/src-tauri/src/stt/mod.rs
[T-updater]: https://github.com/tover0314-w/opentypeless/blob/72914e8db005d1811f46f2dc50bfd91d1660ae01/src/components/UpdatePrompt.tsx
[T-config]: https://github.com/tover0314-w/opentypeless/blob/72914e8db005d1811f46f2dc50bfd91d1660ae01/src-tauri/tauri.conf.json
