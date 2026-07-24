use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use parking_lot::Mutex;
use semver::Version;
use serde::Deserialize;
use tauri::{AppHandle, Emitter};

use crate::{
    error::{CoreError, CoreResult},
    settings::SettingsStore,
    types::{ClientUpdateInfo, FirmwareDownloadProgress, OfficialFirmwareInfo},
};

pub const FIRMWARE_INFO_URL: &str =
    "https://no-chicken.com/resources/Power-Pico/firmware_version.json";
pub const RELEASE_NOTES_URL: &str =
    "https://no-chicken.com/content/Power-Pico/others/release_note.html";
pub const HELP_URL: &str = "https://no-chicken.com/content/Power-Pico/intro.html";
pub const FEEDBACK_URL: &str = "https://no-chicken.com/content/discuss_groups.html";
const GITHUB_LATEST_URL: &str = "https://github.com/haoyn231/power_pico_client/releases/latest";

#[derive(Deserialize)]
struct FirmwareResponse {
    version: String,
    #[serde(rename = "release date")]
    release_date: String,
    url: String,
}

fn client() -> CoreResult<reqwest::blocking::Client> {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(20))
        .user_agent("PowerPico-Client/0.1.1")
        .build()
        .map_err(|error| CoreError::Other(error.to_string()))
}

fn commit_download(temporary: &Path, target: &Path) -> CoreResult<()> {
    fs::rename(temporary, target)?;
    Ok(())
}

pub fn fetch_official_firmware_info(
    app_data: &Path,
    fallback_version: String,
    fallback_release_date: String,
) -> CoreResult<OfficialFirmwareInfo> {
    let path = app_data.join("PowerPico_Firmware.bin");
    let response = client()?
        .get(FIRMWARE_INFO_URL)
        .send()
        .and_then(reqwest::blocking::Response::error_for_status)
        .and_then(reqwest::blocking::Response::json::<FirmwareResponse>);
    let response = match response {
        Ok(response) => response,
        Err(_) if path.exists() => {
            return Ok(OfficialFirmwareInfo {
                version: fallback_version,
                release_date: fallback_release_date,
                url: String::new(),
                local_path: Some(path.to_string_lossy().into_owned()),
                downloaded: true,
            });
        }
        Err(error) => return Err(CoreError::Other(error.to_string())),
    };
    if !response.url.starts_with("https://") {
        return Err(CoreError::Other(
            "official firmware URL must use HTTPS".into(),
        ));
    }
    Ok(OfficialFirmwareInfo {
        version: response.version,
        release_date: response.release_date,
        url: response.url,
        local_path: path.exists().then(|| path.to_string_lossy().into_owned()),
        downloaded: path.exists(),
    })
}

pub fn check_client_update(current_version: &str) -> CoreResult<ClientUpdateInfo> {
    let response = client()?
        .get(GITHUB_LATEST_URL)
        .send()
        .and_then(reqwest::blocking::Response::error_for_status)
        .map_err(|error| CoreError::Other(error.to_string()))?;
    let release_url = response.url().to_string();
    let tag = response
        .url()
        .path_segments()
        .and_then(Iterator::last)
        .filter(|value| !value.is_empty() && *value != "latest")
        .ok_or_else(|| CoreError::Other("GitHub did not return a release tag".into()))?;
    let current = Version::parse(current_version.trim_start_matches('v'))
        .map_err(|error| CoreError::Other(error.to_string()))?;
    let latest = Version::parse(tag.trim_start_matches('v'))
        .map_err(|error| CoreError::Other(error.to_string()))?;
    Ok(ClientUpdateInfo {
        current_version: current.to_string(),
        latest_version: latest.to_string(),
        release_url,
        update_available: latest > current,
    })
}

pub struct DownloadManager {
    cancel: Arc<AtomicBool>,
    thread: Mutex<Option<JoinHandle<()>>>,
}

impl Default for DownloadManager {
    fn default() -> Self {
        Self {
            cancel: Arc::new(AtomicBool::new(false)),
            thread: Mutex::new(None),
        }
    }
}

