import test from "node:test";
import { providerConnectionsFixture } from "./fixtures/provider-connections.mjs";

test("text settings offers explicit creation of saved connections", async (t) => {
  const page = await providerConnectionsFixture(t);
  await page.getByRole("button", { name: "Add connection", exact: true }).waitFor();
});
