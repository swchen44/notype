# 本機里程碑：兩軸程式審查

日期：2026-10-25。固定比較起點 `be6f0a596a9bac16ae6e9d5d35d4ac991c3fd369`，審查整合點 `37f5bdae8cbd8974ae99fb837908716ef9bbad24`。兩個獨立 reviewer 同時檢查 Standards 與 Spec，未互讀對方結果，也未執行 App、Cargo 或修改程式。這次審查涵蓋使用者已確認的本機錄音／辨識／歷史／複製里程碑；公司 API／Proxy、其他 App 自動輸入、公開發布、人聲品質與完整 p95 明確留待後續。

以下保留原審查結果；當時修正尚未完成。最終修正與獨立複審結果追加於文末。完整套件／離線啟動／升級資料保留結果另列 artifact receipt，不從 source review 推定通過。

## Standards

Reviewed `be6f0a5...37f5bda` read-only. Unchanged imported Handy code, formatting/lint enforcement and deferred acceptance are excluded.

1. **Documented rule: missing copy-failure feedback.** `desktop/src/components/settings/history/HistorySettings.tsx:77–84` awaits `commands.copyHistoryText(...)` without `try/catch`. `desktop/src/bindings.ts:953–958` explicitly rethrows JavaScript `Error` instances, so an IPC/bridge rejection produces an unhandled promise and no failure message. `desktop/CRUSH.md:57` requires frontend “Try/catch with user feedback.” Catch rejected calls and show the existing translated `copyFailed` message, as the adjacent action wrapper does.

2. **Documented rule: undocumented new public lifecycle interfaces.** `desktop/src-tauri/src/workflow.rs:82–139,180–239,287–351` exposes `begin`, `begin_with_profile`, `capture_processing_snapshot`, `finish`, `retry`, and `capture_source_context` without API documentation. Similar newly exported configuration APIs in `faithful_provider.rs:29–120,210` lack documentation. `desktop/AGENTS.md:6` requires documenting public interfaces; `desktop/CONTRIBUTING.md:248` specifically requires public API doc comments. Document generation ownership, required capture order, snapshot versus retry settings, cancellation and delivery effects; these non-obvious contracts currently require reading implementation/tests.

3. **Documented rule: modified product regression tests remain outside root test directories.** `desktop/src-tauri/src/tray_i18n.rs:63–67` changes expected `zh-CN`/`zh-TW` fallback behavior inside the source-local test module; `tray.rs:672–696` rewrites its copy-test fixture for the new session model. Root `AGENTS.md` and `desktop/AGENTS.md:5` place product unit tests in `tests/unittest/` (integration tests in `tests/integration/`). Move the affected regression coverage to that approved interface/harness; unrelated unchanged upstream tests can remain.

4. **Judgment: possible Duplicated Code.** Shortcut defaults appear in `ScenarioShortcuts.tsx:8–13` (`translation: "alt_right+shift_right"`), `settings.rs:896–961` (the same chord), and `physical_gestures.rs:31–38` (`translation: [0xa5, 0xa1].into()`). Future changes can make displayed fallback, Reset and router defaults disagree. Derive UI/router defaults from the authoritative settings representation or a shared declarative fixture.

Four findings: three documented-rule findings and one heuristic judgment. No build, test, app, cache or repository mutation was performed.

## Spec

1. **P1 — A cancelled decode can cancel the next recording.** Spec #8 requires 「取消後忽略遲到結果」 and the LOCAL recording → history → copy workflow ([spec](https://github.com/swchen44/notype/issues/8)). In [actions.rs:315](https://github.com/swchen44/notype/blob/37f5bdae8cbd8974ae99fb837908716ef9bbad24/desktop/src-tauri/src/actions.rs#L315), the old `spawn_blocking` STT task reacts to cancellation by calling generation-independent `workflow.cancel_active()`. Esc already returns shortcuts/audio to idle, allowing recording B while decode A continues. When A finishes, it removes B’s workflow; stopping B then finds no active generation (line 267), leaving its audio unsaved and the microphone running until cancelled. Scope cleanup to A’s captured generation, including its finish guard.

2. **P2 — Native direct launch permits persistent data inside sibling npm packages.** Spec #8 requires 「歷史、模型與公司設定位於 npm 版本化程式目錄之外」 ([spec](https://github.com/swchen44/notype/issues/8)). [notype-platform/src/lib.rs:293](https://github.com/swchen44/notype/blob/37f5bdae8cbd8974ae99fb837908716ef9bbad24/crates/notype-platform/src/lib.rs#L293) checks only the application package root. Directly launching the packaged executable with `NOTYPE_DATA_DIR` set to sibling `node_modules/notype` or `node_modules/notype-model` passes this check, then stores settings/history there. Replacing that dependency on npm update can discard the data. Apply the same complete package-boundary policy as the Node launcher.

