use std::{
    fs::{self, File, OpenOptions},
    io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

use chrono::Local;

use crate::{
    error::{CoreError, CoreResult},
    types::{CaptureSummary, PointReading, RenderSeries, Sample},
};

pub const L0_SIZE: u64 = 16;
pub const LN_SIZE: u64 = 40;
const LEVEL_RATIO: usize = 100;
const MAX_LEVELS: usize = 5;

#[derive(Debug, Clone, Copy)]
struct Aggregate {
    start: f64,
    end: f64,
    v_min: f32,
    v_max: f32,
    v_avg: f32,
    c_min: f32,
    c_max: f32,
    c_avg: f32,
}

struct LevelWriter {
    writer: BufWriter<File>,
    buffer: Vec<Aggregate>,
}

pub struct RecordWriter {
    pub l0_path: PathBuf,
    l0: BufWriter<File>,
    levels: Vec<LevelWriter>,
    point_count: u64,
    voltage_sum: f64,
    current_sum: f64,
    power_sum_mw: f64,
    voltage_peak: f32,
    current_peak: f32,
    latest_voltage: f32,
    latest_current: f32,
    duration: f64,
}

impl RecordWriter {
    pub fn create(records_dir: &Path) -> CoreResult<Self> {
        fs::create_dir_all(records_dir)?;
        let timestamp = Local::now().timestamp();
        let l0_path = records_dir.join(format!("record_{timestamp}.bin"));
        let l0 = BufWriter::new(
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(&l0_path)?,
        );
        let mut levels = Vec::with_capacity(MAX_LEVELS);
        for level in 1..=MAX_LEVELS {
            let path = sibling_level_path(&l0_path, level);
            levels.push(LevelWriter {
                writer: BufWriter::new(OpenOptions::new().create(true).append(true).open(path)?),
                buffer: Vec::with_capacity(LEVEL_RATIO),
            });
        }
        Ok(Self {
            l0_path,
            l0,
            levels,
            point_count: 0,
            voltage_sum: 0.0,
            current_sum: 0.0,
            power_sum_mw: 0.0,
            voltage_peak: f32::NEG_INFINITY,
            current_peak: f32::NEG_INFINITY,
            latest_voltage: 0.0,
            latest_current: 0.0,
            duration: 0.0,
        })
    }

    pub fn push_samples(&mut self, samples: &[Sample]) -> CoreResult<()> {
        for sample in samples {
            write_f64(&mut self.l0, sample.time)?;
            write_f32(&mut self.l0, sample.voltage)?;
            write_f32(&mut self.l0, sample.current)?;
            self.point_count += 1;
            self.duration = self.duration.max(sample.time);
            self.voltage_sum += sample.voltage as f64;
            self.current_sum += sample.current as f64;
            self.power_sum_mw += sample.voltage as f64 * sample.current as f64 / 1000.0;
            self.voltage_peak = self.voltage_peak.max(sample.voltage);
            self.current_peak = self.current_peak.max(sample.current);
            self.latest_voltage = sample.voltage;
            self.latest_current = sample.current;
            self.push_aggregate(
                0,
                Aggregate {
                    start: sample.time,
                    end: sample.time,
                    v_min: sample.voltage,
                    v_max: sample.voltage,
                    v_avg: sample.voltage,
                    c_min: sample.current,
                    c_max: sample.current,
                    c_avg: sample.current,
                },
            )?;
        }
        if self.point_count % 1000 < samples.len() as u64 {
            self.l0.flush()?;
        }
        Ok(())
    }

    fn push_aggregate(&mut self, level_index: usize, aggregate: Aggregate) -> CoreResult<()> {
        if level_index >= self.levels.len() {
            return Ok(());
        }
        self.levels[level_index].buffer.push(aggregate);
        if self.levels[level_index].buffer.len() == LEVEL_RATIO {
            let combined = combine(&self.levels[level_index].buffer);
            write_aggregate(&mut self.levels[level_index].writer, combined)?;
            self.levels[level_index].writer.flush()?;
            self.levels[level_index].buffer.clear();
            self.push_aggregate(level_index + 1, combined)?;
        }
        Ok(())
    }

    pub fn summary(&self) -> CaptureSummary {
        if self.point_count == 0 {
            return CaptureSummary::default();
        }
        let count = self.point_count as f64;
        CaptureSummary {
            point_count: self.point_count,
            duration: self.duration,
            latest_voltage: self.latest_voltage as f64,
            voltage_average: self.voltage_sum / count,
            voltage_peak: self.voltage_peak as f64,
            latest_current: self.latest_current as f64,
            current_average: self.current_sum / count,
            current_peak: self.current_peak as f64,
            latest_power_mw: self.latest_voltage as f64 * self.latest_current as f64 / 1000.0,
            power_average_mw: self.power_sum_mw / count,
            energy_mah: (self.current_sum / count) * self.duration / 3_600_000.0,
        }
    }

    pub fn finish(mut self) -> CoreResult<CaptureSummary> {
        self.l0.flush()?;
        for level in &mut self.levels {
            if !level.buffer.is_empty() {
                let combined = combine(&level.buffer);
                write_aggregate(&mut level.writer, combined)?;
                level.buffer.clear();
            }
            level.writer.flush()?;
        }
        Ok(self.summary())
    }
}

fn combine(items: &[Aggregate]) -> Aggregate {
    let count = items.len() as f32;
    Aggregate {
        start: items.first().map_or(0.0, |item| item.start),
        end: items.last().map_or(0.0, |item| item.end),
        v_min: items
            .iter()
            .map(|item| item.v_min)
            .fold(f32::INFINITY, f32::min),
        v_max: items
            .iter()
            .map(|item| item.v_max)
            .fold(f32::NEG_INFINITY, f32::max),
        v_avg: items.iter().map(|item| item.v_avg).sum::<f32>() / count,
        c_min: items
            .iter()
            .map(|item| item.c_min)
            .fold(f32::INFINITY, f32::min),
        c_max: items
            .iter()
            .map(|item| item.c_max)
            .fold(f32::NEG_INFINITY, f32::max),
        c_avg: items.iter().map(|item| item.c_avg).sum::<f32>() / count,
    }
}

fn write_aggregate(writer: &mut BufWriter<File>, item: Aggregate) -> CoreResult<()> {
    write_f64(writer, item.start)?;
    write_f64(writer, item.end)?;
    for value in [
        item.v_min, item.v_max, item.v_avg, item.c_min, item.c_max, item.c_avg,
    ] {
        write_f32(writer, value)?;
    }
    Ok(())
}

fn write_f64(writer: &mut impl Write, value: f64) -> CoreResult<()> {
    writer.write_all(&value.to_le_bytes())?;
    Ok(())
}
fn write_f32(writer: &mut impl Write, value: f32) -> CoreResult<()> {
    writer.write_all(&value.to_le_bytes())?;
    Ok(())
}
fn read_f64(bytes: &[u8]) -> f64 {
    f64::from_le_bytes(bytes.try_into().expect("f64 slice"))
}
fn read_f32(bytes: &[u8]) -> f32 {
    f32::from_le_bytes(bytes.try_into().expect("f32 slice"))
}

pub fn sibling_level_path(l0_path: &Path, level: usize) -> PathBuf {
    let stem = l0_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("record");
    l0_path.with_file_name(format!("{stem}.L{level}.bin"))
}

pub fn validate_l0(path: &Path) -> CoreResult<()> {
    if path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.eq_ignore_ascii_case("bin"))
        != Some(true)
    {
        return Err(CoreError::InvalidRecording("expected a .bin file".into()));
    }
    let size = fs::metadata(path)?.len();
    if size == 0 || size % L0_SIZE != 0 {
        return Err(CoreError::InvalidRecording(format!(
            "file size {size} is not a multiple of {L0_SIZE}"
        )));
    }
    Ok(())
}

