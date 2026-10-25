import { test } from "node:test";
import assert from "node:assert/strict";
import { setTimeout as delay } from "node:timers/promises";
import { desktopFixture, invoke } from "./desktop-driver.mjs";

test(
  "recognition preference persists independently of the interface locale",
  { timeout: 180000 },
  async () => {
    const fixture = await desktopFixture();
    let app;
    try {
      app = await fixture.launch();
      assert.equal(
        (await invoke(app.page, "get_app_settings")).recognition_mode,
        "traditional_english",
      );
      await app.page
        .getByRole("combobox", { name: "辨識模式", exact: true })
        .selectOption("simplified_english");
      await invoke(app.page, "change_app_language_setting", {
        language: "zh-TW",
      });
      const saved = await invoke(app.page, "get_app_settings");
      assert.equal(saved.recognition_mode, "simplified_english");
      assert.equal(saved.app_language, "zh-TW");
      await delay(300);
      await fixture.close(app);
      app = await fixture.launch();
      assert.equal(
        (await invoke(app.page, "get_app_settings")).recognition_mode,
        "simplified_english",
      );
      const profile = app.page.getByRole("combobox", {
        name: "辨識模式",
        exact: true,
      });
      assert.equal(await profile.inputValue(), "simplified_english");
      assert.equal(
        await profile.locator("option:checked").textContent(),
        "簡體中文＋English",
      );
    } finally {
      if (app) await fixture.close(app);
      await fixture.dispose();
    }
  },
);

test(
  "a recording keeps its captured profile when preferences change during STT",
  { timeout: 180000 },
  async () => {
    const fixture = await desktopFixture();
    let app;
    try {
      app = await fixture.launch();
      await invoke(app.page, "change_recognition_mode_setting", {
        mode: "traditional_english",
      });
      const generation = await invoke(app.page, "debug_begin_session", {
        recognitionMode: null,
      });
      await invoke(app.page, "change_recognition_mode_setting", {
        mode: "simplified_english",
      });
      await invoke(app.page, "change_app_language_setting", {
        language: "zh-CN",
      });
      const entry = await invoke(app.page, "debug_finish_session", {
        generation,
        transcript: "今天不要删除 API，保留 42 个 request，头发和发展。",
      });
      assert.equal(entry.recognition_mode, "traditional_english");
      assert.equal(
        entry.transcript,
        "今天不要刪除 API，保留 42 個 request，頭髮和發展。",
      );
      const stored = await invoke(app.page, "get_history_entries", {
        query: "API",
      });
      assert.equal(stored[0].transcript, entry.transcript);
      await fixture.close(app);
      app = await fixture.launch();
      const reopened = await invoke(app.page, "get_history_entries", {
        query: "API",
      });
      assert.equal(reopened[0].transcript, entry.transcript);
      assert.equal(reopened[0].recognition_mode, "traditional_english");
    } finally {
      if (app) await fixture.close(app);
      await fixture.dispose();
    }
  },
);

test(
  "Simplified interface and unknown-locale Traditional fallback survive restart",
  { timeout: 180000 },
  async () => {
    const fixture = await desktopFixture();
    let app;
    try {
      app = await fixture.launch();
      await invoke(app.page, "change_app_language_setting", {
        language: "zh-CN",
      });
      await delay(300);
      await fixture.close(app);
      app = await fixture.launch();
      assert.equal(
        await app.page.locator("html").getAttribute("lang"),
        "zh-CN",
      );
      await app.page
        .getByRole("combobox", { name: "辨识模式", exact: true })
        .waitFor();
      assert.equal(
        await app.page
          .getByRole("combobox", { name: "辨识模式", exact: true })
          .count(),
        1,
      );
      await invoke(app.page, "change_app_language_setting", {
        language: "not-a-locale",
      });
      await delay(300);
      await fixture.close(app);
      app = await fixture.launch();
      assert.equal(
        await app.page.locator("html").getAttribute("lang"),
        "zh-TW",
      );
      await app.page
        .getByRole("combobox", { name: "辨識模式", exact: true })
        .waitFor();
      assert.equal(
        await app.page
          .getByRole("combobox", { name: "辨識模式", exact: true })
          .count(),
        1,
      );
    } finally {
      if (app) await fixture.close(app);
      await fixture.dispose();
    }
  },
);
