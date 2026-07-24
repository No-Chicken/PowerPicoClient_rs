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
    pub message: String,
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
    #[serde(rename = "zh-CN")]
    ZhCn,
    #[serde(rename = "en")]
    En,
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
}

impl AppSettings {
    pub fn sanitized(mut self) -> Self {
        let mut seen = std::collections::HashSet::new();
        self.waveform_metrics
            .retain(|metric| *metric != MetricId::Unknown && seen.insert(*metric));
        if self.waveform_metrics.is_empty() {
            self.waveform_metrics = default_waveform_metrics();
        }
        self
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: ThemeMode::System,
            language: Language::ZhCn,
            waveform_metrics: default_waveform_metrics(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Sample {
    pub time: f64,
    pub voltage: f32,
    pub current: f32,
}
