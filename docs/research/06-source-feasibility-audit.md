# Handy 固定版本原始碼可行性追查

查核日期：2026-10-24。主體為 **Handy v0.9.8，commit `14f6f0d31cb22a4268acfbddba3052820fd0a06e`**，另靜態檢查同版 Windows x64 setup.exe。本輪已確認的目標是 Windows amd64／x64、個人 pilot、核心 dictation MVP，並接受修改現有開源 App；輸入主要為繁中夾英文、忠實整理，首批目標為 browser／VS Code／Teams／Outlook／Windows Terminal。本文件是使用者授權下載 GitHub 後的事實研究；其他產品取捨仍待 grilling 訪談確認。 使用者後續已確認公司版不可用 installer；本文 installer 僅為先前下載的靜態研究輸入，從未執行，不能作為交付方案。LLM 失敗的產品決策已改為保留原稿並停下待選，以下仍清楚標示 upstream 行為。

完整 source archive 已從 GitHub codeload 取得，4,513,305 bytes，SHA-256 `a6bcd28cfb83dee566328432ce1d15e2a4f881d0dee8af8ca7a4d2f5499488f6`。工作副本在 `/private/tmp/handy-stable-source-audit/Handy-14f6f0d31cb22a4268acfbddba3052820fd0a06e`，沒有執行 App、Cargo build、套件安裝、公司 API 或發布。選讀身分另見 [audit metadata](evidence/github/handy-stable-source-audit.json)。

## 1. 對本案最重要的發現

| 已確認的原始碼事實 | 對設計的影響 |
| --- | --- |
| Windows x64 使用 dynamic transcription backends，CI 另帶 ONNX Runtime、VC++ runtime DLL | Rust 應用也需要完整 binary payload；公司端可以免編譯，但不能只分發 handy.exe |
| 已有 portable mode、`Data/models` 與本機模型掃描 | npm／wheel 配送可重用資料路徑，不一定要先修改辨識核心 |
| 本機預載模型可以被啟動時偵測及選取；model load 取得本機 path | 模型可隨公司套件預先放好，不必把初次辨識綁在 public model download |
| 自訂文字 LLM 已支援 base URL、model、Bearer key、prompt | 相容的公司 API 可用設定接入，特殊 schema／SSO／mTLS 仍需 adapter 或改碼 |
| LLM 整理失敗時回到原逐字稿，後續仍可自動貼上 | 這是產品行為，需要使用者決定是否接受；原碼沒有把所有失敗都改成「停下待確認」 |
| 專案根 package 是 private、postinstall 會呼叫 Bun；Windows build 另需 Rust／C++／CMake／Vulkan 等 | 研究案應另做配送 wrapper，內含預編譯 payload；不能把原始碼目錄當成可直接 npm 發行的產品 |

表內依據為下列固定 source；第 9 節另記錄實際 release asset 的 parser-emitted DLL 清單。兩者均不等於 Windows 安裝或效能驗證。

## 2. Windows 建置與 runtime 的邊界

[BUILD.md][build] 列出 Rust stable、Bun、Tauri prerequisites；Windows 另需 Microsoft C++ Build Tools、CMake、Vulkan SDK。根 [package.json][package] 用 `tsc && vite build` 建置前端，`tauri.conf.json` 的 build hooks 用 Bun。Cargo 還有 registry 與 Git dependencies，所以公司 npm／pip Proxy 不會自動供應所有 Rust／native build 來源。[cargo] [config]

這些是 build 機相依，不需要每位使用者都安裝。若在可建置的 Windows 機產生 binary，再透過公司可取得的套件來源配送，終端可使用預編譯成品。公司是否已有對應 build 工具及允許來源，本次無法由這台 Mac 的環境證明。

### Windows x64 的 native 組合

- Windows x64 的 `transcribe-cpp` 開啟 `dynamic-backends`、`vulkan`。Cargo 另有不同的 ARM64 組合，該平台不在本次產品目標，因此不拿 ARM64 的 static／CPU-only 行為套用 x64。[cargo]
- ONNX 路徑是 `transcribe-rs` 的 `onnx` feature。Windows x64 CI 取得 ONNX Runtime 1.24.2 並設 `ORT_LIB_LOCATION`、`ORT_PREFER_DYNAMIC_LINK=1`；ARM64 不走這個 x64 dynamic ORT 步驟。Whisper-family 與 ONNX-family 的 engine／runtime 需分開看。[cargo] [ci]

