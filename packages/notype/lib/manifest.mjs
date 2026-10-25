import { readFile, lstat, readdir } from "node:fs/promises";
import { createReadStream } from "node:fs";
import { createHash } from "node:crypto";
import path from "node:path";

export class PackageError extends Error {
  constructor(code, message) {
    super(message);
    this.code = code;
  }
}
const invalid = () => {
  throw new PackageError(
    "invalid-manifest",
    "Package manifest is invalid or unsupported.",
  );
};
const hashPattern = /^[a-f0-9]{64}$/;
const namePattern = /^(?:@[a-z0-9][a-z0-9._-]*\/)?[a-z0-9][a-z0-9._-]*$/;
export function relativeFile(value) {
  if (typeof value !== "string" || value.length > 240 || value.includes("\\"))
    invalid();
  for (const segment of value.split("/")) {
    if (
      !segment ||
      segment === "." ||
      segment === ".." ||
      /[<>:"|?*\x00-\x1f\x7f]/.test(segment) ||
      /[. ]$/.test(segment) ||
      /^(con|prn|aux|nul|com[1-9]|lpt[1-9])(?:\.|$)/i.test(segment)
    )
      invalid();
  }
  return value;
}
export async function jsonFile(file, maxBytes = 4 * 1024 * 1024) {
  let stat;
  try {
    stat = await lstat(file);
  } catch {
    throw new PackageError(
      "payload-missing",
      "Required package metadata is missing. Reinstall the pinned package.",
    );
  }
  if (!stat.isFile() || stat.isSymbolicLink() || stat.size > maxBytes)
    invalid();
  try {
    return JSON.parse(await readFile(file, "utf8"));
  } catch {
    invalid();
  }
}
export async function sha256(file) {
  const hash = createHash("sha256");
  for await (const chunk of createReadStream(file)) hash.update(chunk);
  return hash.digest("hex");
}
export function validatePin(pin) {
  if (
    !pin ||
    !namePattern.test(pin.name) ||
    !/^\d+\.\d+\.\d+(?:-[a-z0-9.-]+)?$/.test(pin.version) ||
    !hashPattern.test(pin.sha256)
  )
    invalid();
  relativeFile(pin.manifest);
  if (pin.manifest !== "payload.json") invalid();
}
async function regular(root, relative) {
  let current = root;
  for (const component of ["", ...relative.split("/")]) {
    current = path.join(current, component);
    let stat;
    try {
      stat = await lstat(current);
    } catch {
      throw new PackageError(
        "payload-missing",
        "Required package file is missing. Reinstall the pinned package.",
      );
    }
    if (
      stat.isSymbolicLink() ||
      (!stat.isDirectory() && current !== path.join(root, relative))
    )
      throw new PackageError(
        "unsafe-file",
        "Package contains a link or unsupported file. Reinstall from the trusted release.",
      );
    if (current === path.join(root, relative) && !stat.isFile())
      throw new PackageError(
        "unsafe-file",
        "Package contains a link or unsupported file.",
      );
  }
}
async function inventory(root, prefix = "") {
  const result = [];
  for (const entry of await readdir(path.join(root, prefix), {
    withFileTypes: true,
  })) {
    const relative = prefix ? `${prefix}/${entry.name}` : entry.name;
    relativeFile(relative);
    if (entry.isSymbolicLink())
      throw new PackageError("unsafe-file", "Package contains a link.");
    if (entry.isDirectory()) result.push(...(await inventory(root, relative)));
    else if (entry.isFile()) result.push(relative);
    else
      throw new PackageError(
        "unsafe-file",
        "Package contains an unsupported file.",
      );
  }
  return result;
}
export async function verifyPackage(root, pin, role) {
  validatePin(pin);
  await regular(root, pin.manifest);
  if ((await sha256(path.join(root, pin.manifest))) !== pin.sha256)
    throw new PackageError(
      "manifest-corrupt",
      "Pinned package manifest integrity check failed.",
    );
  const manifest = await jsonFile(path.join(root, pin.manifest));
  const metadata = await jsonFile(path.join(root, "package.json"));
  if (
    metadata.name !== pin.name ||
    metadata.version !== pin.version ||
    (metadata.scripts && Object.keys(metadata.scripts).length)
  )
    invalid();
  if (
    manifest.schema !== 1 ||
    manifest.name !== pin.name ||
    manifest.version !== pin.version ||
    manifest.role !== role ||
    !Array.isArray(manifest.files) ||
    manifest.files.length === 0 ||
    manifest.files.length > 10000 ||
    !manifest.sources ||
    typeof manifest.sources !== "object"
  )
    invalid();
  const files = new Map();
  const roles = new Set([
    "entrypoint",
    "webview-entry",
    "host-probe",
    "runtime",
    "native",
    "resource",
    "notice",
    "provenance",
    "model",
    "model-support",
  ]);
  for (const file of manifest.files) {
    relativeFile(file.path);
    const lower = file.path.toLowerCase();
    if (
      /(^|\/)(?:\.env(?:\..*)?|\.npmrc|settings_store\.json|text_processing_store\.json|recordings|private-config|user-data)(\/|$)|\.(?:db(?:-wal|-shm)?|sqlite3?|log)$/i.test(
        lower,
      )
    )
      throw new PackageError(
        "private-file",
        "User data or local configuration cannot be distributed in the application package.",
      );
    if (
      files.has(lower) ||
      ["package.json", pin.manifest].includes(lower) ||
      !roles.has(file.role) ||
      !Number.isSafeInteger(file.bytes) ||
      file.bytes <= 0 ||
      !hashPattern.test(file.sha256) ||
      !Object.hasOwn(manifest.sources, file.source)
    )
      invalid();
    files.set(lower, file);
  }
  for (const source of Object.values(manifest.sources)) {
    if (
      !source ||
      typeof source.revision !== "string" ||
      !source.revision.trim() ||
      typeof source.license !== "string" ||
      !source.license.trim() ||
      !Array.isArray(source.notices) ||
      !source.notices.length
    )
      invalid();
    let url;
    try {
      url = new URL(source.source);
    } catch {
      invalid();
    }
    if (
      url.protocol !== "https:" ||
      url.username ||
      url.password ||
      url.search ||
      url.hash
    )
      invalid();
    for (const notice of source.notices)
      if (files.get(relativeFile(notice).toLowerCase())?.role !== "notice")
        invalid();
  }
  function declared(file, role) {
    relativeFile(file);
    if (
      files.get(file.toLowerCase())?.path !== file ||
      files.get(file.toLowerCase())?.role !== role
    )
      invalid();
  }
  if (role === "application") {
    if (
      manifest.platform !== "win32" ||
      manifest.arch !== "x64" ||
      !["x86-64", "avx2-bmi2"].includes(manifest.cpu)
    )
      invalid();
    declared(manifest.entrypoint, "entrypoint");
    declared(manifest.hostProbe, "host-probe");
    if (
      !Array.isArray(manifest.hostProbeLibraries) ||
      !manifest.hostProbeLibraries.length
    )
      invalid();
    const probeDirectory = manifest.hostProbe.slice(
      0,
      manifest.hostProbe.lastIndexOf("/"),
    );
    if (
      !manifest.hostProbeLibraries.some(
        (file) =>
          file.toLowerCase() ===
          `${probeDirectory}/vcruntime140.dll`.toLowerCase(),
      )
    )
      invalid();
    for (const library of manifest.hostProbeLibraries) {
      declared(library, "native");
      if (
        !library.startsWith(`${probeDirectory}/`) ||
        library.slice(probeDirectory.length + 1).includes("/")
      )
        invalid();
    }
    if (
      !manifest.webview ||
      !/^\d+\.\d+\.\d+\.\d+$/.test(manifest.webview.version)
    )
      invalid();
    relativeFile(manifest.webview.directory);
    declared(manifest.webview.entrypoint, "webview-entry");
    if (
      manifest.webview.entrypoint !==
      `${manifest.webview.directory}/msedgewebview2.exe`
    )
      invalid();
  } else {
    if (
      typeof manifest.identity !== "string" ||
      !/^[a-zA-Z0-9._-]{1,120}$/.test(manifest.identity) ||
      !Array.isArray(manifest.modelFiles) ||
      !manifest.modelFiles.length
    )
      invalid();
    for (const file of manifest.modelFiles) declared(file, "model");
    const activation = manifest.activation;
    if (
      !activation ||
      activation.kind !== "file" ||
      manifest.modelFiles.length !== 1 ||
      activation.source !== manifest.modelFiles[0] ||
      typeof activation.modelId !== "string" ||
      !activation.modelId.trim()
    )
      invalid();
    relativeFile(activation.filename);
    if (
      activation.filename.includes("/") ||
      !/\.(gguf|bin)$/.test(activation.filename)
    )
      invalid();
  }
  for (const file of manifest.files) {
    await regular(root, file.path);
    const full = path.join(root, file.path);
    if (
      (await lstat(full)).size !== file.bytes ||
      (await sha256(full)) !== file.sha256
    )
      throw new PackageError(
        "payload-corrupt",
        "Package integrity check failed. Reinstall the pinned package.",
      );
  }
  const allowed = new Set([
    "package.json",
    pin.manifest,
    ...manifest.files.map((f) => f.path),
  ]);
  if ((await inventory(root)).some((file) => !allowed.has(file)))
    throw new PackageError(
      "unexpected-file",
      "Package contains files outside the release allowlist.",
    );
  return manifest;
}
