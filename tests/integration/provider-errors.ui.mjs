import assert from "node:assert/strict";
import test from "node:test";
import { historyApiFixture, historyEntry } from "./fixtures/history-api.mjs";

for (const [language, forward, reverse] of [
  [
    "en",
    "Authentication failed. Check the service key. The original is saved.",
    "Back-translation: authentication failed. Check the service key. The primary translation is retained; review is incomplete.",
  ],
  [
    "zh-TW",
    "認證失敗，請檢查文字服務金鑰。原稿已保存。",
    "回譯：認證失敗，請檢查文字服務金鑰。主翻譯已保留，審閱尚未完成。",
  ],
  [
    "zh-CN",
    "认证失败，请检查文字服务密钥。原稿已保存。",
    "回译：认证失败，请检查文字服务密钥。主翻译已保留，审阅尚未完成。",
  ],
]) {
  test(`history safely presents forward and reverse authentication errors in ${language}`, async (t) => {
    const page = await historyApiFixture(t, "HistorySettings", language);
    const entries = ["provider-auth", "back-provider-auth"].map(
      (error, index) => ({
        ...structuredClone(historyEntry),
        id: `auth-${index}`,
        error,
        status: index ? "pending" : "failed",
      }),
    );
    await page.evaluate((entries) => {
      window.entries = entries;
      window.render();
    }, entries);
    for (const summary of await page
      .locator("article > details > summary")
      .all())
      await summary.click();
    await page.getByRole("note").getByText(forward, { exact: true }).waitFor();
    await page.getByRole("note").getByText(reverse, { exact: true }).waitFor();
    assert.doesNotMatch(
      await page.locator("body").innerText(),
      /notype\.errors|Bearer|synthetic-key/,
    );
  });
}

for (const [language, labels] of [
  [
    "en",
    [
      "Rate limited",
      "Insufficient balance",
      "Service timed out",
      "Editing did not complete. The original is saved.",
    ],
  ],
  [
    "zh-TW",
    [
      "請求過於頻繁",
      "服務餘額不足",
      "文字服務逾時",
      "整理未完成，原稿已保存。",
    ],
  ],
  [
    "zh-CN",
    [
      "请求过于频繁",
      "服务余额不足",
      "文字服务超时",
      "整理未完成，原稿已保存。",
    ],
  ],
]) {
  test(`history ${language} distinguishes actionable provider categories and safely falls back`, async (t) => {
    const page = await historyApiFixture(t, "HistorySettings", language);
    const entries = [
      "provider-rate-limit",
      "provider-balance",
      "provider-timeout",
      "Bearer unknown-secret https://private.invalid/?token=secret",
    ].map((error, index) => ({
      ...structuredClone(historyEntry),
      id: `category-${index}`,
      error,
      status: "failed",
    }));
    await page.evaluate((entries) => {
      window.entries = entries;
      window.render();
    }, entries);
    for (const summary of await page
      .locator("article > details > summary")
      .all())
      await summary.click();
    for (const label of labels.slice(0, 3)) {
      await page.getByRole("note").filter({ hasText: label }).waitFor();
    }
    assert.equal(
      await page.getByRole("note").filter({ hasText: labels[3] }).count(),
      1,
    );
    assert.doesNotMatch(
      await page.locator("body").innerText(),
      /unknown-secret|private\.invalid|notype\.errors/,
    );
  });
}
