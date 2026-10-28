import test from "node:test";
import assert from "node:assert/strict";
import {
  providerConnectionsFixture,
  savedConnection,
} from "./fixtures/provider-connections.mjs";

test("provider setup exposes an API key without a prerequisite action menu", async (t) => {
  const page = await providerConnectionsFixture(t, "en", [
    {
      ...savedConnection,
      authentication: "bearer",
      app_key_present: true,
      credential_present: true,
      credential_source: "app",
      masked_api_key: "demo****tail",
    },
  ]);
  await page
    .getByRole("button", { name: "Edit Company writing", exact: true })
    .click();
  const key = page.getByLabel("API key", { exact: true });
  await key.waitFor({ state: "visible" });
  assert.equal(await key.getAttribute("type"), "password");
  assert.equal(await key.inputValue(), "");
  await key.fill("synthetic-new-key");
  assert.equal(
    await page.evaluate(
      () => window.calls.filter((c) => c.cmd === "save_text_connection").length,
    ),
    0,
  );
  await page
    .getByRole("button", { name: "Save connection", exact: true })
    .click();
  await page.waitForFunction(() =>
    window.calls.some((c) => c.cmd === "save_text_connection"),
  );
  assert.deepEqual(
    await page.evaluate(
      () =>
        window.calls.find((c) => c.cmd === "save_text_connection").args
          .keyAction,
    ),
    { action: "replace", value: "synthetic-new-key" },
  );
  assert.doesNotMatch(
    await page.locator("body").innerText(),
    /synthetic-new-key/,
  );
});

test("provider setup explains the next step and brings the created editor into view", async (t) => {
  const page = await providerConnectionsFixture(t, "en", [
    savedConnection,
    { ...savedConnection, id: "second", display_name: "Second connection" },
  ]);
  await page.setViewportSize({ width: 682, height: 600 });
  await page
    .getByRole("button", { name: "Add connection", exact: true })
    .click();
  await page
    .getByText(
      "Create the connection first. Then fill in the API address, key and model in Connection settings. Use Edit to change them later.",
      { exact: true },
    )
    .waitFor();
  await page
    .getByLabel("Connection name", { exact: true })
    .fill("New provider");
  await page
    .getByRole("button", { name: "Create connection", exact: true })
    .click();
  const editor = page.getByRole("region", {
    name: "Connection settings",
    exact: true,
  });
  await editor.waitFor();
  await page.waitForFunction(
    () =>
      document.activeElement?.getAttribute("aria-label") === "Connection name",
  );
  const bounds = await editor.boundingBox();
  assert.ok(
    bounds && bounds.y >= 0 && bounds.y < 160,
    "new connection settings must be in view",
  );
  await editor
    .getByLabel("API key", { exact: true })
    .waitFor({ state: "visible" });
  assert.equal(
    await page.evaluate(() => window.configuration.active_connection_id),
    null,
  );
});
