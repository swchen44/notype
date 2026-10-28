import test from 'node:test';
import assert from 'node:assert/strict';
import { runOwnedChild } from '../../scripts/release/smoke-native.mjs';

test('owned child exits successfully within its bound',async()=>{
  const result=await runOwnedChild({executable:process.execPath,args:['-e','process.exit(0)'],env:{},timeoutMs:5000});
  assert.equal(result.exitCode,0); assert.equal(result.timedOut,false);
});
test('timed out owned child is terminated and reported',async()=>{
  const start=Date.now();
  const result=await runOwnedChild({executable:process.execPath,args:['-e','setInterval(()=>{},1000)'],env:{},timeoutMs:200});
  assert.equal(result.timedOut,true); assert.ok(Date.now()-start<7000);
  assert.throws(()=>process.kill(result.pid,0));
});

import { mkdtemp,mkdir,writeFile,readFile,rm } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { smokeExtractedBundle } from '../../scripts/release/smoke-native.mjs';
const sha='a'.repeat(40), version='1.2.3-rc.1';
async function smokeFixture(t, checks=['zip-crc-sha','fresh-extraction','bundled-cli']) {
  const temp=await mkdtemp(join(tmpdir(),'notype-smoke-test-')); t.after(()=>rm(temp,{recursive:true,force:true}));
  const output=join(temp,'notype-assembled'), app=join(output,'extracted','app','node_modules','notype-win32-x64','app');
  await mkdir(app,{recursive:true}); await writeFile(join(app,'notype.exe'),'synthetic');
  await writeFile(join(output,'assembly-report.json'),JSON.stringify({schema:1,source:sha,version,checks}));
  return {temp,output,exe:join(app,'notype.exe')};
}

test('only the verified extracted executable receives the isolated list-models request',async t=>{
  const f=await smokeFixture(t); let inspected=0, started=0;
  const result=await smokeExtractedBundle({outputRoot:f.output,temporaryRoot:f.temp,source:sha,version,systemRoot:'C:/Windows',
    inspect:async options=>{inspected++;assert.equal(options.facadeRoot,join(f.output,'extracted','app'));return {manifest:{source:sha,version},application:{entrypoint:'app/notype.exe'}};},
    run:async options=>{started++;assert.equal(options.executable,f.exe);assert.deepEqual(options.args,['--list-models']);assert.equal(options.timeoutMs,120000);assert.equal(options.env.GH_TOKEN,undefined);assert.equal(options.env.NOTYPE_DATA_DIR,join(f.temp,'notype-native-smoke'));return {exitCode:0,timedOut:false};}});
  assert.equal(result.exitCode,0);assert.equal(inspected,1);assert.equal(started,1);
});

test('missing extracted verification fails before process launch',async t=>{
  const f=await smokeFixture(t,['bundle']);let started=false;
  await assert.rejects(smokeExtractedBundle({outputRoot:f.output,temporaryRoot:f.temp,source:sha,version,systemRoot:'C:/Windows',inspect:async()=>{throw Error('must not inspect')},run:async()=>{started=true}}));
  assert.equal(started,false);
});

test('existing smoke profile is never reused or overwritten',async t=>{
  const f=await smokeFixture(t); await mkdir(join(f.temp,'notype-native-smoke')); let started=false;
  await assert.rejects(smokeExtractedBundle({outputRoot:f.output,temporaryRoot:f.temp,source:sha,version,systemRoot:'C:/Windows',
    inspect:async()=>({manifest:{source:sha,version},application:{entrypoint:'app/notype.exe'}}),run:async()=>{started=true}}));
  assert.equal(started,false);
});

test('timeout terminates the owned descendant as well as its parent on Windows', {skip:process.platform!=='win32'}, async t=>{
  const temp=await mkdtemp(join(tmpdir(),'notype-smoke-tree-'));t.after(()=>rm(temp,{recursive:true,force:true}));
  const pidfile=join(temp,'child-pid.txt');
  const script="const {spawn}=require('child_process');const fs=require('fs');const p=spawn(process.execPath,['-e','setInterval(()=>{},1000)'],{stdio:'ignore'});fs.writeFileSync(process.env.PIDFILE,String(p.pid));setInterval(()=>{},1000);";
  const result=await runOwnedChild({executable:process.execPath,args:['-e',script],env:{PIDFILE:pidfile},timeoutMs:3000});
  const childPid=Number((await readFile(pidfile,'utf8')).trim());
  assert.ok(Number.isInteger(childPid) && childPid>0);assert.equal(result.timedOut,true);
  assert.throws(()=>process.kill(childPid,0));
});
