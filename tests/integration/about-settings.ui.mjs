import assert from "node:assert/strict";
import test from "node:test";
import { uiFixture } from "./fixtures/ui.mjs";

// Only public Tauri transport is replaced. Components, hooks, settings refresh,
// process plugin, translations and Dropdown all run their production code.
async function fixture(t, component = "AboutSettings", holdInitial = false) {
  const page = await uiFixture(t);
  const unhandled = [];
  page.on("pageerror", (error) => unhandled.push(error.message));
  t.after(() => assert.deepEqual(unhandled, []));
  await page.evaluate(
    async ({ component, holdInitial }) => {
      window.calls = [];
      window.mode = "ok";
      window.holdSettings = holdInitial;
      window.settingsWaiters = [];
      window.acknowledgedLevel = "debug";
      const settings = () => ({
        app_language: "en",
        theme: "system",
        log_level: window.acknowledgedLevel,
      });
      window.releaseSettings = () => {
        window.holdSettings = false;
        window.settingsWaiters
          .splice(0)
          .forEach((resolve) => resolve(settings()));
      };
      window.__TAURI_INTERNALS__ = {
        transformCallback: () => 1,
        invoke: async (cmd, args) => {
          window.calls.push({ cmd, args });
          const failure =
            "Bearer synthetic-secret https://company.invalid C:\\Users\\private-user";
          if (cmd === "plugin:process|exit") {
            if (window.mode === "quit-error") throw failure;
            return null;
          }
          if (cmd === "set_log_level") {
            if (window.mode === "level-error") throw failure;
            if (window.mode === "level-pending")
              await new Promise((resolve) => {
                window.releaseSave = resolve;
              });
            // The server acknowledgement can differ from the requested value.
            // This makes a missing real settings refresh observable in the UI.
            return null;
          }
          if (cmd === "get_app_settings") {
            if (window.holdSettings)
              return new Promise((resolve) =>
                window.settingsWaiters.push(resolve),
              );
            return settings();
          }
          if (cmd === "get_default_settings") return settings();
          if (cmd === "check_custom_sounds")
            return { start: false, stop: false };
          if (cmd === "is_update_checks_locked") return false;
          if (cmd === "plugin:event|listen") return 1;
          if (cmd === "plugin:app|version") return "0.1.0";
          if (cmd === "get_app_dir_path" || cmd === "get_log_dir_path")
            return "synthetic-fixture-directory";
          if (cmd === "get_diagnostic_status")
            return {
              initialized: true,
              effective_enabled: true,
              saved_preference: true,
              desired_enabled: null,
              persistence: "saved",
              revision: 0,
              epoch: 0,
            };
          throw new Error(`Unexpected fixture IPC: ${cmd}`);
        },
      };
      const React = (await import("/node_modules/.vite/deps/react.js")).default;
      const { createRoot } = (
        await import("/node_modules/.vite/deps/react-dom_client.js")
      ).default;
      const { default: i18n } = await import("/src/i18n/index.ts");
      await i18n.changeLanguage("en");
      await import("/src/App.css");
      const module =
        component === "AboutSettings"
          ? await import("/src/components/settings/about/AboutSettings.tsx")
          : await import("/src/components/settings/debug/LogLevelSelector.tsx");
      createRoot(document.getElementById("root")).render(
        React.createElement(module[component]),
      );
    },
    { component, holdInitial },
  );
  return page;
}

