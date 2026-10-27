import assert from "node:assert/strict";
import { uiFixture } from "./ui.mjs";

export async function providerConnectionsFixture(
  t,
  language = "en",
  connections = [],
  activeId = null,
  initialMode = "ok",
  showStatus = false,
) {
  const page = await uiFixture(t, {
    cacheName: "provider-connections",
    entry: `export {default as React} from 'react'; export {createRoot} from 'react-dom/client'; export {PostProcessingSettings} from '/src/components/settings/post-processing/PostProcessingSettings.tsx'; export {ConnectionStatus} from '/src/components/ConnectionStatus.tsx'; export {default as i18n} from '/src/i18n/index.ts'; import '/src/App.css';`,
  });
  page.setDefaultTimeout(3000);
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  t.after(() => assert.deepEqual(errors, []));
  await page.evaluate(
    async ({
      language,
      connections,
      activeId,
      initialMode,
      template,
      showStatus,
    }) => {
      window.calls = [];
      window.modelMode = "ok";
      window.mode = initialMode;
      window.settings = {
        custom_words: [],
        app_language: language,
        post_process_enabled: false,
      };
      window.testMode = "ok";
      window.usage = {};
      window.testResults = {};
      const emptySummary = () => ({
        call_count: 0,
        observed_call_count: 0,
        input: {
          status: "not_applicable",
          total: null,
          known_subtotal: null,
          reported_calls: 0,
        },
        output: {
          status: "not_applicable",
          total: null,
          known_subtotal: null,
          reported_calls: 0,
        },
        fees: {
          status: "not_applicable",
          totals: [],
          known_subtotals: [],
          reported_calls: 0,
        },
        requested_models: [],
        reported_models: [],
        model_efforts: [],
      });
      window.configuration = {
        schema_version: 2,
        revision: 0,
        connections,
        active_connection_id: activeId,
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
          if (cmd === "get_text_configuration") {
            if (window.mode === "get-error") throw "Bearer read-secret";
            return window.configuration;
          }
          if (cmd === "get_workflow_readiness") {
            if (window.mode === "readiness-error")
              throw "Bearer readiness-secret";
            return {
              text_configured:
                window.settings.post_process_enabled &&
                window.configuration.active_connection_id !== null,
              delivery_available: false,
              legacy_import_error: null,
            };
          }
          if (cmd === "get_app_settings" || cmd === "get_default_settings")
            return window.settings;
          if (cmd === "change_post_process_enabled_setting") {
            window.settings.post_process_enabled = args.enabled;
            return null;
          }
          if (cmd === "get_connection_activity")
            return window.configuration.connections.map((connection) => ({
              configuration_revision: window.configuration.revision,
              connection_id: connection.id,
              summary: window.usage[connection.id] ?? emptySummary(),
              ...(window.testResults[connection.id] ?? {
                test_success: null,
                test_error: null,
                tested_at_ms: null,
              }),
            }));
          if (cmd === "test_text_connection") {
            if (window.testMode === "pending")
              await new Promise((resolve) => (window.releaseTest = resolve));
            window.usage[args.id] = {
              ...emptySummary(),
              call_count: 1,
              observed_call_count: 1,
              input: {
                status: "complete",
                total: 13,
                known_subtotal: 13,
                reported_calls: 1,
              },
              output: {
                status: "complete",
                total: 2,
                known_subtotal: 2,
                reported_calls: 1,
              },
              fees: {
                status: "unknown",
                totals: [],
                known_subtotals: [],
                reported_calls: 0,
              },
            };
            window.testResults[args.id] = {
              test_success: window.testMode !== "fail",
              test_error:
                window.testMode === "fail" ? "provider-timeout" : null,
              tested_at_ms: 1793200000000,
            };
            if (window.testMode === "fail") throw "provider-timeout";
            return null;
          }
          if (cmd === "clear_connection_usage") {
            if (window.testMode === "clear-fail")
              throw "synthetic-private-clear-error";
            window.usage[args.id] = emptySummary();
            return null;
          }
          if (cmd === "check_custom_sounds")
            return { start: false, stop: false };
          if (cmd === "is_update_checks_locked") return false;
          if (cmd === "create_text_connection") {
            if (args.expectedRevision !== window.configuration.revision)
              throw "configuration-conflict";
            const created = {
              ...template,
              id: "created-connection",
              display_name: args.displayName,
              kind: args.kind,
              dialect: args.dialect,
              model: "",
              api_address: "",
              capability_declaration: null,
              resolution: null,
              effective_endpoint: null,
              authentication: args.kind === "local" ? "none" : "bearer",
              credential_source:
                args.kind === "provider" ? "unresolved" : "none",
              credential_present: null,
            };
            window.configuration = {
              ...window.configuration,
              revision: window.configuration.revision + 1,
              connections: [...window.configuration.connections, created],
            };
            return structuredClone(window.configuration);
          }
          if (
            [
              "save_scenario_prompt",
              "reset_scenario_prompt",
              "save_translation_settings",
            ].includes(cmd)
          ) {
            if (args.expectedRevision !== window.configuration.revision)
              throw "configuration-conflict";
            window.configuration = {
              ...window.configuration,
              revision: window.configuration.revision + 1,
              prompts:
                cmd === "save_scenario_prompt" ||
                cmd === "reset_scenario_prompt"
                  ? {
                      ...window.configuration.prompts,
                      [args.scenario]:
                        cmd === "reset_scenario_prompt"
                          ? "Default prompt"
                          : args.prompt,
                    }
                  : window.configuration.prompts,
              translation:
                cmd === "save_translation_settings"
                  ? args.translation
                  : window.configuration.translation,
            };
            return structuredClone(window.configuration);
          }
          if (cmd === "list_text_models") {
            if (window.modelMode === "invoke-error-object")
              throw new Error("UNIQUE_MODEL_REJECTION_PRIVATE_SENTINEL");
            if (window.modelMode === "invoke-error")
              throw "Bearer upstream-secret";
            const request = args.request;
            const response = {
              request_id: request.request_id,
              configuration_revision: request.expected_revision,
              connection_id: request.connection.id,
              status:
                window.modelMode === "unsupported"
                  ? "unsupported"
                  : ["error", "auth-error"].includes(window.modelMode)
                    ? "error"
                    : "ok",
              models:
                window.modelMode === "empty"
                  ? []
                  : [{ id: "catalog/model-a" }, { id: "catalog/model-b" }],
              error_code:
                window.modelMode === "error"
                  ? "Bearer upstream-secret"
                  : window.modelMode === "auth-error"
                    ? "provider-auth"
                    : null,
              retry_after_seconds: null,
              credential_source: "none",
              cache_hit: false,
            };
            if (window.modelMode === "pending")
              await new Promise((resolve) => (window.releaseModels = resolve));
            return response;
          }
          if (cmd === "cancel_text_models") return null;
          if (cmd === "plugin:clipboard-manager|write_text") {
            window.copiedText = args.text;
            return null;
          }
          if (cmd === "save_text_connection") {
            if (window.mode === "save-error")
              throw "Bearer should-never-render";
            if (args.expectedRevision !== window.configuration.revision)
              throw "configuration-conflict";
            window.configuration = {
              ...window.configuration,
              revision: window.configuration.revision + 1,
              connections: window.configuration.connections.map((connection) =>
                connection.id === args.connection.id
                  ? {
                      ...connection,
                      ...args.connection,
                      app_key_present:
                        args.keyAction.action === "replace"
                          ? true
                          : args.keyAction.action === "clear"
                            ? false
                            : connection.app_key_present,
                    }
                  : connection,
              ),
            };
            return structuredClone(window.configuration);
          }
          if (
            cmd === "activate_text_connection" ||
            cmd === "remove_text_connection"
          ) {
            if (args.expectedRevision !== window.configuration.revision)
              throw "configuration-conflict";
            window.configuration = {
              ...window.configuration,
              revision: window.configuration.revision + 1,
              active_connection_id:
                cmd === "activate_text_connection"
                  ? args.id
                  : window.configuration.active_connection_id === args.id
                    ? null
                    : window.configuration.active_connection_id,
              connections:
                cmd === "remove_text_connection"
                  ? window.configuration.connections.filter(
                      (connection) => connection.id !== args.id,
                    )
                  : window.configuration.connections,
            };
            return structuredClone(window.configuration);
          }
          throw new Error(`Unexpected fixture IPC: ${cmd}`);
        },
      };
      const {
        React,
        createRoot,
        PostProcessingSettings,
        ConnectionStatus,
        i18n,
      } = await import("/__ui_fixture.tsx");
      await i18n.changeLanguage(language);
      window.uiRoot = createRoot(document.getElementById("root"));
      window.render = () =>
        window.uiRoot.render(
          React.createElement(
            React.Fragment,
            null,
            React.createElement(PostProcessingSettings),
            showStatus ? React.createElement(ConnectionStatus) : null,
          ),
        );
      window.render();
    },
    {
      language,
      connections,
      activeId,
      initialMode,
      template: savedConnection,
      showStatus,
    },
  );
  return page;
}

export const savedConnection = {
  id: "connection-a",
  display_name: "Company writing",
  kind: "custom",
  dialect: "generic",
  address_mode: "base_url",
  api_address: "https://synthetic.invalid/v1",
  models_endpoint: null,
  model: "manual-model",
  authentication: "none",
  timeout_ms: 30000,
  thinking_preference: "off",
  effort: null,
  capability_declaration: {
    kind: "non_reasoning",
    minimum_effort: null,
    supported_efforts: [],
    temperature_supported: true,
    top_p_supported: true,
  },
  temperature: null,
  top_p: null,
  max_output_tokens: null,
  app_key_present: false,
  credential_present: false,
  credential_source: "none",
  effective_endpoint: "https://synthetic.invalid/v1/chat/completions",
  resolution: {
    capability: "non_reasoning",
    provenance: "user_declared",
    effective_thinking: null,
    effective_effort: null,
    supported_efforts: [],
    reason: "non_reasoning",
    temperature_wire: null,
    top_p_wire: null,
    temperature_resolution: "omitted",
    top_p_resolution: "omitted",
    documented_top_p: null,
  },
  configuration_error: null,
};
