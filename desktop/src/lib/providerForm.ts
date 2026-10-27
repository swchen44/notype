import type { ConnectionDraft, ConnectionView } from "@/bindings";

type NumericBounds = {
  min: number;
  max: number;
  integer?: boolean;
  exclusiveMin?: boolean;
};

/** Parse user form text without turning a blank provider default into zero. */
export function optionalNumber(
  raw: string,
  bounds: NumericBounds,
): { valid: boolean; value: number | null } {
  const text = raw.trim();
  if (!text) return { valid: true, value: null };
  const value = Number(text);
  const valid =
    /^-?(?:\d+(?:\.\d*)?|\.\d+)(?:e[+-]?\d+)?$/i.test(text) &&
    Number.isFinite(value) &&
    (bounds.exclusiveMin ? value > bounds.min : value >= bounds.min) &&
    value <= bounds.max &&
    (!bounds.integer || Number.isInteger(value));
  return { valid, value: valid ? value : null };
}

/** Nonsecret preview only; backend route validation remains authoritative. */
export function endpointPreview(
  connection: Pick<ConnectionDraft, "api_address" | "address_mode">,
): string | null {
  try {
    const url = new URL(connection.api_address.trim());
    const loopback =
      url.hostname === "localhost" ||
      url.hostname === "[::1]" ||
      /^127\.(?:\d{1,3}\.){2}\d{1,3}$/.test(url.hostname);
    if (
      !(url.protocol === "https:" || (url.protocol === "http:" && loopback)) ||
      url.username ||
      url.password ||
      url.search ||
      url.hash
    )
      return null;
    if (connection.address_mode === "base_url") {
      const prefix = url.pathname.replace(/\/+$/, "");
      if (prefix.endsWith("/chat/completions")) return null;
      url.pathname = prefix + "/chat/completions";
    }
    return url.href;
  } catch {
    return null;
  }
}

/** Project only command input fields; view metadata cannot become request data. */
export function connectionDraft(view: ConnectionView): ConnectionDraft {
  return {
    id: view.id,
    display_name: view.display_name,
    kind: view.kind,
    dialect: view.dialect,
    address_mode: view.address_mode,
    api_address: view.api_address,
    models_endpoint: view.models_endpoint,
    model: view.model,
    authentication: view.authentication,
    timeout_ms: view.timeout_ms,
    thinking_preference: view.thinking_preference,
    effort: view.effort,
    capability_declaration: view.capability_declaration,
    temperature: view.temperature,
    top_p: view.top_p,
    max_output_tokens: view.max_output_tokens,
  };
}
/** UI confirmation only: short credentials are never displayed in full. */
export function maskApiKey(value: string): string {
  const chars = Array.from(value);
  return chars.length <= 8
    ? "********"
    : chars.slice(0, 4).join("") + "****" + chars.slice(-4).join("");
}
