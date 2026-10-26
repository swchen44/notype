import test from "node:test";
import assert from "node:assert/strict";
import path from "node:path";
import { mkdir, readFile, readdir } from "node:fs/promises";
import { spawnSync } from "node:child_process";
import { bundleFixture } from "./fixtures/offline-bundle.mjs";
import { digest } from "./fixtures/portable-package.mjs";
import { inspectBundle, inspectZipArchive } from "../../packages/notype/lib/offline-bundle.mjs";

function python(script, args) {
  const result = spawnSync("python", ["-c", script, ...args], { encoding: "utf8", windowsHide: true, timeout: 10000 });
  assert.equal(result.status, 0, result.stderr);
}
const pack = "import pathlib,sys,zipfile\nr=pathlib.Path(sys.argv[1])\nwith zipfile.ZipFile(sys.argv[2],'w',zipfile.ZIP_DEFLATED) as z:\n for p in sorted(r.rglob('*')):\n  if p.is_file(): z.write(p,p.relative_to(r).as_posix())\n";
const extract = "import sys,zipfile\nwith zipfile.ZipFile(sys.argv[1]) as z: z.extractall(sys.argv[2])\n";

test("a real tiny ZIP is audited before extraction and verified from a Unicode destination", async (t) => {
  const f = await bundleFixture(t);
  const archive = path.join(f.ownerRoot, "fixture.zip");
  python(pack, [f.root, archive]);
  const metadata = await Promise.all(["bundle.json", "SHA256SUMS.txt"].map(async name => { const content = await readFile(path.join(f.root, name)); return { path: name, bytes: content.length, sha256: digest(content) }; }));
  const audit = await inspectZipArchive(archive, { expectedFiles: [...f.manifest.files, ...metadata] });
  assert.match(audit.sha256, /^[a-f0-9]{64}$/);
  assert.equal(audit.files.length, f.manifest.files.length + 2);
  const destination = path.join(f.ownerRoot, "解壓 & new space");
  await mkdir(destination);
  python(extract, [archive, destination]);
  const result = await inspectBundle({ facadeRoot: path.join(destination, "app"), execPath: path.join(destination, "runtime/node.exe") });
  assert.equal(result.model.root, path.join(destination, "models"));
  assert.equal((await readdir(f.ownerRoot)).includes("outside.bin"), false);
});