pub fn recording_summary(path: &Path, window_seconds: Option<f64>) -> CoreResult<CaptureSummary> {
    validate_l0(path)?;
    let mut file = File::open(path)?;
    let count = file.metadata()?.len() / L0_SIZE;
    let last = read_l0_at(&mut file, count - 1)?;
    let start_time = window_seconds.map_or(0.0, |window| (last.time - window).max(0.0));
    let start = lower_bound(&mut file, count, L0_SIZE, start_time, false)?;
    file.seek(SeekFrom::Start(start * L0_SIZE))?;
    let mut reader = BufReader::new(file);
    let mut buffer = [0u8; L0_SIZE as usize];
    let mut points = 0u64;
    let mut v_sum = 0.0;
    let mut c_sum = 0.0;
    let mut p_sum = 0.0;
    let mut v_peak = f32::NEG_INFINITY;
    let mut c_peak = f32::NEG_INFINITY;
    while reader.read_exact(&mut buffer).is_ok() {
        let sample = decode_l0(&buffer);
        points += 1;
        v_sum += sample.voltage as f64;
        c_sum += sample.current as f64;
        p_sum += sample.voltage as f64 * sample.current as f64 / 1000.0;
        v_peak = v_peak.max(sample.voltage);
        c_peak = c_peak.max(sample.current);
    }
    if points == 0 {
        return Ok(CaptureSummary::default());
    }
    let points_f = points as f64;
    let duration = last.time - start_time;
    Ok(CaptureSummary {
        point_count: points,
        duration: last.time,
        latest_voltage: last.voltage as f64,
        voltage_average: v_sum / points_f,
        voltage_peak: v_peak as f64,
        latest_current: last.current as f64,
        current_average: c_sum / points_f,
        current_peak: c_peak as f64,
        latest_power_mw: last.voltage as f64 * last.current as f64 / 1000.0,
        power_average_mw: p_sum / points_f,
        energy_mah: (c_sum / points_f) * duration / 3_600_000.0,
    })
}

