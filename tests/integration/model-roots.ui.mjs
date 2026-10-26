import assert from "node:assert/strict";
import test from "node:test";
import { uiFixture } from "./fixtures/ui.mjs";

const model = {
  id: "fixture-model",
  name: "Retained speech fixture",
  description: "Synthetic supported speech model.",
  filename: "fixture-Q6_K.gguf",
  source: {
    HuggingFace: { repo_id: "fixture/speech", revision: "fixture-revision" },
  },
  size_mb: 1,
  is_downloaded: true,
  is_downloading: false,
  partial_size: 0,
  is_directory: false,
  engine_type: "Whisper",
  accuracy_score: 0.8,
  speed_score: 0.7,
  supports_translation: false,
  is_recommended: false,
  supported_languages: ["en", "zh"],
  supports_language_selection: true,
  is_custom: false,
  supports_streaming: true,
  supports_language_detection: true,
};

async function fixture(
  t,
  component = "onboarding",
  language = "en",
  initial = {},
) {
  const page = await uiFixture(t, {
    cacheName: "model-roots",
    entry: `export {default as React} from 'react'; export {createRoot} from 'react-dom/client'; export {default as Onboarding} from '/src/components/onboarding/Onboarding.tsx'; export {default as App} from '/src/App.tsx'; export {ModelsSettings} from '/src/components/settings/models/ModelsSettings.tsx'; export {useModelStore} from '/src/stores/modelStore.ts'; export {default as i18n} from '/src/i18n/index.ts'; import '/src/App.css';`,
  });
  page.setDefaultTimeout(3000);
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  t.after(() => assert.deepEqual(errors, []));
  await page.evaluate(
    async ({ component, language, initial }) => {
      window.calls = [];
      window.callbacks = {};
      window.listeners = {};
      window.handlers = {};
      window.catalogFailure = initial.catalogFailure ?? false;
      delete initial.catalogFailure;
      window.folder = "D:\\synthetic-retained-models";
      window.catalog = {
        generation: 1,
        root: {
          generation: 1,
          revision: 0,
          folder: "D:\\synthetic-empty",
          status: "no_models",
          writable: true,
          busy: false,
          model_count: 0,
        },
        current_model: "remembered-model",
        models: [],
        artifacts: {},
        ...initial,
      };
      window.candidateRoot = { ...window.catalog.root, folder: window.folder };
      let nextCallback = 0;
      window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener() {} };
      window.__TAURI_OS_PLUGIN_INTERNALS__ = {
        platform: "windows",
        family: "windows",
        arch: "x86_64",
      };
      window.__TAURI_INTERNALS__ = {
        transformCallback(callback) {
          const id = ++nextCallback;
          window.callbacks[id] = callback;
          return id;
        },
        invoke: async (cmd, args) => {
          window.calls.push({ cmd, args });
          if (window.handlers[cmd]) return window.handlers[cmd](args);
          if (cmd === "plugin:event|listen") {
            (window.listeners[args.event] ??= []).push(args.handler);
            return args.handler;
          }
          if (cmd === "plugin:event|unlisten") return null;
          if (cmd === "get_model_catalog" && window.catalogFailure)
            throw new Error("PRIVATE_CATALOG_FAILURE");
          if (cmd === "get_model_catalog") {
            const catalog = structuredClone(window.catalog);
            catalog.artifacts = Object.fromEntries(
              Object.entries(catalog.artifacts).map(([id, artifact]) => [
                id,
                { source: "selected_root", ...artifact },
              ]),
            );
            return catalog;
          }
          if (cmd === "get_model_root")
            return structuredClone(window.catalog.root);
          if (cmd === "get_app_settings" || cmd === "get_default_settings")
            return {
              app_language: language,
              onboarding_completed: true,
              selected_model: window.catalog.current_model,
              custom_words: [],
              bindings: {},
              post_process_providers: [],
              post_process_api_keys: {},
              post_process_models: {},
            };
          if (cmd === "get_windows_microphone_permission_status")
            return { supported: false };
          if (cmd === "get_audio_devices" || cmd === "get_output_devices")
            return [];
          if (cmd === "get_history_entries") return [];
          if (cmd === "get_workflow_readiness")
            return {
              text_configured: false,
              delivery_available: false,
              legacy_import_error: null,
            };
          if (cmd === "check_custom_sounds")
            return { start: false, stop: false };
          if (cmd === "plugin:dialog|open") return window.folder;
          if (cmd === "validate_model_root")
            return {
              request_id: args.requestId,
              candidate_id: args.requestId + "-candidate",
              root: structuredClone(window.candidateRoot),
            };
          if (cmd === "apply_model_root") {
            window.catalog = {
              ...window.catalog,
              generation: 2,
              root: { ...window.candidateRoot, generation: 2, revision: 1 },
              models: window.candidateModels ?? window.catalog.models,
              artifacts: window.candidateArtifacts ?? window.catalog.artifacts,
            };
            return structuredClone(window.catalog.root);
          }
          if (cmd === "rescan_local_models")
            return structuredClone(window.catalog.root);
          if (cmd === "set_active_model") {
            window.catalog.current_model = args.modelId;
            return null;
          }
          return null;
        },
      };
      window.emitModelEvent = (event, payload) => {
        const ids = window.listeners[event];
        if (!ids) throw new Error("Missing native event listener: " + event);
        for (const id of ids) window.callbacks[id]({ event, id, payload });
      };
      const {
        React,
        createRoot,
        Onboarding,
        App,
        ModelsSettings,
        useModelStore,
        i18n,
      } = await import("/__ui_fixture.tsx");
      window.modelStore = useModelStore;
      await i18n.changeLanguage(language);
      await useModelStore.getState().initialize();
      createRoot(document.getElementById("root")).render(
        React.createElement(
          component === "app"
            ? App
            : component === "onboarding" || component === "preview"
              ? Onboarding
              : ModelsSettings,
          {
            onModelSelected() {
              window.selected = true;
            },
            preview: component === "preview",
          },
        ),
      );
    },
    { component, language, initial },
  );
  return page;
}

