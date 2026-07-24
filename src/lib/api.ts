import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type {
  AppSettings,
  AppError,
  CaptureState,
  CaptureSummary,
  ClientUpdateInfo,
  ExternalLinks,
  OfficialFirmwareInfo,
  FirmwareProgress,
  PointReading,
  RenderSeries,
  RangeStatistics,
  SerialDevice,
} from './types';

export const api = {
  listDevices: () => invoke<SerialDevice[]>('list_serial_devices'),
  startCapture: (deviceId: string) => invoke<CaptureState>('start_capture', { deviceId }),
  stopCapture: () => invoke<CaptureState>('stop_capture'),
  captureState: () => invoke<CaptureState>('get_capture_state'),
  renderData: (start: number, end: number, pixelWidth: number) =>
    invoke<RenderSeries>('get_render_data', { start, end, pixelWidth }),
  stats: (windowSeconds?: number) => invoke<CaptureSummary>('get_stats', { windowSeconds }),
  rangeStats: (start: number, end: number) =>
    invoke<RangeStatistics>('get_range_stats', { start, end }),
  pointAt: (timeSeconds: number) => invoke<PointReading>('get_point_at', { timeSeconds }),
  clearRecords: () => invoke<void>('clear_records'),
  importRecording: (path: string) => invoke<CaptureState>('import_recording', { path }),
  exportRecording: (directory: string) => invoke<string[]>('export_recording', { directory }),
  flashFirmware: (deviceId: string, filePath: string) =>
    invoke<void>('flash_firmware', { deviceId, filePath }),
  cancelFlash: () => invoke<void>('cancel_flash'),
  officialFirmwareInfo: () => invoke<OfficialFirmwareInfo>('get_official_firmware_info'),
  downloadOfficialFirmware: (url: string, version: string, releaseDate: string) =>
    invoke<void>('download_official_firmware', { url, version, releaseDate }),
  cancelFirmwareDownload: () => invoke<void>('cancel_firmware_download'),
  checkClientUpdate: () => invoke<ClientUpdateInfo>('check_client_update'),
  externalLinks: () => invoke<ExternalLinks>('external_links'),
  getSettings: () => invoke<AppSettings>('get_settings'),
  updateSettings: (settings: AppSettings) => invoke<AppSettings>('update_settings', { settings }),
};

export function onEvent<T>(name: string, callback: (payload: T) => void): Promise<UnlistenFn> {
  return listen<T>(name, (event) => callback(event.payload));
}

export function formatAppError(error: unknown, serialPermissionMessage: string): string {
  if (typeof error === 'object' && error !== null) {
    const appError = error as Partial<AppError>;
    if (appError.code === 'SERIAL_PERMISSION_DENIED') return serialPermissionMessage;
    if (typeof appError.message === 'string') return appError.message;
  }
  return String(error);
}

export type { UnlistenFn };
