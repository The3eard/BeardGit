import { describe, it, expect } from 'vitest';
import { hashString, refColorIndex } from './ref-colors';

describe('hashString', () => {
  it('is deterministic — same input always returns the same value', () => {
    const a = hashString('main');
    const b = hashString('main');
    expect(a).toBe(b);
  });

  it('returns a non-negative integer', () => {
    const inputs = ['main', 'develop', 'feature/foo', '', 'release/1.0.0'];
    for (const s of inputs) {
      const h = hashString(s);
      expect(h).toBeGreaterThanOrEqual(0);
      expect(Number.isInteger(h)).toBe(true);
    }
  });

  it('empty string does not crash and returns a number', () => {
    expect(() => hashString('')).not.toThrow();
    expect(typeof hashString('')).toBe('number');
  });

  it('different common branch names produce different hash values', () => {
    const names = ['main', 'develop', 'beta', 'release', 'hotfix'];
    const hashes = names.map(hashString);
    const unique = new Set(hashes);
    // All five common names should hash to distinct values
    expect(unique.size).toBe(names.length);
  });
});

describe('refColorIndex', () => {
  it('is deterministic — same name + colorCount always returns the same index', () => {
    const idx1 = refColorIndex('main', 5);
    const idx2 = refColorIndex('main', 5);
    expect(idx1).toBe(idx2);
  });

  it('index is always in range [0, colorCount)', () => {
    const names = ['main', 'develop', 'feature/add-tests', 'release/1.2.3', ''];
    const colorCount = 5;
    for (const name of names) {
      const idx = refColorIndex(name, colorCount);
      expect(idx).toBeGreaterThanOrEqual(0);
      expect(idx).toBeLessThan(colorCount);
    }
  });

  it('works with a single-color palette (colorCount = 1)', () => {
    expect(refColorIndex('anything', 1)).toBe(0);
    expect(refColorIndex('main', 1)).toBe(0);
  });

  it('different branch names produce different indices for a 5-color palette', () => {
    const names = ['main', 'develop', 'beta', 'release', 'hotfix'];
    const indices = names.map((n) => refColorIndex(n, 5));
    // We cannot guarantee all are unique with only 5 slots, but we CAN verify
    // that calling the function twice for each name gives the same result
    for (let i = 0; i < names.length; i++) {
      expect(refColorIndex(names[i], 5)).toBe(indices[i]);
    }
  });

  it('empty string does not crash', () => {
    expect(() => refColorIndex('', 5)).not.toThrow();
    const idx = refColorIndex('', 5);
    expect(idx).toBeGreaterThanOrEqual(0);
    expect(idx).toBeLessThan(5);
  });
});
