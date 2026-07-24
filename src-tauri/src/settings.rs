use crate::{
    error::{CoreError, CoreResult},
    types::AppSettings,
};
use parking_lot::RwLock;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub struct SettingsStore {
    path: PathBuf,
    value: RwLock<AppSettings>,
}

impl SettingsStore {
    pub fn load(config_dir: &Path) -> CoreResult<Self> {
        fs::create_dir_all(config_dir)?;
        let path = config_dir.join("settings.json");
        let value = if path.exists() {
            serde_json::from_slice(&fs::read(&path)?)
                .map_err(|error| CoreError::Config(error.to_string()))?
        } else {
            AppSettings::default()
        }
        .sanitized();
        Ok(Self {
            path,
            value: RwLock::new(value),
        })
    }
    pub fn get(&self) -> AppSettings {
        self.value.read().clone()
    }
    pub fn update(&self, settings: AppSettings) -> CoreResult<AppSettings> {
        let settings = settings.sanitized();
        let json = serde_json::to_vec_pretty(&settings)
            .map_err(|error| CoreError::Config(error.to_string()))?;
        fs::write(&self.path, json)?;
        *self.value.write() = settings.clone();
        Ok(settings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{MetricId, ThemeMode};
    use tempfile::tempdir;

    #[test]
    fn migrates_missing_metrics_and_filters_unknown_or_duplicate_values() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("settings.json"),
            r#"{"theme":"dark","language":"en","waveformMetrics":["latestVoltage","latestVoltage","futureMetric"]}"#,
        )
        .unwrap();
        let store = SettingsStore::load(dir.path()).unwrap();
        assert_eq!(store.get().theme, ThemeMode::Dark);
        assert_eq!(store.get().waveform_metrics, vec![MetricId::LatestVoltage]);

        fs::write(
            dir.path().join("settings.json"),
            r#"{"theme":"system","language":"zh-CN"}"#,
        )
        .unwrap();
        let migrated = SettingsStore::load(dir.path()).unwrap().get();
        assert_eq!(
            migrated.waveform_metrics,
            AppSettings::default().waveform_metrics
        );
    }
}
