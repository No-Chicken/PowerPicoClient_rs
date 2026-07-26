function errorMessage(error: unknown): string {
  if (typeof error === 'string') return error;
  if (typeof error === 'object' && error !== null && 'message' in error) {
    const message = (error as { message?: unknown }).message;
    if (typeof message === 'string') return message;
  }
  return String(error);
}

export function isUpdaterManifestUnavailableError(error: unknown): boolean {
  return /could not fetch a valid release json from the remote/i.test(errorMessage(error));
}
