//! Hardware check for the firmware and capture paths, without the window.
//!
//! ```text
//! cargo run -p powerpico-app --example firmware -- list
//! cargo run -p powerpico-app --example firmware -- download
//! cargo run -p powerpico-app --example firmware -- flash <file.bin> [device id]
//! cargo run -p powerpico-app --example firmware -- probe [seconds] [device id]
//! ```

use std::{
    path::PathBuf,
    sync::mpsc,
    time::{Duration, Instant},
};

use powerpico_core::{AppPaths, AppState, Events, SerialDevice};

fn main() {
    let mut args = std::env::args().skip(1);
    let command = args.next().unwrap_or_else(|| "list".into());
    let rest: Vec<String> = args.collect();

    let paths = AppPaths::resolve().expect("resolve app paths");
    paths.prepare().expect("prepare app paths");
    let (tx, rx) = mpsc::channel::<(&'static str, serde_json::Value)>();
    let events = Events::new(move |name, value| {
        let _ = tx.send((name, value));
    });
    let state = AppState::new(paths, events).expect("start core");

    match command.as_str() {
        "list" => list(&state),
        "download" => download(&state, &rx),
        "flash" => {
            let file = rest
                .first()
                .map(PathBuf::from)
                .expect("usage: flash <file.bin> [device id]");
            flash(&state, &rx, file, rest.get(1).cloned());
        }
        "probe" => {
            let seconds = rest
                .first()
                .and_then(|value| value.parse::<u64>().ok())
                .unwrap_or(3);
            probe(&state, seconds, rest.get(1).cloned());
        }
        other => {
            eprintln!("unknown command: {other}");
            std::process::exit(2);
        }
    }
}

fn list(state: &AppState) {
    let devices = state.list_serial_devices().expect("list devices");
    for device in &devices {
        println!(
            "{}  vid={:04x} pid={:04x} serial={} path={}",
            device.id,
            device.vid.unwrap_or_default(),
            device.pid.unwrap_or_default(),
            device.serial_number.as_deref().unwrap_or("-"),
            device.system_path,
        );
    }
    if devices.is_empty() {
        println!("no serial devices");
    }
}

fn pick(state: &AppState, wanted: Option<String>) -> SerialDevice {
    let devices = state.list_serial_devices().expect("list devices");
    if let Some(id) = wanted {
        return devices
            .into_iter()
            .find(|device| device.id == id)
            .unwrap_or_else(|| panic!("device {id} not found"));
    }
    let mut candidates: Vec<SerialDevice> = devices
        .into_iter()
        .filter(|device| device.vid == Some(0x0483) && device.pid == Some(0x5740))
        .collect();
    match candidates.len() {
        1 => candidates.remove(0),
        0 => panic!("no PowerPico (0483:5740) device; pass an id explicitly"),
        _ => panic!("several PowerPico devices; pass an id explicitly"),
    }
}

fn download(state: &AppState, rx: &mpsc::Receiver<(&'static str, serde_json::Value)>) {
    let info = state.official_firmware_info().expect("firmware info");
    println!(
        "official {} ({}) downloaded={}",
        info.version, info.release_date, info.downloaded
    );
    if info.downloaded {
        println!("path={}", info.local_path.unwrap_or_default());
        return;
    }
    state
        .download_official_firmware(
            info.url.clone(),
            info.version.clone(),
            info.release_date.clone(),
        )
        .expect("start download");
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(180) {
        match rx.recv_timeout(Duration::from_secs(5)) {
            Ok(("firmware-download-progress", value)) => {
                let percent = value.get("percent").and_then(|v| v.as_u64()).unwrap_or(0);
                let stage = value
                    .get("stage")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?")
                    .to_string();
                println!("download {percent}% {stage}");
                if stage == "completed" {
                    let info = state.official_firmware_info().expect("firmware info");
                    println!("path={}", info.local_path.unwrap_or_default());
                    return;
                }
                if stage == "failed" || stage == "cancelled" {
                    panic!("download {stage}");
                }
            }
            Ok(_) => {}
            Err(_) => {}
        }
    }
    panic!("download timed out");
}

fn flash(
    state: &AppState,
    rx: &mpsc::Receiver<(&'static str, serde_json::Value)>,
    file: PathBuf,
    wanted: Option<String>,
) {
    let metadata = std::fs::metadata(&file).expect("firmware file");
    println!("firmware {} bytes: {}", metadata.len(), file.display());
    let device = pick(state, wanted);
    println!("device {} -> {}", device.id, device.system_path);

    state
        .flash_firmware(device.id.clone(), file)
        .expect("start flash");

    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(180) {
        match rx.recv_timeout(Duration::from_secs(5)) {
            Ok(("firmware-progress", value)) => {
                let percent = value.get("percent").and_then(|v| v.as_u64()).unwrap_or(0);
                let stage = value
                    .get("messageKey")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?")
                    .to_string();
                let detail = value
                    .get("detail")
                    .and_then(|v| v.as_str())
                    .map(str::to_string);
                println!(
                    "flash {percent}% {stage}{}",
                    detail.map(|d| format!(": {d}")).unwrap_or_default()
                );
            }
            Ok(("firmware-finished", value)) => {
                let stage = value
                    .get("stage")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?")
                    .to_string();
                println!("finished {stage}");
                if stage != "completed" {
                    std::process::exit(1);
                }
                return;
            }
            Ok(_) => {}
            Err(_) => {}
        }
    }
    panic!("flash timed out");
}

fn probe(state: &AppState, seconds: u64, wanted: Option<String>) {
    let device = pick(state, wanted);
    println!("capture on {}", device.system_path);
    state.start_capture(device.id).expect("start capture");
    std::thread::sleep(Duration::from_secs(seconds));
    state.stop_capture().expect("stop capture");
    let summary = state.stats(None).expect("summary");
    println!(
        "points={} duration={:.3}s voltage={:.3}V current={:.1}uA",
        summary.point_count, summary.duration, summary.latest_voltage, summary.latest_current
    );
    if summary.point_count == 0 {
        std::process::exit(1);
    }
}
