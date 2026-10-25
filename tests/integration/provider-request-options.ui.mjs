import assert from "node:assert/strict";
import test from "node:test";
import { historyApiFixture, historyEntry } from "./fixtures/history-api.mjs";

test("history separates requested thinking, actual wire controls and documented normalization", async (t) => {
  const page = await historyApiFixture(t);
  const entry = structuredClone(historyEntry);
  entry.api_summary.model_efforts = [
    {
      requested_model: "explicit-user-model",
      reported_model: "reported-model",
      requested_effort: "low",
      reported_effort: null,
      parameters: {
        requested_thinking: false,
        preference_effort: null,
        effective_thinking: true,
        effective_effort: "low",
        capability_provenance: "documented",
        resolution_reason: "mandatory_minimum",
        configured_temperature: "0.25",
        wire_temperature: null,
        temperature_resolution: "ignored_by_provider",
        configured_top_p: "0.5",
        wire_top_p: "0.5",
        top_p_resolution: "sent",
        documented_top_p: "0.95",
        max_output_tokens: 512,
      },
    },
  ];
  await page.evaluate((entry) => {
    window.entries = [entry];
    window.render();
  }, entry);
  await page.getByText("API usage and actual fees", { exact: true }).click();
  await page.getByText("Thinking preference: Off", { exact: true }).waitFor();
  await page.getByText("Thinking in request: On", { exact: true }).waitFor();
  await page
    .getByText("Temperature preference: 0.25", { exact: true })
    .waitFor();
  await page
    .getByText("Temperature in request: Omitted / API default", { exact: true })
    .waitFor();
  await page
    .getByText(
      "Documented top p normalization: 0.95 (not reported by the response)",
      { exact: true },
    )
    .waitFor();
  assert.doesNotMatch(
    await page.locator("body").innerText(),
    /notype\.requestOptions/,
  );
});
