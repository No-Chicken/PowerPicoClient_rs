import { describe, expect, it } from 'vitest';
import { metricById, metricCatalog } from './metrics';

describe('metric catalog', () => {
  it('contains each supported metric exactly once', () => {
    expect(metricCatalog).toHaveLength(11);
    expect(metricById.size).toBe(metricCatalog.length);
    expect(metricById.get('latestVoltage')?.labelKey).toBe('metrics.latestVoltage');
  });
});
