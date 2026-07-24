import type { CaptureSummary, MetricId } from './types';

export type MetricTone = 'default' | 'voltage' | 'current' | 'power';

export interface MetricDefinition {
  id: MetricId;
  labelKey: string;
  tone: MetricTone;
  format: (summary: CaptureSummary) => string;
}

export const metricCatalog: MetricDefinition[] = [
  { id: 'latestVoltage', labelKey: 'metrics.latestVoltage', tone: 'voltage', format: (s) => `${s.latestVoltage.toFixed(3)} V` },
  { id: 'averageVoltage', labelKey: 'metrics.averageVoltage', tone: 'voltage', format: (s) => `${s.voltageAverage.toFixed(3)} V` },
  { id: 'peakVoltage', labelKey: 'metrics.peakVoltage', tone: 'voltage', format: (s) => `${s.voltagePeak.toFixed(3)} V` },
  { id: 'latestCurrent', labelKey: 'metrics.latestCurrent', tone: 'current', format: (s) => formatCurrent(s.latestCurrent) },
  { id: 'averageCurrent', labelKey: 'metrics.averageCurrent', tone: 'current', format: (s) => formatCurrent(s.currentAverage) },
  { id: 'peakCurrent', labelKey: 'metrics.peakCurrent', tone: 'current', format: (s) => formatCurrent(s.currentPeak) },
  { id: 'latestPower', labelKey: 'metrics.latestPower', tone: 'power', format: (s) => formatPower(s.latestPowerMw) },
  { id: 'averagePower', labelKey: 'metrics.averagePower', tone: 'power', format: (s) => formatPower(s.powerAverageMw) },
  { id: 'duration', labelKey: 'metrics.duration', tone: 'default', format: (s) => formatDuration(s.duration) },
  { id: 'pointCount', labelKey: 'metrics.pointCount', tone: 'default', format: (s) => s.pointCount.toLocaleString() },
  { id: 'energy', labelKey: 'metrics.energy', tone: 'power', format: (s) => `${s.energyMah.toFixed(4)} mAh` },
];

export const metricById = new Map(metricCatalog.map((metric) => [metric.id, metric]));

export function formatCurrent(value: number): string {
  const abs = Math.abs(value);
  if (abs >= 1_000_000) return `${(value / 1_000_000).toFixed(3)} A`;
  if (abs >= 1_000) return `${(value / 1_000).toFixed(3)} mA`;
  return `${value.toFixed(2)} µA`;
}

export function formatPower(value: number): string {
  return Math.abs(value) >= 1000 ? `${(value / 1000).toFixed(3)} W` : `${value.toFixed(2)} mW`;
}

export function formatDuration(seconds: number): string {
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const secs = seconds % 60;
  return `${String(hours).padStart(2, '0')}:${String(minutes).padStart(2, '0')}:${secs.toFixed(1).padStart(4, '0')}`;
}
