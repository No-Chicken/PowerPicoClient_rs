export interface RefreshScheduler {
  schedule: () => void;
  cancel: () => void;
}

export function createRefreshScheduler(callback: () => void, delayMs: number): RefreshScheduler {
  let timer: ReturnType<typeof setTimeout> | undefined;
  return {
    schedule() {
      if (timer !== undefined) return;
      timer = setTimeout(() => {
        timer = undefined;
        callback();
      }, delayMs);
    },
    cancel() {
      if (timer !== undefined) clearTimeout(timer);
      timer = undefined;
    },
  };
}
