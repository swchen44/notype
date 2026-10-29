import { spawn } from 'node:child_process';
import path from 'node:path';

/** Only explicitly selected system values and synthetic owned paths reach the App. */
export function isolatedEnvironment({dataRoot, systemRoot, separator=path.delimiter}) {
  if (!dataRoot || !systemRoot) throw Error('smoke-environment-invalid');
  return {
    NOTYPE_DATA_DIR:dataRoot,
    USERPROFILE:`${dataRoot}/profile`,
    APPDATA:`${dataRoot}/profile/AppData/Roaming`, LOCALAPPDATA:`${dataRoot}/profile/AppData/Local`,
    TEMP:`${dataRoot}/temp`, TMP:`${dataRoot}/temp`,
    SystemRoot:systemRoot, WINDIR:systemRoot,
    PATH:[`${systemRoot}/System32`,systemRoot].join(separator),
  };
}

/** Run a child we own, kill its Windows process tree on timeout, and await exit. */
export function runOwnedChild({executable,args,env,timeoutMs}) {
  if (!path.isAbsolute(executable) || !Array.isArray(args) || !env || !Number.isInteger(timeoutMs) || timeoutMs < 1 || timeoutMs > 120000) throw Error('smoke-child-invalid');
  return new Promise((resolve,reject)=>{
    const child=spawn(executable,args,{cwd:path.dirname(executable),env,shell:false,windowsHide:true,stdio:'ignore'});
    let timedOut=false; let fallback;
    const timer=setTimeout(()=>{
      timedOut=true;
      if (process.platform==='win32' && Number.isInteger(child.pid) && child.pid>0) {
        const root=process.env.SystemRoot || process.env.SYSTEMROOT;
        if (!root) { child.kill(); return; }
        const killer=spawn(path.join(root,'System32','taskkill.exe'),['/PID',String(child.pid),'/T','/F'],{shell:false,windowsHide:true,stdio:'ignore'});
        killer.on('error',()=>child.kill());
        fallback=setTimeout(()=>child.kill(),5000);
      } else child.kill('SIGKILL');
    },timeoutMs);
    child.once('error',()=>{clearTimeout(timer);if(fallback)clearTimeout(fallback);reject(Error('smoke-child-start-failed'));});
    child.once('exit',code=>{clearTimeout(timer);if(fallback)clearTimeout(fallback);resolve({pid:child.pid,exitCode:code,timedOut});});
  });
}

/** The helper has no caller-selected command: it derives the verified bundle path from the owned data root. */
export function runRestrictedSmoke({executable,args,env,timeoutMs}) {
  if (!env?.NOTYPE_DATA_DIR || !path.isAbsolute(env.NOTYPE_DATA_DIR) ||
      path.basename(env.NOTYPE_DATA_DIR)!=='notype-native-smoke' ||
      executable!==path.join(path.dirname(env.NOTYPE_DATA_DIR),'notype-assembled','extracted','app','node_modules','notype-win32-x64','app','notype.exe') ||
      args.length!==1 || args[0]!=='--list-models' || timeoutMs!==120000) throw Error('smoke-launcher-input-invalid');
  const helper=path.join(path.dirname(env.NOTYPE_DATA_DIR),'notype-restricted-smoke.exe');
  return new Promise((resolve,reject)=>{
    const child=spawn(helper,[],{cwd:path.dirname(helper),env,shell:false,windowsHide:true,stdio:['ignore','ignore','pipe']});
    let stderr='',timedOut=false;
    child.stderr.on('data',chunk=>{if(stderr.length<256)stderr+=(chunk.toString('utf8')).slice(0,256-stderr.length)});
    const timer=setTimeout(()=>{timedOut=true;child.kill()},timeoutMs);
    child.once('error',()=>{clearTimeout(timer);reject(Error('smoke-launcher-start-failed'))});
    child.once('close',code=>{clearTimeout(timer);
      const match=/^error stage=(\d{1,3}) win32=(\d{1,10})\r?\n?$/.exec(stderr);
      if(code!==0 && !timedOut && match) return reject(Error(`smoke-launcher-stage-${match[1]}-win32-${match[2]}`));
      resolve({exitCode:code,timedOut});
    });
  });
}

