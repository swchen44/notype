# 本機繁簡轉換：保留詞義與 literal

2026-10-25。這是 #12 的本機轉換修正，ASR 模型、原始測試錄音、raw outputs 與 gold 沒有被改寫。新的 normalizer 已通過純 Rust 公開 session 測試，以及 #13 一次必要建置後的實際 native CLI／正常使用者 Tauri session 證明；舊 retained CLI 的完整模型矩陣仍保留原本版本標記。

## 原因與修正

`保留发展` 原本保存成 `保留髮展`。舊 ferrous-opencc 0.2.3 的 greedy matching 先吃到 `留发 → 留髮`，無法辨認 `保留 / 发展` 的詞界。這不是 ASR 音近字錯誤，也不能以「字都已經是繁體」宣稱正確。

另有獨立 span bug：未加引號的 URL／Windows path 把緊接的中文標點與後文都當 literal，留下錯誤字形。先以兩個公開 session RED 重現，再只讓 bare URL／path 在 `，。！？；` 停止；沒有改寫引號規則。分離的 URL 測試用「然后检查发展」確認 span 行為，原始「然后保留发展」也保留在後續 context regression，沒有藏掉髮展。

新 backend 固定 `opencc-jieba-rs = "=0.8.1"`，先依實際詞彙分詞，再在詞內轉換。獨立執行的公開 API 確認 `jieba_cut("保留发展", false)` 得到 `["保留", "发展"]`；development／hair、混合與既有正確字形、反向簡體、非地區詞彙翻譯、空字串、長英文、空白與四個共享執行緒探針通過後，才替換產品 normalizer。產品沒有特定句子／字串修補或修改 gold。

兩個 mode 分別呼叫 `s2tw(input, false)`／`tw2s(input, false)`；`false` 關閉 punctuation normalization，不使用會改地區詞彙的 `s2twp`。既有 protected spans 保留 code fence、inline code、quoted literals、URL、drive path、email、identifier 與 exact manual terms；保護區外才執行轉換。mode／terms 仍是開始錄音時的 snapshot。

單一 shared `OnceLock<Result<OpenCC, RecognitionFailure>>` 共用兩種 mode 的 embedded dictionary。`OpenCC::new()` 在損壞字典可能 panic，因此初始化及轉換都在 unwind 邊界內；失敗沿用原子 FAILED／`recognition-script-failed`／可用原稿保存，零 LLM／自動交付，沒有 fail-open。舊 ferrous 依賴只留給上游相容 helper，不再處理正式 recognition profile。

## 來源、授權與資產

實際 Cargo archive SHA256：`df9486028b855a803e8fb64961a2e5506a3a0a16ff859b8bf8443e493ad6477c`。package `.cargo_vcs_info` 宣告 `0f82899d2031247910094ca642e51de6d783d7a7`；研究時的 tag commit `4ad826180e0c29e5719f9166dabcfa024b5853a5` 不同，沒有宣稱二者 byte-identical。以 [published package metadata](https://crates.io/api/v1/crates/opencc-jieba-rs/0.8.1) 的 checksum 與實際 archive 為準。

`default-features = false`，不用 `dictionary-build`、CLI 或 C API。實際 packaged constructor 用 embedded dictionary／Zstd bytes 與 `Cursor` 建立 Jieba；無 C++、client Python 或執行時 dictionary path。沒有加入 ASR engine feature 或重新編譯 C++ 的要求。詞庫增加 embedded data、初始化／記憶體成本與 Rayon graph；舊 ASR 效能表不是新 converter 初始化時間。

程式 MIT、OpenCC 詞庫 Apache-2.0、vendored ruzstd decoder MIT、jieba-rs MIT。對應完整 license／notice 已原樣放入 `desktop/src-tauri/resources/notices/`，由既有 resource 規則納入。`conversion-context-provenance.json` 列出實際 package checksum、source commit、6 個主要 notices 與新增 22 個 registry versions 的 SPDX／32 個原始 notice files。desktop／portable test harness 的 lock 都已更新，native Cargo cache 已備妥核對 checksum 的 archive，沒有複製 ignored 產品原始碼。

## 可重跑的公開證明

```powershell
cargo test --offline --locked --manifest-path tests/integration/desktop-contracts/Cargo.toml --test desktop-workflow --test desktop-provider
cargo clippy --offline --locked --manifest-path tests/integration/desktop-contracts/Cargo.toml --tests
```

公開 session regression：`保留发展`、`保留发展空间`、`留发与发展`、`头发／长发／理发／染发`、已正確繁體、簡體反向、以及原始 URL＋保留发展。包含 fenced code、quoted literals、manual terms、tabs／CRLF／emoji／數字／ASCII 的兩個 profile test 也通過。

結果：workflow unit **9 passed**；HTTP／SQLite／固定 ASR corpus integration **12 passed**；harness clippy exit0，保留匯入 production module 的 unused public export/helper 警告。失敗證據 `保留髮展` 沒有改成期望值，已在 RED receipt 重現後轉 GREEN。

`conversion-context-derived.json` 把既有 24 個已觀察 raw ASR rows 通過公開 `DesktopWorkflow.begin/finish` 與真正 SQLite 原稿保存，停用文字 transport，逐筆檢查 ASCII／mode／saved original。這 24 筆新的 normalized text 都與舊 corpus 相同；舊 Small 的英語省略與模型音近字錯誤因此仍留著。這是新 converter 對凍結輸入的衍生結果，沒有重新推論模型或宣稱模型品質通過。

CLI 另增加 `recognition_ms`／`normalization_ms` arrays，原 `transcribe_ms` 繼續表示兩個階段的總時間。

## 實際最新 native proof

#13 的單次 CPU no-bundle debug 建置通過，未另建新 cache 或擴充 C++ engine features。實際 exe SHA256 `633f5821671edd4e13a634ed21acecda18a11cc763769bbccb9f2c189be34b93`；完整 runtime receipts 在 `conversion-native-proof.json`。

只跑一個既有 numbers WAV、Qwen3-ASR 0.6B、`--repeat 2 --json`，沒有重跑整個模型矩陣。actual ASR phase `[3327,3389]ms`，local normalization `[1176,0]ms`，合計 `[4503,3389]ms`，model load891ms另計。第一個1176ms含 embedded dictionaries／regex初始化；reuse的0ms是毫秒取整，不代表沒有工作。這個量測不含公司LLM、交付、麥克風或其它session階段，不能宣稱整體5秒／p95通過。

同一 binary 透過 pinned normal-user／fixed WebView2 `156.0.4314.42` helper，真正呼叫 Tauri public begin/finish/search：`保留发展，不要留发；头发和发展。`、URL後接`保留发展`、繁體轉簡體共3cases均得到正確 saved original／mode，並讀回3筆SQLite歷史。captureInput=false、text transport未配置，沒有讀取原輸入目標、麥克風、公司API或交付動作。正常使用者helper exit0、剩餘owned processes0；compiled source bindings 前後hash相同，已原樣還原，slot已釋放。這僅證明normalizer／保存路徑，沒有代替 #13 的實際前景輸入驗收。
