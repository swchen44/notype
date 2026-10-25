# #12 語系、辨識 profile 與實際品質

2026-10-25。功能程式已接上，**整體品質／速度驗收仍未達成，尚未選定配送模型**。Breeze 在固定合成語音上的混語結果改善，但 CPU 耗時未達預期；本文件不關閉 #12。未使用公司 key、雲端辨識、LLM 修補或真實工作錄音。

## 功能邊界

- 設定、待交付、歷史、錯誤使用完整 `zh-TW`／`zh-CN` resources；兩者皆對齊 English 的 521 keys。預設／未知語系回到繁體，舊 `zh`／Hans 仍解析為 `zh-CN`，缺 key 不顯示 resource key。其他上游語系不在這次完整度聲明內。
- `recognition_mode` 獨立於介面語系；錄音開始時擷取 `RecognitionProfile`，包含 mode 與 exact manual terms。之後改設定不改寫該 session／舊歷史。舊明確簡體設定只遷移一次。
- 真實引擎語言能力須同時包含 `zh`、`en`。自訂檔改名也不能讓 English-only weights 通過。未知 capability 在實際模型載入後、decode 前檢查。已知不相容模型在開麥克風前拒絕。
- 正式 stream／batch 使用 transcribe、language auto、translate false；Whisper 的 initial prompt 只含 session 擷取的 manual terms。本次品質測量 manual terms 為空。停用上游模糊替換／語助詞刪除，保留 ASR 原稿。
- 本機 OpenCC `S2tw`／`Tw2s` 在原稿 INSERT、文字服務、交付之前執行；保護 code fences、inline code、quoted literals、URL、Windows path、email、識別字與 exact glossary。只轉字形，避免地區詞彙翻譯。

後續公開 regression 找到 greedy conversion 的「保留髮展」問題，已改採 pinned lexical backend；來源／授權、RED→GREEN、衍生 corpus 與最新 native proof 的區別見 [context conversion receipt](conversion-context.md)。下面的模型測量保留各次當時的 binary／raw／gold，沒有覆寫成新 backend 的測量。
- 轉換不可用時原子保存 FAILED／`recognition-script-failed` 與可用 raw，零 LLM／自動交付；原稿可搜尋／複製／明確選用，文字整理重試會拒絕這個錯誤，需要重新辨識。
- Core 保持不依賴 UI／Specta／Windows。desktop DTO 負責 settings 與 bindings；後續 macOS／Linux 沿用 profile／session 邊界。

## 實際觀察

固定輸入是 Hanhan＋Zira 的 **同一個 WAV 內中文與英文切換**，16 kHz／16-bit／mono，三段各 13.34–14.73 秒。`recognition-fixtures.json` 有獨立人工撰寫的繁／簡 gold、語音片段與敏感資訊保留點；provision receipt 有 WAV SHA256。沒有先用 OpenCC 產生 gold。這是合成 smoke，沒有取代人聲與公司環境驗收。

正式 app CLI、i5-1135G7／4 核 8 執行緒，每個模型每個 profile 跑相同三個 WAV，每次 fresh process load＋兩次 inference。Whisper CPU session 明確四執行緒；ONNX 是既有 library default，未宣稱四執行緒或 GPU 綁定。完整 raw ASR、轉字形後 text、gold、錯字率、required literal 損失、每次 load/decode 時間在 `recognition-quality.json`。

| 模型 | 當時 CLI transcribe_ms 範圍 | 字元錯誤率範圍 | 結果 |
| --- | --- | --- | --- |
| Whisper Small FP16／MIT | 3,563–4,088 ms | 44.4–87.5% | 英文片段被省略；英文開頭時中文片段被譯為英文，未通過 |
| 本機量化 Small Q5_1／MIT derivative | 5,284–7,934 ms | 44.4–87.5% | 相同混語損失，這台機器也較慢，未通過 |
| SenseVoice int8／僅私人比較 | 948–1,481 ms | 1.6–8.3% | 保留雙語，但 API→AP、Visual 被省略、GitHub 分詞與 髮→法 仍錯；授權詮釋待釐清，未選作公開配送 |
| Breeze ASR 25 Q5_K／Apache-2.0 candidate | 28,284–33,385 ms | 0–1.67% | 六個 mode／WAV cases 都保留中英、名稱、否定與髮／發；殘差為兩／2及發布／發佈。速度未達預期，未選作配送預設 |
| Qwen3-ASR 0.6B Q6_K／Apache-2.0 candidate | 3,229–3,795 ms | 0–6.67% | 中英／名稱／否定保留；數字是中文書寫形式，字名有法音近字錯誤；仍無人聲／整體p95通過聲明 |

