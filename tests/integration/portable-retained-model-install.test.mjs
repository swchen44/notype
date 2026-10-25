import test from "node:test";
import assert from "node:assert/strict";
import path from "node:path";
import {
  copyFile,
  link,
  mkdir,
  readFile,
  readdir,
  stat,
  unlink,
  writeFile,
} from "node:fs/promises";
import { spawnSync } from "node:child_process";
import { portableFixture, digest, json } from "./fixtures/portable-package.mjs";
import { inspectRelease } from "../../packages/notype/lib/launcher.mjs";

const npmCli =
  process.env.npm_execpath ||
  path.join(path.dirname(process.execPath), "node_modules/npm/bin/npm-cli.js");
const modelVersion = "0.1.0-qwen3-q6k.1";
const appVersion = "0.1.1-local.1";
const weight = "weights/fixture.bin";

async function npm(f, cache, args) {
  return spawnSync(
    process.execPath,
    [
      npmCli,
      ...args,
      "--offline",
      "--ignore-scripts",
      "--no-audit",
      "--no-fund",
      "--no-update-notifier",
      "--cache",
      cache,
      "--userconfig",
      f.npmConfig,
      "--globalconfig",
      f.npmGlobalConfig,
    ],
    {
      encoding: "utf8",
      timeout: 30000,
      maxBuffer: 1024 * 1024,
      windowsHide: true,
      cwd: f.root,
    },
  );
}

async function pack(f, directory) {
  const result = await npm(f, f.packCache, [
    "pack",
    directory,
    "--json",
    "--pack-destination",
    f.tarballs,
  ]);
  assert.equal(result.status, 0, result.stderr);
  const [receipt] = JSON.parse(result.stdout);
  assert.ok(
    receipt.size < 16 * 1024,
    "only tiny synthetic packages are allowed",
  );
  return path.join(f.tarballs, receipt.filename);
}

async function manifestPackage(directory, name, version) {
  const manifest = JSON.parse(
    await readFile(path.join(directory, "payload.json"), "utf8"),
  );
  manifest.name = name;
  manifest.version = version;
  manifest.status = "assembled";
  if (manifest.role === "application") {
    const bytes = `synthetic application ${version}`;
    await writeFile(path.join(directory, "app/notype.exe"), bytes);
    Object.assign(
      manifest.files.find((file) => file.role === "entrypoint"),
      { bytes: Buffer.byteLength(bytes), sha256: digest(bytes) },
    );
  }
  const text = json(manifest);
  await writeFile(path.join(directory, "payload.json"), text);
  await writeFile(
    path.join(directory, "package.json"),
    json({ name, version, private: true }),
  );
  return { name, version, manifest: "payload.json", sha256: digest(text) };
}

async function facadePackage(f, version, application, model = f.modelPin) {
  await writeFile(
    path.join(f.facade, "release.json"),
    json({ schema: 1, status: "assembled", version, application, model }),
  );
  await writeFile(
    path.join(f.facade, "package.json"),
    json({
      name: "notype",
      version,
      private: true,
      files: ["release.json"],
      dependencies: {
        "notype-win32-x64": application.version,
        "notype-model": model.version,
      },
    }),
  );
}

async function retentionFixture(t) {
  const f = await portableFixture(t);
  const npmMetadata = JSON.parse(
    await readFile(path.join(path.dirname(npmCli), "../package.json"), "utf8"),
  );
  t.diagnostic(
    `Node ${process.versions.node}; npm ${npmMetadata.version}; synthetic packages only`,
  );
  Object.assign(f, {
    npmConfig: path.join(f.root, "empty-user.npmrc"),
    npmGlobalConfig: path.join(f.root, "empty-global.npmrc"),
    packCache: path.join(f.root, "pack-cache"),
    tarballs: path.join(f.root, "tarballs"),
    oldPrefix: path.join(f.root, "old-prefix"),
    scratch: path.join(f.root, "new-prefix"),
    updateCache: path.join(f.root, "update-cache"),
  });
  await Promise.all([
    writeFile(f.npmConfig, ""),
    writeFile(f.npmGlobalConfig, ""),
    mkdir(f.tarballs),
    mkdir(f.updateCache),
  ]);
  f.modelPin = await manifestPackage(f.model, "notype-model", modelVersion);
  const oldApp = await manifestPackage(
    f.app,
    "notype-win32-x64",
    "0.1.0-local.1",
  );
  await facadePackage(f, "0.1.0-local.2", oldApp);
  const oldTarballs = [
    await pack(f, f.app),
    await pack(f, f.model),
    await pack(f, f.facade),
  ];
  const installed = await npm(f, path.join(f.root, "old-install-cache"), [
    "install",
    "--prefix",
    f.oldPrefix,
    ...oldTarballs,
  ]);
  assert.equal(installed.status, 0, installed.stderr);
  f.oldLock = JSON.parse(
    await readFile(path.join(f.oldPrefix, "package-lock.json"), "utf8"),
  );
  assert.equal(f.oldLock.lockfileVersion, 3);
  f.appPin = await manifestPackage(f.app, "notype-win32-x64", appVersion);
  await facadePackage(f, appVersion, f.appPin);
  f.newTarballs = [await pack(f, f.app), await pack(f, f.facade)];

  // Retain only the small model metadata while npm is allowed to write.
  // No shared weight inode exists in this incomplete, non-launchable prefix.
  await mkdir(f.scratch);
  for (const name of ["package.json", "package-lock.json"])
    await copyFile(path.join(f.oldPrefix, name), path.join(f.scratch, name));
  f.retainedModel = path.join(f.scratch, "node_modules/notype-model");
  await mkdir(f.retainedModel, { recursive: true });
  const modelManifest = JSON.parse(
    await readFile(path.join(f.model, "payload.json"), "utf8"),
  );
  f.retainedFiles = new Map();
  for (const name of [
    "package.json",
    "payload.json",
    ...modelManifest.files
      .filter((file) => file.path !== weight)
      .map((file) => file.path),
  ]) {
    const target = path.join(f.retainedModel, name);
    await mkdir(path.dirname(target), { recursive: true });
    await copyFile(
      path.join(f.oldPrefix, "node_modules/notype-model", name),
      target,
    );
    f.retainedFiles.set(name, {
      bytes: await readFile(target),
      mtimeMs: (await stat(target)).mtimeMs,
    });
  }
  await unlink(oldTarballs[1]);
  f.unavailableModelTarball = oldTarballs[1];
  return f;
}

