import assert from 'node:assert/strict';
import test from 'node:test';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
const require = createRequire(new URL('../../desktop/package.json', import.meta.url));
const ts = require('typescript');
const source = await readFile(new URL('../../desktop/src/lib/historyPresentation.ts', import.meta.url), 'utf8');
const js = ts.transpileModule(source, {compilerOptions:{module:ts.ModuleKind.ESNext}}).outputText;
const { historyText, historyMatches, historySegments } = await import('data:text/javascript;base64,'+Buffer.from(js).toString('base64'));

test('search and primary history text prefer the edited result and fall back only without one', () => {
  const entry = {transcript:'原稿 secret phrase',result:'整理後的文字 OpenAI'};
  assert.equal(historyText(entry), entry.result);
  assert.equal(historyMatches(historyText(entry),'secret'),false);
  assert.equal(historyMatches(historyText(entry),'openai'),true);
  assert.equal(historyText({...entry,result:null}),entry.transcript);
  assert.equal(historyText({...entry,result:''}),entry.transcript);
});
test('literal repeated query matches are highlighted without interpreting punctuation or markup', () => {
  assert.deepEqual(historySegments('C++ 與 c++，<script>', 'c++'),[
    {text:'C++',match:true},{text:' 與 ',match:false},{text:'c++',match:true},{text:'，<script>',match:false},
  ]);
  assert.deepEqual(historySegments('中文搜尋中文','中文'),[
    {text:'中文',match:true},{text:'搜尋',match:false},{text:'中文',match:true},
  ]);
  assert.deepEqual(historySegments('不符合','不存在'),[{text:'不符合',match:false}]);
});
