import assert from "node:assert/strict";
import test from "node:test";
import { uiFixture } from "./fixtures/ui.mjs";

async function fixture(t, holdInitial = false) {
  const page = await uiFixture(t);
  const unhandled = [];
  page.on("pageerror", (error) => unhandled.push(error.message));
  t.after(() => assert.deepEqual(unhandled, []));
  await page.evaluate(async (holdInitial) => {
    window.calls = [];
    window.preview = {
      report_id: "0123456789abcdef-00000001",
      text: "notype diagnostic report v1\nversion=0.1.0\nsafe snapshot one",
      epoch: 0,
    };
    window.collection = {
      initialized: true,
      effective_enabled: true,
      saved_preference: true,
      desired_enabled: null,
      persistence: "default",
      revision: 0,
      epoch: 0,
    };
    window.holdStatus = holdInitial;
    window.mode = "ok";
    window.__TAURI_INTERNALS__ = {
      invoke: async (cmd, args) => {
        window.calls.push({ cmd, args });
        const privateFailure =
          "Bearer synthetic-secret https://company.invalid C:\\Users\\private-user original speech 原稿";
        if (cmd === "get_app_settings")
          return { app_language: "en", log_level: "debug" };
        if (cmd === "get_diagnostic_status") {
          if (window.holdStatus) {
            window.holdStatus = false;
            return new Promise((resolve) => {
              window.releaseStatus = resolve;
            });
          }
          return { ...window.collection };
        }
        if (cmd === "set_diagnostics_enabled") {
          window.collection = {
            ...window.collection,
            effective_enabled: args.enabled,
            saved_preference: args.enabled,
            persistence: "saved",
            desired_enabled: null,
            revision: window.collection.revision + 1,
          };
          if (window.mode === "preference-error")
            window.collection = {
              ...window.collection,
              effective_enabled: false,
              saved_preference: true,
              persistence: "unsaved",
              desired_enabled: args.enabled,
            };
          return { ...window.collection };
        }
        if (cmd === "clear_diagnostics") return ++window.collection.epoch;
        if (cmd === "get_diagnostic_report") {
          if (window.mode === "preview-error") throw privateFailure;
          return { ...window.preview };
        }
        if (cmd === "export_diagnostic_report") {
          if (window.mode === "cancel")
            return {
              status: "cancelled",
              epoch: window.preview.epoch,
              report_id: window.preview.report_id,
            };
          if (window.mode === "expired") throw "diagnostic-preview-expired";
          if (window.mode === "extension") throw "diagnostic-extension-invalid";
          if (window.mode === "export-error") throw privateFailure;
          return {
            status: "saved",
            epoch: window.preview.epoch,
            report_id: window.preview.report_id,
          };
        }
        if (cmd === "plugin:opener|open_url" && window.mode === "opener-error")
          throw privateFailure;
        return null;
      },
    };
    const React = (await import("/node_modules/.vite/deps/react.js")).default;
    const { createRoot } = (
      await import("/node_modules/.vite/deps/react-dom_client.js")
    ).default;
    const { default: i18n } = await import("/src/i18n/index.ts");
    await i18n.changeLanguage("en");
    await import("/src/App.css");
    const { useSettingsStore } = await import("/src/stores/settingsStore.ts");
    useSettingsStore.setState({
      settings: { log_level: "debug" },
      isLoading: false,
    });
    const { DiagnosticsSupport } = await import(
      "/src/components/settings/about/DiagnosticsSupport.tsx"
    );
    createRoot(document.getElementById("root")).render(
      React.createElement(DiagnosticsSupport),
    );
  }, holdInitial);
  await page
    .getByRole("button", { name: "Preview diagnostics", exact: true })
    .waitFor({ timeout: 3000 });
  return page;
}