### DLL 有明確 staging 程式碼

[build.rs][buildrs] 的 `stage_transcribe_runtime_libs()` 從 `DEP_TRANSCRIBE_CPP_RUNTIME_DIR` 與 `MODULE_DIR` 收集 shared core libraries 及 ggml backend modules；`stage_onnxruntime_dll()` 在動態連結條件下複製 `onnxruntime.dll`；`stage_vc_runtime_dlls()` 依 CI 提供的 redist 路徑複製 `msvcp140*`、`vcruntime140*`、`vcomp140*` families，並檢查必要檔案。

[Windows config][winconfig] 把 `transcribe-libs` 映射到 executable 旁。因此公司包應以完整 staging／bundle 內容為起點。第 9 節的實際 asset 靜態解包確認根目錄有 23 個 DLL，但**依賴 closure 尚未建立，也未驗證安裝／啟動**。自行 build 若沒提供對應環境變數，部分 staging 函式會 no-op，結果也不能直接視為官方 CI 等價。

### WebView2 仍在 runtime 範圍

選讀的兩份 tauri JSON 沒有指定 `webviewInstallMode`。NSIS template 的 `Section WebView2` 有 bootstrapper download、embedded bootstrapper、offline installer 等分支；download 分支使用 Microsoft URL。[config] [winconfig] [nsis]

這證明 installer 有相關機制，沒有證明本次 release asset 展開後採何種實際配置。公司發行需選擇已裝 WebView2、離線 runtime 或固定 runtime，並在乾淨 Windows 驗收；不能由 Rust 或 Tauri 名稱推論零 runtime。

## 3. 模型預載與無初次下載的接縫

[ModelManager::new][model] 用 `portable::app_data_dir(app_handle)/models` 建立模型根目錄，先讀 bundled offline catalog、掃描 `.bin`／`.gguf` 自訂模型及 HF cache，再執行 model migration、`update_download_status()`、`auto_select_model_if_needed()`。選讀的初始化流程沒有為「已存在、可辨識的模型」強制發起模型下載。

`get_model_path(model_id)` 先檢查模型已可用／非下載中，再從 HF cache 或本機 models 目錄回傳完整 path；缺檔時回報 unavailable，不會在這個方法內下載。`TranscriptionManager::load_model_with_device()` 透過這個方法取 path 後載入 engine。[model] [transcription]

但檔案存在／catalog metadata 不等於模型完全可用。目錄型 ONNX 模型須保留正確子檔案與目錄結構；模型與 engine 格式須匹配。本次沒有實際載入權重或驗證其 inference。

可採的兩條原始碼接縫：

1. **預先放入 user models。** 一般安裝模式是 App data 下的 models，Windows README 列 `%APPDATA%/com.pais.handy/models`。Onboarding 有 `handleSelectExistingModel`，可選已有模型；`handleDownloadModel` 是另一條明確下載路徑。[readme] [onboarding]
2. **隨 app resources 帶模型。** `migrate_bundled_models()` 目前只列 `ggml-small.bin`，若 resource 裡有該檔，會在 user models 尚無檔案時複製。其他模型不能僅因放入 resources 就假設也會被這個 migration 搬入；可由 wrapper staging 到 models，或修改其清單／路徑。[model]

這能支援避免第一次模型下載的公司包設計；仍需另外處理 public updater 及其他外部路徑，不能把「模型已預載」當成「全 App 不連外」證明。[updates]

## 4. Handy 已有 portable source 機制

[portable.rs][portable] 的 `init()` 在 executable 旁找 `portable` marker。新建時 marker 內容應以 `Handy Portable Mode` 開頭；空檔案只有在已有 `Data/` 的 migration 情境才會啟用。進入 portable mode 後資料根為 exe 旁的 `Data/`，同時設定 `HF_HOME=Data/huggingface`。

因此可準備下列**未建置／未驗收的公司 payload 示意**：

