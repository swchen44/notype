# 公司 Windows amd64 接手說明

更新日期：2026-10-24。Repository：<https://github.com/swchen44/notype_study>，private。

## 1. 實作與驗收在哪裡做

**應在公司 Windows amd64 上開始實作接點與 runtime 驗收。** 麥克風、快捷鍵、IME、WebView2、native DLL、公司 TLS／Proxy 與跨 App 貼上都需要目標 Windows 的 evidence。macOS source review、Windows CI build 或成功解包均不能替代這一層驗收。

公司 App **不能使用 installer**。首版交付應為可寫入使用者目錄的 portable payload，由 npm／pip Proxy 取得完整檔案；主要渠道尚待 grilling Q9 確認。不得把啟動時下載並執行 setup／MSI／WebView2 bootstrapper 當成解法。

Rust build 工具能否在公司機器使用仍未知。如果已有 MSVC／Rust／CMake 等，才採當地 native build；若缺少，可在另一個具備工具的 Windows build 環境產生預編譯 payload，再回公司 Windows 做 adapter、包裝與 runtime 驗收。這項分工尚未驗證，沒有要求使用者在公司安裝 build tools。

## 2. 已有成果與可接續位置

- [Typeless 研究](01-typeless-macos-analysis.md)：已安裝 macOS 2.8.3 的靜態流程；backend 模型／prompt 未知。
- [候選比較](02-open-source-alternatives.md)：8 個候選、固定 source／stable refs；優先 Handy。
- [Handy source 追查](06-source-feasibility-audit.md)：v0.9.8／`14f6f0d31cb22a4268acfbddba3052820fd0a06e`，portable marker、`Data/models`、LLM 與 paste 路徑。
- Handy upstream 原始行為：LLM 失敗後可能直接貼原逐字稿。**本產品已決定改為停下待選**，不能把 upstream 行為當 acceptance。
- Windows x64 發布物只做靜態檢查。下載的 installer 只是研究輸入，從未執行；**它不是本產品可交付形式**。解出的檔案也尚未驗證完整性閉包、簽章或 runtime。
- 詳見 [決策紀錄](08-decision-log.md)；Terminal 用途、效能門檻、主要套件渠道與歷史保存尚待回答。

所有大檔案／upstream 工作副本在研究 Mac 的 `/private/tmp`，不在 Git。Windows 接手時依固定 URL／SHA 重新取得；不能依賴該暫存目錄存在。

## 3. Windows 先做唯讀 preflight

先記錄 Windows 版本、amd64、CPU／RAM、現有 GPU 與 Node／npm／Python、可用的 C++／Rust build 工具、WebView2、公司 Proxy 與可寫入路徑。這些事實由接手 agent 查，不先安裝工具，也不把 executable 存在或 version 輸出當成功使用證據。

若已無可用 WebView2，需評估可隨 portable payload 執行的 fixed runtime；其檔案、設定、目錄與實際啟動仍需在 Windows 驗證。若此路徑不可行，才重新評估不使用 WebView 的 UI 基底。**沒有 WebView2 時，不得改用 installer 迴避限制。**

公司 API 的 request／response、認證與 CA 尚未提供。若環境已有文件或範例，從該來源取得並遮蔽秘密；否則記錄缺少的 contract。不要從「文字 LLM」推導為已確認 OpenAI-compatible，也不要將憑證放入 repo、issues 或 package。

## 4. 實作與驗收順序

| 順序 | 工作 | 要留下的 receipt |
| --- | --- | --- |
| 1 | 完成當前 grilling 與共同理解確認，整理待辦 | 已確認規格、未驗證事實與對應 issue |
| 2 | 在 Windows 將預編譯 payload、runtime、模型組成 portable baseline | 檔案 manifest、來源 SHA、啟動結果；不執行 installer |
| 3 | 量測本機多語 STT | 非機密測試音訊、預期／實際文字、術語錯誤、延遲及資源 |
| 4 | 接公司文字 API，加入明確 timeout／取消與錯誤待選 | 遮蔽內容與秘密的 request 狀態、原稿保存、重試／貼原稿結果 |
| 5 | 驗證五種 App 的文字插入 | 焦點、IME、換行、剪貼簿、重複插入與 Terminal 行為 |
| 6 | 經實際公司 Proxy 取得包並冷啟動 | package／payload digest、取得來源、無 installer、模型預載與外部連線結果 |

效能目標目前是待確認問題，不能直接填成實測通過。若無法滿足目標，先分開 STT、LLM 與貼上階段，再決定模型／backend 或接點修改。

## 5. 可貼給 Windows Codex 的接手 prompt

```text
接手 private repository swchen44/notype_study。先讀 README.md、docs/research/README.md、
07-windows-handoff.md、08-decision-log.md、06-source-feasibility-audit.md 與目前 GitHub Issues。

目標為公司 Windows amd64 portable 語音輸入 App：熱鍵→本機STT→公司文字LLM→貼回輸入框。
不能使用 installer/setup.exe/MSI 或 runtime bootstrapper。npm/pip Proxy 可配送含預編譯
exe/DLL/模型的套件；主要配送渠道及部分產品決策仍以決策紀錄最新狀態為準。
先供使用者本人使用，首批目標 browser、VS Code、Teams、Outlook、Windows Terminal。
繁中夾英文忠實整理；LLM失敗時保留逐字稿，讓使用者選重試或貼原稿。

先唯讀查核此Windows環境與GitHub狀態，不安裝工具、不執行外來installer、不讀取或提交
實際工作錄音/逐字稿/API秘密。確認grilling的未決問題是否已答完，以及是否已確認共同理解。
若尚未完成，繼續事實研究與訪談，不提前實作未定設計。

原始碼基底優先評估Handy v0.9.8，commit 14f6f0d31cb22a4268acfbddba3052820fd0a06e。
依固定來源取得payload/模型，不依賴Mac的/private/tmp。readiness、build、靜態解包、
Windows實際啟動、ASR品質/效能、公司API和五個App貼上分開記錄證據。
每項變更對應issue及commit；成功與失敗都留下command、版本、狀態和未驗證邊界。
套件publish與CI配置依後續明確授權範圍執行。

遵循AGENTS.md與docs/agents/*.md。unit tests放tests/unittest/，integration tests放
tests/integration/；選定stack後補可分開執行的命令，不能把目前的文件scaffold當測試通過。
```

建議的 skills 流程：以 `grill-with-docs` 接續既有 Q7～Q10；收斂後用 `to-spec` 整理規格，再用 `to-tickets` 拆分含 blocking edges 的可驗證切片。換到 Windows 新 session 前可用 `handoff` 產生暫存交接文件，引用此 repo 既有文件及 issues。

## 6. GitHub 追蹤入口

| Issue | 工作 |
| --- | --- |
| [#1](https://github.com/swchen44/notype_study/issues/1) | grilling 與規格確認 |
| [#2](https://github.com/swchen44/notype_study/issues/2) | Windows amd64 portable 啟動基線 |
| [#3](https://github.com/swchen44/notype_study/issues/3) | STT 品質與端到端效能 |
| [#4](https://github.com/swchen44/notype_study/issues/4) | 公司文字 API 與錯誤待選 |
| [#5](https://github.com/swchen44/notype_study/issues/5) | 五個目標 App 送字驗收 |
| [#6](https://github.com/swchen44/notype_study/issues/6) | Proxy 配送與完整 portable payload |

研究 Mac 的工作區禁止寫入 `.git`，因此此處沒有本地 Git 初始化；遠端 commits 經 GitHub API 建立。Windows 接手可直接 clone private repo，再核對 remote HEAD 與本地 checkout。
