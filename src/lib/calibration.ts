export const calibrationScales = [0.29296875, 29.296875, 2929.6875];
export const displayDivisors = [1, 1, 1000];
export function validCalibration(index: number, gain: number, displayedOffset: number): boolean {
  return index >= 0 && index < 3 && Number.isFinite(gain) && Number.isFinite(displayedOffset)
    && gain >= 0.85 && gain <= 1.15
    && Math.abs(displayedOffset * displayDivisors[index] / (gain * calibrationScales[index])) <= 48;
}
export function offsetLimit(index: number, gain: number): number {
  return gain * calibrationScales[index] * 48 / displayDivisors[index];
}