test("fresh onboarding reuses reviewed models without acquisition and waits for explicit model selection", async (t) => {
  const page = await fixture(t);
  await page.getByText("0 models available", { exact: true }).waitFor();
  await page.evaluate((model) => {
    window.candidateRoot = {
      ...window.candidateRoot,
      status: "ready",
      model_count: 1,
    };
    window.candidateModels = [model];
    window.candidateArtifacts = {
      [model.id]: {
        status: "verified",
        can_delete: false,
        can_download: false,
      },
    };
  }, model);
  await page
    .getByRole("button", { name: "Choose model folder", exact: true })
    .click();
  await page
    .getByText("D:\\synthetic-retained-models", { exact: true })
    .waitFor();
  assert.equal(
    await page.evaluate(
      () => window.calls.filter((c) => c.cmd === "apply_model_root").length,
    ),
    0,
  );
  await page.getByRole("button", { name: "Apply folder", exact: true }).click();
  await page
    .getByRole("button", { name: "Apply folder", exact: true })
    .waitFor({ state: "detached" });
  assert.equal(
    await page.evaluate(() => window.catalog.root.folder),
    "D:\\synthetic-retained-models",
  );
  assert.equal(
    await page.evaluate(() => window.modelStore.getState().currentModel),
    "remembered-model",
  );
  assert.deepEqual(
    await page.evaluate(() =>
      window.calls.filter((c) =>
        /download|set_active|load_model|delete_model/.test(c.cmd),
      ),
    ),
    [],
  );
  await page.getByRole("heading", { name: model.name, exact: true }).waitFor();
  await page.getByRole("button", { name: "Use model", exact: true }).click();
  await page.waitForFunction(() => window.selected === true);
});

test("a returning user with a missing saved model reaches folder recovery and retains the saved preference", async (t) => {
  const page = await fixture(t, "app", "en", {
    root: {
      generation: 1,
      revision: 0,
      folder: "D:\\synthetic-missing-folder",
      status: "unavailable",
      writable: false,
      busy: false,
      model_count: 0,
    },
  });
  await page
    .getByRole("button", { name: "Choose model folder", exact: true })
    .waitFor();
  assert.equal(
    await page.evaluate(() => window.modelStore.getState().currentModel),
    "remembered-model",
  );
  assert.deepEqual(
    await page.evaluate(() =>
      window.calls.filter((c) =>
        /download_model|set_active_model|load_model/.test(c.cmd),
      ),
    ),
    [],
  );
  await page.getByRole("button", { name: "History", exact: true }).click();
  await page
    .getByRole("button", { name: "Choose model folder", exact: true })
    .waitFor();
  assert.equal(
    await page.getByRole("button", { name: "Models", exact: true }).isVisible(),
    true,
  );
  await page.setViewportSize({ width: 800, height: 900 });
  await page.screenshot({
    path: ".local/p28-returning-history-recovery-synthetic.png",
    fullPage: true,
  });
});