pub fn point_at(path: &Path, time_seconds: f64) -> CoreResult<PointReading> {
    validate_l0(path)?;
    let mut file = File::open(path)?;
    let count = file.metadata()?.len() / L0_SIZE;
    if count == 0 {
        return Err(CoreError::NoRecording);
    }

    let mut low = 0u64;
    let mut high = count;
    while low < high {
        let mid = (low + high) / 2;
        let sample = read_l0_at(&mut file, mid)?;
        if sample.time < time_seconds {
            low = mid + 1;
        } else {
            high = mid;
        }
    }

    let after_index = low.min(count - 1);
    let after = read_l0_at(&mut file, after_index)?;
    let sample = if low == 0 {
        after
    } else if low >= count {
        read_l0_at(&mut file, count - 1)?
    } else {
        let before = read_l0_at(&mut file, low - 1)?;
        if (time_seconds - before.time).abs() <= (after.time - time_seconds).abs() {
            before
        } else {
            after
        }
    };

    Ok(PointReading {
        time: sample.time,
        voltage: sample.voltage as f64,
        current: sample.current as f64,
        power_mw: sample.voltage as f64 * sample.current as f64 / 1000.0,
    })
}

pub fn render_data(
    l0_path: &Path,
    start: f64,
    end: f64,
    pixel_width: usize,
) -> CoreResult<RenderSeries> {
    validate_l0(l0_path)?;
    let mut chosen = l0_path.to_path_buf();
    let mut level = 0usize;
    let raw_points = ((end - start).max(0.0) * 10_000.0) as usize;
    let mut density = raw_points / pixel_width.max(1);
    while density > 100 && level < MAX_LEVELS {
        let candidate = sibling_level_path(l0_path, level + 1);
        if !candidate.exists() || fs::metadata(&candidate)?.len() < LN_SIZE {
            break;
        }
        level += 1;
        density /= LEVEL_RATIO;
        chosen = candidate;
    }
    let (items, available_start, available_end) = if level == 0 {
        read_l0_range(&chosen, start, end)?
    } else {
        read_ln_range(&chosen, start, end)?
    };
    let mut series = RenderSeries {
        aggregated: level > 0,
        available_start,
        available_end,
        ..Default::default()
    };
    if items.is_empty() {
        return Ok(series);
    }
    let target = pixel_width.max(1) * 2;
    let chunk = items.len().div_ceil(target).max(1);
    for group in items.chunks(chunk) {
        let combined = combine(group);
        series.time.push(combined.start);
        series.voltage_min.push(combined.v_min);
        series.voltage_max.push(combined.v_max);
        series.voltage_average.push(combined.v_avg);
        series.current_min.push(combined.c_min);
        series.current_max.push(combined.c_max);
        series.current_average.push(combined.c_avg);
    }
    series.aggregated |= chunk > 1;
    Ok(series)
}

