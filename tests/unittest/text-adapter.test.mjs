import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createTextAdapter } from '../../src/text-adapter.mjs';

test('非 loopback 明文 HTTP 與無效 timeout 在連線前拒絕', () => {
  assert.throws(() => createTextAdapter({ endpoint: 'http://example.com' }));
  assert.throws(() => createTextAdapter({ endpoint: 'https://example.com', timeoutMs: 0 }));
});

test('非文字 glossary 與空原稿在連線前拒絕', async () => {
  const polish = createTextAdapter({ endpoint: 'http://127.0.0.1:1' });
  await assert.rejects(polish({ transcript: 'test', glossary: [42] }), /Invalid text request/);
  await assert.rejects(polish({ transcript: '' }), /Invalid text request/);
});
