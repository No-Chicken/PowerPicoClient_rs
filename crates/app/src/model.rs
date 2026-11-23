//! Shared view state, formatting and the command/effect bridge to the core.

use std::{cell::RefCell, path::PathBuf, rc::Rc, time::Instant};

use gpui::{Bounds, Pixels, Point};
use gpui_kit::Hsla;
use powerpico_core::{
    AppSettings, CalibrationState, CaptureState, CaptureSummary, ClientUpdateInfo, ExternalLinks,
    FirmwareDownloadProgress, FirmwareProgress, MetricId, OfficialFirmwareInfo, PointReading,
    RangeStatistics, RenderSeries, SerialDevice, StorageUsage, UninstallInfo, ZeroStatistics,
};

use crate::i18n::{t, Lang};

/// One of the four screens in the sidebar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Waveform,
    Calibration,
    Firmware,
    Settings,
}

impl Page {
    pub fn label_key(self) -> &'static str {
        match self {
            Self::Waveform => "nav.waveform",
            Self::Calibration => "nav.calibration",
            Self::Firmware => "nav.firmware",
            Self::Settings => "nav.settings",
        }
    }

    pub fn icon(self) -> ely_gpui_component::primitives::IconName {
        use ely_gpui_component::primitives::IconName;
        match self {
            Self::Waveform => IconName::Activity,
            Self::Calibration => IconName::SlidersHorizontal,
            Self::Firmware => IconName::Cpu,
            Self::Settings => IconName::Settings,
        }
    }
}

/// How a toast reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Info,
    Error,
    Success,
}

#[derive(Debug, Clone)]
pub struct Toast {
    pub message: String,
    pub tone: Tone,
    pub at: Instant,
}

/// A colour family for a metric or trace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricTone {
    Default,
    Voltage,
    Current,
    Power,
}

// ---------------------------------------------------------------- formatting

pub fn format_current(value: f64) -> String {
    let abs = value.abs();
    if abs >= 1_000_000.0 {
        format!("{:.3} A", value / 1_000_000.0)
    } else if abs >= 1_000.0 {
        format!("{:.3} mA", value / 1_000.0)
    } else {
        format!("{value:.2} µA")
    }
}

pub fn format_power(value: f64) -> String {
    if value.abs() >= 1000.0 {
        format!("{:.3} W", value / 1000.0)
    } else {
        format!("{value:.2} mW")
    }
}

pub fn format_duration(seconds: f64) -> String {
    let total = seconds.max(0.0);
    let hours = (total / 3600.0).floor();
    let minutes = ((total % 3600.0) / 60.0).floor();
    let secs = total % 60.0;
    format!("{hours:02}:{minutes:02}:{secs:04.1}")
}

pub fn format_bytes(bytes: u64) -> String {
    if bytes == 0 {
        return "0 B".into();
    }
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut index = 0;
    while value >= 1024.0 && index < UNITS.len() - 1 {
        value /= 1024.0;
        index += 1;
    }
    let digits = if index == 0 || value >= 100.0 {
        0
    } else if value >= 10.0 {
        1
    } else {
        2
    };
    format!("{value:.digits$} {}", UNITS[index])
}

