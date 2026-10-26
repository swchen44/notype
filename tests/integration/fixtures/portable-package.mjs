import { mkdtemp, mkdir, writeFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { createHash } from "node:crypto";

export const digest = (bytes) =>
  createHash("sha256").update(bytes).digest("hex");
export const json = (value) => JSON.stringify(value, null, 2) + "\n";

export async function replaceManifest(f, role, update) {
  const { readFile } = await import("node:fs/promises");
  const directory = role === "application" ? f.app : f.model;
  const manifest = JSON.parse(
    await readFile(path.join(directory, "payload.json"), "utf8"),
  );
  update(manifest);
  const bytes = json(manifest);
  await writeFile(path.join(directory, "payload.json"), bytes);
  f.release[role].sha256 = digest(bytes);
  await writeFile(path.join(f.facade, "release.json"), json(f.release));
}

// Synthetic bytes exercise distribution contracts, never actual executable/model code.
export async function portableFixture(t) {
  const root = await mkdtemp(path.join(tmpdir(), "notype-package-contract-"));
  t.after(() => rm(root, { recursive: true, force: true }));
  const facade = path.join(root, "facade");
  const app = path.join(facade, "node_modules/notype-win32-x64");
  const model = path.join(facade, "node_modules/notype-model-fixture");
  await mkdir(app, { recursive: true });
  await mkdir(model, { recursive: true });
  await writeFile(
    path.join(facade, "package.json"),
    json({ name: "notype", version: "0.0.0", type: "module" }),
  );
  const provenance = {
    source: "https://example.org/fixture",
    revision: "fixture-v1",
    license: "Fixture-only",
    notices: ["NOTICE.txt"],
  };
  async function makePackage(directory, name, role, contents, extra) {
    await writeFile(
      path.join(directory, "package.json"),
      json({ name, version: "0.0.0", private: true }),
    );
    const files = [];
    for (const [file, fileRole, bytes] of contents) {
      await mkdir(path.dirname(path.join(directory, file)), {
        recursive: true,
      });
      await writeFile(path.join(directory, file), bytes);
      files.push({
        path: file,
        role: fileRole,
        bytes: Buffer.byteLength(bytes),
        sha256: digest(bytes),
        source: "fixture",
      });
    }
    const manifest = {
      schema: 1,
      name,
      version: "0.0.0",
      role,
      sources: { fixture: provenance },
      files,
      ...extra,
    };
    const bytes = json(manifest);
    await writeFile(path.join(directory, "payload.json"), bytes);
    return {
      name,
      version: "0.0.0",
      manifest: "payload.json",
      sha256: digest(bytes),
    };
  }
  const appPin = await makePackage(
    app,
    "notype-win32-x64",
    "application",
    [
      ["app/notype.exe", "entrypoint", "fixture app"],
      ["app/notype-platform-check.exe", "host-probe", "fixture probe"],
      ["app/vcruntime140.dll", "native", "fixture VC runtime"],
      [
        "webview/156.0.4314.42/msedgewebview2.exe",
        "webview-entry",
        "fixture runtime",
      ],
      ["webview/156.0.4314.42/support.dat", "runtime", "fixture support"],
      ["NOTICE.txt", "notice", "Synthetic test fixtures only."],
    ],
    {
      platform: "win32",
      arch: "x64",
      cpu: "x86-64",
      entrypoint: "app/notype.exe",
      hostProbe: "app/notype-platform-check.exe",
      hostProbeLibraries: ["app/vcruntime140.dll"],
      webview: {
        directory: "webview/156.0.4314.42",
        version: "156.0.4314.42",
        entrypoint: "webview/156.0.4314.42/msedgewebview2.exe",
      },
    },
  );
  const modelPin = await makePackage(
    model,
    "notype-model-fixture",
    "model",
    [
      ["weights/fixture.bin", "model", "fixture weights"],
      ["NOTICE.txt", "notice", "Synthetic test fixtures only."],
    ],
    {
      identity: "fixture-v1",
      modelFiles: ["weights/fixture.bin"],
      activation: {
        kind: "file",
        source: "weights/fixture.bin",
        filename: "fixture.bin",
        modelId: "fixture",
      },
    },
  );
  const release = {
    schema: 2,
    status: "assembled",
    version: "0.0.0",
    application: appPin,
    model: modelPin,
  };
  await writeFile(path.join(facade, "release.json"), json(release));
  await writeFile(path.join(facade, "package.json"), json({ name: "notype", version: "0.0.0", type: "module", dependencies: { [appPin.name]: appPin.version } }));
  return { root, facade, app, model, release, appPin, modelPin };
}
