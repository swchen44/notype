import test from "node:test";
import assert from "node:assert/strict";
import path from "node:path";
import { mkdir, readFile, writeFile, readdir } from "node:fs/promises";
import {
  prepareLaunch,
  runApplication,
} from "../../packages/notype/lib/launcher.mjs";
import { activateModel } from "../../packages/notype/lib/model.mjs";
import { portableFixture } from "./fixtures/portable-package.mjs";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";

test("local model activation preserves existing history/settings and reuses verified weights across package versions", async (t) => {
  const f = await portableFixture(t);
  const data = path.join(f.root, "persistent-data");
  await mkdir(data);
  await writeFile(
    path.join(data, "settings_store.json"),
    "existing selected-model fixture",
  );
  await writeFile(path.join(data, "sessions.db"), "existing history fixture");
  const plan = await prepareLaunch({
    facadeRoot: f.facade,
    platform: "win32",
    arch: "x64",
    host: { integrity: "medium" },
    env: { NOTYPE_DATA_DIR: data },
  });
  const result = await activateModel(plan);
  assert.equal(result.status, "activated");
  assert.equal(
    await readFile(path.join(data, "models/fixture.bin"), "utf8"),
    "fixture weights",
  );
  assert.equal((await activateModel(plan)).status, "existing");
  assert.equal(
    await readFile(path.join(data, "settings_store.json"), "utf8"),
    "existing selected-model fixture",
  );
  assert.equal(
    await readFile(path.join(data, "sessions.db"), "utf8"),
    "existing history fixture",
  );
  assert.deepEqual(await readdir(path.join(data, "models")), ["fixture.bin"]);
});

test("application launch preserves existing model files and never performs activation", async (t) => {
  const f = await portableFixture(t);
  const data = path.join(f.root, "process-data");
  await mkdir(path.join(data, "models"), { recursive: true });
  await writeFile(path.join(data, "models/fixture.bin"), "unrelated existing model");
  let launches = 0;
  const options = {
    facadeRoot: f.facade, platform: "win32", arch: "x64", host: { integrity: "medium" }, env: { NOTYPE_DATA_DIR: data },
    launch: async (plan) => {
      launches += 1;
      assert.equal(await readFile(path.join(data, "models/fixture.bin"), "utf8"), "unrelated existing model");
      assert.equal(plan.environment.WEBVIEW2_BROWSER_EXECUTABLE_FOLDER, path.join(f.app, "webview/156.0.4314.42"));
      return "external-launch-boundary";
    },
  };
  assert.equal(await runApplication(options), "external-launch-boundary");
  await writeFile(path.join(f.model, "weights/fixture.bin"), "corrupt optional bytes");
  assert.equal(await runApplication(options), "external-launch-boundary");
  assert.equal(launches, 2);
});

test("restart recovers an actual killed copy without publishing partial weights or deleting unrelated files", async (t) => {
  const f = await portableFixture(t);
  const data = path.join(f.root, "restart-data");
  const plan = await prepareLaunch({
    facadeRoot: f.facade,
    platform: "win32",
    arch: "x64",
    host: { integrity: "medium" },
    env: { NOTYPE_DATA_DIR: data },
  });
  const input = path.join(f.root, "worker-plan.json");
  await writeFile(input, JSON.stringify(plan));
  const child = spawn(
    process.execPath,
    [
      fileURLToPath(
        new URL("./fixtures/activate-model-worker.mjs", import.meta.url),
      ),
      input,
    ],
    { windowsHide: true, stdio: "ignore" },
  );
  await new Promise((resolve, reject) => {
    child.once("error", reject);
    child.once("exit", resolve);
  });
  const interrupted = await readdir(path.join(data, "models"));
  assert.equal(interrupted.length, 1);
  assert.match(interrupted[0], /^\.notype-.*\.partial$/);
  await writeFile(
    path.join(data, "models/manual-model.bin"),
    "unrelated model",
  );
  assert.equal((await activateModel(plan)).status, "activated");
  assert.deepEqual((await readdir(path.join(data, "models"))).sort(), [
    "fixture.bin",
    "manual-model.bin",
  ]);
});

test("simultaneous launches publish one complete model and never replace a conflicting user model", async (t) => {
  const f = await portableFixture(t);
  const data = path.join(f.root, "concurrent-data");
  const plan = await prepareLaunch({
    facadeRoot: f.facade,
    platform: "win32",
    arch: "x64",
    host: { integrity: "medium" },
    env: { NOTYPE_DATA_DIR: data },
  });
  const outcomes = await Promise.all([
    activateModel(plan),
    activateModel(plan),
  ]);
  assert.deepEqual(outcomes.map((outcome) => outcome.status).sort(), [
    "activated",
    "existing",
  ]);
  await writeFile(
    path.join(data, "models/fixture.bin"),
    "existing other model",
  );
  await assert.rejects(activateModel(plan), { code: "model-conflict" });
  assert.equal(
    await readFile(path.join(data, "models/fixture.bin"), "utf8"),
    "existing other model",
  );
  assert.deepEqual(await readdir(path.join(data, "models")), ["fixture.bin"]);
});

test("source changed after package checks is never activated", async (t) => {
  const f = await portableFixture(t);
  const data = path.join(f.root, "changed-source-data");
  const plan = await prepareLaunch({
    facadeRoot: f.facade,
    platform: "win32",
    arch: "x64",
    host: { integrity: "medium" },
    env: { NOTYPE_DATA_DIR: data },
  });
  await writeFile(path.join(f.model, "weights/fixture.bin"), "changed fixture");
  await assert.rejects(activateModel(plan), { code: "model-source-changed" });
  assert.deepEqual(await readdir(path.join(data, "models")), []);
});

test("cancelled model copy leaves no active partial and a retry can recover without changing settings", async (t) => {
  const f = await portableFixture(t);
  const data = path.join(f.root, "cancelled-data");
  const plan = await prepareLaunch({
    facadeRoot: f.facade,
    platform: "win32",
    arch: "x64",
    host: { integrity: "medium" },
    env: { NOTYPE_DATA_DIR: data },
  });
  const controller = new AbortController();
  await assert.rejects(
    activateModel(plan, {
      signal: controller.signal,
      onProgress: () => controller.abort(),
    }),
    { code: "model-activation-cancelled" },
  );
  assert.deepEqual(await readdir(path.join(data, "models")), []);
  assert.equal((await activateModel(plan)).status, "activated");
});

test("insufficient copy headroom refuses activation before creating a partial model", async (t) => {
  const f = await portableFixture(t);
  const data = path.join(f.root, "space-data");
  const plan = await prepareLaunch({
    facadeRoot: f.facade,
    platform: "win32",
    arch: "x64",
    host: { integrity: "medium" },
    env: { NOTYPE_DATA_DIR: data },
  });
  await assert.rejects(
    activateModel(plan, { minimumFreeBytes: Number.MAX_SAFE_INTEGER }),
    { code: "insufficient-model-space" },
  );
  assert.deepEqual(await readdir(path.join(data, "models")), []);
});
