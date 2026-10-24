# Portable 模型與 runtime 準備紀錄

日期：2026-10-25（Asia/Taipei）。此紀錄只證明下載、來源、完整性及獨立 helper 探針；不代表 #12／#14／#16 已通過。本輪未發布 npm package、執行 installer 或使用公司憑證。

## 多語 Whisper Small

已將 `ggml-small.bin` 放在本機外部 build cache，未提交模型：

| 項目 | 值 |
| --- | --- |
| 大小 | 487,601,967 bytes |
| 完整 SHA256 | `1be3a9b2063867b937e64e2ec7483364a79917e157fa98c5d94b5c1fffea987b` |
| 轉換來源 revision | `ggerganov/whisper.cpp@5359861c739e955e79d9a303bcbc70fb988958b1` |
| 最終下載來源 | [固定 Handy catalog 使用的鏡像](https://blob.handy.computer/ggml-small.bin) |
| 比對 | 完整檔案 hash 與 Handy 固定 catalog、轉換來源 LFS metadata 一致 |

這是多語版本，沒有 `.en` 後綴。[原作者明確將 code 與 weights 置於 MIT](https://github.com/openai/whisper/blob/86098128c0b4f24f0e2aa2994de830614b474227/README.md#license)。模型、原始授權、轉換來源與下載 manifest 已保存在外部 cache；#14 打包時仍須帶入授權與 notices。未執行本次模型的 load、推論或品質／速度測量，不能以通過 hash 代替辨識驗收。

另有三段自行合成、非機密、16 kHz／16-bit／mono 的中英混合 WAV，分別長約 13.99、13.34、14.73 秒；各有獨立撰寫的繁／簡中文參考文字，涵蓋數字、否定、英文技術名詞與版本。它們只供可重現 smoke，不代替真人語音品質。

## Fixed WebView2

從 Microsoft 官方下載的 x64 `156.0.4314.42` CAB：310,135,141 bytes，實際 SHA256 為 `cb4612ce79a62a0be094301a72f73cf91bb250f5e83dc51886f66a083fc0694f`。官方 API／headers 未提供 publisher SHA256，因此這是本次觀察到的指紋；CAB 的 Microsoft Authenticode 簽章驗證有效。

安全檢查路徑後，解開得到 257 個檔案，共 706,167,729 bytes，完整 file/size/hash manifest 已保存；六個 EXE 與核心 DLL 的 Microsoft 簽章均有效。包含官方 x86 helper 子目錄，不能以主程式 x64 為由任意裁掉 vendor 檔案。整棵 runtime 的 gzip stream 實測 319,338,846 bytes，這不是 npm tarball／registry 接受紀錄。

[Microsoft 配送文件](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution)及[實際 fixed-runtime EULA](https://developer.microsoft.com/microsoft-edge/api/eula/webview2?locale=en-us&fixed=true)需與 payload 一併評估。Runtime 是專有授權，不能改標 MIT；隨附 Widevine 條款與 Microsoft bundle 的再散布權利／notices 尚未完整釐清。先完成私有打包與啟動測試，公開發布另保留此實際缺口。

尚未證明 notype 選到此 runtime、Evergreen 不可用時仍可啟動、離線依賴完整，或公司 Proxy 可取得。Tauri 的 installer 設定本身不能證明 npm no-bundle launch 已正確選用 fixed runtime。

## 一般權限啟動探針

目前工具宿主為 elevated/high。另以 [Microsoft 文件記載的 Explorer parent-process attribute 機制](https://devblogs.microsoft.com/oldnewthing/20190425-00/?p=102443)，在同使用者／同 session 啟動僅會回報自身權限的隱藏 helper；子程序實際回報 `elevated=false`、`integrityMedium=true`、`integrityBelowHigh=true`、`uiAccess=false`，exit 0，已清理程序。未使用 token duplication、registry、UAC 或排程變更。

這提供 #14 在本機驗證一般使用者啟動的可行測試路徑，尚未啟動 notype／WebView。呼叫者環境仍可能繼承，後續須明確設定測試自己的 fixed folder／資料目錄並觀察實際 runtime process path。

## Native DLL 連結與 VC runtime

補正先前 #9 檢查：`Get-Item` 回報 `DirectML.dll` 的 Length 0 是 symbolic link 自身的長度，不是目標 DLL 的實體大小。`lstat` 確認連結，`realpath` 指向本機 ORT cache；跟隨連結後 `stat`／實際讀取均為 **18,527,776 bytes**，MZ header，SHA256 `9c9e6d822561c6c41b90e6994b3e8857cf1d66dbfb1e0c4c799c7c89b4e92da1`，Microsoft Windows Publisher 簽章有效，版本 `1.15.4+241025-1615.1.dml-1.15.fac7597`。

PE regular import table 確實包含 DirectML，沒有 delay import；#11 的獨立原生測試亦觀察到從此 build path 載入 DirectML。**不能將它當作 optional 空檔而丟棄，也不能把指向開發機 cache 的連結直接放入 npm。** #14 須將已稽核且具備正確授權／來源的相依實體複製為 payload regular file，驗證檔案內容及 manifest。CPU 推論模式不等於 linker 沒有此相依。

`dumpbin /dependents` 另確認 `MSVCP140.dll`、`MSVCP140_1.dll` 與 transcribe／ggml 的 `VCRUNTIME140.dll`、`VCRUNTIME140_1.dll` 等相依。本機 Build Tools 的 `VC/Redist/MSVC/14.44.35112/x64/Microsoft.VC143.CRT/` 有正式 redist 檔案；[Microsoft 支援 app-local 配送，且要求遵守 REDIST list／授權](https://learn.microsoft.com/en-us/cpp/windows/redistributing-visual-cpp-files?view=msvc-170)。尚未將它們打包，也不能用本機已裝 VC runtime 的啟動成功，替代完整 payload 的依賴驗收。

## 外部證據位置

本機 `%LOCALAPPDATA%/notype-study/agent-notes/` 的 `model-fixtures.md`、`packaging-runtime.md`、`normal-user-launch.md` 記錄來源與重跑方式；`build-cache/whisper-small-provision/`、`build-cache/webview2-fixed-prep/` 保存下載與完整 manifests。大型模型／runtime／WAV 不入 Git；公司 endpoint、key、工作內容也不在這些合成資料中。