/// Thousands separators, as `toLocaleString` did.
pub fn count(value: u64) -> String {
    let text = value.to_string();
    let mut out = String::with_capacity(text.len() + text.len() / 3);
    for (index, ch) in text.chars().enumerate() {
        if index > 0 && (text.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

// ------------------------------------------------------------------- metrics

pub const DEFAULT_METRICS: [MetricId; 6] = [
    MetricId::AverageVoltage,
    MetricId::AverageCurrent,
    MetricId::PeakVoltage,
    MetricId::PeakCurrent,
    MetricId::AveragePower,
    MetricId::Duration,
];

pub const METRIC_CATALOG: [MetricId; 11] = [
    MetricId::LatestVoltage,
    MetricId::AverageVoltage,
    MetricId::PeakVoltage,
    MetricId::LatestCurrent,
    MetricId::AverageCurrent,
    MetricId::PeakCurrent,
    MetricId::LatestPower,
    MetricId::AveragePower,
    MetricId::Duration,
    MetricId::PointCount,
    MetricId::Energy,
];

pub fn metric_key(id: MetricId) -> &'static str {
    match id {
        MetricId::LatestVoltage => "metrics.latestVoltage",
        MetricId::AverageVoltage => "metrics.averageVoltage",
        MetricId::PeakVoltage => "metrics.peakVoltage",
        MetricId::LatestCurrent => "metrics.latestCurrent",
        MetricId::AverageCurrent => "metrics.averageCurrent",
        MetricId::PeakCurrent => "metrics.peakCurrent",
        MetricId::LatestPower => "metrics.latestPower",
        MetricId::AveragePower => "metrics.averagePower",
        MetricId::Duration => "metrics.duration",
        MetricId::PointCount => "metrics.pointCount",
        MetricId::Energy => "metrics.energy",
        MetricId::Unknown => "metrics.pointCount",
    }
}

pub fn metric_tone(id: MetricId) -> MetricTone {
    match id {
        MetricId::LatestVoltage | MetricId::AverageVoltage | MetricId::PeakVoltage => {
            MetricTone::Voltage
        }
        MetricId::LatestCurrent | MetricId::AverageCurrent | MetricId::PeakCurrent => {
            MetricTone::Current
        }
        MetricId::LatestPower | MetricId::AveragePower | MetricId::Energy => MetricTone::Power,
        _ => MetricTone::Default,
    }
}

pub fn metric_value(id: MetricId, summary: &CaptureSummary) -> String {
    match id {
        MetricId::LatestVoltage => format!("{:.3} V", summary.latest_voltage),
        MetricId::AverageVoltage => format!("{:.3} V", summary.voltage_average),
        MetricId::PeakVoltage => format!("{:.3} V", summary.voltage_peak),
        MetricId::LatestCurrent => format_current(summary.latest_current),
        MetricId::AverageCurrent => format_current(summary.current_average),
        MetricId::PeakCurrent => format_current(summary.current_peak),
        MetricId::LatestPower => format_power(summary.latest_power_mw),
        MetricId::AveragePower => format_power(summary.power_average_mw),
        MetricId::Duration => format_duration(summary.duration),
        MetricId::PointCount => count(summary.point_count),
        MetricId::Energy => format!("{:.4} mAh", summary.energy_mah),
        MetricId::Unknown => "-".into(),
    }
}

pub fn metric_tone_color(
    tone: MetricTone,
    voltage: Hsla,
    current: Hsla,
    power: Hsla,
) -> Option<Hsla> {
    match tone {
        MetricTone::Voltage => Some(voltage),
        MetricTone::Current => Some(current),
        MetricTone::Power => Some(power),
        MetricTone::Default => None,
    }
}

// --------------------------------------------------------------- range maths

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Range {
    pub min: f64,
    pub max: f64,
}

impl Range {
    pub fn new(min: f64, max: f64) -> Self {
        Self { min, max }
    }

    pub fn span(self) -> f64 {
        (self.max - self.min).max(1e-9)
    }

    pub fn clamped(self, bounds: Range, minimum_span: f64) -> Range {
        let bound_span = (bounds.max - bounds.min).max(0.0);
        if bound_span <= minimum_span {
            return bounds;
        }
        let span = self.span().max(minimum_span).min(bound_span);
        let mut min = self.min;
        let mut max = min + span;
        if min < bounds.min {
            min = bounds.min;
            max = min + span;
        }
        if max > bounds.max {
            max = bounds.max;
            min = max - span;
        }
        Range::new(min, max)
    }

    pub fn zoom(self, center: f64, factor: f64, bounds: Option<Range>, minimum_span: f64) -> Range {
        let span = self.span();
        let next = (span * factor).max(minimum_span);
        let ratio = ((center - self.min) / span).clamp(0.0, 1.0);
        let next = Range::new(center - next * ratio, center + next * (1.0 - ratio));
        match bounds {
            Some(bounds) => next.clamped(bounds, minimum_span),
            None => next,
        }
    }

    pub fn pan(self, delta_pixels: f64, pixel_span: f64, bounds: Option<Range>) -> Range {
        let shift = (delta_pixels / pixel_span.max(1.0)) * self.span();
        let next = Range::new(self.min + shift, self.max + shift);
        match bounds {
            Some(bounds) => next.clamped(bounds, 1e-9),
            None => next,
        }
    }
}

pub fn auto_range(values: impl IntoIterator<Item = f32>) -> Range {
    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    let mut seen = false;
    for value in values {
        if value.is_finite() {
            seen = true;
            min = min.min(value);
            max = max.max(value);
        }
    }
    if !seen {
        return Range::new(0.0, 1.0);
    }
    let (min, max) = (min as f64, max as f64);
    let padding = ((max - min) * 0.08).max(min.abs().max(max.abs()).max(1.0) * 0.02);
    Range::new(min - padding, max + padding)
}

// ---------------------------------------------------------------- waveform UI

/// Which part of the chart a gesture started on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChartTarget {
    Time,
    Voltage,
    Current,
}

#[derive(Debug, Clone)]
pub struct DragState {
    pub target: ChartTarget,
    pub start: Point<Pixels>,
    pub last: Point<Pixels>,
    pub dragged: bool,
    pub started_in_plot: bool,
    pub analysis: bool,
}

/// The geometry the paint pass reports back, so pointer maths can reuse it.
#[derive(Debug, Clone, Copy)]
pub struct PlotGeometry {
    pub plot: Bounds<Pixels>,
}

pub struct WaveformState {
    pub devices: Vec<SerialDevice>,
    pub selected_device: String,
    pub capture: CaptureState,
    pub summary: CaptureSummary,
    pub total: CaptureSummary,
    pub stats_window: Option<f64>,
    pub analysis: Option<RangeStatistics>,
    pub analysis_draft: Option<Range>,
    pub series: RenderSeries,
    pub time_view: Range,
    pub voltage_view: Option<Range>,
    pub current_view: Option<Range>,
    pub follow_live: bool,
    pub selected_point: Option<PointReading>,
    pub geometry: Rc<RefCell<Option<PlotGeometry>>>,
    pub render_pending: bool,
    pub render_queued: bool,
    pub latest_end: f64,
    pub request_revision: u64,
}

impl Default for WaveformState {
    fn default() -> Self {
        Self {
            devices: Vec::new(),
            selected_device: String::new(),
            capture: CaptureState::default(),
            summary: CaptureSummary::default(),
            total: CaptureSummary::default(),
            stats_window: None,
            analysis: None,
            analysis_draft: None,
            series: RenderSeries::default(),
            time_view: Range::new(0.0, 10.0),
            voltage_view: None,
            current_view: None,
            follow_live: true,
            selected_point: None,
            geometry: Rc::new(RefCell::new(None)),
            render_pending: false,
            render_queued: false,
            latest_end: 0.0,
            request_revision: 0,
        }
    }
}

impl WaveformState {
    pub fn live_range(&self) -> Range {
        let end = self.latest_end.max(self.total.duration);
        Range::new((end - 10.0).max(0.0), end.max(10.0))
    }

    pub fn time_bounds(&self) -> Option<Range> {
        let end = self
            .latest_end
            .max(self.total.duration)
            .max(self.series.available_end);
        if end <= 0.0 {
            return None;
        }
        Some(Range::new(self.series.available_start.min(0.0), end))
    }

    pub fn voltage_range(&self) -> Range {
        self.voltage_view.unwrap_or_else(|| {
            auto_range(
                self.series
                    .voltage_min
                    .iter()
                    .chain(self.series.voltage_max.iter())
                    .copied(),
            )
        })
    }

    pub fn current_range(&self) -> Range {
        self.current_view.unwrap_or_else(|| {
            auto_range(
                self.series
                    .current_min
                    .iter()
                    .chain(self.series.current_max.iter())
                    .copied(),
            )
        })
    }

    pub fn point_visible(&self) -> bool {
        self.selected_point.as_ref().is_some_and(|point| {
            point.time >= self.time_view.min && point.time <= self.time_view.max
        })
    }

    pub fn reset_axes(&mut self) {
        self.voltage_view = None;
        self.current_view = None;
    }
}

// ------------------------------------------------------------ calibration UI

#[derive(Debug, Clone)]
pub struct ManualWrite {
    pub range: usize,
    pub gain: f64,
    pub offset_ua: f64,
}

#[derive(Debug, Clone)]
pub enum Confirmation {
    AutoZero,
    Reset,
    Manual(ManualWrite),
    ClearRecords,
    ClearCache,
    Uninstall,
}

impl Confirmation {
    pub fn title_key(&self) -> &'static str {
        match self {
            Self::AutoZero | Self::Reset | Self::Manual(_) => "calibration.confirm",
            Self::ClearRecords => "waveform.confirmClear",
            Self::ClearCache => "settings.confirmClearCache",
            Self::Uninstall => "settings.confirmUninstall",
        }
    }

    pub fn body(&self, lang: Lang) -> Option<String> {
        match self {
            Self::AutoZero => Some(t(lang, "calibration.warning").to_string()),
            Self::Reset => Some(t(lang, "calibration.confirmReset").to_string()),
            Self::Manual(write) => Some(format!(
                "{}\n{} · k={:.6}, b={:.6} {}",
                t(lang, "calibration.confirmManual"),
                ["Low", "Mid", "High"][write.range],
                write.gain,
                write.offset_ua / DISPLAY_DIVISORS[write.range],
                if write.range == 2 { "mA" } else { "µA" }
            )),
            Self::ClearRecords | Self::ClearCache | Self::Uninstall => None,
        }
    }

    pub fn confirm_key(&self) -> &'static str {
        match self {
            Self::ClearRecords => "common.clear",
            Self::ClearCache => "settings.clearCache",
            Self::Uninstall => "settings.uninstall",
            _ => "calibration.apply",
        }
    }
}