test("support previews exact text and export cancellation sends only its snapshot ID", async (t) => {
  const page = await fixture(t);
  const exportButton = page.getByRole("button", {
    name: "Export reviewed .txt",
    exact: true,
  });
  assert.equal(await exportButton.isDisabled(), true);
  assert.equal(
    await page
      .getByRole("button", { name: "Open GitHub issue", exact: true })
      .isDisabled(),
    true,
  );
  await page
    .getByRole("button", { name: "Preview diagnostics", exact: true })
    .click();
  const preview = page.getByRole("textbox", {
    name: "Report preview (exact exported content)",
  });
  assert.equal(
    await preview.inputValue(),
    "notype diagnostic report v1\nversion=0.1.0\nsafe snapshot one",
  );
  assert.equal(await preview.getAttribute("readonly"), "");
  await page.evaluate(() => {
    window.mode = "cancel";
  });
  await exportButton.click();
  await page
    .getByText("Export cancelled. No report was saved.", { exact: true })
    .waitFor({ timeout: 3000 });
  assert.deepEqual(
    await page.evaluate(() =>
      window.calls.filter((call) => call.cmd === "export_diagnostic_report"),
    ),
    [
      {
        cmd: "export_diagnostic_report",
        args: { reportId: "0123456789abcdef-00000001" },
      },
    ],
  );
  assert.equal(
    await preview.inputValue(),
    "notype diagnostic report v1\nversion=0.1.0\nsafe snapshot one",
  );
});

test("collection starts unknown and disabled; OFF persistence failure stays visibly OFF unsaved", async (t) => {
  const page = await fixture(t, true);
  const toggle = page.getByRole("checkbox", {
    name: "Collect structured diagnostics",
    exact: true,
  });
  await toggle.waitFor({ timeout: 3000 });
  assert.equal(await toggle.isDisabled(), true);
  assert.equal(
    await page
      .getByRole("button", { name: "Preview diagnostics", exact: true })
      .isDisabled(),
    true,
  );
  await page.evaluate(() => window.releaseStatus({ ...window.collection }));
  await toggle.waitFor({ state: "visible" });
  await page.waitForFunction(
    () => !document.querySelector('input[type="checkbox"]').disabled,
  );
  assert.equal(await toggle.isChecked(), true);
  await page.evaluate(() => {
    window.mode = "preference-error";
  });
  await toggle.focus();
  await toggle.press("Space");
  await page
    .getByText(
      "Collection is OFF but the preference was not saved. Restart may use the previous preference. Retry saving.",
      { exact: true },
    )
    .waitFor({ timeout: 3000 });
  assert.equal(await toggle.isChecked(), false);
  assert.deepEqual(
    await page.evaluate(
      () =>
        window.calls
          .filter((call) => call.cmd === "set_diagnostics_enabled")
          .at(-1).args,
    ),
    { enabled: false },
  );
  await page.evaluate(() => {
    window.mode = "ok";
  });
  await page
    .getByRole("button", {
      name: "Retry saving collection preference",
      exact: true,
    })
    .click();
  await page
    .getByText(
      "Structured collection is OFF. Existing records can still be reviewed, exported or cleared.",
      { exact: true },
    )
    .waitFor({ timeout: 3000 });
  assert.equal(await toggle.isChecked(), false);
  assert.deepEqual(
    await page.evaluate(
      () =>
        window.calls
          .filter((call) => call.cmd === "set_diagnostics_enabled")
          .at(-1).args,
    ),
    { enabled: false },
  );
});

