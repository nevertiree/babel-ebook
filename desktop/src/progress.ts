/**
 * Typed representation of translation progress events emitted by the backend.
 *
 * The backend serialises Rust enums as externally-tagged objects (e.g.
 * `{ Started: { total: 5 } }`) or the string `"Completed"`. This module parses
 * those payloads into a TypeScript tagged union so the rest of the frontend can
 * use a single `switch` with exhaustiveness checking.
 */

export type ProgressPayload =
  | { type: "Started"; total: number }
  | { type: "ChapterStarted"; index: number; href: string }
  | { type: "ChapterFinished"; index: number; href: string }
  | {
      type: "ChunkStarted";
      index: number;
      href: string;
      chunk_index: number;
      chunk_total: number;
    }
  | {
      type: "ChunkFinished";
      index: number;
      href: string;
      chunk_index: number;
      chunk_total: number;
    }
  | { type: "Failed"; index: number; href: string; error: string }
  | { type: "Completed" };

/**
 * Assert that all progress payload variants are handled. If the compiler
 * reports an error here, a new event type was added and the consumers must be
 * updated.
 */
export function assertProgressPayloadExhaustive(
  _payload: never
): asserts _payload is never {
  // No runtime behaviour: this function exists only for compile-time
  // exhaustiveness checking.
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function extractSingleKey(
  value: Record<string, unknown>
): { key: string; data: unknown } | null {
  const keys = Object.keys(value);
  if (keys.length !== 1) return null;
  const key = keys[0];
  return { key, data: value[key] };
}

function isNumber(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value);
}

function isString(value: unknown): value is string {
  return typeof value === "string";
}

/**
 * Parse a backend progress payload into the tagged union format.
 *
 * Returns `null` for payloads that do not match the expected shape, allowing
 * callers to ignore malformed events safely.
 */
export function parseProgressPayload(raw: unknown): ProgressPayload | null {
  if (raw === "Completed") {
    return { type: "Completed" };
  }
  if (!isRecord(raw)) return null;
  const entry = extractSingleKey(raw);
  if (!entry) return null;
  const { key, data } = entry;
  if (!isRecord(data)) return null;

  switch (key) {
    case "Started": {
      const total = (data as { total?: unknown }).total;
      if (!isNumber(total)) return null;
      return { type: "Started", total };
    }
    case "ChapterStarted":
    case "ChapterFinished": {
      const index = (data as { index?: unknown }).index;
      const href = (data as { href?: unknown }).href;
      if (!isNumber(index) || !isString(href)) return null;
      return { type: key, index, href };
    }
    case "ChunkStarted":
    case "ChunkFinished": {
      const index = (data as { index?: unknown }).index;
      const href = (data as { href?: unknown }).href;
      const chunk_index = (data as { chunk_index?: unknown }).chunk_index;
      const chunk_total = (data as { chunk_total?: unknown }).chunk_total;
      if (
        !isNumber(index) ||
        !isString(href) ||
        !isNumber(chunk_index) ||
        !isNumber(chunk_total)
      ) {
        return null;
      }
      return { type: key, index, href, chunk_index, chunk_total };
    }
    case "Failed": {
      const index = (data as { index?: unknown }).index;
      const href = (data as { href?: unknown }).href;
      const error = (data as { error?: unknown }).error;
      if (!isNumber(index) || !isString(href) || !isString(error)) return null;
      return { type: "Failed", index, href, error };
    }
    default:
      return null;
  }
}
