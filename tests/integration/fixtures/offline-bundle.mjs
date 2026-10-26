import path from "node:path";
import { cp, mkdir, readFile, readdir, rename, rmdir, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { portableFixture, digest, json } from "./portable-package.mjs";
import { startCommand } from "../../../packages/notype/lib/offline-bundle.mjs";

const source = fileURLToPath(new URL("../../../packages/notype/", import.meta.url));
export async function bundleFixture(t) {
  const f = await portableFixture(t);
  const root = path.join(f.root, "離線 bundle & space");
  await mkdir(root);
  await rename(f.facade, path.join(root, "app"));
  const facade = path.join(root, "app");
  const model = path.join(root, "models");
  await rename(path.join(facade, "node_modules/notype-model-fixture"), model);
  await rename(path.join(model, "weights/fixture.bin"), path.join(model, "fixture.bin"));
  await rmdir(path.join(model, "weights"));
  const modelManifest = JSON.parse(await readFile(path.join(model, "payload.json"), "utf8"));
  modelManifest.files.find(file => file.role === "model").path = "fixture.bin";
  modelManifest.modelFiles = ["fixture.bin"];
  modelManifest.activation.source = "fixture.bin";
  const modelBytes = json(modelManifest);
  await writeFile(path.join(model, "payload.json"), modelBytes);
  f.modelPin.sha256 = digest(modelBytes);
  const release = { ...f.release };
  delete release.model;
  await writeFile(path.join(facade, "release.json"), json(release));
  await cp(path.join(source, "lib"), path.join(facade, "lib"), { recursive: true });
  await cp(path.join(source, "bin"), path.join(facade, "bin"), { recursive: true });
  await mkdir(path.join(root, "runtime"));
  await writeFile(path.join(root, "runtime/node.exe"), "synthetic runtime executable");
  await writeFile(path.join(root, "runtime/LICENSE.txt"), "Synthetic fixture notice only.");
  await writeFile(path.join(root, "runtime/provenance.json"), json({ fixture: true }));
  await writeFile(path.join(root, "Start-notype.cmd"), startCommand());
  await writeFile(path.join(root, "README-zh-TW.md"), "Synthetic test bundle only.\n");
  const manifest = {
    schema: 1, mode: "offline-bundle", id: "fixture-v1", version: "0.0.0",
    source: "540d1ee44ac78dd95a381c7a7ed854fe42db79bc", facade: "app",
    application: f.appPin,
    model: { model_id: "fixture", root: "models", pin: f.modelPin },
    node: { path: "runtime/node.exe", version: "22.15.0", sha256: digest("synthetic runtime executable") },
    sources: { fixture: { source: "https://reserved.invalid/fixture", revision: "fixture-v1", license: "Fixture-only", notices: ["runtime/LICENSE.txt"] } },
    files: [],
  };
  const result = { ...f, root, ownerRoot: f.root, facade, app: path.join(facade, "node_modules/notype-win32-x64"), model, manifest };
  await sealBundle(result);
  return result;
}

export async function sealBundle(f) {
  const files = [];
  async function walk(prefix = "") {
    for (const entry of await readdir(path.join(f.root, prefix), { withFileTypes: true })) {
      const relative = prefix ? `${prefix}/${entry.name}` : entry.name;
      if (["bundle.json", "SHA256SUMS.txt"].includes(relative)) continue;
      if (entry.isDirectory()) await walk(relative);
      else {
        const bytes = await readFile(path.join(f.root, relative));
        let role = "resource";
        if (relative.endsWith("LICENSE.txt") || relative.endsWith("NOTICE.txt")) role = "notice";
        if (relative.endsWith(".bin") || relative.endsWith(".gguf")) role = "model";
        if (relative === "runtime/node.exe") role = "node";
        if (relative === "runtime/provenance.json") role = "provenance";
        if (relative === "Start-notype.cmd") role = "launcher";
        files.push({ path: relative, role, bytes: bytes.length, sha256: digest(bytes), source: "fixture" });
      }
    }
  }
  await walk();
  f.manifest.files = files.sort((a, b) => a.path.localeCompare(b.path, "en"));
  const metadata = json(f.manifest);
  await writeFile(path.join(f.root, "bundle.json"), metadata);
  const sums = [...files.map(file => [file.path, file.sha256]), ["bundle.json", digest(metadata)]].sort(([a], [b]) => a < b ? -1 : a > b ? 1 : 0).map(([file, hash]) => `${hash}  ${file}\n`).join("");
  await writeFile(path.join(f.root, "SHA256SUMS.txt"), sums);
}
