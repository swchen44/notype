import assert from "node:assert/strict";
import { uiFixture } from "./ui.mjs";

export async function providerConnectionsFixture(t, language = "en") {
  const page = await uiFixture(t, {
    cacheName: "provider-connections",
    entry: `export {default as React} from 'react'; export {createRoot} from 'react-dom/client'; export {PostProcessingSettings} from '/src/components/settings/post-processing/PostProcessingSettings.tsx'; export {default as i18n} from '/src/i18n/index.ts'; import '/src/App.css';`,
  });
  page.setDefaultTimeout(3000);
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  t.after(() => assert.deepEqual(errors, []));
  await page.evaluate(async (language) => {
    window.calls = [];
    window.configuration = {
      schema_version: 2,
      revision: 0,
      connections: [],
      active_connection_id: null,
      prompts: {
        dictation: "Saved dictation",
        translation: "Saved translation",
        ask: "Saved ask",
      },
      translation: {
        target_language: "en-US",
        domain: "Fixture domain",
        tone: "",
        glossary: [],
        back_translation: false,
      },
    };
    window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener() {} };
    window.__TAURI_INTERNALS__ = {
      transformCallback: () => 1,
      invoke: async (cmd, args) => {
        window.calls.push({ cmd, args });
        if (cmd === "plugin:event|listen") return 1;
        if (cmd === "plugin:event|unlisten") return null;
        if (cmd === "get_text_configuration") return window.configuration;
        if (cmd === "get_workflow_readiness")
          return {
            text_configured: false,
            delivery_available: false,
            legacy_import_error: null,
          };
        if (cmd === "get_app_settings" || cmd === "get_default_settings")
          return { custom_words: [], app_language: language };
        if (cmd === "check_custom_sounds") return { start: false, stop: false };
        if (cmd === "is_update_checks_locked") return false;
        throw new Error(`Unexpected fixture IPC: ${cmd}`);
      },
    };
    const { React, createRoot, PostProcessingSettings, i18n } = await import(
      "/__ui_fixture.tsx"
    );
    await i18n.changeLanguage(language);
    window.uiRoot = createRoot(document.getElementById("root"));
    window.render = () =>
      window.uiRoot.render(React.createElement(PostProcessingSettings));
    window.render();
  }, language);
  return page;
}