test("read-only folders allow explicit model use and rescan while imported models have no delete action", async (t) => {
  const page = await fixture(t, "settings", "en", {
    root: {
      generation: 1,
      revision: 0,
      folder: "D:\\synthetic-readonly",
      status: "ready",
      writable: false,
      busy: false,
      model_count: 1,
    },
    models: [
      model,
      {
        ...model,
        id: "missing",
        name: "Missing speech fixture",
        is_downloaded: false,
      },
    ],
    artifacts: {
      [model.id]: {
        status: "verified",
        can_delete: false,
        can_download: false,
      },
      missing: {
        status: "unavailable",
        can_delete: false,
        can_download: false,
      },
    },
  });
  await page.getByText("1 model available", { exact: true }).waitFor();
  await page.getByRole("button", { name: "Use model", exact: true }).click();
  await page.getByText("Active", { exact: true }).waitFor();
  assert.equal(await page.getByRole("button", { name: /Delete/ }).count(), 0);
  assert.equal(
    await page
      .getByRole("button", { name: "Download model", exact: true })
      .count(),
    0,
  );
  await page
    .getByRole("button", { name: "Rescan models", exact: true })
    .click();
  await page.waitForFunction(() =>
    window.calls.some((c) => c.cmd === "rescan_local_models"),
  );
  assert.deepEqual(
    await page.evaluate(() =>
      window.calls.filter((c) => /download_model|delete_model/.test(c.cmd)),
    ),
    [],
  );
  await page.screenshot({
    path: ".local/p28-models-settings-readonly-synthetic.png",
    fullPage: true,
  });
});

test("initial catalog transport failure offers a safe explicit reload before folder validation", async (t) => {
  const page = await fixture(t, "settings", "en", { catalogFailure: true });
  await page
    .getByRole("button", { name: "Reload model folder", exact: true })
    .waitFor();
  assert.doesNotMatch(
    await page.locator("body").innerText(),
    /PRIVATE_CATALOG_FAILURE/,
  );
  await page.evaluate(() => (window.catalogFailure = false));
  await page
    .getByRole("button", { name: "Reload model folder", exact: true })
    .click();
  await page.getByText("D:\\synthetic-empty", { exact: true }).waitFor();
});

test("only the latest validation owns scan progress and cancellation cannot publish its late result", async (t) => {
  const page = await fixture(t, "settings");
  await page.evaluate(() => {
    window.validations = [];
    window.handlers.validate_model_root = (args) =>
      new Promise((resolve) => window.validations.push({ args, resolve }));
  });
  await page
    .getByRole("button", { name: "Choose model folder", exact: true })
    .click();
  await page.waitForFunction(() => window.validations.length === 1);
  await page
    .getByRole("button", { name: "Choose model folder", exact: true })
    .click();
  await page.waitForFunction(() => window.validations.length === 2);
  await page.evaluate(() => {
    window.emitModelEvent("model-root-scan-progress", {
      generation: 1,
      request_id: window.validations[1].args.requestId,
      checked: 2,
      total: 5,
    });
    window.emitModelEvent("model-root-scan-progress", {
      generation: 1,
      request_id: window.validations[0].args.requestId,
      checked: 99,
      total: 100,
    });
  });
  await page.getByText("Checked 2 of 5 models", { exact: true }).waitFor();
  assert.doesNotMatch(await page.locator("body").innerText(), /Checked 99/);
  await page.getByRole("button", { name: "Cancel", exact: true }).click();
  await page.evaluate(() => {
    for (const { args, resolve } of window.validations)
      resolve({
        request_id: args.requestId,
        candidate_id: "late",
        root: window.candidateRoot,
      });
  });
  await page
    .getByText("Checking models…", { exact: true })
    .waitFor({ state: "detached" });
  assert.equal(
    await page
      .getByRole("button", { name: "Apply folder", exact: true })
      .count(),
    0,
  );
  assert.equal(
    await page.evaluate(() => window.catalog.root.folder),
    "D:\\synthetic-empty",
  );
});

test("busy Apply preserves the old folder after cancellation acknowledgement until the worker ends and the user retries", async (t) => {
  const page = await fixture(t, "settings", "en", {
    root: {
      generation: 1,
      revision: 3,
      folder: "D:\\synthetic-busy",
      status: "ready",
      writable: true,
      busy: true,
      model_count: 1,
    },
    models: [{ ...model, is_downloaded: false, is_downloading: true }],
    artifacts: {
      [model.id]: {
        status: "incomplete",
        can_delete: false,
        can_download: true,
      },
    },
  });
  await page.evaluate(() => {
    window.workerRunning = true;
    window.handlers.apply_model_root = () => {
      if (window.workerRunning) throw "model-root-busy";
      window.catalog = {
        ...window.catalog,
        generation: 2,
        root: {
          ...window.candidateRoot,
          generation: 2,
          revision: 4,
          busy: false,
        },
      };
      return structuredClone(window.catalog.root);
    };
  });
  await page
    .getByRole("button", { name: "Choose model folder", exact: true })
    .click();
  const apply = page.getByRole("button", { name: "Apply folder", exact: true });
  await apply.click();
  await page
    .getByRole("alert")
    .filter({ hasText: "Finish or cancel" })
    .waitFor();
  assert.equal(
    await page.evaluate(() => window.catalog.root.folder),
    "D:\\synthetic-busy",
  );
  await page
    .getByRole("article")
    .getByRole("button", { name: "Cancel", exact: true })
    .click();
  await page.waitForFunction(() =>
    window.calls.some((c) => c.cmd === "cancel_download"),
  );
  await apply.click();
  await page
    .getByRole("alert")
    .filter({ hasText: "Finish or cancel" })
    .waitFor();
  assert.equal(
    await page.evaluate(
      () => window.calls.filter((c) => c.cmd === "apply_model_root").length,
    ),
    2,
  );
  await page.evaluate(() => (window.workerRunning = false));
  await apply.click();
  await apply.waitFor({ state: "detached" });
  assert.equal(await page.evaluate(() => window.catalog.root.revision), 4);
  const revisions = await page.evaluate(() =>
    window.calls
      .filter((c) => c.cmd === "apply_model_root")
      .map((c) => c.args.revision),
  );
  assert.deepEqual(revisions, [3, 3, 3]);
});

