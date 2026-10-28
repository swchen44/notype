import path from 'node:path';
import { createHash } from 'node:crypto';
import { cp, mkdir, readFile, writeFile, stat } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { inspectBundle, bundleInventory, inspectZipArchive } from '../../packages/notype/lib/offline-bundle.mjs';
import { verifyNativeInputs, nativeNames, digestFile, validateVersion } from './assembly-policy.mjs';
import { verifyPEClosure } from './pe-closure.mjs';

const baselineSource = 'e460e37422299c67ec7862ce3b67d9382df3f77b';
const bundlePin = 'b07cf6cb8f1e2f54d8c975ecc3633f38504270587a7dbade193884271a2f78b0';
const cargoPin = '31310c3e28219400de1aae68ea041708e891130c344754685c5c7aa8073fadfb';
const bunPin = '323076c5aa1a53d57fa98c6c97e3e0201d87577450bc835e32c691fb7f14abc5';
const crtLicensePin = 'f1e3d56ceb2ad68aae0711b910375009e651ac5530fa0760f0dea6e81e54fae1';
const appRel = 'app/node_modules/notype-win32-x64';
const json = value => Buffer.from(JSON.stringify(value,null,2)+'\n');
const digest = data => createHash('sha256').update(data).digest('hex');
const readJson = async file => JSON.parse(await readFile(file,'utf8'));
const writeJson = async (file,value) => writeFile(file,json(value));
const paths = ['desktop/src-tauri/Cargo.lock','desktop/src-tauri/Cargo.toml','desktop/src-tauri/build.rs','desktop/src-tauri/tauri.windows.conf.json','desktop/src-tauri/tauri.conf.json','.cargo','desktop/.cargo','crates/notype-platform','desktop/src-tauri/resources','packages/notype','desktop/bun.lock','desktop/package.json'];

function run(cmd,args,opts={}) { return execFileSync(cmd,args,{encoding:'utf8',windowsHide:true,timeout:20*60*1000,maxBuffer:16*1024*1024,...opts}); }
export async function sourceGate(source,options={}) {
  const cwd=options.cwd ?? process.cwd(), base=options.baselineSource ?? baselineSource;
  const cp=options.cargoPin ?? cargoPin, bp=options.bunPin ?? bunPin;
  if (!/^[0-9a-f]{40}$/.test(source) || run('git',['rev-parse','HEAD'],{cwd}).trim() !== source) throw Error('source-revision-mismatch');
  if (run('git',['status','--porcelain','--untracked-files=no'],{cwd}).trim()) throw Error('source-worktree-dirty');
  if (run('git',['diff','--name-only',base,source,'--',...paths],{cwd}).trim()) throw Error('baseline-inputs-changed');
  if (await digestFile(path.join(cwd,'desktop/src-tauri/Cargo.lock')) !== cp || await digestFile(path.join(cwd,'desktop/bun.lock')) !== bp) throw Error('baseline-lock-changed');
}
export function nativeMetadata(m,source,pins={cargoPin,bunPin}) {
  const t=m.tools;
  const exact=(object,keys)=>object && typeof object==='object' && !Array.isArray(object) && Object.keys(object).sort().join(',')===keys.sort().join(',');
  if (!exact(m,['schema','source_revision','tools','cargo_lock_sha256','bun_lock_sha256','files']) || !exact(t,['node','bun','rust','msvc','redist','redist_dir','cpp_source','cpp_revision','crt_source']) || m.schema!==1 || m.source_revision!==source || m.cargo_lock_sha256!==pins.cargoPin || m.bun_lock_sha256!==pins.bunPin ||
      t.node!=='22.15.0' || t.bun!=='1.2.22' || t.rust!=='1.99.1' ||
      t.cpp_source!=='https://github.com/handy-computer/transcribe.cpp' || t.cpp_revision!=='4807edaf210d0d7e8a6f7fb2a44b65966a2797f0' ||
      t.crt_source!=='https://learn.microsoft.com/visualstudio/releases/2022/redistribution' || t.redist!=='14.44.35211' ||
      !/^14\.[0-9]+\.[0-9]+(?:\.[0-9]+)?$/.test(t.msvc) || !/^14\.[0-9]+\.[0-9]+(?:\.[0-9]+)?$/.test(t.redist) || !/^14\.[0-9]+\.[0-9]+(?:\.[0-9]+)?$/.test(t.redist_dir)) throw Error('native-provenance-invalid');
  const msvc=t.msvc.split('.').map(Number), redist=t.redist.split('.').map(Number);
  if(msvc[0]!==redist[0] || msvc[1]!==redist[1] || redist[2]<msvc[2]) throw Error('incompatible-crt-toolset');
  return t;
}
async function hashInventory(root) {
  const entries = await bundleInventory(root); const result=[];
  for (const entry of entries) result.push({...entry,sha256:await digestFile(path.join(root,entry.path))});
  return result;
}

