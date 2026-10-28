import assert from "node:assert/strict";
import test from "node:test";
import { uiFixture } from "./fixtures/ui.mjs";
import {
  providerConnectionsFixture,
  savedConnection,
} from "./fixtures/provider-connections.mjs";

test("selected speech model remains explicitly unloaded until an actual load event", async (t) => {
  const page = await uiFixture(t, {
    cacheName: "footer-status",
    entry: `export {default as React} from 'react'; export {createRoot} from 'react-dom/client'; export {default as ModelSelector} from '/src/components/model-selector/ModelSelector.tsx'; export {useModelStore} from '/src/stores/modelStore.ts'; export {default as i18n} from '/src/i18n/index.ts'; import '/src/App.css';`,
  });
  page.setDefaultTimeout(3000);
  await page.evaluate(async () => {
    window.callbacks = {};
    window.listeners = {};
    let next = 0;
    window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener() {} };
    window.__TAURI_INTERNALS__ = {
      transformCallback(fn) {
        const id = ++next;
        window.callbacks[id] = fn;
        return id;
      },
      invoke: async (cmd, args) => {
        if (cmd === "get_transcription_model_status") return null;
        if (cmd === "plugin:event|listen") {
          (window.listeners[args.event] ??= []).push(args.handler);
          return args.handler;
        }
        if (cmd === "plugin:event|unlisten") return null;
        throw new Error("Unexpected fixture IPC: " + cmd);
      },
    };
    const { React, createRoot, ModelSelector, useModelStore, i18n } =
      await import("/__ui_fixture.tsx");
    await i18n.changeLanguage("en");
    useModelStore.setState({
      generation: 1,
      currentModel: "fixture-model",
      models: [
        {
          id: "fixture-model",
          name: "Fixture speech",
          description: "Synthetic speech model",
          is_downloaded: true,
          is_downloading: false,
          supported_languages: ["en"],
          is_custom: false,
          supports_streaming: false,
        },
      ],
      downloadProgress: {},
      downloadStats: {},
      verifyingModels: {},
      extractingModels: {},
    });
    createRoot(document.getElementById("root")).render(
      React.createElement(ModelSelector),
    );
  });
  const unloaded = page.getByRole("button", {
    name: /Not loaded.*Fixture speech/,
  });
  await unloaded.waitFor();
  assert.match(
    await unloaded.locator("div").first().getAttribute("class"),
    /bg-mid-gray/,
  );
  await unloaded.click();
  await page.getByText("Selected", { exact: true }).waitFor();
  assert.equal(await page.getByText("Active", { exact: true }).count(), 0);
  await page.evaluate(() => {
    for (const id of window.listeners["model-state-changed"] ?? [])
      window.callbacks[id]({
        event: "model-state-changed",
        id,
        payload: {
          generation: 1,
          model_id: "fixture-model",
          event_type: "loading_completed",
        },
      });
  });
  const ready = page.getByRole("button", { name: /Loaded.*Fixture speech/ });
  await ready.waitFor();
  assert.match(
    await ready.locator("div").first().getAttribute("class"),
    /bg-green/,
  );
});

test("AI server footer names the server but never shows enabled-green while AI is off", async (t) => {
  const page = await providerConnectionsFixture(
    t,
    "en",
    [savedConnection],
    savedConnection.id,
    "ok",
    true,
  );
  const footer = page.getByRole("region", {
    name: "Text processing status",
    exact: true,
  });
  await footer.getByText("AI server", { exact: true }).waitFor();
  await footer.getByText("Company writing", { exact: true }).waitFor();
  const indicator = footer.locator("[data-connection-state]");
  assert.equal(
    await indicator.getAttribute("data-connection-state"),
    "disabled",
  );
  await page.evaluate(() => {
    window.testResults["connection-a"] = {
      test_success: true,
      test_error: null,
      tested_at_ms: 1,
    };
    window.dispatchEvent(new Event("notype-text-configuration-changed"));
  });
  await footer.getByText("Last test succeeded", { exact: true }).waitFor();
  assert.equal(
    await indicator.getAttribute("data-connection-state"),
    "disabled",
  );
  await page
    .getByRole("checkbox", { name: "Enable AI text processing", exact: true })
    .locator("..")
    .click();
  await page.waitForFunction(
    () =>
      document
        .querySelector("[data-connection-state]")
        ?.getAttribute("data-connection-state") === "succeeded",
  );
  assert.match(await indicator.getAttribute("class"), /bg-green/);
  assert.equal(
    await page.evaluate(() =>
      window.calls.some((c) => c.cmd === "test_text_connection"),
    ),
    false,
  );
});
