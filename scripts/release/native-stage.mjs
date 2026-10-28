const forbiddenEnvironment = [
  'TRANSCRIBE_DIR', 'TRANSCRIBE_CMAKE_ARGS', 'CMAKE_ARGS', 'ORT_LIB_LOCATION', 'ORT_LIB_PATH',
  'ORT_PREFER_DYNAMIC_LINK', 'ORT_DYLIB_PATH', 'ORT_SKIP_DOWNLOAD', 'ORT_OFFLINE', 'CARGO_NET_OFFLINE',
  'HANDY_VC_REDIST_DIRS', 'RUSTFLAGS',
  'CARGO_ENCODED_RUSTFLAGS', 'CARGO_BUILD_RUSTFLAGS',
];

export function validateNativeInputs({ source, actualSource, clean, tools, env }) {
  if (!/^[0-9a-f]{40}$/.test(source ?? '') || source !== actualSource || clean !== true)
    throw new Error('native-source-invalid');
  if (tools?.node !== '22.15.0' || tools?.bun !== '1.2.22' || tools?.rust !== '1.99.1')
    throw new Error('native-tool-version-invalid');
  if (forbiddenEnvironment.some(name => env && Object.hasOwn(env, name))) throw new Error('native-runtime-override');
}
import { createHash } from 'node:crypto';
import { createReadStream } from 'node:fs';
import { lstat, mkdir, readdir, copyFile, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

import { cppDlls as nativeDlls, crtDlls } from './native-contract.mjs';

async function digest(file) {
  const hash = createHash('sha256');
  for await (const chunk of createReadStream(file)) hash.update(chunk);
  return hash.digest('hex');
}

async function checkedFile(file, maximum = 512 * 1024 * 1024) {
  const stat = await lstat(file);
  if (!stat.isFile() || stat.isSymbolicLink() || stat.size <= 0 || stat.size > maximum)
    throw new Error('native-input-file-invalid');
  return stat;
}

export async function stageNative({ exe, runtime, redist, output, source, tools, cargoLock, bunLock }) {
  if (!/^[0-9a-f]{40}$/.test(source ?? '')) throw new Error('native-source-invalid');
  const allowedTools = ['node', 'bun', 'rust', 'msvc', 'redist', 'redist_dir', 'cpp_source', 'cpp_revision', 'crt_source'];
  if (JSON.stringify(Object.keys(tools ?? {}).sort()) !== JSON.stringify(allowedTools.sort()) ||
      tools.node !== '22.15.0' || tools.bun !== '1.2.22' || tools.rust !== '1.99.1' ||
      tools.cpp_source !== 'https://github.com/handy-computer/transcribe.cpp' ||
      tools.cpp_revision !== '4807edaf210d0d7e8a6f7fb2a44b65966a2797f0' ||
      tools.crt_source !== 'https://learn.microsoft.com/visualstudio/releases/2022/redistribution' ||
      !/^14\.\d+\.\d+$/.test(tools.msvc ?? '') || !/^14\.\d+\.\d+$/.test(tools.redist ?? '') ||
      !/^14\.\d+\.\d+$/.test(tools.redist_dir ?? ''))
    throw new Error('native-msvc-version-invalid');
  const [major, minor, build] = tools.msvc.split('.').map(Number);
  const [crtMajor, crtMinor, crtBuild] = tools.redist.split('.').map(Number);
  if (crtMajor !== major || crtMinor !== minor || crtBuild < build)
    throw new Error('native-crt-incompatible');
  for (const directory of [runtime, redist]) {
    const stat = await lstat(directory);
    if (!stat.isDirectory() || stat.isSymbolicLink()) throw new Error('native-directory-invalid');
  }
  const actualRuntime = (await readdir(runtime)).sort();
  if (JSON.stringify(actualRuntime) !== JSON.stringify([...nativeDlls].sort()))
    throw new Error('native-runtime-set-invalid');
  const inputs = [
    ['notype.exe', exe],
    ...nativeDlls.map(name => [name, path.join(runtime, name)]),
    ...crtDlls.map(name => [name, path.join(redist, name)]),
  ];
  for (const [, file] of inputs) await checkedFile(file);
  await checkedFile(cargoLock, 16 * 1024 * 1024);
  await checkedFile(bunLock, 16 * 1024 * 1024);
  const cargoHash = await digest(cargoLock), bunHash = await digest(bunLock);
  await mkdir(output); // Exclusive creation: existing outputs are never replaced.
  const files = [];
  for (const [name, file] of inputs) {
    const destination = path.join(output, name);
    await copyFile(file, destination, 1); // COPYFILE_EXCL
    const stat = await checkedFile(destination);
    const sourceHash = await digest(file), hash = await digest(destination);
    if (sourceHash !== hash) throw new Error('native-copy-mismatch');
    files.push({ path: name, bytes: stat.size, sha256: hash });
  }
  const manifest = { schema: 1, source_revision: source, tools, cargo_lock_sha256: cargoHash, bun_lock_sha256: bunHash, files };
  await writeFile(path.join(output, 'native-build.json'), JSON.stringify(manifest, null, 2) + '\n', { flag: 'wx' });
  return manifest;
}

function runVersion(command, args, cwd) {
  const result = spawnSync(command, args, { cwd, encoding: 'utf8', windowsHide: true });
  if (result.status !== 0) throw new Error('native-tool-unavailable');
  return result.stdout.trim();
}

function runGit(cwd, args) {
  const command = process.platform === 'win32' ? 'git.exe' : 'git';
  const result = spawnSync(command, args, { cwd, encoding: 'utf8', windowsHide: true });
  if (result.status !== 0) throw new Error('native-git-failed');
  return result.stdout;
}

// Tauri may rewrite Cargo.toml without changing its normalized Git blob. Never
// restore an actual content change, an index change, or unrelated work.
export function restoreEquivalentTauriManifest(cwd, git = runGit) {
  const filename = 'desktop/src-tauri/Cargo.toml';
  const statusArgs = ['status', '--porcelain=v1', '--untracked-files=all'];
  const status = git(cwd, statusArgs).replace(/\r?\n$/, '');
  if (status === '') return;
  if (status !== ` M ${filename}`) throw new Error('native-source-dirty');
  const head = git(cwd, ['rev-parse', `HEAD:${filename}`]).trim();
  const normalized = git(cwd, ['hash-object', '--path', filename, filename]).trim();
  if (!/^[0-9a-f]{40}$/.test(head) || head !== normalized) throw new Error('native-source-dirty');
  git(cwd, ['restore', '--worktree', '--', filename]);
  if (git(cwd, statusArgs) !== '') throw new Error('native-source-dirty');
}

function preflight(source, cwd) {
  const git = process.platform === 'win32' ? 'git.exe' : 'git';
  const bun = process.platform === 'win32' ? 'bun.exe' : 'bun';
  const rustup = process.platform === 'win32' ? 'rustup.exe' : 'rustup';
  const actualSource = runVersion(git, ['rev-parse', 'HEAD'], cwd);
  const clean = runVersion(git, ['status', '--porcelain'], cwd) === '';
  const rustVersion = runVersion(rustup, ['run', '1.99.1', 'rustc', '--version'], cwd);
  const tools = { node: process.versions.node, bun: runVersion(bun, ['--version'], cwd), rust: rustVersion.match(/^rustc (\d+\.\d+\.\d+)\b/)?.[1] };
  validateNativeInputs({ source, actualSource, clean, tools, env: process.env });
  return tools;
}

async function main(args) {
  if (args.length !== 2) throw new Error('native-usage');
  if (args[0] === 'preflight') {
    preflight(args[1], process.cwd());
  } else if (args[0] === 'stage') {
    await checkedFile(args[1], 1024 * 1024);
    const config = JSON.parse(await readFile(args[1], 'utf8'));
    const git = process.platform === 'win32' ? 'git.exe' : 'git';
    if (!/^[0-9a-f]{40}$/.test(config.source ?? '') || runVersion(git, ['rev-parse', 'HEAD'], process.cwd()) !== config.source)
      throw new Error('native-source-invalid');
    restoreEquivalentTauriManifest(process.cwd());
    const liveTools = preflight(config.source, process.cwd());
    if (liveTools.node !== config.tools?.node || liveTools.bun !== config.tools?.bun || liveTools.rust !== config.tools?.rust)
      throw new Error('native-tools-changed');
    await stageNative(config);
  } else throw new Error('native-usage');
  process.stdout.write('native step completed\n');
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main(process.argv.slice(2)).catch(error => { process.stderr.write(`${error.message}\n`); process.exitCode = 1; });
}
