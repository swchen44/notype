export class Sessions {
  constructor({ history, polish }) {
    this.history = history;
    this.polish = polish;
    this.busy = new Set();
  }
  async process(transcript, glossary = [], signal) {
    const entry = this.history.create(transcript);
    return this.#polish(entry.id, glossary, signal);
  }
  async retry(id, glossary = [], signal) {
    const entry = this.history.get(id);
    if (this.busy.has(id) || !['failed', 'cancelled', 'pending'].includes(entry.status)) return entry;
    this.history.update(id, { status: 'processing', result: null, error: null });
    return this.#polish(id, glossary, signal);
  }
  async #polish(id, glossary, signal) {
    this.busy.add(id);
    try {
      signal?.throwIfAborted();
      const result = await this.polish({ transcript: this.history.get(id).transcript, glossary, signal });
      signal?.throwIfAborted();
      if (typeof result !== 'string' || !result.trim()) throw new Error('Empty result');
      return this.history.update(id, { result, status: 'pending', error: null });
    } catch (error) {
      return this.history.update(id, {
        status: signal?.aborted ? 'cancelled' : 'failed',
        error: signal?.aborted ? 'cancelled' : error.code === 'llm-timeout' ? 'llm-timeout' : 'llm-failed',
      });
    } finally { this.busy.delete(id); }
  }
  // The native/UI boundary must verify the ORIGINAL target and insert atomically.
  // false means no insertion (unknown/changed target); exceptions mean uncertain delivery.
  // No fallback to current focus and no Enter key are generated here.
  async deliver(id, insertIntoOriginalTarget) {
    const entry = this.history.get(id);
    if (this.busy.has(id) || entry.status !== 'pending') return entry;
    this.busy.add(id);
    this.history.update(id, { status: 'delivering' });
    try {
      const inserted = await insertIntoOriginalTarget(entry.result);
      if (inserted !== true && inserted !== false) throw new Error('Ambiguous delivery');
      return this.history.update(id, { status: inserted ? 'delivered' : 'pending' });
    } catch {
      return this.history.update(id, { status: 'delivery-unknown', error: 'delivery-unknown' });
    } finally { this.busy.delete(id); }
  }
}
