import test from "node:test";
import assert from "node:assert/strict";
import path from "node:path";
import {
  chmod,
  mkdir,
  readFile,
  readdir,
  rename,
  writeFile,
} from "node:fs/promises";
import { spawnSync } from "node:child_process";
import { bundleFixture, sealBundle } from "./fixtures/offline-bundle.mjs";
import { digest, json } from "./fixtures/portable-package.mjs";
import {
  inspectBundle,
  inspectZipArchive,
} from "../../packages/notype/lib/offline-bundle.mjs";
import { runApplication } from "../../packages/notype/lib/launcher.mjs";

function python(script, args) {
  const result = spawnSync("python", ["-c", script, ...args], {
    encoding: "utf8",
    windowsHide: true,
    timeout: 10000,
  });
  assert.equal(result.status, 0, result.stderr);
}
const pack =
  "import pathlib,sys,zipfile\nr=pathlib.Path(sys.argv[1])\nwith zipfile.ZipFile(sys.argv[2],'w',zipfile.ZIP_DEFLATED) as z:\n for p in sorted(r.rglob('*')):\n  if p.is_file(): z.write(p,p.relative_to(r).as_posix())\n";
const extract =
  "import sys,zipfile\nwith zipfile.ZipFile(sys.argv[1]) as z: z.extractall(sys.argv[2])\n";

test("a real tiny ZIP is audited before extraction and verified from a Unicode destination", async (t) => {
  const f = await bundleFixture(t);
  const archive = path.join(f.ownerRoot, "fixture.zip");
  python(pack, [f.root, archive]);
  const metadata = await Promise.all(
    ["bundle.json", "SHA256SUMS.txt"].map(async (name) => {
      const content = await readFile(path.join(f.root, name));
      return { path: name, bytes: content.length, sha256: digest(content) };
    }),
  );
  const audit = await inspectZipArchive(archive, {
    expectedFiles: [...f.manifest.files, ...metadata],
  });
  assert.match(audit.sha256, /^[a-f0-9]{64}$/);
  assert.equal(audit.files.length, f.manifest.files.length + 2);
  const destination = path.join(f.ownerRoot, "解壓 & new space");
  await mkdir(destination);
  python(extract, [archive, destination]);
  const result = await inspectBundle({
    facadeRoot: path.join(destination, "app"),
    execPath: path.join(destination, "runtime/node.exe"),
  });
  assert.equal(result.model.root, path.join(destination, "models"));
  assert.equal((await readdir(f.ownerRoot)).includes("outside.bin"), false);
});

async function sentinel(f, executable) {
  if (process.platform === "win32") {
    const source = path.join(f.ownerRoot, "sentinel.cs");
    await writeFile(
      source,
      'using System; using System.IO; using System.Web.Script.Serialization; class Sentinel { static int Main(string[] args) { var result = new { args = args, cwd = Environment.CurrentDirectory, bundle = Environment.GetEnvironmentVariable("NOTYPE_BUNDLE_ROOT"), bootstrap = Environment.GetEnvironmentVariable("NOTYPE_BOOTSTRAP_FILE"), model = Environment.GetEnvironmentVariable("NOTYPE_MODEL_ROOT"), marker = Environment.GetEnvironmentVariable("NOTYPE_TEST_MARKER") }; File.WriteAllText(Environment.GetEnvironmentVariable("NOTYPE_TEST_RESULT"), new JavaScriptSerializer().Serialize(result)); return 0; } }',
    );
    const compiler = path.join(
      process.env.WINDIR,
      "Microsoft.NET/Framework64/v4.0.30319/csc.exe",
    );
    const result = spawnSync(
      compiler,
      [
        "/nologo",
        "/target:exe",
        "/reference:System.Web.Extensions.dll",
        `/out:${executable}`,
        source,
      ],
      { encoding: "utf8", timeout: 10000, windowsHide: true },
    );
    assert.equal(result.status, 0, result.stdout + result.stderr);
  } else {
    await writeFile(
      executable,
      '#!/usr/bin/env python3\nimport os,sys,json\nwith open(os.environ["NOTYPE_TEST_RESULT"], "w") as f: json.dump({"args":sys.argv[1:],"cwd":os.getcwd(),"bundle":os.environ.get("NOTYPE_BUNDLE_ROOT"),"bootstrap":os.environ.get("NOTYPE_BOOTSTRAP_FILE"),"model":os.environ.get("NOTYPE_MODEL_ROOT"),"marker":os.environ.get("NOTYPE_TEST_MARKER")},f)\n',
    );
    await chmod(executable, 0o755);
  }
}

