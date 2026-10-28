import test from 'node:test';
import assert from 'node:assert/strict';
import path from 'node:path';
import { mkdir,readFile,writeFile } from 'node:fs/promises';
import { bundleFixture,sealBundle } from './fixtures/offline-bundle.mjs';
import { digest,json } from './fixtures/portable-package.mjs';
import { assemble } from '../../scripts/release/assemble-offline.mjs';
import { nativeNames } from '../../scripts/release/assembly-policy.mjs';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const source='a'.repeat(40);
async function fixture(t) {
  const f=await bundleFixture(t);
  const native=path.join(f.ownerRoot,'native'); await mkdir(native);
  const app=JSON.parse(await readFile(path.join(f.app,'payload.json'),'utf8'));
  for (const key of ['cpp','crt','assembly','runtime_application']) app.sources[key]={source:'https://example.org/fixture',revision:'fixture',license:'Fixture-only',notices:['NOTICE.txt']};
  await mkdir(path.join(f.app,'provenance'),{recursive:true});
  await mkdir(path.join(f.app,'notices'),{recursive:true});
  const add=async (name,content,role,sourceName)=>{
    await writeFile(path.join(f.app,name),content);
    app.files=app.files.filter(x=>x.path!==name);
    app.files.push({path:name,role,bytes:Buffer.byteLength(content),sha256:digest(content),source:sourceName});
  };
  await add('notices/VC-REDIST-SOURCE.txt','fixture-notice','notice','fixture');
  await add('provenance/notype-build.json','fixture-proof','provenance','fixture');
  const files=[];
  for (const name of nativeNames) {
    const data=`native:${name}`;
    if(name!=='notype.exe') await add(`app/${name}`,`old:${name}`,'native',name.startsWith('msvc')||name.startsWith('vcruntime')?'crt':'cpp');
    await writeFile(path.join(native,name),data);
    files.push({path:name,bytes:Buffer.byteLength(data),sha256:digest(data)});
  }
  const tools={msvc:'14.44.35207',redist:'14.44.35211',cpp_revision:'4807edaf210d0d7e8a6f7fb2a44b65966a2797f0',crt_source:'https://learn.microsoft.com/visualstudio/releases/2022/redistribution'};
  await writeFile(path.join(native,'native-build.json'),json({schema:1,source_revision:source,tools,files}));
  const appBytes=json(app);await writeFile(path.join(f.app,'payload.json'),appBytes);
  f.manifest.application.sha256=digest(appBytes);
  const release=JSON.parse(await readFile(path.join(f.facade,'release.json'),'utf8'));
  release.application.sha256=digest(appBytes);await writeFile(path.join(f.facade,'release.json'),json(release));
  await mkdir(path.join(f.root,'bootstrap'));
  const bootstrap=json({schema:1,bootstrap_id:'fixture',stt:{model_id:'fixture',root:'models'}});await writeFile(path.join(f.root,'bootstrap/notype-bootstrap.json'),bootstrap);
  f.manifest.bootstrap={path:'bootstrap/notype-bootstrap.json',sha256:digest(bootstrap)};
  f.manifest.sources.project={source:'https://example.org/project',revision:'fixture',license:'Fixture-only',notices:['runtime/LICENSE.txt']};
  f.manifest.sources.runtime_application={source:'https://example.org/project',revision:'fixture',license:'Fixture-only',notices:['runtime/LICENSE.txt']};
  await mkdir(path.join(f.root,'provenance'));await writeFile(path.join(f.root,'provenance/native-sources.json'),'fixture');
  await sealBundle(f);
  const bundlePin=digest(await readFile(path.join(f.root,'bundle.json')));
  const overrides={bundlePin,sourceGate:async()=>{},nativeMetadata:m=>m.tools,verifyPE:()=>{},skipActualCli:true};
  const args={baseline:f.root,native,source,version:'0.1.2-local.1',output:path.join(f.ownerRoot,'output'),dumpbin:path.join(f.ownerRoot,'dumpbin.exe')};
  return {f,args,overrides};
}

test('assembles and audits a complete synthetic offline bundle from matching native inputs',async t=>{
  const {args,overrides}=await fixture(t);
  const report=await assemble(args,overrides);
  assert.equal(report.source,source);
  assert.equal(report.version,'0.1.2-local.1');
  assert.ok(report.archive.bytes>0);
  const output=await readFile(path.join(args.output,'extracted/app/node_modules/notype-win32-x64/app/notype.exe'),'utf8');
  assert.equal(output,'native:notype.exe');
});

test('a corrupted baseline or native runtime never produces a completed release report',async t=>{
  const {args,overrides}=await fixture(t);
  await writeFile(path.join(args.native,'ggml.dll'),'tampered');
  await assert.rejects(assemble(args,overrides));
  await assert.rejects(readFile(path.join(args.output,'assembly-report.json')));
  const fresh=await fixture(t);
  await writeFile(path.join(fresh.args.baseline,'runtime/LICENSE.txt'),'tampered');
  await assert.rejects(assemble(fresh.args,fresh.overrides));
  await assert.rejects(readFile(path.join(fresh.args.output,'assembly-report.json')));
});

test('baseline unpacker rejects an unpinned archive without creating a destination',async t=>{
  const {f}=await fixture(t);
  const archive=path.join(f.ownerRoot,'untrusted.zip');
  const destination=path.join(f.ownerRoot,'must-not-exist');
  await writeFile(archive,'not a pinned archive');
  const script=fileURLToPath(new URL('../../scripts/release/unpack-baseline.py',import.meta.url));
  const run=spawnSync('python',[script,'--archive',archive,'--destination',destination],{encoding:'utf8',windowsHide:true});
  assert.notEqual(run.status,0);
  await assert.rejects(readFile(path.join(destination,'bundle.json')));
});
