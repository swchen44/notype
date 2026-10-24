import { test } from 'node:test';
import assert from 'node:assert/strict';
import { History } from '../../src/history.mjs';
import { Sessions } from '../../src/sessions.mjs';

function setup(t, polish = async () => '整理結果') {
  const history = new History(':memory:');
  t.after(() => history.close());
  const sessions = new Sessions({ history, polish });
  return { history, sessions };
}

test('原稿先保存，再處理；無 WAV 仍可搜尋原稿與結果', async t => {
  const { history, sessions } = setup(t, async ({ transcript }) => {
    assert.equal(history.search(transcript)[0].status, 'processing');
    return '整理結果';
  });
  const entry = await sessions.process('不要刪除 code', ['code']);
  assert.equal(entry.status, 'pending');
  assert.equal(history.search('整理結果')[0].transcript, '不要刪除 code');
  assert.equal(history.search('不存在').length, 0);
  history.delete(entry.id);
  assert.equal(history.search('').length, 0);
});

test('LLM 失敗保留原稿且不能自動交付', async t => {
  const { sessions } = setup(t, async () => { throw Error('secret response'); });
  const entry = await sessions.process('原稿');
  assert.equal(entry.status, 'failed');
  assert.equal(entry.error, 'llm-failed');
  assert.equal(entry.transcript, '原稿');
  let calls = 0;
  await sessions.deliver(entry.id, async () => { calls++; return true; });
  assert.equal(calls, 0);
});

test('取消後即使 adapter 忽略 signal 也不保存結果或交付', async t => {
  let release;
  const gate = new Promise(resolve => { release = resolve; });
  const { sessions } = setup(t, async () => { await gate; return 'late'; });
  const controller = new AbortController();
  const pending = sessions.process('原稿', [], controller.signal);
  controller.abort();
  release();
  const entry = await pending;
  assert.equal(entry.status, 'cancelled');
  assert.equal(entry.result, null);
});

test('無法確認原目標時待選，成功交付只執行一次', async t => {
  const { sessions } = setup(t);
  const entry = await sessions.process('原稿');
  assert.equal((await sessions.deliver(entry.id, async () => false)).status, 'pending');
  let calls = 0;
  const deliver = async () => { calls++; return true; };
  await Promise.all([sessions.deliver(entry.id, deliver), sessions.deliver(entry.id, deliver)]);
  assert.equal(calls, 1);
});

test('交付例外視為結果不明，不能自動重送', async t => {
  const { sessions } = setup(t);
  const entry = await sessions.process('原稿');
  assert.equal((await sessions.deliver(entry.id, async () => { throw Error(); })).status, 'delivery-unknown');
  await sessions.deliver(entry.id, async () => assert.fail('must not retry'));
});

test('空原稿與空結果不能交付', async t => {
  const { sessions, history } = setup(t, async () => '  ');
  await assert.rejects(sessions.process('  '));
  assert.equal(history.search('').length, 0);
  assert.equal((await sessions.process('原稿')).status, 'failed');
});
