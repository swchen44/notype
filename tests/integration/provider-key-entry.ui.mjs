import test from "node:test";
import assert from "node:assert/strict";
import {
  providerConnectionsFixture,
  savedConnection,
} from "./fixtures/provider-connections.mjs";

test("blank key preserves the saved credential and explicit clear remains separate", async (t) => {
  const page = await providerConnectionsFixture(t, "en", [
    {
      ...savedConnection,
      authentication: "bearer",
      app_key_present: true,
      credential_present: true,
      credential_source: "app",
    },
  ]);
  await page
    .getByRole("button", { name: "Edit Company writing", exact: true })
    .click();
  const key = page.getByLabel("API key", { exact: true });
  await key.fill("synthetic-replacement");
  await key.fill("");
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
    { action: "keep" },
  );
  await page
    .getByLabel("App key action", { exact: true })
    .selectOption("clear");
  assert.equal(await key.isEnabled(), false);
  await page
    .getByRole("button", { name: "Save connection", exact: true })
    .click();
  await page.waitForFunction(
    () =>
      window.calls.filter((c) => c.cmd === "save_text_connection").length === 2,
  );
  assert.deepEqual(
    await page.evaluate(
      () =>
        window.calls.filter((c) => c.cmd === "save_text_connection")[1].args
          .keyAction,
    ),
    { action: "clear" },
  );
});

test("switching a no-auth local connection to Bearer immediately enables direct key input", async (t) => {
  const page = await providerConnectionsFixture(t, "en", [
    {
      ...savedConnection,
      kind: "local",
      authentication: "none",
      app_key_present: false,
      credential_present: false,
      credential_source: "none",
    },
  ]);
  await page
    .getByRole("button", { name: "Edit Company writing", exact: true })
    .click();
  await page
    .getByLabel("Authentication", { exact: true })
    .selectOption("bearer");
  const key = page.getByLabel("API key", { exact: true });
  assert.equal(await key.isEnabled(), true);
  await key.fill("synthetic-local-key");
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
    { action: "replace", value: "synthetic-local-key" },
  );
});
