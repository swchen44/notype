import { test } from 'node:test';
import assert from 'node:assert/strict';
import { validateNativeInputs } from '../../scripts/release/native-stage.mjs';

const source = 'a'.repeat(40);
const tools = { node: '22.15.0', bun: '1.2.22', rust: '1.99.1' };

test('native build accepts only exact source, pinned tools and controlled runtime inputs', () => {
  assert.doesNotThrow(() => validateNativeInputs({ source, actualSource: source, clean: true, tools, env: {} }));
  for (const change of [
    { source: 'HEAD' },
    { actualSource: 'b'.repeat(40) },
    { clean: false },
    { tools: { ...tools, rust: '1.98.0' } },
    { env: { TRANSCRIBE_DIR: 'foreign-prefix' } },
    { env: { ORT_PREFER_DYNAMIC_LINK: '1' } },
    { env: { ORT_PREFER_DYNAMIC_LINK: '' } },
    { env: { ORT_LIB_LOCATION: 'foreign-ort' } },
    { env: { ORT_LIB_PATH: 'foreign-ort' } },
    { env: { ORT_OFFLINE: '1' } },
    { env: { HANDY_VC_REDIST_DIRS: 'other-vs' } },
    { env: { RUSTFLAGS: '-C target-cpu=native' } },
  ]) assert.throws(() => validateNativeInputs({ source, actualSource: source, clean: true, tools, env: {}, ...change }));
});