fn read_l0_range(path: &Path, start: f64, end: f64) -> CoreResult<(Vec<Aggregate>, f64, f64)> {
    let mut file = File::open(path)?;
    let count = file.metadata()?.len() / L0_SIZE;
    if count == 0 {
        return Ok((vec![], 0.0, 0.0));
    }
    let first = read_l0_at(&mut file, 0)?.time;
    let last = read_l0_at(&mut file, count - 1)?.time;
    let from = lower_bound(&mut file, count, L0_SIZE, start, false)?;
    let to = lower_bound(&mut file, count, L0_SIZE, end, false)?.min(count);
    file.seek(SeekFrom::Start(from * L0_SIZE))?;
    let mut output = Vec::with_capacity((to - from) as usize);
    let mut bytes = [0u8; L0_SIZE as usize];
    for _ in from..to {
        file.read_exact(&mut bytes)?;
        let sample = decode_l0(&bytes);
        output.push(Aggregate {
            start: sample.time,
            end: sample.time,
            v_min: sample.voltage,
            v_max: sample.voltage,
            v_avg: sample.voltage,
            c_min: sample.current,
            c_max: sample.current,
            c_avg: sample.current,
        });
    }
    Ok((output, first, last))
}

fn read_ln_range(path: &Path, start: f64, end: f64) -> CoreResult<(Vec<Aggregate>, f64, f64)> {
    let mut file = File::open(path)?;
    let count = file.metadata()?.len() / LN_SIZE;
    if count == 0 {
        return Ok((vec![], 0.0, 0.0));
    }
    let first = read_ln_at(&mut file, 0)?;
    let last = read_ln_at(&mut file, count - 1)?;
    let from = lower_bound(&mut file, count, LN_SIZE, start, true)?;
    let to = lower_bound(&mut file, count, LN_SIZE, end, true)?.min(count);
    file.seek(SeekFrom::Start(from * LN_SIZE))?;
    let mut output = Vec::with_capacity((to - from) as usize);
    let mut bytes = [0u8; LN_SIZE as usize];
    for _ in from..to {
        file.read_exact(&mut bytes)?;
        output.push(decode_ln(&bytes));
    }
    Ok((output, first.start, last.end))
}

fn lower_bound(file: &mut File, count: u64, size: u64, target: f64, _ln: bool) -> CoreResult<u64> {
    let mut low = 0;
    let mut high = count;
    while low < high {
        let mid = (low + high) / 2;
        file.seek(SeekFrom::Start(mid * size))?;
        let mut bytes = [0u8; 8];
        file.read_exact(&mut bytes)?;
        if read_f64(&bytes) < target {
            low = mid + 1;
        } else {
            high = mid;
        }
    }
    Ok(low.saturating_sub(1))
}

fn read_l0_at(file: &mut File, index: u64) -> CoreResult<Sample> {
    file.seek(SeekFrom::Start(index * L0_SIZE))?;
    let mut bytes = [0u8; L0_SIZE as usize];
    file.read_exact(&mut bytes)?;
    Ok(decode_l0(&bytes))
}
fn read_ln_at(file: &mut File, index: u64) -> CoreResult<Aggregate> {
    file.seek(SeekFrom::Start(index * LN_SIZE))?;
    let mut bytes = [0u8; LN_SIZE as usize];
    file.read_exact(&mut bytes)?;
    Ok(decode_ln(&bytes))
}
fn decode_l0(bytes: &[u8]) -> Sample {
    Sample {
        time: read_f64(&bytes[0..8]),
        voltage: read_f32(&bytes[8..12]),
        current: read_f32(&bytes[12..16]),
    }
}
fn decode_ln(bytes: &[u8]) -> Aggregate {
    Aggregate {
        start: read_f64(&bytes[0..8]),
        end: read_f64(&bytes[8..16]),
        v_min: read_f32(&bytes[16..20]),
        v_max: read_f32(&bytes[20..24]),
        v_avg: read_f32(&bytes[24..28]),
        c_min: read_f32(&bytes[28..32]),
        c_max: read_f32(&bytes[32..36]),
        c_avg: read_f32(&bytes[36..40]),
    }
}

