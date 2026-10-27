# Delivery V8 準備狀態（2026-10-27）

這是來源／準備／實際失敗的狀態紀錄，不是成品驗收通過 receipt。完整人工待驗收矩陣以 [README](../../README.md#windows-11-測試版與驗證狀態) 為準。

## 已有證據

- Runtime App 來源 A=`a180a2360009448cdf6e2f4f8e7363e0eb2dbf30`，在 R7 編譯；原生驗證／組裝來源 T=`a7fc1d79822544e6f4858cb32bcd809067367cc3`。A→T 僅 README 與既有 cfg(test) 模組的一行 ReadySignal 匯入不同。App 沒有重新編譯為 T，這次文件 commit 亦不改變建置來源。
- [R10 實際紀錄](https://github.com/swchen44/notype/issues/14#issuecomment-6431337535)：App 驗證／獨立封存、helper、list-tests、run-tests 四階段通過；兩項原生錄音計量 fixtures 為 2 passed／0 failed／0 ignored，不開啟實體麥克風。來源 [ALL40427656120](https://github.com/swchen44/notype/actions/runs/40427656120) 與 [Windows contracts40427656156](https://github.com/swchen44/notype/actions/runs/40427656156) 分開記錄，後者 artifact job skipped。
- d2 count、stage、archive 已通過並保留 ZIP；原 extract 以 independent-monitor-stale 失敗，沒有成功的原 audit。診斷性完整性檢查不追認原失敗階段。
- [V7 計畫](https://github.com/swchen44/notype/issues/14#issuecomment-6432555641)／[Astra 核准](https://github.com/swchen44/notype/issues/14#issuecomment-6432586206) 的準備已凍結：121 passed／0 failed／0 skipped，55 個 fixture Workers 及 owned children 全數 terminal；[root 公開確認完整獨立 Spec0／Standards0 及準備結果](https://github.com/swchen44/notype/issues/14#issuecomment-6433705942)。這些是準備與受控測試；該準備紀錄完成時 actual gate、d3 操作目錄不存在，尚未執行 finalize、既有副本驗證或隨附 Node 的公開 check。

V7 準備 receipt `p14-delivery-v7-final-receipt.json` 的狀態是 `PREPARATION_COMPLETE_NOT_ACTIVATED`；其 current fixture 為 8,511 bytes／SHA256 `34a819af98ae45526a88134a340c1590099d882bbeba658e31d426fae85b7ff3`。凍結 runner／policy／session 的 SHA256 分別為 `7a105fa9f2f78dd00af9c29169729937425b1b412c3f293f13d9d1f6530bb284`、`14ad780ddced3484b6684995a4c0863ccc88cc4da85bb6ec8e836393dbc034a6`、`0101cabc32d928de0011cc33ea45fdf1602924ab25cd4a5369d412d31d35eea8`。舊證據保留原身分，不改標成 V8 已測。

## 已完成的預算修訂與 admission

[P14-delivery-v8-v1](https://github.com/swchen44/notype/issues/14#issuecomment-6433597289) 已獲 [GPT-6-Astra APPROVED／0 unresolved](https://github.com/swchen44/notype/issues/14#issuecomment-6433619965)。已知按時間順序正向累計的下限至少 C2,245,115,904／D1,078,300,672 bytes；C 在加入完整稽核預留前，已超過舊 post-native 上限 2,081,423,360 bytes（1,985 MiB）達 163,692,544 bytes。後續可用空間回升不沖銷已計入用量，未觀測到的短暫峰值與歸屬仍為 UNKNOWN。

計畫引用的實體空間樣本約 C10GB／D2.38GB，足以符合當時的實體空間投影，卻不能抵銷累計超限。V8 對當時尚未啟動的這次最終稽核明確增加 C512MiB：post-native 1985→2497MiB，aggregate 2241→2753MiB；原上限與超限事實保留。實際稽核階段仍為 C64MiB／D8MiB，D、P25、floors、guards、deadline／drain 與 live freshness 界線維持原值。

[新的 preparation carry／轉換見證與 root 獨立重播紀錄](https://github.com/swchen44/notype/issues/14#issuecomment-6433705942)保留。V8 程式與86項current測試已通過，另保留未變更session的64項證據（不是重新執行），全部fixture owned children／Workers terminal；完整獨立Spec0／Standards0及leases交還已完成，見[完整審查紀錄](https://github.com/swchen44/notype/issues/14#issuecomment-6433879274)。[root carry／當下 admission 重播](https://github.com/swchen44/notype/issues/14#issuecomment-6433907847)已通過，包含後續已知正向變化；這些準備與admission並非成品驗收。

## 實際 d3 失敗與當前阻塞

[實際安全紀錄](https://github.com/swchen44/notype/issues/14#issuecomment-6433977441)：2026-10-27 04:41 UTC，root啟動唯一一次固定d3 finalize，約4.4秒後在predecessor-proof以v7-session-worker-rejected失敗，沒有前置證明被接受。ZIP／既有副本驗證、extracted-content及隨附Node公開檢查均未到達。完整failure及完整state已保存並核對內容hash綁定；accountingComplete=true、全部owned children／Workers terminal、terminalUnproved=false、observationErrors為空且persistenceIncomplete=false。

當次最大parent gap約112ms、space age586ms、allocation age795ms、source age676ms，均在原guards內；這不是已證實的stale-monitor失敗。[後續唯讀診斷](https://github.com/swchen44/notype/issues/14#issuecomment-6434068783)已確認底層v7-metadata-fixed-task：原noReparse(paths, tree=false)的實際preflight呼叫省略第二參數，替換adapter遺失該預設值，收到undefined卻須嚴格匹配false，因而拒絕有效呼叫。14個路徑逐位元組相同，無需路徑正規化。實際Worker鏈在2.51秒重現、最小probe約138ms；僅在診斷副本還原預設值的13項反事實檢查通過，未執行production修正或重試，也不是完整predecessor／finalization通過。[V9計畫](https://github.com/swchen44/notype/issues/14#issuecomment-6434094625)已獲[實際GPT-6-Astra APPROVED0](https://github.com/swchen44/notype/issues/14#issuecomment-6434122551)；closure／carry、production修正、current tests、完整來源Spec0／Standards0、root admission及唯一一次d4實際執行均待完成，目前未執行d4。舊code／gates／failed records及d2 ZIP／獨立解壓副本保留原狀。

## 實際完成與發布的界線

未來驗收成功仍須完整現存 ZIP 與獨立解壓樹的公開 inspectors、strictModels、固定隨附 Node `--offline-bundle --check`、累計資源／freshness 與所有 owned child／Worker terminal 證據。既有副本驗證與完整性稽核不改寫原 d2 extract 的 FAILED，也不能補造成功的 v6 extract／audit；本段不是新執行授權。

實際通過後才可公告已授權的 private GitHub prerelease、ZIP、外部 SHA256SUMS、Windows 11 guide 與安全驗證紀錄。目前沒有已驗收下載；clean-host／SmartScreen／Defender／公司執行政策、Excel／研究工具 CSV 開啟及 README 所列 GUI／人聲／品質／公司服務／五 App／跨 OS 驗收仍待實測。也不以套件完整性或計畫通過宣稱上述驗收完成。

[R9 編譯失敗](https://github.com/swchen44/notype/issues/14#issuecomment-6430655722)、d1 授權 BAT 誤擋、d2 extract 失敗及未知觀測均保留；[舊 DeepSeek helper HALTED 回合](https://github.com/swchen44/notype/issues/25#issuecomment-6411081019) 仍為六次請求／DB 五筆／當時 CSV 四筆、費用 UNKNOWN，後續取消／重試未執行，不是本版 App API 驗收。本次文件更新沒有執行 runner／App／API、讀取 key／profile、操作模型或更動舊 gates／輸出。
