import assert from "node:assert/strict";
import test from "node:test";
import { historyApiFixture, historyEntry } from "./fixtures/history-api.mjs";

for (const [
  language,
  details,
  preference,
  wire,
  temperature,
  omitted,
  documented,
] of [
  [
    "en",
    "API usage and actual fees",
    "Thinking preference: Off",
    "Thinking in request: On",
    "Temperature preference: 0.25",
    "Temperature in request: Omitted / API default",
    "Documented top p normalization: 0.95 (not reported by the response)",
  ],
  [
    "zh-TW",
    "API 用量與實際回報費用",
    "Thinking 偏好：關",
    "請求中的 thinking：開",
    "Temperature 偏好：0.25",
    "請求中的 temperature：未指定／API 預設",
    "文件的 top p 正規化參考：0.95（非回應回報值）",
  ],
  [
    "zh-CN",
    "API 用量与实际返回费用",
    "Thinking 偏好：关",
    "请求中的 thinking：开",
    "Temperature 偏好：0.25",
    "请求中的 temperature：未指定／API 默认",
    "文档的 top p 归一化参考：0.95（非响应报告值）",
  ],
]) {
  test(
    "history separates requested thinking, actual wire controls and documented normalization: " +
      language,
    async (t) => {
      const page = await historyApiFixture(t, "HistorySettings", language);
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
      await page.locator("article > details > summary").click();
      await page.getByText(details, { exact: true }).click();
      await page.getByText(preference, { exact: true }).waitFor();
      await page.getByText(wire, { exact: true }).waitFor();
      await page.getByText(temperature, { exact: true }).waitFor();
      await page.getByText(omitted, { exact: true }).waitFor();
      await page.getByText(documented, { exact: true }).waitFor();
      assert.doesNotMatch(
        await page.locator("body").innerText(),
        /notype\.requestOptions/,
      );
    },
  );
}
