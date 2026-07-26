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

const CRC_C: u8 = b'C';

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BootloaderState {
    Prompt,
    Menu,
    Ymodem,
}

fn observe_bootloader(
    transcript: &[u8],
    chunk: &[u8],
    consecutive_crc: &mut usize,
) -> Option<BootloaderState> {
    if transcript
        .windows(b"Main Menu".len())
        .any(|part| part == b"Main Menu")
    {
        return Some(BootloaderState::Menu);
    }
    if transcript
        .windows(b"##PICO_BOOT##".len())
        .any(|part| part == b"##PICO_BOOT##")
    {
        return Some(BootloaderState::Prompt);
    }

    for byte in chunk {
        if *byte == CRC_C {
            *consecutive_crc += 1;
            if *consecutive_crc >= 2 {
                return Some(BootloaderState::Ymodem);
            }
        } else {
            *consecutive_crc = 0;
        }
    }
    None
}

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
            let emit = |stage, percent, message_key: &str, detail: Option<String>| {
                let progress = FirmwareProgress {
                    stage,
                    percent,
                    message_key: message_key.into(),
                    detail,
                };
                app.emit("firmware-progress", &progress).ok();
                progress
            };
            let result = (|| -> CoreResult<()> {
                emit(FirmwareStage::Connecting, 0, "connecting", None);
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
                emit(FirmwareStage::Rebooting, 2, "rebooting", None);
                thread::sleep(Duration::from_secs(1));
                emit(
                    FirmwareStage::SearchingBootloader,
                    4,
                    "searchingBootloader",
                    None,
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
                emit(FirmwareStage::Handshaking, 5, "handshaking", None);
                let started = Instant::now();
                let mut text = Vec::new();
                let mut buffer = [0u8; 512];
                let mut consecutive_crc = 0;
                let mut state = None;
                let mut menu_probe_sent = false;
                while started.elapsed() < Duration::from_secs(6) {
                    if cancel.load(Ordering::Relaxed) {
                        return Err(CoreError::Cancelled);
                    }
                    match port.read(&mut buffer) {
                        Ok(count) if count > 0 => {
                            text.extend_from_slice(&buffer[..count]);
                            if let Some(observed) =
                                observe_bootloader(&text, &buffer[..count], &mut consecutive_crc)
                            {
                                state = Some(observed);
                                break;
                            }
                        }
                        Ok(_) => {}
                        Err(error) if error.kind() == std::io::ErrorKind::TimedOut => {
                            // An active YMODEM receiver sends one CRC request roughly once per
                            // second. Probing its menu between those requests injects a byte into
                            // the transfer and can make it discard the following header packet.
                            // Only probe silent legacy bootloaders after allowing enough time for
                            // two CRC requests to arrive.
                            if !menu_probe_sent
                                && consecutive_crc == 0
                                && started.elapsed() >= Duration::from_millis(2500)
                            {
                                port.write_all(b"0")?;
                                menu_probe_sent = true;
                            }
                        }
                        Err(error) => return Err(CoreError::Serial(error.to_string())),
                    }
                }
                let state = state.ok_or_else(|| {
                    CoreError::Timeout("waiting for bootloader menu or YMODEM receiver".into())
                })?;
                if state == BootloaderState::Prompt {
                    port.write_all(b"0")?;
                    thread::sleep(Duration::from_millis(200));
                    port.write_all(b"1")?;
                } else if state == BootloaderState::Menu {
                    port.write_all(b"1")?;
                }
                emit(FirmwareStage::Uploading, 6, "uploading", None);
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
                                    message_key: "uploading".into(),
                                    detail: None,
                                },
                            )
                            .ok();
                    },
                )?;
                emit(FirmwareStage::Finishing, 98, "finishing", None);
                thread::sleep(Duration::from_millis(500));
                port.write_all(b"3")?;
                Ok(())
            })();
            let final_progress = match result {
                Ok(()) => FirmwareProgress {
                    stage: FirmwareStage::Completed,
                    percent: 100,
                    message_key: "completed".into(),
                    detail: None,
                },
                Err(CoreError::Cancelled) => FirmwareProgress {
                    stage: FirmwareStage::Cancelled,
                    percent: 0,
                    message_key: "cancelled".into(),
                    detail: None,
                },
                Err(error) => FirmwareProgress {
                    stage: FirmwareStage::Failed,
                    percent: 0,
                    message_key: "failed".into(),
                    detail: Some(error.to_string()),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_an_already_active_ymodem_receiver() {
        let mut consecutive_crc = 0;
        assert_eq!(observe_bootloader(b"C", b"C", &mut consecutive_crc), None);
        assert_eq!(
            observe_bootloader(b"CC", b"C", &mut consecutive_crc),
            Some(BootloaderState::Ymodem)
        );
    }

    #[test]
    fn log_text_does_not_look_like_an_active_ymodem_receiver() {
        let log = b"Erase Complete.\r\nStarting Ymodem Receive\r\n";
        let mut consecutive_crc = 0;
        assert_eq!(observe_bootloader(log, log, &mut consecutive_crc), None);
    }

    #[test]
    fn detects_bootloader_markers_across_the_accumulated_transcript() {
        let transcript = b"booting... ##PICO_BOOT##";
        let mut consecutive_crc = 0;
        assert_eq!(
            observe_bootloader(transcript, b"BOOT##", &mut consecutive_crc),
            Some(BootloaderState::Prompt)
        );
    }

    #[test]
    fn main_menu_takes_priority_over_an_earlier_prompt_marker() {
        let transcript = b"##PICO_BOOT##\r\nMain Menu";
        let mut consecutive_crc = 0;
        assert_eq!(
            observe_bootloader(transcript, b"Main Menu", &mut consecutive_crc),
            Some(BootloaderState::Menu)
        );
    }
}