pub fn export_recording(source: &Path, directory: &Path) -> CoreResult<Vec<String>> {
    validate_l0(source)?;
    fs::create_dir_all(directory)?;
    let stamp = Local::now().format("%Y%m%d%H%M%S");
    let base = format!("record_{stamp}");
    let output_l0 = directory.join(format!("{base}.bin"));
    fs::copy(source, &output_l0)?;
    let mut outputs = vec![output_l0.to_string_lossy().into_owned()];
    for level in 1..=MAX_LEVELS {
        let source_level = sibling_level_path(source, level);
        if source_level.exists() {
            let target = directory.join(format!("{base}.L{level}.bin"));
            fs::copy(source_level, &target)?;
            outputs.push(target.to_string_lossy().into_owned());
        }
    }
    let csv_path = directory.join(format!("{base}.csv"));
    export_csv(source, &csv_path)?;
    outputs.push(csv_path.to_string_lossy().into_owned());
    Ok(outputs)
}

fn export_csv(source: &Path, target: &Path) -> CoreResult<()> {
    let mut reader = BufReader::new(File::open(source)?);
    let mut writer = BufWriter::new(File::create(target)?);
    writer.write_all(b"Time(s),Voltage(V),Current(uA)\n")?;
    let mut bytes = [0u8; L0_SIZE as usize];
    while reader.read_exact(&mut bytes).is_ok() {
        let sample = decode_l0(&bytes);
        writeln!(
            writer,
            "{:.6},{:.4},{:.2}",
            sample.time, sample.voltage, sample.current
        )?;
    }
    writer.flush()?;
    Ok(())
}

pub fn clear_record_family(l0_path: &Path) -> CoreResult<()> {
    if l0_path.exists() {
        fs::remove_file(l0_path)?;
    }
    for level in 1..=MAX_LEVELS {
        let path = sibling_level_path(l0_path, level);
        if path.exists() {
            fs::remove_file(path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn writes_legacy_compatible_layout_and_queries_it() {
        let dir = tempdir().unwrap();
        let mut writer = RecordWriter::create(dir.path()).unwrap();
        let samples: Vec<_> = (0..250)
            .map(|i| Sample {
                time: i as f64 / 10_000.0,
                voltage: i as f32,
                current: (i * 2) as f32,
            })
            .collect();
        writer.push_samples(&samples).unwrap();
        let path = writer.l0_path.clone();
        writer.finish().unwrap();
        assert_eq!(
            fs::metadata(&path).unwrap().len(),
            samples.len() as u64 * L0_SIZE
        );
        let mut first_bytes = [0u8; L0_SIZE as usize];
        File::open(&path)
            .unwrap()
            .read_exact(&mut first_bytes)
            .unwrap();
        let mut golden = Vec::new();
        golden.extend(0.0f64.to_le_bytes());
        golden.extend(0.0f32.to_le_bytes());
        golden.extend(0.0f32.to_le_bytes());
        assert_eq!(first_bytes.as_slice(), golden.as_slice());
        assert_eq!(
            fs::metadata(sibling_level_path(&path, 1)).unwrap().len(),
            3 * LN_SIZE
        );
        let result = render_data(&path, 0.0, 1.0, 100).unwrap();
        assert!(!result.time.is_empty());
        let summary = recording_summary(&path, None).unwrap();
        assert_eq!(summary.point_count, 250);
        assert_eq!(summary.latest_voltage, 249.0);
        assert_eq!(summary.latest_current, 498.0);
        assert_eq!(summary.latest_power_mw, 124.002);
    }

    #[test]
    fn finds_the_nearest_original_point_with_stable_boundaries() {
        let dir = tempdir().unwrap();
        let mut writer = RecordWriter::create(dir.path()).unwrap();
        writer
            .push_samples(&[
                Sample {
                    time: 1.0,
                    voltage: 1.0,
                    current: 10.0,
                },
                Sample {
                    time: 2.0,
                    voltage: 2.0,
                    current: 20.0,
                },
                Sample {
                    time: 4.0,
                    voltage: 4.0,
                    current: 40.0,
                },
            ])
            .unwrap();
        let path = writer.l0_path.clone();
        writer.finish().unwrap();

        assert_eq!(point_at(&path, -1.0).unwrap().time, 1.0);
        assert_eq!(point_at(&path, 9.0).unwrap().time, 4.0);
        assert_eq!(point_at(&path, 3.0).unwrap().time, 2.0);
        let selected = point_at(&path, 3.1).unwrap();
        assert_eq!(selected.time, 4.0);
        assert_eq!(selected.power_mw, 0.16);
    }
}
