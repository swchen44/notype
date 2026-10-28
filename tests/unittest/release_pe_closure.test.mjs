import test from 'node:test';
import assert from 'node:assert/strict';
import {parseImports, parseExports, validateClosure} from '../../scripts/release/pe-closure.mjs';

test('a packaged DLL must export every named symbol imported by the executable', () => {
  const imports = parseImports('    transcribe.dll\n             14000 Import Address Table\n               0 time date stamp\n\n              1 speech_open\n');
  const exports = parseExports(' ordinal hint RVA name\n 1 0 00012345 speech_open\n');
  validateClosure(imports, new Map([['transcribe.dll',exports]]));
  assert.throws(() => validateClosure(imports,new Map([['transcribe.dll',new Map()]])));
  assert.throws(() => validateClosure(imports,new Map()));
});

test('unknown DLLs and forwarded exports cannot silently pass closure', () => {
  assert.throws(() => validateClosure(parseImports('  attacker.dll\n    0 bogus\n'),new Map()));
  const imp = parseImports('  example.dll\n    0 item\n');
  assert.throws(() => validateClosure(imp,new Map([['example.dll',parseExports(' 1 0 00000000 item (forwarded to OTHER.item)')]])));
});
