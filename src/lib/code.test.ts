import { describe, expect, it } from 'vitest';
import vectors from './code_vectors.json';
import { extractCode, generateCode, normalizeCode } from './code';

describe('session codes', () => {
  it.each(vectors.valid)('accepts %s', (c) => expect(normalizeCode(c)).not.toBeNull());
  it.each(vectors.invalid)('rejects %j', (c) => expect(normalizeCode(c)).toBeNull());
  it('is case-insensitive', () => expect(normalizeCode('CAESAR-23-pyramid')).toBe(normalizeCode('caesar-23-Pyramid')));
  it('extracts a code from a link', () => {
    expect(extractCode('https://agostaxyz.github.io/sharu/Caesar-23-Pyramid')).toBe('Caesar-23-Pyramid');
    expect(extractCode('  Caesar-23-Pyramid/ ')).toBe('Caesar-23-Pyramid');
  });
  it('generates valid, varied codes that never start with a number', () => {
    const seen = new Set<string>();
    for (let i = 0; i < 300; i++) {
      const c = generateCode(3);
      expect(normalizeCode(c)).not.toBeNull();
      expect(/^\d/.test(c)).toBe(false);
      seen.add(c);
    }
    expect(seen.size).toBeGreaterThan(250);
  });
});
