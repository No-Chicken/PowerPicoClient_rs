export interface SerialDevice {
  id: string;
  displayName: string;
  systemPath: string;
  vid?: number;
  pid?: number;
  serialNumber?: string;
  manufacturer?: string;
  product?: string;
}

export type CaptureStatus = 'idle' | 'connecting' | 'capturing' | 'stopping' | 'error';

export interface CaptureState {
  status: CaptureStatus;
  deviceId?: string;
  recordPath?: string;
  error?: string;
}

export interface CaptureSummary {
  pointCount: number;
  duration: number;
  latestVoltage: number;
  voltageAverage: number;
  voltagePeak: number;
  latestCurrent: number;
  currentAverage: number;
  currentPeak: number;
  latestPowerMw: number;
  powerAverageMw: number;
  energyMah: number;
}

export interface RangeStatistics {
  start: number;
  end: number;
  duration: number;
  pointCount: number;
  voltageAverage: number;
  voltagePeak: number;
  currentAverage: number;
  currentPeak: number;
  powerAverageMw: number;
  powerPeakMw: number;
  energyMah: number;
}

export interface PointReading {
  time: number;
  voltage: number;
  current: number;
  powerMw: number;
}

export interface RenderSeries {
  time: number[];
  voltageMin: number[];
  voltageMax: number[];
  voltageAverage: number[];
  currentMin: number[];
  currentMax: number[];
  currentAverage: number[];
  aggregated: boolean;
  availableStart: number;
  availableEnd: number;
}

export type FirmwareStage =
  | 'idle'
  | 'connecting'
  | 'rebooting'
  | 'searchingBootloader'
  | 'handshaking'
  | 'uploading'
  | 'finishing'
  | 'completed'
  | 'cancelled'
  | 'failed';

export interface FirmwareProgress {
  stage: FirmwareStage;
  percent: number;
  messageKey: string;
  detail?: string;
}

export interface OfficialFirmwareInfo {
  version: string;
  releaseDate: string;
  url: string;
  localPath?: string;
  downloaded: boolean;
}

export interface FirmwareDownloadProgress {
  percent: number;
  stage: 'downloading' | 'completed' | 'cancelled' | 'failed';
  detail?: string;
}

export interface ClientUpdateInfo {
  currentVersion: string;
  latestVersion: string;
  releaseUrl: string;
  updateAvailable: boolean;
}

export interface ExternalLinks {
  help: string;
  feedback: string;
  firmwareReleaseNotes: string;
}

export type ThemeMode = 'system' | 'light' | 'dark';
export type Language = 'auto' | 'zh-CN' | 'zh-HK' | 'en' | 'ja';
export type FirmwareMode = 'official' | 'custom';
export type MetricId =
  | 'latestVoltage'
  | 'averageVoltage'
  | 'peakVoltage'
  | 'latestCurrent'
  | 'averageCurrent'
  | 'peakCurrent'
  | 'latestPower'
  | 'averagePower'
  | 'duration'
  | 'pointCount'
  | 'energy';

export interface AppSettings {
  theme: ThemeMode;
  language: Language;
  waveformMetrics: MetricId[];
  uiScale: 0 | 100 | 125 | 150 | 175 | 200;
  checkUpdateAtStartup: boolean;
  antiAliasing: boolean;
  firmwareMode: FirmwareMode;
  customFirmwarePath: string;
  localFirmwareVersion: string;
  localFirmwareReleaseDate: string;
}

export interface AppError {
  code: string;
  message: string;
}
