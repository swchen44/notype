import path from "node:path";
import { lstat, open, readFile, readdir, realpath } from "node:fs/promises";
import { createReadStream } from "node:fs";
import { createHash } from "node:crypto";
import { createInflateRaw } from "node:zlib";
import { PackageError, relativeFile, jsonFile, sha256, validatePin, verifyPackage } from "./manifest.mjs";

const hashPattern = /^[a-f0-9]{64}$/;
const fail = (code = "invalid-bundle") => {
  throw new PackageError(code, "The offline bundle could not be verified. Restore the complete trusted bundle.");
};
const exact = (value, keys) => value && typeof value === "object" && !Array.isArray(value) && Object.keys(value).every(key => keys.includes(key));
const privatePath = value => /(^|\/)(?:\.env(?:\..*)?|\.npmrc|settings_store\.json|text_processing_store\.json|provider_connections_v2\.json|recordings|private-config|user-data)(\/|$)|\.(?:db(?:-wal|-shm)?|sqlite3?|log)$/i.test(value);

// Fixed arguments only: deriving the bundle root inside the facade also avoids
// passing a quoted trailing backslash through Windows command-line parsing.
export function startCommand() {
  return '@echo off\r\nsetlocal DisableDelayedExpansion\r\n"%~dp0runtime\\node.exe" "%~dp0app\\bin\\notype.mjs" --offline-bundle\r\nexit /b %errorlevel%\r\n';
}

async function regular(full, directory = false, staging = false) {
  const absolute = path.resolve(full);
  const parsed = path.parse(absolute);
  let current = parsed.root;
  for (const segment of absolute.slice(parsed.root.length).split(path.sep)) {
    current = path.join(current, segment);
    let stat;
    try { stat = await lstat(current); } catch { fail("bundle-file-missing"); }
    if (stat.isSymbolicLink() || (current !== absolute || directory ? !stat.isDirectory() : !stat.isFile() || (!staging && stat.nlink !== 1))) fail("unsafe-file");
  }
  return lstat(absolute);
}

/** Enumerate regular files without following links; assembly can count bytes
 * before writing an archive. Every ancestor and each file identity is checked. */
async function inventory(root, staging) {
  await regular(root, true);
  const entries = [];
  const names = new Set();
  const identities = new Set();
  async function walk(prefix = "") {
    for (const entry of await readdir(path.join(root, prefix), { withFileTypes: true })) {
      const relative = relativeFile(prefix ? `${prefix}/${entry.name}` : entry.name);
      const lower = relative.toLowerCase();
      if (names.has(lower)) fail("invalid-bundle");
      names.add(lower);
      if (privatePath(relative)) fail("private-file");
      if (entry.isDirectory()) { await regular(path.join(root, relative), true); await walk(relative); }
      else {
        const stat = await regular(path.join(root, relative), false, staging);
        const identity = `${stat.dev}:${stat.ino}`;
        if (!staging && identities.has(identity)) fail("unsafe-file");
        identities.add(identity);
        entries.push({ path: relative, bytes: stat.size });
      }
      if (names.size > 50000) fail();
    }
  }
  await walk();
  return entries;
}

export async function bundleInventory(root) { return inventory(root, false); }

/** Assembly may read audited immutable hardlinks; ZIP emits regular bytes.
 * This counting seam never weakens the extracted-bundle/launch verifier. */
export async function assemblyInventory(root) {
  const files = await inventory(root, true);
  const bytes = files.reduce((total, file) => total + file.bytes, 0);
  if (!Number.isSafeInteger(bytes)) fail();
  return { files, bytes, count: files.length };
}

