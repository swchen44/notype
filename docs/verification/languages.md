# #12 語系、辨識 profile 與實際品質

2026-10-25。功能程式已接上，**混合語言品質驗收仍未達成，尚未選定配送模型**。本文件不關閉 #12；Breeze ASR 25 的後續實驗另記。未使用公司 key、雲端辨識、LLM 修補或真實工作錄音。

## 功能邊界

- 設定、待交付、歷史、錯誤使用完整 `zh-TW`／`zh-CN` resources；兩者皆對齊 English 的 521 keys。預設／未知語系回到繁體，舊 `zh`／Hans 仍解析為 `zh-CN`，缺 key 不顯示 resource key。其他上游語系不在這次完整度聲明內。
- `recognition_mode` 獨立於介面語系；錄音開始時擷取 `RecognitionProfile`，包含 mode 與 exact manual terms。之後改設定不改寫該 session／舊歷史。舊明確簡體設定只遷移一次。
- 真實引擎語言能力須同時包含 `zh`、`en`。自訂檔改名也不能讓 English-only weights 通過。未知 capability 在實際模型載入後、decode 前檢查。已知不相容模型在開麥克風前拒絕。
- 正式 stream／batch 使用 transcribe、language auto、translate false；Whisper 的 initial prompt 只含 session 擷取的 manual terms。本次品質測量 manual terms 為空。停用上游模糊替換／語助詞刪除，保留 ASR 原稿。
- 本機 OpenCC `S2tw`／`Tw2s` 在原稿 INSERT、文字服務、交付之前執行；保護 code fences、inline code、quoted literals、URL、Windows path、email、識別字與 exact glossary。只轉字形，避免地區詞彙翻譯。
- 轉換不可用時原子保存 FAILED／`recognition-script-failed` 與可用 raw，零 LLM／自動交付；原稿可搜尋／複製／明確選用，文字整理重試會拒絕這個錯誤，需要重新辨識。
- Core 保持不依賴 UI／Specta／Windows。desktop DTO 負責 settings 與 bindings；後續 macOS／Linux 沿用 profile／session 邊界。

## 實際觀察

固定輸入是 Hanhan＋Zira 的 **同一個 WAV 內中文與英文切換**，16 kHz／16-bit／mono，三段各 13.34–14.73 秒。`recognition-fixtures.json` 有獨立人工撰寫的繁／簡 gold、語音片段與敏感資訊保留點；provision receipt 有 WAV SHA256。沒有先用 OpenCC 產生 gold。這是合成 smoke，沒有取代人聲與公司環境驗收。

正式 app CLI、i5-1135G7／4 核 8 執行緒，每個模型每個 profile 跑相同三個 WAV，每次 fresh process load＋兩次 inference。Whisper CPU session 明確四執行緒；ONNX 是既有 library default，未宣稱四執行緒或 GPU 綁定。完整 raw ASR、轉字形後 text、gold、錯字率、required literal 損失、每次 load/decode 時間在 `recognition-quality.json`。

| 模型 | 實測 decode 範圍 | 字元錯誤率範圍 | 結果 |
| --- | --- | --- | --- |
| Whisper Small FP16／MIT | 3,563–4,088 ms | 44.4–87.5% | 英文片段被省略；英文開頭時中文片段被譯為英文，未通過 |
| 本機量化 Small Q5_1／MIT derivative | 5,284–7,934 ms | 44.4–87.5% | 相同混語損失，這台機器也較慢，未通過 |
| SenseVoice int8／僅私人比較 | 948–1,481 ms | 1.6–11.7% | 保留雙語，但 API→AP、Visual 被省略、GitHub 分詞與 髮→法 仍錯；授權詮釋待釐清，未選作公開配送 |

CER 定義是 NFKC／小寫後移除標點／空白的 Levenshtein，**未修補英文、未將中文數字當阿拉伯數字**。required literal 是另列的大小寫不敏感 exact substring 診斷；`兩`／`2`、`Git hub`／`GitHub` 應人工分辨，不把 literal diagnostic 當完整品質分數。沒有從這個小樣本宣稱 p95／人聲／公司通過；RAM 尚未量測。cold 指 fresh process/model load，沒有清除 OS file cache；warm 指同 process 第二次 decode。

另以已核對的 whisper.cpp v1.8.6 CLI、同一 FP16、`-l zh -t 4` 作診斷，numbers WAV 仍完整省略兩個英文句子，因此未以固定中文 language 掩蓋 auto 的失敗，也沒有改成靜默翻譯。這個診斷不是正式 app 的品質通過證據。

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