pub const CALIBRATION_SCALES: [f64; 3] = [0.29296875, 29.296875, 2929.6875];
pub const DISPLAY_DIVISORS: [f64; 3] = [1.0, 1.0, 1000.0];

pub fn valid_calibration(index: usize, gain: f64, displayed_offset: f64) -> bool {
    index < 3
        && gain.is_finite()
        && displayed_offset.is_finite()
        && (0.85..=1.15).contains(&gain)
        && (displayed_offset * DISPLAY_DIVISORS[index] / (gain * CALIBRATION_SCALES[index])).abs()
            <= 48.0
}

pub fn offset_limit(index: usize, gain: f64) -> f64 {
    gain * CALIBRATION_SCALES[index] * 48.0 / DISPLAY_DIVISORS[index]
}

pub struct CalibrationUi {
    pub devices: Vec<SerialDevice>,
    pub selected_device: String,
    pub state: CalibrationState,
    pub gains: [f64; 3],
    pub offsets: [f64; 3],
    pub pending: bool,
}

impl Default for CalibrationUi {
    fn default() -> Self {
        Self {
            devices: Vec::new(),
            selected_device: String::new(),
            state: empty_calibration(),
            gains: [1.0; 3],
            offsets: [0.0; 3],
            pending: false,
        }
    }
}

