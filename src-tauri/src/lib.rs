mod capture;
mod device;
mod error;
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
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
};

use tauri::{Manager, State};

use capture::CaptureManager;
use error::{AppError, CommandResult, CoreError};
use firmware::FirmwareManager;
use network::DownloadManager;
use settings::SettingsStore;
use types::{
    AppSettings, CaptureState, CaptureSummary, ClientUpdateInfo, OfficialFirmwareInfo,
    PointReading, RangeStatistics, RenderSeries, SerialDevice, StorageUsage, UninstallInfo,
};

pub struct AppState {
    capture: CaptureManager,
    firmware: FirmwareManager,
    downloads: DownloadManager,
    settings: SettingsStore,
    app_data_dir: PathBuf,
    records_dir: PathBuf,
    maintenance_paths: maintenance::MaintenancePaths,
    client_update_busy: AtomicBool,
}

fn ensure_maintenance_idle(state: &AppState) -> Result<(), AppError> {
    if state.capture.is_running()
        || state.firmware.is_running()
        || state.downloads.is_running()
        || state.client_update_busy.load(Ordering::SeqCst)
    {
        return Err(CoreError::DeviceBusy.into());
    }
    Ok(())
}

fn command_error(error: CoreError) -> AppError {
    error.into()
}

#[tauri::command]
fn list_serial_devices() -> CommandResult<Vec<SerialDevice>> {
    device::list_devices().map_err(command_error)
}

#[tauri::command]
fn start_capture(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    device_id: String,
) -> CommandResult<CaptureState> {
    if state.firmware.is_running() {
        return Err(CoreError::DeviceBusy.into());
    }
    state
        .capture
        .start(app, device_id, state.records_dir.clone())
        .map_err(command_error)
}

#[tauri::command]
fn stop_capture(app: tauri::AppHandle, state: State<'_, AppState>) -> CommandResult<CaptureState> {
    state.capture.stop(&app).map_err(command_error)
}

#[tauri::command]
fn get_capture_state(state: State<'_, AppState>) -> CaptureState {
    state.capture.state()
}

#[tauri::command]
fn get_render_data(
    state: State<'_, AppState>,
    start: f64,
    end: f64,
    pixel_width: usize,
) -> CommandResult<RenderSeries> {
    let path = state.capture.record_path().map_err(command_error)?;
    storage::render_data(&path, start, end, pixel_width).map_err(command_error)
}

#[tauri::command]
fn get_stats(
    state: State<'_, AppState>,
    window_seconds: Option<f64>,
) -> CommandResult<CaptureSummary> {
    state.capture.summary(window_seconds).map_err(command_error)
}

#[tauri::command]
fn get_range_stats(
    state: State<'_, AppState>,
    start: f64,
    end: f64,
) -> CommandResult<RangeStatistics> {
    state
        .capture
        .range_statistics(start, end)
        .map_err(command_error)
}

#[tauri::command]
fn get_point_at(state: State<'_, AppState>, time_seconds: f64) -> CommandResult<PointReading> {
    let path = state.capture.record_path().map_err(command_error)?;
    storage::point_at(&path, time_seconds).map_err(command_error)
}

#[tauri::command]
fn clear_records(state: State<'_, AppState>) -> CommandResult<()> {
    state
        .capture
        .clear(&state.records_dir)
        .map_err(command_error)
}

#[tauri::command]
fn import_recording(state: State<'_, AppState>, path: String) -> CommandResult<CaptureState> {
    state
        .capture
        .import(&PathBuf::from(path))
        .map_err(command_error)
}

#[tauri::command]
fn export_recording(state: State<'_, AppState>, directory: String) -> CommandResult<Vec<String>> {
    let path = state.capture.record_path().map_err(command_error)?;
    storage::export_recording(&path, &PathBuf::from(directory)).map_err(command_error)
}

#[tauri::command]
fn flash_firmware(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    device_id: String,
    file_path: String,
) -> CommandResult<()> {
    if state.capture.is_running() {
        return Err(CoreError::DeviceBusy.into());
    }
    state
        .firmware
        .start(app, device_id, PathBuf::from(file_path))
        .map_err(command_error)
}

#[tauri::command]
fn cancel_flash(state: State<'_, AppState>) {
    state.firmware.cancel();
}

#[tauri::command]
async fn get_official_firmware_info(
    state: State<'_, AppState>,
) -> CommandResult<OfficialFirmwareInfo> {
    let app_data = state.app_data_dir.clone();
    let settings = state.settings.get();
    tauri::async_runtime::spawn_blocking(move || {
        network::fetch_official_firmware_info(
            &app_data,
            settings.local_firmware_version,
            settings.local_firmware_release_date,
        )
    })
    .await
    .map_err(|error| CoreError::Other(error.to_string()))?
    .map_err(command_error)
}

