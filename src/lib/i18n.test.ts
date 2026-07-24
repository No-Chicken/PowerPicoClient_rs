import { describe, expect, it } from 'vitest';
import { en, zh } from './i18n';

describe('translations', () => {
  it('keeps the primary navigation in both languages', () => {
    expect(Object.keys(en.nav)).toEqual(Object.keys(zh.nav));
    expect(en.nav.waveform).toBe('Waveform');
    expect(zh.nav.waveform).toBe('波形');
  });
});
