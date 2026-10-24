// Development contract only: POST {transcript, glossary} -> {text}.
// A company-specific adapter must be implemented after its contract is available.
export function createTextAdapter({ endpoint, timeoutMs = 15000 }) {
  const url = new URL(endpoint);
  if (url.protocol !== 'https:' && !(url.protocol === 'http:' && ['127.0.0.1', '[::1]', 'localhost'].includes(url.hostname))) {
    throw new Error('HTTPS required outside loopback');
  }
  if (!Number.isFinite(timeoutMs) || timeoutMs <= 0) throw new Error('Invalid timeout');
  return async ({ transcript, glossary = [], signal }) => {
    if (typeof transcript !== 'string' || !transcript.trim() || !Array.isArray(glossary) || glossary.some(term => typeof term !== 'string')) {
      throw new Error('Invalid text request');
    }
    const timeout = AbortSignal.timeout(timeoutMs);
    try {
      const response = await fetch(url, {
        method: 'POST', redirect: 'error',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ transcript, glossary }),
        signal: signal ? AbortSignal.any([signal, timeout]) : timeout,
      });
      if (!response.ok) throw new Error('HTTP failure');
      const body = await response.json();
      if (typeof body.text !== 'string' || !body.text.trim()) throw new Error('Invalid response');
      return body.text;
    } catch {
      const error = new Error('Text processing failed');
      error.code = signal?.aborted ? 'cancelled' : timeout.aborted ? 'llm-timeout' : 'llm-failed';
      throw error;
    }
  };
}
