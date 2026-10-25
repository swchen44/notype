import { uiFixture } from "./ui.mjs";

export async function historyApiFixture(
  t,
  component = "HistorySettings",
  language = "en",
) {
  const page = await uiFixture(t, {
    cacheName:
      component === "HistorySettings"
        ? "history-api-history"
        : "history-api-options",
    entry: `export {default as React} from 'react'; export {createRoot} from 'react-dom/client'; export {HistorySettings} from '/src/components/settings/history/HistorySettings.tsx'; export {PostProcessingSettings} from '/src/components/settings/post-processing/PostProcessingSettings.tsx'; export {default as i18n} from '/src/i18n/index.ts'; import '/src/App.css';`,
  });
  page.setDefaultTimeout(3000);
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  t.after(() => assertNoErrors(errors));
  await page.evaluate(
    async ({ component, language }) => {
      window.calls = [];
      window.entries = [];
      window.mode = "ok";
      window.configuration = {
        transport: "open_ai_chat",
        endpoint: "https://synthetic.invalid/chat",
        model: "fixture-model",
        authentication: "bearer",
        api_key: "synthetic-key",
        timeout_ms: 30000,
        provider_profile: "generic",
        effort: null,
        max_output_tokens: null,
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
          if (cmd === "get_history_entries")
            return args.query ? [] : window.entries;
          if (cmd === "get_workflow_readiness")
            return {
              text_configured: true,
              delivery_available: false,
              legacy_import_error: null,
            };
          if (cmd === "get_text_configuration") return window.configuration;
          if (cmd === "set_text_configuration") {
            if (window.mode === "save-error")
              throw "Bearer synthetic-secret C:\\Users\\private";
            if (window.mode === "save-pending")
              await new Promise((resolve) => (window.releaseSave = resolve));
            window.configuration = args.configuration;
            return null;
          }
          if (cmd === "get_app_settings" || cmd === "get_default_settings")
            return { custom_words: [], app_language: language };
          if (cmd === "check_custom_sounds")
            return { start: false, stop: false };
          if (cmd === "is_update_checks_locked") return false;
          if (cmd === "export_history_csv") {
            if (window.mode === "export-error")
              throw "Bearer synthetic-secret C:\\Users\\private";
            if (window.mode === "export-pending")
              await new Promise((resolve) => (window.releaseExport = resolve));
            return window.mode === "export-cancel"
              ? { status: "cancelled", rows: null }
              : { status: "saved", rows: 83 };
          }
          throw new Error(`Unexpected fixture IPC: ${cmd}`);
        },
      };
      const { React, createRoot, i18n, ...components } = await import(
        "/__ui_fixture.tsx"
      );
      await i18n.changeLanguage(language);
      window.uiRoot = createRoot(document.getElementById("root"));
      window.render = () =>
        window.uiRoot.render(React.createElement(components[component]));
    },
    { component, language },
  );
  return page;
}
function assertNoErrors(errors) {
  if (errors.length) throw new Error(errors.join("\n"));
}

export const historyEntry = {
  id: "session-1",
  transcript: "Raw text has different JS length",
  result: "Result",
  created_at: "2026-10-25T00:00:00Z",
  updated_at: "2026-10-25T00:00:00Z",
  status: "pending",
  error: null,
  recognition_mode: null,
  selection: "polished",
  scenario: "dictation",
  translation_target: null,
  source_language: null,
  back_translation: null,
  review_required: false,
  review_confirmed: false,
  revision: 1,
  observations: {
    recording: { audio_ms: 1234, quality: "incomplete" },
    requested_asr_model: "requested-asr",
    actual_asr_model: {
      id: "actual-asr",
      name: "Actual producer",
      backend: "cpu",
    },
    original_count: 91,
    result_count: 0,
    count_rule: "zh-en-v1",
    asr_after_stop_ms: 0,
    initial_stop_to_result_ms: 2345,
    latest_processing_kind: "retry",
    latest_processing_ms: 456,
    latest_api_ms: null,
    latest_outcome: "cancelled",
    api_metadata_unavailable: false,
  },
  api_summary: {
    call_count: 3,
    observed_call_count: 3,
    input: {
      status: "complete",
      total: 0,
      known_subtotal: 0,
      reported_calls: 3,
    },
    output: {
      status: "partial",
      total: null,
      known_subtotal: 17,
      reported_calls: 2,
    },
    fees: {
      status: "partial",
      totals: [],
      known_subtotals: [
        { currency: "USD", amount: "0.100000000000000001" },
        { currency: "TWD", amount: "2.50" },
      ],
      reported_calls: 1,
    },
    requested_models: ["request-a", "request-b"],
    reported_models: ["reported-a"],
    model_efforts: [
      {
        requested_model: "request-a",
        reported_model: "reported-a",
        requested_effort: "high",
        reported_effort: null,
      },
      {
        requested_model: "request-b",
        reported_model: null,
        requested_effort: null,
        reported_effort: "low",
      },
    ],
  },
};
