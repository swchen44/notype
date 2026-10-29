import test from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, rmSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { fileURLToPath } from 'node:url';

test('restricted owned child independently reports a medium non-elevated token', {skip:process.platform!=='win32',timeout:60000}, t=>{
  const dir=mkdtempSync(join(tmpdir(),'notype-token-probe-'));
  t.after(()=>rmSync(dir,{recursive:true,force:true}));
  const exe=join(dir,'probe.exe');
  const source=fileURLToPath(new URL('../../scripts/release/runner-restricted-token-diagnostic.rs',import.meta.url));
  const systemRoot=process.env.SystemRoot || process.env.SYSTEMROOT;
  const env={SystemRoot:systemRoot,WINDIR:systemRoot,PATH:process.env.PATH,USERPROFILE:process.env.USERPROFILE,TEMP:dir,TMP:dir};
  for(const key of ['RUSTUP_HOME','CARGO_HOME']) if(process.env[key]) env[key]=process.env[key];
  const compile=spawnSync('rustup',['run','1.99.1','rustc','--edition','2024',source,'-o',exe],{env,timeout:30000,windowsHide:true,encoding:'utf8'});
  assert.equal(compile.status,0,'probe compilation failed');
  const result=spawnSync(exe,[],{env:{SystemRoot:systemRoot},timeout:20000,windowsHide:true,encoding:'utf8'});
  assert.equal(result.status,0,`probe failed: ${result.stderr}`);
  assert.match(result.stdout,/restricted_after elevation=0 uiaccess=0 rid=8192/);
  assert.match(result.stdout,/child elevation=0 uiaccess=0 rid=8192/);
  assert.match(result.stdout,/child_exit=0/);
  const lines=result.stdout.trim().split(/\r?\n/);
  assert.equal(lines.length,5);
  for (const line of lines) assert.match(line,/^(?:(?:parent|restricted_before|restricted_after|child) elevation=\d+ uiaccess=\d+ rid=\d+|child_exit=\d+)$/);
});
