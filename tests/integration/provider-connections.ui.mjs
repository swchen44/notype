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
  assert.doesNotMatch(
    await page.locator("body").innerText(),
    /read-secret|Bearer/,
  );
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
