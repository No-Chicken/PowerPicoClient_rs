//! PowerPico core: device discovery, capture, storage, calibration, firmware
//! and maintenance. Nothing here knows about a window; progress travels through
//! [`events::Events`].

pub mod calibration;
pub mod capture;
mod device;
pub mod error;
pub mod events;
mod firmware;
mod maintenance;
mod network;
mod protocol;
mod settings;
mod storage;
mod types;
mod ymodem;

use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

use serde::{Deserialize, Serialize};

pub use calibration::{CalibrationState, Values, ZeroStatistics};
pub use capture::CaptureManager;
pub use error::{AppError, CommandResult, CoreError, CoreResult};
pub use events::Events;
pub use firmware::FirmwareManager;
pub use maintenance::MaintenancePaths;
pub use network::DownloadManager;
pub use settings::SettingsStore;
pub use types::{
    AppSettings, CaptureState, CaptureStatus, CaptureSummary, ClientUpdateInfo,
    FirmwareDownloadProgress, FirmwareMode, FirmwareProgress, FirmwareStage, Language, MetricId,
    OfficialFirmwareInfo, PointReading, RangeStatistics, RenderSeries, Sample, SerialDevice,
    StorageUsage, ThemeMode, UninstallInfo,
};

pub const BUNDLE_ID: &str = "com.openfeasttech.powerpico-client";

/// The directories the application owns, resolved once at start-up.
#[derive(Debug, Clone)]
pub struct AppPaths {
    pub app_data: PathBuf,
    pub cache: PathBuf,
    pub logs: PathBuf,
    pub home: PathBuf,
    pub records: PathBuf,
}

impl AppPaths {
    /// Resolves the platform directories Tauri used, so an existing install
    /// keeps its settings and local firmware.
    pub fn resolve() -> CoreResult<Self> {
        let home = home_dir().ok_or_else(|| CoreError::Config("no home directory".into()))?;
        let (app_data, cache, logs) = platform_dirs(&home);
        let records = cache.join("records");
        Ok(Self {
            app_data,
            cache,
            logs,
            home,
            records,
        })
    }

    pub fn prepare(&self) -> CoreResult<()> {
        fs::create_dir_all(&self.app_data)?;
        fs::create_dir_all(&self.cache)?;
        fs::create_dir_all(&self.logs)?;
        fs::create_dir_all(&self.records)?;
        Ok(())
    }

    pub fn maintenance(&self) -> MaintenancePaths {
        MaintenancePaths {
            app_data: self.app_data.clone(),
            cache: self.cache.clone(),
            logs: self.logs.clone(),
            home: self.home.clone(),
        }
    }
}

fn home_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var_os("USERPROFILE").map(PathBuf::from)
    }
    #[cfg(not(windows))]
    {
        std::env::var_os("HOME").map(PathBuf::from)
    }
}

fn platform_dirs(home: &Path) -> (PathBuf, PathBuf, PathBuf) {
    #[cfg(target_os = "macos")]
    {
        let library = home.join("Library");
        (
            library.join("Application Support").join(BUNDLE_ID),
            library.join("Caches").join(BUNDLE_ID),
            library.join("Logs").join(BUNDLE_ID),
        )
    }
    #[cfg(target_os = "windows")]
    {
        let app_data = std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join("AppData").join("Roaming"))
            .join(BUNDLE_ID);
        let local = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join("AppData").join("Local"))
            .join(BUNDLE_ID);
        (app_data.clone(), local.join("cache"), local.join("logs"))
    }
    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        let data = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".local").join("share"))
            .join(BUNDLE_ID);
        let cache = std::env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".cache"))
            .join(BUNDLE_ID);
        let logs = data.join("logs");
        (data, cache, logs)
    }
}

