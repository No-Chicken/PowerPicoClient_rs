//! PowerPico V1 calibration protocol, sharing the ADC stream with acquisition.
use crate::events::AppHandle;
use crate::{
    device,
    error::{CoreError, CoreResult},
};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    io::{Read, Write},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

pub const SCALES: [f64; 3] = [0.29296875, 29.296875, 2929.6875];
const SAMPLE_COUNT: usize = 20_000;
const GET: u8 = 0x10;
const RANGE_GET: u8 = 0x11;
const RANGE_SET: u8 = 0x12;
const SET: u8 = 0x13;
const RESET: u8 = 0x14;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Values {
    pub gains: [f64; 3],
    pub offsets_ua: [f64; 3],
}
impl Default for Values {
    fn default() -> Self {
        Self {
            gains: [1.; 3],
            offsets_ua: [0.; 3],
        }
    }
}
impl Values {
    fn payload(self) -> Vec<u8> {
        (0..3)
            .flat_map(|i| {
                [
                    (self.gains[i] as f32).to_le_bytes(),
                    (self.offsets_ua[i] as f32).to_le_bytes(),
                ]
                .concat()
            })
            .collect()
    }
    fn decode(payload: &[u8]) -> CoreResult<Self> {
        if payload.len() != 25 {
            return Err(problem("invalid calibration response length"));
        }
        let mut values = Self::default();
        for i in 0..3 {
            values.gains[i] =
                f32::from_le_bytes(payload[1 + i * 8..5 + i * 8].try_into().unwrap()) as f64;
            values.offsets_ua[i] =
                f32::from_le_bytes(payload[5 + i * 8..9 + i * 8].try_into().unwrap()) as f64;
        }
        if values
            .gains
            .iter()
            .chain(values.offsets_ua.iter())
            .any(|v| !v.is_finite())
        {
            return Err(problem("device returned non-finite calibration values"));
        }
        Ok(values)
    }
    fn close(self, other: Self) -> bool {
        self.gains
            .iter()
            .chain(self.offsets_ua.iter())
            .zip(other.gains.iter().chain(other.offsets_ua.iter()))
            .all(|(a, b)| (a - b).abs() <= 1e-5_f64.max(1e-5 * a.abs().max(b.abs())))
    }
}
fn problem(message: impl Into<String>) -> CoreError {
    CoreError::Other(message.into())
}
pub fn validate_range(index: usize, gain: f64, offset: f64) -> CoreResult<()> {
    if index >= 3 || !gain.is_finite() || !offset.is_finite() || !(0.85..=1.15).contains(&gain) {
        return Err(problem(
            "Gain must be finite and within [0.85, 1.15]; range must be 1–3.",
        ));
    }
    let zero = -offset / (gain * SCALES[index]);
    if zero.abs() > 48. {
        return Err(problem(format!(
            "Range {}: offset {offset:.6} µA = {zero:.6} LSB exceeds ±48 LSB (maximum ±{:.6} µA).",
            index + 1,
            gain * SCALES[index] * 48.
        )));
    }
    Ok(())
}

pub fn crc16(data: &[u8]) -> u16 {
    let mut crc = 0xffff;
    for byte in data {
        crc ^= (*byte as u16) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 {
                (crc << 1) ^ 0x1021
            } else {
                crc << 1
            };
        }
    }
    crc
}
fn request(command: u8, sequence: u16, payload: &[u8]) -> Vec<u8> {
    let mut data = vec![0xa5, 0x5a, 1, command];
    data.extend(sequence.to_le_bytes());
    data.extend((payload.len() as u16).to_le_bytes());
    data.extend(payload);
    data.extend(crc16(&data[2..]).to_le_bytes());
    data
}
#[derive(Debug)]
pub enum Frame {
    Adc(Vec<u8>),
    Control {
        version: u8,
        kind: u8,
        sequence: u16,
        payload: Vec<u8>,
    },
}
#[derive(Default)]
pub struct MixedParser {
    buffer: Vec<u8>,
}
impl MixedParser {
    pub fn push(&mut self, bytes: &[u8]) -> Vec<Frame> {
        self.buffer.extend(bytes);
        let mut frames = Vec::new();
        loop {
            let Some(index) = self
                .buffer
                .windows(2)
                .position(|w| w == [0xaa, 0x55] || w == [0xa5, 0x5a])
            else {
                let keep = usize::from(matches!(self.buffer.last(), Some(0xaa | 0xa5)));
                self.buffer.drain(..self.buffer.len().saturating_sub(keep));
                break;
            };
            self.buffer.drain(..index);
            let control = self.buffer[0] == 0xa5;
            let header_len = if control { 8 } else { 11 };
            if self.buffer.len() < header_len {
                break;
            }
            let length = if control {
                u16::from_le_bytes([self.buffer[6], self.buffer[7]]) as usize
            } else {
                self.buffer[10] as usize
            };
            if (control && length > 48) || (!control && !(1..=100).contains(&length)) {
                self.buffer.drain(..1);
                continue;
            }
            let total = if control {
                10 + length
            } else {
                11 + 7 * length
            };
            if self.buffer.len() < total {
                break;
            }
            if control
                && crc16(&self.buffer[2..total - 2])
                    != u16::from_le_bytes([self.buffer[total - 2], self.buffer[total - 1]])
            {
                self.buffer.drain(..1);
                continue;
            }
            let raw: Vec<_> = self.buffer.drain(..total).collect();
            frames.push(if control {
                Frame::Control {
                    version: raw[2],
                    kind: raw[3],
                    sequence: u16::from_le_bytes([raw[4], raw[5]]),
                    payload: raw[8..total - 2].to_vec(),
                }
            } else {
                Frame::Adc(raw)
            });
        }
        frames
    }
}

