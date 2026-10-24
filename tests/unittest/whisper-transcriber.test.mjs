import { test } from 'node:test';
import assert from 'node:assert/strict';
import { access, writeFile } from 'node:fs/promises';
import { createWhisperTranscriber } from '../../src/whisper-transcriber.mjs';

const wav = Buffer.from('RIFF0000WAVE');

test('辨識成功後清理當次 WAV 與暫存逐字稿', async () => {
  let audio, output;
  const transcribe = createWhisperTranscriber({ executable: 'fixture', model: 'fixture', run: async (_exe, args) => {
    audio = args[args.indexOf('-f') + 1];
    output = `${args[args.indexOf('-of') + 1]}.txt`;
    await access(audio);
    await writeFile(output, ' 原稿 ');
  } });
  assert.equal(await transcribe(wav), '原稿');
  await assert.rejects(access(audio));
  await assert.rejects(access(output));
});

test('辨識失敗也清理當次 WAV，不外洩子程序輸出', async () => {
  let audio;
  const transcribe = createWhisperTranscriber({ executable: 'fixture', model: 'fixture', run: async (_exe, args) => {
    audio = args[args.indexOf('-f') + 1];
    throw new Error('private child output');
  } });
  await assert.rejects(transcribe(wav), { message: 'Transcription failed', code: 'stt-failed' });
  await assert.rejects(access(audio));
});

test('取消遲到的辨識與非法音訊不產生原稿', async () => {
  const controller = new AbortController();
  const transcribe = createWhisperTranscriber({ executable: 'fixture', model: 'fixture', run: async () => controller.abort() });
  await assert.rejects(transcribe(wav, controller.signal), { code: 'cancelled' });
  await assert.rejects(transcribe(Buffer.from('invalid')), /Expected WAV/);
});