test("the public facade reaches only its declared owned child with sanitized context and no source writes", async (t) => {
  const f = await bundleFixture(t);
  const app = path.join(f.app, "app/notype.exe");
  await sentinel(f, app);
  const payload = JSON.parse(
    await readFile(path.join(f.app, "payload.json"), "utf8"),
  );
  const entry = payload.files.find((file) => file.path === "app/notype.exe");
  const content = await readFile(app);
  entry.bytes = content.length;
  entry.sha256 = digest(content);
  const metadata = json(payload);
  await writeFile(path.join(f.app, "payload.json"), metadata);
  f.manifest.application.sha256 = digest(metadata);
  const release = JSON.parse(
    await readFile(path.join(f.facade, "release.json"), "utf8"),
  );
  release.application.sha256 = digest(metadata);
  await writeFile(path.join(f.facade, "release.json"), json(release));
  await sealBundle(f);
  const before = await Promise.all(
    f.manifest.files.map(async (file) => [
      file.path,
      digest(await readFile(path.join(f.root, file.path))),
    ]),
  );
  const resultFile = path.join(f.ownerRoot, "child-result.json");
  await runApplication({
    facadeRoot: f.facade,
    offlineBundle: true,
    execPath: path.join(f.root, "runtime/node.exe"),
    platform: "win32",
    arch: "x64",
    host: { integrity: "medium" },
    env: {
      ...process.env,
      NOTYPE_DATA_DIR: path.join(f.ownerRoot, "owned-data"),
      NOTYPE_TEST_RESULT: resultFile,
      NOTYPE_TEST_MARKER: "owned-child",
      NOTYPE_BUNDLE_ROOT: "hostile",
      NOTYPE_BOOTSTRAP_FILE: "hostile",
      NOTYPE_MODEL_ROOT: "hostile",
    },
  });
  const result = JSON.parse(await readFile(resultFile, "utf8"));
  assert.deepEqual(result.args, ["--offline-bundle-root", f.root]);
  assert.equal(result.cwd, path.dirname(app));
  assert.equal(result.marker, "owned-child");
  assert.equal(result.bundle, null);
  assert.equal(result.bootstrap, null);
  assert.equal(result.model, null);
  assert.deepEqual(
    await Promise.all(
      f.manifest.files.map(async (file) => [
        file.path,
        digest(await readFile(path.join(f.root, file.path))),
      ]),
    ),
    before,
  );
});

test(
  "Windows Start selects its adjacent runtime double with empty PATH and quoted Unicode paths",
  { skip: process.platform !== "win32" ? "Windows CMD contract only" : false },
  async (t) => {
    const f = await bundleFixture(t);
    const runtime = path.join(f.root, "runtime/node.exe");
    await sentinel(f, runtime);
    const resultFile = path.join(f.ownerRoot, "runtime-result.json");
    const cmd = path.join(process.env.WINDIR, "System32/cmd.exe");
    const result = spawnSync(
      cmd,
      [
        "/d",
        "/s",
        "/c",
        `""${path.join(f.root, "Start-notype.cmd")}" "ignored & arguments""`,
      ],
      {
        cwd: f.ownerRoot,
        encoding: "utf8",
        timeout: 10000,
        windowsHide: true,
        windowsVerbatimArguments: true,
        env: {
          ...process.env,
          PATH: "",
          NOTYPE_TEST_RESULT: resultFile,
          NOTYPE_TEST_MARKER: "runtime-double",
        },
      },
    );
    assert.equal(result.status, 0, result.stdout + result.stderr);
    const observed = JSON.parse(await readFile(resultFile, "utf8"));
    assert.deepEqual(observed.args, [
      path.join(f.root, "app/bin/notype.mjs"),
      "--offline-bundle",
    ]);
    assert.equal(observed.marker, "runtime-double");
  },
);

