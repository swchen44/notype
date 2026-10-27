import test from "node:test";
import assert from "node:assert/strict";
import {
  providerConnectionsFixture,
  savedConnection,
} from "./fixtures/provider-connections.mjs";

test("text settings offers explicit creation of saved connections", async (t) => {
  const page = await providerConnectionsFixture(t);
  await page
    .getByRole("button", { name: "Add connection", exact: true })
    .waitFor();
});

test("editing a saved connection preserves manual model input until explicit save", async (t) => {
  const page = await providerConnectionsFixture(t, "en", [savedConnection]);
  await page
    .getByRole("button", { name: "Edit Company writing", exact: true })
    .click();
  const model = page.getByRole("textbox", {
    name: "Model ID (manual entry)",
    exact: true,
  });
  assert.equal(await model.inputValue(), "manual-model");
  await model.fill("user-entered-model");
  assert.equal(
    await page.evaluate(
      () =>
        window.calls.filter((call) => call.cmd === "save_text_connection")
          .length,
    ),
    0,
  );
  await page
    .getByRole("button", { name: "Save connection", exact: true })
    .click();
  await page.waitForFunction(() =>
    window.calls.some((call) => call.cmd === "save_text_connection"),
  );
  assert.equal(
    await page.evaluate(() => window.configuration.connections[0].model),
    "user-entered-model",
  );
  assert.equal(
    await page.evaluate(() => window.configuration.active_connection_id),
    null,
  );
});

test("API options preserve zero temperature and send an explicit key replacement without returning stored secrets", async (t) => {
  const connection = {
    ...savedConnection,
    authentication: "bearer",
    app_key_present: true,
    credential_present: true,
    credential_source: "app",
    api_key: "legacy-secret-must-not-render",
  };
  const page = await providerConnectionsFixture(t, "en", [connection]);
  await page
    .getByRole("button", { name: "Edit Company writing", exact: true })
    .click();
  await page
    .getByLabel("App key action", { exact: true })
    .selectOption("replace");
  await page
    .getByLabel("Replacement API key", { exact: true })
    .fill("synthetic-new-key");
  await page.getByLabel("Temperature", { exact: true }).fill("0");
  await page.getByLabel("Top p", { exact: true }).fill("0.5");
  await page
    .getByLabel("Maximum output tokens (blank uses API default)", {
      exact: true,
    })
    .fill("512");
  await page
    .getByRole("button", { name: "Save connection", exact: true })
    .click();
  await page.waitForFunction(() =>
    window.calls.some((call) => call.cmd === "save_text_connection"),
  );
  const payload = await page.evaluate(
    () => window.calls.find((call) => call.cmd === "save_text_connection").args,
  );
  assert.deepEqual(payload.keyAction, {
    action: "replace",
    value: "synthetic-new-key",
  });
  assert.equal(payload.connection.temperature, 0);
  assert.equal(payload.connection.top_p, 0.5);
  assert.equal(payload.connection.max_output_tokens, 512);
  assert.equal("api_key" in payload.connection, false);
  await page.getByText("Connection settings saved.", { exact: true }).waitFor();
  assert.doesNotMatch(
    await page.locator("body").innerText(),
    /legacy-secret|synthetic-new-key/,
  );
});

test("model IDs are queried and copied only on user action without changing the manual model", async (t) => {
  const page = await providerConnectionsFixture(t, "en", [savedConnection]);
  await page
    .getByRole("button", { name: "Edit Company writing", exact: true })
    .click();
  assert.equal(
    await page.evaluate(
      () =>
        window.calls.filter((call) => call.cmd === "list_text_models").length,
    ),
    0,
  );
  await page
    .getByRole("button", { name: "View model ID list", exact: true })
    .click();
  await page
    .getByRole("list", { name: "Model ID list", exact: true })
    .waitFor();
  await page
    .getByRole("button", { name: "Copy catalog/model-a", exact: true })
    .click();
  await page.waitForFunction(() => window.copiedText === "catalog/model-a");
  assert.equal(
    await page
      .getByLabel("Model ID (manual entry)", { exact: true })
      .inputValue(),
    "manual-model",
  );
  assert.equal(
    await page.evaluate(() => window.configuration.active_connection_id),
    null,
  );
  assert.equal(
    await page.evaluate(
      () =>
        window.calls.filter((call) => call.cmd === "save_text_connection")
          .length,
    ),
    0,
  );
  await page
    .getByLabel("Model ID (manual entry)", { exact: true })
    .fill("catalog/model-a");
  assert.equal(
    await page.evaluate(
      () =>
        window.calls.filter((call) => call.cmd === "save_text_connection")
          .length,
    ),
    0,
  );
});

