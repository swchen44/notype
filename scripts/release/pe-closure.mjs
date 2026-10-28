import { execFileSync } from 'node:child_process';
import path from 'node:path';
const system = new Set('advapi32 bcrypt bcryptprimitives comctl32 crypt32 d3d12 dbghelp dwmapi dxgi gdi32 imm32 kernel32 ntdll ole32 oleaut32 secur32 setupapi shell32 shlwapi user32 ws2_32 wtsapi32'.split(' ').map(x=>`${x}.dll`));
for (const name of 'appmodel-runtime core-console core-datetime core-debug core-errorhandling core-fibers core-file core-handle core-heap core-interlocked core-localization core-memory core-path core-processenvironment core-profile core-rtlsupport core-string core-sysinfo core-timezone core-util crt-convert crt-environment crt-filesystem crt-heap crt-locale crt-math crt-runtime crt-stdio crt-string crt-time crt-utility eventing-provider'.split(' ')) system.add(`api-ms-win-${name}-l1-1-0.dll`);
for (const name of ['core-libraryloader-l1-2-0','core-libraryloader-l1-2-1','core-localization-l1-2-0','core-processthreads-l1-1-0','core-processthreads-l1-1-1','core-synch-l1-1-0','core-synch-l1-2-0']) system.add(`api-ms-win-${name}.dll`);
export function parseImports(text) {
  const imports = new Map(); let current;
  for (const line of text.split(/\r?\n/)) {
    if (/^\s*Summary\s*$/i.test(line)) { current = undefined; continue; }
    const dll = /^\s+([a-z0-9_.-]+\.dll)\s*$/i.exec(line);
    if (dll) { current=dll[1].toLowerCase(); if (!imports.has(current)) imports.set(current,[]); continue; }
    if (!current) continue;
    const symbol = /^\s+[0-9A-F]+\s+(\S+)\s*$/i.exec(line);
    if (symbol && !/^(ordinal|summary)$/i.test(symbol[1])) imports.get(current).push(symbol[1]);
    if (/\bordinal\s+[0-9]+\s*$/i.test(line)) throw Error('unsupported-ordinal-import');
  }
  if (!imports.size || [...imports.values()].some(x=>!x.length)) throw Error('unparseable-pe-imports');
  return imports;
}
export function parseExports(text) {
  const out = new Map();
  for (const line of text.split(/\r?\n/)) {
    const m = /^\s*\d+\s+[0-9a-f]+\s+[0-9a-f]+\s+(\S+)(.*)$/i.exec(line);
    if (m) out.set(m[1], !/forwarded to/i.test(m[2]));
  }
  return out;
}
export function validateClosure(imports, localExports) {
  for (const [dll,symbols] of imports) {
    if (system.has(dll)) continue;
    const provided=localExports.get(dll);
    if (!provided) throw Error(`unresolved-dll:${dll}`);
    for (const name of symbols) if (provided.get(name) !== true) throw Error(`unresolved-export:${dll}`);
  }
}
export function verifyPEClosure(dumpbin, dir, names) {
  if (!path.isAbsolute(dumpbin)) throw Error('dumpbin-path-invalid');
  const run=(mode,file)=>execFileSync(dumpbin,[mode,path.join(dir,file)],{encoding:'utf8',windowsHide:true,timeout:120000,maxBuffer:16*1024*1024});
  const local = new Map();
  for (const name of names.filter(x=>x.toLowerCase().endsWith('.dll'))) {
    const exports = parseExports(run('/exports',name));
    if (!exports.size) throw Error(`missing-pe-exports:${name}`);
    local.set(name.toLowerCase(),exports);
  }
  for (const name of names) validateClosure(parseImports(run('/imports',name)),local);
}