test("late catalogs and all old-generation model events cannot overwrite the new folder or report errors", async (t) => {
  const page = await fixture(t, "settings");
  await page.evaluate(() => {
    const old = structuredClone(window.catalog);
    window.handlers.get_model_catalog = () =>
      new Promise((resolve) => (window.releaseCatalog = () => resolve(old)));
  });
  await page
    .getByRole("button", { name: "Rescan models", exact: true })
    .click();
  await page.waitForFunction(() => typeof window.releaseCatalog === "function");
  await page.evaluate((model) => {
    window.catalog = {
      generation: 2,
      root: {
        ...window.catalog.root,
        generation: 2,
        revision: 1,
        folder: "D:\\synthetic-successor",
        status: "ready",
        model_count: 1,
      },
      current_model: model.id,
      models: [model],
      artifacts: {
        [model.id]: {
          status: "verified",
          can_delete: false,
          can_download: false,
        },
      },
    };
    delete window.handlers.get_model_catalog;
    window.emitModelEvent("model-root-changed", { generation: 2 });
  }, model);
  await page.getByText("D:\\synthetic-successor", { exact: true }).waitFor();
  await page.getByRole("heading", { name: model.name, exact: true }).waitFor();
  await page.evaluate(() => {
    window.releaseCatalog();
    for (const event of [
      "model-download-progress",
      "model-download-complete",
      "model-download-failed",
      "model-download-cancelled",
      "model-verification-started",
      "model-verification-completed",
      "model-extraction-started",
      "model-extraction-completed",
      "model-extraction-failed",
      "model-deleted",
      "models-updated",
      "model-root-changed",
      "model-state-changed",
    ])
      window.emitModelEvent(event, {
        generation: 1,
        model_id: "fixture-model",
        error: "PRIVATE_OLD_ROOT_SENTINEL",
        downloaded: 10,
        total: 10,
        percentage: 100,
      });
  });
  assert.equal(
    await page
      .getByRole("heading", { name: model.name, exact: true })
      .isVisible(),
    true,
  );
  assert.equal(
    await page
      .getByText("D:\\synthetic-successor", { exact: true })
      .isVisible(),
    true,
  );
  assert.equal(await page.getByRole("progressbar").count(), 0);
  assert.equal(await page.getByRole("alert").count(), 0);
  assert.doesNotMatch(
    await page.locator("body").innerText(),
    /PRIVATE_OLD_ROOT_SENTINEL/,
  );
});

test("old rescan rejection and finally cannot clear a successor scan spinner", async (t) => {
  const page = await fixture(t, "settings");
  await page.evaluate(() => {
    window.scans = [];
    window.handlers.rescan_local_models = () =>
      new Promise((resolve, reject) => window.scans.push({ resolve, reject }));
  });
  const rescan = page.getByRole("button", {
    name: "Rescan models",
    exact: true,
  });
  await rescan.click();
  await page.waitForFunction(() => window.scans.length === 1);
  await page.evaluate(() => {
    window.catalog.generation = 2;
    window.catalog.root = {
      ...window.catalog.root,
      generation: 2,
      revision: 1,
      folder: "D:\\synthetic-new-scan",
    };
    window.emitModelEvent("model-root-changed", { generation: 2 });
  });
  await page.getByText("D:\\synthetic-new-scan", { exact: true }).waitFor();
  await rescan.click();
  await page.waitForFunction(() => window.scans.length === 2);
  await page.evaluate(() =>
    window.scans[0].reject(new Error("PRIVATE_STALE_SCAN")),
  );
  assert.equal(await rescan.isDisabled(), true);
  assert.equal(await page.getByRole("alert").count(), 0);
  await page.evaluate(() =>
    window.scans[1].resolve(structuredClone(window.catalog.root)),
  );
  await page.waitForFunction(() => !window.modelStore.getState().isRescanning);
  assert.equal(await rescan.isEnabled(), true);
});

