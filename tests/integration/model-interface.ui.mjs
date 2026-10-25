import assert from "node:assert/strict";
import test from "node:test";
import { desktop, uiFixture } from "./fixtures/ui.mjs";

async function fixture(t, component = "card") {
  const page = await uiFixture(t);
  await page.evaluate(async (component) => {
    window.downloadCalls = [];
    window.__TAURI_INTERNALS__ = {
      invoke: async (cmd, args) =>
        cmd === "get_model_path"
          ? "D:\\fixture-cache\\model.gguf"
          : cmd === "get_app_settings"
            ? { app_language: "en" }
            : null,
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
      settings: { recognition_mode: "traditional_english" },
      isLoading: false,
    });
    const languages = (
      await import("/src/lib/constants/languages.ts")
    ).MODEL_CAPABILITY_LANGUAGES.map((l) => l.value);
    const model = {
      id: "fixture-unknown",
      name: "Qwen fixture",
      description: "Speech recognition fixture.",
      filename: "model-Q6_K.gguf",
      source: {
        HuggingFace: {
          repo_id: "fixture/repository",
          revision: "test-revision",
        },
      },
      size_mb: 659,
      is_downloaded: false,
      is_downloading: false,
      partial_size: 0,
      is_directory: false,
      engine_type: "Whisper",
      accuracy_score: 0.8,
      speed_score: 0.6,
      supports_translation: false,
      is_recommended: true,
      supported_languages: languages,
      supports_language_selection: true,
      is_custom: false,
      supports_streaming: true,
      supports_language_detection: true,
    };
    window.fixtureModel = model;
    if (component === "card") {
      const { default: ModelCard } = await import(
        "/src/components/onboarding/ModelCard.tsx"
      );
      window.renderCard = (props) =>
        window.fixtureRoot.render(
          React.createElement(ModelCard, {
            model,
            onSelect: () => {},
            onDownload: (id) => window.downloadCalls.push(id),
            ...props,
          }),
        );
      window.fixtureRoot = createRoot(document.getElementById("root"));
      window.renderCard({});
    } else {
      const { useModelStore } = await import("/src/stores/modelStore.ts");
      useModelStore.setState({
        loading: false,
        models: [
          model,
          {
            ...model,
            id: "speedy",
            name: "Faster fixture",
            accuracy_score: 0.4,
            speed_score: 0.9,
          },
          {
            ...model,
            id: "unscored",
            name: "Unscored fixture",
            accuracy_score: 0,
            speed_score: 0,
          },
        ],
      });
      const { ModelsSettings } = await import(
        "/src/components/settings/models/ModelsSettings.tsx"
      );
      let content = React.createElement(ModelsSettings);
      if (component === "shell") {
        const { Sidebar } = await import("/src/components/Sidebar.tsx");
        content = React.createElement(
          "div",
          { className: "h-screen flex" },
          React.createElement(Sidebar, {
            activeSection: "models",
            onSectionChange: () => {},
          }),
          React.createElement(
            "main",
            { className: "settings-content min-w-0 flex-1 overflow-y-auto" },
            content,
          ),
        );
      }
      createRoot(document.getElementById("root")).render(content);
    }
  }, component);
  await page
    .getByRole("heading", { name: "Qwen fixture", exact: true })
    .waitFor();
  return page;
}

test("model advanced information exposes every metadata language, source, and real path without starting a download", async (t) => {
  const page = await fixture(t);
  await page
    .getByText("Advanced model information", { exact: true })
    .click({ timeout: 3000 });
  await page
    .getByText(
      "The actual local path is available after download and verification.",
      { exact: true },
    )
    .waitFor({ timeout: 3000 });
  await page.evaluate(() =>
    window.renderCard({
      model: { ...window.fixtureModel, is_downloaded: true },
      status: "available",
    }),
  );
  await page
    .getByText("D:\\fixture-cache\\model.gguf", { exact: true })
    .waitFor();
  assert.match(await page.locator("details").innerText(), /Sundanese/);
  assert.match(
    await page.locator("details").innerText(),
    /fixture\/repository/,
  );
  assert.match(await page.locator("details").innerText(), /test-revision/);
  assert.match(await page.locator("details").innerText(), /Q6_K/);
  assert.deepEqual(await page.evaluate(() => window.downloadCalls), []);
});

test("catalog scores remain relative and sort descending with unscored models last", async (t) => {
  const page = await fixture(t, "settings");
  await page
    .getByRole("button", { name: "Sort by speed", exact: true })
    .click({ timeout: 3000 });
  assert.deepEqual(await page.locator("article h3").allTextContents(), [
    "Faster fixture",
    "Qwen fixture",
    "Unscored fixture",
  ]);
  assert.match(await page.locator("article").first().innerText(), /0\.90/);
  assert.match(await page.locator("article").last().innerText(), /Not rated/);
  assert.match(
    await page.locator("body").innerText(),
    /Not accuracy percentages/,
  );
  await page
    .getByRole("button", { name: "Sort by accuracy", exact: true })
    .click();
  assert.deepEqual(await page.locator("article h3").allTextContents(), [
    "Qwen fixture",
    "Faster fixture",
    "Unscored fixture",
  ]);
});

