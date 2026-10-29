import test from 'node:test';
import assert from 'node:assert/strict';
import { isolatedEnvironment } from '../../scripts/release/smoke-native.mjs';

test('native smoke environment excludes credentials and uses only owned directories',()=>{
  const env=isolatedEnvironment({dataRoot:'C:/runner/notype-smoke', systemRoot:'C:/Windows', separator:';'});
  assert.equal(env.NOTYPE_DATA_DIR,'C:/runner/notype-smoke');
  assert.equal(env.USERPROFILE,'C:/runner/notype-smoke/profile');
  assert.equal(env.APPDATA,'C:/runner/notype-smoke/profile/AppData/Roaming');
  assert.equal(env.LOCALAPPDATA,'C:/runner/notype-smoke/profile/AppData/Local');
  assert.equal(env.HOME,undefined);
  assert.equal(Object.hasOwn(env,'HOME'),false);
  assert.equal(env.GH_TOKEN,undefined); assert.equal(env.DEEPSEEK_API_KEY,undefined);
  assert.ok(!env.PATH.includes('notype-study'));
});