3. **P2 — Local script conversion changes valid path literals.** Spec #8 says 「保護 code、網址、路徑、email、小數及版本號等字面值」 ([spec](https://github.com/swchen44/notype/issues/8)). The protected-span expression in [recognition.rs:60](https://github.com/swchen44/notype/blob/37f5bdae8cbd8974ae99fb837908716ef9bbad24/desktop/src-tauri/src/recognition.rs#L60) recognizes drive-prefixed paths only. Unquoted UNC `\\server\头发\log.txt` or POSIX `/tmp/头发/log.txt` therefore reaches OpenCC and changes `头发` to `頭髮` in Traditional mode, corrupting the exact path saved to history and copied. Protect these path forms before normalization.

Reviewed pinned head `37f5bda` against `be6f0a5`; static source review only. No apps, builds, tests or native caches were changed. Artifact/cold-launch/upgrade proof remains in progress; company API, five-app insertion, human quality, p95 and public release remain explicitly deferred. No additional scope creep identified.

Standards：4 項（3 項文件規則、1 項啟發式判斷），主要使用者影響為複製失敗未顯示提示。Spec：3 項，最嚴重為已取消的舊辨識任務可能取消新錄音。兩軸結果分開回報；後續修正／重新檢查會補上實際 commit 與測試。

## 修正與最終獨立複審

同一個 `ticket/local-review-fixes` 分支完成修正：`06c7e04` 處理原始七項 findings，`dd40397c4b44d6ccda5b16cc23ce50457b0cc373` 處理後續複審發現的兩項行為問題與兩項 Standards 殘留。分支已包含整合點 `fca6a65d206166a6db446840b6c102d21e2e1043`；以 no-ff 合入 `8c5fb86b81867c889fe6571f09fe39e401b1568b`。逐項對照、RED 證據與命令見 [修正紀錄](local-review-fixes.md)。

原始取消競態改以 generation owner 隔離音訊、辨識、UI 與清理；native 資料位置政策涵蓋 sibling npm roles；字形轉換保護 UNC／POSIX／相對路徑。複製失敗顯示既有翻譯提示、公開生命週期介面補上文件、產品回歸移至根目錄測試邊界，快捷鍵預設值由共同定義提供。

對 `e3e27468883dbc55236145e9daf0210b4c120e75` 的後續檢查另發現：slash prose 被過度視為路徑，以及開始音效持有 ownership lock 而阻塞取消；Standards 則仍缺 coordinator 通知文件，且 tray 私有 helper 有測試耦合。最終修正讓普通 slash prose 正常轉換、保留真正路徑；播放音效移至 lock 外，之後再檢查 owner／readiness 才能靜音；補齊通知與 readiness 介面文件，移除私有 helper 測試，以公開 session 選擇／恢復測試覆蓋行為。原始與後續失敗證據均保留，沒有改寫為一次全綠。

兩位獨立 reviewer 在乾淨的固定 HEAD `dd40397c4b44d6ccda5b16cc23ce50457b0cc373` 完成最後限定範圍的唯讀複審，未執行測試、App 或 Cargo：

| 審查軸 | 最終結果 |
| --- | --- |
| Standards | 兩項後續殘留已解決；此前原始修正結論維持。**0 項剩餘文件規則 findings、0 項可執行的啟發式 findings** |
| Spec | slash prose 與播放阻塞取消兩項已解決；此前原始修正結論維持。**0 項剩餘 findings／blockers（限本次複審範圍）** |

獨立報告保存在 repo 外的 `agent-notes/review-standards-final.md` 與 `agent-notes/review-spec-final.md`。這是原始及後續 findings 的結案判斷，不擴張為未執行的 runtime／artifact 驗收。

實際執行者測試結果分開記錄：

| 驗證 | 結果與適用範圍 |
| --- | --- |
| 最終四個 desktop Release 契約 targets | **33 passed，0 failed／ignored**：14 workflow、12 provider、6 gestures、1 native token；包含 slash prose 及播放期間 stop／Esc→B 回歸 |
| 最終公開 core-session target | **9 passed，0 failed／ignored**；涵蓋結果交付、明確選擇原稿與缺少結果後的恢復，未操作 clipboard |
| 前段 native platform target | **4 passed**；沒有在只涉及後續修正時重跑 |
| 前段 native Release tray filter | **7 passed，250 filtered** 為歷史結果；移除兩個私有 helper 測試後沒有重跑該 filter，現在剩五項，不宣稱最新七項通過 |
| 前端與格式 | 初次修正的 ESLint、TypeScript／Vite（2031 modules）、變更檔案 Prettier 通過；後續沒有前端變更。最終變更 Rustfmt／diff check 通過 |

Merger 僅驗證固定 SHA、乾淨狀態、祖先關係與合併 diff check；未重跑上述測試、原生建置、C++ 或 App，避免與 #14 的唯一建置 target 衝突。最終 App／helper relink、完整套件、離線冷啟動、模型與歷史升級保留仍由 artifact 驗證接續。公司 API／Proxy、其他 App 自動輸入、公開發布、人聲品質與完整 p95 仍屬後續範圍；此次沒有關閉 GitHub Issues 或建立 PR。