impl DownloadManager {
    pub fn start(
        &self,
        app: AppHandle,
        url: String,
        version: String,
        release_date: String,
        app_data: PathBuf,
        settings: SettingsStore,
    ) -> CoreResult<()> {
        if self
            .thread
            .lock()
            .as_ref()
            .is_some_and(|handle| !handle.is_finished())
        {
            return Err(CoreError::DeviceBusy);
        }
        if !url.starts_with("https://") {
            return Err(CoreError::Other(
                "official firmware URL must use HTTPS".into(),
            ));
        }
        self.cancel.store(false, Ordering::SeqCst);
        let cancel = Arc::clone(&self.cancel);
        let handle = thread::spawn(move || {
            let emit = |percent, stage: &str, detail: Option<String>| {
                app.emit(
                    "firmware-download-progress",
                    FirmwareDownloadProgress {
                        percent,
                        stage: stage.into(),
                        detail,
                    },
                )
                .ok();
            };
            emit(0, "downloading", None);
            let temporary = app_data.join("PowerPico_Firmware.bin.part");
            let result = (|| -> CoreResult<()> {
                fs::create_dir_all(&app_data)?;
                let target = app_data.join("PowerPico_Firmware.bin");
                let mut response = client()?
                    .get(url)
                    .send()
                    .and_then(reqwest::blocking::Response::error_for_status)
                    .map_err(|error| CoreError::Other(error.to_string()))?;
                let total = response.content_length().unwrap_or(0);
                let mut file = File::create(&temporary)?;
                let mut buffer = [0u8; 64 * 1024];
                let mut received = 0u64;
                loop {
                    if cancel.load(Ordering::Relaxed) {
                        let _ = fs::remove_file(&temporary);
                        return Err(CoreError::Cancelled);
                    }
                    let count = response
                        .read(&mut buffer)
                        .map_err(|error| CoreError::Other(error.to_string()))?;
                    if count == 0 {
                        break;
                    }
                    file.write_all(&buffer[..count])?;
                    received += count as u64;
                    let percent = if total == 0 {
                        0
                    } else {
                        ((received.saturating_mul(100) / total).min(99)) as u8
                    };
                    emit(percent, "downloading", None);
                }
                file.flush()?;
                if received == 0 {
                    let _ = fs::remove_file(&temporary);
                    return Err(CoreError::Other("downloaded firmware is empty".into()));
                }
                commit_download(&temporary, &target)?;
                let mut next = settings.get();
                next.local_firmware_version = version;
                next.local_firmware_release_date = release_date;
                settings.update(next)?;
                Ok(())
            })();
            match result {
                Ok(()) => emit(100, "completed", None),
                Err(CoreError::Cancelled) => emit(0, "cancelled", None),
                Err(error) => {
                    let _ = fs::remove_file(&temporary);
                    emit(0, "failed", Some(error.to_string()));
                }
            }
        });
        *self.thread.lock() = Some(handle);
        Ok(())
    }

    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }
}

impl Drop for DownloadManager {
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
    fn semantic_versions_ignore_v_prefix() {
        let current = Version::parse("0.1.1").unwrap();
        let latest = Version::parse("v0.2.0".trim_start_matches('v')).unwrap();
        assert!(latest > current);
    }

    #[test]
    fn firmware_response_accepts_legacy_release_date_field() {
        let value: FirmwareResponse = serde_json::from_str(
            r#"{"version":"1.2.3","release date":"2026-01-01","url":"https://example.com/fw.bin"}"#,
        )
        .unwrap();
        assert_eq!(value.release_date, "2026-01-01");
    }

    #[test]
    fn downloaded_firmware_replaces_the_target_atomically() {
        let dir = tempfile::tempdir().unwrap();
        let temporary = dir.path().join("firmware.part");
        let target = dir.path().join("firmware.bin");
        fs::write(&temporary, b"new").unwrap();
        fs::write(&target, b"old").unwrap();
        commit_download(&temporary, &target).unwrap();
        assert_eq!(fs::read(&target).unwrap(), b"new");
        assert!(!temporary.exists());
    }
}
