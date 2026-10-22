import { describe, expect, it } from 'vitest';
import vectors from '../../../crates/proto/fixtures/code_vectors.json';
import { extractCode, normalizeCode } from './code';

describe('session codes (shared vectors with Rust)', () => {
  it.each(vectors.valid)('accepts %s', (c) => expect(normalizeCode(c)).not.toBeNull());
  it.each(vectors.invalid)('rejects %j', (c) => expect(normalizeCode(c)).toBeNull());
  it('is case-insensitive', () => expect(normalizeCode('CAESAR-23-pyramid')).toBe(normalizeCode('caesar-23-Pyramid')));
  it('extracts a code from a link', () => {
    expect(extractCode('https://agostaxyz.github.io/sharu/Caesar-23-Pyramid')).toBe('Caesar-23-Pyramid');
    expect(extractCode('  Caesar-23-Pyramid/ ')).toBe('Caesar-23-Pyramid');
  });
});
