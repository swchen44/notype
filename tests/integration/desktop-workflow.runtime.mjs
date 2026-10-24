import { test } from "node:test";
import assert from "node:assert/strict";
import { readdir, readFile, writeFile, mkdir } from "node:fs/promises";
import { desktopFixture, invoke } from "./desktop-driver.mjs";
import { createServer } from "node:http";
import { once } from "node:events";
import { setTimeout as delay } from "node:timers/promises";
import { DatabaseSync } from "node:sqlite";
import { join } from "node:path";

test(
  "native workflow preserves original without configuration or WAV across actual restart",
  { timeout: 120000 },
  async () => {
    const fixture = await desktopFixture();
    try {
      let app = await fixture.launch();
      const config = await invoke(app.page, "get_text_configuration");
      assert.equal(config.transport, "disabled");
      assert.equal(config.endpoint, "");
      assert.equal(config.model, "");
      const generation = await invoke(app.page, "debug_begin_session", {
        recognitionMode: "simplified_english",
      });
      const entry = await invoke(app.page, "debug_finish_session", {
        generation,
        transcript: "不要删除 API_42 [不明]",
      });
      assert.equal(entry.status, "failed");
      assert.equal(entry.error, "configuration-required");
      assert.equal(entry.transcript, "不要删除 API_42 [不明]");
      assert.equal(entry.result, null);
      assert.equal(entry.recognition_mode, "simplified_english");
      const found = await invoke(app.page, "get_history_entries", {
        query: "API_42",
      });
      assert.deepEqual(
        found.map((row) => row.id),
        [entry.id],
      );
      assert.ok(!(await readdir(fixture.directory)).includes("recordings"));
      await fixture.close(app);
      app = await fixture.launch();
      const restored = await invoke(app.page, "get_history_entries", {
        query: "API_42",
      });
      assert.equal(restored[0].id, entry.id);
      assert.equal(restored[0].created_at, entry.created_at);
      assert.equal(restored[0].transcript, entry.transcript);
      assert.equal(restored[0].error, "configuration-required");
      await fixture.close(app);
    } finally {
      await fixture.dispose();
    }
  },
);

test(
  "native history UI searches originals and offers explicit original selection and manual delete",
  { timeout: 120000 },
  async () => {
    const fixture = await desktopFixture();
    try {
      const app = await fixture.launch();
      const generation = await invoke(app.page, "debug_begin_session", {
        recognitionMode: "traditional_english",
      });
      const entry = await invoke(app.page, "debug_finish_session", {
        generation,
        transcript: "原稿 UI_API_42 不要刪除",
      });
      await app.page
        .getByText("歷史紀錄", { exact: true })
        .click({ timeout: 3000 });
      const search = app.page.getByRole("searchbox", { name: "搜尋文字歷史" });
      await search.waitFor({ timeout: 3000 });
      await app.page
        .getByText("原稿 UI_API_42 不要刪除", { exact: true })
        .waitFor();
      assert.equal(await app.page.locator("audio").count(), 0);
      await search.fill("UI_API");
      await app.page
        .getByRole("button", { name: "選用原稿", exact: true })
        .click();
      await app.page.getByText("已選用原稿", { exact: true }).waitFor();
      assert.equal(
        (await invoke(app.page, "get_history_entries", { query: "UI_API" }))[0]
          .selection,
        "original",
      );
      await search.fill("no matching text");
      await app.page.getByText("沒有符合的文字歷史", { exact: true }).waitFor();
      const other = await invoke(app.page, "debug_begin_session", {
        recognitionMode: "simplified_english",
      });
      await invoke(app.page, "debug_finish_session", {
        generation: other,
        transcript: "another unrelated original",
      });
      assert.equal(
        await app.page
          .getByText("another unrelated original", { exact: true })
          .count(),
        0,
      );
      await search.fill("UI_API");
      await app.page.getByRole("button", { name: "刪除", exact: true }).click();
      await app.page.getByText("沒有符合的文字歷史", { exact: true }).waitFor();
      assert.equal(
        (await invoke(app.page, "get_history_entries", { query: "UI_API" }))
          .length,
        0,
      );
      assert.equal(
        (await invoke(app.page, "get_history_entries", { query: "another" }))
          .length,
        1,
      );
      assert.ok(entry.id);
      await fixture.close(app);
    } finally {
      await fixture.dispose();
    }
  },
);

