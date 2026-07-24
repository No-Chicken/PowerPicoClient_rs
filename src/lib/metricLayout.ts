import type { MetricId } from './types';

export const defaultWaveformMetrics: MetricId[] = [
  'averageVoltage', 'averageCurrent', 'peakVoltage', 'peakCurrent', 'averagePower', 'duration',
];

export function addMetric(metrics: MetricId[], metric: MetricId): MetricId[] {
  return metrics.includes(metric) ? metrics : [...metrics, metric];
}

export function removeMetric(metrics: MetricId[], metric: MetricId): MetricId[] {
  return metrics.length <= 1 ? metrics : metrics.filter((item) => item !== metric);
}

export function moveMetric(metrics: MetricId[], metric: MetricId, delta: -1 | 1): MetricId[] {
  const from = metrics.indexOf(metric);
  const to = from + delta;
  if (from < 0 || to < 0 || to >= metrics.length) return metrics;
  const next = [...metrics];
  [next[from], next[to]] = [next[to], next[from]];
  return next;
}

export function reorderMetric(metrics: MetricId[], source: MetricId, target: MetricId): MetricId[] {
  if (source === target) return metrics;
  const from = metrics.indexOf(source);
  const to = metrics.indexOf(target);
  if (from < 0 || to < 0) return metrics;
  const next = [...metrics];
  next.splice(from, 1);
  next.splice(to, 0, source);
  return next;
}
