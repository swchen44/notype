# Windows 11 開發接手

## 最新 checkpoint：#26/#27 來源驗收完成

2026-10-26 08:56：完整多連線／key fallback／thinking／模型清單／安全 API diagnostics 的 exact source `ab9e785e6b1faac2d70a759ed70b6582245b49ab`，以保留歷史的 merge `c5605f44ecb50ad36ee16e57f7e26973b8887974` 合入 `implement/notype-mvp`。獨立 Spec **0 未解**、Standards **0 違反／0 未解 smell**；[完整 ALL CI40251455558](https://github.com/swchen44/notype/actions/runs/40251455558) 在 exact ab9 **completed/success**，兩個 job 全通過：core49＋doc1、platform4、desktop158（另1個由通過的 parent 明確執行的 controlled child role）、Node24／18、UI3／66；lint／Clippy／TypeScript／Vite 通過。見 [最終 receipt／計畫與審查](docs/verification/provider-config.md) 及 [parent gate](https://github.com/swchen44/notype/issues/25#issuecomment-6405850082)。#26/#27 的 issue 明訂 source-only，來源／owned fixtures 已完成；native／live 驗收留在 #25/#14/#8，父規格仍 open。

下一個產品實作 frontier 為 **#28 模型根目錄選取／重用**（[計畫](https://github.com/swchen44/notype/issues/28#issuecomment-6405356803)／[Astra 核准0](https://github.com/swchen44/notype/issues/28#issuecomment-6405400881)），之後 **#29 完整離線 ZIP／可選無 key 管理者預設**（[計畫](https://github.com/swchen44/notype/issues/29#issuecomment-6405357391)／[核准0](https://github.com/swchen44/notype/issues/29#issuecomment-6405428255)）；這次 merge 尚未包含兩者產品實作。使用者自行移動模型，App 重新指定／重用，不替使用者搬移。基本 App 缺模型可開設定，完整 ZIP 嚴格驗證宣告模型。**P14-build-v5／delivery-v5 現已實際 Astra 核准0**：[build 計畫](https://github.com/swchen44/notype/issues/14#issuecomment-6405766076)／[核准](https://github.com/swchen44/notype/issues/14#issuecomment-6405783543)，[delivery 計畫](https://github.com/swchen44/notype/issues/14#issuecomment-6405766320)／[核准](https://github.com/swchen44/notype/issues/14#issuecomment-6405789439)；依賴 #28/#29，核准不代表新版 App、原生 build 或 ZIP 已完成。

來源階段 leases 全部交還，worktrees／review references 保留。沒有本機 Cargo、原生 App、真實 home/key、使用者 profile、live／paid API、npm／ZIP 交付或清理；保護原 App／Starter／模型／歷史。最近先前資源觀測約 C6.85GB／D4.14GB，舊空間不足文字已過時；任何 native 工作仍先做當下 budget／resource gate。#25/#8／native／live／公司與五 App 未驗收項目維持 open。以下所有較舊 checkpoint 的時態，以本節及 live issues 為準。

更新日期：2026-10-26。Repository：[swchen44/notype](https://github.com/swchen44/notype)，private。

使用者已決定在自己的 **Windows 11** 開發，MVP 只交付 **Windows amd64／x64**；架構預留 **macOS／Linux**，見 [ADR 0003](docs/adr/0003-shared-core-platform-adapters.md)。目前尚未交付 macOS／Linux App。Q1～Q15 為原始決策，後續三場景與點按操作更新以 #8／decision log 為準，不重問已確認項目。

## 目前 checkpoint

2026-10-26 台灣時間 08:40 接續狀態：integration `cfd8c1b` 已包含 #27 provider UI（產品 `a8aacbe`、凍結 `7d2e4e5`）及模型資料夾／離線 ZIP 決策文件。#26 多連線／安全 config／API diagnostics 的 composed candidate `2977db0635e7f02afb8064ad33abd644bb0c246e` 在 [完整 ALL CI40249253105](https://github.com/swchen44/notype/actions/runs/40249253105) completed/success，包含真實 owned proxy 與 headers 後停滯 body 的回歸。獨立 Spec review 發現三項問題：準備失敗時遺失翻譯設定、model-list Error 後 pending guard 未釋放、無關設定儲存繞過限流；Standards 無規則違反，另有 credential policy 重複建議。修正依 [P25-config-reviewfix-v2](https://github.com/swchen44/notype/issues/25#issuecomment-6405575893)／[實際 GPT-6-Astra APPROVED0](https://github.com/swchen44/notype/issues/25#issuecomment-6405614167) 由 diagnostics 單一 fixer 做 RED→GREEN，最後仍須 exact CI 與兩軸複審後合入。這不是未修問題的 source0，也不是新版 App 已交付。

使用者最新模型資料夾需求已完整 to-spec：[#28 模型根目錄計畫](https://github.com/swchen44/notype/issues/28#issuecomment-6405356803)／[Astra 核准0](https://github.com/swchen44/notype/issues/28#issuecomment-6405400881)，與 [#29 完整離線 ZIP／無 key 預設](https://github.com/swchen44/notype/issues/29#issuecomment-6405357391)／[核准0](https://github.com/swchen44/notype/issues/29#issuecomment-6405428255)。待 #26 最終 source gate 後實作：使用者可自行移動整個模型資料夾，重新安裝後指定新位置重用，App 不搬移／複製／刪除模型；動態套用須涵蓋錄音、預載、解碼、下載與真正 worker 結束的 admission／generation fences。基本 App 不強制取得模型，缺模型仍可開啟選擇資料夾；整包 ZIP 完整性檢查維持嚴格。離線包帶完整 runtime／已驗證模型及可選管理者無 key 內網預設，已有個人設定優先。實際公司 IP／model ID 未知，不猜測。見已修訂 [ADR0001](docs/adr/0001-portable-npm-distribution.md)。兩份計畫核准不代表來源或 ZIP 已完成。

本機資源最新只讀值（00:35 UTC）：C 6,862,983,168 B、D 4,151,177,216 B，並非仍卡在舊 1.8GB 狀態，沒有刪任何快取／個人檔。無 notype 程序的觀測只適用該時刻；任何原生 QA 前仍需當下檢查。#14 build/delivery-v3 只有 DRAFT、未核准，必須合併 #28/#29 的 optional-model／ZIP 語意另訂新版本並實際 Astra 審查，不能逕行本機 Cargo／組包。持續保護舊 App／local-trial／Starter／原模型及歷史；沒有真實 API 呼叫或 key 值輸出。新的 live 計畫仍待 source／artifact／預算／免費資格 gate，不啟動舊 source41 helper。

以下 checkpoint 保留先前證據；「最新／正在實作／空間不足」等時態以本節及 live issues 為準。

## 先前 checkpoint

2026-10-26 最新 source：#25 第一階段錯誤處理 `d4315522171a393f01012a934d78b110cd72fb25` 已合入並推送 `ec37ab6741422c9e7ba5aeeb5f93553cd508f401`。依 [P25-errors-v1](https://github.com/swchen44/notype/issues/25#issuecomment-6403957943)／[Astra APPROVED0](https://github.com/swchen44/notype/issues/25#issuecomment-6403993599) 及 [context 優先序修正 v2](https://github.com/swchen44/notype/issues/25#issuecomment-6404301764)／[核准](https://github.com/swchen44/notype/issues/25#issuecomment-6404307928)，真實 HTTP adapter 現在提供安全錯誤分類、保存可重試原稿／回譯前向結果與 usage，歷史介面有 EN／繁／簡訊息；沒有自動重送。最終獨立 Spec／Standards 皆 0，[完整 Linux CI 40240970555](https://github.com/swchen44/notype/actions/runs/40240970555) 在精確 `d431552` **completed／success**。見 [provider errors receipt](docs/verification/provider-errors.md)。這不代表 Windows 新版 App 或真實 API 已驗收。

#25 其餘使用者決策已確認，不重新訪談：DeepSeek Flash4.1 的本輪總預算 **US$2**，取代先前 US$0.10；Gemini／OpenRouter 僅可免費路線。App 非空 key 優先、家目錄 `~/.env` 後備；已授權的 names-only 查核只確認三個預期變數存在，未輸出值。多份 connection、一份 active，包含 Custom／Local；Local 先指明確設定的 OpenAI 相容 endpoint，不自動啟動或下載模型。UI 的 Gemini／OpenRouter model ID 預設空白，由使用者填寫；View model IDs 只顯示／Copy，手動貼上，不自動選或覆寫。實驗模型 ID 與 UI 設定分開。Thinking 預設 OFF；不能關閉時提示限制並使用已知最小值，requested／effective 分開記錄。

上述多連線／key fallback／model list／參數與 thinking 屬 [後續 config-v2 計畫](https://github.com/swchen44/notype/issues/25#issuecomment-6404377843)，已獲 [GPT-6-Astra APPROVED0](https://github.com/swchen44/notype/issues/25#issuecomment-6404401509)，從已合入的第一階段 gate 接續實作；目前 receipt 不包含第二階段完成證據。debug 後續切片另列，沿既有 P20 白名單／預設 ON／可關閉清除／預覽匯出／手動 GitHub issue 附檔，須有精確計畫核准及 resource gate，不收 secrets／逐字稿／raw body，與歷史業務計量獨立。Gemini 的免費計費資格仍是外部 live gate，不能只因模型標價免費就發送；免費資格未證明不阻擋 owned-fixture 實作。舊 source41 helper 維持固定來源／舊模型／舊單輪預算的歷史證據，**不直接啟動新的多 Provider／US$2 測試**。沒有付費請求，未修改現用 App／profile；磁碟不足，未執行新的本機 Cargo／Tauri 建置。

前一來源 checkpoint：`af4ce80a315de025c0d8001e71081c3794c8fea2`，分支 `implement/notype-mvp`。#23 完整 source `41cb62da8482592bde02cc0b2b52460e361e75a9` 已合入；獨立最終 Spec／Standards 皆 0，[完整 Linux CI 40224287522](https://github.com/swchen44/notype/actions/runs/40224287522) 在精確 `41cb62d` 通過。這是來源與限定驗證完成，不代表使用者 helper／付費 API／新版 App 已交付。

#23 實際 production provider → owned HTTP fixture → SessionService／SQLite → 單一 UTF-8 CSV 的 bridge 6 tests 通過（含 12 error variants、固定 8 requests、取消／同 ID retry／回譯、6-row CSV）；shared console algorithm 7 tests 通過。受影響 core 43＋doc 1、Clippy 等 portable checks 已通過。原生 Windows console 的實際 direct-rustc driver **2 tests／9 cases 通過**，確認專屬 hidden owned console、非回顯輸入及模式還原，沒有修改使用者既有 terminal，見 [native console receipt](docs/verification/deepseek-native-console.md)。這些 fixture 使用非機密合成輸入，未使用真實 key、未付費呼叫。

Windows helper 的 UNACTIVATED 編譯與參數拒絕之後，依 [精確來源啟用建置計畫](https://github.com/swchen44/notype/issues/23#issuecomment-6402345307)／[GPT-6-Astra 核准](https://github.com/swchen44/notype/issues/23#issuecomment-6402369558)，已完成精確 `41cb62d` 啟用型別／連結建置、固定參數拒絕、無 key 的專屬 console gate smoke 與 dated 私有 package，見 [完整 receipt](docs/verification/deepseek-reviewed-helper.md)。實際入口為 `D:/git/notype-build/compiler-temp/p23-reviewed-helper-41cb62da/Start-deepseek-probe.cmd`，binary SHA-256 為 `43d3a2c26c65dfcfdde8d51d7487b4782acd6e6829073447cff49e997363925d`。無 key smoke 在 round／results 建立前拒絕，前後三個固定路徑皆不存在，沒有留下 owned helper 程序；AvailableSpace 已連結，但本輪沒有執行實際 quota 查詢，不能宣稱該路徑 runtime 通過。

原 source41 的本機 key-entry 入口僅保留已完成的限定證據；最新 key 來源與模型／thinking／本輪 US$2 決策由 #25 接續，不能把舊 `RUN P23-live-v1` 當成新需求的啟用方式。沒有 key 值被輸出或複製，沒有付費呼叫。#23、#22、#14、#8 保持 open，真實模型／用量／文字品質與 CSV 核對仍待執行；新的 live 計畫與精確 artifact 尚未完成。

#22 backend／#24 UI 與 readiness／owner-release 修正已合入；frontend-only #24 已結案，native acceptance 留在 #22／#14。Windows core 43＋doc 1、provider 20／workflow 15／DTO 1、Clippy 通過；UI integration 36／unit 1 通過；對應來源差異獨立 Spec／Standards 皆 0。[Linux CI 40216512564](https://github.com/swchen44/notype/actions/runs/40216512564) 通過產品來源 `fb4ab689`，後續 receipt `8214d317` 產品／測試雜湊相同；不宣稱文件 commit 自身跑過該 CI。見 [history metrics](docs/verification/history-metrics.md)、[UI](docs/verification/history-api-ui.md)、[owner release](docs/verification/history-owner-release.md)。

新版 Tauri App `0.1.1` **尚未建置／安裝**。Native App build 起始 C、D 均須 ≥2 GiB，執行中 C≥2 GiB／D≥1 GiB 並保留 32 MiB guard；D 尚低於起始門檻。唯一 Cargo target 為 `D:/git/notype-build`，舊 target 不重建；所有已完成階段的 Cargo leases 已明確交還。helper 後續執行仍須其精確計畫與資源 guard；不得與其他 writer 重疊。使用者最新空間問題是 C 至少 500 MB、D 至少 2 GB，尚待回覆。現用 App／`local-trial` 資料未動，不降低 floor，不自行刪個人檔案。

完整未完範圍保留在 [主規格 #8](https://github.com/swchen44/notype/issues/8)：Tauri native build／bindings／Clippy／recording fixture；新版乾淨安裝與 exact reset 路徑；右 Alt readiness 真機基線與原因定位、tray／diagnostics／秒數 overlay 的原生操作；繁簡中英人聲品質及完整 session p95；公司 API／TLS／CA／Proxy、五個 App 自動輸入；完整 npm 配送／公開 notices／發布與公司 Proxy 取得。公司與五 App 依使用者範圍延後，未以 DeepSeek fixture 或 paid acceptance 代替。macOS／Linux 僅預留架構，未交付 App。使用者取消本輪舊版相容，採 fresh schema4 與新版本化資料目錄，現在不刪任何現用程式／資料。

所有新實作仍須先取得對應 issue 的 [GPT-6-Astra 計畫 APPROVED／0 未解問題](docs/agents/plan-review.md)。[P23-live-v1](https://github.com/swchen44/notype/issues/23#issuecomment-6399600403)／[核准](https://github.com/swchen44/notype/issues/23#issuecomment-6399640205) 與 [P14-build-v2](https://github.com/swchen44/notype/issues/14#issuecomment-6399567273)／[核准](https://github.com/swchen44/notype/issues/14#issuecomment-6399589852)、[delivery-v2](https://github.com/swchen44/notype/issues/14#issuecomment-6399567585)／[核准](https://github.com/swchen44/notype/issues/14#issuecomment-6399590175) 不會免除各自 binary／runtime／資源 gate。

本輪曾誤帶 App Release overrides，造成錯 profile 依賴重編，135,110,656 B 超過原 128 MiB 監測額 892,928 B；runtime floors 未越界，該次程序已停止。372 個新快取檔案完整保留；精確刪除雖有計畫核准，工具自動核准審查在 CreateProcess 前以「blocked by policy」拒絕，**零檔案被刪，不改格式／拆批重試**。[不刪檔恢復計畫](https://github.com/swchen44/notype/issues/23#issuecomment-6401040948)／[核准](https://github.com/swchen44/notype/issues/23#issuecomment-6401056228) 及 [剩餘額計算](https://github.com/swchen44/notype/issues/23#issuecomment-6401113135)／[核准](https://github.com/swchen44/notype/issues/23#issuecomment-6401126474) 保留單一 v2 baseline／累積 ledger、192 MiB cap／160 MiB stop 與各自監測證據，不能重設 ledger 取得新額度。

## 歷史快照

以下按當時證據保留；若有「正在實作／待審查／未合入」等舊狀態，以本頁最新 checkpoint 與 live issues 為準，不重做已完成切片或重問已確認選項。

2026-10-25 本機回饋與 P19 review fixes 已合入並推送 `89d239c441bf8f4482b8a3fd9a5f9b7c3b0a1b80`；使用者實際包仍是舊版。P20/P21 基線雙軸複核 Spec 0／Standards 0；P19 的計時起點及 observe 測試缺口已依核准計畫修正，待最終獨立複核。修正 std 18／overlay UI 8／lint/build 通過，先前完整 UI 22 通過。原生建置、bindings、實機與本輪大型交付尚未執行；右 Alt 真機等待仍未定位。

使用者已確認右 Alt 短按有效，但很久才看到收音提示；不是長按模式。原 `notype.log` 為 0 bytes，不能推定沒有錯誤。diagnostics 預設開啟／可關閉／可清除，敏感 token/key 中段遮罩，短值與模糊值全遮；白名單仍不收集憑證。**所有新實作須先通過 [GPT-6-Astra issue 計畫審查](docs/agents/plan-review.md)。** P20-v3／P21-v3 分別獲 [核准](https://github.com/swchen44/notype/issues/20#issuecomment-6398249625)／[核准](https://github.com/swchen44/notype/issues/21#issuecomment-6398249848)；[P19-latency-v3](https://github.com/swchen44/notype/issues/19#issuecomment-6398258525) 亦已 [核准](https://github.com/swchen44/notype/issues/19#issuecomment-6398268939)，相依 commits 已於 [gate 留言](https://github.com/swchen44/notype/issues/19#issuecomment-6398707430) 登錄，正在獨立 worktree 實作觀測，未確認真機等待原因。P14 建置仍有前置條件；[交付計畫](https://github.com/swchen44/notype/issues/14#issuecomment-6398408287) 已 [核准](https://github.com/swchen44/notype/issues/14#issuecomment-6398427504)，只完成 Section A 真實 tiny npm 2 tests，不代表大型包裝／更新／App 驗收已執行。

使用者再次啟用 to-spec／implement-spec，要求涵蓋所有未完項目；[主規格 #8](https://github.com/swchen44/notype/issues/8) 已補完整剩餘工作表。[#22 歷史／CSV／effort](https://github.com/swchen44/notype/issues/22) 的 [P22-v1](https://github.com/swchen44/notype/issues/22#issuecomment-6399288240) 已 [Astra 核准0](https://github.com/swchen44/notype/issues/22#issuecomment-6399333180)，獨立 history-metrics worktree 實作中。最終決策：原稿／結果分別計字，中文逐字／英文字詞、排除空白標點；停止到結果可用總耗時另列 STT/API。單一 UTF-8 CSV 每ID一列，含兩份文字與計量，跨重試／回譯加總 input/output tokens、API實際回報費用按幣別合計，模型／effort列多值，不再雙CSV／API1欄位。費用未知不可估算或補0。使用者明確取消本輪舊版相容，採 fresh schema4 與乾淨版本化資料目錄，交付時列明移除／重建路徑；現在不刪任何現用程式或資料。P14 build/delivery v2 草擬中，舊案同profile回退條件不再適用。[#23 DeepSeek](https://github.com/swchen44/notype/issues/23) 本輪授權 US$0.10（含取消／重試／回譯），Flash非思考短句；live runner／預算guard待另案審查，未取得key／未付費呼叫。本機入口準備好後才請使用者輸入key，不貼聊天。

正式功能與審查修正已合入 `implement/notype-mvp`。#9 桌面基線、#10 共用核心、#11 文字流程、#17 三場景／提示詞已依各自 receipt 結案；#12／#13 的功能已合入，品質／實際操作仍有未驗收項目。兩個獨立 reviewer 的 Standards／Spec findings 均已修正，最終限定複核各 0 項，見 [完整審查紀錄](docs/verification/local-code-review.md) 及 [修正與 RED→GREEN](docs/verification/local-review-fixes.md)。

正式 Release 建置來源 `239d6f05cdf7883032889b8d25042a7a427d72d9`，App SHA256 `7a70d5abe3558c2b12d2c8a16c0ed583bf0a231d3cbe8ba14fa89300b74257ab`，host helper SHA256 `6cd855cb408ccee695d800ce70de8c94ddd76e4d38e510a50c68976aac5bd38d`。其後 `a2d2328` 只有平台 Rustfmt 換行；[Linux CI 40162614988](https://github.com/swchen44/notype/actions/runs/40162614988) 通過核心 27、桌面 34（debug）、平台 4、Node 23 unit +16 integration、fmt／clippy。Windows Release 桌面契約為 33 項（native token target 的條件編譯數量不同），另有 9 項核心 session 與 4 項平台檢查。繁簡各 570 keys、前端 lint/build 通過。

**本機試用包已備妥**，入口 `D:/git/notype-local-artifacts/Start-notype.cmd`，見 [試用說明](docs/local-trial.md)。三個實際私有 tarball 為 `notype-0.1.0-local.1.tgz`、`notype-win32-x64-0.1.0-local.1.tgz`、`notype-model-0.1.0-qwen3-q6k.1.tgz`。內容清單／regular bytes／hash、實際離線安裝、normal-medium CPU 兩模式與固定 WebView GUI／模組檢查均通過；同 facade `.1→.2` 的實際更新、第二次 60 秒 GUI 重開與資料庫原有 ID／時間／文字、模型 inode／mtime 保留亦通過。這個更新沒有換 native App／模型／schema，不能視為完整 #14 更新驗收。五次自有 fixture job（含修正前的 no-tray 可見視窗失敗）均已清到 0，使用者試用採全新資料目錄及預設快捷鍵，未自動替使用者錄音。

首次 npm 試裝觸及 D 槽 1 GiB 下限而停止，失敗證據保留；以全新 C 槽 task-owned cache 重試後成功。npm 取得本機 tarball metadata 會先解包暫存，須計入約 1.49 GB 解包＋1.03 GB cache 的峰值；成功安裝及核對後已先清除此暫存，再啟用 690 MB 模型。重試觀察最低 C 2,661,081,088 bytes／D 2,749,550,592 bytes，未降低 C≥2 GiB／D≥1 GiB 界線。`npm --offline`／本機 CPU 證據不等於做過系統斷網或完整封包監測。

Qwen 固定 6 cases／12 次推論與新版 converter 的 3 個 native 保存案例已完成，見 [Qwen](docs/verification/qwen3-quality.md)／[語言](docs/verification/languages.md)；首次 normalization 1176 ms、重用 0 ms 是階段測量，並非完整 session p95。人工原生 suite 的 1 次完整通過／2 次 Pending 斷言失敗與未知精確焦點差異保留於 [原生輸入 receipt](docs/verification/guarded-input.md)；舊 QA 入口已停用，新入口未啟用，不再要求使用者盲目重跑。整體規格仍未結案。

**使用者最新確認的本輪交付**：先完成「錄音 → 繁／簡中英本機 STT → 本機歷史 → 複製」。其他 App 自動輸入下一階段再補。使用者目前不在公司網路，公司 API／Proxy 與三場景真實文字品質驗收延後，現在不需 key；不另選 provider 或假造生成結果。本機 pilot 使用 Qwen3-ASR 0.6B Q6_K，完整 artifact/hash／Apache-2.0 LICENSE／NOTICE 見 Qwen receipt。#14 本機實際包與限定 proof 已完成；完整 native 更新／中斷／鎖檔、公開發布、公司 Proxy、五個 App、人聲品質及 p95 仍未驗收。此里程碑已同步 #8，不代表刪除完整規格的後續驗收。

2026-10-25 最新需求：口述、翻譯、隨便問三場景，提示詞獨立可編輯／保存／還原；使用者只要還原預設提示詞。開始鍵依序 Right Alt、Right Alt + Right Shift、Right Alt + Space；改為點按開始，錄音中以獨立可設定停止鍵結束（預設 Right Alt），Esc 取消，取代舊按住／放開。公司 API 格式已確認為 OpenAI 相容 Chat Completions。隨便問依意圖回答或代寫；**使用者已取消網域擷取，情境只保留 App 名稱，不讀頁面／輸入內容／標題或網址**。回譯可選且預設關閉，啟用時先顯示譯文及回譯，明確確認後才嘗試受保護交付。詳見 [設計紀錄](docs/research/11-scenario-prompt-design.md)。本輪產品選項已回答，不重問；新未定案行為仍須詢問。

2026-10-25 Prompt 補充：使用者提供角色／處理原則／格式／限制及 few-shot 草稿；已加入 #8／#11 的保真文字整理設計。只處理原稿文字，保留數字／否定／英文／code／既有不確定標記，依上下文刪贅詞，不新增講者或臆測音訊；原稿及術語表當作資料，不執行其指令。中英排版保護字面值，提示詞依當次辨識模式選字形，與介面 locale 獨立。真正模型品質仍待驗收，無需為 prompt 設計先提供 key。

2026-10-25 implement-spec 已啟動：整合分支 `implement/notype-mvp`（起點 `be6f0a5`），工作圖 #9～#17，具 GitHub 原生 blocking edges，見 [TODO](TODO.md)。#11 接合後解鎖 #12 語言及 #17 場景／提示詞；#13 需等待 #17 場景介面，再做配送／目標驗收。公司連線／真實 App／Proxy 未就緒時 #15／#16 保持未完成。實作以 TDD、獨立 worktree、整合與最終 code review 推進；只有實際達成驗收才結案。

2026-10-25 語音需求補充：本機 STT 必須支援「繁體中文＋英文」及「簡體中文＋英文」兩種同段中英混合模式，保存模式偏好，預設沿用繁體中文＋英文，並在 LLM 前產生與保存所選中文字形的原稿。介面 locale 與辨識模式獨立；模式切換不改動既有歷史，整理結果預設沿用當次原稿字形。規格 #8 與 STT issue #3 同步此需求，兩種中文品質／效能尚未驗收，英文 tiny.en smoke 不能代替。

2026-10-25 補充：介面納入繁體中文（zh-TW）與簡體中文（zh-CN），可切換並保存偏好，翻譯資源跨平台共用；介面語言獨立於 STT／文字輸出，不自動改寫原稿、整理結果或文字歷史。已更新規格 #8，尚待正式介面實作。使用者要求真正需要 API key 時再通知；目前以本機 STT／HTTP fixture 繼續開發，不要求先申請或提供 key。

2026-10-25：已依 to-spec 發布 [完整規格 #8](https://github.com/swchen44/notype/issues/8)，標記 ready-for-agent；使用者確認主要測試 seam。正式 App 方向為 Handy 的 Rust + Tauri 2 + React／TypeScript + SQLite；現有 Node 實作為 prototype／驗收先例。先建立共用核心與平台 adapter 的 contract，將 Windows 專用資料位置及原生互動隔離，再繼續正式 App 整合。macOS／Linux 目前只規劃架構，不宣稱已可執行。

2026-10-24 Windows 接手成果：產品與 private repo 改名為 **notype**（本機資料夾仍為 notype_study）。GitHub CLI、CMake、Bun、Rust、MSVC／Windows SDK 已安裝，Rust／C++ 編譯及執行驗證通過；Vulkan 安裝被取消、尚未完成。文字核心與固定英文合成 WAV→CPU STT→HTTP fixture→SQLite→Edge 文字框已接通，11 unit／5 integration／1 runtime smoke 通過。麥克風熱鍵、繁中品質、原生跨 App 送字、公司 API 與完整 portable 產品仍待完成。詳見 [preflight](docs/verification/windows11-preflight.md)。使用者已授權安裝開發所需工具，產品交付仍維持 npm portable。以下研究 checkpoint 為接手前背景。

- 研究、選型比較、15 項產品決策、兩份 ADR、測試目錄規範與 GitHub issues 已保存。
- 研究定稿 commit：[bdbcb13](https://github.com/swchen44/notype/commit/bdbcb13bdb2a3f529236b0e690344522351c5e54)。本交接文件屬於其後續更新，請以 clone 後的 `HEAD` 與 live issues 核對最新狀態。
- 尚無產品程式碼、可執行產品測試、Windows runtime／效能結果或已發布 npm 產品。`tests/` 目前只有規範，不能回報測試通過。
- [#1 設計收斂](https://github.com/swchen44/notype/issues/1) 已結案；[#2～#7](TODO.md) 是開放待辦，尚未全部拆成可直接執行的規格切片。

## 先讀哪些檔案

依序讀 [AGENTS.md](AGENTS.md)、[GLOSSARY.md](GLOSSARY.md)、[npm 配送 ADR](docs/adr/0001-portable-npm-distribution.md)、[文字歷史 ADR](docs/adr/0002-text-history-independent-of-audio.md)、[MVP 定稿](docs/research/10-confirmed-windows-mvp.md) 與 [TODO.md](TODO.md)。

再按工作範圍查 [決策來源](docs/research/08-decision-log.md)、[Windows 技術接手](docs/research/07-windows-handoff.md)、[Handy source／payload audit](docs/research/06-source-feasibility-audit.md)、[history source audit](docs/research/09-history-source-audit.md)、[測試規範](tests/README.md) 與 live GitHub issues。原始研究入口為 [docs/research/README.md](docs/research/README.md)。

## 下一個 session 的工作

1. **核對目前狀態。** 讀此 checkpoint、TODO、live #8／#12～#16 與 receipts，核對 remote／branch／HEAD／未提交變更。工具及原生 Release 已建置完成，依現有證據接續，不重做最初選型或要求 key。
2. **接續實際試用回饋。** 本機三包安裝、兩模式 CPU、固定 runtime、歷史及限定 facade 更新已有實際證據，依使用者麥克風／實體快捷鍵回饋做必要的針對性驗證，不重跑整套選型／模型矩陣。保留失敗證據；完整 native binary／schema migration／中斷／鎖檔更新仍屬 #14 後續。
3. **保留使用者資料。** 已安裝於 `%LOCALAPPDATA%/notype/local-app`，使用者試用資料為 `%LOCALAPPDATA%/notype/local-trial`，都在建置快取外。使用者資料不能列入 QA/cache cleanup；使用者可能正在操作 App，不可再啟動自有 GUI 測試或終止其程序。正常使用者從 Explorer／Win＋R 開啟 Starter；文字服務未設定時會保存原稿供複製。
4. **按最新範圍接續驗收。** 先交付本機錄音／辨識／歷史／複製，再處理人聲與實體快捷鍵回饋。其他 App 自動輸入與公司 API／Proxy 延後；有新產品歧義才詢問，不重新訪談已確認選項。各項成果連回 issue／commit／receipt。

使用者已確認目前不在公司網路，先完成本機版本。公司 API／Proxy 維持未驗收，現在不需要 key，也不另選公開 provider。

上一輪八個已合入 worktrees 已清理，branch commits 與外部證據保留；第二輪另建 `model-interface`、`input-tray`、`diagnostics` 與待實作的 `recording-elapsed` worktrees。後續從最新整合分支接續並遵守新計畫 gate。唯一 native target 為 `D:/git/notype-build`，舊 debug／core target 已退役，不能重建第二份來迴避 cache 使用權。獨立 C++ prefix 是 `%LOCALAPPDATA%/notype-study/build-cache/native-prebuilt/transcribe-cpp-0.2.4-win64-cpu-release-E4b0TR`；舊 tcs 是指向已刪 debug 的 junction，不能當作獨立備份。清理只限確認 idle、驗證絕對範圍且不跟隨 reparse point 的 task-owned derived files；不得清除使用者模型、歷史、來源 registry、系統或 pagefile。完整本機包與失敗／成功細節見 [套件驗證](docs/verification/portable-local-assembly.md)。

## 接手時特別保留的邊界

產品交付只用 npm 取得完整預編譯 portable payload，禁止 setup／MSI／runtime bootstrapper。公司只有文字 LLM API，因此 STT 必須在本機。持久文字歷史及模型應放在 npm 版本化程式目錄之外；不可假設 Handy 已有 `--data-dir` 等未經 source 核對的參數。

Handy 的錯誤 fallback、WAV gate／cleanup 與 installer updater 已按本產品修改，原始差異見兩份 audit。現有 Qwen／固定 WebView／Release 證據與公司 API／認證／CA、完整效能、publisher／Proxy 等未驗收項目分開記錄。短錄音完整 session p95≤5 秒仍是目標。

公司 endpoint、憑證、工作錄音及逐字稿留本機，不提交 GitHub、issues 或通用 npm 包。固定來源、hash、靜態檢查、build、Windows 啟動、五個 App、公司 API 與效能各自記錄；測試替身不代表公司驗收通過。

## 建議使用的 skills

Windows 已有 Codex 與使用者所稱的 `grillme` skill。先查當地實際 skill 名稱；只對研究後仍需使用者決定、且會改變既有方案的新問題使用 `grill-me`／`grilling`。已確認的 Q1～Q15 直接沿用，技術事實由 agent 查核。

若有 `to-spec`／`to-tickets`，可用來補規格與相依順序；沒有時直接在 repo／issues 記錄等效的範圍、驗收、相依與測試方式，繼續工作。不要因 Mac 曾有某個 skill 就假設 Windows 也已安裝。重大新決策更新 decision log／ADR；一般實作與可逆驗證直接執行。

## 直接貼給 Windows Codex

```text
請接手並繼續開發 private repo https://github.com/swchen44/notype。
若目前不是這個 repo，先取得或定位工作副本，保留現有未提交變更。

先讀 HANDOFF.md 的目前 checkpoint、AGENTS.md、TODO.md、GLOSSARY.md、docs/adr/，
再查 live #8、#12～#16 及對應 receipts。本期 Windows amd64，架構預留 macOS／Linux。
產品選項已確認；有新歧義才問我，不能自行猜測，也不要重問已回答的問題。

核對 repo 與現有產物後，接續本機試用回饋，必要時做針對性修正；
三包實際安裝／啟動／兩模式推論與限定 facade 更新已有證據，不重做已完成工作。
這輪先交付錄音→繁／簡中英本機 STT→本機歷史→複製；其他 App 自動輸入下一階段再補。
我目前不在公司網路，公司 API／Proxy／真實文字品質延後，真正需要 key 時再通知我。
沿用已固定的 Handy source、Qwen pilot、Release 與完整私有 tarball，不重做選型或安裝工具。
只用 npm 配送完整 portable payload，不使用 installer，不把 fixture 或 source checks 當作實際啟動。
依 tests/README.md 在 tests/unittest/ 與 tests/integration/ 加入並執行實際測試。
遵循已定稿的失敗待選、焦點保護、手動 Enter 與錄音清理規則。
持續更新 issue、TODO、commit 和驗證紀錄；公司設定、憑證與工作內容留在本機。
只有 Codex 和 grillme 也要能接續，不要因缺少其他規劃 skills 停工。
```
