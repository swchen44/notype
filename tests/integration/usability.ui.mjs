import assert from "node:assert/strict";
import test from "node:test";
import { readFile } from "node:fs/promises";
import { uiFixture } from "./fixtures/ui.mjs";
import { historyApiFixture, historyEntry } from "./fixtures/history-api.mjs";
import {
  providerConnectionsFixture,
  savedConnection,
} from "./fixtures/provider-connections.mjs";

async function dictionaryFixture(t, language = "en") {
  const page = await uiFixture(t, {
    cacheName: "dictionary-usability",
    entry: `export {default as React} from 'react'; export {createRoot} from 'react-dom/client'; export {DictionarySettings} from '/src/components/settings/DictionarySettings.tsx'; export {Sidebar} from '/src/components/Sidebar.tsx'; export {default as i18n} from '/src/i18n/index.ts'; import '/src/App.css';`,
  });
  page.setDefaultTimeout(5000);
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  t.after(() => assert.deepEqual(errors, []));
  await page.evaluate(async (language) => {
    window.calls = [];
    window.settings = {
      custom_words: ["OpenAI", "繁體中文", "Docker"],
      app_language: language,
      post_process_enabled: false,
    };
    window.mode = "ok";
    window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener() {} };
    window.__TAURI_INTERNALS__ = {
      transformCallback: () => 1,
      invoke: async (cmd, args) => {
        window.calls.push({ cmd, args });
        if (cmd === "plugin:event|listen") return 1;
        if (cmd === "plugin:event|unlisten") return null;
        if (cmd === "get_app_settings" || cmd === "get_default_settings")
          return window.settings;
        if (cmd === "is_update_checks_locked") return false;
        if (cmd === "check_custom_sounds") return { start: false, stop: false };
        if (cmd === "get_text_configuration")
          return {
            schema_version: 2,
            revision: 0,
            connections: [],
            active_connection_id: null,
          };
        if (cmd === "get_connection_activity") return [];
        if (cmd === "update_custom_words") {
          if (window.mode === "save-fail")
            throw "Bearer SYNTHETIC_PRIVATE_REJECTION";
          if (window.mode === "save-pending")
            await new Promise((resolve) => (window.releaseSave = resolve));
          window.settings = { ...window.settings, custom_words: args.words };
          return null;
        }
        throw new Error("Unexpected fixture IPC: " + cmd);
      },
    };
    const { React, createRoot, DictionarySettings, Sidebar, i18n } =
      await import("/__ui_fixture.tsx");
    await i18n.changeLanguage(language);
    const root = createRoot(document.getElementById("root"));
    root.render(
      React.createElement(
        "div",
        { className: "flex min-h-screen" },
        React.createElement(Sidebar, {
          activeSection: "dictionary",
          onSectionChange: () => {},
        }),
        React.createElement(
          "main",
          { className: "p-4 min-w-0 flex-1" },
          React.createElement(DictionarySettings),
        ),
      ),
    );
  }, language);
  await page.getByRole("table").waitFor();
  return page;
}

