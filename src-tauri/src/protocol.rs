use crate::types::Sample;

const HEADER: [u8; 2] = [0xAA, 0x55];
const META_SIZE: usize = 11;
const SAMPLE_SIZE: usize = 7;
const SAMPLE_INTERVAL_US: f64 = 100.0;
const VOLT_RES_HIGH: f32 = 1_000_000.0;
const VOLT_RES_LOW: f32 = 100_000.0;
const VOLT_FACTOR: f32 = 3.0 / 4096.0 / (VOLT_RES_LOW / (VOLT_RES_HIGH + VOLT_RES_LOW));
const MAX_INPUT_VOLTAGE: f32 = 22.0;
const MAX_VOLT_ADC: u16 = (MAX_INPUT_VOLTAGE / VOLT_FACTOR) as u16;

#[derive(Default)]
pub struct FrameParser {
    buffer: Vec<u8>,
    base_timestamp_us: Option<u64>,
}

impl FrameParser {
    pub fn push(&mut self, bytes: &[u8]) -> Vec<Sample> {
        self.buffer.extend_from_slice(bytes);
        let mut output = Vec::new();

        loop {
            if self.buffer.len() < META_SIZE + SAMPLE_SIZE {
                break;
            }
            let Some(header_index) = self.buffer.windows(2).position(|window| window == HEADER)
            else {
                let keep = usize::from(self.buffer.last() == Some(&HEADER[0]));
                let tail = self.buffer.len().saturating_sub(keep);
                self.buffer.drain(..tail);
                break;
            };
            if header_index > 0 {
                self.buffer.drain(..header_index);
            }
            if self.buffer.len() < META_SIZE {
                break;
            }

            let timestamp_us = u64::from_le_bytes(
                self.buffer[2..10]
                    .try_into()
                    .expect("fixed timestamp slice"),
            );
            let count = self.buffer[10] as usize;
            if count == 0 {
                self.buffer.drain(..1);
                continue;
            }
            let frame_len = META_SIZE + count * SAMPLE_SIZE;
            if self.buffer.len() < frame_len {
                break;
            }
            let frame: Vec<u8> = self.buffer.drain(..frame_len).collect();
            let base = *self.base_timestamp_us.get_or_insert(timestamp_us);

            for index in 0..count {
                let offset = META_SIZE + index * SAMPLE_SIZE;
                let range = frame[offset];
                let voltage_adc = u16::from_le_bytes([frame[offset + 1], frame[offset + 2]]);
                let current_adc = u16::from_le_bytes([frame[offset + 3], frame[offset + 4]]);
                let reference_adc = u16::from_le_bytes([frame[offset + 5], frame[offset + 6]]);
                if !(1..=3).contains(&range)
                    || !(1998..=2098).contains(&reference_adc)
                    || voltage_adc > MAX_VOLT_ADC
                    || current_adc > 4096
                {
                    continue;
                }

                let resistance = match range {
                    1 => 50.0,
                    2 => 0.5,
                    3 => 0.005,
                    _ => unreachable!(),
                };
                let current_factor = (3.0 / 4096.0 / 50.0 / resistance) * 1_000_000.0;
                let voltage = voltage_adc as f32 * VOLT_FACTOR;
                let current = (current_adc as f32 - reference_adc as f32) * current_factor
                    - voltage / (VOLT_RES_HIGH + VOLT_RES_LOW) * 1_000_000.0;
                let time = ((timestamp_us - base) as f64 + index as f64 * SAMPLE_INTERVAL_US)
                    / 1_000_000.0;
                output.push(Sample {
                    time,
                    voltage,
                    current,
                });
            }
        }
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(timestamp: u64, points: &[(u8, u16, u16, u16)]) -> Vec<u8> {
        let mut bytes = vec![0xAA, 0x55];
        bytes.extend(timestamp.to_le_bytes());
        bytes.push(points.len() as u8);
        for (range, voltage, current, reference) in points {
            bytes.push(*range);
            bytes.extend(voltage.to_le_bytes());
            bytes.extend(current.to_le_bytes());
            bytes.extend(reference.to_le_bytes());
        }
        bytes
    }

    #[test]
    fn parses_fragmented_and_misaligned_frames() {
        let bytes = frame(1000, &[(1, 1000, 2050, 2048), (2, 2000, 2052, 2048)]);
        let mut parser = FrameParser::default();
        assert!(parser.push(&[0, 1, 2, bytes[0]]).is_empty());
        let mut rest = vec![bytes[1]];
        rest.extend_from_slice(&bytes[2..]);
        let samples = parser.push(&rest);
        assert_eq!(samples.len(), 2);
        assert_eq!(samples[0].time, 0.0);
        assert!((samples[1].time - 0.0001).abs() < 1e-9);
    }

    #[test]
    fn rejects_invalid_measurements() {
        let bytes = frame(
            1000,
            &[
                (0, 1000, 2050, 2048),
                (1, 1000, 5000, 2048),
                (1, 1000, 2050, 1800),
            ],
        );
        assert!(FrameParser::default().push(&bytes).is_empty());
    }
}
