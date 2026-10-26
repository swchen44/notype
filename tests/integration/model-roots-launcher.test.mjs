import test from "node:test";
import assert from "node:assert/strict";
import path from "node:path";
import { readFile, writeFile, access } from "node:fs/promises";
import { portableFixture, json } from "./fixtures/portable-package.mjs";
import { runApplication } from "../../packages/notype/lib/launcher.mjs";

test("an application-only release reaches the chooser launch without model acquisition or copying", async (t) => {
  const f = await portableFixture(t);
  const release = { ...f.release, schema: 2 };
  delete release.model;
  await writeFile(path.join(f.facade, "release.json"), json(release));
  await writeFile(path.join(f.facade, "package.json"), json({ name: "notype", version: "0.0.0", type: "module", dependencies: { [f.appPin.name]: f.appPin.version } }));
  const before = await readFile(path.join(f.model, "weights/fixture.bin"));
  let launches = 0;
  await runApplication({ facadeRoot: f.facade, platform: "win32", arch: "x64", host: { integrity: "medium" }, env: { APPDATA: path.join(f.root, "roaming") }, launch: async (plan) => {
    launches += 1;
    assert.equal(plan.model, undefined);
    await assert.rejects(access(path.join(plan.dataDirectory, "models")), { code: "ENOENT" });
    return { exitCode: 0 };
  } });
  assert.equal(launches, 1);
  assert.deepEqual(await readFile(path.join(f.model, "weights/fixture.bin")), before);
});
