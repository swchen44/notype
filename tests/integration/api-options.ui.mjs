import assert from "node:assert/strict";
import test from "node:test";
import {
  providerConnectionsFixture,
  savedConnection,
} from "./fixtures/provider-connections.mjs";

async function editor(t, connection = savedConnection) {
  const page = await providerConnectionsFixture(t, "en", [connection]);
  await page
    .getByRole("button", { name: "Edit Company writing", exact: true })
    .click();
  return page;
}

test("blank options remain omitted preferences and do not trigger model requests", async (t) => {
  const page = await editor(t);
  assert.equal(
    await page.getByLabel("Temperature", { exact: true }).inputValue(),
    "",
  );
  assert.equal(
    await page.getByLabel("Top p", { exact: true }).inputValue(),
    "",
  );
  assert.equal(
    await page.getByLabel("Request thinking", { exact: true }).isChecked(),
    false,
  );
  await page
    .getByRole("button", { name: "Save connection", exact: true })
    .click();
  await page.getByText("Connection settings saved.", { exact: true }).waitFor();
  const call = await page.evaluate(() =>
    window.calls.find((call) => call.cmd === "save_text_connection"),
  );
  assert.equal(call.args.connection.temperature, null);
  assert.equal(call.args.connection.top_p, null);
  assert.equal(call.args.connection.max_output_tokens, null);
  assert.equal(call.args.connection.thinking_preference, "off");
  assert.deepEqual(call.args.keyAction, { action: "keep" });
  assert.equal(
    await page.evaluate(() =>
      window.calls.some((call) => call.cmd === "list_text_models"),
    ),
    false,
  );
});

test("an explicit declared effort and output cap are saved independently of the model", async (t) => {
  const connection = structuredClone(savedConnection);
  connection.capability_declaration.kind = "optional";
  connection.capability_declaration.supported_efforts = ["low", "high"];
  connection.resolution.capability = "optional";
  connection.resolution.supported_efforts = ["low", "high"];
  const page = await editor(t, connection);
  await page.getByLabel("Request thinking", { exact: true }).check();
  await page.getByLabel("Effort", { exact: true }).selectOption("high");
  await page
    .getByLabel("Maximum output tokens (blank uses API default)", {
      exact: true,
    })
    .fill("512");
  await page
    .getByRole("button", { name: "Save connection", exact: true })
    .click();
  await page.getByText("Connection settings saved.", { exact: true }).waitFor();
  const draft = await page.evaluate(
    () =>
      window.calls.find((call) => call.cmd === "save_text_connection").args
        .connection,
  );
  assert.equal(draft.model, "manual-model");
  assert.equal(draft.effort, "high");
  assert.equal(draft.thinking_preference, "on");
  assert.equal(draft.max_output_tokens, 512);
});

test("invalid numeric options cannot be submitted and valid zero remains usable", async (t) => {
  const page = await editor(t);
  const output = page.getByLabel(
    "Maximum output tokens (blank uses API default)",
    { exact: true },
  );
  const save = page.getByRole("button", {
    name: "Save connection",
    exact: true,
  });
  for (const value of ["0", "1.5", "1000001", "Infinity"]) {
    await output.fill(value);
    assert.equal(await save.isDisabled(), true);
  }
  await output.fill("");
  await page.getByLabel("Temperature", { exact: true }).fill("0");
  await page.getByLabel("Top p", { exact: true }).fill("0");
  assert.equal(await save.isDisabled(), true);
  await page.getByLabel("Top p", { exact: true }).fill("1");
  assert.equal(await save.isDisabled(), false);
  assert.equal(
    await page.evaluate(
      () =>
        window.calls.filter((call) => call.cmd === "save_text_connection")
          .length,
    ),
    0,
  );
});

test("a rejected save keeps the edit and supports an explicit retry without raw rejection text", async (t) => {
  const page = await editor(t);
  await page
    .getByLabel("Model ID (manual entry)", { exact: true })
    .fill("retry-model");
  await page.evaluate(() => (window.mode = "save-error"));
  await page
    .getByRole("button", { name: "Save connection", exact: true })
    .click();
  await page
    .getByRole("alert")
    .getByText("Unable to update connections. Please try again.", {
      exact: true,
    })
    .waitFor();
  assert.equal(
    await page
      .getByLabel("Model ID (manual entry)", { exact: true })
      .inputValue(),
    "retry-model",
  );
  assert.doesNotMatch(
    await page.locator("body").innerText(),
    /should-never-render/,
  );
  await page.evaluate(() => (window.mode = "ok"));
  await page
    .getByRole("button", { name: "Save connection", exact: true })
    .click();
  await page.getByText("Connection settings saved.", { exact: true }).waitFor();
  assert.equal(
    await page.evaluate(() => window.configuration.connections[0].model),
    "retry-model",
  );
});

test("prompt reset and translation mutation preserve the saved connection collection", async (t) => {
  const page = await editor(t);
  await page
    .getByRole("button", { name: "Restore this default prompt", exact: true })
    .click();
  await page
    .getByText("Selected default prompt restored", { exact: true })
    .waitFor();
  assert.equal(
    await page.evaluate(() => window.configuration.connections[0].model),
    "manual-model",
  );
  assert.equal(
    await page.evaluate(() => window.configuration.prompts.ask),
    "Saved ask",
  );
  await page
    .getByLabel("Scenario", { exact: true })
    .selectOption("translation");
  await page
    .getByLabel("Translation target language", { exact: true })
    .fill("zh-TW");
  await page
    .getByRole("button", { name: "Save translation settings", exact: true })
    .click();
  await page.getByText("Saved locally", { exact: true }).waitFor();
  assert.equal(
    await page.evaluate(() => window.configuration.connections.length),
    1,
  );
  assert.equal(
    await page.evaluate(() => window.configuration.translation.target_language),
    "zh-TW",
  );
  assert.equal(
    await page.evaluate(
      () =>
        window.calls.find((call) => call.cmd === "save_translation_settings")
          .args.expectedRevision,
    ),
    1,
  );
});

test("a mandatory model displays minimum thinking without inventing a minimal option", async (t) => {
  const connection = structuredClone(savedConnection);
  connection.capability_declaration = null;
  connection.resolution = {
    ...connection.resolution,
    capability: "mandatory",
    provenance: "documented",
    effective_thinking: true,
    effective_effort: "low",
    supported_efforts: ["low", "medium", "high"],
    reason: "mandatory_minimum",
  };
  const page = await editor(t, connection);
  await page
    .getByText(
      "This model cannot turn thinking off. The request will use its minimum supported effort: low.",
      { exact: true },
    )
    .waitFor();
  assert.equal(
    await page.getByLabel("Request thinking", { exact: true }).isChecked(),
    false,
  );
  await page.getByLabel("Request thinking", { exact: true }).check();
  assert.equal(
    await page.getByRole("option", { name: "minimal", exact: true }).count(),
    0,
  );
  await page.getByLabel("Effort", { exact: true }).selectOption("low");
});
