import test from "node:test";
import assert from "node:assert/strict";
import path from "node:path";
import { startCommand, inspectBundle } from "../../packages/notype/lib/offline-bundle.mjs";
import { bundleFixture } from "../integration/fixtures/offline-bundle.mjs";
import { prepareLaunch, inspectRelease } from "../../packages/notype/lib/launcher.mjs";
import { unlink, writeFile, readFile } from "node:fs/promises";

test("offline Start invokes only the quoted adjacent runtime and fixed facade", () => {
  assert.equal(startCommand(), '@echo off\r\nsetlocal DisableDelayedExpansion\r\n"%~dp0runtime\\node.exe" "%~dp0app\\bin\\notype.mjs" --offline-bundle\r\nexit /b %errorlevel%\r\n');
});

test("normal bundle startup removes inherited defaults and hands native one verified root", async (t) => {
  const f = await bundleFixture(t);
  const options = { facadeRoot: f.facade, offlineBundle: true, execPath: path.join(f.root, "runtime/node.exe"), platform: "win32", arch: "x64", host: { integrity: "medium" }, env: { APPDATA: f.ownerRoot, NOTYPE_BOOTSTRAP_FILE: "outside-secret", notype_offline_root: "outside", NOTYPE_MODEL_ROOT: "outside", NOTYPE_BUNDLE_ROOT: "outside", KEEP_FIXTURE: "retained" } };
  const plan = await prepareLaunch(options);
  assert.deepEqual(plan.arguments, ["--offline-bundle-root", f.root]);
  assert.equal(plan.model.root, f.model);
  assert.equal(plan.environment.KEEP_FIXTURE, "retained");
  assert.equal(Object.keys(plan.environment).some(key => /^NOTYPE_(?:BOOTSTRAP|OFFLINE|MODEL|BUNDLE)/i.test(key)), false);
  await unlink(path.join(f.model, "fixture.bin"));
  assert.equal((await prepareLaunch(options)).model, undefined);
  await assert.rejects(inspectRelease(options), { code: "bundle-file-missing" });
  const notice = path.join(f.root, "runtime/LICENSE.txt");
  const original = await readFile(notice);
  await writeFile(notice, "damaged notice");
  await assert.rejects(prepareLaunch(options), { code: "bundle-file-corrupt" });
  await writeFile(notice, original);
});

test("complete offline inspection binds the local runtime and every declared file", async (t) => {
  const f = await bundleFixture(t);
  const result = await inspectBundle({ facadeRoot: f.facade, execPath: path.join(f.root, "runtime/node.exe") });
  assert.equal(result.root, f.root);
  assert.equal(result.model.root, f.model);
  assert.equal(result.model.identity, "fixture-v1");
  assert.equal(result.manifest.node.version, "22.15.0");
});