test("untrusted ZIP paths, aliases, links and expanded sizes fail before extraction", async (t) => {
  const f = await bundleFixture(t);
  const archive = path.join(f.ownerRoot, "negative.zip");
  const create =
    "import sys,json,zipfile\nentries=json.loads(sys.argv[2])\nwith zipfile.ZipFile(sys.argv[1],'w',zipfile.ZIP_DEFLATED) as z:\n for e in entries:\n  i=zipfile.ZipInfo(e['path']);i.create_system=3;i.external_attr=e.get('mode',0o100600)<<16;i.compress_type=zipfile.ZIP_DEFLATED\n  z.writestr(i,e.get('data','fixture'))\n";
  const expectedFiles = [
    { path: "safe.txt", bytes: 7, sha256: digest("fixture") },
  ];
  const cases = [
    [{ path: "../outside.bin" }],
    [{ path: "/outside.bin" }],
    [{ path: "C:/outside.bin" }],
    [{ path: "safe.txt:stream" }],
    [{ path: "CON.txt" }],
    [{ path: "folder\\safe.txt" }],
    [{ path: "safe.txt" }, { path: "safe.txt" }],
    [{ path: "safe.txt" }, { path: "SAFE.TXT" }],
    [{ path: "safe.txt", mode: 0o120777 }],
    [{ path: "safe.txt", mode: 0o060600 }],
    [{ path: "safe.txt", data: "X".repeat(10000) }],
    [{ path: "safe.txt", data: "changed" }],
    [{ path: "safe.txt" }, { path: "extra.txt" }],
  ];
  for (const entries of cases) {
    python(create, [archive, JSON.stringify(entries)]);
    await assert.rejects(
      inspectZipArchive(archive, { expectedFiles }),
      (error) =>
        ["unsafe-archive", "invalid-manifest"].includes(error.code) &&
        !/outside|stream|changed/.test(error.message),
    );
  }
  assert.equal((await readdir(f.ownerRoot)).includes("outside.bin"), false);
  const folderCase = [{ path: "Folder/one.txt" }, { path: "folder/two.txt" }];
  python(create, [archive, JSON.stringify(folderCase)]);
  await assert.rejects(
    inspectZipArchive(archive, {
      expectedFiles: folderCase.map((e) => ({
        path: e.path,
        bytes: 7,
        sha256: digest("fixture"),
      })),
    }),
    { code: "unsafe-archive" },
  );
});

test("optional STT bootstrap is verified without importing settings or touching a moved model folder", async (t) => {
  const f = await bundleFixture(t);
  const bootstrap = json({
    schema: 1,
    bootstrap_id: "fixture-v1",
    stt: { model_id: "fixture", root: "models" },
  });
  await mkdir(path.join(f.root, "bootstrap"));
  const file = path.join(f.root, "bootstrap/notype-bootstrap.json");
  await writeFile(file, bootstrap);
  f.manifest.bootstrap = {
    path: "bootstrap/notype-bootstrap.json",
    sha256: digest(bootstrap),
  };
  await sealBundle(f);
  const options = {
    facadeRoot: f.facade,
    execPath: path.join(f.root, "runtime/node.exe"),
  };
  const readonlyModel = path.join(f.model, "fixture.bin");
  await chmod(readonlyModel, 0o444);
  try {
    assert.equal(
      (await inspectBundle(options)).bootstrap.bootstrap_id,
      "fixture-v1",
    );
  } finally {
    await chmod(readonlyModel, 0o644);
  }
  await writeFile(
    file,
    json({
      schema: 1,
      bootstrap_id: "fixture-v1",
      api_key: "synthetic-secret-sentinel",
    }),
  );
  await assert.rejects(inspectBundle({ ...options, strictModels: false }), {
    code: "bundle-file-corrupt",
  });
  await writeFile(file, bootstrap);
  const moved = path.join(f.ownerRoot, "moved-model-folder");
  await rename(f.model, moved);
  const weights = await readFile(path.join(moved, "fixture.bin"));
  assert.equal(
    (await inspectBundle({ ...options, strictModels: false })).model,
    undefined,
  );
  await assert.rejects(inspectBundle(options), { code: "bundle-file-missing" });
  assert.deepEqual(await readFile(path.join(moved, "fixture.bin")), weights);
  assert.equal((await readdir(f.ownerRoot)).includes("owned-data"), false);
});

test("actual CLI derives bundle context from its module and refuses an external Node safely", async (t) => {
  const f = await bundleFixture(t);
  const result = spawnSync(
    process.execPath,
    [path.join(f.facade, "bin/notype.mjs"), "--offline-bundle", "--check"],
    {
      cwd: f.ownerRoot,
      encoding: "utf8",
      timeout: 15000,
      windowsHide: true,
      env: {
        ...process.env,
        NOTYPE_BUNDLE_ROOT: "private-path-sentinel",
        NOTYPE_BOOTSTRAP_FILE: "private-secret-sentinel",
      },
    },
  );
  assert.equal(result.status, 2);
  assert.match(
    result.stderr,
    process.platform === "win32" && process.arch === "x64"
      ? /bundled-node-required/
      : /unsupported-platform/,
  );
  assert.doesNotMatch(
    result.stdout + result.stderr,
    /sentinel|離線|notype-package-contract/,
  );
});
