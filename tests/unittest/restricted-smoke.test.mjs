import test from 'node:test';
import assert from 'node:assert/strict';
import path from 'node:path';
import { runRestrictedSmoke } from '../../scripts/release/smoke-native.mjs';
test('restricted launcher rejects all commands except fixed verified bundle request',()=>{
 const base=path.resolve('owned'); const env={NOTYPE_DATA_DIR:path.join(base,'notype-native-smoke')};
 const exe=path.join(base,'notype-assembled','extracted','app','node_modules','notype-win32-x64','app','notype.exe');
 assert.throws(()=>runRestrictedSmoke({executable:exe,args:['--anything'],env,timeoutMs:120000}));
 assert.throws(()=>runRestrictedSmoke({executable:process.execPath,args:['--list-models'],env,timeoutMs:120000}));
 assert.throws(()=>runRestrictedSmoke({executable:exe,args:['--list-models'],env,timeoutMs:1}));
});
