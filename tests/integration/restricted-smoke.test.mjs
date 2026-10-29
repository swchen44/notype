import test from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync,mkdirSync,rmSync,writeFileSync,readFileSync,renameSync,symlinkSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { fileURLToPath } from 'node:url';

test('restricted launcher enforces fixed command and kills its owned job on timeout', {skip:process.platform!=='win32',timeout:60000}, t=>{
 const base=mkdtempSync(join(tmpdir(),'notype-restricted-smoke-'));t.after(()=>rmSync(base,{recursive:true,force:true}));
 const systemRoot=process.env.SystemRoot || process.env.SYSTEMROOT;
 const compilerEnv={SystemRoot:systemRoot,WINDIR:systemRoot,PATH:process.env.PATH,USERPROFILE:process.env.USERPROFILE,TEMP:base,TMP:base};
 for(const key of ['RUSTUP_HOME','CARGO_HOME']) if(process.env[key]) compilerEnv[key]=process.env[key];
 const compile=(source,out,args=[])=>{const r=spawnSync('rustup',['run','1.99.1','rustc','--edition','2024',...args,source,'-o',out],{env:compilerEnv,timeout:30000,windowsHide:true,encoding:'utf8'});assert.equal(r.status,0,'compilation failed')};
 const helper=join(base,'notype-restricted-smoke.exe');
 compile(fileURLToPath(new URL('../../scripts/release/restricted-smoke-launcher.rs',import.meta.url)),helper,['--cfg','test_probe']);
 const policy=join(base,'policy-test.exe');
 compile(fileURLToPath(new URL('../../scripts/release/restricted-smoke-launcher.rs',import.meta.url)),policy,['--test']);
 const policyResult=spawnSync(policy,[],{env:{SystemRoot:systemRoot},timeout:10000,windowsHide:true,encoding:'utf8'});assert.equal(policyResult.status,0,'owner policy test failed');
 const data=join(base,'notype-native-smoke'),app=join(base,'notype-assembled','extracted','app','node_modules','notype-win32-x64','app');
 mkdirSync(join(data,'profile','AppData','Roaming'),{recursive:true});mkdirSync(join(data,'profile','AppData','Local'));mkdirSync(join(data,'temp'));mkdirSync(app,{recursive:true});
 const target=join(app,'notype.exe'),src=join(base,'fixture.rs');
 // Temporary hosted diagnostic can select the old console fixture as a control.
 const subsystem=process.env.NOTYPE_FIXTURE_SUBSYSTEM || 'windows';
 assert.ok(['windows','console'].includes(subsystem));
 const prefix=subsystem==='windows' ? '#![windows_subsystem="windows"]\n' : '';
 const env={SystemRoot:systemRoot,RUNNER_TEMP:base,NOTYPE_DATA_DIR:data,GH_TOKEN:"synthetic-secret"};
 const run=(args=[])=>spawnSync(helper,args,{env,timeout:12000,windowsHide:true,encoding:'utf8'});
 writeFileSync(src,prefix+'fn main() { assert!(std::env::var_os("GH_TOKEN").is_none()); assert!(std::env::var_os("HOME").is_none()); }');compile(src,target);
 let r=spawnSync(helper,[],{env:{...env,RUNNER_TEMP:join(base,'wrong')},timeout:12000,windowsHide:true,encoding:'utf8'});assert.notEqual(r.status,0);assert.match(r.stderr,/error stage=53/);
 renameSync(data,join(base,'actual-data'));symlinkSync(join(base,'actual-data'),data,'junction');
 r=run();assert.notEqual(r.status,0);assert.match(r.stderr,/error stage=55/);
 rmSync(data);renameSync(join(base,'actual-data'),data);
 r=run();assert.equal(r.status,0,`valid launch failed ${r.stderr}`);
 r=run(['ignored']);assert.notEqual(r.status,0);assert.match(r.stderr,/error stage=50/);
 writeFileSync(src,prefix+`use std::{process::Command,time::Duration};fn main(){if std::env::args().any(|a|a=="descendant"){std::thread::sleep(Duration::from_secs(60));return} let child=Command::new(std::env::current_exe().unwrap()).arg("descendant").spawn().unwrap();std::fs::write(std::path::PathBuf::from(std::env::var_os("NOTYPE_DATA_DIR").unwrap()).join("pid"),child.id().to_string()).unwrap();std::thread::sleep(Duration::from_secs(60));}`);compile(src,target);
 r=run();assert.notEqual(r.status,0);assert.match(r.stderr,/error stage=71/);
 const pid=Number(readFileSync(join(data,'pid'),'utf8'));assert.ok(pid>0);assert.throws(()=>process.kill(pid,0));
});
