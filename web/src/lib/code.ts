// Mirrors crates/server/src/codes.rs; shared vectors in crates/proto/fixtures/code_vectors.json.
const isNumber = (p: string) => /^\d{2}$/.test(p);
const isWord = (p: string) => /^[A-Za-z]{4,16}$/.test(p);

/** Accepts a full link or a bare code; returns the last path segment. */
export function extractCode(input: string): string {
  let s = input.trim();
  try { s = new URL(s).pathname; } catch { /* not a URL */ }
  const seg = s.split('/').filter(Boolean).pop() ?? '';
  try { return decodeURIComponent(seg); } catch { return seg; }
}

/** Canonical lower-case key, or null when the shape is invalid. */
export function normalizeCode(input: string): string | null {
  const parts = input.trim().split('-');
  if (parts.length < 3 || parts.length > 5 || isNumber(parts[0]!)) return null;
  let numbers = 0;
  for (const p of parts) {
    if (isNumber(p)) numbers++;
    else if (!isWord(p)) return null;
  }
  return numbers === 1 ? parts.join('-').toLowerCase() : null;
}
