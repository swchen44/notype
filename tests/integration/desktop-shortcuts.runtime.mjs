import { test } from "node:test";
import assert from "node:assert/strict";
import { desktopFixture, invoke } from "./desktop-driver.mjs";

test(
  "three independent tap starts and common Stop persist, validate conflicts and reset to product defaults",
  { timeout: 60000 },
  async () => {
    const fixture = await desktopFixture();
    let app;
    try {
      app = await fixture.launch();
      for (const [id, binding] of [
        ["transcribe", "f8"],
        ["translation", "f9"],
        ["ask", "f10"],
        ["stop", "f11"],
      ]) {
        const response = await invoke(app.page, "change_binding", {
          id,
          binding,
        });
        assert.equal(response.success, true);
      }
      await assert.rejects(
        invoke(app.page, "change_binding", {
          id: "translation",
          binding: "f8",
        }),
        /shortcut-start-conflict/,
      );
      await assert.rejects(
        invoke(app.page, "change_binding", { id: "ask", binding: "escape" }),
        /shortcut/,
      );
      let settings = await invoke(app.page, "get_app_settings");
      assert.equal(settings.bindings.stop.current_binding, "f11");
      assert.equal(settings.bindings.translation.current_binding, "f9");
      await fixture.close(app);
      app = await fixture.launch();
      settings = await invoke(app.page, "get_app_settings");
      assert.equal(settings.bindings.transcribe.current_binding, "f8");
      assert.equal(settings.bindings.stop.current_binding, "f11");
      for (const [id, want] of [
        ["transcribe", "alt_right"],
        ["translation", "alt_right+shift_right"],
        ["ask", "alt_right+space"],
        ["stop", "alt_right"],
      ]) {
        const response = await invoke(app.page, "reset_binding", { id });
        assert.equal(response.success, true);
        assert.equal(response.binding.current_binding, want);
      }
      await app.page.getByLabel("開始口述", { exact: true }).waitFor();
      assert.equal(
        await app.page.getByLabel("停止當次錄音", { exact: true }).inputValue(),
        "alt_right",
      );
      await app.page
        .getByText(
          "Esc 可取消錄音或處理。不支援的輸入欄位會保留待選結果，供手動複製。",
          { exact: true },
        )
        .waitFor();
      await fixture.close(app);
      app = null;
    } finally {
      if (app) await fixture.close(app);
      await fixture.dispose();
    }
  },
);
