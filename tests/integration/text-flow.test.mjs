import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { History } from '../../src/history.mjs';
import { Sessions } from '../../src/sessions.mjs';
import { createTextAdapter } from '../../src/text-adapter.mjs';

async function server(t, handler) {
  const s = createServer(handler);
  await new Promise(resolve => s.listen(0, '127.0.0.1', resolve));
  t.after(() => { s.closeAllConnections(); return new Promise(resolve => s.close(resolve)); });
  return `http://127.0.0.1:${s.address().port}/polish`;
}

test('HTTP → SQLite → 重啟 → 原稿重試 → 測試輸入接點；不需要音訊', async t => {
  const dir = mkdtempSync(join(tmpdir(), 'notype-test-'));
  t.after(() => { history.close(); rmSync(dir, { recursive: true, force: true }); });
  const requests = [];
  const endpoint = await server(t, async (req, res) => {
    let body = '';
    for await (const chunk of req) body += chunk;
    requests.push(JSON.parse(body));
    res.setHeader('content-type', 'application/json');
    if (requests.length === 1) { res.writeHead(503); res.end('{}'); }
    else res.end(JSON.stringify({ text: '保留 42，不要刪除 API。' }));
  });
  const polish = createTextAdapter({ endpoint, timeoutMs: 1000 });
  const path = join(dir, 'history.db');
  let history = new History(path);
  const first = await new Sessions({ history, polish }).process('保留42不要刪除API', ['API']);
  assert.equal(first.status, 'failed');
  history.close();
  history = new History(path);
  const sessions = new Sessions({ history, polish });
  const retried = await sessions.retry(first.id, ['API']);
  assert.equal(retried.status, 'pending');
  let input = '';
  await sessions.deliver(first.id, async text => { input += text; return true; });
  assert.equal(input, '保留 42，不要刪除 API。');
  assert.equal(history.get(first.id).status, 'delivered');
  assert.deepEqual(requests, Array(2).fill({ transcript: '保留42不要刪除API', glossary: ['API'] }));
});

test('HTTP timeout 保留原稿待選', async t => {
  const endpoint = await server(t, () => {});
  const history = new History(':memory:');
  t.after(() => history.close());
  const sessions = new Sessions({ history, polish: createTextAdapter({ endpoint, timeoutMs: 50 }) });
  const entry = await sessions.process('測試 timeout');
  assert.equal(entry.status, 'failed');
  assert.equal(entry.error, 'llm-timeout');
  assert.equal(entry.transcript, '測試 timeout');
});

test('HTTP 非法 JSON 或空結果均保留原稿、不保存 response 內容', async t => {
  for (const response of ['secret invalid JSON', '{"text":""}']) {
    const endpoint = await server(t, (_req, res) => res.end(response));
    const history = new History(':memory:');
    t.after(() => history.close());
    const sessions = new Sessions({ history, polish: createTextAdapter({ endpoint }) });
    const entry = await sessions.process('原稿');
    assert.equal(entry.status, 'failed');
    assert.equal(entry.result, null);
    assert.equal(entry.error, 'llm-failed');
  }
});

test('取消進行中的 HTTP，不等待 timeout 或保存遲到結果', async t => {
  const controller = new AbortController();
  const endpoint = await server(t, () => controller.abort());
  const history = new History(':memory:');
  t.after(() => history.close());
  const sessions = new Sessions({ history, polish: createTextAdapter({ endpoint }) });
  const entry = await sessions.process('原稿', [], controller.signal);
  assert.equal(entry.status, 'cancelled');
  assert.equal(entry.result, null);
});

test('重新啟動將處理中與交付中狀態恢復為可辨識的未完成狀態', t => {
  const dir = mkdtempSync(join(tmpdir(), 'notype-recovery-'));
  t.after(() => { history.close(); rmSync(dir, { recursive: true, force: true }); });
  const path = join(dir, 'history.db');
  let history = new History(path);
  const processing = history.create('原稿');
  const delivering = history.create('另一份');
  history.update(delivering.id, { status: 'delivering', result: '結果' });
  history.close();
  history = new History(path);
  assert.equal(history.get(processing.id).status, 'failed');
  assert.equal(history.get(delivering.id).status, 'delivery-unknown');
});
