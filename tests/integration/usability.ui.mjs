import assert from "node:assert/strict";
import test from "node:test";
import { readFile } from "node:fs/promises";
import { uiFixture } from "./fixtures/ui.mjs";
import { historyApiFixture, historyEntry } from "./fixtures/history-api.mjs";
import {
  providerConnectionsFixture,
  savedConnection,
} from "./fixtures/provider-connections.mjs";

async function dictionaryFixture(
  t,
  language = "en",
  words = ["OpenAI", "繁體中文", "Docker"],
) {
  const page = await uiFixture(t, {
    cacheName: "dictionary-usability",
    entry: `export {default as React} from 'react'; export {createRoot} from 'react-dom/client'; export {DictionarySettings} from '/src/components/settings/DictionarySettings.tsx'; export {Sidebar} from '/src/components/Sidebar.tsx'; export {default as i18n} from '/src/i18n/index.ts'; import '/src/App.css';`,
  });
  page.setDefaultTimeout(5000);
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  t.after(() => assert.deepEqual(errors, []));
  await page.evaluate(
    async ({ language, words }) => {
      window.calls = [];
      window.settings = {
        custom_words: words,
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
          if (cmd === "check_custom_sounds")
            return { start: false, stop: false };
          if (cmd === "get_text_configuration")
            return (
              window.configuration ?? {
                schema_version: 2,
                revision: 0,
                connections: [],
                active_connection_id: null,
              }
            );
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
          {
            className: "flex overflow-hidden",
            style: { height: "calc(100vh - 32px)" },
          },
          React.createElement(Sidebar, {
            activeSection: "dictionary",
            onSectionChange: () => {},
          }),
          React.createElement(
            "main",
            { className: "p-4 min-w-0 flex-1 overflow-y-auto" },
            React.createElement(DictionarySettings),
          ),
        ),
      );
    },
    { language, words },
  );
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

