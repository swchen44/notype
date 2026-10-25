import assert from "node:assert/strict";
import test from "node:test";
import { historyApiFixture } from "./fixtures/history-api.mjs";

test("API profile preserves endpoint/model/key, validates unsupported effort and saves explicit limits", async (t) => {
  const page = await historyApiFixture(t, "PostProcessingSettings");
  await page.evaluate(() => {
    window.configuration.effort = "medium";
    window.render();
  });
  const profile = page.getByRole("combobox", {
    name: "Provider profile",
    exact: true,
  });
  await profile.selectOption("deepseek");
  await page
    .getByText(
      "Choose provider default, none, low, high or max for DeepSeek.",
      { exact: true },
    )
    .waitFor();
  assert.equal(
    await page
      .getByRole("textbox", { name: "Full request URL", exact: true })
      .inputValue(),
    "https://synthetic.invalid/chat",
  );
  assert.equal(
    await page
      .getByRole("textbox", { name: "Model ID (manual entry)", exact: true })
      .inputValue(),
    "fixture-model",
  );
  assert.equal(
    await page.getByLabel("Local API key", { exact: true }).inputValue(),
    "synthetic-key",
  );
  await page
    .getByRole("button", { name: "Save settings", exact: true })
    .click();
  assert.equal(
    await page.evaluate(
      () =>
        window.calls.filter((x) => x.cmd === "set_text_configuration").length,
    ),
    0,
  );
  await page
    .getByRole("combobox", { name: "Effort", exact: true })
    .selectOption("none");
  await page
    .getByRole("textbox", {
      name: "Maximum output tokens (blank uses API default)",
      exact: true,
    })
    .fill("393216");
  await page
    .getByRole("button", { name: "Save settings", exact: true })
    .click();
  await page.getByText("Local settings saved", { exact: true }).waitFor();
  const sent = await page.evaluate(
    () =>
      window.calls.find((x) => x.cmd === "set_text_configuration").args
        .configuration,
  );
  assert.deepEqual(
    {
      profile: sent.provider_profile,
      effort: sent.effort,
      limit: sent.max_output_tokens,
    },
    { profile: "deepseek", effort: "none", limit: 393216 },
  );
  assert.deepEqual(sent.prompts, {
    dictation: "Saved dictation",
    translation: "Saved translation",
    ask: "Saved ask",
  });
  assert.equal(sent.translation.domain, "Fixture domain");
  assert.equal(sent.endpoint, "https://synthetic.invalid/chat");
  assert.equal(sent.model, "fixture-model");
  assert.equal(sent.api_key, "synthetic-key");
  assert.equal(
    await page.evaluate(() =>
      window.calls.some((x) => /list.*model|test.*api/.test(x.cmd)),
    ),
    false,
  );
});

test("generic custom effort remains editable through standard prefixes and persists provider-default omissions", async (t) => {
  const page = await historyApiFixture(t, "PostProcessingSettings");
  await page.evaluate(() => window.render());
  const effort = page.getByRole("combobox", { name: "Effort", exact: true });
  await effort.selectOption("custom");
  const custom = page.getByRole("textbox", {
    name: "Custom effort",
    exact: true,
  });
  await custom.pressSequentially("low-cost_v2");
  assert.equal(await custom.inputValue(), "low-cost_v2");
  const limit = page.getByRole("textbox", {
    name: "Maximum output tokens (blank uses API default)",
    exact: true,
  });
  await limit.fill("1000000");
  await page
    .getByRole("button", { name: "Save settings", exact: true })
    .click();
  await page.getByText("Local settings saved", { exact: true }).waitFor();
  assert.deepEqual(
    await page.evaluate(() => {
      const c = window.configuration;
      return [c.effort, c.max_output_tokens];
    }),
    ["low-cost_v2", 1000000],
  );
  await effort.selectOption("default");
  await limit.fill("");
  await page
    .getByRole("button", { name: "Save settings", exact: true })
    .click();
  await page.getByText("Local settings saved", { exact: true }).waitFor();
  assert.deepEqual(
    await page.evaluate(() => {
      const c = window.configuration;
      return [c.provider_profile, c.effort, c.max_output_tokens];
    }),
    ["generic", null, null],
  );
});