pub fn empty_calibration() -> CalibrationState {
    CalibrationState {
        device_id: None,
        connected: false,
        busy: false,
        stage: String::new(),
        samples: 0,
        values: None,
        statistics: Vec::<ZeroStatistics>::new(),
        error: None,
        possibly_committed: false,
    }
}

impl CalibrationUi {
    pub fn apply(&mut self, state: CalibrationState) {
        if let Some(id) = &state.device_id {
            self.selected_device = id.clone();
        }
        if let Some(values) = &state.values {
            if !state.busy {
                for index in 0..3 {
                    self.gains[index] = (values.gains[index] * 1e6).round() / 1e6;
                    self.offsets[index] =
                        ((values.offsets_ua[index] / DISPLAY_DIVISORS[index]) * 1e6).round() / 1e6;
                }
            }
        }
        self.state = state;
    }

    pub fn busy(&self) -> bool {
        self.state.busy || self.pending
    }

    pub fn editable(&self) -> bool {
        self.state.connected && self.state.values.is_some() && !self.busy()
    }

    pub fn stage_key(&self) -> String {
        let stage = self.state.stage.as_str();
        if stage.starts_with("range:") {
            "calibration.collecting".into()
        } else if stage == "warning" {
            "calibration.warningStatus".into()
        } else if stage.is_empty() {
            "calibration.idle".into()
        } else {
            format!("calibration.{stage}")
        }
    }

