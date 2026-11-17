// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { locale } from 'svelte-i18n';
import '../i18n';
import CalibrationPage from './CalibrationPage.svelte';
import type { CalibrationState } from '../types';

const mocks = vi.hoisted(() => ({
  state: {} as CalibrationState,
  event: (_state: CalibrationState) => {},
  run: vi.fn(),
}));
vi.mock('../api', () => ({
  api: {
    listDevices: async () => [{ id: 'pico', displayName: 'PowerPico' }],
    calibrationState: async () => mocks.state,
    runCalibration: mocks.run,
    connectCalibration: async () => ({ ...mocks.state, connected: true, deviceId: 'pico' }),
    disconnectCalibration: async () => ({ ...mocks.state, connected: false, values: null }),
    cancelCalibration: vi.fn(),
  },
  onEvent: async (_name: string, callback: (value: CalibrationState) => void) => { mocks.event = callback; return () => {}; },
  formatAppError: (e: unknown) => String(e),
}));
beforeEach(() => {
  mocks.run.mockReset(); locale.set('en');
  mocks.state = { deviceId: 'pico', connected: true, busy: false, stage: 'connected', samples: 0, values: null, statistics: [], error: null, possiblyCommitted: false };
  HTMLDialogElement.prototype.showModal = function () { this.open = true; };
});
afterEach(cleanup);

describe('calibration page', () => {
  it('locks manual writes and factory reset until parameters are read', async () => {
    render(CalibrationPage, { notify: vi.fn() });
    await waitFor(() => expect((screen.getByRole('button', { name: 'Read parameters' }) as HTMLButtonElement).disabled).toBe(false));
    expect((screen.getByRole('spinbutton', { name: 'Low Gain' }) as HTMLInputElement).disabled).toBe(true);
    expect((screen.getByRole('button', { name: 'Restore factory calibration' }) as HTMLButtonElement).disabled).toBe(true);
    await fireEvent.click(screen.getByRole('button', { name: 'Read parameters' }));
    expect(mocks.run).toHaveBeenCalledWith('read', undefined, undefined, undefined);
  });
  it('confirms High writes with the exact displayed values converted to µA', async () => {
    mocks.state = { ...mocks.state, values: { gains: [1, 1, 1], offsetsUa: [0, 0, -1000] } };
    render(CalibrationPage, { notify: vi.fn() });
    const input = screen.getByRole('spinbutton', { name: 'High Zero offset' }) as HTMLInputElement;
    await waitFor(() => expect(input.disabled).toBe(false));
    expect(input.value).toBe('-1');
    await fireEvent.input(input, { target: { value: '2.5' } });
    await fireEvent.click(screen.getAllByRole('button', { name: 'Apply' })[2]);
    expect(mocks.run).not.toHaveBeenCalled();
    expect(screen.getByRole('dialog').textContent).toContain('2.5 mA');
    await fireEvent.click(screen.getAllByRole('button', { name: 'Apply' })[3]);
    expect(mocks.run).toHaveBeenCalledWith('manual', 3, 1, 2500);
  });
  it('shows uncertain writes persistently and reconnects with all edits disabled', async () => {
    mocks.state = { ...mocks.state, connected: false, stage: 'failed', error: 'link lost', possiblyCommitted: true };
    render(CalibrationPage, { notify: vi.fn() });
    expect(await screen.findByRole('alert')).toHaveProperty('textContent', expect.stringContaining('Parameters may have been saved'));
    expect((screen.getByRole('spinbutton', { name: 'Low Gain' }) as HTMLInputElement).disabled).toBe(true);
  });
  it('locks writes during sampling and removes cancellation after commit starts', async () => {
    mocks.state = { ...mocks.state, busy: true, stage: 'range:2', samples: 25000, values: { gains: [1,1,1], offsetsUa: [0,0,0] } };
    render(CalibrationPage, { notify: vi.fn() });
    await screen.findByRole('button', { name: 'Cancel' });
    expect((screen.getByRole('spinbutton', { name: 'Mid Gain' }) as HTMLInputElement).disabled).toBe(true);
    mocks.event({ ...mocks.state, stage: 'commit' });
    await waitFor(() => expect(screen.queryByRole('button', { name: 'Cancel' })).toBeNull());
  });
});