test("expired preview and save failures remain readable; refreshing exports the new snapshot", async (t) => {
  const page = await fixture(t);
  const previewButton = page.getByRole("button", {
    name: "Preview diagnostics",
    exact: true,
  });
  const exportButton = page.getByRole("button", {
    name: "Export reviewed .txt",
    exact: true,
  });
  await previewButton.click();
  for (const [mode, message] of [
    [
      "expired",
      "This preview has expired. Preview diagnostics again before exporting.",
    ],
    [
      "export-error",
      "Unable to save the report. Choose a writable location and try again.",
    ],
    ["extension", "Choose a file name ending in .txt and try again."],
  ]) {
    await page.evaluate((mode) => {
      window.mode = mode;
    }, mode);
    await exportButton.click();
    await page.getByText(message, { exact: true }).waitFor({ timeout: 3000 });
    assert.doesNotMatch(
      await page.locator("body").innerText(),
      /synthetic-secret|company\.invalid|private-user|original speech|原稿/,
    );
  }
  await page.evaluate(() => {
    window.mode = "ok";
    window.preview = {
      report_id: "0123456789abcdef-00000002",
      text: "version=0.1.0\nsafe snapshot two",
      epoch: 0,
    };
  });
  await previewButton.click();
  await page.waitForFunction(
    () =>
      document.querySelector("textarea")?.value ===
      "version=0.1.0\nsafe snapshot two",
  );
  assert.equal(
    await page.getByRole("textbox").inputValue(),
    "version=0.1.0\nsafe snapshot two",
  );
  await exportButton.click();
  await page
    .getByText(
      "Report saved locally. Review it before attaching it to GitHub.",
      { exact: true },
    )
    .waitFor({ timeout: 3000 });
  assert.deepEqual(
    await page.evaluate(
      () =>
        window.calls
          .filter((call) => call.cmd === "export_diagnostic_report")
          .at(-1).args,
    ),
    { reportId: "0123456789abcdef-00000002" },
  );
});

test("Clear discards late preview responses and keeps the collection preference", async (t) => {
  const page = await fixture(t);
  await page.evaluate(() => {
    const original = window.__TAURI_INTERNALS__.invoke;
    window.__TAURI_INTERNALS__.invoke = (cmd, args) =>
      cmd === "get_diagnostic_report"
        ? new Promise((resolve) => {
            window.releasePreview = resolve;
          })
        : original(cmd, args);
  });
  await page
    .getByRole("button", { name: "Preview diagnostics", exact: true })
    .click();
  await page.waitForFunction(() => typeof window.releasePreview === "function");
  await page
    .getByRole("button", { name: "Clear structured diagnostics", exact: true })
    .click();
  const cleared =
    "Structured diagnostics and the app preview were cleared. Preferences and manual exports were kept.";
  await page.getByText(cleared, { exact: true }).waitFor({ timeout: 3000 });
  await page.evaluate(() => window.releasePreview({ ...window.preview }));
  await page.evaluate(
    () =>
      new Promise((resolve) =>
        requestAnimationFrame(() => requestAnimationFrame(resolve)),
      ),
  );
  assert.equal(await page.getByRole("textbox").count(), 0);
  assert.equal(
    await page
      .getByRole("button", { name: "Export reviewed .txt", exact: true })
      .isDisabled(),
    true,
  );
  assert.equal(
    await page
      .getByRole("checkbox", {
        name: "Collect structured diagnostics",
        exact: true,
      })
      .isChecked(),
    true,
  );
  assert.equal(await page.getByText(cleared, { exact: true }).count(), 1);
  assert.equal(
    await page.evaluate(
      () =>
        window.calls.filter((call) => call.cmd === "set_diagnostics_enabled")
          .length,
    ),
    0,
  );
});

