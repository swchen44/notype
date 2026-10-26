import test from "node:test";
import assert from "node:assert/strict";
import path from "node:path";
import {
  startCommand,
  inspectBundle,
  validateBootstrap,
  assemblyInventory,
} from "../../packages/notype/lib/offline-bundle.mjs";
import {
  bundleFixture,
  sealBundle,
} from "../integration/fixtures/offline-bundle.mjs";
import {
  prepareLaunch,
  inspectRelease,
} from "../../packages/notype/lib/launcher.mjs";
import {
  unlink,
  writeFile,
  readFile,
  link,
  symlink,
  mkdir,
  rename,
} from "node:fs/promises";

test("offline Start invokes only the quoted adjacent runtime and fixed facade", () => {
  assert.equal(
    startCommand(),
    '@echo off\r\nsetlocal DisableDelayedExpansion\r\n"%~dp0runtime\\node.exe" "%~dp0app\\bin\\notype.mjs" --offline-bundle\r\nexit /b %errorlevel%\r\n',
  );
});

test("bootstrap transport rejects unknown capability fields and numeric identities", () => {
  const model = { model_id: "fixture" };
  const stt = { model_id: "fixture", root: "models" };
  assert.throws(
    () => validateBootstrap({ schema: 1, bootstrap_id: 123, stt }, model),
    { code: "bootstrap-invalid" },
  );
  const connection = {
    display_name: "Synthetic",
    kind: "custom",
    dialect: "generic",
    address_mode: "base_url",
    api_address: "https://reserved.invalid/v1",
    models_endpoint: null,
    model: "fixture-text",
    authentication: "bearer",
    timeout_ms: 1000,
    thinking_preference: "off",
    effort: null,
    capability_declaration: {
      kind: "non_reasoning",
      minimum_effort: null,
      supported_efforts: [],
      temperature_supported: true,
      top_p_supported: true,
      extra: true,
    },
    temperature: null,
    top_p: null,
    max_output_tokens: null,
  };
  assert.throws(
    () =>
      validateBootstrap(
        { schema: 1, bootstrap_id: "fixture-v1", stt, connection },
        model,
      ),
    { code: "bootstrap-invalid" },
  );
});

test("normal bundle startup removes inherited defaults and hands native one verified root", async (t) => {
  const f = await bundleFixture(t);
  const options = {
    facadeRoot: f.facade,
    offlineBundle: true,
    execPath: path.join(f.root, "runtime/node.exe"),
    platform: "win32",
    arch: "x64",
    host: { integrity: "medium" },
    env: {
      APPDATA: f.ownerRoot,
      NOTYPE_BOOTSTRAP_FILE: "outside-secret",
      notype_offline_root: "outside",
      NOTYPE_MODEL_ROOT: "outside",
      NOTYPE_BUNDLE_ROOT: "outside",
      KEEP_FIXTURE: "retained",
    },
  };
  const plan = await prepareLaunch(options);
  assert.deepEqual(plan.arguments, ["--offline-bundle-root", f.root]);
  assert.equal(plan.model.root, f.model);
  assert.equal(plan.environment.KEEP_FIXTURE, "retained");
  assert.equal(
    Object.keys(plan.environment).some((key) =>
      /^NOTYPE_(?:BOOTSTRAP|OFFLINE|MODEL|BUNDLE)/i.test(key),
    ),
    false,
  );
  await unlink(path.join(f.model, "fixture.bin"));
  assert.equal((await prepareLaunch(options)).model, undefined);
  await assert.rejects(inspectRelease(options), {
    code: "bundle-file-missing",
  });
  const notice = path.join(f.root, "runtime/LICENSE.txt");
  const original = await readFile(notice);
  await writeFile(notice, "damaged notice");
  await assert.rejects(prepareLaunch(options), { code: "bundle-file-corrupt" });
  await writeFile(notice, original);
});

test("complete offline inspection binds the local runtime and every declared file", async (t) => {
  const f = await bundleFixture(t);
  const result = await inspectBundle({
    facadeRoot: f.facade,
    execPath: path.join(f.root, "runtime/node.exe"),
  });
  assert.equal(result.root, f.root);
  assert.equal(result.model.root, f.model);
  assert.equal(result.model.identity, "fixture-v1");
  assert.equal(result.manifest.node.version, "22.15.0");
});