#[tauri::command]
fn download_official_firmware(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    url: String,
    version: String,
    release_date: String,
) -> CommandResult<()> {
    state
        .downloads
        .start(
            app,
            url,
            version,
            release_date,
            state.app_data_dir.clone(),
            state.settings.clone(),
        )
        .map_err(command_error)
}

#[tauri::command]
fn cancel_firmware_download(state: State<'_, AppState>) {
    state.downloads.cancel();
}

#[tauri::command]
async fn check_client_update() -> CommandResult<ClientUpdateInfo> {
    tauri::async_runtime::spawn_blocking(|| network::check_client_update(env!("CARGO_PKG_VERSION")))
        .await
        .map_err(|error| CoreError::Other(error.to_string()))?
        .map_err(command_error)
}

#[tauri::command]
fn external_links() -> serde_json::Value {
    serde_json::json!({
        "help": network::HELP_URL,
        "feedback": network::FEEDBACK_URL,
        "firmwareReleaseNotes": network::RELEASE_NOTES_URL,
    })
}

#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> AppSettings {
    state.settings.get()
}

#[tauri::command]
fn update_settings(
    state: State<'_, AppState>,
    settings: AppSettings,
) -> CommandResult<AppSettings> {
    state.settings.update(settings).map_err(command_error)
}

#[tauri::command]
fn get_storage_usage(state: State<'_, AppState>) -> CommandResult<StorageUsage> {
    maintenance::storage_usage(&state.maintenance_paths).map_err(command_error)
}

#[tauri::command]
fn clear_app_cache(state: State<'_, AppState>) -> CommandResult<StorageUsage> {
    ensure_maintenance_idle(&state)?;
    state
        .capture
        .clear(&state.records_dir)
        .map_err(command_error)?;
    maintenance::clear_directory_contents(&state.maintenance_paths.cache).map_err(command_error)?;
    fs::create_dir_all(&state.records_dir)
        .map_err(CoreError::from)
        .map_err(command_error)?;
    maintenance::storage_usage(&state.maintenance_paths).map_err(command_error)
}

#[tauri::command]
fn get_uninstall_info(state: State<'_, AppState>) -> CommandResult<UninstallInfo> {
    maintenance::uninstall_info(&state.maintenance_paths).map_err(command_error)
}

#[tauri::command]
fn set_client_update_busy(state: State<'_, AppState>, busy: bool) {
    state.client_update_busy.store(busy, Ordering::SeqCst);
}

#[tauri::command]
fn uninstall_app(app: tauri::AppHandle, state: State<'_, AppState>) -> CommandResult<()> {
    ensure_maintenance_idle(&state)?;
    maintenance::begin_uninstall(&state.maintenance_paths).map_err(command_error)?;
    app.exit(0);
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let app_data = app.path().app_data_dir()?;
            let cache = app.path().app_cache_dir()?;
            let logs = app.path().app_log_dir()?;
            let home = app.path().home_dir()?;
            fs::create_dir_all(&app_data)?;
            fs::create_dir_all(&cache)?;
            fs::create_dir_all(&logs)?;
            let records_dir = cache.join("records");
            fs::create_dir_all(&records_dir)?;
            if let Err(error) = storage::clean_stale_recordings(
                &records_dir,
                std::time::Duration::from_secs(3 * 24 * 60 * 60),
            ) {
                tracing::warn!("failed to clean stale recordings: {error}");
            }
            let file_appender = tracing_appender::rolling::daily(&logs, "powerpico.log");
            let _ = tracing_subscriber::fmt()
                .with_writer(file_appender)
                .with_env_filter("info")
                .try_init();
            let settings = SettingsStore::load(&app_data)
                .map_err(|error| Box::<dyn std::error::Error>::from(error.to_string()))?;
            app.manage(AppState {
                capture: CaptureManager::default(),
                firmware: FirmwareManager::default(),
                downloads: DownloadManager::default(),
                settings,
                app_data_dir: app_data,
                records_dir,
                maintenance_paths: maintenance::MaintenancePaths {
                    app_data: app.path().app_data_dir()?,
                    cache,
                    logs,
                    home,
                },
                client_update_busy: AtomicBool::new(false),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_serial_devices,
            start_capture,
            stop_capture,
            get_capture_state,
            get_render_data,
            get_stats,
            get_range_stats,
            get_point_at,
            clear_records,
            import_recording,
            export_recording,
            flash_firmware,
            cancel_flash,
            get_official_firmware_info,
            download_official_firmware,
            cancel_firmware_download,
            check_client_update,
            external_links,
            get_settings,
            update_settings,
            get_storage_usage,
            clear_app_cache,
            get_uninstall_info,
            set_client_update_busy,
            uninstall_app,
        ])
        .run(tauri::generate_context!())
        .expect("error while running PowerPico Client");
}
