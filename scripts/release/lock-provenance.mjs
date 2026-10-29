import { createHash } from 'node:crypto';
import { readFile, lstat } from 'node:fs/promises';
import { spawnSync } from 'node:child_process';
import path from 'node:path';

export const lockPins=Object.freeze({
  cargo:'31310c3e28219400de1aae68ea041708e891130c344754685c5c7aa8073fadfb',
  bun:'1f7cda0b297dba77dbca24ab8a4c48e8774f7fa8849798fd0455f58ed3f602f2',
});
const paths=Object.freeze({cargo:'desktop/src-tauri/Cargo.lock',bun:'desktop/bun.lock'});
const hash=bytes=>createHash('sha256').update(bytes).digest('hex');

function git(cwd,args,encoding) {
  const result=spawnSync(process.platform==='win32'?'git.exe':'git',args,{cwd,encoding,windowsHide:true,maxBuffer:16*1024*1024});
  if(result.status!==0) throw Error('lock-git-unavailable');
  return result.stdout;
}

function textBytes(bytes,name) {
  let text;
  try { text=new TextDecoder('utf-8',{fatal:true,ignoreBOM:true}).decode(bytes); }
  catch { throw Error(`lock-binary-or-invalid-utf8:${name}`); }
  if (text.includes('\0') || /\r(?!\n)/.test(text)) throw Error(`lock-binary-or-invalid-utf8:${name}`);
  return Buffer.from(text.replace(/\r\n/g,'\n'),'utf8');
}

/** SHA-256 of the exact source-commit Git blob after verifying the checkout. */
export async function canonicalLockDigest({cwd,source,name,file}) {
  const relative=paths[name];
  if(!relative || !/^[0-9a-f]{40}$/.test(source ?? '')) throw Error('lock-input-invalid');
  if(git(cwd,['rev-parse','HEAD'],'utf8').trim()!==source) throw Error(`lock-source-invalid:${name}`);
  const expectedPath=path.resolve(cwd,relative);
  const compare=value=>process.platform==='win32'?value.toLowerCase():value;
  if(file && compare(path.resolve(file))!==compare(expectedPath)) throw Error(`lock-path-invalid:${name}`);
  const info=await lstat(expectedPath);
  if(!info.isFile() || info.isSymbolicLink() || info.size<1 || info.size>16*1024*1024) throw Error(`lock-file-invalid:${name}`);
  const attrs=git(cwd,['check-attr','filter','working-tree-encoding','text','--',relative],'utf8').trim().split(/\r?\n/);
  if(attrs.length!==3) throw Error(`lock-attributes-invalid:${name}`);
  for(const line of attrs) {
    const match=/: (filter|working-tree-encoding|text): (.*)$/.exec(line);
    if(!match || (match[1]==='text' ? match[2]==='unset' : !['unspecified','unset'].includes(match[2])))
      throw Error(`lock-attributes-invalid:${name}`);
  }
  const committed=git(cwd,['show',`${source}:${relative}`]);
  const worktree=await readFile(expectedPath);
  const normalized=textBytes(worktree,name), committedText=textBytes(committed,name);
  if(!committedText.equals(committed)) throw Error(`lock-blob-not-lf:${name}`);
  const expected=hash(committed), actual=hash(normalized);
  if(expected!==actual) throw Error(`lock-content-changed:${name}:expected=${expected}:actual=${actual}`);
  const expectedOid=git(cwd,['rev-parse',`${source}:${relative}`],'utf8').trim();
  const worktreeOid=git(cwd,['hash-object',`--path=${relative}`,'--',relative],'utf8').trim();
  if(!/^[0-9a-f]{40}$/.test(expectedOid) || expectedOid!==worktreeOid)
    throw Error(`lock-git-normalization-mismatch:${name}:expected=${expected}:actual=${actual}`);
  return expected;
}
