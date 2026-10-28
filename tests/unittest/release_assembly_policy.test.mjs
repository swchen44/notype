import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, mkdir, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { verifyNativeInputs, validateVersion } from '../../scripts/release/assembly-policy.mjs';

const runtime = ['transcribe.dll', 'ggml.dll', 'ggml-base.dll', ...['x64','sse42','sandybridge','haswell','alderlake','cannonlake','cascadelake','icelake','skylakex'].map(x => `ggml-cpu-${x}.dll`), 'msvcp140.dll','msvcp140_1.dll','vcruntime140.dll','vcruntime140_1.dll'];
const source = 'a'.repeat(40);
const hash = b => createHash('sha256').update(b).digest('hex');
async function fixture(t) {
  const root = await mkdtemp(path.join(tmpdir(), 'notype-assembly-native-'));
  t.after(async () => { const {rm} = await import('node:fs/promises'); await rm(root, {recursive:true,force:true}); });
  const files = [];
  const baseline = [];
  for (const name of ['notype.exe', ...runtime]) {
    const b = Buffer.from(`trusted ${name}`);
    await writeFile(path.join(root,name),b);
    files.push({path:name,bytes:b.length,sha256:hash(b)});
    if (name !== 'notype.exe') baseline.push({path:`app/${name}`,bytes:b.length,sha256:hash(b),source:name.startsWith('msvc') || name.startsWith('vcruntime') ? 'crt':'cpp'});
  }
  await writeFile(path.join(root,'native-build.json'),JSON.stringify({schema:1,source_revision:source,files}));
  return {root,source,baseline};
}

test('matches a complete same-source native runtime to the verified bundle baseline', async t => {
  const f = await fixture(t);
  const accepted = await verifyNativeInputs(f);
  assert.equal(accepted.get('notype.exe').path,'notype.exe');
  assert.equal(accepted.size,17);
});

test('rejects changed runtime bytes and unexpected files before packaging', async t => {
  const f = await fixture(t);
  await writeFile(path.join(f.root,'transcribe.dll'),'changed');
  await assert.rejects(verifyNativeInputs(f));
  const g = await fixture(t);
  await mkdir(path.join(g.root,'unlisted'));
  await assert.rejects(verifyNativeInputs(g));
});

test('accepts only bounded SemVer without metadata or numeric leading zeros', () => {
  for (const v of ['1.2.3','1.2.3-beta.1','1.2.3-local.1']) assert.doesNotThrow(()=>validateVersion(v));
  for (const v of ['v1.2.3','01.2.3','1.2.3-rc.01','1.2.3+build','1.2.3;echo','1.2.3-'+('x'.repeat(120))]) assert.throws(()=>validateVersion(v));
});

test('rejects a native build attributed to another commit and untrusted metadata', async t => {
  const f=await fixture(t);
  await assert.rejects(verifyNativeInputs({...f,source:'b'.repeat(40)}));
  const p=path.join(f.root,'native-build.json');
  const data=JSON.parse(await (await import('node:fs/promises')).readFile(p,'utf8'));
  data.files[0].note='untrusted';
  await writeFile(p,JSON.stringify(data));
  await assert.rejects(verifyNativeInputs(f));
});