test("sidebar keeps readable labels and the complete connection status inside short and narrow windows", async (t) => {
  const page = await dictionaryFixture(t);
  await page.evaluate(
    (connection) => {
      window.configuration = {
        schema_version: 2,
        revision: 0,
        connections: [connection],
        active_connection_id: connection.id,
      };
      window.dispatchEvent(new Event("notype-text-configuration-changed"));
    },
    {
      ...savedConnection,
      display_name: "Internal writing service",
      effective_endpoint: "https://intranet.example.test/v1/chat/completions",
      model: "qwen/language-model-2026",
    },
  );
  const sidebar = page.locator(".notype-sidebar");
  const footer = sidebar.getByRole("region", {
    name: "Text processing status",
    exact: true,
  });
  await footer.getByText("Internal writing service", { exact: true }).waitFor();
  for (const width of [420, 650, 580]) {
    await page.setViewportSize({ width, height: 600 });
    const bounds = await footer.evaluate((element) => {
      const sidebar = element.closest(".notype-sidebar");
      const panel = sidebar.getBoundingClientRect(),
        status = element.getBoundingClientRect();
      return {
        width: panel.width,
        bottom: status.bottom,
        sidebarBottom: panel.bottom,
        inside: status.top >= panel.top && status.bottom <= panel.bottom + 1,
        linesFit: [...element.querySelectorAll("p")].every(
          (line) => line.scrollWidth <= line.clientWidth + 1,
        ),
        overflow: document.documentElement.scrollWidth > window.innerWidth,
      };
    });
    assert.equal(bounds.inside, true, JSON.stringify(bounds));
    assert.ok(
      bounds.width >= 160,
      `sidebar width ${bounds.width} at viewport ${width}`,
    );
    assert.equal(
      await sidebar.locator(".sidebar-label").first().isVisible(),
      true,
    );
    assert.ok(bounds.bottom <= 568 + 1, JSON.stringify(bounds));
    assert.equal(bounds.linesFit, true);
    assert.equal(bounds.overflow, false);
    await footer.getByText("AI processing: off", { exact: true }).waitFor();
    await footer.getByText("Connection not tested", { exact: true }).waitFor();
  }
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

test("large dictionaries page 100 rows, search all terms, deduplicate across pages and export all words", async (t) => {
  const words = Array.from(
    { length: 250 },
    (_, i) => `term-${String(i + 1).padStart(3, "0")}`,
  );
  const page = await dictionaryFixture(t, "en", words);
  assert.equal(await page.locator("tbody tr").count(), 100);
  await page
    .getByText("Page 1 of 3 · 250 matches / 250 words", { exact: true })
    .waitFor();
  const next = page.getByRole("button", { name: "Next page", exact: true });
  await next.click();
  await page.getByRole("cell", { name: "term-101", exact: true }).waitFor();
  await page
    .getByRole("checkbox", { name: "Select all visible words", exact: true })
    .check();
  await page
    .getByRole("button", { name: "Delete 100 selected words", exact: true })
    .waitFor();
  await next.click();
  assert.equal(await page.locator("tbody tr").count(), 50);
  assert.equal(
    await page
      .getByRole("checkbox", { name: "Select all visible words", exact: true })
      .isChecked(),
    false,
  );
  await page.getByRole("button", { name: "Add word", exact: true }).click();
  const dialog = page.getByRole("dialog");
  await dialog.getByLabel("Word", { exact: true }).fill("TERM-001");
  await dialog.getByRole("button", { name: "Save", exact: true }).click();
  await dialog
    .getByText("This word already exists (ignoring case).", { exact: true })
    .waitFor();
  await dialog.getByRole("button", { name: "Cancel", exact: true }).click();
  await page
    .getByRole("searchbox", { name: "Search words", exact: true })
    .fill("TERM-250");
  await page
    .getByText("Page 1 of 1 · 1 matches / 250 words", { exact: true })
    .waitFor();
  await page.getByRole("cell", { name: "term-250", exact: true }).waitFor();
  const waiting = page.waitForEvent("download");
  await page.getByRole("button", { name: "Export CSV", exact: true }).click();
  const download = await waiting;
  const csv = (await readFile(await download.path())).toString("utf8");
  assert.equal(csv.trim().split("\r\n").length, 251);
  assert.match(csv, /"term-001"/);
  assert.match(csv, /"term-250"/);
  await page
    .getByRole("searchbox", { name: "Search words", exact: true })
    .fill("");
  await page
    .getByRole("button", { name: "Delete 100 selected words", exact: true })
    .click();
  await page
    .getByText("Page 1 of 2 · 150 matches / 150 words", { exact: true })
    .waitFor();
  const remaining = await page.evaluate(() => window.settings.custom_words);
  assert.equal(remaining.length, 150);
  assert.ok(
    remaining.includes("term-001") &&
      remaining.includes("term-250") &&
      !remaining.includes("term-101"),
  );
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

for (const [language, button, cases] of [
  [
    "en",
    "Test connection",
    [
      [
        "provider-rate-limit",
        "Connection test rate limited. Wait before trying again.",
      ],
      [
        "provider-auth",
        "Connection test authentication failed. Check the API key.",
      ],
      [
        "Bearer PRIVATE_TEST_ERROR token=SYNTHETIC",
        "The connection test did not complete. Check the connection settings and service status.",
      ],
    ],
  ],
  [
    "zh-TW",
    "測試連線",
    [
      ["provider-rate-limit", "測試請求過於頻繁，請稍後再試。"],
      ["provider-auth", "連線測試認證失敗，請檢查 API 金鑰。"],
      [
        "Bearer PRIVATE_TEST_ERROR token=SYNTHETIC",
        "連線測試未完成，請檢查連線設定與服務狀態。",
      ],
    ],
  ],
  [
    "zh-CN",
    "测试连接",
    [
      ["provider-rate-limit", "测试请求过于频繁，请稍后再试。"],
      ["provider-auth", "连接测试认证失败，请检查 API 密钥。"],
      [
        "Bearer PRIVATE_TEST_ERROR token=SYNTHETIC",
        "连接测试未完成，请检查连接设置与服务状态。",
      ],
    ],
  ],
])
  test(`explicit connection test ${language} errors never claim speech text was saved`, async (t) => {
    const page = await providerConnectionsFixture(t, language, [
      savedConnection,
    ]);
    for (const [code, copy] of cases) {
      await page.evaluate((code) => {
        window.testMode = "fail";
        window.testError = code;
      }, code);
      await page.getByRole("button", { name: button, exact: true }).click();
      await page.getByRole("note").waitFor();
      assert.doesNotMatch(
        await page.locator("body").innerText(),
        /原稿已保存|original is saved|transcript.*saved/i,
      );
      await page.getByRole("note").getByText(copy, { exact: true }).waitFor();
      assert.doesNotMatch(
        await page.locator("body").innerText(),
        /原稿已保存|original is saved|transcript.*saved|notype\.usability\.testErrors|PRIVATE_TEST_ERROR|token=SYNTHETIC/i,
      );
    }
    assert.equal(
      await page.evaluate(
        () =>
          window.calls.filter((c) => c.cmd === "test_text_connection").length,
      ),
      3,
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