    pub fn stage_range(&self) -> Option<usize> {
        let stage = self.state.stage.as_str();
        stage
            .strip_prefix("range:")
            .and_then(|index| index.parse::<usize>().ok())
    }
}

// --------------------------------------------------------------- firmware UI

pub struct FirmwareUi {
    pub devices: Vec<SerialDevice>,
    pub selected_device: String,
    pub official: Option<OfficialFirmwareInfo>,
    pub progress: FirmwareProgress,
    pub download: Option<FirmwareDownloadProgress>,
    pub loading_official: bool,
}

impl Default for FirmwareUi {
    fn default() -> Self {
        Self {
            devices: Vec::new(),
            selected_device: String::new(),
            official: None,
            progress: FirmwareProgress {
                stage: powerpico_core::FirmwareStage::Idle,
                percent: 0,
                message_key: "idle".into(),
                detail: None,
            },
            download: None,
            loading_official: false,
        }
    }
}

impl FirmwareUi {
    pub fn flashing(&self) -> bool {
        !matches!(
            self.progress.stage,
            powerpico_core::FirmwareStage::Idle
                | powerpico_core::FirmwareStage::Completed
                | powerpico_core::FirmwareStage::Cancelled
                | powerpico_core::FirmwareStage::Failed
        )
    }

    pub fn downloading(&self) -> bool {
        self.download
            .as_ref()
            .is_some_and(|progress| progress.stage == "downloading")
    }

    pub fn busy(&self) -> bool {
        self.flashing() || self.downloading()
    }

    pub fn percent(&self) -> u8 {
        if self.downloading() {
            self.download
                .as_ref()
                .map(|progress| progress.percent)
                .unwrap_or(0)
        } else {
            self.progress.percent
        }
    }
}

// --------------------------------------------------------------- settings UI

#[derive(Default)]
pub struct SettingsUi {
    pub links: Option<ExternalLinks>,
    pub storage: Option<StorageUsage>,
    pub uninstall: Option<UninstallInfo>,
    pub checking_update: bool,
    pub update: Option<ClientUpdateInfo>,
    pub refreshing_storage: bool,
    pub clearing_cache: bool,
    pub uninstalling: bool,
    pub update_version: Option<String>,
}

// -------------------------------------------------------------- commands

#[derive(Debug, Clone)]
pub enum CalibrationOp {
    Connect(String),
    Close,
    Read,
    Auto,
    Manual {
        range: usize,
        gain: f64,
        offset_ua: f64,
    },
    Reset,
}

#[derive(Debug, Clone)]
pub enum Command {
    RefreshDevices,
    StartCapture(String),
    StopCapture,
    ImportRecording(PathBuf),
    ExportRecording(PathBuf),
    ClearRecords,
    RefreshPlot {
        start: f64,
        end: f64,
        width: f64,
    },
    RefreshStats(Option<f64>),
    AnalyzeRange(f64, f64),
    PointAt(f64),
    FetchFirmwareInfo,
    DownloadFirmware {
        url: String,
        version: String,
        release_date: String,
    },
    FlashFirmware {
        device_id: String,
        path: PathBuf,
    },
    Calibration(CalibrationOp),
    CancelCalibration,
    CancelFlash,
    CancelDownload,
    UpdateSettings(AppSettings),
    RefreshStorage,
    ClearCache,
    RefreshUninstall,
    RefreshCalibration,
    FetchLinks,
    CheckUpdate,
    Uninstall,
    OpenExternal(String),
}

