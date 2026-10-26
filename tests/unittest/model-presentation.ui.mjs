import { readFile } from "node:fs/promises";
import { createRequire } from "node:module";
import assert from "node:assert/strict";
import test from "node:test";
const require = createRequire(
  new URL("../../desktop/package.json", import.meta.url),
);
const ts = require("typescript");
const { createInstance } = require("i18next");
const source = await readFile(
  new URL("../../desktop/src/lib/utils/modelTranslation.ts", import.meta.url),
  "utf8",
);
const js = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.ESNext },
}).outputText;
const { getTranslatedModelName, getTranslatedModelDescription } = await import(
  `data:text/javascript;base64,${Buffer.from(js).toString("base64")}`
);

test("model presentation falls back to real metadata with the app missing-key handler", async () => {
  const i18n = createInstance();
  await i18n.init({
    lng: "zh-TW",
    fallbackLng: "zh-TW",
    resources: {
      "zh-TW": {
        translation: {
          onboarding: {
            models: {
              translated: { name: "目錄譯名", description: "目錄說明" },
            },
          },
        },
      },
    },
    parseMissingKeyHandler: () => "目前無法顯示這段文字，請稍後重試。",
  });
  const model = {
    id: "publisher/Qwen3-ASR/file.gguf",
    name: "Qwen3-ASR 0.6B Q6_K",
    description: "Multilingual speech recognition.",
    is_custom: false,
  };
  assert.equal(getTranslatedModelName(model, i18n), "Qwen3-ASR 0.6B Q6_K");
  assert.equal(
    getTranslatedModelDescription(model, i18n),
    "Multilingual speech recognition.",
  );
  assert.equal(
    getTranslatedModelName({ ...model, id: "translated" }, i18n),
    "目錄譯名",
  );
  assert.equal(
    getTranslatedModelDescription({ ...model, id: "translated" }, i18n),
    "目錄說明",
  );
});

test("Simplified Chinese recovery has direct resources without Traditional Chinese fallback", async () => {
  const resources = {};
  for (const language of ["zh-CN", "zh-TW"]) {
    resources[language] = {
      translation: JSON.parse(
        await readFile(
          new URL(
            `../../desktop/src/i18n/locales/${language}/translation.json`,
            import.meta.url,
          ),
          "utf8",
        ),
      ),
    };
  }
  const i18n = createInstance();
  await i18n.init({ lng: "zh-CN", fallbackLng: "zh-TW", resources });
  for (const [key, expected] of Object.entries({
    title: "首次设置尚未完成",
    description:
      "离线包默认设置尚未完整保存，录音与 API 操作暂停。仍可查看文字历史、安全设置及模型文件夹；如有需要，请选择有效的模型文件夹后重试。",
    retry: "重试设置",
    retrying: "重试中…",
  })) {
    const path = `notype.bootstrap.${key}`;
    assert.equal(
      i18n.getResource("zh-CN", "translation", path),
      expected,
      path,
    );
    assert.equal(i18n.t(path), expected);
  }
});