test("quality receipt opener failures are translated and a retry clears the error", async (t) => {
  const page = await fixture(t);
  const unhandled = [];
  page.on("pageerror", (error) => unhandled.push(error.message));
  await page.evaluate(() => {
    window.renderCard({model: {...window.fixtureModel, filename: "Qwen3-ASR-0.6B-Q6_K.gguf"}});
    window.__TAURI_INTERNALS__.invoke = async () => { throw new Error("synthetic private opener failure"); };
  });
  await page.getByText("Advanced model information", {exact: true}).click();
  await page.getByRole("button", {name: "Read the Qwen verification receipt"}).click({timeout: 3000});
  await page.getByRole("alert").filter({hasText: "Unable to open the quality report. Please try again."}).waitFor({timeout: 3000});
  assert.doesNotMatch(await page.locator("body").innerText(), /synthetic private/);
  assert.deepEqual(unhandled, []);
  await page.evaluate(() => { window.__TAURI_INTERNALS__.invoke = async () => null; });
  await page.getByRole("button", {name: "Read the Qwen verification receipt"}).click({timeout: 3000});
  await page.getByRole("alert").waitFor({state: "detached", timeout: 3000});
});

test("download UI shows actual transferred bytes and preserves verification, extraction, and missing-path states", async (t) => {
  const page = await fixture(t);
  await page.evaluate(() =>
    window.renderCard({
      status: "downloading",
      downloadProgress: 25,
      downloadedBytes: 1048576,
      totalBytes: 4194304,
      downloadSpeed: 2,
    }),
  );
  await page
    .getByText("1.0 MiB / 4.0 MiB downloaded", { exact: true })
    .waitFor({ timeout: 3000 });
  assert.equal(await page.getByRole("progressbar").getAttribute("value"), "25");
  await page.evaluate(() => window.renderCard({ status: "verifying" }));
  await page.getByRole("status").waitFor();
  assert.match(await page.locator("article").innerText(), /Verifying/);
  await page.evaluate(() => window.renderCard({ status: "extracting" }));
  assert.match(await page.locator("article").innerText(), /Extracting/);
  await page.evaluate(() => {
    window.__TAURI_INTERNALS__.invoke = async () => {
      throw new Error("unavailable");
    };
    window.renderCard({
      model: { ...window.fixtureModel, is_downloaded: true },
      status: "downloading",
    });
  });
  await page.getByText("Advanced model information", { exact: true }).click();
  await page
    .getByText("Location unavailable. Expand again to retry.", { exact: true })
    .waitFor();
  await page.getByText("Waiting for download size…", { exact: true }).waitFor();
});

test("model explorer remains usable in narrow light and dark windows with both Chinese interfaces", async (t) => {
  const page = await fixture(t, "shell");
  for (const [locale, theme, advanced, profile] of [
    ["zh-TW", "light", "進階模型資訊", "繁體中文"],
    ["zh-CN", "dark", "进阶模型信息", "简体中文"],
  ]) {
    await page.setViewportSize({ width: 440, height: 900 });
    await page.evaluate(
      async ({ locale, theme }) => {
        const { default: i18n } = await import("/src/i18n/index.ts");
        const { useSettingsStore } = await import(
          "/src/stores/settingsStore.ts"
        );
        useSettingsStore.setState({
          settings: {
            recognition_mode:
              locale === "zh-CN" ? "simplified_english" : "traditional_english",
          },
        });
        await i18n.changeLanguage(locale);
        document.documentElement.dataset.theme = theme;
      },
      { locale, theme },
    );
    const disclosure = page.getByText(advanced, { exact: true }).first();
    await disclosure.waitFor();
    if (!(await page.locator("details").first().getAttribute("open"))) {
      const open = await page
        .locator("details")
        .first()
        .evaluate((el) => el.open);
      if (!open) await disclosure.click();
    }
    await page
      .locator("details")
      .first()
      .getByText("model-Q6_K.gguf", { exact: true })
      .waitFor();
    assert.match(
      await page.locator("article").first().innerText(),
      new RegExp(profile),
    );
    assert.equal(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= window.innerWidth,
      ),
      true,
    );
    const text = await page.locator("article").first().innerText();
    assert.doesNotMatch(
      text,
      /目前無法顯示這段文字|notype\.modelExplorer|\?{3}/,
    );
    await page.screenshot({
      path: `${desktop}/../.local/model-explorer-${locale}-${theme}-440.png`,
      fullPage: true,
    });
  }
  await page.setViewportSize({ width: 1080, height: 900 });
  await page.evaluate(() => {
    document.documentElement.dataset.theme = "light";
  });
  await page.screenshot({
    path: `${desktop}/../.local/model-explorer-wide.png`,
    fullPage: true,
  });
});