```text
CompanyHandy/
  handy.exe
  native-runtime-dlls...
  resources/...
  portable                 # 內容為 Handy Portable Mode
  Data/models/<fixed-model-files>
```

這是 source 機制，**不代表 GitHub 已有本次驗證過的官方 portable ZIP**。公司 package wrapper 若把持久 Data 放在 npm／pip 自己管理的安裝目錄，更新或解除安裝可能影響資料；較穩定的設計是把 payload staging 到明確的每使用者可寫目錄，保留 Data，再啟動 exe。此處為工程建議，尚待訪談決定資料位置與更新方式。

## 5. 自訂文字 LLM 與失敗行為

[settings.rs][settings] 有 `post_process_*` providers、model、key、prompt 與選定 prompt；Custom provider 的 base URL 可編輯。啟用 UI 設定後，實際呼叫仍與 action binding 有關：`ACTION_MAP` 區分 `transcribe` 的 `post_process:false` 與 `transcribe_with_post_process` 的 `true`。公司版本要驗收的是使用者實際觸發的 binding，不能只檢查開關存在。[actions]

`post_process_transcription()` 的流程：

1. 空逐字稿、缺 provider／model／prompt 等情況直接回 None。
2. 讀取所選 key；Custom 等 provider 會嘗試停用 reasoning。
3. 支援 structured output 的 provider 可走 schema 分支；否則將 prompt 的 `${output}` 替換成逐字稿，作為 legacy user content。
4. API 成功後清除部分 invisible chars／前導 think block；失敗回 None。[actions]

[llm_client.rs][llm] 以 `base_url.trim_end_matches('/') + /chat/completions` 組 URL，送 model、messages、`stream:false`；非空 key 對一般 provider 帶 Bearer。Custom UI 沒有因此取得任意 header、企業 SSO 或 mTLS 能力，這些仍需針對公司 API 查核。

該 client 在停用 reasoning 參數遭 400／422 拒絕時，有一次移除這些參數的重試及 endpoint／model 記憶。這有助相容性，也表示第一次請求可能多一次往返。選讀的 `create_client()` 僅建立 default headers，沒有設定應用明確 timeout；本次不替 reqwest default 或全流程補一個未觀察的 timeout 數字。[llm]

**關鍵產品行為：** `process_transcription_output()` 先把 `final_text` 設為原逐字稿，只在整理回 Some 時替換；其後 stop action 仍會將非空 `final_text` 送到 paste。也就是 LLM 失敗可以讓原稿自動貼上。另若 API 回 Some 空字串，後續空結果分支不 paste；不能概括為每種 API 異常都自動保留／插入原稿。[actions]

使用者後續已選擇公司版本在整理失敗時**停止並讓人選擇重試或貼原稿**。這是與 upstream 不同的產品要求，尚未實作；不能由「沿用 Handy」就宣稱現成 App 已符合。

## 6. 主要程式碼路徑

| 階段 | 固定版本 file／symbol |
| --- | --- |
| 快捷鍵接收 | `shortcut/handy_keys.rs`／`tauri_impl.rs` → `handler::handle_shortcut_event` |
| 按住／toggle 協調 | `TranscriptionCoordinator::send_input` → `run_effect` → `start`／`stop` → `ACTION_MAP` |
| 錄音開始 | `TranscribeAction::start` → model／VAD load kickoff → `AudioRecordingManager::try_start_recording` |
| 錄音停止／辨識 | `TranscribeAction::stop` → `stop_recording` → streaming result 或 `TranscriptionManager::transcribe(samples)` |
| 文字整理 | `process_transcription_output` → `post_process_transcription` → `llm_client::send_chat_completion*` |
| 歷史與送字 | action 保存 WAV／history 相關結果，再在主執行緒呼叫 `utils::paste`；實作由 `clipboard::paste` 選 clipboard／direct 等方法 |

來源：[shortcut handler][handler]、[coordinator][coordinator]、[actions][actions]、[clipboard][clipboard]。此為 source-level 連結；沒有證明在公司目標 App、IME、RDP 或高權限視窗中輸入成功。

## 7. npm／pip 包裝的具體工作