pub struct Protocol<P> {
    port: P,
    parser: MixedParser,
    adc: VecDeque<Vec<u8>>,
    sequence: u16,
    collecting: bool,
}
impl<P: Read + Write> Protocol<P> {
    pub fn new(port: P) -> Self {
        Self {
            port,
            parser: MixedParser::default(),
            adc: VecDeque::new(),
            sequence: 0,
            collecting: false,
        }
    }
    fn read(&mut self) -> CoreResult<Vec<Frame>> {
        let mut bytes = [0; 8192];
        let count = match self.port.read(&mut bytes) {
            Ok(n) => n,
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                ) =>
            {
                0
            }
            Err(e) => return Err(e.into()),
        };
        let frames = self.parser.push(&bytes[..count]);
        if self.collecting {
            for f in &frames {
                if let Frame::Adc(raw) = f {
                    self.adc.push_back(raw.clone());
                }
            }
        }
        Ok(frames)
    }
    fn transact(&mut self, command: u8, payload: &[u8]) -> CoreResult<Vec<u8>> {
        self.sequence = self.sequence.wrapping_add(1);
        let seq = self.sequence;
        let data = request(command, seq, payload);
        for _ in 0..3 {
            self.port.write_all(&data)?;
            let start = Instant::now();
            while start.elapsed() < Duration::from_millis(500) {
                for frame in self.read()? {
                    if let Frame::Control {
                        version,
                        kind,
                        sequence,
                        payload,
                    } = frame
                    {
                        if kind != command | 0x80 || sequence != seq {
                            continue;
                        }
                        if version != 1 || payload.is_empty() {
                            return Err(problem("invalid control response"));
                        }
                        if payload[0] != 0 {
                            return Err(problem(format!(
                                "Device status {}: {}",
                                payload[0],
                                match payload[0] {
                                    1 => "unsupported protocol version",
                                    2 => "unsupported command",
                                    3 => "invalid length",
                                    4 => "invalid parameter",
                                    5 => "EEPROM error",
                                    6 => "busy",
                                    7 => "CRC error",
                                    _ => "unknown status",
                                }
                            )));
                        }
                        return Ok(payload);
                    }
                }
            }
        }
        Err(CoreError::Timeout(format!(
            "waiting for calibration command 0x{command:02x}; check firmware support and connection"
        )))
    }
    pub fn read_values(&mut self) -> CoreResult<Values> {
        Values::decode(&self.transact(GET, &[])?)
    }
    fn range(&mut self, command: u8, payload: &[u8]) -> CoreResult<u8> {
        let reply = self.transact(command, payload)?;
        if reply.len() != 3 || reply[1] > 3 || !(1..=3).contains(&reply[2]) {
            return Err(problem("invalid range response"));
        }
        if command == RANGE_SET
            && (reply[1] != payload[0] || (payload[0] != 0 && reply[2] != payload[0]))
        {
            return Err(problem("range readback mismatch"));
        }
        Ok(reply[1])
    }
    fn write_values(&mut self, values: Values) -> CoreResult<()> {
        let saved = Values::decode(&self.transact(SET, &values.payload())?)?;
        if !saved.close(values) {
            return Err(problem("saved calibration response mismatch"));
        }
        Ok(())
    }
    fn verify_values(&mut self, values: Values) -> CoreResult<Values> {
        let verified = self.read_values()?;
        if !verified.close(values) {
            return Err(problem("calibration readback mismatch"));
        }
        Ok(verified)
    }
    fn save(&mut self, values: Values) -> CoreResult<Values> {
        self.write_values(values)?;
        self.verify_values(values)
    }
    fn collect(
        &mut self,
        range: u8,
        cancel: &AtomicBool,
        progress: &mut impl FnMut(&str, usize, Option<ZeroStatistics>),
    ) -> CoreResult<ZeroStatistics> {
        self.adc.clear();
        self.collecting = false;
        let start = Instant::now();
        while start.elapsed() < Duration::from_millis(500) {
            if cancel.load(Ordering::Relaxed) {
                return Err(CoreError::Cancelled);
            }
            self.read()?;
        }
        self.collecting = true;
        let result = (|| {
            let start = Instant::now();
            let mut samples = Vec::with_capacity(SAMPLE_COUNT);
            let mut last_progress = 0;
            while samples.len() < SAMPLE_COUNT {
                if cancel.load(Ordering::Relaxed) {
                    return Err(CoreError::Cancelled);
                }
                if start.elapsed() > Duration::from_secs(4) {
                    return Err(CoreError::Timeout(format!(
                        "collecting range {range}: {}/20000 samples",
                        samples.len()
                    )));
                }
                if self.adc.is_empty() {
                    self.read()?;
                }
                while let Some(raw) = self.adc.pop_front() {
                    for s in raw[11..].as_chunks::<7>().0 {
                        if s[0] != range {
                            continue;
                        }
                        let current = u16::from_le_bytes([s[3], s[4]]);
                        let reference = u16::from_le_bytes([s[5], s[6]]);
                        if !(1998..=2098).contains(&reference) || !(16..=4079).contains(&current) {
                            return Err(problem(format!(
                                "Range {range}: invalid ADC current={current}, reference={reference}; check wiring and disconnect the load."
                            )));
                        }
                        samples.push(current as i32 - reference as i32);
                        if samples.len() == SAMPLE_COUNT {
                            break;
                        }
                    }
                    if samples.len() >= SAMPLE_COUNT {
                        break;
                    }
                }
                if samples.len() >= last_progress + 1000 {
                    last_progress = samples.len();
                    progress(
                        &format!("range:{range}"),
                        (range as usize - 1) * SAMPLE_COUNT + samples.len(),
                        None,
                    );
                }
            }
            estimate_zero(&samples, range)
        })();
        self.collecting = false;
        self.adc.clear();
        result
    }
    fn set_range_values(&mut self, range: u8, gain: f64, offset: f64) -> Result<Values, Failure> {
        let index = range.wrapping_sub(1) as usize;
        validate_range(index, gain, offset).map_err(Failure::from)?;
        let mut next = self.read_values().map_err(Failure::from)?;
        next.gains[index] = gain;
        next.offsets_ua[index] = offset;
        self.save(next).map_err(|error| Failure {
            possibly_committed: true,
            ..Failure::from(error)
        })
    }
    fn reset_factory(&mut self) -> Result<Values, Failure> {
        (|| {
            let saved = Values::decode(&self.transact(RESET, &[0xc3, 0x3c])?)?;
            if !saved.close(Values::default()) {
                return Err(problem("factory reset response mismatch"));
            }
            let verified = self.read_values()?;
            if !verified.close(saved) {
                return Err(problem("factory reset readback mismatch"));
            }
            Ok(verified)
        })()
        .map_err(|error| Failure {
            possibly_committed: true,
            ..Failure::from(error)
        })
    }
    fn auto_zero(
        &mut self,
        cancel: &AtomicBool,
        progress: &mut impl FnMut(&str, usize, Option<ZeroStatistics>),
    ) -> Result<Values, Failure> {
        let initial = self.read_values().map_err(Failure::from)?;
        let original = self.range(RANGE_GET, &[]).map_err(Failure::from)?;
        let mut committed = false;
        let result = (|| -> CoreResult<Values> {
            let mut next = initial;
            for (i, scale) in SCALES.iter().enumerate() {
                validate_range(i, initial.gains[i], 0.)?;
                if cancel.load(Ordering::Relaxed) {
                    return Err(CoreError::Cancelled);
                }
                let stage = format!("range:{}", i + 1);
                progress(&stage, i * SAMPLE_COUNT, None);
                self.range(RANGE_SET, &[i as u8 + 1])?;
                let stats = self.collect(i as u8 + 1, cancel, progress)?;
                next.offsets_ua[i] = -initial.gains[i] * scale * stats.zero_lsb;
                progress(&stage, (i + 1) * SAMPLE_COUNT, Some(stats));
            }
            if cancel.load(Ordering::Relaxed) {
                return Err(CoreError::Cancelled);
            }
            progress("commit", 60000, None);
            committed = true;
            self.write_values(next)?;
            progress("verify", 60000, None);
            self.verify_values(next)
        })();
        progress("restore", 60000, None);
        let restore = self.range(RANGE_SET, &[original]);
        match (result, restore) {
            (Ok(saved), Ok(_)) => Ok(saved),
            (Ok(saved), Err(error)) => Err(Failure {
                message: format!(
                    "Calibration saved and verified, but range restoration failed: {error}"
                ),
                possibly_committed: false,
                cancelled: false,
                values: Some(saved),
            }),
            (Err(error), restore) => {
                let mut failure = Failure::from(error);
                failure.possibly_committed = committed;
                if let Err(e) = restore {
                    failure.cancelled = false;
                    failure
                        .message
                        .push_str(&format!("; original range restoration failed: {e}"));
                }
                Err(failure)
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ZeroStatistics {
    pub range: u8,
    pub sample_count: usize,
    pub zero_lsb: f64,
    pub mean_lsb: f64,
    pub noise_lsb: f64,
    pub minimum_lsb: i32,
    pub maximum_lsb: i32,
}
fn estimate_zero(samples: &[i32], range: u8) -> CoreResult<ZeroStatistics> {
    if samples.len() != SAMPLE_COUNT {
        return Err(problem("incomplete zero calibration samples"));
    }
    let mut blocks: Vec<f64> = samples
        .as_chunks::<1000>()
        .0
        .iter()
        .map(|b| b.iter().map(|v| *v as f64).sum::<f64>() / 1000.)
        .collect();
    let mean = blocks.iter().sum::<f64>() / 20.;
    let noise = (blocks.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / 20.).sqrt();
    blocks.sort_by(f64::total_cmp);
    let zero = (blocks[9] + blocks[10]) / 2.;
    tracing::info!(range, zero, noise, mean, "zero calibration statistics");
    if noise > 5. || zero.abs() > 32. {
        return Err(problem(format!(
            "Range {range}: zero={zero:.6} LSB (limit ±32), noise={noise:.6} LSB (limit 5). Disconnect the load and check wiring/interference."
        )));
    }
    Ok(ZeroStatistics {
        range,
        sample_count: samples.len(),
        zero_lsb: zero,
        mean_lsb: mean,
        noise_lsb: noise,
        minimum_lsb: *samples.iter().min().unwrap(),
        maximum_lsb: *samples.iter().max().unwrap(),
    })
}
#[derive(Debug)]
struct Failure {
    message: String,
    possibly_committed: bool,
    cancelled: bool,
    values: Option<Values>,
}
impl From<CoreError> for Failure {
    fn from(error: CoreError) -> Self {
        Self {
            cancelled: matches!(error, CoreError::Cancelled),
            message: error.to_string(),
            possibly_committed: false,
            values: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CalibrationState {
    pub device_id: Option<String>,
    pub connected: bool,
    pub busy: bool,
    pub stage: String,
    pub samples: usize,
    pub values: Option<Values>,
    pub statistics: Vec<ZeroStatistics>,
    pub error: Option<String>,
    pub possibly_committed: bool,
}
type SerialProtocol = Protocol<Box<dyn serialport::SerialPort>>;

#[derive(Default)]
pub struct CalibrationManager {
    port: Arc<Mutex<Option<SerialProtocol>>>,
    state: Arc<Mutex<CalibrationState>>,
    cancel: Arc<AtomicBool>,
    worker: Mutex<Option<thread::JoinHandle<()>>>,
}
impl CalibrationManager {
    pub fn state(&self) -> CalibrationState {
        self.state.lock().clone()
    }
    pub fn connected(&self) -> bool {
        self.state.lock().connected
    }
    pub fn connect(&self, id: String) -> CoreResult<CalibrationState> {
        if self.connected() {
            return Err(CoreError::DeviceBusy);
        }
        let device = device::find_device(&id)?;
        let port = serialport::new(&device.system_path, 1_500_000)
            .timeout(Duration::from_millis(20))
            .open()
            .map_err(|e| CoreError::from_serial_open(&device.system_path, e))?;
        port.clear(serialport::ClearBuffer::Input)?;
        *self.port.lock() = Some(Protocol::new(port));
        *self.state.lock() = CalibrationState {
            connected: true,
            device_id: Some(id),
            stage: "connected".into(),
            ..Default::default()
        };
        Ok(self.state())
    }
    pub fn disconnect(&self) -> CoreResult<CalibrationState> {
        if self.state.lock().busy {
            return Err(CoreError::DeviceBusy);
        }
        self.port.lock().take();
        *self.state.lock() = CalibrationState::default();
        Ok(self.state())
    }
    pub fn shutdown(&self) {
        self.cancel.store(true, Ordering::SeqCst);
        if let Some(worker) = self.worker.lock().take() {
            let _ = worker.join();
        }
        self.port.lock().take();
    }
    pub fn cancel(&self) {
        let state = self.state.lock();
        if state.busy && state.stage.starts_with("range:") {
            self.cancel.store(true, Ordering::SeqCst);
        }
    }
    pub fn start(
        &self,
        app: AppHandle,
        operation: String,
        range: Option<u8>,
        gain: Option<f64>,
        offset: Option<f64>,
    ) -> CoreResult<()> {
        {
            let mut state = self.state.lock();
            if !state.connected || state.busy {
                return Err(CoreError::DeviceBusy);
            }
            if !["read", "manual", "auto", "reset"].contains(&operation.as_str()) {
                return Err(problem("invalid calibration operation"));
            }
            if (operation == "manual" || operation == "reset") && state.values.is_none() {
                return Err(problem("Read calibration parameters before editing."));
            }
            if operation == "manual" {
                validate_range(
                    range.unwrap_or(0).wrapping_sub(1) as usize,
                    gain.unwrap_or(f64::NAN),
                    offset.unwrap_or(f64::NAN),
                )?;
            }
            state.busy = true;
            state.stage = "start".into();
            state.samples = 0;
            state.error = None;
            state.possibly_committed = false;
            state.statistics.clear();
        }
        self.cancel.store(false, Ordering::SeqCst);
        let state = Arc::clone(&self.state);
        let port = Arc::clone(&self.port);
        let cancel = Arc::clone(&self.cancel);
        let handle = thread::spawn(move || {
            let emit = || {
                app.emit("calibration-state", state.lock().clone());
            };
            emit();
            let mut guard = port.lock();
            let result = if let Some(protocol) = guard.as_mut() {
                match operation.as_str() {
                    "auto" => protocol.auto_zero(&cancel, &mut |stage, samples, stats| {
                        {
                            let mut s = state.lock();
                            s.stage = stage.into();
                            s.samples = samples;
                            if let Some(v) = stats {
                                s.statistics.push(v);
                            }
                        }
                        emit();
                    }),
                    "read" => protocol.read_values().map_err(Failure::from),
                    "manual" => {
                        protocol.set_range_values(range.unwrap(), gain.unwrap(), offset.unwrap())
                    }
                    "reset" => protocol.reset_factory(),
                    _ => unreachable!(),
                }
            } else {
                Err(Failure::from(problem("calibration disconnected")))
            };
            {
                let mut s = state.lock();
                s.busy = false;
                match result {
                    Ok(values) => {
                        s.values = Some(values);
                        s.stage = "completed".into();
                    }
                    Err(failure) => {
                        tracing::warn!("calibration {operation}: {}", failure.message);
                        s.stage = if failure.cancelled {
                            "cancelled"
                        } else if failure.values.is_some() {
                            "warning"
                        } else {
                            "failed"
                        }
                        .into();
                        s.error = (!failure.cancelled).then_some(failure.message);
                        s.possibly_committed = failure.possibly_committed;
                        if let Some(values) = failure.values {
                            s.values = Some(values);
                        } else if !failure.cancelled {
                            s.values = None;
                            s.connected = false;
                            guard.take();
                        }
                    }
                }
            }
            drop(guard);
            emit();
        });
        *self.worker.lock() = Some(handle);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    fn adc(range: u8, delta: i32) -> Vec<u8> {
        let mut raw = vec![0xaa, 0x55];
        raw.extend(123_u64.to_le_bytes());
        raw.push(100);
        for _ in 0..100 {
            raw.push(range);
            raw.extend(0_u16.to_le_bytes());
            raw.extend(((2048 + delta) as u16).to_le_bytes());
            raw.extend(2048_u16.to_le_bytes());
        }
        raw
    }
    struct Fake {
        values: Values,
        range: u8,
        pending: VecDeque<u8>,
        writes: Vec<Vec<u8>>,
        delta: i32,
        mismatch: bool,
        drop_reply: bool,
        restore_error: bool,
        initial_range: u8,
    }
    impl Default for Fake {
        fn default() -> Self {
            Self {
                values: Values {
                    gains: [1.1, 0.9, 1.05],
                    offsets_ua: [0.; 3],
                },
                range: 0,
                pending: VecDeque::new(),
                writes: Vec::new(),
                delta: 2,
                mismatch: false,
                drop_reply: false,
                restore_error: false,
                initial_range: 0,
            }
        }
    }
    impl Write for Fake {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.writes.push(bytes.to_vec());
            let cmd = bytes[3];
            let seq = u16::from_le_bytes([bytes[4], bytes[5]]);
            let mut payload = vec![0];
            match cmd {
                GET => payload.extend(self.values.payload()),
                RANGE_GET => payload.extend([self.range, self.range.max(1)]),
                RANGE_SET => {
                    self.range = bytes[8];
                    if self.restore_error && self.range == self.initial_range {
                        payload = vec![5];
                    } else {
                        payload.extend([self.range, self.range.max(1)]);
                    }
                }
                SET => {
                    let mut p = vec![0];
                    p.extend(&bytes[8..32]);
                    self.values = Values::decode(&p).unwrap();
                    if self.mismatch {
                        self.values.offsets_ua[0] += 1.;
                    }
                    payload.extend(self.values.payload());
                }
                RESET => {
                    assert_eq!(&bytes[8..10], &[0xc3, 0x3c]);
                    self.values = Values::default();
                    payload.extend(self.values.payload());
                }
                _ => panic!("unexpected command"),
            }
            if !self.drop_reply {
                self.pending.extend(request(cmd | 0x80, seq, &payload));
            }
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    impl Read for Fake {
        fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
            if self.pending.is_empty() {
                self.pending.extend(adc(self.range.max(1), self.delta));
            }
            let n = output.len().min(self.pending.len()).min(173);
            for b in &mut output[..n] {
                *b = self.pending.pop_front().unwrap();
            }
            Ok(n)
        }
    }
    #[test]
    fn wire_matches_reference_vectors() {
        assert_eq!(crc16(b"123456789"), 0x29b1);
        assert_eq!(
            request(GET, 0x1234, &[]),
            [0xa5, 0x5a, 1, 0x10, 0x34, 0x12, 0, 0, 0xf1, 0x84]
        );
        let values = Values {
            gains: [1., 1.25, 0.5],
            offsets_ua: [-2., 3., -4.],
        };
        assert_eq!(
            request(SET, 42, &values.payload()),
            [
                0xa5, 0x5a, 1, 0x13, 42, 0, 24, 0, 0, 0, 128, 63, 0, 0, 0, 192, 0, 0, 160, 63, 0,
                0, 64, 64, 0, 0, 0, 63, 0, 0, 128, 192, 0xa2, 0xe7
            ]
        );
    }
    #[test]
    fn mixed_parser_recovers_noise_crc_invalid_length_and_all_fragment_sizes() {
        let control = request(GET | 0x80, 42, &[0]);
        let mut corrupt = control.clone();
        corrupt[10] ^= 1;
        let mut stream = vec![0, 3, 0xa5, 0x5a, 1, 0x10, 0, 0, 49, 0];
        stream.extend(corrupt);
        stream.extend(adc(1, 2));
        stream.extend(&control);
        for size in 1..=stream.len() {
            let mut parser = MixedParser::default();
            let mut frames = Vec::new();
            for chunk in stream.chunks(size) {
                frames.extend(parser.push(chunk));
            }
            assert_eq!(frames.len(), 2, "fragment size {size}");
            assert!(matches!(frames[0], Frame::Adc(_)));
            assert!(matches!(frames[1], Frame::Control { sequence: 42, .. }));
        }
    }
    #[test]
    fn manual_limits_include_endpoints_and_reject_nonfinite_or_invalid_range() {
        for (i, scale) in SCALES.iter().enumerate() {
            for gain in [0.85, 1., 1.15] {
                assert!(validate_range(i, gain, gain * scale * 48.).is_ok());
                assert!(validate_range(i, gain, gain * scale * 48.01).is_err());
            }
        }
        for gain in [f64::NAN, f64::INFINITY, 0.849, 1.151] {
            assert!(validate_range(0, gain, 0.).is_err());
        }
        assert!(validate_range(3, 1., 0.).is_err());
        assert!(validate_range(0, 1., f64::NAN).is_err());
    }
    #[test]
    fn robust_zero_and_noise_limits_match_reference() {
        let mut samples = vec![2; SAMPLE_COUNT];
        samples[4000..5000].fill(12);
        let stats = estimate_zero(&samples, 1).unwrap();
        assert_eq!(stats.zero_lsb, 2.);
        assert!(stats.noise_lsb > 0.);
        assert!(estimate_zero(&[33; SAMPLE_COUNT], 1).is_err());
        assert!(estimate_zero(&[32; SAMPLE_COUNT], 1).is_ok());
        let noisy: Vec<_> = (0..SAMPLE_COUNT)
            .map(|i| if i / 1000 % 2 == 0 { -6 } else { 6 })
            .collect();
        assert!(estimate_zero(&noisy, 1).is_err());
        assert!(estimate_zero(&samples[..19999], 1).is_err());
    }
    #[test]
    fn manual_preserves_other_ranges_and_reset_verifies_defaults() {
        let mut protocol = Protocol::new(Fake::default());
        let initial = protocol.read_values().unwrap();
        let saved = protocol.set_range_values(3, 1.02, 1000.).unwrap();
        assert_eq!(saved.gains[0], initial.gains[0]);
        assert_eq!(saved.gains[1], initial.gains[1]);
        assert_eq!(saved.offsets_ua[2], 1000.);
        assert!(protocol.reset_factory().unwrap().close(Values::default()));
        let count = protocol.port.writes.len();
        assert!(protocol.set_range_values(1, 5., 0.).is_err());
        assert_eq!(count, protocol.port.writes.len());
    }
    #[test]
    fn auto_zero_preserves_gains_uses_raw_adc_and_restores_mode() {
        let mut protocol = Protocol::new(Fake::default());
        let initial = protocol.read_values().unwrap();
        let cancel = AtomicBool::new(false);
        let mut statistics = Vec::new();
        let saved = protocol
            .auto_zero(&cancel, &mut |_, _, stats| {
                if let Some(s) = stats {
                    statistics.push(s)
                }
            })
            .unwrap();
        assert_eq!(saved.gains, initial.gains);
        assert_eq!(statistics.len(), 3);
        for i in 0..3 {
            assert!((saved.offsets_ua[i] + initial.gains[i] * SCALES[i] * 2.).abs() < 0.01);
            assert_eq!(statistics[i].sample_count, 20000);
        }
        assert_eq!(protocol.port.range, 0);
        assert_eq!(
            protocol.port.writes.iter().filter(|w| w[3] == SET).count(),
            1
        );
    }
    #[test]
    fn cancellation_and_unsafe_zero_never_write_and_restore_original_range() {
        for unsafe_zero in [false, true] {
            let mut protocol = Protocol::new(Fake {
                range: 2,
                initial_range: 2,
                delta: if unsafe_zero { 33 } else { 2 },
                ..Default::default()
            });
            let cancel = AtomicBool::new(false);
            let failure = protocol
                .auto_zero(&cancel, &mut |_, _, stats| {
                    if !unsafe_zero && stats.is_some() {
                        cancel.store(true, Ordering::SeqCst)
                    }
                })
                .unwrap_err();
            assert_eq!(failure.cancelled, !unsafe_zero);
            assert!(!failure.possibly_committed);
            assert_eq!(protocol.port.range, 2);
            assert!(!protocol.port.writes.iter().any(|w| w[3] == SET));
        }
    }
    #[test]
    fn restoration_failure_after_cancellation_is_reported_as_failure() {
        let mut protocol = Protocol::new(Fake {
            restore_error: true,
            ..Default::default()
        });
        let failure = protocol
            .auto_zero(&AtomicBool::new(true), &mut |_, _, _| {})
            .unwrap_err();
        assert!(!failure.cancelled);
        assert!(!failure.possibly_committed);
        assert!(failure.message.contains("restoration failed"));
        assert!(!protocol.port.writes.iter().any(|w| w[3] == SET));
    }

    #[test]
    fn mismatch_after_commit_is_uncertain_and_restores_range() {
        let mut protocol = Protocol::new(Fake {
            mismatch: true,
            ..Default::default()
        });
        let failure = protocol
            .auto_zero(&AtomicBool::new(false), &mut |_, _, _| {})
            .unwrap_err();
        assert!(failure.possibly_committed);
        assert_eq!(protocol.port.range, 0);
    }
    #[test]
    fn saved_calibration_is_returned_when_only_restore_fails() {
        let mut protocol = Protocol::new(Fake {
            restore_error: true,
            ..Default::default()
        });
        let failure = protocol
            .auto_zero(&AtomicBool::new(false), &mut |_, _, _| {})
            .unwrap_err();
        assert!(failure.values.is_some());
        assert!(!failure.possibly_committed);
    }
    #[test]
    fn timeout_retries_same_sequence_and_reports_firmware_support() {
        let mut protocol = Protocol::new(Fake {
            drop_reply: true,
            ..Default::default()
        });
        assert!(matches!(protocol.read_values(), Err(CoreError::Timeout(_))));
        assert_eq!(protocol.port.writes.len(), 3);
        assert!(protocol.port.writes.windows(2).all(|w| w[0] == w[1]));
    }

    /// Explicit opt-in: run only after disconnecting the load. Saves the original
    /// parameters before any write; leaves the final automatic calibration saved.
    #[test]
    #[ignore = "requires POWERPICO_CALIBRATION_PORT and a confirmed zero-current device"]
    fn hardware_calibration_smoke() {
        let path = std::env::var("POWERPICO_CALIBRATION_PORT").expect("serial port path");
        let report = std::env::var("POWERPICO_CALIBRATION_REPORT").expect("backup/report path");
        let port = serialport::new(path, 1_500_000)
            .timeout(Duration::from_millis(20))
            .open()
            .unwrap();
        port.clear(serialport::ClearBuffer::Input).unwrap();
        let mut protocol = Protocol::new(port);
        let original = protocol.read_values().unwrap();
        let original_range = protocol.range(RANGE_GET, &[]).unwrap();
        std::fs::write(
            &report,
            serde_json::to_string_pretty(
                &serde_json::json!({"original":original,"originalRange":original_range}),
            )
            .unwrap(),
        )
        .unwrap();
        let test_result = || {
            let manual = protocol
                .set_range_values(1, original.gains[0], original.offsets_ua[0] + 0.01)
                .unwrap();
            assert!((manual.offsets_ua[0] - original.offsets_ua[0] - 0.01).abs() < 1e-4);
            assert_eq!(manual.gains[1..], original.gains[1..]);
            protocol.save(original).unwrap();
            assert!(protocol.reset_factory().unwrap().close(Values::default()));
            protocol.save(original).unwrap();
            let cancel = AtomicBool::new(false);
            let cancelled = protocol
                .auto_zero(&cancel, &mut |_, _, stats| {
                    if stats.is_some() {
                        cancel.store(true, Ordering::SeqCst)
                    }
                })
                .unwrap_err();
            assert!(cancelled.cancelled);
            assert!(!cancelled.possibly_committed);
            assert!(protocol.read_values().unwrap().close(original));
            assert_eq!(protocol.range(RANGE_GET, &[]).unwrap(), original_range);
            let mut statistics = Vec::new();
            let final_values = protocol
                .auto_zero(&AtomicBool::new(false), &mut |stage, samples, stats| {
                    println!("{stage}: {samples}/60000");
                    if let Some(stats) = stats {
                        statistics.push(stats);
                    }
                })
                .unwrap();
            assert_eq!(final_values.gains, original.gains);
            assert_eq!(protocol.range(RANGE_GET, &[]).unwrap(), original_range);
            std::fs::write(&report,serde_json::to_string_pretty(&serde_json::json!({"original":original,"originalRange":original_range,"final":final_values,"statistics":statistics,"checks":["read","manual preserves other ranges","factory reset","restore backup","cancel without writing","auto zero","readback verification","original range restored"]})).unwrap()).unwrap();
        };
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(test_result));
        if let Err(error) = result {
            let _ = protocol.save(original);
            let _ = protocol.range(RANGE_SET, &[original_range]);
            std::panic::resume_unwind(error);
        }
    }
}