test("a late selection acknowledgement cannot clear a successor selection or change the saved preference", async (t) => {
  const page = await fixture(t, "settings", "en", {
    models: [model],
    artifacts: {
      [model.id]: {
        status: "verified",
        can_delete: false,
        can_download: false,
      },
    },
  });
  await page.evaluate(() => {
    window.selections = [];
    window.handlers.set_active_model = () =>
      new Promise((resolve, reject) =>
        window.selections.push({ resolve, reject }),
      );
  });
  await page.getByRole("button", { name: "Use model", exact: true }).click();
  await page.waitForFunction(() => window.selections.length === 1);
  await page.evaluate(() => {
    window.catalog.generation = 2;
    window.catalog.root = {
      ...window.catalog.root,
      generation: 2,
      revision: 1,
      folder: "D:\\synthetic-new-selection",
    };
    window.emitModelEvent("model-root-changed", { generation: 2 });
  });
  await page
    .getByText("D:\\synthetic-new-selection", { exact: true })
    .waitFor();
  await page.getByRole("button", { name: "Use model", exact: true }).click();
  await page.waitForFunction(() => window.selections.length === 2);
  await page.evaluate(() =>
    window.selections[0].reject("PRIVATE_OLD_SELECTION"),
  );
  assert.equal(
    await page.getByRole("button", { name: "Use model", exact: true }).count(),
    0,
  );
  assert.equal(await page.getByRole("alert").count(), 0);
  assert.equal(
    await page.evaluate(() => window.modelStore.getState().currentModel),
    "remembered-model",
  );
  await page.evaluate(() => {
    window.catalog.current_model = "fixture-model";
    window.selections[1].resolve(null);
  });
  await page.getByText("Active", { exact: true }).waitFor();
});

test("picker cancellation and candidate cancellation leave the current folder unchanged", async (t) => {
  const page = await fixture(t);
  await page.evaluate(() => (window.folder = null));
  await page
    .getByRole("button", { name: "Choose model folder", exact: true })
    .click();
  assert.deepEqual(
    await page.evaluate(() =>
      window.calls.filter((c) =>
        /validate_model_root|apply_model_root/.test(c.cmd),
      ),
    ),
    [],
  );
  await page.evaluate(() => (window.folder = "D:\\synthetic-retained-models"));
  await page
    .getByRole("button", { name: "Choose model folder", exact: true })
    .click();
  await page.getByRole("button", { name: "Cancel", exact: true }).click();
  assert.equal(
    await page
      .getByRole("button", { name: "Apply folder", exact: true })
      .count(),
    0,
  );
  assert.equal(
    await page.evaluate(() => window.catalog.root.folder),
    "D:\\synthetic-empty",
  );
});

test("failed durable Apply keeps the old folder and displays only a finite translated error", async (t) => {
  const page = await fixture(t, "settings");
  await page.evaluate(() => {
    window.handlers.apply_model_root = () => {
      throw "model-root-persistence-failed";
    };
  });
  await page
    .getByRole("button", { name: "Choose model folder", exact: true })
    .click();
  await page.getByRole("button", { name: "Apply folder", exact: true }).click();
  await page
    .getByRole("alert")
    .getByText(
      "Unable to save the folder. Your previous folder is still in use.",
      { exact: true },
    )
    .waitFor();
  assert.equal(
    await page.evaluate(() => window.modelStore.getState().root.folder),
    "D:\\synthetic-empty",
  );
  await page.evaluate(() => {
    window.handlers.apply_model_root = () => {
      throw new Error("D:\\PRIVATE_PROFILE\\Bearer-private");
    };
  });
  await page.getByRole("button", { name: "Apply folder", exact: true }).click();
  await page
    .getByRole("alert")
    .getByText("Unable to access the model folder. Please try again.", {
      exact: true,
    })
    .waitFor();
  assert.doesNotMatch(
    await page.locator("body").innerText(),
    /PRIVATE_PROFILE|Bearer-private/,
  );
});

for (const [language, choose, apply, invalid] of [
  [
    "en",
    "Choose model folder",
    "Apply folder",
    "Folder or models failed validation",
  ],
  ["zh-TW", "選擇模型資料夾", "套用資料夾", "資料夾或模型未通過驗證"],
  ["zh-CN", "选择模型文件夹", "应用文件夹", "文件夹或模型未通过验证"],
])
  test(`${language} invalid folder preview cannot be applied`, async (t) => {
    const page = await fixture(t, "settings", language);
    await page.evaluate(() => {
      window.candidateRoot.status = "invalid";
    });
    await page.getByRole("button", { name: choose, exact: true }).click();
    await page.getByText(invalid, { exact: true }).waitFor();
    assert.equal(
      await page.getByRole("button", { name: apply, exact: true }).isDisabled(),
      true,
    );
    assert.equal(
      await page.evaluate(
        () => window.calls.filter((c) => c.cmd === "apply_model_root").length,
      ),
      0,
    );
  });

