import test from 'node:test';
import assert from 'node:assert/strict';
import { validateReleaseInput } from '../../scripts/release/release-controller.mjs';
import { validateVersion } from '../../scripts/release/assembly-policy.mjs';
import { validatePin } from '../../packages/notype/lib/manifest.mjs';

test('preflight and assembly agree with the product manifest on prerelease casing',()=>{
  const source='a'.repeat(40), input={ref:'refs/heads/main',source,main:source,repository:'swchen44/notype'};
  const pin=version=>({name:'notype-win32-x64',version,manifest:'payload.json',sha256:'b'.repeat(64)});
  for (const version of ['1.2.3','1.2.3-rc.1']) {
    assert.doesNotThrow(()=>validateReleaseInput({...input,version}));
    assert.doesNotThrow(()=>validateVersion(version));
    assert.doesNotThrow(()=>validatePin(pin(version)));
  }
  for (const version of ['1.2.3-RC.1','1.2.3-rc.A']) {
    assert.throws(()=>validateReleaseInput({...input,version}));
    assert.throws(()=>validateVersion(version));
    assert.throws(()=>validatePin(pin(version)));
  }
});

test('release input is limited to a safe version on exact main source', () => {
  const source = 'a'.repeat(40);
  assert.deepEqual(validateReleaseInput({version:'0.2.0-rc.1', ref:'refs/heads/main', source, main:source, repository:'swchen44/notype'}), {version:'0.2.0-rc.1', tag:'v0.2.0-rc.1', source});
  for (const version of ['v0.2.0', '1.0.0; touch x', '../v1', '1.2', '01.2.3', '1.2.3-01', '1.2.3-rc.01', '1.2.3\n', '1.2.3+'.repeat(100)])
    assert.throws(() => validateReleaseInput({version, ref:'refs/heads/main', source, main:source, repository:'swchen44/notype'}));
  for (const version of ['1.2.3', '1.2.3-beta.1', '1.2.3-0.3', '1.2.3-alpha-01'])
    assert.equal(validateReleaseInput({version, ref:'refs/heads/main', source, main:source, repository:'swchen44/notype'}).version,version);
  for (const change of [{ref:'refs/heads/other'}, {main:'b'.repeat(40)}, {repository:'other/repo'}])
    assert.throws(() => validateReleaseInput({version:'0.2.0',ref:'refs/heads/main',source,main:source,repository:'swchen44/notype',...change}));
});

import { createGitHubAdapter } from '../../scripts/release/release-controller.mjs';
test('GitHub adapter rejects collisions and uses only literal command arguments', async () => {
  const calls=[];
  const run=(args) => {
    calls.push(args);
    if(args.includes('repos/swchen44/notype/commits/main')) return JSON.stringify({sha:'a'.repeat(40)});
    if(args.includes('repos/swchen44/notype/releases')) return JSON.stringify([[{tag_name:'v1.2.3',draft:true}]]);
    if(args.includes('repos/swchen44/notype/git/matching-refs/tags/v1.2.4')) return JSON.stringify(args.includes('--slurp')?[[{ref:'refs/tags/v1.2.4-extra'}]]:[{ref:'refs/tags/v1.2.4-extra'}]);
    if(args.includes('repos/swchen44/notype/git/matching-refs/tags/v1.2.5')) return JSON.stringify(args.includes('--slurp')?[[{ref:'refs/tags/v1.2.5'}]]:[{ref:'refs/tags/v1.2.5'}]);
    throw Error('unexpected command');
  };
  const gh=createGitHubAdapter(run);
  assert.equal(await gh.mainHead(),'a'.repeat(40));
  assert.equal(await gh.exists('v1.2.3'),true);
  assert.equal(await gh.exists('v1.2.4'),false);
  assert.equal(await gh.exists('v1.2.5'),true);
  assert.equal(await gh.tagExists('v1.2.4'),false);
  assert.equal(await gh.tagExists('v1.2.5'),true);
  assert.ok(calls.every(args=>Array.isArray(args) && args[0]==='api'));
});

// This transfer is ID-selected, so download-artifact v4 otherwise adds a name directory.
test('draft workflow downloads the one ID-selected artifact directly into the controller root',async()=>{
  const {readFile}=await import('node:fs/promises');
  const workflow=await readFile(new URL('../../.github/workflows/windows-release-draft.yml',import.meta.url),'utf8');
  const step=workflow.replace(/\r\n/g,'\n').match(/- uses: actions\/download-artifact@[^\n]+\n\s+with:\n((?:[ \t]+[^\n]+\n)+)/)?.[1];
  assert.ok(step,'download step missing');
  assert.match(step,/artifact-ids:\s*\$\{\{ needs\.build\.outputs\.artifact-id \}\}/);
  assert.match(step,/path:\s*\$\{\{ runner\.temp \}\}\/notype-assembled/);
  assert.match(step,/merge-multiple:\s*true/);
  assert.match(workflow,/RELEASE_ASSEMBLY_DIR:\s*\$\{\{ runner\.temp \}\}\/notype-assembled/);
});