test("dictionary navigation, table CRUD, case-insensitive duplicates and visible selection", async (t) => {
  const page = await dictionaryFixture(t);
  const sidebar = page.locator(".notype-sidebar");
  const names = await sidebar.getByRole("button").allTextContents();
  assert.equal(names.indexOf("Advanced") + 1, names.indexOf("About"));
  await sidebar
    .getByRole("button", { name: "Dictionary", exact: true })
    .waitFor();
  const status = sidebar.getByRole("region", {
    name: "Text processing status",
  });
  await status.getByText("AI processing: off", { exact: true }).waitFor();
  await page.getByRole("button", { name: "Add word", exact: true }).click();
  const dialog = page.getByRole("dialog");
  await dialog.getByLabel("Word", { exact: true }).fill(" openai ");
  await dialog.getByRole("button", { name: "Save", exact: true }).click();
  await dialog
    .getByText("This word already exists (ignoring case).", { exact: true })
    .waitFor();
  assert.equal(
    await page.evaluate(
      () => window.calls.filter((c) => c.cmd === "update_custom_words").length,
    ),
    0,
  );
  await dialog.getByLabel("Word", { exact: true }).fill(" Mermaid ");
  await dialog.getByRole("button", { name: "Save", exact: true }).click();
  await page.getByRole("cell", { name: "Mermaid", exact: true }).waitFor();
  const row = page
    .getByRole("row")
    .filter({ has: page.getByRole("cell", { name: "Mermaid", exact: true }) });
  await row.getByRole("button", { name: "Edit", exact: true }).click();
  await dialog.getByLabel("Word", { exact: true }).fill("DOCKER");
  await dialog.getByRole("button", { name: "Save", exact: true }).click();
  await dialog
    .getByText("This word already exists (ignoring case).", { exact: true })
    .waitFor();
  await dialog.getByLabel("Word", { exact: true }).fill("mermaid.js");
  await dialog.getByRole("button", { name: "Save", exact: true }).click();
  await page.getByRole("cell", { name: "mermaid.js", exact: true }).waitFor();
  await page.getByRole("searchbox", { name: "Search words" }).fill("MERMAID");
  assert.equal(await page.getByRole("row").count(), 2);
  await page
    .getByRole("checkbox", { name: "Select all visible words" })
    .check();
  await page
    .getByRole("button", { name: "Delete 1 selected words", exact: true })
    .click();
  await page.getByText("No matching words.", { exact: true }).waitFor();
  assert.deepEqual(await page.evaluate(() => window.settings.custom_words), [
    "OpenAI",
    "繁體中文",
    "Docker",
  ]);
});

test("dictionary UTF-8 CSV import/export preserves words, skips duplicates and rejects malformed files atomically", async (t) => {
  const page = await dictionaryFixture(t);
  await page.locator("input[type=file]").setInputFiles({
    name: "terms.csv",
    mimeType: "text/csv",
    buffer: Buffer.from(
      '\ufeffword\r\n"openai"\r\n"简体词"\r\n"name, value"\r\n"a""b"\r\n',
      "utf8",
    ),
  });
  await page
    .getByText("Added 3 words; skipped 1 duplicates.", { exact: true })
    .waitFor();
  const words = await page.evaluate(() => window.settings.custom_words);
  assert.deepEqual(words, [
    "OpenAI",
    "繁體中文",
    "Docker",
    "简体词",
    "name, value",
    'a"b',
  ]);
  const callsBefore = await page.evaluate(
    () => window.calls.filter((c) => c.cmd === "update_custom_words").length,
  );
  await page.locator("input[type=file]").setInputFiles({
    name: "bad.csv",
    mimeType: "text/csv",
    buffer: Buffer.from('word\r\nokay\r\n"unterminated', "utf8"),
  });
  await page
    .getByText(
      "Invalid CSV. Use UTF-8, one word column, and words of 1–50 characters.",
      { exact: true },
    )
    .waitFor();
  assert.equal(
    await page.evaluate(
      () => window.calls.filter((c) => c.cmd === "update_custom_words").length,
    ),
    callsBefore,
  );
  assert.deepEqual(
    await page.evaluate(() => window.settings.custom_words),
    words,
  );
  const download = page.waitForEvent("download");
  await page.getByRole("button", { name: "Export CSV", exact: true }).click();
  const file = await download;
  assert.equal(file.suggestedFilename(), "notype-dictionary.csv");
  const bytes = await readFile(await file.path());
  assert.deepEqual([...bytes.subarray(0, 3)], [239, 187, 191]);
  assert.match(bytes.toString("utf8"), /"简体词"/);
  assert.match(bytes.toString("utf8"), /"a""b"/);
});

