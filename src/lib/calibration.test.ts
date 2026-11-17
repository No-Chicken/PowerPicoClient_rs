import { describe, expect, it } from 'vitest';
import { calibrationScales, displayDivisors, offsetLimit, validCalibration } from './calibration';

describe('calibration input validation', () => {
  it('uses µA on the wire while displaying mA for High', () => {
    expect(displayDivisors).toEqual([1, 1, 1000]);
    expect(offsetLimit(2, 1)).toBe(140.625);
    expect(validCalibration(2, 1, 140.625)).toBe(true);
    expect(validCalibration(2, 1, 140.626)).toBe(false);
  });
  it('accepts firmware boundaries for every range and gain', () => {
    calibrationScales.forEach((_, i) => {
      for (const gain of [0.85, 1, 1.15]) {
        expect(validCalibration(i, gain, offsetLimit(i, gain))).toBe(true);
        expect(validCalibration(i, gain, -offsetLimit(i, gain))).toBe(true);
      }
    });
  });
  it('rejects nonfinite, blank numeric inputs and invalid ranges', () => {
    for (const gain of [NaN, Infinity, 0.849, 1.151, undefined]) expect(validCalibration(0, gain as number, 0)).toBe(false);
    for (const offset of [NaN, Infinity, undefined]) expect(validCalibration(0, 1, offset as number)).toBe(false);
    expect(validCalibration(3, 1, 0)).toBe(false);
    expect(validCalibration(-1, 1, 0)).toBe(false);
  });
});