test('draft PATCH explicitly preserves the draft tag when updating its body',async()=>{
  const tag='v0.1.2-test.1',source='a'.repeat(40);let draft={id:42,tag_name:tag,target_commitish:source,draft:true,prerelease:true,assets:[],body:'initial'};
  const run=async args=>{
    if(args.includes('POST'))return JSON.stringify(draft);
    if(args.includes('PATCH')){
      const tagField=args.find(x=>x.startsWith('tag_name='));
      draft={...draft,tag_name:tagField?.slice('tag_name='.length) ?? 'untagged-synthetic',body:args.find(x=>x.startsWith('body=')).slice(5)};
      return JSON.stringify(draft);
    }
    return JSON.stringify(draft);
  };
  const gh=createGitHubAdapter(run);
  const id=await gh.createDraft({tag,source,version:'0.1.2-test.1',body:'initial'});
  await gh.finalize(id,'final');assert.equal(draft.tag_name,tag);assert.equal(draft.body,'final');
});

test('ID-scoped rollback repairs the known untagged draft and rejects changed ownership',async()=>{
 const tag='v0.1.2-test.1',source='a'.repeat(40);let patches=0;
 let draft={id:42,tag_name:tag,target_commitish:source,draft:true,prerelease:true,assets:[],body:'initial'};
 const run=async args=>{
  if(args.includes('POST'))return JSON.stringify(draft);
  if(args.includes('PATCH')){patches++;draft={...draft,tag_name:args.find(x=>x.startsWith('tag_name=')).slice(9),body:args.find(x=>x.startsWith('body=')).slice(5)};return JSON.stringify(draft)}
  return JSON.stringify(draft);
 };
 const gh=createGitHubAdapter(run);const id=await gh.createDraft({tag,source,version:'0.1.2-test.1',body:'initial'});
 draft.tag_name='untagged-84a15933b0a68805a5dc';draft.body='wrong complete';
 await gh.restoreIncomplete(id,'INCOMPLETE');assert.equal(draft.tag_name,tag);assert.equal(draft.body,'INCOMPLETE');assert.equal(patches,1);
 for(const change of [{tag_name:'v9.9.9'},{tag_name:'untagged-not-safe'},{target_commitish:'b'.repeat(40)},{draft:false},{prerelease:false}]){
  draft={...draft,tag_name:tag,target_commitish:source,draft:true,prerelease:true,...change};
  await assert.rejects(gh.restoreIncomplete(id,'INCOMPLETE'));assert.equal(patches,1);
 }
});

test('finalize rejects changed source or publication before writing',async()=>{
 const tag='v0.1.2-test.1',source='a'.repeat(40);let patches=0;
 let draft={id:42,tag_name:tag,target_commitish:source,draft:true,prerelease:true,assets:[],body:'initial'};
 const run=async args=>{if(args.includes('PATCH'))patches++;return JSON.stringify(draft)};
 const gh=createGitHubAdapter(run);const id=await gh.createDraft({tag,source,version:'0.1.2-test.1',body:'initial'});
 for(const change of [{target_commitish:'b'.repeat(40)},{draft:false},{prerelease:false}]){
  draft={...draft,target_commitish:source,draft:true,prerelease:true,...change};await assert.rejects(gh.finalize(id,'final'));assert.equal(patches,0);
 }
});

test('rollback rejects another ID, non-incomplete body and a mismatching reread',async()=>{
 const tag='v0.1.2-test.1',source='a'.repeat(40);let patches=0,reads=0;
 let draft={id:42,tag_name:tag,target_commitish:source,draft:true,prerelease:true,assets:[],body:'initial'};
 const run=async args=>{
  if(args.includes('POST'))return JSON.stringify(draft);
  if(args.includes('PATCH')){patches++;draft={...draft,body:'INCOMPLETE'};return JSON.stringify(draft)}
  reads++;return JSON.stringify(reads===2?{...draft,target_commitish:'b'.repeat(40)}:draft);
 };
 const gh=createGitHubAdapter(run);await gh.createDraft({tag,source,version:'0.1.2-test.1',body:'initial'});
 await assert.rejects(gh.restoreIncomplete(43,'INCOMPLETE'));await assert.rejects(gh.restoreIncomplete(42,'ordinary text'));assert.equal(patches,0);
 await assert.rejects(gh.restoreIncomplete(42,'INCOMPLETE'),/draft-rollback-failed/);assert.equal(patches,1);
});
