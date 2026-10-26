import test from "node:test";
import assert from "node:assert/strict";
import path from "node:path";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { spawnSync } from "node:child_process";
import { gunzipSync } from "node:zlib";
import { fileURLToPath } from "node:url";

function tarFiles(gzip) {
  const tar = gunzipSync(gzip);
  const entries = new Map();
  for (let offset = 0; offset + 512 <= tar.length; ) {
    const header = tar.subarray(offset, offset + 512);
    if (header.every((byte) => byte === 0)) break;
    const name = header.subarray(0, 100).toString().replace(/\0.*$/, "");
    const bytes = parseInt(
      header.subarray(124, 136).toString().replace(/\0.*$/, "").trim(),
      8,
    );
    assert.ok(Number.isSafeInteger(bytes) && bytes >= 0);
    assert.equal(header[156], 48, "npm tarball may contain regular files only");
    assert.ok(
      name.startsWith("package/") &&
        !name.includes("..") &&
        !name.includes("\\"),
    );
    entries.set(
      name.slice(8),
      tar.subarray(offset + 512, offset + 512 + bytes),
    );
    offset += 512 + Math.ceil(bytes / 512) * 512;
  }
  return entries;
}

test("real npm tarballs contain only the private package role allowlists and no lifecycle scripts", async (t) => {
  const output = await mkdtemp(path.join(tmpdir(), "notype-tarball-contract-"));
  t.after(() => rm(output, { recursive: true, force: true }));
  const npmCli =
    process.env.npm_execpath ||
    path.join(
      path.dirname(process.execPath),
      "node_modules/npm/bin/npm-cli.js",
    );
  const packages = fileURLToPath(new URL("../../packages/", import.meta.url));
  for (const role of ["notype", "notype-win32-x64", "notype-model"]) {
    const result = spawnSync(
      process.execPath,
      [
        npmCli,
        "pack",
        path.join(packages, role),
        "--ignore-scripts",
        "--offline",
        "--json",
        "--pack-destination",
        output,
      ],
      { encoding: "utf8", timeout: 30000, windowsHide: true },
    );
    assert.equal(result.status, 0, result.stderr);
    const [receipt] = JSON.parse(result.stdout);
    const entries = tarFiles(
      await readFile(path.join(output, receipt.filename)),
    );
    const metadata = JSON.parse(entries.get("package.json"));
    assert.equal(metadata.private, true);
    assert.equal(metadata.scripts, undefined);
    const expected =
      role === "notype"
        ? [
            "package.json",
            "README.md",
            "OFFLINE-README-zh-TW.md",
            "bootstrap.example.json",
            "release.json",
            "bin/notype.mjs",
            "lib/launcher.mjs",
            "lib/manifest.mjs",
            "lib/model.mjs",
            "lib/offline-bundle.mjs",
            "lib/windows-host.mjs",
          ]
        : ["package.json", "README.md", "payload.json"];
    assert.deepEqual([...entries.keys()].sort(), expected.sort());
    assert.equal(
      JSON.parse(
        entries.get(role === "notype" ? "release.json" : "payload.json"),
      ).status,
      "unassembled",
    );
    for (const name of entries.keys())
      assert.doesNotMatch(
        name,
        /\.env|\.db$|recordings|private-config|node_modules|desktop\//,
      );
  }
});
