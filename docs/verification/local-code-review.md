# 本機里程碑：兩軸程式審查

日期：2026-10-25。固定比較起點 `be6f0a596a9bac16ae6e9d5d35d4ac991c3fd369`，審查整合點 `37f5bdae8cbd8974ae99fb837908716ef9bbad24`。兩個獨立 reviewer 同時檢查 Standards 與 Spec，未互讀對方結果，也未執行 App、Cargo 或修改程式。這次審查涵蓋使用者已確認的本機錄音／辨識／歷史／複製里程碑；公司 API／Proxy、其他 App 自動輸入、公開發布、人聲品質與完整 p95 明確留待後續。

以下保留原審查結果。**修正正在獨立 worktree 進行，尚未列為全部解決。** 完整套件／離線啟動／升級資料保留結果另列 artifact receipt，不從 source review 推定通過。

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
