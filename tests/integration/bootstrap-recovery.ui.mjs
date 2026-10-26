import test from "node:test";
import assert from "node:assert/strict";
import { uiFixture } from "./fixtures/ui.mjs";

for (const labels of [
  {
    language: "en",
    title: "Setup incomplete",
    description:
      "Local defaults have not been fully saved. Capture and API operations are paused. History, safe settings and the model folder controls remain available. Choose a valid model folder if needed, then retry.",
    retry: "Retry setup",
    retrying: "Retrying…",
  },
  {
    language: "zh-CN",
    title: "首次设置尚未完成",
    description:
      "离线包默认设置尚未完整保存，录音与 API 操作暂停。仍可查看文字历史、安全设置及模型文件夹；如有需要，请选择有效的模型文件夹后重试。",
    retry: "重试设置",
    retrying: "重试中…",
  },
]) {
  test(`setup recovery in ${labels.language} offers explicit retry without automatic API calls and waits for durable acknowledgment`, async (t) => {
    const page = await uiFixture(t, {
      cacheName: "bootstrap-recovery",
      entry: `export {default as React} from 'react'; export {createRoot} from 'react-dom/client'; export {BootstrapRecovery,useBootstrapRecovery} from '/src/components/BootstrapRecovery.tsx'; export {default as i18n} from '/src/i18n/index.ts'; import '/src/App.css';`,
    });
    await page.evaluate(async (language) => {
      window.calls = [];
      window.__TAURI_INTERNALS__ = {
        invoke: async (cmd) => {
          window.calls.push(cmd);
          if (cmd === "get_app_settings") return { app_language: language };
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
      const {
        React,
        createRoot,
        BootstrapRecovery,
        useBootstrapRecovery,
        i18n,
      } = await import("/__ui_fixture.tsx");
      await i18n.changeLanguage(language);
      function Fixture() {
        return React.createElement(BootstrapRecovery, useBootstrapRecovery());
      }
      createRoot(document.getElementById("root")).render(
        React.createElement(Fixture),
      );
    }, labels.language);
    const recovery = page.getByRole("status");
    await recovery
      .getByRole("heading", { name: labels.title, exact: true })
      .waitFor();
    await recovery.getByText(labels.description, { exact: true }).waitFor();
    assert.deepEqual(
      await page.evaluate(() =>
        window.calls.filter((cmd) => cmd !== "get_app_settings"),
      ),
      ["get_bootstrap_status"],
    );
    await recovery
      .getByRole("button", { name: labels.retry, exact: true })
      .click();
    assert.equal(
      await page
        .getByRole("button", { name: labels.retrying, exact: true })
        .isDisabled(),
      true,
    );
    await page.evaluate(() =>
      window.finishBootstrap({ state: "ready", code: null, can_retry: false }),
    );
    await page
      .getByText(labels.title, { exact: true })
      .waitFor({ state: "detached" });
    assert.deepEqual(
      await page.evaluate(() =>
        window.calls.filter((cmd) => cmd !== "get_app_settings"),
      ),
      ["get_bootstrap_status", "retry_offline_bootstrap"],
    );
  });
}
