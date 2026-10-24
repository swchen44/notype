# notype Windows 11 開發機查核

日期：2026-10-24。起點 commit `0cfe2e40acf63d3af971229ae2120ecdd3bf2da1`，`main`，起始工作副本乾淨；`git ls-remote origin HEAD` 與本機一致。使用者在本次工作中指定產品名稱 **notype**，並授權將 private repository 改為 `swchen44/notype`；GitHub API 回傳 200、`private: true`，本機 origin 已同步。工作目錄仍為 `D:/git/notype_study`。

## 實測環境

| 項目 | 結果 | 查核方式 |
| --- | --- | --- |
| OS | Windows 11 家用版，10.0.26200；winget log 顯示 build 9457 | `Get-CimInstance Win32_OperatingSystem` |
| 架構 | OS 64-bit；CPU Architecture 9（x64）；程序 X64 | Win32_Processor、RuntimeInformation.ProcessArchitecture |
| CPU | Intel Core i5-1135G7 @ 2.40GHz | Win32_Processor |
| RAM | 16,965,230,592 bytes | Win32_ComputerSystem.TotalPhysicalMemory |
| GPU | Intel Iris Xe Graphics | Win32_VideoController |
| Node | v22.15.0 | `node --version` |
| npm | 11.7.0 | `npm.cmd --version` |
| SQLite | 3.49.1，Node experimental API 可建立／查詢資料庫 | `node --experimental-sqlite` |
| WebView2 | Application 目錄有 153.0.4234.48、155.0.4283.62，存在 msedgewebview2.exe | Program Files (x86)/Microsoft/EdgeWebView/Application |
| 初始 native tools | PATH 未找到 rustc、cargo、cl、cmake；預設 .cargo/bin/rustc.exe 與 vswhere.exe 不存在 | Get-Command、Test-Path |
| 可寫路徑 | 使用者 LocalAppData build-cache 已下載與解壓；Temp 可寫 SQLite，重啟可讀回 | source 取得與 integration tests |
| 公司內網／Proxy／API | not-performed；未取得公司 contract，不列印或提交本機憑證 | 未將公開網路可達當作公司相容性 |

PowerShell execution policy 阻擋 npm.ps1；改用 npm.cmd，不變更 policy。WebView2 檔案存在不等同 Handy 啟動成功。尚未測量 STT 品質、p95 延遲或五個 App。

## 固定 Handy source

來源：https://codeload.github.com/cjpais/Handy/tar.gz/14f6f0d31cb22a4268acfbddba3052820fd0a06e

`curl.exe -L --fail --max-time 60 <URL> -o <cache>/handy-pinned.tar.gz`，再執行 `Get-FileHash` 與 `tar -xzf`。SHA256：`a6bcd28cfb83dee566328432ce1d15e2a4f881d0dee8af8ca7a4d2f5499488f6`，與既有 audit 完全一致。source 位於 `%LOCALAPPDATA%/notype-study/build-cache/Handy-14f6f0d31cb22a4268acfbddba3052820fd0a06e`，未將上游大檔加入產品 repo。

讀取固定 source 的 BUILD.md、package.json、src-tauri/Cargo.toml。Windows x64 預設啟用 Vulkan，所以除了 Rust MSVC/C++ tools/CMake，也需要 Vulkan SDK；frontend 使用 Bun。正式產品應以 `bun run tauri build --no-bundle` 避開 installer/signing 步驟。上游目前仍有 WAV gate、fallback 與 installer updater，不可直接當成 notype 完成品。

## 開發工具安裝

使用者明確要求「請安裝必要工具」，因此此處允許開發工具 installer。**產品交付仍禁止 installer**。

- GitHub CLI 2.80.0：winget GitHub.cli，安裝成功且 gh --version 通過。
- CMake 4.1.1：winget Kitware.CMake，安裝成功且 cmake --version 通過。
- Bun 1.2.22：winget 安裝成功；固定 Handy source 執行 `bun install --frozen-lockfile` 成功（342 packages），`bun run build` 成功（tsc + Vite 6.4.1，2123 modules）。有上游 chunk 大於 500 kB 警告，無 build error。這只是上游前端 build，不是 notype native build。
- Rust：stable x86_64-pc-windows-msvc，rustc 1.99.1（58cf8f9c3）、cargo 1.99.1（5f94df478），minimal profile 加 rustfmt/clippy。Rust metadata smoke 編譯成功；當時 link smoke 因 MSVC 尚未裝完而找不到 link.exe。工具鏈已安裝，但最後 rustup 自我更新另回報找不到 updater 檔案；不把此錯誤當工具鏈未安裝，也不宣稱 native linking 已通過。
- Vulkan SDK：winget 下載及 hash 驗證完成，但 Windows 回報 0x800704c7（使用者取消），未重新啟動安裝；尚未安裝驗收。
- Visual Studio Build Tools：已通過 UAC、原始下載恢復並進入 MSVC 14.44 元件安裝，完整安裝／SDK／linking 結果待接續記錄。另下載官方 vs_installer.opc 作備援，SHA256 `222cc676d30cfa4aeb3de b2ff73933f06a2c07fba4c6e5acdc168cf3ee4f17b2`（以下更正），備援提升權限提示被取消，未執行備援腳本；原安裝仍繼續。
- VS winget 首次卡在 source2.msix 註冊；尚未啟動 VS installer，停止該 winget 程序後，改由 https://aka.ms/vs/17/release/vs_BuildTools.exe 取得官方 bootstrapper。Authenticode Valid，簽署者 Microsoft Corporation。
- VS 參數：`--wait --quiet --norestart --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended`。不自動重新開機。

