use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SerialDevice {
    pub id: String,
    pub display_name: String,
    pub system_path: String,
    pub vid: Option<u16>,
    pub pid: Option<u16>,
    pub serial_number: Option<String>,
    pub manufacturer: Option<String>,
    pub product: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CaptureStatus {
    Idle,
    Connecting,
    Capturing,
    Stopping,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureState {
    pub status: CaptureStatus,
    pub device_id: Option<String>,
    pub record_path: Option<String>,
    pub error: Option<String>,
}

impl Default for CaptureState {
    fn default() -> Self {
        Self {
            status: CaptureStatus::Idle,
            device_id: None,
            record_path: None,
            error: None,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureSummary {
    pub point_count: u64,
    pub duration: f64,
    pub latest_voltage: f64,
    pub voltage_average: f64,
    pub voltage_peak: f64,
    pub latest_current: f64,
    pub current_average: f64,
    pub current_peak: f64,
    pub latest_power_mw: f64,
    pub power_average_mw: f64,
    pub energy_mah: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RangeStatistics {
    pub start: f64,
    pub end: f64,
    pub duration: f64,
    pub point_count: u64,
    pub voltage_average: f64,
    pub voltage_peak: f64,
    pub current_average: f64,
    pub current_peak: f64,
    pub power_average_mw: f64,
    pub power_peak_mw: f64,
    pub energy_mah: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PointReading {
    pub time: f64,
    pub voltage: f64,
    pub current: f64,
    pub power_mw: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderSeries {
    pub time: Vec<f64>,
    pub voltage_min: Vec<f32>,
    pub voltage_max: Vec<f32>,
    pub voltage_average: Vec<f32>,
    pub current_min: Vec<f32>,
    pub current_max: Vec<f32>,
    pub current_average: Vec<f32>,
    pub aggregated: bool,
    pub available_start: f64,
    pub available_end: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum FirmwareStage {
    Idle,
    Connecting,
    Rebooting,
    SearchingBootloader,
    Handshaking,
    Uploading,
    Finishing,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FirmwareProgress {
    pub stage: FirmwareStage,
    pub percent: u8,
    pub message_key: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OfficialFirmwareInfo {
    pub version: String,
    pub release_date: String,
    pub url: String,
    pub local_path: Option<String>,
    pub downloaded: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FirmwareDownloadProgress {
    pub percent: u8,
    pub stage: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClientUpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub release_url: String,
    pub update_available: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StorageUsage {
    pub cache_bytes: u64,
    pub app_data_bytes: u64,
    pub log_bytes: u64,
    pub total_bytes: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UninstallInfo {
    pub supported: bool,
    pub app_path: Option<String>,
    pub app_bytes: u64,
    pub cache_bytes: u64,
    pub app_data_bytes: u64,
    pub log_bytes: u64,
    pub system_data_bytes: u64,
    pub total_bytes: u64,
    pub mounted_volumes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ThemeMode {
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Language {
    #[serde(rename = "auto")]
    Auto,
    #[serde(rename = "zh-CN")]
    ZhCn,
    #[serde(rename = "zh-HK")]
    ZhHk,
    #[serde(rename = "en")]
    En,
    #[serde(rename = "ja")]
    Ja,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum FirmwareMode {
    Official,
    Custom,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum MetricId {
    LatestVoltage,
    AverageVoltage,
    PeakVoltage,
    LatestCurrent,
    AverageCurrent,
    PeakCurrent,
    LatestPower,
    AveragePower,
    Duration,
    PointCount,
    Energy,
    #[serde(other)]
    Unknown,
}

pub fn default_waveform_metrics() -> Vec<MetricId> {
    vec![
        MetricId::AverageVoltage,
        MetricId::AverageCurrent,
        MetricId::PeakVoltage,
        MetricId::PeakCurrent,
        MetricId::AveragePower,
        MetricId::Duration,
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub theme: ThemeMode,
    pub language: Language,
    #[serde(default = "default_waveform_metrics")]
    pub waveform_metrics: Vec<MetricId>,
    #[serde(default)]
    pub ui_scale: u16,
    #[serde(default = "default_true")]
    pub check_update_at_startup: bool,
    #[serde(default)]
    pub anti_aliasing: bool,
    #[serde(default = "default_firmware_mode")]
    pub firmware_mode: FirmwareMode,
    #[serde(default)]
    pub custom_firmware_path: String,
    #[serde(default)]
    pub local_firmware_version: String,
    #[serde(default)]
    pub local_firmware_release_date: String,
}

fn default_true() -> bool {
    true
}

fn default_firmware_mode() -> FirmwareMode {
    FirmwareMode::Official
}

impl AppSettings {
    pub fn sanitized(mut self) -> Self {
        let mut seen = std::collections::HashSet::new();
        self.waveform_metrics
            .retain(|metric| *metric != MetricId::Unknown && seen.insert(*metric));
        if self.waveform_metrics.is_empty() {
            self.waveform_metrics = default_waveform_metrics();
        }
        if !matches!(self.ui_scale, 0 | 100 | 125 | 150 | 175 | 200) {
            self.ui_scale = 0;
        }
        self
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: ThemeMode::System,
            language: Language::Auto,
            waveform_metrics: default_waveform_metrics(),
            ui_scale: 0,
            check_update_at_startup: true,
            anti_aliasing: false,
            firmware_mode: FirmwareMode::Official,
            custom_firmware_path: String::new(),
            local_firmware_version: String::new(),
            local_firmware_release_date: String::new(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Sample {
    pub time: f64,
    pub voltage: f32,
    pub current: f32,
}