test("API options reject invalid spellings and limits without persistence and retain rejected save drafts for retry", async (t) => {
  const page = await historyApiFixture(t, "PostProcessingSettings");
  await page.evaluate(() => window.render());
  const effort = page.getByRole("combobox", { name: "Effort", exact: true });
  const profile = page.getByRole("combobox", {
    name: "Provider profile",
    exact: true,
  });
  const limit = page.getByRole("textbox", {
    name: "Maximum output tokens (blank uses API default)",
    exact: true,
  });
  const save = page.getByRole("button", { name: "Save settings", exact: true });
  await effort.selectOption("custom");
  for (const invalid of ["", "HIGH", "bad value", "a".repeat(33), "中文"]) {
    await page
      .getByRole("textbox", { name: "Custom effort", exact: true })
      .fill(invalid);
    await page
      .getByText(
        "Use 1–32 lowercase ASCII letters, digits, underscores or hyphens.",
        { exact: true },
      )
      .waitFor();
    await save.click();
  }
  await page
    .getByRole("textbox", { name: "Custom effort", exact: true })
    .fill("a".repeat(32));
  for (const invalid of ["0", "-1", "1.5", "1e3", "1000001"]) {
    await limit.fill(invalid);
    await page
      .getByText(
        "Enter a whole number from 1 to 1000000, or leave blank for API default.",
        { exact: true },
      )
      .waitFor();
    await save.click();
  }
  assert.equal(
    await page.evaluate(
      () =>
        window.calls.filter((x) => x.cmd === "set_text_configuration").length,
    ),
    0,
  );
  await effort.selectOption("low");
  await limit.fill("393217");
  await profile.selectOption("deepseek");
  await page
    .getByText(
      "Enter a whole number from 1 to 393216, or leave blank for API default.",
      { exact: true },
    )
    .waitFor();
  await save.click();
  assert.equal(
    await page.evaluate(
      () =>
        window.calls.filter((x) => x.cmd === "set_text_configuration").length,
    ),
    0,
  );
  await limit.fill("1");
  await page.evaluate(() => (window.mode = "save-error"));
  await save.click();
  await page
    .getByText("Unable to save API settings. Please try again.", {
      exact: true,
    })
    .waitFor();
  assert.equal(await effort.inputValue(), "low");
  assert.equal(await limit.inputValue(), "1");
  assert.equal(
    await page.getByText("Local settings saved", { exact: true }).count(),
    0,
  );
  assert.doesNotMatch(
    await page.locator("body").innerText(),
    /Bearer synthetic-secret|private/,
  );
  await page.evaluate(() => (window.mode = "save-pending"));
  await save.click();
  await page.waitForFunction(() => typeof window.releaseSave === "function");
  assert.equal(await save.isDisabled(), true);
  assert.equal(await profile.isDisabled(), true);
  await page.evaluate(() => window.releaseSave());
  await page.getByText("Local settings saved", { exact: true }).waitFor();
  assert.deepEqual(
    await page.evaluate(() => {
      const c = window.configuration;
      return [c.provider_profile, c.effort, c.max_output_tokens];
    }),
    ["deepseek", "low", 1],
  );
  await page.evaluate(() => {
    window.uiRoot.unmount();
  });
  await page.evaluate(async () => {
    const { React, createRoot, PostProcessingSettings } = await import(
      "/__ui_fixture.tsx"
    );
    window.uiRoot = createRoot(document.getElementById("root"));
    window.uiRoot.render(React.createElement(PostProcessingSettings));
  });
  await profile.waitFor();
  assert.equal(await profile.inputValue(), "deepseek");
  assert.equal(await effort.inputValue(), "low");
  assert.equal(await limit.inputValue(), "1");
});

for (const [language, labels] of [
  [
    "en",
    [
      "Provider profile",
      "Effort",
      "Maximum output tokens (blank uses API default)",
      "Save settings",
      "Local settings saved",
    ],
  ],
  [
    "zh-TW",
    [
      "服務設定檔",
      "Effort",
      "輸出 token 上限（留白使用 API 預設）",
      "保存設定",
      "已保存本機設定",
    ],
  ],
  [
    "zh-CN",
    [
      "服务配置",
      "Effort",
      "输出 token 上限（留空使用 API 默认）",
      "保存设置",
      "已保存本机设置",
    ],
  ],
])
  test(`API settings ${language} render and save default controls at narrow width`, async (t) => {
    const page = await historyApiFixture(t, "PostProcessingSettings", language);
    await page.setViewportSize({ width: 360, height: 900 });
    await page.evaluate(() => window.render());
    await page
      .getByRole("combobox", { name: labels[0], exact: true })
      .waitFor();
    await page
      .getByRole("combobox", { name: labels[1], exact: true })
      .selectOption("default");
    await page.getByRole("textbox", { name: labels[2], exact: true }).fill("");
    await page.getByRole("button", { name: labels[3], exact: true }).click();
    await page.getByText(labels[4], { exact: true }).waitFor();
    assert.equal(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= window.innerWidth,
      ),
      true,
    );
    assert.deepEqual(
      await page.evaluate(() => {
        const c = window.configuration;
        return [c.effort, c.max_output_tokens];
      }),
      [null, null],
    );
    assert.doesNotMatch(
      await page.locator("body").innerText(),
      /notype\.text|notype\.metrics|目前無法顯示|\?\?/,
    );
  });
