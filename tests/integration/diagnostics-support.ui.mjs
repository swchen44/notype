import assert from "node:assert/strict";
import test from "node:test";
import { uiFixture } from "./fixtures/ui.mjs";

async function fixture(t) {
  const page = await uiFixture(t);
  const unhandled = [];
  page.on("pageerror", (error) => unhandled.push(error.message));
  t.after(() => assert.deepEqual(unhandled, []));
  await page.evaluate(async () => {
    window.calls = [];
    window.preview = {report_id: "0123456789abcdef-00000001", text: "notype diagnostic report v1\nversion=0.1.0\nsafe snapshot one"};
    window.mode = "ok";
    window.__TAURI_INTERNALS__ = {invoke: async (cmd, args) => {
      window.calls.push({cmd, args});
      const privateFailure = "Bearer synthetic-secret https://company.invalid C:\\Users\\private-user original speech 原稿";
      if (cmd === "get_app_settings") return {app_language: "en", log_level: "debug"};
      if (cmd === "get_diagnostic_report") {
        if (window.mode === "preview-error") throw privateFailure;
        return {...window.preview};
      }
      if (cmd === "export_diagnostic_report") {
        if (window.mode === "cancel") return false;
        if (window.mode === "expired") throw "diagnostic-preview-expired";
        if (window.mode === "extension") throw "diagnostic-extension-invalid";
        if (window.mode === "export-error") throw privateFailure;
        return true;
      }
      if (cmd === "plugin:opener|open_url" && window.mode === "opener-error") throw privateFailure;
      return null;
    }};
    const React = (await import("/node_modules/.vite/deps/react.js")).default;
    const {createRoot} = (await import("/node_modules/.vite/deps/react-dom_client.js")).default;
    const {default: i18n} = await import("/src/i18n/index.ts");
    await i18n.changeLanguage("en");
    await import("/src/App.css");
    const {useSettingsStore} = await import("/src/stores/settingsStore.ts");
    useSettingsStore.setState({settings: {log_level: "debug"}, isLoading: false});
    const {DiagnosticsSupport} = await import("/src/components/settings/about/DiagnosticsSupport.tsx");
    createRoot(document.getElementById("root")).render(React.createElement(DiagnosticsSupport));
  });
  await page.getByRole("button", {name: "Preview diagnostics", exact: true}).waitFor({timeout: 3000});
  return page;
}

test("support previews exact text and export cancellation sends only its snapshot ID", async (t) => {
  const page = await fixture(t);
  const exportButton = page.getByRole("button", {name: "Export reviewed .txt", exact: true});
  assert.equal(await exportButton.isDisabled(), true);
  assert.equal(await page.getByRole("button", {name: "Open GitHub issue", exact: true}).isDisabled(), true);
  await page.getByRole("button", {name: "Preview diagnostics", exact: true}).click();
  const preview = page.getByRole("textbox", {name: "Report preview (exact exported content)"});
  assert.equal(await preview.inputValue(), "notype diagnostic report v1\nversion=0.1.0\nsafe snapshot one");
  assert.equal(await preview.getAttribute("readonly"), "");
  await page.evaluate(() => {window.mode = "cancel";});
  await exportButton.click();
  await page.getByText("Export cancelled. No report was saved.", {exact: true}).waitFor({timeout: 3000});
  assert.deepEqual(await page.evaluate(() => window.calls.filter(call => call.cmd === "export_diagnostic_report")), [{cmd: "export_diagnostic_report", args: {reportId: "0123456789abcdef-00000001"}}]);
  assert.equal(await preview.inputValue(), "notype diagnostic report v1\nversion=0.1.0\nsafe snapshot one");
});

test("expired preview and save failures remain readable; refreshing exports the new snapshot", async (t) => {
  const page = await fixture(t);
  const previewButton = page.getByRole("button", {name: "Preview diagnostics", exact: true});
  const exportButton = page.getByRole("button", {name: "Export reviewed .txt", exact: true});
  await previewButton.click();
  for (const [mode, message] of [
    ["expired", "This preview has expired. Preview diagnostics again before exporting."],
    ["export-error", "Unable to save the report. Choose a writable location and try again."],
    ["extension", "Choose a file name ending in .txt and try again."],
  ]) {
    await page.evaluate((mode) => {window.mode = mode;}, mode);
    await exportButton.click();
    await page.getByText(message, {exact: true}).waitFor({timeout: 3000});
    assert.doesNotMatch(await page.locator("body").innerText(), /synthetic-secret|company\.invalid|private-user|original speech|原稿/);
  }
  await page.evaluate(() => {window.mode = "ok"; window.preview = {report_id: "0123456789abcdef-00000002", text: "version=0.1.0\nsafe snapshot two"};});
  await previewButton.click();
  assert.equal(await page.getByRole("textbox").inputValue(), "version=0.1.0\nsafe snapshot two");
  await exportButton.click();
  await page.getByText("Report saved locally. Review it before attaching it to GitHub.", {exact: true}).waitFor({timeout: 3000});
  assert.deepEqual(await page.evaluate(() => window.calls.filter(call => call.cmd === "export_diagnostic_report").at(-1).args), {reportId: "0123456789abcdef-00000002"});
});

test("preview and opener rejection are translated with no raw error; issue retry uses safe summary", async (t) => {
  const page = await fixture(t);
  await page.evaluate(() => {window.mode = "preview-error";});
  await page.getByRole("button", {name: "Preview diagnostics", exact: true}).click();
  await page.getByText("Unable to prepare diagnostics. Please try again.", {exact: true}).waitFor({timeout: 3000});
  assert.equal(await page.getByRole("button", {name: "Export reviewed .txt", exact: true}).isDisabled(), true);
  await page.evaluate(() => {window.mode = "ok";});
  await page.getByRole("button", {name: "Preview diagnostics", exact: true}).click();
  await page.evaluate(() => {window.mode = "opener-error";});
  await page.getByRole("button", {name: "Open GitHub issue", exact: true}).click();
  await page.getByText("Unable to open GitHub. Open github.com/swchen44/notype/issues manually and attach the reviewed report.", {exact: true}).waitFor({timeout: 3000});
  await page.evaluate(() => {window.mode = "ok";});
  await page.getByRole("button", {name: "Open GitHub issue", exact: true}).click();
  const opened = await page.evaluate(() => window.calls.filter(call => call.cmd === "plugin:opener|open_url").at(-1));
  const url = new URL(opened.args.url);
  assert.equal(url.origin + url.pathname, "https://github.com/swchen44/notype/issues/new");
  assert.match(url.searchParams.get("body"), /0123456789abcdef-00000001/);
  assert.ok(!decodeURIComponent(url.href).includes("safe snapshot one"));
  assert.doesNotMatch(await page.locator("body").innerText(), /synthetic-secret|company\.invalid|private-user|original speech|原稿/);
});
