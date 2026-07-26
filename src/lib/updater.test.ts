import { describe, expect, it } from 'vitest';
import { isUpdaterManifestUnavailableError } from './updater';

describe('isUpdaterManifestUnavailableError', () => {
  it('recognizes the Tauri updater error for a missing latest.json', () => {
    expect(isUpdaterManifestUnavailableError('Could not fetch a valid release JSON from the remote')).toBe(true);
    expect(isUpdaterManifestUnavailableError({ message: 'Could not fetch a valid release JSON from the remote' })).toBe(true);
  });

  it('does not hide unrelated updater failures', () => {
    expect(isUpdaterManifestUnavailableError('network timed out')).toBe(false);
    expect(isUpdaterManifestUnavailableError({ message: 'signature verification failed' })).toBe(false);
  });
});