test("failed configuration loading can be retried without exposing the rejection", async (t) => {
  const page = await providerConnectionsFixture(t, "en", [], null, "get-error");
  await page.getByRole("alert").waitFor();
  await page.evaluate(() => (window.mode = "ok"));
  await page
    .getByRole("button", { name: "Reload connections", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Add connection", exact: true })
    .waitFor();
  assert.doesNotMatch(await page.locator("body").innerText(), /read-secret/);
});

test("model-list authentication errors are actionable while manual input stays available", async (t) => {
  const page = await providerConnectionsFixture(t, "en", [savedConnection]);
  await page
    .getByRole("button", { name: "Edit Company writing", exact: true })
    .click();
  await page.evaluate(() => (window.modelMode = "auth-error"));
  await page
    .getByRole("button", { name: "View model ID list", exact: true })
    .click();
  await page
    .getByRole("alert")
    .getByText("Model-list authentication failed. Check the connection key.", {
      exact: true,
    })
    .waitFor();
  await page
    .getByLabel("Model ID (manual entry)", { exact: true })
    .fill("still-manual");
  assert.equal(
    await page
      .getByLabel("Model ID (manual entry)", { exact: true })
      .inputValue(),
    "still-manual",
  );
});

for (const mode of ["invoke-error", "invoke-error-object"]) {
  test(`a rejected model-list invocation can be explicitly retried (${mode})`, async (t) => {
    const page = await providerConnectionsFixture(t, "en", [savedConnection]);
    await page
      .getByRole("button", { name: "Edit Company writing", exact: true })
      .click();
    await page.evaluate((mode) => (window.modelMode = mode), mode);
    await page
      .getByRole("button", { name: "View model ID list", exact: true })
      .click();
    await page.getByRole("alert").waitFor();
    await page.evaluate(() => (window.modelMode = "ok"));
    await page
      .getByRole("button", { name: "View model ID list", exact: true })
      .click();
    await page
      .getByRole("list", { name: "Model ID list", exact: true })
      .waitFor();
    assert.doesNotMatch(
      await page.locator("body").innerText(),
      /upstream-secret|UNIQUE_MODEL_REJECTION_PRIVATE_SENTINEL/,
    );
    assert.equal(
      await page.evaluate(
        () =>
          window.calls.filter((call) => call.cmd === "list_text_models").length,
      ),
      2,
    );
    assert.equal(
      await page
        .getByLabel("Model ID (manual entry)", { exact: true })
        .inputValue(),
      "manual-model",
    );
    assert.equal(
      await page.evaluate(
        () =>
          window.calls.filter((call) =>
            ["save_text_connection", "activate_text_connection"].includes(
              call.cmd,
            ),
          ).length,
      ),
      0,
    );
  });
}

test("explicit model-list cancellation fences a late reply and permits a new query", async (t) => {
  const page = await providerConnectionsFixture(t, "en", [savedConnection]);
  await page
    .getByRole("button", { name: "Edit Company writing", exact: true })
    .click();
  await page.evaluate(() => (window.modelMode = "pending"));
  await page
    .getByRole("button", { name: "View model ID list", exact: true })
    .click();
  await page.waitForFunction(() => typeof window.releaseModels === "function");
  await page.getByRole("button", { name: "Cancel query", exact: true }).click();
  await page.evaluate(() => window.releaseModels());
  await page
    .getByText("Model-list query cancelled.", { exact: true })
    .waitFor();
  assert.equal(
    await page
      .getByRole("list", { name: "Model ID list", exact: true })
      .count(),
    0,
  );
  await page.evaluate(() => (window.modelMode = "ok"));
  await page
    .getByRole("button", { name: "View model ID list", exact: true })
    .click();
  await page
    .getByRole("list", { name: "Model ID list", exact: true })
    .waitFor();
});

test("a cancelled query finalizer cannot clear its successor's pending request", async (t) => {
  const page = await providerConnectionsFixture(t, "en", [savedConnection]);
  await page
    .getByRole("button", { name: "Edit Company writing", exact: true })
    .click();
  await page.evaluate(() => (window.modelMode = "pending"));
  await page
    .getByRole("button", { name: "View model ID list", exact: true })
    .click();
  await page.waitForFunction(() => typeof window.releaseModels === "function");
  await page.evaluate(
    () => (window.previousModelsRelease = window.releaseModels),
  );
  await page.getByRole("button", { name: "Cancel query", exact: true }).click();
  await page
    .getByRole("button", { name: "View model ID list", exact: true })
    .click();
  await page.waitForFunction(
    () => window.releaseModels !== window.previousModelsRelease,
  );
  await page.evaluate(async () => {
    window.previousModelsRelease();
    await new Promise((resolve) =>
      requestAnimationFrame(() => requestAnimationFrame(resolve)),
    );
  });
  await page
    .getByRole("button", { name: "Cancel query", exact: true })
    .waitFor();
  await page.evaluate(() => window.releaseModels());
  await page
    .getByRole("list", { name: "Model ID list", exact: true })
    .waitFor();
  assert.equal(
    await page.evaluate(
      () =>
        window.calls.filter((call) => call.cmd === "list_text_models").length,
    ),
    2,
  );
});

test("new local connections have a blank model and are not implicitly activated", async (t) => {
  const page = await providerConnectionsFixture(t);
  await page
    .getByRole("button", { name: "Add connection", exact: true })
    .click();
  await page
    .getByLabel("Connection name", { exact: true })
    .fill("My local server");
  await page
    .getByLabel("Connection type", { exact: true })
    .selectOption("local");
  await page
    .getByLabel("API format", { exact: true })
    .selectOption("ollama_openai");
  await page
    .getByRole("button", { name: "Create connection", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Edit My local server", exact: true })
    .click();
  assert.equal(
    await page
      .getByLabel("Model ID (manual entry)", { exact: true })
      .inputValue(),
    "",
  );
  assert.equal(
    await page.getByLabel("Request thinking", { exact: true }).isChecked(),
    false,
  );
  assert.equal(
    await page.evaluate(() => window.configuration.active_connection_id),
    null,
  );
  assert.equal(
    await page.evaluate(
      () =>
        window.calls.filter((call) => call.cmd === "list_text_models").length,
    ),
    0,
  );
});

test("removing the active connection does not activate another saved connection", async (t) => {
  const second = {
    ...savedConnection,
    id: "connection-b",
    display_name: "Second provider",
  };
  const page = await providerConnectionsFixture(
    t,
    "en",
    [savedConnection, second],
    savedConnection.id,
  );
  const row = page.getByRole("listitem").filter({ hasText: "Company writing" });
  await row.getByRole("button", { name: "Remove", exact: true }).click();
  await page
    .getByText("No active connection. Originals remain recoverable.", {
      exact: true,
    })
    .waitFor();
  assert.equal(
    await page.evaluate(() => window.configuration.connections.length),
    1,
  );
  assert.equal(
    await page.evaluate(
      () =>
        window.calls.filter((call) => call.cmd === "activate_text_connection")
          .length,
    ),
    0,
  );
});

test("a late model-list reply cannot enter a changed route", async (t) => {
  const page = await providerConnectionsFixture(t, "en", [savedConnection]);
  await page
    .getByRole("button", { name: "Edit Company writing", exact: true })
    .click();
  await page.evaluate(() => (window.modelMode = "pending"));
  await page
    .getByRole("button", { name: "View model ID list", exact: true })
    .click();
  await page.waitForFunction(() => typeof window.releaseModels === "function");
  await page
    .getByLabel("API address", { exact: true })
    .fill("https://another.synthetic.invalid/v1");
  await page.waitForFunction(() =>
    window.calls.some((call) => call.cmd === "cancel_text_models"),
  );
  await page.evaluate(() => window.releaseModels());
  assert.equal(
    await page
      .getByRole("list", { name: "Model ID list", exact: true })
      .count(),
    0,
  );
  assert.equal(
    await page
      .getByLabel("Model ID (manual entry)", { exact: true })
      .inputValue(),
    "manual-model",
  );
});

test("prompt saving preserves the connection collection and an unsaved model edit", async (t) => {
  const page = await providerConnectionsFixture(t, "en", [savedConnection]);
  await page
    .getByRole("button", { name: "Edit Company writing", exact: true })
    .click();
  await page
    .getByLabel("Model ID (manual entry)", { exact: true })
    .fill("unsaved-model");
  await page
    .getByLabel("Editable prompt", { exact: true })
    .fill("New dictation rules");
  await page
    .getByRole("button", { name: "Save this prompt", exact: true })
    .click();
  await page.getByText("Saved locally", { exact: true }).waitFor();
  assert.equal(
    await page
      .getByLabel("Model ID (manual entry)", { exact: true })
      .inputValue(),
    "unsaved-model",
  );
  await page
    .getByRole("button", { name: "Save connection", exact: true })
    .click();
  await page.getByText("Connection settings saved.", { exact: true }).waitFor();
  assert.equal(
    await page.evaluate(() => window.configuration.prompts.dictation),
    "New dictation rules",
  );
  assert.equal(
    await page.evaluate(() => window.configuration.connections[0].model),
    "unsaved-model",
  );
});

for (const [mode, message] of [
  [
    "empty",
    "The API returned an empty model list. Manual model entry is still available.",
  ],
  [
    "unsupported",
    "This API does not provide a supported model list. Enter a model ID manually.",
  ],
  [
    "error",
    "Unable to query model IDs. Check the connection or enter an ID manually.",
  ],
]) {
  test(
    "model-list " + mode + " retains manual entry and hides raw errors",
    async (t) => {
      const page = await providerConnectionsFixture(t, "en", [savedConnection]);
      await page
        .getByRole("button", { name: "Edit Company writing", exact: true })
        .click();
      await page.evaluate((mode) => (window.modelMode = mode), mode);
      await page
        .getByRole("button", { name: "View model ID list", exact: true })
        .click();
      await page.getByText(message, { exact: true }).waitFor();
      await page
        .getByLabel("Model ID (manual entry)", { exact: true })
        .fill("manual-after-failure");
      assert.doesNotMatch(
        await page.locator("body").innerText(),
        /upstream-secret/,
      );
    },
  );
}

test("a stored save remains successful if readiness refresh fails", async (t) => {
  const page = await providerConnectionsFixture(t, "en", [savedConnection]);
  await page
    .getByRole("checkbox", { name: "Enable AI text processing", exact: true })
    .locator("..")
    .click();
  await page.waitForFunction(
    () =>
      window.settings.post_process_enabled &&
      window.calls.some((call) => call.cmd === "get_workflow_readiness"),
  );
  await page
    .getByRole("button", { name: "Edit Company writing", exact: true })
    .click();
  await page
    .getByLabel("Model ID (manual entry)", { exact: true })
    .fill("saved-during-readiness-failure");
  const before = await page.evaluate(
    () =>
      window.calls.filter((call) => call.cmd === "get_workflow_readiness")
        .length,
  );
  await page.evaluate(() => (window.mode = "readiness-error"));
  await page
    .getByRole("button", { name: "Save connection", exact: true })
    .click();
  await page.getByText("Connection settings saved.", { exact: true }).waitFor();
  await page.waitForFunction(
    (before) =>
      window.calls.filter((call) => call.cmd === "get_workflow_readiness")
        .length > before,
    before,
  );
  assert.equal(
    await page.evaluate(() => window.configuration.connections[0].model),
    "saved-during-readiness-failure",
  );
  assert.doesNotMatch(
    await page.locator("body").innerText(),
    /readiness-secret/,
  );
});

for (const [language, edit, label, view] of [
  [
    "en",
    "Edit Company writing",
    "Model ID (manual entry)",
    "View model ID list",
  ],
  ["zh-TW", "編輯 Company writing", "模型 ID（手動輸入）", "查看模型 ID 清單"],
  ["zh-CN", "编辑 Company writing", "模型 ID（手动输入）", "查看模型 ID 列表"],
]) {
  test(
    "connection controls stay translated and usable in a narrow viewport: " +
      language,
    async (t) => {
      const page = await providerConnectionsFixture(t, language, [
        savedConnection,
      ]);
      await page.setViewportSize({ width: 360, height: 900 });
      await page.getByRole("button", { name: edit, exact: true }).click();
      await page.getByLabel(label, { exact: true }).waitFor();
      await page.getByRole("button", { name: view, exact: true }).waitFor();
      assert.doesNotMatch(
        await page.locator("body").innerText(),
        /notype\.connections|currently unavailable|目前無法顯示|\?\?/,
      );
      assert.equal(
        await page.evaluate(
          () => document.documentElement.scrollWidth <= window.innerWidth,
        ),
        true,
      );
    },
  );
}
