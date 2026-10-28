import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, mkdir, writeFile, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createHash } from 'node:crypto';
import { publishDraft } from '../../scripts/release/release-controller.mjs';

const source = 'a'.repeat(40);
const version = '0.1.2-test.1';
const tag = `v${version}`;
const hash = b => createHash('sha256').update(b).digest('hex');
async function fixture(t) {
  const root = await mkdtemp(join(tmpdir(), 'notype-release-'));
  t.after(() => rm(root, {recursive:true, force:true}));
  const assets = join(root, 'assets'); await mkdir(assets);
  const entries = new Map([[`notype-${version}-offline-win-x64.zip`,Buffer.from('zip fixture')],['Windows11-README.md',Buffer.from('readme')],['VERIFICATION.md',Buffer.from('verification')]]);
  for (const [name, bytes] of entries) await writeFile(join(assets,name), bytes);
  await writeFile(join(assets, 'SHA256SUMS.txt'), [...entries].map(([n,b]) => `${hash(b)}  ${n}\n`).join(''));
  await writeFile(join(root,'assembly-report.json'), JSON.stringify({source,version,checks:['zip-crc-sha','fresh-extraction','bundled-cli','native-headless-list-models'],archive:{file:`notype-${version}-offline-win-x64.zip`,bytes:entries.values().next().value.length,sha256:hash(entries.values().next().value)}}));
  return root;
}
function fakeGitHub(options={}) {
  const state={draft:null, writes:[], assetList:[]};
  return {state, async mainHead(){return source}, async exists(){return !!options.exists},
    async createDraft(data){state.writes.push('create');state.draft={...data,id:12,draft:true,prerelease:true}; return 12;},
    async upload(id,file){state.writes.push('upload'); if(options.failUpload) throw Error('network');state.assetList.push({...file,digest:options.badDigest?'0'.repeat(64):file.digest});},
    async getDraft(){return {...state.draft,assets:state.assetList}},
    async finalize(id,body){state.writes.push('finalize');state.draft.body=body},
  };
}
const input = {version,ref:'refs/heads/main',source,repository:'swchen44/notype'};

test('validated artifacts produce only a completed draft after the remote digests match',async t=>{
  const root=await fixture(t); const gh=fakeGitHub();
  const receipt=await publishDraft({input,artifactRoot:root,github:gh});
  assert.equal(receipt.tag,tag); assert.equal(gh.state.draft.draft,true);
  assert.equal(gh.state.draft.prerelease,true); assert.equal(gh.state.assetList.length,4);
  assert.equal(gh.state.writes.at(-1),'finalize'); assert.doesNotMatch(gh.state.draft.body,/INCOMPLETE/);
});

test('invalid local files and existing releases are rejected before a write',async t=>{
  const root=await fixture(t); const gh=fakeGitHub({exists:true});
  await assert.rejects(publishDraft({input,artifactRoot:root,github:gh})); assert.deepEqual(gh.state.writes,[]);
  await writeFile(join(root,'assets','Windows11-README.md'),'tampered');
  const gh2=fakeGitHub(); await assert.rejects(publishDraft({input,artifactRoot:root,github:gh2})); assert.deepEqual(gh2.state.writes,[]);
});

test('failed upload or remote digest leaves an explicitly incomplete draft',async t=>{
  const root=await fixture(t);
  for(const options of [{failUpload:true},{badDigest:true}]){
    const gh=fakeGitHub(options); await assert.rejects(publishDraft({input,artifactRoot:root,github:gh}));
    assert.match(gh.state.draft.body,/INCOMPLETE/); assert.ok(!gh.state.writes.includes('finalize'));
  }
});

test('source changing during publication prevents a draft being marked complete',async t=>{
  const root=await fixture(t); const gh=fakeGitHub(); let calls=0;
  gh.mainHead=async()=> (++calls >= 3 ? 'b'.repeat(40) : source);
  await assert.rejects(publishDraft({input,artifactRoot:root,github:gh}),/release-source-changed/);
  assert.match(gh.state.draft.body,/INCOMPLETE/);
  assert.ok(!gh.state.writes.includes('finalize'));
});

test('temporary missing remote digest is retried before the draft is marked complete',async t=>{
  const root=await fixture(t); const gh=fakeGitHub(); const get=gh.getDraft; let reads=0, waits=0;
  gh.getDraft=async()=>{const draft=await get(); if(++reads===1) draft.assets=draft.assets.map(a=>({...a,digest:null})); return draft;};
  await publishDraft({input,artifactRoot:root,github:gh,wait:async()=>{waits++;}});
  assert.equal(waits,1); assert.equal(gh.state.writes.at(-1),'finalize');
});

test('publishing requires a successful isolated native smoke receipt',async t=>{
  const root=await fixture(t);const gh=fakeGitHub();
  const report=JSON.parse(await readFile(join(root,'assembly-report.json'),'utf8'));
  report.checks=[]; await writeFile(join(root,'assembly-report.json'),JSON.stringify(report));
  await assert.rejects(publishDraft({input,artifactRoot:root,github:gh}));assert.deepEqual(gh.state.writes,[]);
});
