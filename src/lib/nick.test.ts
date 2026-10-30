import { describe, expect, it } from 'vitest';
import { defaultNickname, sanitizeNickname } from './nick';

describe('nicknames', () => {
  it('defaults are valid two-word names without digits', () => {
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
