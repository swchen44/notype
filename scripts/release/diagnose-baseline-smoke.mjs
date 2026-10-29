// Temporary local.7 runner diagnostic; this is not a source-revision release acceptance.
import path from 'node:path';
import { mkdir } from 'node:fs/promises';
import { inspectBundle } from '../../packages/notype/lib/offline-bundle.mjs';
import { digestFile } from './assembly-policy.mjs';
import { isolatedEnvironment,runRestrictedSmoke,runOwnedChild } from './smoke-native.mjs';
const base=process.env.RUNNER_TEMP, systemRoot=process.env.SystemRoot || process.env.SYSTEMROOT;
if (!base || !path.isAbsolute(base) || !systemRoot) throw Error('diagnostic-environment-invalid');
const root=path.join(base,'notype-assembled','extracted');
if(await digestFile(path.join(root,'bundle.json'))!=='b07cf6cb8f1e2f54d8c975ecc3633f38504270587a7dbade193884271a2f78b0') throw Error('diagnostic-baseline-pin');
await inspectBundle({facadeRoot:path.join(root,'app'),execPath:path.join(root,'runtime','node.exe'),strictModels:true});
const data=path.join(base,'notype-native-smoke');await mkdir(data);
await mkdir(path.join(data,'profile','AppData','Roaming'),{recursive:true});await mkdir(path.join(data,'profile','AppData','Local'));await mkdir(path.join(data,'temp'));
const env=isolatedEnvironment({dataRoot:data,systemRoot});
const check=await runOwnedChild({executable:path.join(root,'runtime','node.exe'),args:[path.join(root,'app','bin','notype.mjs'),'--offline-bundle','--check'],env,timeoutMs:120000});
if(check.timedOut || check.exitCode!==0) throw Error('diagnostic-baseline-check-failed');
const result=await runRestrictedSmoke({executable:path.join(root,'app','node_modules','notype-win32-x64','app','notype.exe'),args:['--list-models'],env,timeoutMs:120000});
if(result.timedOut || result.exitCode!==0) throw Error('diagnostic-restricted-smoke-failed');
console.log('Pinned local.7 baseline passed; not a current-source release acceptance.');