#[derive(Debug)]
pub enum Effect {
    Devices(Result<Vec<SerialDevice>, String>),
    Capture(Result<CaptureState, String>),
    Plot {
        start: f64,
        end: f64,
        result: Result<RenderSeries, String>,
    },
    Stats {
        window: Option<f64>,
        result: Result<CaptureSummary, String>,
    },
    Range(Result<RangeStatistics, String>),
    Point(Result<PointReading, String>),
    Exported(Result<Vec<String>, String>),
    FirmwareInfo(Result<OfficialFirmwareInfo, String>),
    FirmwareStarted(Result<(), String>),
    Calibration(Result<CalibrationState, String>),
    CalibrationStarted(Result<(), String>),
    Settings(Result<AppSettings, String>),
    Storage(Result<StorageUsage, String>),
    Uninstall(Result<UninstallInfo, String>),
    UpdateCheck(Result<ClientUpdateInfo, String>),
    Links(Result<ExternalLinks, String>),
    Notify(String, Tone),
    Exit,
}

/// Runs one command against the core, on a background thread.
pub fn run(command: Command, core: &powerpico_core::AppState) -> Effect {
    match command {
        Command::RefreshDevices => Effect::Devices(
            core.list_serial_devices()
                .map_err(|error| powerpico_core::AppError::from(error).message),
        ),
        Command::StartCapture(device) => Effect::Capture(
            core.start_capture(device)
                .map_err(|error| powerpico_core::AppError::from(error).message),
        ),
        Command::StopCapture => Effect::Capture(
            core.stop_capture()
                .map_err(|error| powerpico_core::AppError::from(error).message),
        ),
        Command::ImportRecording(path) => Effect::Capture(
            core.import_recording(&path)
                .map_err(|error| powerpico_core::AppError::from(error).message),
        ),
        Command::ExportRecording(directory) => Effect::Exported(
            core.export_recording(&directory)
                .map_err(|error| powerpico_core::AppError::from(error).message),
        ),
        Command::ClearRecords => Effect::Capture(
            core.clear_records()
                .map(|()| core.capture_state())
                .map_err(|error| powerpico_core::AppError::from(error).message),
        ),
        Command::RefreshPlot { start, end, width } => Effect::Plot {
            start,
            end,
            result: core
                .render_data(start, end, width.max(100.0) as usize)
                .map_err(|error| powerpico_core::AppError::from(error).message),
        },
        Command::RefreshStats(window) => Effect::Stats {
            window,
            result: core
                .stats(window)
                .map_err(|error| powerpico_core::AppError::from(error).message),
        },
        Command::AnalyzeRange(start, end) => Effect::Range(
            core.range_stats(start, end)
                .map_err(|error| powerpico_core::AppError::from(error).message),
        ),
        Command::PointAt(time) => Effect::Point(
            core.point_at(time)
                .map_err(|error| powerpico_core::AppError::from(error).message),
        ),
        Command::FetchFirmwareInfo => Effect::FirmwareInfo(
            core.official_firmware_info()
                .map_err(|error| powerpico_core::AppError::from(error).message),
        ),
        Command::DownloadFirmware {
            url,
            version,
            release_date,
        } => Effect::FirmwareStarted(
            core.download_official_firmware(url, version, release_date)
                .map_err(|error| powerpico_core::AppError::from(error).message),
        ),
        Command::FlashFirmware { device_id, path } => Effect::FirmwareStarted(
            core.flash_firmware(device_id, path)
                .map_err(|error| powerpico_core::AppError::from(error).message),
        ),
        Command::Calibration(operation) => {
            if let CalibrationOp::Connect(device) = &operation {
                return Effect::Calibration(
                    core.connect_calibration(device.clone())
                        .map_err(|error| powerpico_core::AppError::from(error).message),
                );
            }
            if let CalibrationOp::Close = &operation {
                return Effect::Calibration(
                    core.disconnect_calibration()
                        .map_err(|error| powerpico_core::AppError::from(error).message),
                );
            }
            let result = match operation {
                CalibrationOp::Connect(_) | CalibrationOp::Close => unreachable!(),
                CalibrationOp::Read => core.run_calibration("read".into(), None, None, None),
                CalibrationOp::Auto => core.run_calibration("auto".into(), None, None, None),
                CalibrationOp::Reset => core.run_calibration("reset".into(), None, None, None),
                CalibrationOp::Manual {
                    range,
                    gain,
                    offset_ua,
                } => core.run_calibration(
                    "manual".into(),
                    Some((range + 1) as u8),
                    Some(gain),
                    Some(offset_ua),
                ),
            };
            Effect::CalibrationStarted(
                result.map_err(|error| powerpico_core::AppError::from(error).message),
            )
        }
        Command::UpdateSettings(settings) => Effect::Settings(
            core.update_settings(settings)
                .map_err(|error| powerpico_core::AppError::from(error).message),
        ),
        Command::RefreshStorage => Effect::Storage(
            core.storage_usage()
                .map_err(|error| powerpico_core::AppError::from(error).message),
        ),
        Command::ClearCache => Effect::Storage(
            core.clear_app_cache()
                .map_err(|error| powerpico_core::AppError::from(error).message),
        ),
        Command::RefreshCalibration => Effect::Calibration(Ok(core.calibration_state())),
        Command::FetchLinks => Effect::Links(Ok(core.external_links())),
        Command::RefreshUninstall => Effect::Uninstall(
            core.uninstall_info()
                .map_err(|error| powerpico_core::AppError::from(error).message),
        ),
        Command::CheckUpdate => Effect::UpdateCheck(
            core.check_client_update()
                .map_err(|error| powerpico_core::AppError::from(error).message),
        ),
        Command::Uninstall => match core.uninstall_app() {
            Ok(()) => Effect::Exit,
            Err(error) => {
                Effect::Notify(powerpico_core::AppError::from(error).message, Tone::Error)
            }
        },
        Command::OpenExternal(url) => {
            if let Err(error) = open::that_detached(&url) {
                Effect::Notify(error.to_string(), Tone::Error)
            } else {
                Effect::Notify(String::new(), Tone::Info)
            }
        }
        Command::CancelCalibration => {
            core.cancel_calibration();
            Effect::Notify(String::new(), Tone::Info)
        }
        Command::CancelFlash => {
            core.cancel_flash();
            Effect::Notify(String::new(), Tone::Info)
        }
        Command::CancelDownload => {
            core.cancel_firmware_download();
            Effect::Notify(String::new(), Tone::Info)
        }
    }
}

