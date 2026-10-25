import assert from "node:assert/strict";
import test from "node:test";
import { historyApiFixture, historyEntry } from "./fixtures/history-api.mjs";

test("history renders backend counts, observed duration, exact partial fees and associated models", async (t) => {
  const page = await historyApiFixture(t);
  await page.evaluate((entry) => {
    window.entries = [entry];
    window.render();
  }, historyEntry);
  const article = page.getByRole("article");
  await article.getByText("Original count: 91", { exact: true }).waitFor();
  await article.getByText("Result count: 0", { exact: true }).waitFor();
  await article.getByText("1.234 s · Incomplete", { exact: true }).waitFor();
  await article
    .getByText("Actual producer (actual-asr; cpu)", { exact: true })
    .waitFor();
  await article.getByText("0 ms", { exact: true }).waitFor();
  await article.getByText("API usage and actual fees", { exact: true }).click();
  await article.getByText("Complete total: 0", { exact: true }).waitFor();
  await article
    .getByText("Partial · Known subtotal: 17", { exact: true })
    .waitFor();
  await article
    .getByText("USD 0.100000000000000001", { exact: true })
    .waitFor();
  await article.getByText("TWD 2.50", { exact: true }).waitFor();
  const associations = article.getByRole("list", {
    name: "Model and effort associations",
  });
  assert.match(
    await associations.innerText(),
    /request-a.*high.*reported-a.*Unknown/s,
  );
  assert.match(
    await associations.innerText(),
    /request-b.*Provider default.*Unknown.*low/s,
  );
  assert.equal(
    await page.evaluate(() =>
      window.calls.some((x) => x.cmd === "get_text_configuration"),
    ),
    false,
  );
});

test("all-history export ignores empty search, prevents duplicates and distinguishes save, cancellation and failure", async (t) => {
  const page = await historyApiFixture(t);
  await page.evaluate(() => window.render());
  await page.getByRole("searchbox").fill("no matching history");
  await page.getByText("No matching text history", { exact: true }).waitFor();
  await page.evaluate(() => {
    window.mode = "export-pending";
  });
  const button = page.getByRole("button", {
    name: "Export all history CSV",
    exact: true,
  });
  await button.click();
  await page.waitForFunction(() => typeof window.releaseExport === "function");
  assert.equal(
    await page
      .getByRole("button", { name: "Exporting all history…", exact: true })
      .isDisabled(),
    true,
  );
  await page.evaluate(() => window.releaseExport());
  await page
    .getByRole("status")
    .getByText("Saved all history CSV: 83 rows.", { exact: true })
    .waitFor();
  await page.evaluate(() => {
    window.mode = "export-cancel";
  });
  await button.click();
  await page
    .getByText("Export cancelled. No CSV was saved.", { exact: true })
    .waitFor();
  await page.evaluate(() => {
    window.mode = "export-error";
  });
  await button.click();
  await page
    .getByText(
      "Unable to save history CSV. Choose a writable location and try again.",
      { exact: true },
    )
    .waitFor();
  assert.doesNotMatch(
    await page.locator("body").innerText(),
    /Bearer|synthetic-secret|private/,
  );
  assert.deepEqual(
    await page.evaluate(() =>
      window.calls.filter((x) => x.cmd === "export_history_csv"),
    ),
    [
      { cmd: "export_history_csv", args: {} },
      { cmd: "export_history_csv", args: {} },
      { cmd: "export_history_csv", args: {} },
    ],
  );
  await page.evaluate(() => {
    window.mode = "ok";
  });
  await button.click();
  await page
    .getByText("Saved all history CSV: 83 rows.", { exact: true })
    .waitFor();
});

