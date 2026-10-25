import { test } from "node:test";
import assert from "node:assert/strict";
import { createServer } from "node:http";
import { once } from "node:events";
import { desktopFixture, invoke } from "./desktop-driver.mjs";

test(
  "visible scenario prompts and selected-only reset persist across native restart",
  { timeout: 120000 },
  async () => {
    const fixture = await desktopFixture();
    let app;
    try {
      app = await fixture.launch();
      const initial = await invoke(app.page, "get_text_configuration");
      await invoke(app.page, "set_text_configuration", {
        configuration: {
          ...initial,
          endpoint: "https://api.example.test/v1/chat/completions",
          model: "preserved-model",
          authentication: "bearer",
          api_key: "NONSECRET_TEST_KEY",
        },
      });
      await app.page.getByText("文字整理", { exact: true }).first().click();
      const scene = app.page.getByLabel("場景", { exact: true });
      await scene.waitFor({ timeout: 4000 });
      const prompt = app.page.getByLabel("可編輯提示詞", { exact: true });
      for (const [id, text] of [
        ["dictation", "DICTATION_USER_STYLE keep API_42."],
        ["ask", "ASK_USER_STYLE answer or write."],
        ["translation", "TRANSLATION_USER_STYLE preserve sense."],
      ]) {
        await scene.selectOption(id);
        await prompt.fill(text);
        await app.page
          .getByRole("button", { name: "儲存此提示詞", exact: true })
          .click();
        await app.page.getByText("已儲存至本機", { exact: true }).waitFor();
      }
      await app.page.getByLabel("翻譯目標語言", { exact: true }).fill("fr-FR");
      await app.page
        .getByLabel("領域（選填）", { exact: true })
        .fill("software");
      await app.page.getByLabel("啟用獨立回譯檢視", { exact: true }).check();
      await app.page
        .getByRole("button", { name: "儲存翻譯設定", exact: true })
        .click();
      await app.page.getByText("已儲存至本機", { exact: true }).waitFor();
      await app.page
        .getByRole("button", { name: "還原此預設提示詞", exact: true })
        .click();
      await app.page
        .getByText("已還原所選預設提示詞", { exact: true })
        .waitFor();
      const config = await invoke(app.page, "get_text_configuration");
      assert.equal(
        config.prompts.dictation,
        "DICTATION_USER_STYLE keep API_42.",
      );
      assert.equal(config.prompts.ask, "ASK_USER_STYLE answer or write.");
      assert.ok(config.prompts.translation.includes("sense-for-sense"));
      assert.equal(config.translation.target_language, "fr-FR");
      assert.equal(config.translation.domain, "software");
      assert.equal(config.translation.back_translation, true);
      assert.equal(config.transport, "disabled");
      assert.equal(
        config.endpoint,
        "https://api.example.test/v1/chat/completions",
      );
      assert.equal(config.model, "preserved-model");
      assert.equal(config.api_key, "NONSECRET_TEST_KEY");
      assert.equal(config.authentication, "bearer");
      await fixture.close(app);
      app = await fixture.launch();
      assert.deepEqual(
        await invoke(app.page, "get_text_configuration"),
        config,
      );
      await app.page.getByText("文字整理", { exact: true }).first().click();
      assert.equal(
        await app.page.getByLabel("可編輯提示詞", { exact: true }).inputValue(),
        "DICTATION_USER_STYLE keep API_42.",
      );
      await fixture.close(app);
      app = null;
    } finally {
      if (app) await fixture.close(app);
      await fixture.dispose();
    }
  },
);

test(
  "native translation history shows exact pair and rejects stale confirmation after retry/restart",
  { timeout: 120000 },
  async () => {
    const fixture = await desktopFixture();
    const requests = [];
    let app;
    let round = 0;
    const server = createServer(async (req, res) => {
      let body = "";
      for await (const chunk of req) body += chunk;
      requests.push(JSON.parse(body));
      const content =
        round++ % 2 === 0
          ? JSON.stringify({
              text: `Keep API_42 revision ${round}.`,
              source_language: "zh-TW",
            })
          : `保留 API_42 回譯 ${round}。`;
      res.writeHead(200, { "content-type": "application/json" });
      res.end(
        JSON.stringify({
          choices: [
            { message: { role: "assistant", content }, finish_reason: "stop" },
          ],
        }),
      );
    });
    server.listen(0, "127.0.0.1");
    await once(server, "listening");
    try {
      app = await fixture.launch();
      let config = await invoke(app.page, "get_text_configuration");
      config = {
        ...config,
        transport: "open_ai_chat",
        authentication: "none",
        endpoint: `http://127.0.0.1:${server.address().port}/v1/chat/completions`,
        model: "fixture",
        timeout_ms: 2000,
      };
      await invoke(app.page, "set_text_configuration", {
        configuration: config,
      });
      await invoke(app.page, "save_translation_settings", {
        translation: { ...config.translation, back_translation: true },
      });
      const generation = await invoke(app.page, "debug_begin_session", {
        recognitionMode: "traditional_english",
        scenario: "translation",
      });
      await invoke(app.page, "save_scenario_prompt", {
        scenario: "translation",
        prompt: "LATE_PROMPT_MUST_NOT_CHANGE_ACTIVE_SESSION",
      });
      const entry = await invoke(app.page, "debug_finish_session", {
        generation,
        transcript: "原始私密口述 API_42",
      });
      assert.equal(entry.scenario, "translation");
      assert.equal(entry.translation_target, "en-US");
      assert.equal(entry.source_language, "zh-TW");
      assert.equal(entry.review_required, true);
      assert.equal(entry.review_confirmed, false);
      assert.equal(entry.error, null);
      assert.equal(requests.length, 2);
      assert.ok(!JSON.stringify(requests[0]).includes("LATE_PROMPT"));
      assert.ok(!JSON.stringify(requests[1]).includes("原始私密口述"));
      await app.page.getByText("歷史紀錄", { exact: true }).click();
      const row = app.page.locator(`[data-session-id="${entry.id}"]`);
      await row.getByText(entry.result, { exact: true }).waitFor();
      await row.getByText(entry.back_translation, { exact: true }).waitFor();
      await row
        .getByRole("button", { name: "確認此顯示版本", exact: true })
        .click();
      await row
        .getByText("已確認此版本；交付仍須核對原輸入目標。", { exact: true })
        .waitFor();
      const retried = await invoke(app.page, "retry_history_entry_processing", {
        id: entry.id,
      });
      assert.equal(retried.review_confirmed, false);
      assert.notEqual(retried.revision, entry.revision);
      await assert.rejects(
        invoke(app.page, "confirm_history_review", {
          id: entry.id,
          revision: entry.revision,
        }),
      );
      await fixture.close(app);
      app = await fixture.launch();
      const saved = (
        await invoke(app.page, "get_history_entries", { query: "API_42" })
      )[0];
      assert.equal(saved.result, retried.result);
      assert.equal(saved.back_translation, retried.back_translation);
      assert.equal(saved.source_language, "zh-TW");
      assert.equal(saved.scenario, "translation");
      assert.equal(saved.review_confirmed, false);
      assert.equal(requests.length, 4);
      await fixture.close(app);
      app = null;
    } finally {
      if (app) await fixture.close(app);
      await fixture.dispose();
      server.close();
      await once(server, "close");
    }
  },
);
