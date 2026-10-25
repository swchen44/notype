import test from "node:test";
import assert from "node:assert/strict";
import path from "node:path";
import { unlink, writeFile, readFile, mkdir } from "node:fs/promises";
import { prepareLaunch } from "../../packages/notype/lib/launcher.mjs";
import {
  portableFixture,
  json,
  replaceManifest,
  digest,
} from "../integration/fixtures/portable-package.mjs";

test("verified packages resolve fixed runtime and stable external data independently of package version", async (t) => {
  const f = await portableFixture(t);
  const plan = await prepareLaunch({
    facadeRoot: f.facade,
    platform: "win32",
    arch: "x64",
    env: { APPDATA: path.join(f.root, "roaming") },
    host: { integrity: "medium" },
  });
  assert.equal(plan.executable, path.join(f.app, "app/notype.exe"));
  assert.equal(
    plan.dataDirectory,
    path.join(f.root, "roaming/io.github.swchen44.notype"),
  );
  assert.equal(
    plan.environment.WEBVIEW2_BROWSER_EXECUTABLE_FOLDER,
    path.join(f.app, "webview/156.0.4314.42"),
  );
  assert.equal(
    plan.environment.WEBVIEW2_USER_DATA_FOLDER,
    path.join(plan.dataDirectory, "webview2"),
  );
  assert.equal(plan.environment.NOTYPE_DATA_DIR, plan.dataDirectory);
  assert.equal(plan.model.identity, "fixture-v1");
});

test("package inventory rejects undeclared private data and traversal in pinned manifests", async (t) => {
  const f = await portableFixture(t);
  await writeFile(path.join(f.app, "sessions.db"), "private fixture");
  const options = {
    facadeRoot: f.facade,
    platform: "win32",
    arch: "x64",
    env: { APPDATA: f.root },
    host: { integrity: "medium" },
  };
  await assert.rejects(prepareLaunch(options), { code: "unexpected-file" });
  await unlink(path.join(f.app, "sessions.db"));
  await replaceManifest(f, "application", (m) => {
    m.entrypoint = "../outside.exe";
  });
  await assert.rejects(prepareLaunch(options), { code: "invalid-manifest" });
});

test("replaced dependency manifest cannot redefine the trusted file hashes", async (t) => {
  const f = await portableFixture(t);
  await writeFile(
    path.join(f.app, "payload.json"),
    json({
      schema: 1,
      files: [],
      entrypoint: "app/notype.exe",
      webview: { directory: "webview" },
    }),
  );
  await assert.rejects(
    prepareLaunch({
      facadeRoot: f.facade,
      platform: "win32",
      arch: "x64",
      env: { APPDATA: f.root },
      host: { integrity: "medium" },
    }),
    { code: "manifest-corrupt" },
  );
});

test("missing fixed runtime refuses a launch plan instead of falling back to Evergreen", async (t) => {
  const f = await portableFixture(t);
  await unlink(path.join(f.app, "webview/156.0.4314.42/msedgewebview2.exe"));
  await assert.rejects(
    prepareLaunch({
      facadeRoot: f.facade,
      platform: "win32",
      arch: "x64",
      env: { APPDATA: f.root },
      host: { integrity: "medium" },
    }),
    { code: "payload-missing" },
  );
});

test("same-size corruption of any pinned dependency refuses a launch plan", async (t) => {
  const f = await portableFixture(t);
  await writeFile(path.join(f.model, "weights/fixture.bin"), "FIXTURE weights");
  await assert.rejects(
    prepareLaunch({
      facadeRoot: f.facade,
      platform: "win32",
      arch: "x64",
      env: { APPDATA: f.root },
      host: { integrity: "medium" },
    }),
    { code: "payload-corrupt" },
  );
});

test("unsupported platform and elevated hosts refuse before package lookup or mutable data work", async (t) => {
  const f = await portableFixture(t);
  await assert.rejects(
    prepareLaunch({
      facadeRoot: f.facade,
      platform: "linux",
      arch: "x64",
      env: {},
    }),
    { code: "unsupported-platform" },
  );
  await assert.rejects(
    prepareLaunch({
      facadeRoot: f.facade,
      platform: "win32",
      arch: "arm64",
      env: {},
    }),
    { code: "unsupported-platform" },
  );
  await assert.rejects(
    prepareLaunch({
      facadeRoot: f.facade,
      platform: "win32",
      arch: "x64",
      env: {},
      host: { integrity: "high" },
    }),
    { code: "normal-user-required" },
  );
});

test("absolute data override keeps existing history, while package-local or relative data paths are refused", async (t) => {
  const f = await portableFixture(t);
  const data = path.join(f.root, "previous-version-data");
  await mkdir(data);
  await writeFile(path.join(data, "sessions.db"), "existing history fixture");
  const options = {
    facadeRoot: f.facade,
    platform: "win32",
    arch: "x64",
    host: { integrity: "medium" },
  };
  const plan = await prepareLaunch({
    ...options,
    env: { NOTYPE_DATA_DIR: data },
  });
  assert.equal(plan.dataDirectory, data);
  assert.equal(
    await readFile(path.join(data, "sessions.db"), "utf8"),
    "existing history fixture",
  );
  f.release.version = "1.0.0";
  await writeFile(path.join(f.facade, "release.json"), json(f.release));
  await writeFile(
    path.join(f.facade, "package.json"),
    json({ name: "notype", version: "1.0.0", type: "module" }),
  );
  const updated = await prepareLaunch({
    ...options,
    env: { NOTYPE_DATA_DIR: data },
  });
  assert.equal(updated.dataDirectory, plan.dataDirectory);
  assert.equal(
    await readFile(path.join(data, "sessions.db"), "utf8"),
    "existing history fixture",
  );
  await assert.rejects(
    prepareLaunch({ ...options, env: { NOTYPE_DATA_DIR: "relative" } }),
    { code: "invalid-data-directory" },
  );
  await assert.rejects(
    prepareLaunch({
      ...options,
      env: { NOTYPE_DATA_DIR: path.join(f.app, "mutable") },
    }),
    { code: "invalid-data-directory" },
  );
});

test("drive-relative Windows data paths cannot depend on the caller's current drive", async (t) => {
  const f = await portableFixture(t);
  const configured =
    process.platform === "win32" ? "\\notype-drive-implicit" : "relative";
  await assert.rejects(
    prepareLaunch({
      facadeRoot: f.facade,
      platform: "win32",
      arch: "x64",
      env: { NOTYPE_DATA_DIR: configured },
      host: { integrity: "medium" },
    }),
    { code: "invalid-data-directory" },
  );
});

test("private settings cannot be added even to an otherwise hash-pinned resource allowlist", async (t) => {
  const f = await portableFixture(t);
  const bytes = "private fixture, no real credential";
  await writeFile(path.join(f.app, "text_processing_store.json"), bytes);
  await replaceManifest(f, "application", (m) =>
    m.files.push({
      path: "text_processing_store.json",
      role: "resource",
      bytes: Buffer.byteLength(bytes),
      sha256: digest(bytes),
      source: "fixture",
    }),
  );
  await assert.rejects(
    prepareLaunch({
      facadeRoot: f.facade,
      platform: "win32",
      arch: "x64",
      env: { APPDATA: f.root },
      host: { integrity: "medium" },
    }),
    { code: "private-file" },
  );
});
