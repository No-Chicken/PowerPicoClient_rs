import { describe, expect, it } from 'vitest';
import { en, ja, resolveLocale, zh, zhHk } from './i18n';

describe('translations', () => {
  it('keeps the primary navigation in both languages', () => {
    expect(Object.keys(en.nav)).toEqual(Object.keys(zh.nav));
    expect(en.nav.waveform).toBe('Waveform');
    expect(zh.nav.waveform).toBe('波形');
  });

  it('keeps all supported locales structurally compatible', () => {
    for (const messages of [zhHk, ja]) {
      expect(Object.keys(messages.nav)).toEqual(Object.keys(en.nav));
      expect(Object.keys(messages.firmware)).toEqual(Object.keys(en.firmware));
      expect(Object.keys(messages.settings)).toEqual(Object.keys(en.settings));
    }
  });

  it('resolves system languages with stable fallbacks', () => {
    expect(resolveLocale('auto', 'zh-TW')).toBe('zh-HK');
    expect(resolveLocale('auto', 'ja-JP')).toBe('ja');
    expect(resolveLocale('auto', 'de-DE')).toBe('en');
    expect(resolveLocale('zh-CN', 'en-US')).toBe('zh-CN');
  });
});
