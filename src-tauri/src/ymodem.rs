use std::{
    fs::File,
    io::Read,
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use serialport::SerialPort;

use crate::error::{CoreError, CoreResult};

const SOH: u8 = 0x01;
const STX: u8 = 0x02;
const EOT: u8 = 0x04;
const ACK: u8 = 0x06;
const NAK: u8 = 0x15;
const CAN: u8 = 0x18;
const CRC_C: u8 = b'C';

pub fn crc16(data: &[u8]) -> u16 {
    let mut crc = 0u16;
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

fn packet(sequence: u8, data: &[u8], one_k: bool) -> Vec<u8> {
    let mut output = Vec::with_capacity(data.len() + 5);
    output.extend([if one_k { STX } else { SOH }, sequence, 0xff - sequence]);
    output.extend_from_slice(data);
    output.extend(crc16(data).to_be_bytes());
    output
}

fn wait_for(
    port: &mut dyn SerialPort,
    expected: u8,
    timeout: Duration,
    max_garbage: usize,
    cancel: &AtomicBool,
) -> CoreResult<()> {
    let started = Instant::now();
    let mut garbage = 0;
    let mut byte = [0u8; 1];
    while started.elapsed() < timeout {
        if cancel.load(Ordering::Relaxed) {
            let _ = port.write_all(&[CAN, CAN]);
            return Err(CoreError::Cancelled);
        }
        match port.read(&mut byte) {
            Ok(1) if byte[0] == expected => return Ok(()),
            Ok(1) => {
                garbage += 1;
                if garbage > max_garbage {
                    return Err(CoreError::Other(format!(
                        "received too much unexpected data while waiting for 0x{expected:02x}"
                    )));
                }
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::TimedOut => {}
            Err(error) => return Err(CoreError::Serial(error.to_string())),
        }
    }
    Err(CoreError::Timeout(format!("waiting for 0x{expected:02x}")))
}

pub fn send_file(
    port: &mut dyn SerialPort,
    path: &Path,
    cancel: Arc<AtomicBool>,
    mut progress: impl FnMut(u64, u64),
) -> CoreResult<()> {
    let file_size = path.metadata()?.len();
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| CoreError::Other("invalid firmware filename".into()))?;
    port.clear(serialport::ClearBuffer::Input)?;
    wait_for(port, CRC_C, Duration::from_secs(60), 500, &cancel)?;
    let mut header = vec![0u8; 128];
    let metadata = format!("{file_name}\0{file_size}");
    if metadata.len() > header.len() {
        return Err(CoreError::Other(
            "firmware filename is too long for YMODEM".into(),
        ));
    }
    header[..metadata.len()].copy_from_slice(metadata.as_bytes());
    port.write_all(&packet(0, &header, false))?;
    wait_for(port, ACK, Duration::from_secs(10), 100, &cancel)?;
    wait_for(port, CRC_C, Duration::from_secs(10), 100, &cancel)?;

    let mut file = File::open(path)?;
    let mut sequence = 1u8;
    let mut sent = 0u64;
    loop {
        if cancel.load(Ordering::Relaxed) {
            port.write_all(&[CAN, CAN])?;
            return Err(CoreError::Cancelled);
        }
        let mut block = vec![0x1a; 1024];
        let count = file.read(&mut block)?;
        if count == 0 {
            break;
        }
        port.write_all(&packet(sequence, &block, true))?;
        wait_for(port, ACK, Duration::from_secs(10), 50, &cancel)?;
        sent = (sent + count as u64).min(file_size);
        progress(sent, file_size);
        sequence = sequence.wrapping_add(1);
    }
    port.write_all(&[EOT])?;
    let mut response = [0u8; 1];
    let started = Instant::now();
    loop {
        if started.elapsed() > Duration::from_secs(3) {
            return Err(CoreError::Timeout("waiting for EOT response".into()));
        }
        match port.read(&mut response) {
            Ok(1) => break,
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::TimedOut => {}
            Err(error) => return Err(CoreError::Serial(error.to_string())),
        }
    }
    if response[0] == NAK {
        port.write_all(&[EOT])?;
        wait_for(port, ACK, Duration::from_secs(10), 50, &cancel)?;
    } else if response[0] != ACK {
        return Err(CoreError::Other(format!(
            "unexpected EOT response 0x{:02x}",
            response[0]
        )));
    }
    wait_for(port, CRC_C, Duration::from_secs(10), 50, &cancel)?;
    port.write_all(&packet(0, &[0u8; 128], false))?;
    wait_for(port, ACK, Duration::from_secs(10), 50, &cancel)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn crc_matches_xmodem_reference() {
        assert_eq!(crc16(b"123456789"), 0x31c3);
    }
    #[test]
    fn packet_contains_complement_and_big_endian_crc() {
        let block = [0x42u8; 128];
        let bytes = packet(7, &block, false);
        assert_eq!(&bytes[..3], &[SOH, 7, 248]);
        assert_eq!(
            u16::from_be_bytes(bytes[131..133].try_into().unwrap()),
            crc16(&block)
        );
    }
}