CER 定義是 NFKC／小寫後移除標點／空白的 Levenshtein，**未修補英文、未將中文數字當阿拉伯數字**。required literal 是另列的大小寫不敏感 exact substring 診斷；`兩`／`2`、`Git hub`／`GitHub` 應人工分辨，不把 literal diagnostic 當完整品質分數。沒有從這個小樣本宣稱 p95／人聲／公司通過；Small／Q5／SenseVoice 的 RAM 未量測，Breeze／Qwen 分述下方。cold 指 fresh process/model load，沒有清除 OS file cache；warm 指同 process 第二次 decode。

Qwen 是後續加入的精確 runtime-publisher candidate，6cases／12decodes與memory另見 [Qwen完整receipt](qwen3-quality.md)；來源、不同binary／新normalizer初始化的界線都有記錄，沒有用模型大小或首段速度代替整體驗收。

參考勘誤：最初簡體 numbers gold 把來源語音的「測試資料」寫成「测试数据」，現人工修正為只轉字形的「测试资料」；這使該簡體 case 的 CER 降低 3.3 個百分點。另修正 numeric literal 邊界，讓 required `2` 不再誤匹配 `42`。原始模型輸出完全保留，JSON 的 `originalDiagnostic` 保存舊 gold／分數／literal 診斷，`referenceErrata` 記錄理由；外部原始 receipts 不覆寫。這些修正沒有讓 Small 的英文省略／翻譯通過。

另以已核對的 whisper.cpp v1.8.6 CLI、同一 FP16、`-l zh -t 4` 作診斷，numbers WAV 仍完整省略兩個英文句子，因此未以固定中文 language 掩蓋 auto 的失敗，也沒有改成靜默翻譯。這個診斷不是正式 app 的品質通過證據。

### Breeze 實測

完整觀察在 `breeze-quality.json`，來源／完整 hash／授權 snapshots 在 `breeze-provenance.json`。使用同一批 WAV、已勘誤的獨立 gold、正式 retained app CLI，六個 cases 各 fresh process load＋兩次 inference。task=transcribe、language=auto、translate=false、manual terms 空、CPU device index0／四執行緒；#17 明確暫停 Cargo／native／Vite／lint，只有低 CPU 的 UI／HTTP 檢查可繼續。沒有清 OS cache 或控制溫度／所有背景工作。

實測 process peak working set 1,449.8–1,457.2 MiB，以 Windows `GetProcessMemoryInfo` 的 `PeakWorkingSetSize` 每50ms觀察，只含 native CLI process。code-switching、API／Visual Studio Code／GitHub、否定、髮／發上下文在這三個合成錄音保留；`兩` 對 gold `2` 是數字書寫形式，`發布`／`發佈` 是字形變體，原始分數不因此改成零。這不是人聲／公司／p95 驗收；約14秒音訊需要約30秒 CPU inference，不能宣稱達成5秒目標。

