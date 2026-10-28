import assert from "node:assert/strict";
import test from "node:test";
import { readFile } from "node:fs/promises";
import { createRequire } from "node:module";
const require = createRequire(
  new URL("../../desktop/package.json", import.meta.url),
);
const ts = require("typescript");
const source = await readFile(
  new URL("../../desktop/src/lib/connectionIndicator.ts", import.meta.url),
  "utf8",
);
const js = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.ESNext },
}).outputText;
const { connectionIndicator } = await import(
  "data:text/javascript;base64," + Buffer.from(js).toString("base64")
);
const selected = {
  configurationLoaded: true,
  readFailed: false,
  enabled: true,
  hasSelection: true,
  testSucceeded: true,
};

test("a tested but disabled or unselected AI server cannot appear enabled-green", () => {
  assert.deepEqual(connectionIndicator(selected), {
    state: "succeeded",
    color: "bg-green-400",
  });
  assert.deepEqual(connectionIndicator({ ...selected, enabled: false }), {
    state: "disabled",
    color: "bg-mid-gray/60",
  });
  assert.equal(
    connectionIndicator({ ...selected, hasSelection: false }).state,
    "unselected",
  );
});

test("unavailable or missing evidence never inherits an old successful indicator", () => {
  assert.equal(
    connectionIndicator({ ...selected, readFailed: true }).state,
    "unavailable",
  );
  assert.equal(
    connectionIndicator({ ...selected, configurationLoaded: false }).state,
    "loading",
  );
  assert.equal(
    connectionIndicator({ ...selected, testSucceeded: null }).state,
    "untested",
  );
  assert.deepEqual(connectionIndicator({ ...selected, testSucceeded: false }), {
    state: "failed",
    color: "bg-red-400",
  });
});
