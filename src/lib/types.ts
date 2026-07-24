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
  message: string;
}

export type ThemeMode = 'system' | 'light' | 'dark';
export type Language = 'zh-CN' | 'en';
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
}

export interface AppError {
  code: string;
  message: string;
}