test("dictionary save failure keeps the editor and existing words, pending save prevents duplicate mutation", async (t) => {
  const page = await dictionaryFixture(t);
  await page.getByRole("button", { name: "Add word", exact: true }).click();
  const dialog = page.getByRole("dialog");
  await dialog.getByLabel("Word", { exact: true }).fill("New word");
  await page.evaluate(() => (window.mode = "save-fail"));
  await dialog.getByRole("button", { name: "Save", exact: true }).click();
  await dialog
    .getByText("Could not save. Your changes have not been confirmed.", {
      exact: true,
    })
    .waitFor();
  assert.deepEqual(await page.evaluate(() => window.settings.custom_words), [
    "OpenAI",
    "繁體中文",
    "Docker",
  ]);
  assert.doesNotMatch(
    await page.locator("body").innerText(),
    /SYNTHETIC_PRIVATE|Bearer/,
  );
  await page.evaluate(() => (window.mode = "save-pending"));
  await dialog.getByRole("button", { name: "Save", exact: true }).click();
  await page.waitForFunction(() => typeof window.releaseSave === "function");
  assert.equal(
    await dialog
      .getByRole("button", { name: "Save", exact: true })
      .isDisabled(),
    true,
  );
  assert.equal(
    await dialog.getByLabel("Word", { exact: true }).isDisabled(),
    true,
  );
  await page.evaluate(() => window.releaseSave());
  await page.getByRole("cell", { name: "New word", exact: true }).waitFor();
});

test("history starts with timestamp and primary text, literal search hides originals and highlights matches", async (t) => {
  const page = await historyApiFixture(t);
  await page.evaluate((entry) => {
    window.entries = [
      { ...entry, result: "Edited C++ 與 c++" },
      { ...entry, id: "raw-only", result: null, transcript: "Raw only 中文" },
    ];
    window.render();
  }, historyEntry);
  const article = page.getByRole("article").first();
  await article
    .locator(":scope > p")
    .getByText("Edited C++ 與 c++", { exact: true })
    .waitFor();
  assert.equal(
    await article.locator(".history-advanced").getAttribute("open"),
    null,
  );
  assert.equal(
    await article.getByText("Original count: 91", { exact: true }).isVisible(),
    false,
  );
  assert.equal(await article.locator(":scope > time").count(), 1);
  await page.getByRole("searchbox").fill("c++");
  await page.waitForFunction(
    () => document.querySelectorAll("article").length === 1,
  );
  assert.deepEqual(await page.locator("article > p mark").allTextContents(), [
    "C++",
    "c++",
  ]);
  await page.getByRole("searchbox").fill("Raw text has different");
  await page.getByText("No matching text history", { exact: true }).waitFor();
  await page.getByRole("searchbox").fill("中文");
  await page
    .getByRole("article")
    .locator(":scope > p")
    .getByText("Raw only 中文", { exact: true })
    .waitFor();
  assert.deepEqual(await page.locator("article > p mark").allTextContents(), [
    "中文",
  ]);
});

test("history Advanced retains the AI failure separately when raw delivery targets have changed", async (t) => {
  const page = await historyApiFixture(t);
  await page.evaluate((entry) => {
    window.entries = [
      {
        ...entry,
        result: null,
        error: "target-changed",
        status: "pending",
        selection: "original",
        observations: {
          ...entry.observations,
          latest_processing_error: "provider-timeout",
        },
      },
    ];
    window.render();
  }, historyEntry);
  const article = page.getByRole("article");
  const failure = article.getByRole("note", {
    name: "Text processing error",
    exact: true,
  });
  assert.equal(await failure.isVisible(), false);
  await article.locator(":scope > details > summary").click();
  await failure
    .getByText(
      "Service timed out. The request may already have been charged; check before explicitly retrying. The original is saved.",
      { exact: true },
    )
    .waitFor();
  await article
    .getByText("The original target changed. The result remains pending.", {
      exact: true,
    })
    .waitFor();
});

