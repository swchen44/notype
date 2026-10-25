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
