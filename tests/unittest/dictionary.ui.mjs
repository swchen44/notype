import assert from "node:assert/strict";
import test from "node:test";
import { readFile } from "node:fs/promises";
import { createRequire } from "node:module";
const require = createRequire(
  new URL("../../desktop/package.json", import.meta.url),
);
const ts = require("typescript");
const source = await readFile(
  new URL("../../desktop/src/lib/dictionary.ts", import.meta.url),
  "utf8",
);
const js = ts.transpileModule(source, {
  compilerOptions: {
    module: ts.ModuleKind.ESNext,
    target: ts.ScriptTarget.ES2022,
  },
}).outputText;
const { dictionaryCsv, parseDictionaryCsv, mergeWords, dictionaryPage } =
  await import(
    "data:text/javascript;base64," + Buffer.from(js).toString("base64")
  );

test("dictionary pages bound the rendered rows, search all words and clamp deleted pages", () => {
  const words = Array.from(
    { length: 250 },
    (_, i) => `term-${String(i + 1).padStart(3, "0")}`,
  );
  const first = dictionaryPage(words, "", 1);
  assert.equal(first.items.length, 100);
  assert.equal(first.items[0], "term-001");
  assert.equal(first.pages, 3);
  assert.equal(first.total, 250);
  const last = dictionaryPage(words, "", 3);
  assert.equal(last.items.length, 50);
  assert.equal(last.items[49], "term-250");
  const search = dictionaryPage(words, "TERM-250", 1);
  assert.deepEqual(search.items, ["term-250"]);
  assert.equal(search.pages, 1);
  assert.equal(search.matched, 1);
  const clamped = dictionaryPage(words.slice(0, 150), "", 3);
  assert.equal(clamped.page, 2);
  assert.equal(clamped.items[0], "term-101");
});

test("CSV is UTF-8 BOM and preserves Chinese, commas and quotes when reimported", () => {
  const words = ["繁體中文", "简体中文", "OpenAI", "name, value", 'a"b'];
  const csv = dictionaryCsv(words);
  assert.deepEqual(
    [...Buffer.from(csv, "utf8").subarray(0, 3)],
    [239, 187, 191],
  );
  assert.match(csv, /"name, value"/);
  assert.match(csv, /"a""b"/);
  assert.deepEqual(parseDictionaryCsv(csv), words);
});
test("imports preserve first spelling and count only newly added incoming words", () => {
  assert.deepEqual(
    mergeWords(
      [" OpenAI ", "openai", "既有"],
      ["OPENAI", " Docker ", "docker", "新詞"],
    ),
    {
      words: ["OpenAI", "既有", "Docker", "新詞"],
      duplicates: 3,
      added: 2,
    },
  );
});

test("spreadsheet formulas are neutralized in CSV while normalized words round-trip", () => {
  const words = ["=1+1", "+SUM(A1)", "-10", "@term"];
  const csv = dictionaryCsv(words);
  for (const word of words) assert.ok(csv.includes('"\t' + word + '"'));
  assert.deepEqual(parseDictionaryCsv(csv), words);
});
test("malformed or wrong-column CSV is rejected as a whole", () => {
  for (const csv of [
    "wrong\r\nword",
    "word,extra\r\na,b",
    'word\r\n"unclosed',
    'word\r\n"a"z',
    'word\r\n"a\nb"',
    "word\r\na\u0085b",
    "word\r\n" + "a".repeat(51),
  ]) {
    assert.throws(() => parseDictionaryCsv(csv), /dictionary-csv-invalid/);
  }
});