test("connection test is explicit, counted per connection, reset retains history and sidebar reflects latest real test", async (t) => {
  const second = {
    ...savedConnection,
    id: "connection-b",
    display_name: "Other server",
  };
  const page = await providerConnectionsFixture(
    t,
    "en",
    [savedConnection, second],
    savedConnection.id,
    "ok",
    true,
  );
  const rows = page
    .getByRole("list", { name: "Saved connections" })
    .getByRole("listitem");
  await rows
    .first()
    .getByRole("button", { name: "Test connection", exact: true })
    .waitFor();
  const sidebar = page.getByRole("region", { name: "Text processing status" });
  await sidebar.getByText("Connection not tested", { exact: true }).waitFor();
  assert.equal(
    await page.evaluate(
      () => window.calls.filter((c) => c.cmd === "test_text_connection").length,
    ),
    0,
  );
  await page
    .getByRole("checkbox", { name: "Enable AI text processing", exact: true })
    .locator("..")
    .click();
  await sidebar.getByText("AI processing: on", { exact: true }).waitFor();
  await page.evaluate(() => (window.testMode = "pending"));
  await rows
    .first()
    .getByRole("button", { name: "Test connection", exact: true })
    .click();
  await page.waitForFunction(() => typeof window.releaseTest === "function");
  assert.equal(
    await rows
      .first()
      .getByRole("button", { name: "Testing…", exact: true })
      .isDisabled(),
    true,
  );
  assert.equal(
    await rows
      .first()
      .getByRole("button", { name: "Clear usage counters", exact: true })
      .isDisabled(),
    true,
  );
  await page.evaluate(() => {
    window.testMode = "ok";
    window.releaseTest();
  });
  await rows
    .first()
    .getByText("Input tokens: 13 · Output tokens: 2", { exact: true })
    .waitFor();
  await rows
    .first()
    .getByText("Actual fees: Unknown", { exact: true })
    .waitFor();
  await sidebar.getByText("Last test succeeded", { exact: true }).waitFor();
  await rows
    .last()
    .getByText("Input tokens: 0 · Output tokens: 0", { exact: true })
    .waitFor();
  await rows
    .first()
    .getByRole("button", { name: "Clear usage counters", exact: true })
    .click();
  await rows
    .first()
    .getByText("Usage counters cleared. History is retained.", { exact: true })
    .waitFor();
  await rows
    .first()
    .getByText("Input tokens: 0 · Output tokens: 0", { exact: true })
    .waitFor();
  assert.equal(
    await page.evaluate(() =>
      window.calls.some((c) => c.cmd === "delete_history_entry"),
    ),
    false,
  );
  await page
    .getByRole("radio", { name: "Use Other server", exact: true })
    .check();
  await sidebar.getByText("Other server", { exact: true }).waitFor();
  await sidebar.getByText("Connection not tested", { exact: true }).waitFor();
  assert.equal(await page.getByRole("radio", { checked: true }).count(), 1);
  await page.evaluate(() => (window.testMode = "fail"));
  await rows
    .last()
    .getByRole("button", { name: "Test connection", exact: true })
    .click();
  await sidebar.getByText("Last test failed", { exact: true }).waitFor();
  await page
    .getByRole("checkbox", { name: "Enable AI text processing", exact: true })
    .locator("..")
    .click();
  await sidebar.getByText("AI processing: off", { exact: true }).waitFor();
  assert.equal(
    await page.evaluate(
      () => window.calls.filter((c) => c.cmd === "test_text_connection").length,
    ),
    2,
  );
});

test("saved keys and replacement input show masked head/tail without exposing full key", async (t) => {
  const page = await providerConnectionsFixture(t, "en", [
    {
      ...savedConnection,
      authentication: "bearer",
      app_key_present: true,
      credential_source: "app",
      masked_api_key: "abcd****wxyz",
    },
  ]);
  await page.getByText("Saved key: abcd****wxyz", { exact: true }).waitFor();
  await page
    .getByRole("button", { name: "Edit Company writing", exact: true })
    .click();
  await page
    .getByLabel("App key action", { exact: true })
    .selectOption("replace");
  const input = page.getByLabel("Replacement API key", { exact: true });
  await input.fill("head-private-middle-tail");
  assert.equal(await input.getAttribute("type"), "password");
  await page
    .getByLabel("Key preview", { exact: true })
    .getByText("head****tail", { exact: true })
    .waitFor();
  assert.doesNotMatch(await page.locator("body").innerText(), /private-middle/);
});