/// One message from a core worker thread.
#[derive(Debug, Clone)]
pub enum CoreEvent {
    Capture(CaptureState),
    Summary(CaptureSummary),
    DataReady,
    Disconnected(String),
    Calibration(CalibrationState),
    Firmware(FirmwareProgress, bool),
    Download(FirmwareDownloadProgress),
}

impl CoreEvent {
    pub fn parse(name: &str, value: serde_json::Value) -> Option<Self> {
        fn from<T: serde::de::DeserializeOwned>(value: serde_json::Value) -> Option<T> {
            serde_json::from_value(value).ok()
        }
        match name {
            "capture-state-changed" => from(value).map(Self::Capture),
            "capture-summary-updated" => from(value).map(Self::Summary),
            "capture-data-ready" => Some(Self::DataReady),
            "device-disconnected" => {
                let message = value
                    .get("message")
                    .and_then(|message| message.as_str())
                    .unwrap_or("device disconnected")
                    .to_string();
                Some(Self::Disconnected(message))
            }
            "calibration-state" => from(value).map(Self::Calibration),
            "firmware-progress" => from(value).map(|progress| Self::Firmware(progress, false)),
            "firmware-finished" => from(value).map(|progress| Self::Firmware(progress, true)),
            "firmware-download-progress" => from(value).map(Self::Download),
            _ => None,
        }
    }
}