test("onboarding preview leaves native folder controls inert", async (t) => {
  const page = await fixture(t, "preview");
  assert.equal(
    await page
      .getByRole("button", { name: "Choose model folder", exact: true })
      .isDisabled(),
    true,
  );
  assert.equal(
    await page
      .getByRole("button", { name: "Rescan models", exact: true })
      .isDisabled(),
    true,
  );
  assert.deepEqual(
    await page.evaluate(() =>
      window.calls.filter((c) =>
        /dialog\|open|validate_model_root|apply_model_root|rescan_local_models/.test(
          c.cmd,
        ),
      ),
    ),
    [],
  );
});

test("onboarding does not carry an unfinished download selection into the same model in a new folder", async (t) => {
  const page = await fixture(t, "onboarding", "en", {
    models: [{ ...model, is_downloaded: false }],
    artifacts: {
      [model.id]: {
        status: "unavailable",
        can_delete: false,
        can_download: true,
      },
    },
  });
  await page.evaluate(() => {
    window.handlers.download_model = () =>
      new Promise((resolve) => (window.releaseOldDownload = resolve));
  });
  await page
    .getByRole("button", { name: "Download model", exact: true })
    .click();
  await page.waitForFunction(
    () => typeof window.releaseOldDownload === "function",
  );
  await page.evaluate((model) => {
    window.catalog = {
      ...window.catalog,
      generation: 2,
      root: {
        ...window.catalog.root,
        generation: 2,
        revision: 1,
        folder: "D:\\synthetic-other-download",
        status: "ready",
        model_count: 1,
      },
      models: [model],
      artifacts: {
        [model.id]: {
          status: "verified",
          can_delete: false,
          can_download: false,
        },
      },
    };
    window.emitModelEvent("model-root-changed", { generation: 2 });
    window.releaseOldDownload(null);
  }, model);
  await page
    .getByText("D:\\synthetic-other-download", { exact: true })
    .waitFor();
  await page.getByRole("button", { name: "Use model", exact: true }).waitFor();
  assert.deepEqual(
    await page.evaluate(() =>
      window.calls.filter((c) => c.cmd === "set_active_model"),
    ),
    [],
  );
  assert.equal(await page.evaluate(() => window.selected ?? false), false);
});

test("selection queued before a folder change cannot mutate the successor preference at native admission", async (t) => {
  const page = await fixture(t, "settings", "en", {
    models: [model],
    artifacts: {
      [model.id]: {
        status: "verified",
        can_delete: false,
        can_download: false,
      },
    },
  });
  await page.evaluate(() => {
    window.handlers.set_active_model = (args) =>
      new Promise(
        (resolve, reject) =>
          (window.admitSelection = () => {
            if (
              args.expectedGeneration !== undefined &&
              args.expectedGeneration !== window.catalog.generation
            ) {
              reject("model-root-stale");
              return;
            }
            window.catalog.current_model = args.modelId;
            resolve(null);
          }),
      );
  });
  await page.getByRole("button", { name: "Use model", exact: true }).click();
  await page.waitForFunction(() => typeof window.admitSelection === "function");
  await page.evaluate(() => {
    window.catalog = {
      ...window.catalog,
      generation: 2,
      root: {
        ...window.catalog.root,
        generation: 2,
        revision: 1,
        folder: "D:\\synthetic-before-admission",
      },
    };
    window.emitModelEvent("model-root-changed", { generation: 2 });
  });
  await page
    .getByText("D:\\synthetic-before-admission", { exact: true })
    .waitFor();
  await page.evaluate(() => window.admitSelection());
  await page
    .getByRole("button", { name: "Rescan models", exact: true })
    .click();
  await page.getByRole("button", { name: "Use model", exact: true }).waitFor();
  assert.equal(await page.getByText("Active", { exact: true }).count(), 0);
});

