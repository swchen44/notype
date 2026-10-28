import assert from "node:assert/strict";
import test from "node:test";
import { uiFixture } from "./fixtures/ui.mjs";

async function fixture(t, language = "en") {
  const page = await uiFixture(t, {
    cacheName: "settings-transfer",
    entry: `export {default as React} from 'react'; export {createRoot} from 'react-dom/client'; export {SettingsTransfer} from '/src/components/settings/advanced/SettingsTransfer.tsx'; export {default as i18n} from '/src/i18n/index.ts'; import '/src/App.css';`,
  });
  page.setDefaultTimeout(3000);
  await page.evaluate(async (language) => {
    window.calls = [];
    window.preview = {
      id: "owned-preview",
      before_yaml:
        "custom_words: [舊詞]\nai:\n  app_keys:\n    fixture: '********'",
      after_yaml: "custom_words: [新詞]",
      changed_fields: ["settings.custom_words"],
      key_effects: [{ name: "Synthetic server", effect: "kept" }],
      conflicts: [],
    };
    window.responses = {};
    window.__TAURI_INTERNALS__ = {
      invoke: async (cmd, args) => {
        window.calls.push({ cmd, args });
        if (window.responses[cmd] === "error") throw "settings-transfer-stale";
        if (cmd === "export_settings_yaml") return { status: "saved" };
        if (cmd === "preview_settings_import") return window.preview;
        if (cmd === "resolve_settings_import_folder")
          return { ...window.preview, conflicts: [] };
        if (cmd === "resolve_settings_import_audio")
          return { ...window.preview, conflicts: [] };
        if (cmd === "cancel_settings_import") return null;
        if (cmd === "apply_settings_import") return { status: "restarting" };
        throw new Error("Unexpected fixture IPC: " + cmd);
      },
    };
    const { React, createRoot, SettingsTransfer, i18n } = await import(
      "/__ui_fixture.tsx"
    );
    await i18n.changeLanguage(language);
    createRoot(document.getElementById("root")).render(
      React.createElement(SettingsTransfer),
    );
  }, language);
  return page;
}
test("YAML export excludes keys by default; included keys are labelled plaintext", async (t) => {
  const page = await fixture(t);
  assert.equal(await page.getByRole("checkbox").isChecked(), false);
  await page.getByRole("button", { name: "Export YAML", exact: true }).click();
  await page
    .getByRole("status")
    .filter({ hasText: "Settings exported" })
    .waitFor();
  assert.equal(
    await page.evaluate(
      () =>
        window.calls.find((c) => c.cmd === "export_settings_yaml").args
          .includeKeys,
    ),
    false,
  );
  await page.getByRole("checkbox").check();
  await page
    .getByRole("note")
    .filter({ hasText: "plaintext API keys" })
    .waitFor();
  await page.getByRole("button", { name: "Export YAML", exact: true }).click();
  await page.waitForFunction(
    () =>
      window.calls.filter((c) => c.cmd === "export_settings_yaml").length === 2,
  );
  assert.equal(
    await page.evaluate(
      () =>
        window.calls.filter((c) => c.cmd === "export_settings_yaml")[1].args
          .includeKeys,
    ),
    true,
  );
});
test("unavailable audio devices require explicit system-default resolution", async (t) => {
  const page = await fixture(t);
  await page.evaluate(() => {
    window.preview.conflicts = ["audio-device-unavailable"];
  });
  await page.getByRole("button", { name: "Import YAML", exact: true }).click();
  const confirm = page.getByRole("button", {
    name: "Replace settings and restart",
    exact: true,
  });
  assert.equal(await confirm.isDisabled(), true);
  await page
    .getByRole("button", {
      name: "Use system default audio devices",
      exact: true,
    })
    .click();
  await page.waitForFunction(
    () =>
      !Array.from(document.querySelectorAll("button")).find((b) =>
        /Replace settings/.test(b.textContent),
      ).disabled,
  );
  assert.equal(
    await page.evaluate(() =>
      window.calls.some((c) => c.cmd === "apply_settings_import"),
    ),
    false,
  );
});
test("preview is non-applying, cancellation revokes it, stale confirmation requires review", async (t) => {
  const page = await fixture(t);
  await page.getByRole("button", { name: "Import YAML", exact: true }).click();
  await page.getByRole("region", { name: "Import preview" }).waitFor();
  await page.getByText("settings.custom_words", { exact: true }).waitFor();
  await page.getByText("Synthetic server: existing key retained").waitFor();
  assert.equal(
    await page.evaluate(() =>
      window.calls.some((c) => c.cmd === "apply_settings_import"),
    ),
    false,
  );
  await page
    .getByRole("button", { name: "Cancel import", exact: true })
    .click();
  await page.getByRole("status").filter({ hasText: "Cancelled" }).waitFor();
  assert.equal(
    await page.getByRole("region", { name: "Import preview" }).count(),
    0,
  );
  assert.deepEqual(await page.evaluate(() => window.calls.at(-1)), {
    cmd: "cancel_settings_import",
    args: { id: "owned-preview" },
  });
  await page.getByRole("button", { name: "Import YAML", exact: true }).click();
  await page.evaluate(() => {
    window.responses.apply_settings_import = "error";
  });
  await page
    .getByRole("button", { name: "Replace settings and restart", exact: true })
    .click();
  await page
    .getByRole("alert")
    .filter({ hasText: "Cancel and import again" })
    .waitFor();
  assert.equal(
    await page.getByRole("button", { name: "Cancel import" }).isEnabled(),
    true,
  );
});
for (const [language, labels] of Object.entries({
  en: [
    "Import YAML",
    "Choose model folder",
    "Replace settings and restart",
    "Restarting notype",
  ],
  "zh-TW": [
    "匯入 YAML",
    "選擇模型資料夾",
    "取代設定並重新啟動",
    "正在重新啟動",
  ],
  "zh-CN": [
    "导入 YAML",
    "选择模型文件夹",
    "替换设置并重新启动",
    "正在重新启动",
  ],
})) {
  test(`${language}: model conflict blocks confirmation, folder resolution permits one restart`, async (t) => {
    const page = await fixture(t, language);
    await page.setViewportSize({ width: 720, height: 900 });
    await page.evaluate(() => {
      window.preview.conflicts = ["model-folder-unavailable"];
    });
    await page.getByRole("button", { name: labels[0], exact: true }).click();
    const confirm = page.getByRole("button", { name: labels[2], exact: true });
    assert.equal(await confirm.isDisabled(), true);
    await page.getByRole("button", { name: labels[1], exact: true }).click();
    await confirm.waitFor();
    await page.waitForFunction(
      () =>
        !Array.from(document.querySelectorAll("button")).find((b) =>
          /Replace settings|取代設定|替换设置/.test(b.textContent),
        ).disabled,
    );
    await confirm.click();
    await page.getByRole("status").filter({ hasText: labels[3] }).waitFor();
    assert.equal(await confirm.isDisabled(), true);
    assert.equal(
      await page.evaluate(
        () =>
          window.calls.filter((c) => c.cmd === "apply_settings_import").length,
      ),
      1,
    );
    assert.equal(
      await page.evaluate(
        () => document.documentElement.scrollWidth > window.innerWidth,
      ),
      false,
    );
  });
}