test(
  "visible native text settings drive real HTTP editing, history retry, timeout and cancellation",
  { timeout: 120000 },
  async () => {
    const fixture = await desktopFixture();
    const requests = [];
    const replies = [];
    const response = (text) => ({
      choices: [
        {
          message: { role: "assistant", content: text },
          finish_reason: "stop",
        },
      ],
    });
    const server = createServer(async (req, res) => {
      let body = "";
      for await (const chunk of req) body += chunk;
      requests.push({
        method: req.method,
        url: req.url,
        body: body ? JSON.parse(body) : null,
      });
      const reply = replies.shift() ?? {
        status: 500,
        body: { error: "unexpected-request" },
      };
      if (reply.gate) await reply.gate;
      await delay(reply.delay ?? 0);
      res.writeHead(reply.status ?? 200, {
        "content-type": "application/json",
      });
      res.end(JSON.stringify(reply.body));
    });
    server.listen(0, "127.0.0.1");
    await once(server, "listening");
    const endpoint = `http://127.0.0.1:${server.address().port}/v1/chat/completions`;
    const waitRequest = async (count) => {
      for (let attempt = 0; attempt < 300; attempt++) {
        if (requests.length >= count) return;
        await delay(10);
      }
      assert.fail("Expected HTTP fixture request");
    };
    try {
      const app = await fixture.launch();
      await app.page
        .getByText("文字整理", { exact: true })
        .first()
        .click({ timeout: 3000 });
      await app.page
        .getByLabel("服務格式", { exact: true })
        .selectOption("open_ai_chat");
      await app.page.getByLabel("完整請求 URL", { exact: true }).fill(endpoint);
      await app.page
        .getByLabel("模型 ID（手動輸入）", { exact: true })
        .fill("native-fixture-model");
      await app.page
        .getByLabel("認證方式", { exact: true })
        .selectOption("none");
      await app.page
        .getByLabel("逾時秒數（最多 120 秒）", { exact: true })
        .fill("1");
      await app.page
        .getByRole("button", { name: "保存設定", exact: true })
        .click();
      await app.page
        .getByText("設定完整，可從文字歷史重試整理。", { exact: true })
        .waitFor();
      await delay(50);
      assert.equal(
        requests.length,
        0,
        "Editing local config never discovers models or sends user text",
      );
      await invoke(app.page, "update_custom_words", { words: ["session_id"] });
      replies.push({
        body: response("不要刪除 API 的 session_id，保留 42 個 request。"),
      });
      const generation = await invoke(app.page, "debug_begin_session", {
        recognitionMode: "traditional_english",
      });
      const raw = "那個不要刪除 API 的 session_id 保留42個request";
      const entry = await invoke(app.page, "debug_finish_session", {
        generation,
        transcript: raw,
      });
      assert.equal(entry.status, "pending");
      assert.equal(entry.transcript, raw);
      assert.equal(
        entry.result,
        "不要刪除 API 的 session_id，保留 42 個 request。",
      );
      assert.equal(requests[0].url, "/v1/chat/completions");
      assert.deepEqual(JSON.parse(requests[0].body.messages.at(-1).content), {
        transcript: raw,
        glossary: ["session_id"],
        recognition_mode: "traditional_english",
      });
      await app.page.getByText("歷史紀錄", { exact: true }).click();
      const row = app.page.locator(`[data-session-id="${entry.id}"]`);
      await row.getByText(entry.result, { exact: true }).waitFor();
      await row.getByRole("button", { name: "複製原稿", exact: true }).click();
      await row.getByText("已複製原稿", { exact: true }).waitFor();
      await row
        .getByRole("button", { name: "複製整理結果", exact: true })
        .click();
      await row.getByText("已複製整理結果", { exact: true }).waitFor();
      assert.equal(await app.page.locator("audio").count(), 0);
      if (process.env.NOTYPE_NATIVE_SCREENSHOT)
        await app.page.screenshot({
          path: process.env.NOTYPE_NATIVE_SCREENSHOT,
        });
      replies.push({
        body: {
          choices: [
            { message: { content: { transcription: "invalid-object" } } },
          ],
        },
      });
      await row.getByRole("button", { name: "重試整理", exact: true }).click();
      await row.getByText("整理失敗，原稿已保存", { exact: true }).waitFor();
      assert.equal(
        (
          await invoke(app.page, "get_history_entries", { query: "session_id" })
        )[0].result,
        null,
      );
      replies.push({ body: response("重試成功，保留 session_id 與 42。") });
      await row.getByRole("button", { name: "重試整理", exact: true }).click();
      await row
        .getByText("重試成功，保留 session_id 與 42。", { exact: true })
        .waitFor();
      assert.equal(
        (
          await invoke(app.page, "get_history_entries", { query: "session_id" })
        )[0].transcript,
        raw,
      );
      replies.push({ body: response("too late"), delay: 1400 });
      await row.getByRole("button", { name: "重試整理", exact: true }).click();
      await row
        .getByText("文字整理逾時，原稿已保存。", { exact: true })
        .waitFor();
      replies.push({ body: response("cancelled late response"), delay: 300 });
      await row.getByRole("button", { name: "重試整理", exact: true }).click();
      await waitRequest(5);
      await row.getByRole("button", { name: "取消處理", exact: true }).click();
      await row.getByText("已取消", { exact: true }).waitFor();
      await delay(350);
      const saved = (
        await invoke(app.page, "get_history_entries", { query: "session_id" })
      )[0];
      assert.equal(saved.status, "cancelled");
      assert.equal(saved.result, null);
      assert.equal(saved.transcript, raw);
      let releaseRetry;
      const retryGate = new Promise((resolve) => {
        releaseRetry = resolve;
      });
      replies.push({
        body: response("history retry survives unrelated cancellation"),
        gate: retryGate,
      });
      const independentRetry = invoke(
        app.page,
        "retry_history_entry_processing",
        { id: entry.id },
      );
      await waitRequest(6);
      await invoke(app.page, "debug_begin_session", {
        recognitionMode: "simplified_english",
      });
      await invoke(app.page, "cancel_operation");
      releaseRetry();
      assert.equal((await independentRetry).status, "pending");
      let releaseActive;
      const activeGate = new Promise((resolve) => {
        releaseActive = resolve;
      });
      replies.push({ body: response("must be ignored"), gate: activeGate });
      const activeGeneration = await invoke(app.page, "debug_begin_session", {
        recognitionMode: "simplified_english",
      });
      const activeCompletion = invoke(app.page, "debug_finish_session", {
        generation: activeGeneration,
        transcript: "active recording original",
      });
      await waitRequest(7);
      await invoke(app.page, "cancel_operation");
      const cancelledActive = await activeCompletion;
      releaseActive();
      assert.equal(cancelledActive.status, "cancelled");
      assert.equal(cancelledActive.result, null);
      assert.equal(cancelledActive.transcript, "active recording original");
      assert.ok(
        requests.every(
          (request) =>
            request.method === "POST" && request.url === "/v1/chat/completions",
        ),
      );
      assert.equal(requests.length, 7);
      assert.ok(!(await readdir(fixture.directory)).includes("recordings"));
      await fixture.close(app);
      const restored = await fixture.launch();
      assert.equal(
        (await invoke(restored.page, "get_text_configuration")).model,
        "native-fixture-model",
      );
      assert.equal(
        (
          await invoke(restored.page, "get_history_entries", {
            query: "session_id",
          })
        )[0].id,
        entry.id,
      );
      await fixture.close(restored);
    } finally {
      await fixture.dispose();
      server.closeAllConnections();
      await new Promise((done) => server.close(done));
    }
  },
);