test("unknown observations remain unknown, separate from actual zero and no API use", async (t) => {
  const page = await historyApiFixture(t);
  const entry = structuredClone(historyEntry);
  Object.assign(entry.observations, {
    recording: null,
    original_count: null,
    result_count: null,
    actual_asr_model: null,
    requested_asr_model: null,
    initial_stop_to_result_ms: null,
    asr_after_stop_ms: null,
    latest_processing_kind: null,
    latest_processing_ms: null,
    latest_outcome: null,
    api_metadata_unavailable: true,
  });
  entry.api_summary.call_count = null;
  entry.api_summary.observed_call_count = 0;
  entry.api_summary.input = {
    status: "unknown",
    total: null,
    known_subtotal: null,
    reported_calls: 0,
  };
  entry.api_summary.output = {
    status: "not_applicable",
    total: null,
    known_subtotal: null,
    reported_calls: 0,
  };
  entry.api_summary.fees = {
    status: "unknown",
    totals: [],
    known_subtotals: [],
    reported_calls: 0,
  };
  entry.api_summary.model_efforts = [];
  await page.evaluate((entry) => {
    window.entries = [entry];
    window.render();
  }, entry);
  const article = page.getByRole("article");
  await article.getByText("Original count: Unknown", { exact: true }).waitFor();
  await article.getByText("Result count: Unknown", { exact: true }).waitFor();
  await article.getByText("API usage and actual fees", { exact: true }).click();
  await article
    .getByText("Complete call count: Unknown", { exact: true })
    .waitFor();
  await article
    .getByText("Not applicable (no API calls)", { exact: true })
    .waitFor();
  await article
    .getByRole("note")
    .getByText(
      "API metadata is incomplete. Observed calls and known subtotals may omit calls.",
      { exact: true },
    )
    .waitFor();
  assert.equal(
    await article.getByText("Complete total: 0", { exact: true }).count(),
    0,
  );
  assert.equal(await article.getByText("0 ms", { exact: true }).count(), 0);
  assert.equal(await article.getByText("USD 0", { exact: true }).count(), 0);
});

test("complete actual fees retain decimal strings and returned zero, received audio zero stays observed", async (t) => {
  const page = await historyApiFixture(t);
  const entry = structuredClone(historyEntry);
  entry.observations.recording = { audio_ms: 0, quality: "complete" };
  entry.api_summary.fees = {
    status: "complete",
    totals: [
      { currency: "USD", amount: "0" },
      { currency: "TWD", amount: "1.000000000000000001" },
    ],
    known_subtotals: [],
    reported_calls: 3,
  };
  await page.evaluate((entry) => {
    window.entries = [entry];
    window.render();
  }, entry);
  await page.getByText("0.000 s · Complete", { exact: true }).waitFor();
  await page.getByText("API usage and actual fees", { exact: true }).click();
  await page.getByText("USD 0", { exact: true }).waitFor();
  await page.getByText("TWD 1.000000000000000001", { exact: true }).waitFor();
});

for (const [language, labels] of [
  [
    "en",
    [
      "Original count: 91",
      "API usage and actual fees",
      "Export all history CSV",
      "Partial · Known subtotal: 17",
    ],
  ],
  [
    "zh-TW",
    [
      "原稿字數：91",
      "API 用量與實際回報費用",
      "匯出全部歷史 CSV",
      "部分資料 · 已知小計：17",
    ],
  ],
  [
    "zh-CN",
    [
      "原稿字数：91",
      "API 用量与实际返回费用",
      "导出全部历史 CSV",
      "部分数据 · 已知小计：17",
    ],
  ],
])
  test(`history ${language} remains translated and fits a narrow viewport`, async (t) => {
    const page = await historyApiFixture(t, "HistorySettings", language);
    await page.setViewportSize({ width: 360, height: 900 });
    await page.evaluate((entry) => {
      window.entries = [entry];
      window.render();
    }, historyEntry);
    await page.getByText(labels[0], { exact: true }).waitFor();
    await page.getByText(labels[1], { exact: true }).click();
    await page.getByText(labels[3], { exact: true }).waitFor();
    await page.getByRole("button", { name: labels[2], exact: true }).waitFor();
    assert.equal(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= window.innerWidth,
      ),
      true,
    );
    assert.doesNotMatch(
      await page.locator("body").innerText(),
      /notype\.metrics|notype\.history|目前無法顯示|currently unavailable|\?\?/,
    );
  });
