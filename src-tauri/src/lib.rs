mod capture;
mod device;
mod error;
mod firmware;
mod protocol;
mod settings;
mod storage;
mod types;
mod ymodem;

use std::{fs, path::PathBuf};

use tauri::{Manager, State};

use capture::CaptureManager;
use error::{AppError, CommandResult, CoreError};
use firmware::FirmwareManager;
use settings::SettingsStore;
use types::{AppSettings, CaptureState, CaptureSummary, PointReading, RenderSeries, SerialDevice};

pub struct AppState {
    capture: CaptureManager,
    firmware: FirmwareManager,
    settings: SettingsStore,
    records_dir: PathBuf,
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_data = app.path().app_data_dir()?;
            let cache = app.path().app_cache_dir()?;
            let logs = app.path().app_log_dir()?;
            fs::create_dir_all(&app_data)?;
            fs::create_dir_all(&cache)?;
            fs::create_dir_all(&logs)?;
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
                settings,
                records_dir: cache.join("records"),
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
            get_point_at,
            clear_records,
            import_recording,
            export_recording,
            flash_firmware,
            cancel_flash,
            get_settings,
            update_settings,
        ])
        .run(tauri::generate_context!())
        .expect("error while running PowerPico Client");
}
