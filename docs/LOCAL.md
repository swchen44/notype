# notype 本機試用版

本版先提供錄音、本機繁／簡中英辨識、文字歷史與複製。公司文字 API、跨 App 自動輸入與公開 npm 發布另行驗收；不用先提供 API key。

## 安裝與啟動

需要 Windows x64、Node.js 22.15 以上。本機試用組建限制 AVX2／BMI2；其他 CPU／作業系統尚未驗收。執行時不需要 Rust、C++、Bun 或 installer，請用一般權限的終端機。

將收到的三個 `.tgz` 放在同一個資料夾，在該資料夾執行以下 PowerShell 指令。三個檔案必須一起安裝，避免向 registry 尋找其他同名套件：

```powershell
$notypeInstall = Join-Path $env:USERPROFILE 'notype-local'
npm.cmd install --offline --ignore-scripts --no-audit --no-fund --prefix $notypeInstall .\notype-0.1.0-local.1.tgz .\notype-win32-x64-0.1.0-local.1.tgz .\notype-model-0.1.0-qwen3-q6k.1.tgz
& (Join-Path $notypeInstall 'node_modules/.bin/notype.cmd') --check
& (Join-Path $notypeInstall 'node_modules/.bin/notype.cmd')
```

`--check` 只核對套件完整性；實際啟動另會檢查權限與 CPU，並從已安裝套件複製、驗證本機模型。首次需要約 690 MB 額外模型空間及工作餘裕，不會另下載模型或 runtime。

初次設定時，在「現有模型」選取已附的 **Qwen3-ASR 0.6B Q6_K**。不用按其他模型的下載按鈕。

## 使用

1. 在設定選擇「繁體中文＋英文」或「简体中文＋English」辨識模式；介面語言可以獨立設定。
2. 點按 **右 Alt** 開始口述，再點按停止鍵結束；預設停止鍵也是右 Alt。**Esc** 取消。快捷鍵可在設定修改。
3. 開啟文字歷史，查看原稿，再按原稿的複製按鈕，手動貼到需要的 App。

未設定文字 API 時，整理／翻譯／隨便問會顯示需要設定文字服務；已辨識的原稿仍會保存並可複製。本版先用原稿複製，不以假的文字整理結果代替服務。跨 App 自動輸入後續提供。

## 資料與更新

文字歷史、設定及模型保存在 `%APPDATA%\io.github.swchen44.notype`，與安裝套件分開。歷史保留到手動刪除；當次錄音不作長期歷史。更新同一安裝目錄時不要刪除資料目錄。

如需指定其他資料位置，啟動前明確設定絕對本機路徑的 `NOTYPE_DATA_DIR`，往後每次使用同一路徑。省略此設定會使用上述預設位置。

更新時先從系統匣結束 notype，再用同一個 `$notypeInstall` 路徑及新版本的三個檔案執行安裝。已驗證相同模型會沿用；衝突模型保留並回報，不會自動覆寫。

本機套件暫不支援開機啟動登錄，介面會顯示停用；既有偏好與 Windows 項目不會被改寫。第三方授權與來源清單隨套件附上；此為私有本機試用，尚非公司部署或公開發布驗收。