test("AI enable changes refresh readiness without discarding an unsaved connection edit", async (t) => {
  const page = await providerConnectionsFixture(
    t,
    "en",
    [savedConnection],
    savedConnection.id,
  );
  await page
    .getByRole("button", { name: "Edit Company writing", exact: true })
    .click();
  const model = page.getByRole("textbox", {
    name: "Model ID (manual entry)",
    exact: true,
  });
  await model.fill("unsaved-model-draft");
  const toggle = page
    .getByRole("checkbox", { name: "Enable AI text processing", exact: true })
    .locator("..");
  await page.getByText("AI processing: off", { exact: true }).waitFor();
  await toggle.click();
  await page.getByText("AI settings are complete.", { exact: true }).waitFor();
  assert.equal(await model.inputValue(), "unsaved-model-draft");
  await toggle.click();
  await page.getByText("AI processing: off", { exact: true }).waitFor();
  assert.equal(
    await page.getByText("AI settings are complete.", { exact: true }).count(),
    0,
  );
  assert.equal(await model.inputValue(), "unsaved-model-draft");
  assert.equal(
    await page.evaluate(() =>
      window.calls.some((c) => c.cmd === "test_text_connection"),
    ),
    false,
  );
});

test("connection counters preserve exact currencies and partial data, failed clear retains usage", async (t) => {
  const page = await providerConnectionsFixture(
    t,
    "en",
    [savedConnection],
    savedConnection.id,
  );
  await page
    .getByRole("button", { name: "Test connection", exact: true })
    .waitFor();
  await page
    .getByText(
      "Usage recorded by this app since tracking began or the last reset, including connection tests.",
      { exact: true },
    )
    .waitFor();
  await page.evaluate((id) => {
    window.usage[id] = {
      call_count: 2,
      observed_call_count: 2,
      input: {
        status: "partial",
        total: null,
        known_subtotal: 0,
        reported_calls: 1,
      },
      output: {
        status: "unknown",
        total: null,
        known_subtotal: null,
        reported_calls: 0,
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
      requested_models: [],
      reported_models: [],
      model_efforts: [],
    };
    window.dispatchEvent(new Event("notype-text-configuration-changed"));
  }, savedConnection.id);
  await page
    .getByText("Input tokens: 0 + Unknown · Output tokens: Unknown", {
      exact: true,
    })
    .waitFor();
  await page
    .getByText("Actual fees: USD 0.100000000000000001 + TWD 2.50 + Unknown", {
      exact: true,
    })
    .waitFor();
  await page.evaluate(() => (window.testMode = "clear-fail"));
  await page
    .getByRole("button", { name: "Clear usage counters", exact: true })
    .click();
  await page
    .getByText("Unable to clear usage counters.", { exact: true })
    .waitFor();
  await page
    .getByText("Input tokens: 0 + Unknown · Output tokens: Unknown", {
      exact: true,
    })
    .waitFor();
  assert.doesNotMatch(
    await page.locator("body").innerText(),
    /synthetic-private-clear-error/,
  );
});

for (const [language, heading, search, empty] of [
  ["zh-TW", "字典", "搜尋詞彙", "沒有符合的詞彙。"],
  ["zh-CN", "词典", "搜索词汇", "没有符合的词汇。"],
])
  test(`dictionary ${language} remains translated and fits a narrow viewport`, async (t) => {
    const page = await dictionaryFixture(t, language);
    await page.setViewportSize({ width: 540, height: 900 });
    await page.getByRole("heading", { name: heading, exact: true }).waitFor();
    await page
      .getByRole("searchbox", { name: search, exact: true })
      .fill("not present");
    await page.getByText(empty, { exact: true }).waitFor();
    assert.equal(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= window.innerWidth,
      ),
      true,
    );
    assert.doesNotMatch(
      await page.locator("body").innerText(),
      /notype\.usability|notype\.connections/,
    );
  });
