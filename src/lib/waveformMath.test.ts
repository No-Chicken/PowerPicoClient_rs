import { describe, expect, it } from 'vitest';
import { clampRange, exceedsDragThreshold, panRange, zoomRange } from './waveformMath';

describe('waveform viewport math', () => {
  it('zooms around the pointer and clamps to available time', () => {
    expect(zoomRange({ min: 0, max: 10 }, 2, 0.5, { min: 0, max: 20 })).toEqual({ min: 1, max: 6 });
    expect(clampRange({ min: -4, max: 6 }, { min: 0, max: 20 })).toEqual({ min: 0, max: 10 });
  });

  it('pans one range without mutating another', () => {
    const voltage = { min: 0, max: 5 };
    const current = { min: -10, max: 10 };
    expect(panRange(voltage, 50, 100)).toEqual({ min: 2.5, max: 7.5 });
    expect(current).toEqual({ min: -10, max: 10 });
  });

  it('distinguishes a click from a drag', () => {
    expect(exceedsDragThreshold(0, 0, 3, 4)).toBe(false);
    expect(exceedsDragThreshold(0, 0, 6, 0)).toBe(true);
  });
});
