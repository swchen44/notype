/** History's primary text and search share the same result-first projection. */
export function historyText(entry: {
  result: string | null;
  transcript: string;
}) {
  return entry.result || entry.transcript;
}

/** Literal substring matching: user search text is never a regular expression. */
export function historyMatches(text: string, query: string) {
  return text.toLowerCase().includes(query.trim().toLowerCase());
}

export function historySegments(text: string, query: string) {
  const needle = query.trim().toLowerCase();
  if (!needle) return [{ text, match: false }];
  // Unicode lowercasing can expand a character (İ → i + combining dot).
  // Map folded UTF-16 positions back to complete original characters.
  const positions: { start: number; end: number }[] = [];
  let originalOffset = 0;
  for (const character of text) {
    const end = originalOffset + character.length;
    for (let i = 0; i < character.toLowerCase().length; i++)
      positions.push({ start: originalOffset, end });
    originalOffset = end;
  }
  const lower = text.toLowerCase();
  const parts: { text: string; match: boolean }[] = [];
  let offset = 0;
  for (
    let index = lower.indexOf(needle);
    index !== -1;
    index = lower.indexOf(needle, index + needle.length)
  ) {
    const start = Math.max(offset, positions[index].start);
    const end = positions[index + needle.length - 1].end;
    if (start > offset)
      parts.push({ text: text.slice(offset, start), match: false });
    if (end > start) parts.push({ text: text.slice(start, end), match: true });
    offset = end;
  }
  if (offset < text.length)
    parts.push({ text: text.slice(offset), match: false });
  return parts;
}
