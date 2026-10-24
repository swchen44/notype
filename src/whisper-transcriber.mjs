import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { mkdtemp, writeFile, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const execute = promisify(execFile);

// Diagnostic CPU adapter. Executable/model are provisioned and hash-verified outside
// this boundary; no downloads or installer invocation occur during transcription.
export function createWhisperTranscriber({ executable, model, language = 'auto', threads = 4, run = execute }) {
  return async (wav, signal) => {
    signal?.throwIfAborted();
    if (!Buffer.isBuffer(wav) || wav.toString('ascii', 0, 4) !== 'RIFF' || wav.toString('ascii', 8, 12) !== 'WAVE') {
      throw new Error('Expected WAV audio');
    }
    const directory = await mkdtemp(join(tmpdir(), 'notype-audio-'));
    try {
      const audioPath = join(directory, 'session.wav');
      const output = join(directory, 'transcript');
      await writeFile(audioPath, wav);
      await run(executable, ['-m', model, '-f', audioPath, '-l', language,
        '-t', String(threads), '-ng', '-otxt', '-of', output], {
        windowsHide: true, signal, timeout: 120000, maxBuffer: 1024 * 1024,
      });
      signal?.throwIfAborted();
      const transcript = (await readFile(`${output}.txt`, 'utf8')).trim();
      if (!transcript) throw new Error('Empty transcription');
      return transcript;
    } catch {
      const error = new Error(signal?.aborted ? 'Transcription cancelled' : 'Transcription failed');
      error.code = signal?.aborted ? 'cancelled' : 'stt-failed';
      throw error;
    } finally {
      // directory is the exact unique absolute directory returned by mkdtemp.
      await rm(directory, { recursive: true, force: true, maxRetries: 3 });
    }
  };
}
