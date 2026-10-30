import { describe, expect, it } from 'vitest';
import rustWords from '../../../crates/server/src/wordlist.txt?raw';
import { WORDS, generateCode, normalizeCode } from './code';
import { defaultNickname, sanitizeNickname } from './nick';

describe('serverless helpers', () => {
  it('wordlist is identical to the Rust server list', () => expect(WORDS).toEqual([...new Set(rustWords.split('\n').map((w) => w.trim()).filter((w) => /^[a-z]{4,}$/.test(w)))].sort()));
  it('generated codes are valid, never start with a number, and vary', () => {
    const seen = new Set<string>();
    for (let i = 0; i < 300; i++) {
      const c = generateCode(3);
      expect(normalizeCode(c)).not.toBeNull();
      expect(/^\d/.test(c)).toBe(false);
      seen.add(c);
    }
    expect(seen.size).toBeGreaterThan(250);
  });
  it('default nicknames are valid two-word names without digits', () => {
    for (let i = 0; i < 100; i++) {
      const n = defaultNickname();
      expect(sanitizeNickname(n)).toBe(n);
      expect(/\d/.test(n)).toBe(false);
    }
  });
  it('rejects bad nicknames', () => {
    for (const bad of ['a', 'has space', 'evil\u202ename', '<script>', 'x'.repeat(25)]) expect(sanitizeNickname(bad)).toBeNull();
  });
});
