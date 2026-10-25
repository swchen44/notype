import test from "node:test";
import assert from "node:assert/strict";
import { cp, writeFile, unlink, readFile } from "node:fs/promises";
import { spawnSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { portableFixture, json } from "./fixtures/portable-package.mjs";

const source = fileURLToPath(
  new URL("../../packages/notype/", import.meta.url),
);
const supported = process.platform === "win32" && process.arch === "x64";
async function cliFixture(t) {
  const f = await portableFixture(t);
  await cp(path.join(source, "lib"), path.join(f.facade, "lib"), {
    recursive: true,
  });
  await cp(path.join(source, "bin"), path.join(f.facade, "bin"), {
    recursive: true,
  });
  return f;
}
function cli(f, args = []) {
  return spawnSync(
    process.execPath,
    [path.join(f.facade, "bin/notype.mjs"), ...args],
    {
      encoding: "utf8",
      timeout: 10000,
      windowsHide: true,
      env: {
        ...process.env,
        APPDATA: path.join(f.root, "roaming"),
        NOTYPE_DATA_DIR: path.join(f.root, "user-data"),
        NOTYPE_TEST_SECRET: "must-never-appear-in-diagnostics",
      },
    },
  );
}

test("actual CLI refuses a missing runtime before app spawn and does not expose environment values", async (t) => {
  const f = await cliFixture(t);
  await unlink(path.join(f.app, "webview/156.0.4314.42/msedgewebview2.exe"));
  const result = cli(f);
  assert.equal(result.status, 2);
  assert.match(
    result.stderr,
    supported ? /payload-missing/ : /unsupported-platform/,
  );
  assert.doesNotMatch(
    result.stderr + result.stdout,
    /must-never-appear|fixture app|spawn/i,
  );
});

test("actual CLI refuses corrupted model bytes and unassembled releases", async (t) => {
  const f = await cliFixture(t);
  await writeFile(path.join(f.model, "weights/fixture.bin"), "corrupted bytes");
  const corrupt = cli(f);
  assert.equal(corrupt.status, 2);
  assert.match(
    corrupt.stderr,
    supported ? /payload-corrupt/ : /unsupported-platform/,
  );
  await writeFile(
    path.join(f.facade, "release.json"),
    json({ schema: 1, status: "unassembled", version: "0.0.0" }),
  );
  const unassembled = cli(f);
  assert.equal(unassembled.status, 2);
  assert.match(
    unassembled.stderr,
    supported ? /release-not-assembled/ : /unsupported-platform/,
  );
});

test("actual CLI check verifies artifacts without activating a host or writing user data", async (t) => {
  const f = await cliFixture(t);
  const result = cli(f, ["--check"]);
  assert.equal(result.status, supported ? 0 : 2);
  if (result.status === 0)
    assert.match(result.stdout, /application launch were not tested/);
  else assert.match(result.stderr, /unsupported-platform/);
  await assert.rejects(readFile(path.join(f.root, "user-data/sessions.db")), {
    code: "ENOENT",
  });
});
