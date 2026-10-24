import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFile, mkdtemp, rm } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { createServer } from 'node:http';
import { tmpdir } from 'node:os';
import { join, dirname } from 'node:path';
import { chromium } from 'playwright';
import { History } from '../../../src/history.mjs';
import { Sessions } from '../../../src/sessions.mjs';
import { createTextAdapter } from '../../../src/text-adapter.mjs';
import { createWhisperTranscriber } from '../../../src/whisper-transcriber.mjs';

test('固定非機密 WAV → CPU STT → HTTP fixture → SQLite → Edge 測試文字框', { timeout: 180000 }, async t => {
  const executable = process.env.NOTYPE_WHISPER_EXE;
  const model = process.env.NOTYPE_WHISPER_MODEL;
  const audio = process.env.NOTYPE_SMOKE_WAV;
  assert.ok(executable && model && audio, 'Set NOTYPE_WHISPER_EXE, NOTYPE_WHISPER_MODEL, NOTYPE_SMOKE_WAV; missing runtime is a failure, not a skip');
  const hash = bytes => createHash('sha256').update(bytes).digest('hex');
  const manifest = JSON.parse(await readFile(new URL('../../../docs/verification/local-stt-manifest.json', import.meta.url), 'utf8'));
  for (const file of manifest.files) {
    assert.equal(hash(await readFile(join(dirname(executable), file.name))), file.sha256, `Runtime file ${file.name}`);
  }
  assert.equal(hash(await readFile(executable)), '111bd344b7bf0356818f2795a525cb5240ed0a99028fa9f3f1c68b4ff5b17b91');
  assert.equal(hash(await readFile(model)), '921e4cf8686fdd993dcd081a5da5b6c365bfde1162e72b08d75ac75289920b1f');
  const wav = await readFile(audio);
  assert.equal(hash(wav), 'e39a4a9e0b86c5187e586aeb5f0ae9b7eebf6d75508d02955560818040b1ca00');
  const start = performance.now();
  const transcript = await createWhisperTranscriber({ executable, model, language: 'en', threads: 4 })(wav);
  const sttMs = performance.now() - start;
  assert.match(transcript, /(?:forty.two|42)/i);
  assert.match(transcript, /not delete/i);

  const expected = 'Please keep the number 42 and do not delete the API.';
  let request;
  const server = createServer(async (req, res) => {
    let body = '';
    for await (const chunk of req) body += chunk;
    request = JSON.parse(body);
    res.setHeader('content-type', 'application/json');
    res.end(JSON.stringify({ text: expected }));
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  t.after(() => { server.closeAllConnections(); return new Promise(resolve => server.close(resolve)); });
  const dir = await mkdtemp(join(tmpdir(), 'notype-runtime-'));
  let history;
  t.after(async () => { history?.close(); await rm(dir, { recursive: true, force: true }); });
  const database = join(dir, 'history.db');
  history = new History(database);
  const polish = createTextAdapter({ endpoint: `http://127.0.0.1:${server.address().port}/polish` });
  let sessions = new Sessions({ history, polish });
  const entry = await sessions.process(transcript, ['API']);
  assert.deepEqual(request, { transcript, glossary: ['API'] });
  assert.equal(entry.status, 'pending');
  history.close();
  history = new History(database);
  assert.equal(history.get(entry.id).result, expected);
  sessions = new Sessions({ history, polish });

  const browser = await chromium.launch({ channel: 'msedge', headless: true });
  t.after(() => browser.close());
  const page = await browser.newPage();
  await page.setContent('<label>Original target<textarea id="original"></textarea></label><label>Other<textarea id="other"></textarea></label>');
  await page.locator('#original').focus();
  await page.evaluate(() => { window.enterCount = 0; document.addEventListener('keydown', e => { if (e.key === 'Enter') window.enterCount++; }); });
  // In-page focus check and insertion share one JS turn. This does not prove
  // the native Windows foreground/IME/clipboard boundary for arbitrary apps.
  const insert = text => page.evaluate(text => {
    const target = document.getElementById('original');
    if (document.activeElement !== target) return false;
    target.setRangeText(text, target.selectionStart, target.selectionEnd, 'end');
    return true;
  }, text);
  await page.locator('#other').focus();
  assert.equal((await sessions.deliver(entry.id, insert)).status, 'pending');
  assert.equal(await page.locator('#other').inputValue(), '');
  await page.locator('#original').focus();
  assert.equal((await sessions.deliver(entry.id, insert)).status, 'delivered');
  await sessions.deliver(entry.id, insert);
  assert.equal(await page.locator('#original').inputValue(), expected);
  assert.equal(await page.evaluate(() => window.enterCount), 0);
  t.diagnostic(JSON.stringify({ sttMs: Math.round(sttMs), transcript, browser: browser.version(), samples: 1, companyApi: 'not-performed', nativePaste: 'not-performed' }));
});
