use std::{
    io::Read,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use parking_lot::{Mutex, RwLock};
use tauri::{AppHandle, Emitter};

use crate::{
    device,
    error::{CoreError, CoreResult},
    protocol::FrameParser,
    storage::{self, RecordWriter},
    types::{CaptureState, CaptureStatus, CaptureSummary},
};

enum ReaderMessage {
    Data(Vec<u8>),
    Error(String),
}

pub struct CaptureManager {
    state: Arc<RwLock<CaptureState>>,
    summary: Arc<RwLock<CaptureSummary>>,
    record_path: Arc<RwLock<Option<PathBuf>>>,
    stop: Arc<AtomicBool>,
    thread: Mutex<Option<JoinHandle<()>>>,
}

impl Default for CaptureManager {
    fn default() -> Self {
        Self {
            state: Arc::new(RwLock::new(CaptureState::default())),
            summary: Arc::new(RwLock::new(CaptureSummary::default())),
            record_path: Arc::new(RwLock::new(None)),
            stop: Arc::new(AtomicBool::new(false)),
            thread: Mutex::new(None),
        }
    }
}

impl CaptureManager {
    pub fn state(&self) -> CaptureState {
        self.state.read().clone()
    }
    pub fn summary(&self, window: Option<f64>) -> CoreResult<CaptureSummary> {
        if window.is_none() && self.state.read().status == CaptureStatus::Capturing {
            return Ok(self.summary.read().clone());
        }
        let path = self
            .record_path
            .read()
            .clone()
            .ok_or(CoreError::NoRecording)?;
        storage::recording_summary(&path, window)
    }
    pub fn record_path(&self) -> CoreResult<PathBuf> {
        self.record_path
            .read()
            .clone()
            .ok_or(CoreError::NoRecording)
    }
    pub fn is_running(&self) -> bool {
        matches!(
            self.state.read().status,
            CaptureStatus::Connecting | CaptureStatus::Capturing | CaptureStatus::Stopping
        )
    }

    pub fn start(
        &self,
        app: AppHandle,
        device_id: String,
        records_dir: PathBuf,
    ) -> CoreResult<CaptureState> {
        if self.is_running() {
            return Err(CoreError::DeviceBusy);
        }
        let device = device::find_device(&device_id)?;
        self.stop.store(false, Ordering::SeqCst);
        *self.summary.write() = CaptureSummary::default();
        let connecting = CaptureState {
            status: CaptureStatus::Connecting,
            device_id: Some(device_id.clone()),
            record_path: None,
            error: None,
        };
        *self.state.write() = connecting.clone();
        app.emit("capture-state-changed", &connecting).ok();

        let state = Arc::clone(&self.state);
        let summary = Arc::clone(&self.summary);
        let record_path = Arc::clone(&self.record_path);
        let stop = Arc::clone(&self.stop);
        let handle = thread::spawn(move || {
            let result = (|| -> CoreResult<()> {
                let mut port = serialport::new(&device.system_path, 1_500_000)
                    .timeout(Duration::from_millis(20))
                    .open()
                    .map_err(|error| CoreError::from_serial_open(&device.system_path, error))?;
                port.clear(serialport::ClearBuffer::Input)?;
                let mut writer = RecordWriter::create(&records_dir)?;
                *record_path.write() = Some(writer.l0_path.clone());
                let active = CaptureState {
                    status: CaptureStatus::Capturing,
                    device_id: Some(device_id.clone()),
                    record_path: Some(writer.l0_path.to_string_lossy().into_owned()),
                    error: None,
                };
                *state.write() = active.clone();
                app.emit("capture-state-changed", &active).ok();
                let mut parser = FrameParser::default();
                let mut last_summary = Instant::now();
                let mut last_ready = Instant::now();
                let (sender, receiver) = crossbeam_channel::bounded::<ReaderMessage>(64);
                let reader_stop = Arc::clone(&stop);
                let reader = thread::spawn(move || {
                    let mut bytes = vec![0u8; 65_536];
                    while !reader_stop.load(Ordering::Relaxed) {
                        match port.read(&mut bytes) {
                            Ok(count) if count > 0 => {
                                if sender
                                    .send(ReaderMessage::Data(bytes[..count].to_vec()))
                                    .is_err()
                                {
                                    break;
                                }
                            }
                            Ok(_) => {}
                            Err(error) if error.kind() == std::io::ErrorKind::TimedOut => {}
                            Err(error) => {
                                let _ = sender.send(ReaderMessage::Error(error.to_string()));
                                break;
                            }
                        }
                    }
                });

                let processing_result = loop {
                    if stop.load(Ordering::Relaxed) {
                        break Ok(());
                    }
                    match receiver.recv_timeout(Duration::from_millis(50)) {
                        Ok(ReaderMessage::Data(bytes)) => {
                            let samples = parser.push(&bytes);
                            if !samples.is_empty() {
                                if let Err(error) = writer.push_samples(&samples) {
                                    break Err(error);
                                }
                                *summary.write() = writer.summary();
                                if last_ready.elapsed() >= Duration::from_millis(33) {
                                    app.emit("capture-data-ready", ()).ok();
                                    last_ready = Instant::now();
                                }
                                if last_summary.elapsed() >= Duration::from_millis(100) {
                                    app.emit("capture-summary-updated", writer.summary()).ok();
                                    last_summary = Instant::now();
                                }
                            }
                        }
                        Ok(ReaderMessage::Error(error)) => break Err(CoreError::Serial(error)),
                        Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
                        Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break Ok(()),
                    }
                };
                stop.store(true, Ordering::SeqCst);
                reader
                    .join()
                    .map_err(|_| CoreError::Other("serial reader thread panicked".into()))?;
                let final_summary = writer.finish()?;
                *summary.write() = final_summary.clone();
                app.emit("capture-summary-updated", final_summary).ok();
                processing_result
            })();
            let next = match result {
                Ok(()) => CaptureState {
                    status: CaptureStatus::Idle,
                    device_id: None,
                    record_path: record_path
                        .read()
                        .as_ref()
                        .map(|path| path.to_string_lossy().into_owned()),
                    error: None,
                },
                Err(error) => {
                    let app_error = crate::error::AppError::from(error);
                    app.emit("device-disconnected", &app_error).ok();
                    CaptureState {
                        status: CaptureStatus::Error,
                        device_id: Some(device_id),
                        record_path: record_path
                            .read()
                            .as_ref()
                            .map(|path| path.to_string_lossy().into_owned()),
                        error: Some(app_error.message),
                    }
                }
            };
            *state.write() = next.clone();
            app.emit("capture-state-changed", next).ok();
        });
        *self.thread.lock() = Some(handle);
        Ok(connecting)
    }

    pub fn stop(&self, app: &AppHandle) -> CoreResult<CaptureState> {
        if !self.is_running() {
            return Ok(self.state());
        }
        let stopping = CaptureState {
            status: CaptureStatus::Stopping,
            ..self.state()
        };
        *self.state.write() = stopping.clone();
        app.emit("capture-state-changed", &stopping).ok();
        self.stop.store(true, Ordering::SeqCst);
        if let Some(handle) = self.thread.lock().take() {
            handle
                .join()
                .map_err(|_| CoreError::Other("capture thread panicked".into()))?;
        }
        Ok(self.state())
    }

    pub fn import(&self, path: &Path) -> CoreResult<CaptureState> {
        if self.is_running() {
            return Err(CoreError::DeviceBusy);
        }
        storage::validate_l0(path)?;
        *self.record_path.write() = Some(path.to_path_buf());
        *self.summary.write() = storage::recording_summary(path, None)?;
        let next = CaptureState {
            status: CaptureStatus::Idle,
            device_id: None,
            record_path: Some(path.to_string_lossy().into_owned()),
            error: None,
        };
        *self.state.write() = next.clone();
        Ok(next)
    }

    pub fn clear(&self, records_dir: &Path) -> CoreResult<()> {
        if self.is_running() {
            return Err(CoreError::DeviceBusy);
        }
        if let Some(path) = self.record_path.write().take() {
            if path.starts_with(records_dir) {
                storage::clear_record_family(&path)?;
            }
        }
        *self.summary.write() = CaptureSummary::default();
        *self.state.write() = CaptureState::default();
        Ok(())
    }
}

impl Drop for CaptureManager {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(handle) = self.thread.get_mut().take() {
            let _ = handle.join();
        }
    }
}