function validateManifest(m) {
  if (!exact(m, ["schema", "mode", "id", "version", "source", "facade", "application", "model", "node", "bootstrap", "files", "sources"]) || m.schema !== 1 || m.mode !== "offline-bundle" || !/^[a-zA-Z0-9._-]{1,120}$/.test(m.id) || !/^\d+\.\d+\.\d+(?:-[a-z0-9.-]+)?$/.test(m.version) || !/^[a-f0-9]{40}$/.test(m.source) || m.facade !== "app") fail();
  validatePin(m.application);
  if (m.application.name !== "notype-win32-x64") fail();
  if (!exact(m.model, ["model_id", "root", "pin"]) || m.model.root !== "models" || typeof m.model.model_id !== "string" || !m.model.model_id.trim() || m.model.model_id.length > 240) fail();
  validatePin(m.model.pin);
  if (!exact(m.node, ["path", "version", "sha256"]) || m.node.path !== "runtime/node.exe" || m.node.version !== "22.15.0" || !hashPattern.test(m.node.sha256)) fail();
  if (m.bootstrap !== undefined && (!exact(m.bootstrap, ["path", "sha256"]) || m.bootstrap.path !== "bootstrap/notype-bootstrap.json" || !hashPattern.test(m.bootstrap.sha256))) fail();
  if (!Array.isArray(m.files) || !m.files.length || m.files.length > 50000 || !m.sources || typeof m.sources !== "object" || Array.isArray(m.sources)) fail();
  const files = new Map();
  const roles = new Set(["launcher", "resource", "metadata", "node", "native", "runtime", "model", "model-support", "notice", "provenance", "bootstrap", "entrypoint", "host-probe", "webview-entry"]);
  for (const file of m.files) {
    if (!exact(file, ["path", "bytes", "sha256", "role", "source"])) fail();
    const name = relativeFile(file.path);
    if (privatePath(name)) fail("private-file");
    if (files.has(name.toLowerCase()) || ["bundle.json", "sha256sums.txt"].includes(name.toLowerCase()) || !Number.isSafeInteger(file.bytes) || file.bytes <= 0 || !hashPattern.test(file.sha256) || !roles.has(file.role) || !Object.hasOwn(m.sources, file.source)) fail();
    files.set(name.toLowerCase(), file);
  }
  for (const source of Object.values(m.sources)) {
    if (!exact(source, ["source", "revision", "license", "notices"]) || typeof source.revision !== "string" || !source.revision.trim() || typeof source.license !== "string" || !source.license.trim() || !Array.isArray(source.notices) || !source.notices.length) fail();
    let url;
    try { url = new URL(source.source); } catch { fail(); }
    if (url.protocol !== "https:" || url.username || url.password || url.search || url.hash) fail();
    for (const notice of source.notices) if (files.get(relativeFile(notice).toLowerCase())?.role !== "notice") fail();
  }
  function declared(name, role) {
    const file = files.get(name.toLowerCase());
    if (file?.path !== name || (role && file.role !== role)) fail();
    return file;
  }
  declared("Start-notype.cmd", "launcher");
  for (const name of ["README-zh-TW.md", "app/bin/notype.mjs", "app/lib/offline-bundle.mjs", "app/package.json", "app/release.json", "runtime/provenance.json"]) declared(name);
  if (declared(m.node.path, "node").sha256 !== m.node.sha256) fail();
  if (!m.files.some(f => f.path.startsWith("runtime/") && f.role === "notice") || declared("runtime/provenance.json").role !== "provenance") fail();
  if (m.bootstrap && declared(m.bootstrap.path, "bootstrap").sha256 !== m.bootstrap.sha256) fail();
  if (!m.bootstrap && m.files.some(f => f.path.startsWith("bootstrap/"))) fail();
  return files;
}

/** Strict transport validation only. Native ConnectionStore/capability and root
 * services remain authoritative; this routine never resolves keys or requests. */
