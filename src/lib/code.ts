import wordlistRaw from './wordlist.txt?raw';

// Session links: 2-4 capitalised words joined with dashes (e.g. Caesar-Pyramid-Orchid).
// Shared test vectors: ./code_vectors.json.
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
  if (parts.length < 2 || parts.length > 4 || !parts.every(isWord)) return null;
  return parts.join('-').toLowerCase();
}

const WORDS: string[] = [...new Set(wordlistRaw.split('\n').map((w) => w.trim()).filter((w) => /^[a-z]{4,}$/.test(w)))].sort();

function randInt(n: number): number {
  const a = new Uint32Array(1);
  const limit = Math.floor(2 ** 32 / n) * n; // rejection sampling: no modulo bias
  do crypto.getRandomValues(a); while (a[0]! >= limit);
  return a[0]! % n;
}

/** 2-4 capitalised words joined with dashes, e.g. Caesar-Pyramid-Orchid. */
export function generateCode(nWords = 3): string {
  const n = Math.min(4, Math.max(2, nWords));
  return Array.from({ length: n }, () => {
    const w = WORDS[randInt(WORDS.length)]!;
    return w[0]!.toUpperCase() + w.slice(1);
  }).join('-');
}

