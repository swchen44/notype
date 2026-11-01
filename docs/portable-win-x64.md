# Windows portable ZIP 與模型資料夾

本版 `notype-0.1.2-local.1-portable-win-x64.zip` **不含語音辨識模型**。程式與 Node、完整 Fixed WebView2 和原生 runtime 已包含在 ZIP，不需要 installer、Node、Rust 或開發工具。系統需求是 Windows 11 x64 與支援 AVX2／BMI2 的 CPU。

## 解壓與啟動

1. 從 [v0.1.2-local.1 Release](https://github.com/swchen44/notype/releases/tag/v0.1.2-local.1) 下載 ZIP 與 `SHA256SUMS.txt`，用 `Get-FileHash` 核對 ZIP。
2. 先從系統匣結束舊版，將 ZIP 完整解壓到新的本機資料夾，保留 `app`、`runtime` 與其他檔案。
3. 雙擊 `Start-notype.cmd`。沒有模型時仍可開啟介面；完成以下模型設定後才能辨識語音。

## 另外下載語音模型

可沿用已有相容模型，或下載 Qwen3-ASR 0.6B Q6_K：

- [固定版本 Qwen3-ASR-0.6B-Q6_K.gguf](https://huggingface.co/handy-computer/Qwen3-ASR-0.6B-gguf/resolve/e4e16599b900eb0cb36e524514756bb92eb092b7/Qwen3-ASR-0.6B-Q6_K.gguf)
- 大小：690,417,824 bytes。
- SHA-256：`3b051f108f03c0c91bbe1a3b2c1ee15e3ed51e4caec2a48751b01f2a21441cc3`。
- 模型授權為 Apache-2.0；請保留下載來源的 LICENSE／NOTICE。

在可連網的電腦下載，核對後透過公司允許的方式轉移到離線電腦。不要把網頁另存成 `.gguf`，也不要將模型放入 ZIP 的 `app` 或 `runtime`。

```powershell
Get-FileHash .\Qwen3-ASR-0.6B-Q6_K.gguf -Algorithm SHA256
```

## 在 App 指定資料夾

1. 建立獨立模型資料夾，例如 `D:\notype-models`，將完整 `.gguf` 放在最上層，避免再包一層子資料夾。
2. 開啟 notype 的「模型」，選取這個模型資料夾，按「重新掃描」。
3. 選擇掃描到的模型，等待載入完成，再開始錄音。資料夾可以唯讀；App 不會自動搬移、複製或刪除你的模型。

更新程式時保留此模型資料夾，重新指定即可，不需再次下載。搬移模型前先結束 App，搬完後指定新位置並重新掃描。

個人設定與文字歷史預設存放於 `%APPDATA%\io.github.swchen44.notype`，與程式及模型分開。ZIP 不附 API key 或公司連線；本機辨識不需要 key，文字整理／翻譯／隨便問仍須設定可連線的 AI server。

來源測試、封裝完整性與實際 Windows 啟動各自記錄於 Release 的 `VERIFICATION.md`；它們不代表乾淨主機、麥克風辨識品質或公司環境驗收已完成。
