use std::{
    io::{Read, Write},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use parking_lot::Mutex;
use tauri::{AppHandle, Emitter};

use crate::{
    device,
    error::{CoreError, CoreResult},
    types::{FirmwareProgress, FirmwareStage},
    ymodem,
};

pub struct FirmwareManager {
    cancel: Arc<AtomicBool>,
    thread: Mutex<Option<JoinHandle<()>>>,
}
impl Default for FirmwareManager {
    fn default() -> Self {
        Self {
            cancel: Arc::new(AtomicBool::new(false)),
            thread: Mutex::new(None),
        }
    }
}

impl FirmwareManager {
    pub fn is_running(&self) -> bool {
        self.thread
            .lock()
            .as_ref()
            .is_some_and(|handle| !handle.is_finished())
    }
    pub fn start(&self, app: AppHandle, device_id: String, firmware: PathBuf) -> CoreResult<()> {
        if self.is_running() {
            return Err(CoreError::DeviceBusy);
        }
        if !firmware.is_file() {
            return Err(CoreError::Other("firmware file does not exist".into()));
        }
        let original = device::find_device(&device_id)?;
        let cancel = Arc::clone(&self.cancel);
        cancel.store(false, Ordering::SeqCst);
        let handle = thread::spawn(move || {
            let emit = |stage, percent, message: String| {
                let progress = FirmwareProgress {
                    stage,
                    percent,
                    message,
                };
                app.emit("firmware-progress", &progress).ok();
                progress
            };
            let result = (|| -> CoreResult<()> {
                emit(
                    FirmwareStage::Connecting,
                    0,
                    "Connecting to PowerPico…".into(),
                );
                let previous_paths: Vec<String> = device::list_devices()?
                    .into_iter()
                    .map(|item| item.system_path)
                    .collect();
                match serialport::new(&original.system_path, 1_500_000)
                    .timeout(Duration::from_millis(200))
                    .open()
                {
                    Ok(mut app_port) => {
                        app_port.write_all(b"update\r\n")?;
                        app_port.flush()?;
                    }
                    Err(error)
                        if matches!(
                            error.kind(),
                            serialport::ErrorKind::Io(std::io::ErrorKind::PermissionDenied)
                        ) =>
                    {
                        return Err(CoreError::from_serial_open(&original.system_path, error));
                    }
                    Err(_) => {}
                }
                emit(
                    FirmwareStage::Rebooting,
                    2,
                    "Rebooting into bootloader…".into(),
                );
                thread::sleep(Duration::from_secs(1));
                emit(
                    FirmwareStage::SearchingBootloader,
                    4,
                    "Searching for bootloader…".into(),
                );
                let started = Instant::now();
                let boot = loop {
                    if cancel.load(Ordering::Relaxed) {
                        return Err(CoreError::Cancelled);
                    }
                    if let Ok(found) = device::rediscover_device(&original, &previous_paths) {
                        break found;
                    }
                    if started.elapsed() > Duration::from_secs(8) {
                        return Err(CoreError::Timeout("searching for bootloader".into()));
                    }
                    thread::sleep(Duration::from_millis(300));
                };
                let mut port = serialport::new(&boot.system_path, 115_200)
                    .timeout(Duration::from_millis(100))
                    .open()
                    .map_err(|error| CoreError::from_serial_open(&boot.system_path, error))?;
                emit(
                    FirmwareStage::Handshaking,
                    5,
                    "Waiting for bootloader handshake…".into(),
                );
                let started = Instant::now();
                let mut text = Vec::new();
                let mut buffer = [0u8; 512];
                let mut ready = false;
                while started.elapsed() < Duration::from_secs(6) {
                    if cancel.load(Ordering::Relaxed) {
                        return Err(CoreError::Cancelled);
                    }
                    match port.read(&mut buffer) {
                        Ok(count) if count > 0 => {
                            text.extend_from_slice(&buffer[..count]);
                            if text.windows(13).any(|part| part == b"##PICO_BOOT##")
                                || text.windows(9).any(|part| part == b"Main Menu")
                            {
                                ready = true;
                                break;
                            }
                        }
                        Ok(_) => {}
                        Err(error) if error.kind() == std::io::ErrorKind::TimedOut => {
                            port.write_all(b"0")?;
                        }
                        Err(error) => return Err(CoreError::Serial(error.to_string())),
                    }
                }
                if !ready {
                    port.write_all(b"0")?;
                }
                thread::sleep(Duration::from_millis(200));
                port.clear(serialport::ClearBuffer::Input)?;
                port.write_all(b"1")?;
                emit(FirmwareStage::Uploading, 6, "Uploading firmware…".into());
                let app_for_progress = app.clone();
                ymodem::send_file(
                    &mut *port,
                    &firmware,
                    Arc::clone(&cancel),
                    move |sent, total| {
                        let percent = 6 + ((sent as f64 / total.max(1) as f64) * 91.0) as u8;
                        app_for_progress
                            .emit(
                                "firmware-progress",
                                FirmwareProgress {
                                    stage: FirmwareStage::Uploading,
                                    percent,
                                    message: "Uploading firmware…".into(),
                                },
                            )
                            .ok();
                    },
                )?;
                emit(
                    FirmwareStage::Finishing,
                    98,
                    "Starting updated firmware…".into(),
                );
                thread::sleep(Duration::from_millis(500));
                port.write_all(b"3")?;
                Ok(())
            })();
            let final_progress = match result {
                Ok(()) => FirmwareProgress {
                    stage: FirmwareStage::Completed,
                    percent: 100,
                    message: "Firmware updated successfully".into(),
                },
                Err(CoreError::Cancelled) => FirmwareProgress {
                    stage: FirmwareStage::Cancelled,
                    percent: 0,
                    message: "Firmware update cancelled".into(),
                },
                Err(error) => FirmwareProgress {
                    stage: FirmwareStage::Failed,
                    percent: 0,
                    message: error.to_string(),
                },
            };
            app.emit("firmware-progress", &final_progress).ok();
            app.emit("firmware-finished", final_progress).ok();
        });
        *self.thread.lock() = Some(handle);
        Ok(())
    }
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }
}

impl Drop for FirmwareManager {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::SeqCst);
        if let Some(handle) = self.thread.get_mut().take() {
            let _ = handle.join();
        }
    }
}
