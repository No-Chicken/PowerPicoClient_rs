export interface NumericRange {
  min: number;
  max: number;
}

const EPSILON = 1e-9;

export function rangeSpan(range: NumericRange): number {
  return Math.max(EPSILON, range.max - range.min);
}

export function clampRange(range: NumericRange, bounds: NumericRange, minimumSpan = EPSILON): NumericRange {
  const boundSpan = Math.max(0, bounds.max - bounds.min);
  if (boundSpan <= minimumSpan) return { ...bounds };
  const span = Math.min(Math.max(rangeSpan(range), minimumSpan), boundSpan);
  let min = range.min;
  let max = min + span;
  if (min < bounds.min) {
    min = bounds.min;
    max = min + span;
  }
  if (max > bounds.max) {
    max = bounds.max;
    min = max - span;
  }
  return { min, max };
}

export function zoomRange(
  range: NumericRange,
  center: number,
  factor: number,
  bounds?: NumericRange,
  minimumSpan = EPSILON,
): NumericRange {
  const span = rangeSpan(range);
  const nextSpan = Math.max(minimumSpan, span * factor);
  const ratio = Math.min(1, Math.max(0, (center - range.min) / span));
  const next = { min: center - nextSpan * ratio, max: center + nextSpan * (1 - ratio) };
  return bounds ? clampRange(next, bounds, minimumSpan) : next;
}

export function panRange(
  range: NumericRange,
  deltaPixels: number,
  pixelSpan: number,
  bounds?: NumericRange,
): NumericRange {
  const shift = (deltaPixels / Math.max(1, pixelSpan)) * rangeSpan(range);
  const next = { min: range.min + shift, max: range.max + shift };
  return bounds ? clampRange(next, bounds) : next;
}

export function autoRange(values: number[]): NumericRange {
  const finite = values.filter(Number.isFinite);
  if (finite.length === 0) return { min: 0, max: 1 };
  const min = Math.min(...finite);
  const max = Math.max(...finite);
  const padding = Math.max((max - min) * 0.08, Math.max(Math.abs(min), Math.abs(max), 1) * 0.02);
  return { min: min - padding, max: max + padding };
}

export function exceedsDragThreshold(startX: number, startY: number, x: number, y: number, threshold = 5): boolean {
  return Math.hypot(x - startX, y - startY) > threshold;
}