test("bundle inventory rejects extras, unsafe paths and runtime substitution", async (t) => {
  const f = await bundleFixture(t);
  const options = {
    facadeRoot: f.facade,
    execPath: path.join(f.root, "runtime/node.exe"),
  };
  await assert.rejects(
    inspectBundle({ ...options, execPath: process.execPath }),
    { code: "bundled-node-required" },
  );
  await writeFile(path.join(f.root, "extra.txt"), "undeclared");
  await assert.rejects(inspectBundle(options), { code: "unexpected-file" });
  await unlink(path.join(f.root, "extra.txt"));
  for (const unsafe of [
    "../outside",
    "C:/outside",
    "models/x:stream",
    "models/CON.txt",
    "models/trailing.",
    "models\\outside",
  ]) {
    const original = f.manifest.files[0].path;
    f.manifest.files[0].path = unsafe;
    await writeFile(
      path.join(f.root, "bundle.json"),
      JSON.stringify(f.manifest),
    );
    await assert.rejects(inspectBundle(options), { code: "invalid-manifest" });
    f.manifest.files[0].path = original;
  }
  await sealBundle(f);
  f.manifest.files.push({
    ...f.manifest.files[0],
    path: f.manifest.files[0].path.toUpperCase(),
  });
  await writeFile(path.join(f.root, "bundle.json"), JSON.stringify(f.manifest));
  await assert.rejects(inspectBundle(options), { code: "invalid-bundle" });
});

test("every required program role stays a hard gate while optional model notices recover", async (t) => {
  const f = await bundleFixture(t);
  const options = {
    facadeRoot: f.facade,
    execPath: path.join(f.root, "runtime/node.exe"),
    strictModels: false,
  };
  for (const name of [
    "runtime/node.exe",
    "runtime/LICENSE.txt",
    "runtime/provenance.json",
    "app/bin/notype.mjs",
    "app/node_modules/notype-win32-x64/app/notype.exe",
    "app/node_modules/notype-win32-x64/webview/156.0.4314.42/support.dat",
    "app/node_modules/notype-win32-x64/NOTICE.txt",
  ]) {
    const full = path.join(f.root, name);
    const original = await readFile(full);
    await writeFile(full, "damaged fixture");
    await assert.rejects(inspectBundle(options), {
      code: "bundle-file-corrupt",
    });
    await writeFile(full, original);
  }
  await writeFile(
    path.join(f.model, "NOTICE.txt"),
    "damaged optional model notice",
  );
  assert.equal((await inspectBundle(options)).model, undefined);
  await assert.rejects(inspectBundle({ ...options, strictModels: true }), {
    code: "bundle-file-corrupt",
  });
});

test("extracted links are refused while the separate assembly counter can read immutable hardlinks", async (t) => {
  const f = await bundleFixture(t);
  const original = path.join(f.root, "runtime/LICENSE.txt");
  const outside = path.join(f.ownerRoot, "immutable-notice.txt");
  await rename(original, outside);
  await link(outside, original);
  const stage = await assemblyInventory(f.root);
  assert.ok(stage.bytes > 0 && stage.count > 0);
  await assert.rejects(
    inspectBundle({
      facadeRoot: f.facade,
      execPath: path.join(f.root, "runtime/node.exe"),
    }),
    { code: "unsafe-file" },
  );
  await unlink(original);
  await rename(outside, original);
  const alias = path.join(f.ownerRoot, "junction");
  await symlink(f.root, alias, "junction");
  await assert.rejects(
    inspectBundle({
      facadeRoot: path.join(alias, "app"),
      execPath: path.join(alias, "runtime/node.exe"),
    }),
    { code: "unsafe-file" },
  );
});

test("bootstrap rejects secret fields, credential URLs, expansion and raw diagnostic sentinels", () => {
  const valid = {
    schema: 1,
    bootstrap_id: "fixture-v1",
    stt: { model_id: "fixture", root: "models" },
  };
  assert.deepEqual(validateBootstrap(valid, { model_id: "fixture" }), valid);
  for (const bad of [
    { ...valid, api_key: "secret-sentinel" },
    { ...valid, bootstrap_id: "%SECRET%" },
    { ...valid, stt: { ...valid.stt, root: "../private" } },
  ]) {
    assert.throws(
      () => validateBootstrap(bad, { model_id: "fixture" }),
      (error) =>
        error.code === "bootstrap-invalid" &&
        !/sentinel|private|SECRET/.test(error.message),
    );
  }
});

test("a self-consistent bundle cannot trim the pinned full Fixed WebView inventory", async (t) => {
  const f = await bundleFixture(t, { completeRuntime: false });
  await assert.rejects(
    inspectBundle({
      facadeRoot: f.facade,
      execPath: path.join(f.root, "runtime/node.exe"),
    }),
    { code: "runtime-incomplete" },
  );
});
