import { test } from "node:test";
import assert from "node:assert/strict";
import { writeFile } from "node:fs/promises";
import { join } from "node:path";
import { desktopFixture, invoke } from "./desktop-driver.mjs";

test(
  "debug input capture rejects missing or mismatched fixture ownership before any finish or delivery",
  { timeout: 60000 },
  async () => {
    const fixture = await desktopFixture();
    const observations = [];
    let app;
    try {
      app = await fixture.launch();
      for (const [name, owner] of [
        ["missing", undefined],
        [
          "mismatched",
          {
            pid: 0,
            image:
              "C:\\notype-owned-fixture-does-not-exist\\NativeInputFixture.exe",
          },
        ],
      ]) {
        let rejected = false;
        try {
          await invoke(app.page, "debug_begin_session", {
            captureInput: true,
            fixtureOwner: owner,
            requireInputTarget: true,
          });
        } catch (error) {
          rejected = /owned-fixture/.test(String(error));
        } finally {
          // An old binary incorrectly accepts this request. Cancel without finishing,
          // creating history, invoking delivery or launching any fixture control.
          await invoke(app.page, "cancel_operation");
        }
        observations.push({
          name,
          rejected,
          finished: false,
          delivered: false,
        });
      }
      assert.equal(
        (await invoke(app.page, "get_history_entries", { query: "" })).length,
        0,
      );
      if (process.env.NOTYPE_DATA_DIR)
        await writeFile(
          join(process.env.NOTYPE_DATA_DIR, "scope-negative.json"),
          JSON.stringify(observations, null, 2),
        );
      assert.deepEqual(
        observations.map((x) => x.rejected),
        [true, true],
      );
    } finally {
      if (app) await fixture.close(app);
      await fixture.dispose();
    }
  },
);
