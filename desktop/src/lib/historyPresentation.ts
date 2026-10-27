/** History's primary text and search share the same result-first projection. */
export function historyText(entry: { result: string | null; transcript: string }) {
  return entry.result || entry.transcript;
}

/** Literal substring matching: user search text is never a regular expression. */
export function historyMatches(text: string, query: string) {
  return text.toLowerCase().includes(query.trim().toLowerCase());
}

export function historySegments(text: string, query: string) {
  const needle = query.trim().toLowerCase();
  if (!needle) return [{ text, match: false }];
  const lower = text.toLowerCase();
  const parts: { text: string; match: boolean }[] = [];
  let offset = 0;
  for (let index = lower.indexOf(needle); index !== -1; index = lower.indexOf(needle, offset)) {
    if (index > offset) parts.push({ text: text.slice(offset, index), match: false });
    offset = index + needle.length;
    parts.push({ text: text.slice(index, offset), match: true });
  }
  if (offset < text.length) parts.push({ text: text.slice(offset), match: false });
  return parts;
}