GitHub CLI 初始未登入，既有 Git Credential Manager 可存取 private repo；最初讀取與留言用 GitHub connector。Repository 改名使用 Git credential 在記憶體中呼叫 GitHub API，未列印或保存 token。

後續以 gh CLI 配合既有 Git credential（僅 process environment）重新核對 `swchen44/notype` 仍為 private，issues #2–#7 均保留且 open；已在 #2 留下進度。

## 發行檔取得（未執行）

Handy_0.9.8_x64-setup.exe 從既有 audit 的固定 GitHub release URL 重新下載。首次 curl 120 秒 timeout，只取得部分內容；未將部分檔案認作有效 payload。使用 `curl.exe -L -C - --fail --max-time 300` 續傳完成後 SHA256 為 `4212edb5658ec2867855fc2ee83ac16ec5bbad09db3c2da8d3d9d5f8bd5eeccc`，符合 audit。**未執行 Handy installer**；尚未代表 portable runtime 成功。

固定 source 另外逐一比對 audit 的 21 個 source 檔案，mismatch 0。

## 文字流程驗證

`npm.cmd run test:unit`：11 passed、0 failed、0 skipped（含當次 WAV 清理 3 項）。

`npm.cmd run test:integration`：5 passed、0 failed、0 skipped。

最初在尚無實作時測試因缺少模組失敗；實作後 unit 通過。Integration 首次暴露 Windows SQLite handle 尚未關閉時刪暫存目錄的 EBUSY；修正 teardown 為先關 DB 再刪除後通過。額外驗證 HTTP malformed response 與 cancellation。

涵蓋原稿先落地、無 WAV 歷史、搜尋／刪除、timeout／錯誤待選、取消遲到結果、重啟恢復、原稿重試及 callback 去重。交付中斷視為 delivery-unknown，避免自動重送。

這些測試沒有使用真實 STT、Windows 輸入框或公司 API。開發 contract 是 `POST {transcript, glossary} -> {text}`；只傳當次原稿與術語表。SQLite 預設路徑是 `%LOCALAPPDATA%/notype/history.db`，但真實 npm 升級／Handy SQLite migration 仍未驗收。History 目前限定單程序使用，正式接合須加 single-instance guard；callback 的原目標辨識須由 native/UI 邊界完成。

## 獨立 CPU STT → Edge runtime smoke

`npm.cmd run test:runtime`：1 passed、0 failed、0 skipped。這是另一個獨立驗證，並非上面 deterministic 測試的替身結果。

使用 whisper.cpp v1.8.6 的預編譯 x64 CPU CLI（source commit `23ee03506a91ac3d3f0071b40e66a430eebdfa1d`），固定 tiny.en 模型，以及 Microsoft Zira Desktop 生成的非機密 4.07 秒 WAV。來源、hash、全部 runtime 檔案及 fixture 參數見 [manifest](local-stt-manifest.json)。完整模型透過 HTTP range 分段下載，再合併並核對來源 LFS SHA256；不是以分段成功當完整模型成功。

實際辨識：`please keep the number 42 and do not delete the API.`。單次 STT 919 ms，4 threads、CPU-only；樣本數 1、合成英文、音訊短於需求的 10～20 秒，**沒有 p50/p95 或繁中品質結論**。

流程經 loopback HTTP fixture，保存原稿／結果、關閉再開啟 SQLite，然後交付至全新 headless Edge 156.0.4314.42 的 textarea。原文字框失焦時不插入，恢復後只插入一次，Enter 次數 0。插入使用頁面內原子焦點檢查與 setRangeText；沒有存取既有使用者 profile、分頁或工作內容。**這不是 Win32 原生焦點／剪貼簿／IME／五個 App 驗收**，也不是公司真實 LLM。

當次辨識 WAV 與 CLI 暫存文字在 success/error/cancel 後刪除。固定、非機密測試 fixture 另外留在開發 cache；文字歷史不依賴錄音。`createWhisperTranscriber` 是診斷接點，尚未決定最終產品使用 CLI，也未替換優先評估的 Handy。

重跑（資料未隨 Git 發布；先依 manifest 取得、核對 runtime/model，並以 runtime/generate-smoke.ps1 產生 WAV；voice/runtime 不同可能導致 fixture hash 不符，需另記 receipt，不要直接放寬驗證）：

```powershell
$cache = Join-Path $env:LOCALAPPDATA 'notype-study/build-cache'
$env:NOTYPE_WHISPER_EXE = "$cache/whisper-v1.8.6/Release/whisper-cli.exe"
$env:NOTYPE_WHISPER_MODEL = "$cache/ggml-tiny.en-verified.bin"
$env:NOTYPE_SMOKE_WAV = "$cache/smoke-zira.wav"
npm.cmd ci --ignore-scripts
npm.cmd run test:runtime
```

備援 vs_installer.opc 正確 SHA256：`222cc676d30cfa4aeb3deb2ff73933f06a2c07fba4c6e5acdc168cf3ee4f17b2`。
