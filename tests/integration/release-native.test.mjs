import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, mkdir, readFile, writeFile, rm } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { restoreEquivalentTauriManifest, stageNative } from '../../scripts/release/native-stage.mjs';

const cpp = ['transcribe.dll', 'ggml.dll', 'ggml-base.dll', 'ggml-cpu-x64.dll', 'ggml-cpu-sse42.dll', 'ggml-cpu-sandybridge.dll', 'ggml-cpu-haswell.dll', 'ggml-cpu-skylakex.dll', 'ggml-cpu-cannonlake.dll', 'ggml-cpu-icelake.dll', 'ggml-cpu-cascadelake.dll', 'ggml-cpu-alderlake.dll'];
const crt = ['msvcp140.dll', 'msvcp140_1.dll', 'vcruntime140.dll', 'vcruntime140_1.dll'];

async function fixture() {
  const root = await mkdtemp(path.join(tmpdir(), 'notype-native-test-'));
  const runtime = path.join(root, 'runtime'), redist = path.join(root, 'redist');
  await mkdir(runtime); await mkdir(redist);
  for (const name of cpp) await writeFile(path.join(runtime, name), `runtime ${name}`);
  for (const name of crt) await writeFile(path.join(redist, name), `crt ${name}`);
  const exe = path.join(root, 'notype.exe'); await writeFile(exe, 'native application');
  const cargoLock = path.join(root, 'Cargo.lock'), bunLock = path.join(root, 'bun.lock');
  await writeFile(cargoLock, 'cargo lock'); await writeFile(bunLock, 'bun lock');
  return { root, runtime, redist, exe, cargoLock, bunLock, output: path.join(root, 'out'), source: 'a'.repeat(40), tools: { node: '22.15.0', bun: '1.2.22', rust: '1.99.1', msvc: '14.44.35207', redist: '14.44.35211', redist_dir: '14.44.35112', cpp_source: 'https://github.com/handy-computer/transcribe.cpp', cpp_revision: '4807edaf210d0d7e8a6f7fb2a44b65966a2797f0', crt_source: 'https://learn.microsoft.com/visualstudio/releases/2022/redistribution' } };
}

test('stages real file bytes and nonsecret metadata with accurate hashes', async t => {
  const f = await fixture(); t.after(() => rm(f.root, { recursive: true, force: true }));
  await stageNative(f);
  const manifest = JSON.parse(await readFile(path.join(f.output, 'native-build.json'), 'utf8'));
  assert.equal(manifest.source_revision, f.source); assert.equal(manifest.schema, 1);
  assert.equal(manifest.files.length, 17);
  for (const item of manifest.files) {
    const bytes = await readFile(path.join(f.output, item.path));
    assert.equal(bytes.length, item.bytes); assert.equal(createHash('sha256').update(bytes).digest('hex'), item.sha256);
  }
  assert.ok(!JSON.stringify(manifest).includes(f.root));
});

test('missing runtime or an existing destination refuses without replacing files', async t => {
  const f = await fixture(); t.after(() => rm(f.root, { recursive: true, force: true }));
  await rm(path.join(f.runtime, 'ggml.dll'));
  await assert.rejects(stageNative(f));
  await mkdir(f.output); await writeFile(path.join(f.output, 'sentinel'), 'untouched');
  await assert.rejects(stageNative(f));
  assert.equal(await readFile(path.join(f.output, 'sentinel'), 'utf8'), 'untouched');
});

test('refuses incompatible CRT and forged C++ origin', async t => {
  const f = await fixture(); t.after(() => rm(f.root, { recursive: true, force: true }));
  await assert.rejects(stageNative({ ...f, tools: { ...f.tools, redist: '14.43.35211' } }));
  await assert.rejects(stageNative({ ...f, tools: { ...f.tools, cpp_revision: 'f'.repeat(40) } }));
});

test('refuses extra runtime DLLs so dynamic ORT cannot silently enter the package', async t => {
  const f = await fixture(); t.after(() => rm(f.root, { recursive: true, force: true }));
  await writeFile(path.join(f.runtime, 'onnxruntime.dll'), 'unexpected runtime');
  await assert.rejects(stageNative(f), /native-runtime-set-invalid/);
});

test('real git repository keeps genuine manifest and unrelated changes untouched', async t => {
  const root = await mkdtemp(path.join(tmpdir(), 'notype-git-cleanup-'));
  t.after(() => rm(root, { recursive: true, force: true }));
  const git = (...args) => {
    const result = spawnSync('git', args, { cwd: root, encoding: 'utf8' });
    assert.equal(result.status, 0, result.stderr);
    return result.stdout;
  };
  git('init', '-q'); git('config', 'user.name', 'fixture'); git('config', 'user.email', 'fixture@example.invalid');
  const directory = path.join(root, 'desktop', 'src-tauri'); await mkdir(directory, { recursive: true });
  const file = path.join(directory, 'Cargo.toml'); await writeFile(file, 'name = "original"\n');
  git('add', '.'); git('commit', '-qm', 'owned fixture');
  assert.doesNotThrow(() => restoreEquivalentTauriManifest(root));
  await writeFile(file, 'name = "changed"\n');
  assert.throws(() => restoreEquivalentTauriManifest(root), /native-source-dirty/);
  assert.equal(await readFile(file, 'utf8'), 'name = "changed"\n');
  await writeFile(path.join(root, 'other.txt'), 'keep');
  assert.throws(() => restoreEquivalentTauriManifest(root), /native-source-dirty/);
  assert.equal(await readFile(file, 'utf8'), 'name = "changed"\n');
  assert.equal(await readFile(path.join(root, 'other.txt'), 'utf8'), 'keep');
});