模型 1,080,732,108 bytes，完整 SHA256 `8efbf0ce8a3f50fe332b7617da787fb81354b358c288b008d3bdef8359df64c6`，同固定 Handy catalog 與 [immutable mirror LFS metadata](https://huggingface.co/coco-research/coco-voice-models/blob/3cb3e6de7c20f58fe5beaaab3d5159384feff3f0/breeze-asr/breeze-asr-q5_k.bin)。原始 [MediaTek weights revision](https://huggingface.co/MediaTek-Research/Breeze-ASR-25/blob/cffe7ccb404d025296a00758d0a33468bec3a9d0/README.md) 宣告 Apache-2.0；mirror publisher 的 [attribution](https://huggingface.co/coco-research/coco-voice-models/blob/3cb3e6de7c20f58fe5beaaab3d5159384feff3f0/breeze-asr/ATTRIBUTION.md) 說明是 unchanged Handy ggml／Q5_K conversion，並保留 base OpenAI Whisper MIT 與 converter Handy MIT notices。這是來源方聲明及 exact hash mapping，沒有宣稱自己重做 bit-reproducible conversion。另一個同名 shdennlin Q5_K 的大小／hash 不同，沒有混用。

下載是單一 in-place 檔案、exact206 ranges＋完整 SHA 後才執行；只在外部 cache，benchmark data dir 是同 volume hard link，沒有複製大型權重。retained runner 37個 regular files／111,066,246 bytes，DirectML symlink 已解引用，exe SHA256 `86ec70cdac8f54547f886a715acec9b27ab0d2bfb360da3ec580cfaec7e69dba`。它未含 fixed WebView2，僅是獨立 benchmark snapshot，沒有被當作 portable payload closure。對應 product source 是功能 commit `8ef71e7`／文件 merge `0bd8ed54`；後續 URL/path delimiter 修正不在這個 binary，三個 WAV 都沒有這類 literal。

## 模型與來源

- FP16 487,601,967 bytes，SHA256 `1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b`，同固定 Handy catalog 與 `ggerganov/whisper.cpp@5359861c739e955e79d9a303bcbc70fb988958b1` 的 LFS metadata；OpenAI 對 [code 與 weights 的 MIT 聲明](https://github.com/openai/whisper/blob/86098128c0b4f24f0e2aa2994de830614b474227/README.md#license) 已保存。
- 本機 Q5_1 190,085,487 bytes，SHA256 `52914f6730a59593fd6108d21dcac060a35ce569d9d70eec431e5623f387c82f`；由上述 FP16 與已核對 v1.8.6 whisper-quantize 產生，是自己的 derivative，**不是**同大小 publisher Q5_1。外部 `whisper-small-q5-local.md` 有完整 input/tool/output chain。
- SenseVoice ONNX 239,233,841 bytes，SHA256 `c71f0ce00bec95b07744e116345e33d8cbbe08cef896382cf907bf4b51a2cd51`，tokens `f449eb28dc567533d7fa59be34e2abca8784f771850c78a47fb731a31429a1dc`；converted checkpoint mapping 已比對，但模型授權 owner 的 issue #334 仍重開待詮釋。此處只有私人實驗，沒有以 code MIT 代替 weights 授權。

## 重跑與驗證

在 `desktop/` 使用自己的 frozen Bun dependencies；C++ baseline 不增加 features。範例 native env：`CARGO_TARGET_DIR=D:/git/notype-build`、`CARGO_INCREMENTAL=0`、`CARGO_PROFILE_DEV_DEBUG=0`、`CARGO_PROFILE_TEST_DEBUG=0`、`CARGO_BUILD_JOBS=2`。

```powershell
bun install --frozen-lockfile
bun run lint
bun run build
bun run check:translations zh-TW zh-CN
cargo test --offline --locked --features tauri/custom-protocol --manifest-path src-tauri/Cargo.toml --test desktop-workflow --test desktop-provider
bun run tauri build --no-bundle --debug -- --locked
```

根目錄設定 `NOTYPE_DESKTOP_EXE` 為實際 debug exe，`NOTYPE_TINY_EN_MODEL` 為已驗證的 77,704,715-byte English-only weights，`NOTYPE_MIXED_WAV` 為固定混語 WAV：

```powershell
node --test tests/integration/desktop-languages.runtime.mjs tests/integration/runtime/formal-recognition.runtime.mjs
```

後者刻意把 English-only weights 改名 `renamed-model.bin`，驗證 semantic rejection。runtime 不缺件 skip。

手動實際品質 receipt：外部 `NOTYPE_BENCHMARK_CACHE` 應含 `notype-stt-fixtures/`、`whisper-small-provision/` 與已核對模型；輸出也只在 cache。`node tests/integration/runtime/recognition-benchmark.mjs` 跑 FP16／本機 Q5，`--private-sensevoice` 額外跑私人比較。它記錄失敗，不會以不嚴格斷言把模型品質變成測試通過。

已見行為 RED：舊流程保存簡體 raw；舊 converter 改寫 literal；沒有 recognition-failure seam；舊設定缺 mode；舊 engine 接受 English-only；舊 `zh-CN` 會解析成 `zh`。修正後 workflow unit 5、HTTP／SQLite integration 6、實際 Tauri／CLI runtime 4 通過。frontend lint/build、兩語 521 keys 通過。發布前仍須跑最新整合後 checks，不能以本表代替公司 API、五個 App、安全 native target 或完整 npm payload 驗收。
