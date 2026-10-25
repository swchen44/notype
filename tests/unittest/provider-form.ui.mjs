import assert from "node:assert/strict";
import test from "node:test";
import { readFile } from "node:fs/promises";
import { createRequire } from "node:module";
const require = createRequire(
  new URL("../../desktop/package.json", import.meta.url),
);
const ts = require("typescript");
const source = await readFile(
  new URL("../../desktop/src/lib/providerForm.ts", import.meta.url),
  "utf8",
);
const js = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.ESNext },
}).outputText;
const { optionalNumber, endpointPreview } = await import(
  "data:text/javascript;base64," + Buffer.from(js).toString("base64")
);

test("optional API numbers distinguish an omitted default from explicit zero and invalid input", () => {
  assert.deepEqual(optionalNumber("", { min: 0, max: 2 }), {
    valid: true,
    value: null,
  });
  assert.deepEqual(optionalNumber("0", { min: 0, max: 2 }), {
    valid: true,
    value: 0,
  });
  assert.deepEqual(optionalNumber(".25", { min: 0, max: 2 }), {
    valid: true,
    value: 0.25,
  });
  for (const raw of ["NaN", "Infinity", "0x1", "-1", "2.1"])
    assert.equal(optionalNumber(raw, { min: 0, max: 2 }).valid, false);
  assert.equal(
    optionalNumber("0", { min: 0, max: 1, exclusiveMin: true }).valid,
    false,
  );
  assert.equal(
    optionalNumber("1.5", { min: 1, max: 1000, integer: true }).valid,
    false,
  );
  assert.deepEqual(
    optionalNumber("512", { min: 1, max: 1000, integer: true }),
    { valid: true, value: 512 },
  );
});

test("endpoint preview preserves prefixes and rejects unsafe or mistaken full base inputs", () => {
  assert.equal(
    endpointPreview({
      address_mode: "base_url",
      api_address: "https://api.example.invalid/prefix/v1/",
    }),
    "https://api.example.invalid/prefix/v1/chat/completions",
  );
  assert.equal(
    endpointPreview({
      address_mode: "full_chat_endpoint",
      api_address: "https://api.example.invalid/custom/chat",
    }),
    "https://api.example.invalid/custom/chat",
  );
  assert.equal(
    endpointPreview({
      address_mode: "base_url",
      api_address: "http://127.0.0.1:11434/v1",
    }),
    "http://127.0.0.1:11434/v1/chat/completions",
  );
  for (const api_address of [
    "https://api.example.invalid/chat/completions",
    "https://secret@api.example.invalid/v1",
    "https://api.example.invalid/v1?key=secret",
    "http://127.evil.invalid/v1",
    "http://10.0.0.1/v1",
  ])
    assert.equal(
      endpointPreview({ address_mode: "base_url", api_address }),
      null,
    );
});