源碼根 [package.json][package] 的 `private:true` 及 Bun postinstall 表示它不是可直接上傳的同事配送包。另建 wrapper 可包含完整預編譯 bundle，負責選平台、staging 到可寫目錄、初始化 marker／模型、啟動 native process；公司設定與使用者秘密／歷史應分開。

模型 package 與 App package 分別鎖版可減少更新下載；wrapper 須把模型放到上述能被掃描的路徑，或新增明確 model-dir 設定。僅安裝一個模型 wheel／npm package 到其預設目錄，Handy 不會因此自動知道位置。

要保留的工程未知是 Windows release binary 的實際 DLL closure、公司 WebView2、CPU／GPU 支援、Proxy／registry 上限、公司 API schema／認證、資料目錄權限與更新策略。這些是需要環境 evidence 或進一步實作的事項；安裝前是否允許模型佔用一定空間、是否自動貼上、是否保留 history 則是訪談決策。

## 8. License 與證據狀態

固定版本根 [LICENSE][license] 是 MIT；Cargo manifest 及 Tauri bundle license 亦為 MIT。這只記錄所讀文件名稱，不替第三方模型、native libraries 或公司的發布情境作完整授權判定。

已完成：固定 SHA source archive 下載、選讀原始碼及配置、主要功能路徑與 packaging seams 追查，以及第 9 節限定範圍的 release asset hash／靜態解包。未執行：Windows build、完整 binary import inventory、runtime、模型推論、公司 API、npm／wheel 建置或 publish。

## 9. Windows x64 發布物：實際下載與靜態解包

下載對象為 [Handy_0.9.8_x64-setup.exe][asset]。實際 **19,615,736 bytes**，SHA-256 **`4212edb5658ec2867855fc2ee83ac16ec5bbad09db3c2da8d3d9d5f8bd5eeccc`**，與已保存 GitHub release API 的 bytes／digest 一致。這是下載檔案身分驗證，沒有做 Authenticode signature 驗證。

使用本機既有 **7-Zip (z) 26.03 arm64** 的 `l -slt` 與 `x` 解析 NSIS；沒有安裝工具、沒有啟動 installer／uninstaller／任何 payload。parser 判定 PE 內有 NSIS-3 Unicode、LZMA:23 solid archive；listing 與 extraction 均 exit 0。7zz 共輸出 **50 個檔案、102,981,396 bytes**，逐檔大小、SHA-256、簡單 COFF Machine 欄位與限制記在 audit JSON 的 `windows_shipped_artifact_probe`。

| 靜態輸出 | 實際觀察 |
| --- | --- |
| 主程式 | `handy.exe` 30,401,312 bytes；SHA-256 `9a1d2246b64322cb334960a5b3830b0d125070735c355648c498076ec7c0a0ff`；COFF Machine `0x8664`，即 AMD64 |
| 根目錄 DLL | 23 個，均讀到 Machine `0x8664`；包含 `transcribe.dll`、`ggml.dll`、`ggml-base.dll`、多個 `ggml-cpu-*.dll`、`ggml-vulkan.dll`、`onnxruntime.dll`、`msvcp140*`、`vcruntime140*`、`vcomp140.dll` |
| 主要 DLL 體積 | `ggml-vulkan.dll` 42,406,168 bytes；`onnxruntime.dll` 14,148,680 bytes；`transcribe.dll` 1,872,160 bytes |
| resources/models | 觀察到 `silero_vad_v4.onnx` 1,807,522 bytes、`gigaam_vocab.txt` 2,264 bytes；這個 parser 輸出集合沒有完整 Whisper ASR 權重 |
| installer／plugins | installer stub 是 Machine `0x14c`，NSIS plugin／uninstaller 也可為 x86；這與 payload 的 AMD64 目標不同，不能用 stub 架構判斷主程式架構 |

這與 build.rs 的 core／backend／ORT／VC++ staging 類別相符，卻沒有證明所有已載入模組、OS library、GPU driver 或 WebView2 都已備齊。

