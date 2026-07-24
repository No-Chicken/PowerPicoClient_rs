import { describe, expect, it } from 'vitest';
import { addMetric, moveMetric, removeMetric, reorderMetric } from './metricLayout';

describe('metric layout', () => {
  it('adds hidden metrics at the end and ignores duplicates', () => {
    expect(addMetric(['averageVoltage'], 'latestVoltage')).toEqual(['averageVoltage', 'latestVoltage']);
    expect(addMetric(['averageVoltage'], 'averageVoltage')).toEqual(['averageVoltage']);
  });

  it('removes metrics while retaining at least one visible item', () => {
    expect(removeMetric(['averageVoltage', 'duration'], 'duration')).toEqual(['averageVoltage']);
    expect(removeMetric(['averageVoltage'], 'averageVoltage')).toEqual(['averageVoltage']);
  });

  it('supports button moves and drag ordering', () => {
    expect(moveMetric(['averageVoltage', 'duration'], 'duration', -1)).toEqual(['duration', 'averageVoltage']);
    expect(reorderMetric(['averageVoltage', 'duration', 'energy'], 'energy', 'averageVoltage')).toEqual([
      'energy', 'averageVoltage', 'duration',
    ]);
  });
});