test("late download rejection cannot clear the same model's download progress in a successor folder", async (t) => {
  const initial = {
    models: [{ ...model, is_downloaded: false }],
    artifacts: {
      [model.id]: {
        status: "unavailable",
        can_delete: false,
        can_download: true,
      },
    },
  };
  const page = await fixture(t, "settings", "en", initial);
  await page.evaluate(() => {
    window.downloads = [];
    window.handlers.download_model = () =>
      new Promise((resolve, reject) =>
        window.downloads.push({ resolve, reject }),
      );
  });
  await page
    .getByRole("button", { name: "Download model", exact: true })
    .click();
  await page.waitForFunction(() => window.downloads.length === 1);
  await page.evaluate(() => {
    window.catalog = {
      ...window.catalog,
      generation: 2,
      root: {
        ...window.catalog.root,
        generation: 2,
        revision: 1,
        folder: "D:\\synthetic-new-download",
      },
    };
    window.emitModelEvent("model-root-changed", { generation: 2 });
  });
  await page.getByText("D:\\synthetic-new-download", { exact: true }).waitFor();
  await page
    .getByRole("button", { name: "Download model", exact: true })
    .click();
  await page.waitForFunction(() => window.downloads.length === 2);
  await page.evaluate(() => {
    window.emitModelEvent("model-download-progress", {
      generation: 2,
      model_id: "fixture-model",
      downloaded: 1048576,
      total: 4194304,
      percentage: 25,
    });
    window.downloads[0].reject(new Error("PRIVATE_OLD_DOWNLOAD"));
  });
  await page
    .getByText("1.0 MiB / 4.0 MiB downloaded", { exact: true })
    .waitFor();
  assert.equal(await page.getByRole("progressbar").getAttribute("value"), "25");
  assert.equal(await page.getByRole("alert").count(), 0);
});

test("a completed download in the returning-user shell never selects or loads a model", async (t) => {
  const page = await fixture(t, "app");
  await page
    .getByRole("button", { name: "Choose model folder", exact: true })
    .waitFor();
  await page.evaluate((model) => {
    window.catalog = {
      ...window.catalog,
      models: [model],
      artifacts: {
        [model.id]: {
          status: "verified",
          can_delete: false,
          can_download: false,
        },
      },
    };
    window.emitModelEvent("model-download-complete", {
      generation: 1,
      model_id: model.id,
    });
  }, model);
  await page.waitForTimeout(650);
  assert.deepEqual(
    await page.evaluate(() =>
      window.calls.filter((c) =>
        /is_recording|set_active_model|load_model/.test(c.cmd),
      ),
    ),
    [],
  );
});

test("a valid empty folder can be applied without replacing the requested model or acquiring files", async (t) => {
  const page = await fixture(t);
  await page
    .getByRole("button", { name: "Choose model folder", exact: true })
    .click();
  await page.getByRole("button", { name: "Apply folder", exact: true }).click();
  await page
    .getByRole("button", { name: "Apply folder", exact: true })
    .waitFor({ state: "detached" });
  await page.getByText("No models in this folder", { exact: true }).waitFor();
  assert.equal(
    await page.evaluate(() => window.modelStore.getState().currentModel),
    "remembered-model",
  );
  assert.deepEqual(
    await page.evaluate(() =>
      window.calls.filter((c) =>
        /download_model|delete_model|set_active_model|load_model/.test(c.cmd),
      ),
    ),
    [],
  );
});

test("custom unverified models stay distinct from invalid incomplete and unavailable retained artifacts", async (t) => {
  const statuses = [
    "custom_unverified",
    "invalid",
    "incomplete",
    "unavailable",
  ];
  const models = statuses.map((status) => ({
    ...model,
    id: status,
    name: status + " speech fixture",
    is_downloaded: status === "custom_unverified",
    is_custom: status === "custom_unverified",
  }));
  const artifacts = Object.fromEntries(
    statuses.map((status) => [
      status,
      { status, can_delete: false, can_download: false },
    ]),
  );
  const page = await fixture(t, "settings", "en", { models, artifacts });
  for (const text of [
    "Custom · unverified",
    "Invalid model",
    "Incomplete model",
    "Model unavailable",
  ])
    await page.getByText(text, { exact: true }).waitFor();
  assert.equal(
    await page.getByRole("button", { name: "Use model", exact: true }).count(),
    1,
  );
  assert.equal(await page.getByRole("button", { name: /Delete/ }).count(), 0);
  assert.equal(
    await page
      .getByRole("button", { name: "Download model", exact: true })
      .count(),
    0,
  );
});