**提取限制：** `$PLUGINSDIR/StartMenu.dll` 的 listing 宣告 13,316 bytes，而 parser 輸出 7,680 bytes，原因未追查；metadata 保留這筆差異。其餘 48 個有數值 listing 大小的項目與輸出大小一致；`uninstall.exe` 沒有數值 listing 大小，另保留其輸出 hash。NSIS 靜態 parser 的輸出可能涉及重建／轉換，並非觀察安裝腳本實際執行後的檔案樹。檔案可能被 script 重新命名、產生、依條件安裝或下載；本次都沒有執行。50 個輸出檔不等於 Windows runtime acceptance，亦不能宣稱依賴 closure 或完全離線安裝通過。

[build]: https://github.com/cjpais/Handy/blob/14f6f0d31cb22a4268acfbddba3052820fd0a06e/BUILD.md
[readme]: https://github.com/cjpais/Handy/blob/14f6f0d31cb22a4268acfbddba3052820fd0a06e/README.md
[license]: https://github.com/cjpais/Handy/blob/14f6f0d31cb22a4268acfbddba3052820fd0a06e/LICENSE
[package]: https://github.com/cjpais/Handy/blob/14f6f0d31cb22a4268acfbddba3052820fd0a06e/package.json
[cargo]: https://github.com/cjpais/Handy/blob/14f6f0d31cb22a4268acfbddba3052820fd0a06e/src-tauri/Cargo.toml
[config]: https://github.com/cjpais/Handy/blob/14f6f0d31cb22a4268acfbddba3052820fd0a06e/src-tauri/tauri.conf.json
[winconfig]: https://github.com/cjpais/Handy/blob/14f6f0d31cb22a4268acfbddba3052820fd0a06e/src-tauri/tauri.windows.conf.json
[ci]: https://github.com/cjpais/Handy/blob/14f6f0d31cb22a4268acfbddba3052820fd0a06e/.github/workflows/build.yml
[buildrs]: https://github.com/cjpais/Handy/blob/14f6f0d31cb22a4268acfbddba3052820fd0a06e/src-tauri/build.rs
[nsis]: https://github.com/cjpais/Handy/blob/14f6f0d31cb22a4268acfbddba3052820fd0a06e/src-tauri/nsis/installer.nsi
[model]: https://github.com/cjpais/Handy/blob/14f6f0d31cb22a4268acfbddba3052820fd0a06e/src-tauri/src/managers/model.rs
[transcription]: https://github.com/cjpais/Handy/blob/14f6f0d31cb22a4268acfbddba3052820fd0a06e/src-tauri/src/managers/transcription.rs
[portable]: https://github.com/cjpais/Handy/blob/14f6f0d31cb22a4268acfbddba3052820fd0a06e/src-tauri/src/portable.rs
[onboarding]: https://github.com/cjpais/Handy/blob/14f6f0d31cb22a4268acfbddba3052820fd0a06e/src/components/onboarding/Onboarding.tsx
[updates]: https://github.com/cjpais/Handy/blob/14f6f0d31cb22a4268acfbddba3052820fd0a06e/src/components/update-checker/UpdateChecker.tsx
[settings]: https://github.com/cjpais/Handy/blob/14f6f0d31cb22a4268acfbddba3052820fd0a06e/src-tauri/src/settings.rs
[llm]: https://github.com/cjpais/Handy/blob/14f6f0d31cb22a4268acfbddba3052820fd0a06e/src-tauri/src/llm_client.rs
[actions]: https://github.com/cjpais/Handy/blob/14f6f0d31cb22a4268acfbddba3052820fd0a06e/src-tauri/src/actions.rs
[handler]: https://github.com/cjpais/Handy/blob/14f6f0d31cb22a4268acfbddba3052820fd0a06e/src-tauri/src/shortcut/handler.rs
[coordinator]: https://github.com/cjpais/Handy/blob/14f6f0d31cb22a4268acfbddba3052820fd0a06e/src-tauri/src/transcription_coordinator.rs
[clipboard]: https://github.com/cjpais/Handy/blob/14f6f0d31cb22a4268acfbddba3052820fd0a06e/src-tauri/src/clipboard.rs
[asset]: https://github.com/cjpais/Handy/releases/download/v0.9.8/Handy_0.9.8_x64-setup.exe