/// Points `tracing` at the platform log directory. Safe to call once per
/// process; a second call is ignored.
pub fn init_logging(paths: &AppPaths) {
    let appender = tracing_appender::rolling::daily(&paths.logs, "powerpico.log");
    let _ = tracing_subscriber::fmt()
        .with_writer(appender)
        .with_env_filter("info")
        .try_init();
}

/// Links shipped to the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalLinks {
    pub help: String,
    pub feedback: String,
    pub firmware_release_notes: String,
}

/// Everything the application owns, assembled once and shared by the views.
pub struct AppState {
    pub capture: CaptureManager,
    pub calibration: calibration::CalibrationManager,
    serial_gate: parking_lot::Mutex<()>,
    pub firmware: FirmwareManager,
    pub downloads: DownloadManager,
    pub settings: SettingsStore,
    pub paths: AppPaths,
    events: Events,
    client_update_busy: AtomicBool,
}

impl AppState {
    pub fn new(paths: AppPaths, events: Events) -> CoreResult<Self> {
        paths.prepare()?;
        if let Err(error) = storage::clean_stale_recordings(
            &paths.records,
            std::time::Duration::from_secs(3 * 24 * 60 * 60),
        ) {
            tracing::warn!("failed to clean stale recordings: {error}");
        }
        let settings = SettingsStore::load(&paths.app_data)?;
        Ok(Self {
            capture: CaptureManager::default(),
            calibration: calibration::CalibrationManager::default(),
            serial_gate: parking_lot::Mutex::new(()),
            firmware: FirmwareManager::default(),
            downloads: DownloadManager::default(),
            settings,
            paths,
            events,
            client_update_busy: AtomicBool::new(false),
        })
    }

    fn ensure_maintenance_idle(&self) -> CoreResult<()> {
        if self.capture.is_running()
            || self.calibration.connected()
            || self.firmware.is_running()
            || self.downloads.is_running()
            || self.client_update_busy.load(Ordering::SeqCst)
        {
            return Err(CoreError::DeviceBusy);
        }
        Ok(())
    }

    pub fn list_serial_devices(&self) -> CoreResult<Vec<SerialDevice>> {
        device::list_devices()
    }

    pub fn start_capture(&self, device_id: String) -> CoreResult<CaptureState> {
        let _gate = self.serial_gate.lock();
        if self.firmware.is_running() || self.calibration.connected() {
            return Err(CoreError::DeviceBusy);
        }
        self.capture
            .start(self.events.clone(), device_id, self.paths.records.clone())
    }

    pub fn stop_capture(&self) -> CoreResult<CaptureState> {
        self.capture.stop(&self.events)
    }

    pub fn capture_state(&self) -> CaptureState {
        self.capture.state()
    }

    pub fn render_data(
        &self,
        start: f64,
        end: f64,
        pixel_width: usize,
    ) -> CoreResult<RenderSeries> {
        let path = self.capture.record_path()?;
        storage::render_data(&path, start, end, pixel_width)
    }

    pub fn stats(&self, window_seconds: Option<f64>) -> CoreResult<CaptureSummary> {
        self.capture.summary(window_seconds)
    }

    pub fn range_stats(&self, start: f64, end: f64) -> CoreResult<RangeStatistics> {
        self.capture.range_statistics(start, end)
    }

    pub fn point_at(&self, time_seconds: f64) -> CoreResult<PointReading> {
        let path = self.capture.record_path()?;
        storage::point_at(&path, time_seconds)
    }

    pub fn clear_records(&self) -> CoreResult<()> {
        self.capture.clear(&self.paths.records)
    }

    pub fn import_recording(&self, path: &Path) -> CoreResult<CaptureState> {
        self.capture.import(path)
    }

    pub fn export_recording(&self, directory: &Path) -> CoreResult<Vec<String>> {
        let path = self.capture.record_path()?;
        storage::export_recording(&path, directory)
    }

