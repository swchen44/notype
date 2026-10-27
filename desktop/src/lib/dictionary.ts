export function normalizeWord(word: string) {
  return word.trim();
}
export function wordIdentity(word: string) {
  return normalizeWord(word).toLowerCase();
}
export function validWord(word: string) {
  return word.length > 0 && Array.from(word).length <= 50 && !/[\x00-\x1f\x7f]/u.test(word);
}
/** UTF-8 CSV has one explicitly named `word` column, compatible with Excel. */
export function dictionaryCsv(words: string[]) {
  return '\uFEFFword\r\n' + words.map(word => '"' + word.replace(/"/g, '""') + '"').join('\r\n') + '\r\n';
}
export function parseDictionaryCsv(csv: string): string[] {
  if (csv.length > 1024 * 1024) throw new Error('dictionary-csv-invalid');
  const rows: string[][] = [];
  let row: string[] = [], cell = '', quoted = false, closed = false;
  const source = csv.replace(/^\uFEFF/, '');
  for (let i = 0; i < source.length; i++) {
    const c = source[i];
    if (quoted) {
      if (c === '"') {
        if (source[i + 1] === '"') { cell += '"'; i++; }
        else { quoted = false; closed = true; }
      } else cell += c;
    } else if (c === '"' && !cell && !closed) quoted = true;
    else if (c === ',' || c === '\r' || c === '\n') {
      row.push(cell); cell = ''; closed = false;
      if (c !== ',') {
        rows.push(row); row = [];
        if (c === '\r' && source[i + 1] === '\n') i++;
      }
    } else {
      if (closed || c === '"') throw new Error('dictionary-csv-invalid');
      cell += c;
    }
  }
  if (quoted) throw new Error('dictionary-csv-invalid');
  if (cell || closed || row.length) { row.push(cell); rows.push(row); }
  if (rows.shift()?.join(',').toLowerCase() !== 'word') throw new Error('dictionary-csv-invalid');
  return rows.filter(r => r.some(c => c.trim())).map(r => {
    const word = normalizeWord(r[0]);
    if (r.length !== 1 || !validWord(word)) throw new Error('dictionary-csv-invalid');
    return word;
  });
}
export function mergeWords(existing: string[], incoming: string[]) {
  const words: string[] = [], seen = new Set<string>();
  let duplicates = 0;
  for (const raw of [...existing, ...incoming]) {
    const word = normalizeWord(raw), identity = wordIdentity(word);
    if (!validWord(word)) throw new Error('dictionary-word-invalid');
    if (seen.has(identity)) { duplicates++; continue; }
    seen.add(identity); words.push(word);
  }
  return { words, duplicates };
}