test(
  "native startup preserves Node v1 destination history and imports Handy text without touching source audio",
  { timeout: 120000 },
  async () => {
    const fixture = await desktopFixture();
    try {
      const destination = new DatabaseSync(
        join(fixture.directory, "sessions.db"),
      );
      destination.exec(`CREATE TABLE sessions (id TEXT PRIMARY KEY,transcript TEXT NOT NULL,result TEXT,created_at TEXT NOT NULL,updated_at TEXT NOT NULL,status TEXT NOT NULL,error TEXT);
      INSERT INTO sessions VALUES ('old-node-id','Node 原稿 API 42',NULL,'2026-10-24T00:00:00.000Z','2026-10-24T00:01:00.000Z','processing',NULL); PRAGMA user_version=1;`);
      destination.close();
      const source = join(fixture.directory, "history.db");
      const legacy = new DatabaseSync(source);
      legacy.exec(`CREATE TABLE transcription_history (id INTEGER PRIMARY KEY,file_name TEXT NOT NULL,timestamp INTEGER NOT NULL,saved BOOLEAN NOT NULL,title TEXT NOT NULL,transcription_text TEXT NOT NULL,post_processed_text TEXT,post_process_prompt TEXT,post_process_requested BOOLEAN NOT NULL DEFAULT 0);
      INSERT INTO transcription_history VALUES (1,'handy-1.wav',1700000000,0,'old title','Handy 原稿 API 7','Handy 整理結果 API 7。',NULL,1); PRAGMA user_version=4;`);
      legacy.close();
      const audioDirectory = join(fixture.directory, "recordings");
      await mkdir(audioDirectory);
      const audio = join(audioDirectory, "handy-1.wav");
      await writeFile(audio, "legacy audio sentinel");
      const originalDatabase = await readFile(source);
      let app = await fixture.launch();
      const oldNode = (
        await invoke(app.page, "get_history_entries", { query: "Node" })
      )[0];
      assert.equal(oldNode.id, "old-node-id");
      assert.equal(oldNode.transcript, "Node 原稿 API 42");
      assert.equal(oldNode.created_at, "2026-10-24T00:00:00.000Z");
      assert.equal(oldNode.status, "failed");
      assert.equal(oldNode.error, "interrupted");
      const imported = (
        await invoke(app.page, "get_history_entries", { query: "Handy" })
      )[0];
      assert.equal(imported.transcript, "Handy 原稿 API 7");
      assert.equal(imported.result, "Handy 整理結果 API 7。");
      assert.equal(imported.recognition_mode, null);
      assert.equal(imported.status, "pending");
      assert.equal(
        (await invoke(app.page, "get_workflow_readiness")).legacy_import_error,
        null,
      );
      await app.page.getByText("歷史紀錄", { exact: true }).click();
      await app.page
        .getByText("Handy 整理結果 API 7。", { exact: true })
        .waitFor();
      assert.equal(await app.page.locator("audio").count(), 0);
      await invoke(app.page, "delete_history_entry", { id: imported.id });
      await fixture.close(app);
      app = await fixture.launch();
      assert.equal(
        (await invoke(app.page, "get_history_entries", { query: "Handy" }))
          .length,
        0,
        "Repeat import does not restore deleted text",
      );
      assert.equal(
        (await invoke(app.page, "get_history_entries", { query: "Node" }))[0]
          .id,
        "old-node-id",
      );
      assert.deepEqual(await readFile(source), originalDatabase);
      assert.equal(await readFile(audio, "utf8"), "legacy audio sentinel");
      assert.deepEqual(await readdir(audioDirectory), ["handy-1.wav"]);
      await fixture.close(app);
    } finally {
      await fixture.dispose();
    }
  },
);
