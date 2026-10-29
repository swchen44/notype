import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, mkdir, writeFile, readFile, rm, cp } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createHash } from 'node:crypto';
import { publishDraft, runGh } from '../../scripts/release/release-controller.mjs';

const source = 'a'.repeat(40);
const version = '0.1.2-test.1';
const tag = `v${version}`;
const hash = b => createHash('sha256').update(b).digest('hex');

test('GitHub client requests are bounded and errors never expose stderr', async () => {
  await assert.rejects(runGh(['-e','setInterval(()=>{},1000)'], {executable:process.execPath,timeoutMs:50}), /github-timeout/);
  await assert.rejects(runGh(['-e','console.error("secret-token");process.exit(7)'], {executable:process.execPath,timeoutMs:5000}), error => {
    assert.match(error.message, /github-request-failed:7/);
    assert.doesNotMatch(error.message, /secret-token/);
    return true;
  });
  await assert.rejects(runGh(['-e','console.error("secret-token HTTP 403");process.exit(1)'], {executable:process.execPath,timeoutMs:5000}), error => {
    assert.match(error.message, /github-http-403/); assert.doesNotMatch(error.message, /secret-token/); return true;
  });
});
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
    async tagExists(){return false},
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

test('download layout must place verified assets at the controller root before any draft write',async t=>{
  const original=await fixture(t);const root=await mkdtemp(join(tmpdir(),'notype-artifact-layout-'));t.after(()=>rm(root,{recursive:true,force:true}));
  // Default ID-based download nests the artifact by name and cannot be published.
  await cp(original,join(root,'notype-offline-fixture'),{recursive:true});const nested=fakeGitHub();
  await assert.rejects(publishDraft({input,artifactRoot:root,github:nested}),{code:'ENOENT'});assert.deepEqual(nested.state.writes,[]);
  // merge-multiple extracts the same ID-selected files directly under the root.
  const merged=join(root,'merged');await cp(original,merged,{recursive:true});const gh=fakeGitHub();
  const receipt=await publishDraft({input,artifactRoot:merged,github:gh});
  assert.equal(receipt.assets.length,4);assert.equal(gh.state.writes.at(-1),'finalize');
});

test('a tag created after draft creation remains visibly incomplete',async t=>{
  const root=await fixture(t); const gh=fakeGitHub(); gh.tagExists=async()=>true;
  await assert.rejects(publishDraft({input,artifactRoot:root,github:gh}),/tag-conflict/);
  assert.match(gh.state.draft.body,/INCOMPLETE/); assert.deepEqual(gh.state.writes,['create']);
});

test('a tag appearing after finalization returns the draft to incomplete',async t=>{
  const root=await fixture(t); const gh=fakeGitHub(); let calls=0;
  gh.tagExists=async()=>++calls===3;
  await assert.rejects(publishDraft({input,artifactRoot:root,github:gh}),/tag-conflict/);
  assert.match(gh.state.draft.body,/INCOMPLETE/);
});

test('unavailable tag verification after finalization restores incomplete state',async t=>{
  const root=await fixture(t); const gh=fakeGitHub(); let calls=0;
  gh.tagExists=async()=>{if(++calls===3) throw Error('network');return false;};
  await assert.rejects(publishDraft({input,artifactRoot:root,github:gh}),/network/);
  assert.match(gh.state.draft.body,/INCOMPLETE/);
});

test('a failed rollback reports that manual draft inspection is required',async t=>{
  const root=await fixture(t); const gh=fakeGitHub(); let reads=0, writes=0;
  gh.tagExists=async()=>{if(++reads===3) throw Error('network');return false;};
  const finalize=gh.finalize;gh.finalize=async(...args)=>{if(++writes===2) throw Error('network');return finalize(...args)};
  await assert.rejects(publishDraft({input,artifactRoot:root,github:gh}),/draft-rollback-failed-manual-review-required/);
  assert.match(gh.state.draft.body,/人工發布前/);
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