test("offline npm installs the two new roles while retaining an unavailable model and its lock, then verifies the completed release", async (t) => {
  const f = await retentionFixture(t);
  const retainedModel = path.join(f.scratch, "node_modules/notype-model");
  assert.deepEqual(await readdir(f.updateCache), []);
  await assert.rejects(stat(f.unavailableModelTarball), { code: "ENOENT" });
  await assert.rejects(stat(path.join(retainedModel, weight)), {
    code: "ENOENT",
  });

  const installed = await npm(f, f.updateCache, [
    "install",
    "--prefix",
    f.scratch,
    ...f.newTarballs,
  ]);
  assert.equal(installed.status, 0, installed.stderr);
  const lock = JSON.parse(
    await readFile(path.join(f.scratch, "package-lock.json"), "utf8"),
  );
  assert.equal(lock.packages["node_modules/notype"].version, "0.1.1-local.1");
  assert.equal(
    lock.packages["node_modules/notype-win32-x64"].version,
    "0.1.1-local.1",
  );
  assert.deepEqual(
    lock.packages["node_modules/notype-model"],
    f.oldLock.packages["node_modules/notype-model"],
  );
  for (const [name, before] of f.retainedFiles) {
    assert.deepEqual(
      await readFile(path.join(retainedModel, name)),
      before.bytes,
    );
    assert.equal(
      (await stat(path.join(retainedModel, name))).mtimeMs,
      before.mtimeMs,
    );
  }
  await assert.rejects(stat(path.join(retainedModel, weight)), {
    code: "ENOENT",
  });
  const options = {
    facadeRoot: path.join(f.scratch, "node_modules/notype"),
    platform: "win32",
    arch: "x64",
    env: { NOTYPE_DATA_DIR: path.join(f.root, "unused-data") },
  };
  await assert.rejects(inspectRelease(options), { code: "payload-missing" });
  await mkdir(path.dirname(path.join(retainedModel, weight)), {
    recursive: true,
  });
  await link(path.join(f.model, weight), path.join(retainedModel, weight));
  const plan = await inspectRelease(options);
  assert.equal(
    plan.executable,
    path.join(f.scratch, "node_modules/notype-win32-x64/app/notype.exe"),
  );
  assert.equal(
    await readFile(path.join(retainedModel, weight), "utf8"),
    "fixture weights",
  );
  await assert.rejects(stat(options.env.NOTYPE_DATA_DIR), { code: "ENOENT" });
});

test("offline npm refuses a retained model whose version differs from the locked unavailable artifact", async (t) => {
  const f = await retentionFixture(t);
  const metadataFile = path.join(f.retainedModel, "package.json");
  const metadata = JSON.parse(await readFile(metadataFile, "utf8"));
  metadata.version = "0.1.0-qwen3-q6k.2";
  await writeFile(metadataFile, json(metadata));
  assert.deepEqual(await readdir(f.updateCache), []);
  await assert.rejects(stat(f.unavailableModelTarball), { code: "ENOENT" });
  const installed = await npm(f, f.updateCache, [
    "install",
    "--prefix",
    f.scratch,
    ...f.newTarballs,
  ]);
  assert.notEqual(
    installed.status,
    null,
    "npm must fail normally, not time out",
  );
  assert.notEqual(installed.status, 0);
  assert.match(installed.stderr, /npm error code ENOENT/);
  assert.ok(
    installed.stderr.includes(path.basename(f.unavailableModelTarball)),
  );
  await assert.rejects(stat(path.join(f.retainedModel, weight)), {
    code: "ENOENT",
  });
});