export async function assemble({baseline,native,source,version,output,dumpbin,crtLicense}, overrides={}) {
  validateVersion(version);
  for (const p of [baseline,native,output,dumpbin,crtLicense]) if (!p || !path.isAbsolute(p)) throw Error('absolute-path-required');
  await (overrides.sourceGate ?? sourceGate)(source);
  const licenseStat=await stat(crtLicense);
  if (!licenseStat.isFile() || licenseStat.size<1 || licenseStat.size>65536 || (licenseStat.size!==39644 && !overrides.crtLicensePin)) throw Error('crt-license-invalid');
  const license=await readFile(crtLicense);
  if (digest(license)!==(overrides.crtLicensePin ?? crtLicensePin)) throw Error('crt-license-invalid');
  if (await digestFile(path.join(baseline,'bundle.json')) !== (overrides.bundlePin ?? bundlePin)) throw Error('baseline-pin-mismatch');
  await inspectBundle({facadeRoot:path.join(baseline,'app'),execPath:path.join(baseline,'runtime/node.exe'),strictModels:true});
  const original=await readJson(path.join(baseline,'bundle.json'));
  const appPath=path.join(baseline,appRel);
  const app=await readJson(path.join(appPath,'payload.json'));
  const metadata=await readJson(path.join(native,'native-build.json'));
  const tools=(overrides.nativeMetadata ?? nativeMetadata)(metadata,source);
  const nativeFiles=await verifyNativeInputs({root:native,source,baseline:app.files});
  await mkdir(output); // existing output is never overwritten
  const root=path.join(output,'bundle');
  await cp(baseline,root,{recursive:true,errorOnExist:true,force:false});
  const dstApp=path.join(root,appRel);
  const appDir=path.join(dstApp,'app');
  for (const name of nativeNames) await cp(path.join(native,name),path.join(appDir,name),{force:true});
  (overrides.verifyPE ?? verifyPEClosure)(dumpbin,appDir,[...nativeNames,'DirectML.dll','notype-platform-check.exe']);
  const replacements=new Map();
  for (const name of nativeNames) replacements.set(`app/${name}`,nativeFiles.get(name));
  const crtNotice=Buffer.from(`Visual C++ Redistributable ${tools.redist}; toolset ${tools.msvc}.\nSource: ${tools.crt_source}\nSee Visual-C-Runtime-2015-2022-License.docx for Microsoft runtime terms. Hash verification is not a legal determination.\n`);
  await writeFile(path.join(dstApp,'notices/VC-REDIST-SOURCE.txt'),crtNotice);
  replacements.set('notices/VC-REDIST-SOURCE.txt',{bytes:crtNotice.length,sha256:digest(crtNotice)});
  const licensePath='notices/Visual-C-Runtime-2015-2022-License.docx';
  await writeFile(path.join(dstApp,licensePath),license,{flag:'wx'});
  app.files.push({path:licensePath,role:'notice',source:'crt',bytes:license.length,sha256:digest(license)});
  app.sources.crt.notices.push(licensePath);
  const proof={schema:1,source,version,baseline_bundle_sha256:bundlePin,native:metadata};
  const proofBytes=json(proof);
  await writeFile(path.join(dstApp,'provenance/notype-build.json'),proofBytes);
  replacements.set('provenance/notype-build.json',{bytes:proofBytes.length,sha256:digest(proofBytes)});
  app.version=version;
  app.sources.runtime_application.revision=source;
  app.sources.assembly.revision=source;
  app.sources.cpp.revision=tools.cpp_revision;
  app.sources.crt.revision=`MSVC ${tools.msvc}; Visual C++ Redistributable ${tools.redist}`;
  app.sources.thirdparty && (app.sources.thirdparty.revision=`Locked native and frontend dependencies for ${source}; audited baseline licenses retained.`);
  const inventoryPath=path.join(dstApp,'notices/dependency-inventory.json');
  if (app.files.some(entry=>entry.path==='notices/dependency-inventory.json')) {
    const inventory=await readJson(inventoryPath);
    inventory.scope=`Native App built from ${source}; locked third-party dependencies and notices retained from verified baseline ${baselineSource}.`;
    const inventoryBytes=json(inventory);await writeFile(inventoryPath,inventoryBytes);
    replacements.set('notices/dependency-inventory.json',{bytes:inventoryBytes.length,sha256:digest(inventoryBytes)});
  }
  if (app.files.some(entry=>entry.path==='README.md')) {
    const appReadme=Buffer.from(`# notype ${version}\n\nWindows 11 x64 portable application. Source: ${source}. For offline setup use Start-notype.cmd in the complete bundle.\n`);
    await writeFile(path.join(dstApp,'README.md'),appReadme);
    replacements.set('README.md',{bytes:appReadme.length,sha256:digest(appReadme)});
  }
  for (const entry of app.files) if (replacements.has(entry.path)) Object.assign(entry,replacements.get(entry.path),{path:entry.path});
  await writeJson(path.join(dstApp,'payload.json'),app);
  const pin={...original.application,version,sha256:await digestFile(path.join(dstApp,'payload.json'))};
  const appPackage=await readJson(path.join(dstApp,'package.json'));appPackage.version=version;await writeJson(path.join(dstApp,'package.json'),appPackage);
  const facadePackage=await readJson(path.join(root,'app/package.json'));facadePackage.version=version;facadePackage.dependencies={'notype-win32-x64':version};await writeJson(path.join(root,'app/package.json'),facadePackage);
  await writeJson(path.join(root,'app/release.json'),{schema:2,status:'assembled',version,application:pin});
  const bootstrap=await readJson(path.join(root,'bootstrap/notype-bootstrap.json'));bootstrap.bootstrap_id=`notype-${version}-${source.slice(0,8)}`;await writeJson(path.join(root,'bootstrap/notype-bootstrap.json'),bootstrap);
  const guide=Buffer.concat([await readFile('packages/notype/OFFLINE-README-zh-TW.md'),Buffer.from(`\n## ${version}\n\n來源：${source}\n\n完整包已核對內容與雜湊；真人麥克風、乾淨 Windows 11 與公司網路尚未驗收。\n`)]);
  await writeFile(path.join(root,'README-zh-TW.md'),guide);
  await writeJson(path.join(root,'provenance/native-sources.json'),{schema:1,runtimeBuildSource:source,assemblySource:source,runtimeSha256:nativeFiles.get('notype.exe').sha256,baselineBundleSha256:bundlePin});
  original.version=version;original.source=source;original.id=bootstrap.bootstrap_id;original.application=pin;
  original.sources.project.revision=source;original.sources.runtime_application.revision=source;
  for (const [key,value] of Object.entries(app.sources)) original.sources[`app_${key}`]={...value,notices:value.notices.map(n=>`${appRel}/${n}`)};
  original.bootstrap.sha256=await digestFile(path.join(root,'bootstrap/notype-bootstrap.json'));
  original.files.push({path:`${appRel}/${licensePath}`,role:'notice',source:'app_crt',bytes:license.length,sha256:digest(license)});
  for (const entry of original.files) { const file=path.join(root,entry.path); entry.bytes=(await stat(file)).size;entry.sha256=await digestFile(file); }
  await writeJson(path.join(root,'bundle.json'),original);
  const hashes=[...original.files.map(f=>[f.path,f.sha256]),['bundle.json',await digestFile(path.join(root,'bundle.json'))]].sort(([a],[b])=>a<b?-1:a>b?1:0).map(([p,h])=>`${h}  ${p}\n`).join('');
  await writeFile(path.join(root,'SHA256SUMS.txt'),hashes);
  await inspectBundle({facadeRoot:path.join(root,'app'),execPath:path.join(root,'runtime/node.exe'),strictModels:true});
  if (!overrides.skipActualCli) run(path.join(root,'runtime/node.exe'),[path.join(root,'app/bin/notype.mjs'),'--offline-bundle','--check']);
  const expected=await hashInventory(root);
  const inventoryFile=path.join(output,'zip-inventory.json');await writeJson(inventoryFile,expected);
  const assets=path.join(output,'assets');await mkdir(assets);
  const name=`notype-${version}-offline-win-x64.zip`, archive=path.join(assets,name);
  const archiveScript=fileURLToPath(new URL('./archive.py',import.meta.url));
  run('python',[archiveScript,'write',root,archive,inventoryFile]);
  const audit=await inspectZipArchive(archive,{expectedFiles:expected});
  const extracted=path.join(output,'extracted');run('python',[archiveScript,'extract',extracted,archive,inventoryFile]);
  await inspectBundle({facadeRoot:path.join(extracted,'app'),execPath:path.join(extracted,'runtime/node.exe'),strictModels:true});
  if (await digestFile(archive)!==audit.sha256) throw Error('archive-changed');
  const readme=`# notype ${version}\n\nWindows 11 x64：完整解壓後執行 Start-notype.cmd。更新前從系統匣結束舊版。保留既有模型、歷史與設定。\nZIP SHA-256: ${audit.sha256}\n`;
  const verification=`# Verification\n\nSource: ${source}\nZIP SHA-256: ${audit.sha256}\n完整套件、ZIP 及獨立解壓已核對；真人麥克風、乾淨 Windows 11 與公司網路尚未驗收。\n`;
  await writeFile(path.join(assets,'Windows11-README.md'),readme);
  await writeFile(path.join(assets,'VERIFICATION.md'),verification);
  const outer=[];for(const n of [name,'Windows11-README.md','VERIFICATION.md']) outer.push(`${await digestFile(path.join(assets,n))}  ${n}\n`);
  await writeFile(path.join(assets,'SHA256SUMS.txt'),outer.join(''));
  const report={schema:1,source,version,archive:{file:name,bytes:audit.bytes,sha256:audit.sha256},checks:['native-source','native-files','pe-closure','baseline-bundle','bundle',...(!overrides.skipActualCli?['bundled-cli']:[]),'zip-crc-sha','fresh-extraction']};
  await writeJson(path.join(output,'assembly-report.json'),report);
  return report;
}

if (process.argv[1] && path.resolve(process.argv[1])===fileURLToPath(import.meta.url)) {
  try {
    const a=process.argv.slice(2); if(a.length!==14) throw Error('expected seven named arguments');
    const options={}; for(let i=0;i<a.length;i+=2) { if(!/^--(?:baseline|native|source|version|output|dumpbin|crtLicense)$/.test(a[i]) || options[a[i].slice(2)]) throw Error('invalid-argument');options[a[i].slice(2)]=a[i+1]; }
    console.log(JSON.stringify(await assemble(options)));
  } catch(error) { console.error(`assembly failed: ${error.message}`);process.exitCode=1; }
}