test("late export success cannot replace Clear feedback or restore the old report", async (t) => {
  const page = await fixture(t);
  await page
    .getByRole("button", { name: "Preview diagnostics", exact: true })
    .click();
  await page.getByRole("textbox").waitFor();
  await page.evaluate(() => {
    const original = window.__TAURI_INTERNALS__.invoke;
    window.__TAURI_INTERNALS__.invoke = (cmd, args) =>
      cmd === "export_diagnostic_report"
        ? new Promise((resolve) => {
            window.releaseExport = resolve;
          })
        : original(cmd, args);
  });
  await page
    .getByRole("button", { name: "Export reviewed .txt", exact: true })
    .click();
  await page.waitForFunction(() => typeof window.releaseExport === "function");
  await page
    .getByRole("button", { name: "Clear structured diagnostics", exact: true })
    .click();
  const cleared =
    "Structured diagnostics and the app preview were cleared. Preferences and manual exports were kept.";
  await page.getByText(cleared, { exact: true }).waitFor({ timeout: 3000 });
  await page.evaluate(() =>
    window.releaseExport({
      status: "saved",
      report_id: window.preview.report_id,
      epoch: 0,
    }),
  );
  await page.evaluate(
    () =>
      new Promise((resolve) =>
        requestAnimationFrame(() => requestAnimationFrame(resolve)),
      ),
  );
  assert.equal(await page.getByText(cleared, { exact: true }).count(), 1);
  assert.equal(await page.getByRole("textbox").count(), 0);
});

test("a delayed old status response cannot undo a newer acknowledged toggle", async (t) => {
  const page = await fixture(t);
  const toggle = page.getByRole("checkbox", {
    name: "Collect structured diagnostics",
    exact: true,
  });
  await page.waitForFunction(
    () => !document.querySelector('input[type="checkbox"]').disabled,
  );
  await page.evaluate(() => {
    window.holdStatus = true;
    window.oldStatus = { ...window.collection };
  });
  await page
    .getByRole("button", { name: "Refresh collection status", exact: true })
    .click();
  await page.waitForFunction(() => typeof window.releaseStatus === "function");
  await toggle.uncheck();
  await page
    .getByText(
      "Structured collection is OFF. Existing records can still be reviewed, exported or cleared.",
      { exact: true },
    )
    .waitFor({ timeout: 3000 });
  await page.evaluate(() => window.releaseStatus(window.oldStatus));
  await page.evaluate(
    () =>
      new Promise((resolve) =>
        requestAnimationFrame(() => requestAnimationFrame(resolve)),
      ),
  );
  assert.equal(await toggle.isChecked(), false);
});

test("preview and opener rejection are translated with no raw error; issue retry uses safe summary", async (t) => {
  const page = await fixture(t);
  await page.evaluate(() => {
    window.mode = "preview-error";
  });
  await page
    .getByRole("button", { name: "Preview diagnostics", exact: true })
    .click();
  await page
    .getByText("Unable to prepare diagnostics. Please try again.", {
      exact: true,
    })
    .waitFor({ timeout: 3000 });
  assert.equal(
    await page
      .getByRole("button", { name: "Export reviewed .txt", exact: true })
      .isDisabled(),
    true,
  );
  await page.evaluate(() => {
    window.mode = "ok";
  });
  await page
    .getByRole("button", { name: "Preview diagnostics", exact: true })
    .click();
  await page.evaluate(() => {
    window.mode = "opener-error";
  });
  await page
    .getByRole("button", { name: "Open GitHub issue", exact: true })
    .click();
  await page
    .getByText(
      "Unable to open GitHub. Open github.com/swchen44/notype/issues manually and attach the reviewed report.",
      { exact: true },
    )
    .waitFor({ timeout: 3000 });
  await page.evaluate(() => {
    window.mode = "ok";
  });
  await page
    .getByRole("button", { name: "Open GitHub issue", exact: true })
    .click();
  const opened = await page.evaluate(() =>
    window.calls.filter((call) => call.cmd === "plugin:opener|open_url").at(-1),
  );
  const url = new URL(opened.args.url);
  assert.equal(
    url.origin + url.pathname,
    "https://github.com/swchen44/notype/issues/new",
  );
  assert.match(url.searchParams.get("body"), /0123456789abcdef-00000001/);
  assert.ok(!decodeURIComponent(url.href).includes("safe snapshot one"));
  assert.doesNotMatch(
    await page.locator("body").innerText(),
    /synthetic-secret|company\.invalid|private-user|original speech|原稿/,
  );
});