test("About Quit sends exit zero, reports translated rejection and allows retry", async (t) => {
  const page = await fixture(t);
  const quit = page.getByRole("button", { name: "Quit notype", exact: true });
  await quit.click();
  await page.waitForFunction(() =>
    window.calls.some((call) => call.cmd === "plugin:process|exit"),
  );
  assert.deepEqual(
    await page.evaluate(() =>
      window.calls.filter((call) => call.cmd === "plugin:process|exit"),
    ),
    [{ cmd: "plugin:process|exit", args: { code: 0 } }],
  );
  await page.evaluate(() => {
    window.mode = "quit-error";
  });
  await quit.click();
  await page
    .getByRole("status")
    .filter({
      hasText: "Unable to quit notype. Please try again.",
    })
    .waitFor({ timeout: 3000 });
  assert.doesNotMatch(
    await page.locator("body").innerText(),
    /Bearer|synthetic-secret|company\.invalid|private-user|notype\.diagnostics\.quitFailed/,
  );
  await page.evaluate(() => {
    window.mode = "ok";
  });
  await quit.click();
  await page.waitForFunction(
    () =>
      window.calls.filter((call) => call.cmd === "plugin:process|exit")
        .length === 3,
  );
  assert.deepEqual(
    await page.evaluate(() =>
      window.calls
        .filter((call) => call.cmd === "plugin:process|exit")
        .map((call) => call.args),
    ),
    [{ code: 0 }, { code: 0 }, { code: 0 }],
  );
});

test("log level waits for settings and save, then displays the refreshed server acknowledgement", async (t) => {
  const page = await fixture(t, "LogLevelSelector", true);
  const dropdown = page.getByRole("button", { name: "Debug", exact: true });
  await dropdown.waitFor({ timeout: 3000 });
  assert.equal(await dropdown.isDisabled(), true);
  await page.waitForFunction(() => window.settingsWaiters.length > 0);
  await page.evaluate(() => window.releaseSettings());
  await page.waitForFunction(() => !document.querySelector("button").disabled);
  await page.evaluate(() => {
    window.mode = "level-pending";
    window.acknowledgedLevel = "warn";
    window.calls = [];
  });
  await dropdown.click();
  await page.getByRole("button", { name: "Trace", exact: true }).click();
  await page.waitForFunction(() => typeof window.releaseSave === "function");
  assert.equal(await dropdown.isDisabled(), true);
  assert.deepEqual(
    await page.evaluate(() =>
      window.calls.filter((call) => call.cmd === "set_log_level"),
    ),
    [{ cmd: "set_log_level", args: { level: "trace" } }],
  );
  await page.evaluate(() => {
    window.holdSettings = true;
    window.releaseSave();
  });
  await page.waitForFunction(() => window.settingsWaiters.length > 0);
  assert.equal(await dropdown.isDisabled(), true);
  await page.evaluate(() => window.releaseSettings());
  const acknowledged = page.getByRole("button", { name: "Warn", exact: true });
  await acknowledged.waitFor({ timeout: 3000 });
  assert.equal(await acknowledged.isDisabled(), false);
  assert.equal(
    await page.evaluate(() =>
      window.calls.some((call) => call.cmd === "get_app_settings"),
    ),
    true,
  );
});

test("log level rejection is translated, preserves the acknowledged value and allows retry", async (t) => {
  const page = await fixture(t, "LogLevelSelector");
  const dropdown = page.getByRole("button", { name: "Debug", exact: true });
  await page.waitForFunction(
    () => document.querySelector("button")?.disabled === false,
  );
  await page.evaluate(() => {
    window.mode = "level-error";
  });
  await dropdown.click();
  await page.getByRole("button", { name: "Info", exact: true }).click();
  await page
    .getByRole("status")
    .filter({
      hasText: "Unable to change the local log level. Please try again.",
    })
    .waitFor({ timeout: 3000 });
  assert.equal(await dropdown.isDisabled(), false);
  assert.doesNotMatch(
    await page.locator("body").innerText(),
    /Bearer|synthetic-secret|company\.invalid|private-user|notype\.diagnostics\.logLevelFailed/,
  );
  await page.evaluate(() => {
    window.mode = "ok";
    window.acknowledgedLevel = "info";
  });
  await dropdown.click();
  await page.getByRole("button", { name: "Info", exact: true }).click();
  await page
    .getByRole("button", { name: "Info", exact: true })
    .waitFor({ timeout: 3000 });
  assert.equal(await page.getByRole("status").innerText(), "");
  assert.deepEqual(
    await page.evaluate(() =>
      window.calls.filter((call) => call.cmd === "set_log_level"),
    ),
    [
      { cmd: "set_log_level", args: { level: "info" } },
      { cmd: "set_log_level", args: { level: "info" } },
    ],
  );
});
