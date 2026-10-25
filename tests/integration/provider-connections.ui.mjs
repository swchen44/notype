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
