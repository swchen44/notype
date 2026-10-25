# #14 保留模型的 tiny npm gate

範圍僅為 [P14-delivery-v1](https://github.com/swchen44/notype/issues/14#issuecomment-6398408287) **Section A**；[GPT-6-Astra 核准，0 項未解問題](https://github.com/swchen44/notype/issues/14#issuecomment-6398427504)。2026-10-25，Windows x64，Node 22.15.0、npm 10.9.2（`C:/Program Files/nodejs/node.exe` 與同目錄 `node_modules/npm/bin/npm-cli.js`）。測試會回報實際 Node/npm 版本，版本或依賴形狀不符時，不能拿這次結果授權其他真實交付路徑。

公開 seam 是真正 npm CLI 與產品 `inspectRelease`；沒有 mock npm、patch Arborist 或執行 App。所有 tgz 小於16 KiB、只含合成文字（包含假的 exe/runtime/model bytes），使用 fresh owned temp roots/cache/空 npm configs。沒有真正語音模型、下載、公司設定、使用者 profile、原始 log 或權杖。

## 可執行行為與結果

```powershell
node.exe --test tests/integration/portable-retained-model-install.test.mjs
```

先以實際三個 tiny tgz 建立舊 fixture 安裝與 canonical v3 root lock，接著全新 scratchprefix 只複製 root metadata/lock 與模型的小檔。**模型 weight 不存在，原 model tgz 已刪除，更新 cache 起初為空。** 不複製舊 hidden lock、不共用模型 inode，也不讀取真實已安裝程式。

1. 只給新 App + facade 兩個 tgz 進行 offline、ignore-scripts、no-audit、no-fund 的真實 npm install。兩角色版本皆為 `0.1.1-local.1`，保留模型為 `0.1.0-qwen3-q6k.1`。成功後模型的小檔 bytes/mtime 與整個 lock node（包含 resolved/integrity/version）不變，weight 仍缺席。產品 `inspectRelease` 拒絕不完整 prefix；npm 完全結束後才 exclusive hardlink tiny immutable weight，完整公開檢查通過，沒有產生 data 目錄或啟動任何 executable。
2. 負向對照刻意把保留模型 package.json version 改成 `0.1.0-qwen3-q6k.2`，與 canonical lock／新 facade 的 exact model pin 不符。真實 npm 嘗試取得 lock 指定而已不存在的舊 model tgz，正常以 `ENOENT` 非零結束，沒有 timeout，weight 仍缺席。這個 expected npm failure 對應一項通過的測試，證明不是測試忽略依賴。

完整 gate：**2 passed／0 failed／0 skipped**。首次切片：只複製 root metadata/lock、沒有模型小檔時，實際 npm 因 model tgz `ENOENT` 而 **RED（1 failed）**；加入計畫要求的模型小檔保留後 **GREEN（1 passed）**，再加上述負向對照成為2項。這是交付配方的必要條件實驗，不宣稱修過產品 bug 或完成大型更新。

## 尚未執行

此切片沒有 native build、PDB／QA 模型回收、真正大型 staging/pack/install、舊 prefix／Starter 更新、GUI／麥克風／快捷鍵或使用者資料操作。Section B–E 的乾淨來源／build／空間／檔案鎖／使用者明確退出等前置條件仍必須分別滿足。tiny gate 不能替代實際 tarball audit、完整 installed inventory、新版 native QA 或真機 latency baseline，也不是全新三個大型 tarball 的安裝驗收。
