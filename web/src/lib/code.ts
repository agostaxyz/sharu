import wordlistRaw from './wordlist.txt?raw';

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

export const WORDS: string[] = [...new Set(wordlistRaw.split('\n').map((w) => w.trim()).filter((w) => /^[a-z]{4,}$/.test(w)))].sort();

function randInt(n: number): number {
  const a = new Uint32Array(1);
  const limit = Math.floor(2 ** 32 / n) * n; // rejection sampling: no modulo bias
  do crypto.getRandomValues(a); while (a[0]! >= limit);
  return a[0]! % n;
}

/** Same shape as the Rust server: 2-4 capitalised words plus one two-digit number after a word. */
export function generateCode(nWords = 3): string {
  const n = Math.min(4, Math.max(2, nWords));
  const numberAfter = 1 + randInt(n);
  const parts: string[] = [];
  for (let i = 1; i <= n; i++) {
    const w = WORDS[randInt(WORDS.length)]!;
    parts.push(w[0]!.toUpperCase() + w.slice(1));
    if (i === numberAfter) parts.push(String(10 + randInt(90)));
  }
  return parts.join('-');
}

/** Suggested passphrase for a new session, e.g. Cedar-72-Mellow. */
export const generatePassphrase = (): string => generateCode(2);