    pub fn flash_firmware(&self, device_id: String, file_path: PathBuf) -> CoreResult<()> {
        let _gate = self.serial_gate.lock();
        if self.capture.is_running() || self.calibration.connected() {
            return Err(CoreError::DeviceBusy);
        }
        self.firmware
            .start(self.events.clone(), device_id, file_path)
    }

    pub fn calibration_state(&self) -> CalibrationState {
        self.calibration.state()
    }

    pub fn connect_calibration(&self, device_id: String) -> CoreResult<CalibrationState> {
        let _gate = self.serial_gate.lock();
        if self.capture.is_running() || self.firmware.is_running() {
            return Err(CoreError::DeviceBusy);
        }
        let next = self.calibration.connect(device_id)?;
        self.events.emit("calibration-state", &next);
        Ok(next)
    }

    pub fn disconnect_calibration(&self) -> CoreResult<CalibrationState> {
        let _gate = self.serial_gate.lock();
        let next = self.calibration.disconnect()?;
        self.events.emit("calibration-state", &next);
        Ok(next)
    }

    pub fn run_calibration(
        &self,
        operation: String,
        range: Option<u8>,
        gain: Option<f64>,
        offset_ua: Option<f64>,
    ) -> CoreResult<()> {
        let _gate = self.serial_gate.lock();
        self.calibration
            .start(self.events.clone(), operation, range, gain, offset_ua)
    }

    pub fn cancel_calibration(&self) {
        self.calibration.cancel();
    }

    pub fn cancel_flash(&self) {
        self.firmware.cancel();
    }

    pub fn official_firmware_info(&self) -> CoreResult<OfficialFirmwareInfo> {
        let settings = self.settings.get();
        network::fetch_official_firmware_info(
            &self.paths.app_data,
            settings.local_firmware_version,
            settings.local_firmware_release_date,
        )
    }

    pub fn download_official_firmware(
        &self,
        url: String,
        version: String,
        release_date: String,
    ) -> CoreResult<()> {
        self.downloads.start(
            self.events.clone(),
            url,
            version,
            release_date,
            self.paths.app_data.clone(),
            self.settings.clone(),
        )
    }

    pub fn cancel_firmware_download(&self) {
        self.downloads.cancel();
    }

    pub fn check_client_update(&self) -> CoreResult<ClientUpdateInfo> {
        network::check_client_update(env!("CARGO_PKG_VERSION"))
    }

    pub fn external_links(&self) -> ExternalLinks {
        ExternalLinks {
            help: network::HELP_URL.to_string(),
            feedback: network::FEEDBACK_URL.to_string(),
            firmware_release_notes: network::RELEASE_NOTES_URL.to_string(),
        }
    }

    pub fn settings(&self) -> AppSettings {
        self.settings.get()
    }

    pub fn update_settings(&self, settings: AppSettings) -> CoreResult<AppSettings> {
        self.settings.update(settings)
    }

    pub fn storage_usage(&self) -> CoreResult<StorageUsage> {
        maintenance::storage_usage(&self.paths.maintenance())
    }

    pub fn clear_app_cache(&self) -> CoreResult<StorageUsage> {
        self.ensure_maintenance_idle()?;
        self.capture.clear(&self.paths.records)?;
        maintenance::clear_directory_contents(&self.paths.cache)?;
        fs::create_dir_all(&self.paths.records)?;
        maintenance::storage_usage(&self.paths.maintenance())
    }

    pub fn uninstall_info(&self) -> CoreResult<UninstallInfo> {
        maintenance::uninstall_info(&self.paths.maintenance())
    }

    pub fn set_client_update_busy(&self, busy: bool) {
        self.client_update_busy.store(busy, Ordering::SeqCst);
    }

    pub fn uninstall_app(&self) -> CoreResult<()> {
        self.ensure_maintenance_idle()?;
        maintenance::begin_uninstall(&self.paths.maintenance())
    }

    /// Called when the window closes so the calibration port is released.
    pub fn shutdown(&self) {
        self.calibration.shutdown();
    }
}
