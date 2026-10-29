import test from 'node:test';
import assert from 'node:assert/strict';
import path from 'node:path';
import { mkdir,readFile,writeFile } from 'node:fs/promises';
import { bundleFixture,sealBundle } from './fixtures/offline-bundle.mjs';
import { digest,json } from './fixtures/portable-package.mjs';
import { assemble, nativeMetadata, sourceGate } from '../../scripts/release/assemble-offline.mjs';
import { nativeNames } from '../../scripts/release/assembly-policy.mjs';
import { cppDlls, crtDlls } from '../../scripts/release/native-contract.mjs';
import { stageNative } from '../../scripts/release/native-stage.mjs';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { mkdtemp,rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';

const source='a'.repeat(40);

test('CRLF working locks and LF Git blobs give identical source and native provenance',async t=>{
  const root=await mkdtemp(path.join(tmpdir(),'notype-lock-eol-'));t.after(()=>rm(root,{recursive:true,force:true}));
  const repo=path.join(root,'repo'), desktop=path.join(repo,'desktop'), tauri=path.join(desktop,'src-tauri');
  await mkdir(tauri,{recursive:true});
  const cargo=path.join(tauri,'Cargo.lock'), bun=path.join(desktop,'bun.lock');
  await writeFile(cargo,'cargo-line\r\nsecond-line\n');await writeFile(bun,'bun-line\r\n');
  const git=(...argv)=>{const p=spawnSync('git',argv,{cwd:repo,encoding:'utf8'});assert.equal(p.status,0,p.stderr);return p.stdout.trim();};
  git('init','-q');git('config','user.name','fixture');git('config','user.email','fixture@example.invalid');git('config','core.autocrlf','true');
  git('add','.');git('commit','-qm','fixture');const head=git('rev-parse','HEAD');
  const cargoPin=digest('cargo-line\nsecond-line\n'),bunPin=digest('bun-line\n');
  await sourceGate(head,{cwd:repo,baselineSource:head,cargoPin,bunPin});
  await assert.rejects(sourceGate(head,{cwd:repo,baselineSource:head,cargoPin:'0'.repeat(64),bunPin}),error=>{
    assert.match(error.message,new RegExp(`baseline-lock-changed:cargo:expected=${'0'.repeat(64)}:actual=${cargoPin}`));
    assert.ok(!error.message.includes(repo));assert.ok(!error.message.includes('cargo-line'));return true;
  });
  const runtime=path.join(root,'runtime'),redist=path.join(root,'redist'),exe=path.join(root,'notype.exe');
  await mkdir(runtime);await mkdir(redist);await writeFile(exe,'fixture');
  for(const name of cppDlls) await writeFile(path.join(runtime,name),name);
  for(const name of crtDlls) await writeFile(path.join(redist,name),name);
  const tools={node:'22.15.0',bun:'1.2.22',rust:'1.99.1',msvc:'14.44.35207',redist:'14.44.35211',redist_dir:'14.44.35112',cpp_source:'https://github.com/handy-computer/transcribe.cpp',cpp_revision:'4807edaf210d0d7e8a6f7fb2a44b65966a2797f0',crt_source:'https://learn.microsoft.com/visualstudio/releases/2022/redistribution'};
  const staged=await stageNative({exe,runtime,redist,output:path.join(root,'out'),source:head,tools,cargoLock:cargo,bunLock:bun,sourceRoot:repo});
  assert.equal(staged.cargo_lock_sha256,cargoPin);assert.equal(staged.bun_lock_sha256,bunPin);
  await assert.rejects(stageNative({exe,runtime,redist,output:path.join(root,'wrong-source'),source:'b'.repeat(40),tools,cargoLock:cargo,bunLock:bun,sourceRoot:repo}),/lock-source-invalid/);
  const lfRepo=path.join(root,'lf-repo'),lfTauri=path.join(lfRepo,'desktop','src-tauri');await mkdir(lfTauri,{recursive:true});
  const lfCargo=path.join(lfTauri,'Cargo.lock'),lfBun=path.join(lfRepo,'desktop','bun.lock');
  await writeFile(lfCargo,'cargo-line\nsecond-line\n');await writeFile(lfBun,'bun-line\n');
  const lfGit=(...argv)=>{const p=spawnSync('git',argv,{cwd:lfRepo,encoding:'utf8'});assert.equal(p.status,0,p.stderr);return p.stdout.trim();};
  lfGit('init','-q');lfGit('config','user.name','fixture');lfGit('config','user.email','fixture@example.invalid');lfGit('config','core.autocrlf','false');
  lfGit('add','.');lfGit('commit','-qm','fixture');const lfHead=lfGit('rev-parse','HEAD');
  await sourceGate(lfHead,{cwd:lfRepo,baselineSource:lfHead,cargoPin,bunPin});
  const lfStaged=await stageNative({exe,runtime,redist,output:path.join(root,'lf-out'),source:lfHead,tools,cargoLock:lfCargo,bunLock:lfBun,sourceRoot:lfRepo});
  assert.equal(lfStaged.cargo_lock_sha256,cargoPin);assert.equal(lfStaged.bun_lock_sha256,bunPin);
  await writeFile(bun,'bun-evil\r\n');
  await assert.rejects(sourceGate(head,{cwd:repo,baselineSource:head,cargoPin,bunPin}));
  await assert.rejects(stageNative({exe,runtime,redist,output:path.join(root,'bad'),source:head,tools,cargoLock:cargo,bunLock:bun,sourceRoot:repo}));
});

test('binary, invalid UTF-8 and explicit binary lock attributes are refused',async t=>{
  for(const [name,content,attributes] of [
    ['nul',Buffer.from([65,0,66]),null],
    ['invalid',Buffer.from([0xff,0xfe]),null],
    ['bare-cr',Buffer.from('a\rb'),null],
    ['binary-attribute',Buffer.from('valid\n'),'/desktop/src-tauri/Cargo.lock -text\n'],
    ['filter',Buffer.from('valid\n'),'/desktop/src-tauri/Cargo.lock filter=example\n'],
  ]) {
    const root=await mkdtemp(path.join(tmpdir(),`notype-lock-${name}-`));t.after(()=>rm(root,{recursive:true,force:true}));
    const tauri=path.join(root,'desktop','src-tauri');await mkdir(tauri,{recursive:true});
    await writeFile(path.join(tauri,'Cargo.lock'),content);await writeFile(path.join(root,'desktop','bun.lock'),'bun\n');
    if(attributes) await writeFile(path.join(root,'.gitattributes'),attributes);
    const git=(...argv)=>{const p=spawnSync('git',argv,{cwd:root,encoding:'utf8'});assert.equal(p.status,0,p.stderr);return p.stdout.trim();};
    git('init','-q');git('config','user.name','fixture');git('config','user.email','fixture@example.invalid');git('add','.');git('commit','-qm','fixture');
    const head=git('rev-parse','HEAD');
    await assert.rejects(sourceGate(head,{cwd:root,baselineSource:head,cargoPin:digest(content),bunPin:digest('bun\n')}),/lock-(?:binary-or-invalid-utf8|attributes-invalid):cargo/);
  }
});
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
    if(name!=='notype.exe') {
      const isCrt=name.startsWith('msvc')||name.startsWith('vcruntime');
      await add(`app/${name}`,isCrt?data:`old:${name}`,'native',isCrt?'crt':'cpp');
    }
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
  const crtLicense=path.join(f.ownerRoot,'license.docx');await writeFile(crtLicense,'fixture licence');
  const overrides={bundlePin,sourceGate:async()=>{},nativeMetadata:m=>m.tools,verifyPE:()=>{},skipActualCli:true,crtLicensePin:digest('fixture licence')};
  const args={baseline:f.root,native,source,version:'0.1.2-local.1',output:path.join(f.ownerRoot,'output'),dumpbin:path.join(f.ownerRoot,'dumpbin.exe'),crtLicense};
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
  const licence=await readFile(path.join(args.output,'extracted/app/node_modules/notype-win32-x64/notices/Visual-C-Runtime-2015-2022-License.docx'),'utf8');
  assert.equal(licence,'fixture licence');
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

test('archive boundary rejects traversal even when Python assertions are disabled',async t=>{
  const {f}=await fixture(t);
  const root=path.join(f.ownerRoot,'archive-input'); await mkdir(root);
  const outside=path.join(f.ownerRoot,'outside'); await writeFile(outside,'outside');
  const inventory=path.join(f.ownerRoot,'malicious-inventory.json');
  await writeFile(inventory,JSON.stringify([{path:'../outside',bytes:7,sha256:digest('outside')}]));
  const script=fileURLToPath(new URL('../../scripts/release/archive.py',import.meta.url));
  const run=spawnSync('python',['-O',script,'write',root,path.join(f.ownerRoot,'out.zip'),inventory],{encoding:'utf8',windowsHide:true});
  assert.notEqual(run.status,0);
});

test('native provenance rejects lock, toolchain and CRT changes',()=>{
  const m={schema:1,source_revision:source,cargo_lock_sha256:'c'.repeat(64),bun_lock_sha256:'d'.repeat(64),files:[],tools:{node:'22.15.0',bun:'1.2.22',rust:'1.99.1',msvc:'14.44.35207',redist:'14.44.35211',redist_dir:'14.44.35112',cpp_source:'https://github.com/handy-computer/transcribe.cpp',cpp_revision:'4807edaf210d0d7e8a6f7fb2a44b65966a2797f0',crt_source:'https://learn.microsoft.com/visualstudio/releases/2022/redistribution'}};
  const pins={cargoPin:'c'.repeat(64),bunPin:'d'.repeat(64)};
  assert.doesNotThrow(()=>nativeMetadata(m,source,pins));
  for(const changed of [
    {...m,cargo_lock_sha256:'e'.repeat(64)},
    {...m,tools:{...m.tools,rust:'1.98.0'}},
    {...m,tools:{...m.tools,redist:'14.44.35212'}},
    {...m,source_revision:'b'.repeat(40)},
  ]) assert.throws(()=>nativeMetadata(changed,source,pins));
});

test('changed runtime licence is rejected before generating a report',async t=>{
  const {args,overrides}=await fixture(t);
  await writeFile(args.crtLicense,'altered licence');
  await assert.rejects(assemble(args,overrides),/crt-license-invalid/);
  await assert.rejects(readFile(path.join(args.output,'assembly-report.json')));
});

test('real git source and lock gates reject mismatches while preserving existing output',async t=>{
  const {f,args,overrides}=await fixture(t);
  const repo=path.join(f.ownerRoot,'repo'); await mkdir(path.join(repo,'desktop','src-tauri'),{recursive:true});
  await writeFile(path.join(repo,'desktop','src-tauri','Cargo.lock'),'cargo');
  await writeFile(path.join(repo,'desktop','bun.lock'),'bun');
  const git=(...argv)=>{const p=spawnSync('git',argv,{cwd:repo,encoding:'utf8'});assert.equal(p.status,0,p.stderr);return p.stdout.trim();};
  git('init','-q'); git('config','user.name','fixture');git('config','user.email','fixture@example.invalid');
  git('add','.');git('commit','-qm','fixture');
  const head=git('rev-parse','HEAD'), options={cwd:repo,baselineSource:head,cargoPin:digest('cargo'),bunPin:digest('bun')};
  await sourceGate(head,options);
  await mkdir(args.output);await writeFile(path.join(args.output,'sentinel'),'keep');
  overrides.sourceGate=s=>sourceGate(s,options);
  await assert.rejects(assemble({...args,source:'b'.repeat(40)},overrides),/source-revision-mismatch/);
  overrides.sourceGate=s=>sourceGate(s,{...options,cargoPin:'0'.repeat(64)});
  await assert.rejects(assemble({...args,source:head},overrides),/baseline-lock-changed/);
  overrides.sourceGate=s=>sourceGate(s,options);
  await writeFile(path.join(repo,'desktop','bun.lock'),'changed');
  await assert.rejects(assemble({...args,source:head},overrides),/source-worktree-dirty/);
  git('add','.');git('commit','-qm','changed lock');
  const changed=git('rev-parse','HEAD');
  await assert.rejects(assemble({...args,source:changed},overrides),/baseline-inputs-changed/);
  assert.equal(await readFile(path.join(args.output,'sentinel'),'utf8'),'keep');
  await assert.rejects(readFile(path.join(args.output,'assembly-report.json')));
});

test('actual native provenance gate rejects toolchain and CRT changes before touching output',async t=>{
  const {args,overrides}=await fixture(t);
  const file=path.join(args.native,'native-build.json');
  const original=JSON.parse(await readFile(file,'utf8'));
  Object.assign(original,{cargo_lock_sha256:'c'.repeat(64),bun_lock_sha256:'d'.repeat(64)});
  Object.assign(original.tools,{node:'22.15.0',bun:'1.2.22',rust:'1.99.1',redist_dir:'14.44.35112',cpp_source:'https://github.com/handy-computer/transcribe.cpp'});
  overrides.nativeMetadata=(m,s)=>nativeMetadata(m,s,{cargoPin:'c'.repeat(64),bunPin:'d'.repeat(64)});
  await mkdir(args.output);await writeFile(path.join(args.output,'sentinel'),'keep');
  for(const change of [{rust:'1.98.0'},{redist:'14.44.35212'}]) {
    await writeFile(file,JSON.stringify({...original,tools:{...original.tools,...change}}));
    await assert.rejects(assemble(args,overrides),/native-provenance-invalid/);
    assert.equal(await readFile(path.join(args.output,'sentinel'),'utf8'),'keep');
    await assert.rejects(readFile(path.join(args.output,'assembly-report.json')));
  }
  const crt=Buffer.from('replacement runtime');
  await writeFile(path.join(args.native,'vcruntime140.dll'),crt);
  const entry=original.files.find(x=>x.path==='vcruntime140.dll');
  Object.assign(entry,{bytes:crt.length,sha256:digest(crt)});
  await writeFile(file,JSON.stringify(original));
  await assert.rejects(assemble(args,overrides),/crt-baseline-mismatch/);
  assert.equal(await readFile(path.join(args.output,'sentinel'),'utf8'),'keep');
  await assert.rejects(readFile(path.join(args.output,'assembly-report.json')));
});

test('valid source, provenance and inputs cannot replace an existing output directory',async t=>{
  const {f,args,overrides}=await fixture(t);
  const repo=path.join(f.ownerRoot,'valid-repo');await mkdir(path.join(repo,'desktop','src-tauri'),{recursive:true});
  await writeFile(path.join(repo,'desktop','src-tauri','Cargo.lock'),'cargo');
  await writeFile(path.join(repo,'desktop','bun.lock'),'bun');
  const git=(...argv)=>{const p=spawnSync('git',argv,{cwd:repo,encoding:'utf8'});assert.equal(p.status,0,p.stderr);return p.stdout.trim();};
  git('init','-q');git('config','user.name','fixture');git('config','user.email','fixture@example.invalid');
  git('add','.');git('commit','-qm','fixture');
  const head=git('rev-parse','HEAD'); args.source=head;
  const sourceOptions={cwd:repo,baselineSource:head,cargoPin:digest('cargo'),bunPin:digest('bun')};
  overrides.sourceGate=s=>sourceGate(s,sourceOptions);
  const file=path.join(args.native,'native-build.json');const manifest=JSON.parse(await readFile(file,'utf8'));
  Object.assign(manifest,{source_revision:head,cargo_lock_sha256:digest('cargo'),bun_lock_sha256:digest('bun')});
  Object.assign(manifest.tools,{node:'22.15.0',bun:'1.2.22',rust:'1.99.1',redist_dir:'14.44.35112',cpp_source:'https://github.com/handy-computer/transcribe.cpp'});
  await writeFile(file,JSON.stringify(manifest));
  overrides.nativeMetadata=(m,s)=>nativeMetadata(m,s,{cargoPin:digest('cargo'),bunPin:digest('bun')});
  await mkdir(args.output);await writeFile(path.join(args.output,'sentinel'),'untouched');
  await assert.rejects(assemble(args,overrides),error=>error.code==='EEXIST');
  assert.equal(await readFile(path.join(args.output,'sentinel'),'utf8'),'untouched');
  await assert.rejects(readFile(path.join(args.output,'assembly-report.json')));
});

test('changed upstream license or provenance cannot reuse old package notices',async t=>{
  for (const changedPath of ['desktop/LICENSE','desktop/UPSTREAM.md']) {
    const {f,args,overrides}=await fixture(t);
    const repo=path.join(f.ownerRoot,'notice-repo');await mkdir(path.join(repo,'desktop','src-tauri'),{recursive:true});
    await writeFile(path.join(repo,'desktop','src-tauri','Cargo.lock'),'cargo');
    await writeFile(path.join(repo,'desktop','bun.lock'),'bun');
    await writeFile(path.join(repo,'desktop','LICENSE'),'old license');
    await writeFile(path.join(repo,'desktop','UPSTREAM.md'),'old source');
    const git=(...argv)=>{const p=spawnSync('git',argv,{cwd:repo,encoding:'utf8'});assert.equal(p.status,0,p.stderr);return p.stdout.trim();};
    git('init','-q');git('config','user.name','fixture');git('config','user.email','fixture@example.invalid');
    git('add','.');git('commit','-qm','baseline');const baseline=git('rev-parse','HEAD');
    await writeFile(path.join(repo,...changedPath.split('/')),'new contents');git('add','.');git('commit','-qm','changed notice');
    const head=git('rev-parse','HEAD');
    overrides.sourceGate=s=>sourceGate(s,{cwd:repo,baselineSource:baseline,cargoPin:digest('cargo'),bunPin:digest('bun')});
    await mkdir(args.output);await writeFile(path.join(args.output,'sentinel'),'keep');
    await assert.rejects(assemble({...args,source:head},overrides),/baseline-inputs-changed/);
    assert.equal(await readFile(path.join(args.output,'sentinel'),'utf8'),'keep');
    await assert.rejects(readFile(path.join(args.output,'assembly-report.json')));
  }
});
