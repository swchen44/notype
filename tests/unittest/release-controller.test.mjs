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
