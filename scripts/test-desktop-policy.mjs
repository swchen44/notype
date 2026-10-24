import { spawnSync } from 'node:child_process';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const repository = resolve(fileURLToPath(new URL('..', import.meta.url)));
const temporary = mkdtempSync(join(tmpdir(), 'notype-desktop-policy-'));
const executable = join(temporary, process.platform === 'win32' ? 'policy.exe' : 'policy');
const compiler = process.env.RUSTC || (process.platform === 'win32'
  ? join(process.env.USERPROFILE, '.cargo', 'bin', 'rustc.exe')
  : 'rustc');

try {
  for (const [command, args] of [
    [compiler, ['--edition', '2021', '--test', 'tests/unittest/desktop_data_policy.rs', '-o', executable]],
    [executable, []],
  ]) {
    const result = spawnSync(command, args, { cwd: repository, stdio: 'inherit' });
    if (result.error) throw result.error;
    if (result.status !== 0) process.exitCode = result.status || 1;
    if (process.exitCode) break;
  }
} finally {
  rmSync(temporary, { recursive: true, force: true });
}