import { mkdir, lstat, readFile, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { inspectBundle } from '../../packages/notype/lib/offline-bundle.mjs';
import { validateVersion } from './assembly-policy.mjs';

/** Inspect the independently extracted bundle before launching only its declared EXE. */
export async function smokeExtractedBundle({outputRoot,temporaryRoot,source,version,systemRoot,inspect=inspectBundle,run=runRestrictedSmoke}) {
  validateVersion(version);
  if (!/^[a-f0-9]{40}$/.test(source) || !path.isAbsolute(temporaryRoot) ||
      path.resolve(outputRoot)!==path.join(path.resolve(temporaryRoot),'notype-assembled') || !systemRoot) throw Error('smoke-input-invalid');
  const reportPath=path.join(outputRoot,'assembly-report.json');
  const report=JSON.parse(await readFile(reportPath,'utf8'));
  const required=['zip-crc-sha','fresh-extraction','bundled-cli'];
  if(report.schema!==1 || report.source!==source || report.version!==version ||
     !Array.isArray(report.checks) || required.some(check=>!report.checks.includes(check))) throw Error('smoke-assembly-unverified');
  const root=path.join(outputRoot,'extracted');
  const bundle=await inspect({facadeRoot:path.join(root,'app'),execPath:path.join(root,'runtime','node.exe'),strictModels:true});
  if(bundle.manifest?.source!==source || bundle.manifest?.version!==version ||
     bundle.application?.entrypoint!=='app/notype.exe') throw Error('smoke-extracted-source-mismatch');
  const executable=path.join(root,'app','node_modules','notype-win32-x64','app','notype.exe');
  const exeStat=await lstat(executable);
  if(!exeStat.isFile() || exeStat.isSymbolicLink() || exeStat.size<=0) throw Error('smoke-executable-invalid');
  const dataRoot=path.join(path.resolve(temporaryRoot),'notype-native-smoke');
  await mkdir(dataRoot); // Exclusive: never reuse or overwrite an existing user directory.
  await mkdir(path.join(dataRoot,'profile','AppData','Roaming'),{recursive:true});
  await mkdir(path.join(dataRoot,'profile','AppData','Local'));
  await mkdir(path.join(dataRoot,'temp'));
  const result=await run({executable,args:['--list-models'],env:isolatedEnvironment({dataRoot,systemRoot}),timeoutMs:120000});
  if(result.timedOut || result.exitCode!==0) throw Error(result.timedOut?'smoke-timeout':'smoke-child-failed');
  if(!report.checks.includes('native-headless-list-models')) report.checks.push('native-headless-list-models');
  await writeFile(reportPath,JSON.stringify(report,null,2)+'\n','utf8');
  return {exitCode:result.exitCode};
}

if(process.argv[1] && path.resolve(process.argv[1])===fileURLToPath(import.meta.url)) {
  const temporaryRoot=process.env.RUNNER_TEMP;
  const systemRoot=process.env.SystemRoot || process.env.SYSTEMROOT;
  if(process.platform!=='win32' || !temporaryRoot || !systemRoot) {
    console.error('native smoke failed: windows-runner-required'); process.exitCode=1;
  } else {
    smokeExtractedBundle({outputRoot:path.join(temporaryRoot,'notype-assembled'),temporaryRoot,
      source:process.env.GITHUB_SHA,version:process.env.RELEASE_VERSION,systemRoot})
      .then(()=>console.log('Verified extracted App --list-models exited 0 (isolated data).'))
      .catch(error=>{console.error(`native smoke failed: ${error.message}`);process.exitCode=1;});
  }
}
