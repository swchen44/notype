# Qwen3-ASR 0.6B Q6_K：固定中英語音實測

2026-10-25。候選已由實際正式引擎成功載入；這三個合成輸入保留中文、英文與技術名稱，CPU 表現優於先前 Breeze。**未宣稱人聲／公司／完整 session p95 或 5 秒驗收通過，未自行指定公開配送預設**。

## 精確 artifact 與載入

- Publisher：`handy-computer/Qwen3-ASR-0.6B-gguf@7e7c69a577fa2725dfedbe34b5e999336e30a19b`。
- 檔案 `Qwen3-ASR-0.6B-Q6_K.gguf`，690,417,824 bytes，完整 SHA256 `3b051f108f03c0c91bbe1a3b2c1ee15e3ed51e4caec2a48751b01f2a21441cc3`，與 immutable LFS metadata 一致。
- 使用 discovery 真正回傳的 catalog ID：`handy-computer/Qwen3-ASR-0.6B-gguf/Qwen3-ASR-0.6B-Q6_K.gguf`。短 stem ID 第一次只得到 Model not found，尚未載入；保留該 setup receipt，沒有把它算作 schema／模型失敗。
- 實際 retained native runner namespace 為 `qwen3_asr`，CPU device index0／四執行緒；載入後通過正式 profile 的 zh＋en semantic capability check。實際 caps 為 streaming=false、translate=false、languageDetect=true。沒有猜測不同 GGUF／外部 serving 格式可相容。

[固定 publisher metadata](https://huggingface.co/api/models/handy-computer/Qwen3-ASR-0.6B-gguf/revision/7e7c69a577fa2725dfedbe34b5e999336e30a19b?blobs=true)、[NOTICE](https://huggingface.co/handy-computer/Qwen3-ASR-0.6B-gguf/blob/7e7c69a577fa2725dfedbe34b5e999336e30a19b/NOTICE)、[LICENSE](https://huggingface.co/handy-computer/Qwen3-ASR-0.6B-gguf/blob/7e7c69a577fa2725dfedbe34b5e999336e30a19b/LICENSE) 已保存 snapshots／hash，詳見 `qwen3-provenance.json`。這是 runtime publisher 從 Qwen base 轉換／量化的 Apache-2.0 artifact；不是 Qwen 原作者發布的 GGUF，也沒有宣稱自己重做 bit-reproducible conversion。正常打包須保留 LICENSE／NOTICE；不需要付費 API key。

下載前 C: free 7,174,217,728 bytes，達到6.5GiB門檻；採一個 in-place 檔案、165個 exact206 ranges，再核對完整 SHA 才執行。benchmark data dir 使用同 volume hard link，沒有第二份 model bytes；執行時仍保有至少5GiB free。沒有 installer、Python 模型推論、engine feature 擴充或 C++ 重建。

## 方法與結果

三個固定 WAV，Hanhan／Zira **同一個錄音內切換中文與英文**，16kHz／16-bit／mono，13.34–14.73秒；每種 profile 跑同樣三個 WAV，每次 fresh process load後兩次 inference。因此是 **3個合成輸入 × 2個 profiles = 6個 cases／12次實際 decode**，沒有獨立人聲樣本。task=transcribe、language=auto、translate=false、manual terms 空。

`qwen3-quality.json` 保留 raw、當時 normalized、獨立 gold、兩次時間、兩次 raw 與 Windows memory counters。六個 case 的兩次 raw 都相同；CER 是 NFKC／小寫、移除空白與標點後的 Levenshtein，不修補數字或英文。

| 固定 case | 繁體 CER | 簡體 CER | 觀察 |
| --- | ---: | ---: | --- |
| numbers-negation-switch | 6.67% | 6.67% | 42／2 寫為四十二／两，數值意義與否定保留；兩個英文句子／API保留。Arabic gold與 strict literal 診斷仍原樣計分 |
| technical-names | 0% | 0% | 中文段落、API、session underscore id、Visual Studio Code、GitHub保留 |
| script-context-version | 4.76% | 3.17% | raw把兩個發／髮字名聽成法；這是ASR音近字錯誤，沒有偷偷以字典修補。英文build／version2.0與否定保留；繁體另含發布／發佈變體 |

實測 retained CLI `transcribe_ms` 範圍 **3,229–3,795ms**。這個舊 binary 的欄位包括 ASR＋當時 local normalization，load另外計時；不含新 lexical converter 的初始化、公司文字服務、交付或其他 session 階段。因此不能從12次合成 decode 推導整體 p95／5秒通過。

native process observed peak working set 是 **1,355.875–1,379.58203125 MiB**，以 Windows `GetProcessMemoryInfo.PeakWorkingSetSize` 每50ms觀察，不含 helper／child process。原始 byte數逐筆保留；不是模型檔案大小或全機RAM。

分兩次明確確認的 quiet CPU windows：第一批硬上限49秒、實際40.42秒完成5 cases；第二批硬上限14秒、實際8.358秒完成最後case。其它 agent 暫停 Cargo／native／Vite；完成後立即釋放，沒有為了湊結果長時間凍結其它工作。沒有清 OS file cache／控制溫度，僅描述觀察範圍。

## Binary 與新 normalizer 的區別

retained exe SHA256 `86ec70cdac8f54547f886a715acec9b27ab0d2bfb360da3ec580cfaec7e69dba`，來源功能 commit `8ef71e7`／文件 merge `0bd8ed54`，是舊 ferrous normalizer。後續 [lexical conversion 修正](conversion-context.md) 另有純 Rust RED→GREEN／保存原稿證明，並以最新633f binary單一WAV確認ASR `[3327,3389]ms`／local normalization `[1176,0]ms`及3個actual public sessions；見 `conversion-native-proof.json`。沒有把這批舊 binary 的完整矩陣改成新 backend 的測量，也沒有重跑整個矩陣。

debug binary 的舊 bindings exporter 會寫回編譯時 source path；benchmark wrapper 只接受已知舊 export SHA，並在 child結束後原樣還原較新的 tracked bindings，未覆寫陌生變更。首個成功 probe 的實際 engine exit0／JSON先保存，之後 Python cp950 console顯示失敗；已記錄工具勘誤，完整 matrix 改用UTF8，沒有為顯示問題重跑推論。

外部完整證據：`%LOCALAPPDATA%/notype-study/build-cache/qwen3-asr-provision/`、`qwen3-initial-probe/`、`language-quality-qwen3/`。大型 weights／WAV 不入 Git；原始 Small／Q5／SenseVoice／Breeze 的失敗與限制也保留，沒有被新候選覆寫。
