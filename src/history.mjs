import { DatabaseSync } from 'node:sqlite';
import { mkdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { randomUUID } from 'node:crypto';

export function defaultHistoryPath() {
  if (!process.env.LOCALAPPDATA) throw new Error('LOCALAPPDATA is required');
  return join(process.env.LOCALAPPDATA, 'notype', 'history.db');
}

// One owning process per database. Audio is never stored in this database.
export class History {
  constructor(path = defaultHistoryPath()) {
    if (path !== ':memory:') mkdirSync(dirname(path), { recursive: true });
    this.db = new DatabaseSync(path);
    this.db.exec(`PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;`);
    const version = this.db.prepare('PRAGMA user_version').get().user_version;
    if (version > 1) { this.db.close(); throw new Error('Unsupported history version'); }
    this.db.exec(`BEGIN;
      CREATE TABLE IF NOT EXISTS sessions (
        id TEXT PRIMARY KEY, transcript TEXT NOT NULL, result TEXT,
        created_at TEXT NOT NULL, updated_at TEXT NOT NULL,
        status TEXT NOT NULL, error TEXT
      );
      PRAGMA user_version=1;
      UPDATE sessions SET status='failed', error='interrupted' WHERE status='processing';
      UPDATE sessions SET status='delivery-unknown', error='interrupted' WHERE status='delivering';
      COMMIT;`);
  }
  create(transcript) {
    if (typeof transcript !== 'string' || !transcript.trim()) throw new Error('Empty transcript');
    const id = randomUUID();
    const now = new Date().toISOString();
    this.db.prepare('INSERT INTO sessions VALUES (?, ?, NULL, ?, ?, ?, NULL)')
      .run(id, transcript, now, now, 'processing');
    return this.get(id);
  }
  get(id) {
    const row = this.db.prepare('SELECT * FROM sessions WHERE id=?').get(id);
    if (!row) throw new Error('History entry not found');
    return { ...row };
  }
  update(id, patch) {
    const entry = { ...this.get(id), ...patch };
    this.db.prepare('UPDATE sessions SET result=?, status=?, error=?, updated_at=? WHERE id=?')
      .run(entry.result, entry.status, entry.error, new Date().toISOString(), id);
    return this.get(id);
  }
  search(query) {
    return this.db.prepare(`SELECT * FROM sessions WHERE instr(transcript, ?) > 0
      OR instr(coalesce(result, ''), ?) > 0 ORDER BY created_at DESC`).all(query, query).map(row => ({ ...row }));
  }
  delete(id) { this.db.prepare('DELETE FROM sessions WHERE id=?').run(id); }
  close() { this.db.close(); }
}
