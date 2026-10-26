import test from "node:test";
import assert from "node:assert/strict";
import { uiFixture } from "./fixtures/ui.mjs";

test("setup recovery offers explicit retry without automatic API calls and waits for durable acknowledgment", async (t) => {
  const page = await uiFixture(t, {
    cacheName: "bootstrap-recovery",
    entry: `export {default as React} from 'react'; export {createRoot} from 'react-dom/client'; export {BootstrapRecovery,useBootstrapRecovery} from '/src/components/BootstrapRecovery.tsx'; export {default as i18n} from '/src/i18n/index.ts'; import '/src/App.css';`,
  });
  await page.evaluate(async () => {
    window.calls = [];
    window.__TAURI_INTERNALS__ = {
      invoke: async (cmd) => {
        window.calls.push(cmd);
        if (cmd === "get_app_settings") return { ui_language: "en" };
        if (cmd === "get_bootstrap_status")
          return {
            state: "incomplete",
            code: "bootstrap-settings-invalid",
            can_retry: true,
          };
        if (cmd === "retry_offline_bootstrap")
          return await new Promise((resolve) => {
            window.finishBootstrap = resolve;
          });
        throw new Error("Unexpected operation");
      },
    };
    const { React, createRoot, BootstrapRecovery, useBootstrapRecovery, i18n } =
      await import("/__ui_fixture.tsx");
    await i18n.changeLanguage("en");
    function Fixture() {
      return React.createElement(BootstrapRecovery, useBootstrapRecovery());
    }
    createRoot(document.getElementById("root")).render(
      React.createElement(Fixture),
    );
  });
  await page.getByText("Setup incomplete", { exact: true }).waitFor();
  assert.deepEqual(
    await page.evaluate(() =>
      window.calls.filter((cmd) => cmd !== "get_app_settings"),
    ),
    ["get_bootstrap_status"],
  );
  await page.getByRole("button", { name: "Retry setup", exact: true }).click();
  assert.equal(
    await page
      .getByRole("button", { name: "Retrying…", exact: true })
      .isDisabled(),
    true,
  );
  await page.evaluate(() =>
    window.finishBootstrap({ state: "ready", code: null, can_retry: false }),
  );
  await page
    .getByText("Setup incomplete", { exact: true })
    .waitFor({ state: "detached" });
  assert.deepEqual(
    await page.evaluate(() =>
      window.calls.filter((cmd) => cmd !== "get_app_settings"),
    ),
    ["get_bootstrap_status", "retry_offline_bootstrap"],
  );
});