export function validateBootstrap(value, model) {
  const connectionKeys = ["display_name", "kind", "dialect", "address_mode", "api_address", "models_endpoint", "model", "authentication", "timeout_ms", "thinking_preference", "effort", "capability_declaration", "temperature", "top_p", "max_output_tokens"];
  if (!exact(value, ["schema", "bootstrap_id", "stt", "connection"]) || value.schema !== 1 || !/^[a-zA-Z0-9._-]{1,120}$/.test(value.bootstrap_id) || !exact(value.stt, ["model_id", "root"]) || value.stt.model_id !== model.model_id || value.stt.root !== "models") fail("bootstrap-invalid");
  function scan(input) {
    if (typeof input === "string") {
      if (input.length > 4096 || /[\x00-\x1f\x7f]|\$\{|%[^%]+%/.test(input)) fail("bootstrap-invalid");
      if (/^https?:/i.test(input)) { let url; try { url = new URL(input); } catch { fail("bootstrap-invalid"); } if (url.username || url.password || url.search || url.hash) fail("bootstrap-invalid"); }
    } else if (input && typeof input === "object") for (const [key, item] of Object.entries(input)) {
      if (/(?:api_?key|token|authorization|password|secret|credential)/i.test(key) && key !== "max_output_tokens") fail("bootstrap-invalid");
      scan(item);
    }
  }
  scan(value);
  if (value.connection !== undefined) {
    const c = value.connection;
    if (!exact(c, connectionKeys) || Object.keys(c).length !== connectionKeys.length || ["display_name", "kind", "dialect", "address_mode", "api_address", "model", "authentication", "thinking_preference"].some(key => typeof c[key] !== "string" || !c[key].trim()) || !Number.isSafeInteger(c.timeout_ms) || c.timeout_ms <= 0 || ["temperature", "top_p"].some(key => c[key] !== null && (typeof c[key] !== "number" || !Number.isFinite(c[key]))) || (c.max_output_tokens !== null && (!Number.isSafeInteger(c.max_output_tokens) || c.max_output_tokens <= 0))) fail("bootstrap-invalid");
  }
  return value;
}

function checksums(m, manifestHash) {
  return [...m.files.map(f => [f.path, f.sha256]), ["bundle.json", manifestHash]].sort(([a], [b]) => a < b ? -1 : a > b ? 1 : 0).map(([name, hash]) => `${hash}  ${name}\n`).join("");
}

/** Read-only integrity inspection. Normal launch may recover from unavailable
 * weights; all application/runtime/bootstrap failures remain hard failures. */
export async function inspectBundle({ facadeRoot, execPath = process.execPath, strictModels = true }) {
  if (!path.isAbsolute(facadeRoot) || path.basename(facadeRoot) !== "app") fail();
  const root = path.dirname(facadeRoot);
  await regular(root, true);
  await regular(path.join(root, "bundle.json"));
  const manifest = await jsonFile(path.join(root, "bundle.json"));
  const declared = validateManifest(manifest);
  const actual = await bundleInventory(root);
  const actualNames = new Set(actual.map(f => f.path));
  if (actual.some(f => !declared.has(f.path.toLowerCase()) && !["bundle.json", "SHA256SUMS.txt"].includes(f.path))) fail("unexpected-file");
  for (const file of manifest.files) {
    const modelFile = file.path.startsWith("models/");
    const optional = !strictModels && modelFile;
    if (optional) continue;
    if (!actualNames.has(file.path)) fail("bundle-file-missing");
    if ((await lstat(path.join(root, file.path))).size !== file.bytes || await sha256(path.join(root, file.path)) !== file.sha256) fail("bundle-file-corrupt");
  }
  await regular(path.join(root, "SHA256SUMS.txt"));
  const sums = await readFile(path.join(root, "SHA256SUMS.txt"));
  if (sums.length > 8 * 1024 * 1024 || sums.toString("utf8") !== checksums(manifest, await sha256(path.join(root, "bundle.json")))) fail("bundle-checksums-invalid");
  if (await readFile(path.join(root, "Start-notype.cmd"), "utf8") !== startCommand()) fail("bundle-launcher-invalid");
  const runtime = path.join(root, manifest.node.path);
  await regular(execPath);
  if ((await realpath(execPath)).toLowerCase() !== (await realpath(runtime)).toLowerCase()) fail("bundled-node-required");
  const release = await jsonFile(path.join(facadeRoot, "release.json"));
  if (release.schema !== 2 || release.status !== "assembled" || release.version !== manifest.version || ["name", "version", "manifest", "sha256"].some(key => release.application?.[key] !== manifest.application[key]) || release.model !== undefined) fail();
  const application = await verifyPackage(path.join(facadeRoot, "node_modules/notype-win32-x64"), manifest.application, "application");
  let bootstrap;
  if (manifest.bootstrap) bootstrap = validateBootstrap(await jsonFile(path.join(root, manifest.bootstrap.path), 65536), manifest.model);
  let model;
  try {
    const modelRoot = path.join(root, "models");
    const metadata = await verifyPackage(modelRoot, manifest.model.pin, "model");
    if (metadata.activation.modelId !== manifest.model.model_id || metadata.activation.source.includes("/")) fail();
    model = { ...metadata, root: modelRoot };
  } catch (error) { if (strictModels) throw error; }
  return { root, manifest, bootstrap, model, application };
}

const crcTable = Array.from({ length: 256 }, (_, value) => {
  for (let bit = 0; bit < 8; bit++) value = value & 1 ? 0xedb88320 ^ (value >>> 1) : value >>> 1;
  return value >>> 0;
});

/** Audit ZIP32 before extraction. Expected inventory and expanded sizes are
 * mandatory, so archive-controlled sizes cannot authorize a decompression bomb.
 * Entries and archive bytes are streamed; no second archive or full temp exists. */
export async function inspectZipArchive(archive, { expectedFiles }) {
  if (!Array.isArray(expectedFiles) || !expectedFiles.length || expectedFiles.length > 50000) fail("unsafe-archive");
  const expected = new Map();
  let expandedBytes = 0;
  for (const file of expectedFiles) {
    const name = relativeFile(file.path);
    if (privatePath(name) || expected.has(name.toLowerCase()) || !Number.isSafeInteger(file.bytes) || file.bytes <= 0 || !hashPattern.test(file.sha256)) fail("unsafe-archive");
    expected.set(name.toLowerCase(), file);
    expandedBytes += file.bytes;
  }
  if (!Number.isSafeInteger(expandedBytes) || expandedBytes >= 0xffffffff) fail("unsafe-archive");
  const stat = await regular(archive);
  if (stat.size < 22 || stat.size >= 0xffffffff) fail("unsafe-archive");
  const handle = await open(archive, "r");
  try {
    async function bytes(offset, length) {
      if (offset < 0 || length < 0 || offset + length > stat.size) fail("unsafe-archive");
      const buffer = Buffer.alloc(length);
      if ((await handle.read(buffer, 0, length, offset)).bytesRead !== length) fail("unsafe-archive");
      return buffer;
    }
    const tail = await bytes(Math.max(0, stat.size - 65557), Math.min(stat.size, 65557));
    let end = -1;
    for (let i = tail.length - 22; i >= 0; i--) if (tail.readUInt32LE(i) === 0x06054b50 && i + 22 + tail.readUInt16LE(i + 20) === tail.length) { end = i; break; }
    if (end < 0) fail("unsafe-archive");
    const count = tail.readUInt16LE(end + 10);
    const centralBytes = tail.readUInt32LE(end + 12);
    const centralOffset = tail.readUInt32LE(end + 16);
    const endOffset = stat.size - tail.length + end;
    if (tail.readUInt16LE(end + 4) !== 0 || tail.readUInt16LE(end + 6) !== 0 || tail.readUInt16LE(end + 8) !== count || !count || count > 50000 || centralBytes > 16 * 1024 * 1024 || centralOffset + centralBytes !== endOffset || tail.readUInt16LE(end + 20) !== 0) fail("unsafe-archive");
    const central = await bytes(centralOffset, centralBytes);
    const names = new Set();
    const files = [];
    const ranges = [];
    let position = 0;
    for (let index = 0; index < count; index++) {
      if (position + 46 > central.length || central.readUInt32LE(position) !== 0x02014b50) fail("unsafe-archive");
      const flags = central.readUInt16LE(position + 8);
      const method = central.readUInt16LE(position + 10);
      const crc = central.readUInt32LE(position + 16);
      const compressedBytes = central.readUInt32LE(position + 20);
      const length = central.readUInt32LE(position + 24);
      const nameLength = central.readUInt16LE(position + 28);
      const extraLength = central.readUInt16LE(position + 30);
      const commentLength = central.readUInt16LE(position + 32);
      const attributes = central.readUInt32LE(position + 38);
      const offset = central.readUInt32LE(position + 42);
      const next = position + 46 + nameLength + extraLength + commentLength;
      if (next > central.length || !nameLength || extraLength !== 0 || commentLength !== 0 || central.readUInt16LE(position + 34) !== 0 || flags & ~0x800 || ![0, 8].includes(method)) fail("unsafe-archive");
      const rawName = central.subarray(position + 46, position + 46 + nameLength);
      let name;
      try { name = new TextDecoder("utf-8", { fatal: true }).decode(rawName); } catch { fail("unsafe-archive"); }
      if (!(flags & 0x800) && rawName.some(byte => byte >= 128)) fail("unsafe-archive");
      const directory = name.endsWith("/");
      relativeFile(directory ? name.slice(0, -1) : name);
      if (privatePath(name) || names.has(name.replace(/\/$/, "").toLowerCase())) fail("unsafe-archive");
      names.add(name.replace(/\/$/, "").toLowerCase());
      const type = (attributes >>> 16) & 0xf000;
      if ((attributes & 0x400) || (type !== 0 && type !== (directory ? 0x4000 : 0x8000)) || (directory && length !== 0)) fail("unsafe-archive");
      const declared = expected.get(name.toLowerCase());
      if (!directory && (!declared || declared.path !== name || declared.bytes !== length)) fail("unsafe-archive");
      const local = await bytes(offset, 30);
      if (local.readUInt32LE(0) !== 0x04034b50 || local.readUInt16LE(6) !== flags || local.readUInt16LE(8) !== method || local.readUInt32LE(14) !== crc || local.readUInt32LE(18) !== compressedBytes || local.readUInt32LE(22) !== length || local.readUInt16LE(26) !== nameLength || local.readUInt16LE(28) !== 0) fail("unsafe-archive");
      if (!(await bytes(offset + 30, nameLength)).equals(rawName)) fail("unsafe-archive");
      const dataOffset = offset + 30 + nameLength;
      if (dataOffset + compressedBytes > centralOffset) fail("unsafe-archive");
      ranges.push([offset, dataOffset + compressedBytes]);
      if (!directory) files.push({ path: name, bytes: length, compressedBytes, dataOffset, method, crc, sha256: declared.sha256 });
      position = next;
    }
    if (position !== central.length || files.length !== expected.size) fail("unsafe-archive");
    let previousEnd = 0;
    for (const [start, finish] of ranges.sort((a, b) => a[0] - b[0])) { if (start !== previousEnd) fail("unsafe-archive"); previousEnd = finish; }
    if (previousEnd !== centralOffset) fail("unsafe-archive");
    // File/ancestor conflicts are rejected even if an extractor would normalize.
    for (const file of files) for (let name = path.posix.dirname(file.path); name !== "."; name = path.posix.dirname(name)) if (expected.has(name.toLowerCase())) fail("unsafe-archive");
    for (const file of files) {
      if (file.compressedBytes === 0) fail("unsafe-archive");
      const input = createReadStream(archive, { start: file.dataOffset, end: file.dataOffset + file.compressedBytes - 1 });
      const stream = file.method === 8 ? input.pipe(createInflateRaw()) : input;
      const hash = createHash("sha256");
      let crc = 0xffffffff;
      let length = 0;
      try {
        for await (const chunk of stream) {
          length += chunk.length;
          if (length > file.bytes) { stream.destroy(); input.destroy(); fail("unsafe-archive"); }
          hash.update(chunk);
          for (const byte of chunk) crc = crcTable[(crc ^ byte) & 255] ^ (crc >>> 8);
        }
      } catch { input.destroy(); fail("unsafe-archive"); }
      if (length !== file.bytes || hash.digest("hex") !== file.sha256 || ((crc ^ 0xffffffff) >>> 0) !== file.crc) fail("unsafe-archive");
    }
    return { files: files.map(({ path, bytes, compressedBytes }) => ({ path, bytes, compressedBytes })), bytes: stat.size, expandedBytes, sha256: await sha256(archive) };
  } finally { await handle.close(); }
}
