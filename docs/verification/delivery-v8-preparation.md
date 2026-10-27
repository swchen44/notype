# Delivery V8 準備狀態（2026-10-27）

這是來源／準備／失敗的狀態紀錄，不是實際 finalization 或成品交付 receipt。完整人工待驗收矩陣以 [README](../../README.md#windows-11-測試版與驗證狀態) 為準。

## 已有證據

- Runtime App 來源 A=`a180a2360009448cdf6e2f4f8e7363e0eb2dbf30`，在 R7 編譯；原生驗證／組裝來源 T=`a7fc1d79822544e6f4858cb32bcd809067367cc3`。A→T 僅 README 與既有 cfg(test) 模組的一行 ReadySignal 匯入不同。App 沒有重新編譯為 T，這次文件 commit 亦不改變建置來源。
- [R10 實際紀錄](https://github.com/swchen44/notype/issues/14#issuecomment-6431337535)：App 驗證／獨立封存、helper、list-tests、run-tests 四階段通過；兩項原生錄音計量 fixtures 為 2 passed／0 failed／0 ignored，不開啟實體麥克風。來源 [ALL40427656120](https://github.com/swchen44/notype/actions/runs/40427656120) 與 [Windows contracts40427656156](https://github.com/swchen44/notype/actions/runs/40427656156) 分開記錄，後者 artifact job skipped。
- d2 count、stage、archive 已通過並保留 ZIP；原 extract 以 independent-monitor-stale 失敗，沒有成功的原 audit。診斷性完整性檢查不追認原失敗階段。
- [V7 計畫](https://github.com/swchen44/notype/issues/14#issuecomment-6432555641)／[Astra 核准](https://github.com/swchen44/notype/issues/14#issuecomment-6432586206) 的準備已凍結：121 passed／0 failed／0 skipped，55 個 fixture Workers 及 owned children 全數 terminal；[root 公開確認完整獨立 Spec0／Standards0 及準備結果](https://github.com/swchen44/notype/issues/14#issuecomment-6433705942)。這些是準備與受控測試，actual gate、d3 操作目錄仍不存在，沒有執行 finalize、認養或隨附 Node 的公開 check。

V7 準備 receipt `p14-delivery-v7-final-receipt.json` 的狀態是 `PREPARATION_COMPLETE_NOT_ACTIVATED`；其 current fixture 為 8,511 bytes／SHA256 `34a819af98ae45526a88134a340c1590099d882bbeba658e31d426fae85b7ff3`。凍結 runner／policy／session 的 SHA256 分別為 `7a105fa9f2f78dd00af9c29169729937425b1b412c3f293f13d9d1f6530bb284`、`14ad780ddced3484b6684995a4c0863ccc88cc4da85bb6ec8e836393dbc034a6`、`0101cabc32d928de0011cc33ea45fdf1602924ab25cd4a5369d412d31d35eea8`。舊證據保留原身分，不改標成 V8 已測。

## 當前阻塞與核准變更

[P14-delivery-v8-v1](https://github.com/swchen44/notype/issues/14#issuecomment-6433597289) 已獲 [GPT-6-Astra APPROVED／0 unresolved](https://github.com/swchen44/notype/issues/14#issuecomment-6433619965)。已知按時間順序正向累計的下限至少 C2,245,115,904／D1,078,300,672 bytes；C 在加入完整稽核預留前，已超過舊 post-native 上限 2,081,423,360 bytes（1,985 MiB）達 163,692,544 bytes。後續可用空間回升不沖銷已計入用量，未觀測到的短暫峰值與歸屬仍為 UNKNOWN。

計畫引用的實體空間樣本約 C10GB／D2.38GB，足以符合當時的實體空間投影，卻不能抵銷累計超限。V8 只對這次尚未啟動的最終稽核明確增加 C512MiB：post-native 1985→2497MiB，aggregate 2241→2753MiB；原上限與超限事實保留。實際稽核階段仍為 C64MiB／D8MiB，D、P25、floors、guards、deadline／drain 與 live freshness 界線維持原值。

[新的 preparation carry／轉換見證與 root 獨立重播紀錄](https://github.com/swchen44/notype/issues/14#issuecomment-6433705942)已保留。**尚待完成**：新 V8 程式／測試、完整獨立 Spec0／Standards0、全部 leases 交還、包含後續正向變化的 fresh exclusive gate／當下 admission，以及唯一一次實際 d3 finalize。計畫核准與已記錄的實體空間都不是實際 admission。此 checkpoint 不把正在準備的 V8 程式或測試列為已完成。

## 實際完成與發布的界線

後續成功須有新 V8 receipt：完整現存 ZIP 與獨立解壓樹的公開 inspectors、strictModels、固定隨附 Node `--offline-bundle --check`、累計資源／freshness 與所有 owned child／Worker terminal 證據。這是新的唯讀認養與完整性稽核，原 d2 extract 仍為 FAILED，不能補造成功的 v6 extract／audit。

實際通過後才可公告已授權的 private GitHub prerelease、ZIP、外部 SHA256SUMS、Windows 11 guide 與安全驗證紀錄。目前沒有已驗收下載；clean-host／SmartScreen／Defender／公司執行政策、Excel／研究工具 CSV 開啟及 README 所列 GUI／人聲／品質／公司服務／五 App／跨 OS 驗收仍待實測。也不以套件完整性或計畫通過宣稱上述驗收完成。

[R9 編譯失敗](https://github.com/swchen44/notype/issues/14#issuecomment-6430655722)、d1 授權 BAT 誤擋、d2 extract 失敗及未知觀測均保留；[舊 DeepSeek helper HALTED 回合](https://github.com/swchen44/notype/issues/25#issuecomment-6411081019) 仍為六次請求／DB 五筆／當時 CSV 四筆、費用 UNKNOWN，後續取消／重試未執行，不是本版 App API 驗收。本次文件更新沒有執行 runner／App／API、讀取 key／profile、操作模型或更動舊 gates／輸出。