test("folder-local Hugging Face models and read-only legacy cache models show their actual storage origins", async (t) => {
  const page = await fixture(t, "settings", "en", {
    root: {
      generation: 1,
      revision: 0,
      folder: "D:\\synthetic-root",
      status: "ready",
      writable: true,
      busy: false,
      model_count: 2,
    },
    models: [
      model,
      { ...model, id: "legacy-model", name: "Legacy speech fixture" },
    ],
    artifacts: {
      [model.id]: {
        status: "verified",
        source: "selected_root",
        can_delete: false,
        can_download: false,
      },
      "legacy-model": {
        status: "verified",
        source: "legacy_cache",
        can_delete: false,
        can_download: false,
      },
    },
  });
  const folder = page.getByRole("article", { name: model.name, exact: true });
  const legacy = page.getByRole("article", {
    name: "Legacy speech fixture",
    exact: true,
  });
  await folder.getByText("Storage: Model folder", { exact: true }).waitFor();
  await legacy
    .getByText("Storage: Read-only legacy cache", { exact: true })
    .waitFor();
  await page.evaluate(() => {
    window.handlers.get_model_path = (args) =>
      args.modelId === "legacy-model"
        ? "D:\\synthetic-legacy-cache\\model.gguf"
        : "D:\\synthetic-root\\.hf-cache\\hub\\model.gguf";
  });
  await folder.getByText("Advanced model information", { exact: true }).click();
  await folder
    .getByText("D:\\synthetic-root\\.hf-cache\\hub\\model.gguf", {
      exact: true,
    })
    .waitFor();
  await legacy.getByText("Advanced model information", { exact: true }).click();
  await legacy
    .getByText("D:\\synthetic-legacy-cache\\model.gguf", { exact: true })
    .waitFor();
  assert.equal(await legacy.getByRole("button", { name: /Delete/ }).count(), 0);
});

for (const component of ["settings", "onboarding"])
  test(`a retained legacy directory remains visible and unavailable in ${component} without repair or deletion`, async (t) => {
    const retained = {
      ...model,
      id: "retained-directory",
      name: "Retained directory fixture",
      filename: "legacy-speech",
      source: {
        Url: { url: "https://synthetic.invalid/archive", sha256: null },
      },
      is_directory: true,
      is_downloaded: false,
    };
    const page = await fixture(t, component, "en", {
      models: [retained],
      artifacts: {
        [retained.id]: {
          status: "unavailable",
          source: "selected_root",
          can_download: false,
          can_delete: false,
        },
      },
    });
    const card = page.getByRole("article", {
      name: retained.name,
      exact: true,
    });
    await card.getByText("Model unavailable", { exact: true }).waitFor();
    assert.equal(
      await card
        .getByRole("button", { name: "Use model", exact: true })
        .count(),
      0,
    );
    assert.equal(
      await card
        .getByRole("button", { name: "Download model", exact: true })
        .count(),
      0,
    );
    assert.equal(await card.getByRole("button", { name: /Delete/ }).count(), 0);
    assert.deepEqual(
      await page.evaluate(() =>
        window.calls.filter((c) =>
          /download_model|delete_model|set_active_model/.test(c.cmd),
        ),
      ),
      [],
    );
  });

test("Show folder uses the generation-bound native action, safely retries and rejects stale failures", async (t) => {
  const page = await fixture(t, "settings");
  await page.evaluate(() => {
    window.handlers.show_model_root = () => {
      throw new Error("PRIVATE_FOLDER_OPENER_FAILURE");
    };
  });
  await page.getByRole("button", { name: "Show folder", exact: true }).click();
  await page
    .getByRole("alert")
    .getByText("Unable to access the model folder. Please try again.", {
      exact: true,
    })
    .waitFor();
  assert.doesNotMatch(
    await page.locator("body").innerText(),
    /PRIVATE_FOLDER_OPENER_FAILURE/,
  );
  assert.deepEqual(
    await page.evaluate(() =>
      window.calls
        .filter((c) => c.cmd === "show_model_root")
        .map((c) => c.args),
    ),
    [{ expectedGeneration: 1 }],
  );
  await page.evaluate(() => {
    window.handlers.show_model_root = () => null;
  });
  await page.getByRole("button", { name: "Show folder", exact: true }).click();
  await page.getByRole("alert").waitFor({ state: "detached" });
  await page.evaluate(() => {
    window.handlers.show_model_root = () =>
      new Promise((resolve, reject) => (window.rejectOldShow = reject));
  });
  await page.getByRole("button", { name: "Show folder", exact: true }).click();
  await page.waitForFunction(() => typeof window.rejectOldShow === "function");
  await page.evaluate(() => {
    window.catalog = {
      ...window.catalog,
      generation: 2,
      root: {
        ...window.catalog.root,
        generation: 2,
        revision: 1,
        folder: "D:\\synthetic-show-successor",
      },
    };
    window.emitModelEvent("model-root-changed", { generation: 2 });
  });
  await page
    .getByText("D:\\synthetic-show-successor", { exact: true })
    .waitFor();
  await page.evaluate(() => window.rejectOldShow("model-root-unavailable"));
  assert.equal(await page.getByRole("alert").count(), 0);
  assert.deepEqual(
    await page.evaluate(() =>
      window.calls.filter((c) => c.cmd === "plugin:opener|open_path"),
    ),
    [],
  );
});
