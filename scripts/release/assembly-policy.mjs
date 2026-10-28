import { createHash } from 'node:crypto';
import { createReadStream } from 'node:fs';
import { lstat, readFile, readdir } from 'node:fs/promises';
import path from 'node:path';

import { cppDlls, crtDlls, nativeNames } from './native-contract.mjs';
export { nativeNames } from './native-contract.mjs';
const dlls = [...cppDlls,...crtDlls];
export function validateVersion(version) {
  if (typeof version !== 'string' || version.length > 64 || !/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$/.test(version)) throw Error('invalid-version');
  const pre=version.split('-').slice(1).join('-');
  if (pre && pre.split('.').some(x=>/^\d+$/.test(x) && x.length>1 && x[0]==='0')) throw Error('invalid-version');
}
export async function digestFile(file) {
  const hash = createHash('sha256');
  for await (const chunk of createReadStream(file)) hash.update(chunk);
  return hash.digest('hex');
}
export async function verifyNativeInputs({root, source, baseline}) {
  if (!/^[0-9a-f]{40}$/.test(source)) throw Error('invalid-source');
  const rootStat=await lstat(root);
  if(!rootStat.isDirectory() || rootStat.isSymbolicLink()) throw Error('invalid-native-root');
  const manifestBytes=await readFile(path.join(root,'native-build.json'));
  if(manifestBytes.length>65536) throw Error('native-manifest-too-large');
  const manifest = JSON.parse(manifestBytes.toString('utf8'));
  if (manifest.schema !== 1 || manifest.source_revision !== source || !Array.isArray(manifest.files)) throw Error('invalid-native-manifest');
  const names = new Set(nativeNames);
  const actual = await readdir(root, {withFileTypes:true});
  if (actual.length !== names.size + 1 || actual.some(x => !x.isFile() || (!names.has(x.name) && x.name !== 'native-build.json'))) throw Error('native-file-set-mismatch');
  if (manifest.files.length !== names.size) throw Error('native-manifest-file-set-mismatch');
  const accepted = new Map();
  for (const item of manifest.files) {
    if (!item || Object.keys(item).sort().join(',')!=='bytes,path,sha256' || !names.has(item.path) || accepted.has(item.path) || !Number.isSafeInteger(item.bytes) || item.bytes < 1 || !/^[a-f0-9]{64}$/.test(item.sha256)) throw Error('invalid-native-file');
    const full = path.join(root,item.path);
    const stat = await lstat(full);
    if (!stat.isFile() || stat.isSymbolicLink() || stat.nlink !== 1 || stat.size !== item.bytes || await digestFile(full) !== item.sha256) throw Error('native-file-integrity');
    accepted.set(item.path, item);
  }
  for (const name of dlls) if (!baseline.some(x => x.path === `app/${name}` && (x.source === 'cpp' || x.source === 'crt'))) throw Error('native-baseline-role-mismatch');
  for (const name of crtDlls) {
    const pinned=baseline.find(x=>x.path===`app/${name}` && x.source==='crt'), actual=accepted.get(name);
    if (!pinned || pinned.bytes!==actual.bytes || pinned.sha256!==actual.sha256) throw Error('crt-baseline-mismatch');
  }
  return accepted;
}
