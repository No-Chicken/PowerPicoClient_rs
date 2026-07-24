import { afterEach, describe, expect, it, vi } from 'vitest';
import { createRefreshScheduler } from './refreshScheduler';

describe('refresh scheduler', () => {
  afterEach(() => vi.useRealTimers());

  it('cannot be starved by events arriving faster than the delay', () => {
    vi.useFakeTimers();
    const callback = vi.fn();
    const scheduler = createRefreshScheduler(callback, 50);
    for (let index = 0; index < 100; index += 1) scheduler.schedule();
    vi.advanceTimersByTime(50);
    expect(callback).toHaveBeenCalledTimes(1);
    scheduler.schedule();
    vi.advanceTimersByTime(50);
    expect(callback).toHaveBeenCalledTimes(2);
  });

  it('cancels a pending refresh', () => {
    vi.useFakeTimers();
    const callback = vi.fn();
    const scheduler = createRefreshScheduler(callback, 50);
    scheduler.schedule();
    scheduler.cancel();
    vi.runAllTimers();
    expect(callback).not.toHaveBeenCalled();
  });
});
