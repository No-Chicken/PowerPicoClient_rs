import { describe, expect, it } from 'vitest';
import { formatBytes } from './storageFormat';

describe('formatBytes', () => {
  it('formats common storage sizes', () => {
    expect(formatBytes(0)).toBe('0 B');
    expect(formatBytes(1024)).toBe('1.00 KB');
    expect(formatBytes(253.1 * 1024 * 1024)).toBe('253 MB');
    expect(formatBytes(1.5 * 1024 * 1024 * 1024)).toBe('1.50 GB');
  });
});
