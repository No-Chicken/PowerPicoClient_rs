use std::{
    fs,
    path::{Path, PathBuf},
};

#[cfg(target_os = "macos")]
use std::process::Command;

use crate::{
    error::{CoreError, CoreResult},
    types::{StorageUsage, UninstallInfo},
};

pub const BUNDLE_ID: &str = "com.openfeasttech.powerpico-client";
#[cfg(target_os = "macos")]
const APP_NAME: &str = "PowerPico Client.app";

#[derive(Clone)]
pub struct MaintenancePaths {
    pub app_data: PathBuf,
    pub cache: PathBuf,
    pub logs: PathBuf,
    pub home: PathBuf,
}

impl MaintenancePaths {
    fn system_data(&self) -> Vec<PathBuf> {
        let library = self.home.join("Library");
        vec![
            library
                .join("Preferences")
                .join(format!("{BUNDLE_ID}.plist")),
            library
                .join("Saved Application State")
                .join(format!("{BUNDLE_ID}.savedState")),
            library.join("WebKit").join(BUNDLE_ID),
            library.join("HTTPStorages").join(BUNDLE_ID),
        ]
    }

    #[cfg(any(target_os = "macos", test))]
    pub fn all_data(&self) -> Vec<PathBuf> {
        let mut paths = vec![self.app_data.clone(), self.cache.clone(), self.logs.clone()];
        paths.extend(self.system_data());
        paths
    }
}

pub fn path_size(path: &Path) -> CoreResult<u64> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(error.into()),
    };
    if metadata.file_type().is_symlink() || metadata.is_file() {
        return Ok(metadata.len());
    }
    if !metadata.is_dir() {
        return Ok(0);
    }
    let mut total = 0u64;
    for entry in fs::read_dir(path)? {
        total = total.saturating_add(path_size(&entry?.path())?);
    }
    Ok(total)
}

pub fn storage_usage(paths: &MaintenancePaths) -> CoreResult<StorageUsage> {
    let cache_bytes = path_size(&paths.cache)?;
    let app_data_bytes = path_size(&paths.app_data)?;
    let log_bytes = path_size(&paths.logs)?;
    Ok(StorageUsage {
        cache_bytes,
        app_data_bytes,
        log_bytes,
        total_bytes: cache_bytes
            .saturating_add(app_data_bytes)
            .saturating_add(log_bytes),
    })
}

pub fn clear_directory_contents(path: &Path) -> CoreResult<()> {
    if !path.exists() {
        fs::create_dir_all(path)?;
        return Ok(());
    }
    for entry in fs::read_dir(path)? {
        let child = entry?.path();
        let metadata = fs::symlink_metadata(&child)?;
        if metadata.is_dir() && !metadata.file_type().is_symlink() {
            fs::remove_dir_all(child)?;
        } else {
            fs::remove_file(child)?;
        }
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn bundle_identifier(app: &Path) -> CoreResult<String> {
    let info = app.join("Contents/Info.plist");
    let output = Command::new("/usr/bin/plutil")
        .args(["-extract", "CFBundleIdentifier", "raw", "-o", "-"])
        .arg(info)
        .output()?;
    if !output.status.success() {
        return Err(CoreError::Other(
            "unable to read application bundle identifier".into(),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

#[cfg(target_os = "macos")]
pub fn current_app_bundle() -> CoreResult<Option<PathBuf>> {
    let executable = std::env::current_exe()?;
    for ancestor in executable.ancestors() {
        if ancestor.extension().is_some_and(|value| value == "app") {
            if bundle_identifier(ancestor)? != BUNDLE_ID {
                return Err(CoreError::Other(
                    "application bundle identifier mismatch".into(),
                ));
            }
            return Ok(Some(ancestor.to_path_buf()));
        }
    }
    Ok(None)
}

#[cfg(not(target_os = "macos"))]
pub fn current_app_bundle() -> CoreResult<Option<PathBuf>> {
    Ok(None)
}

#[cfg(target_os = "macos")]
fn mount_points_from_hdiutil(output: &str) -> Vec<PathBuf> {
    let mut volumes = Vec::new();
    for line in output.lines() {
        let Some(index) = line.find("/Volumes/") else {
            continue;
        };
        volumes.push(PathBuf::from(line[index..].trim()));
    }
    volumes.sort();
    volumes.dedup();
    volumes
}

#[cfg(target_os = "macos")]
pub fn mounted_product_volumes() -> CoreResult<Vec<PathBuf>> {
    let output = Command::new("/usr/bin/hdiutil").arg("info").output()?;
    if !output.status.success() {
        return Err(CoreError::Other(
            "unable to inspect mounted disk images".into(),
        ));
    }
    let mut volumes = Vec::new();
    for mount in mount_points_from_hdiutil(&String::from_utf8_lossy(&output.stdout)) {
        let app = mount.join(APP_NAME);
        if app.is_dir() && bundle_identifier(&app).is_ok_and(|value| value == BUNDLE_ID) {
            volumes.push(mount);
        }
    }
    volumes.sort();
    volumes.dedup();
    Ok(volumes)
}

#[cfg(not(target_os = "macos"))]
pub fn mounted_product_volumes() -> CoreResult<Vec<PathBuf>> {
    Ok(Vec::new())
}

pub fn uninstall_info(paths: &MaintenancePaths) -> CoreResult<UninstallInfo> {
    let app = current_app_bundle()?;
    let app_bytes = app.as_deref().map(path_size).transpose()?.unwrap_or(0);
    let usage = storage_usage(paths)?;
    let system_data_bytes = paths.system_data().iter().try_fold(0u64, |total, path| {
        Ok::<_, CoreError>(total.saturating_add(path_size(path)?))
    })?;
    let mounted_volumes = mounted_product_volumes()?
        .into_iter()
        .map(|path| path.to_string_lossy().into_owned())
        .collect();
    Ok(UninstallInfo {
        supported: cfg!(target_os = "macos") && app.is_some(),
        app_path: app.map(|path| path.to_string_lossy().into_owned()),
        app_bytes,
        cache_bytes: usage.cache_bytes,
        app_data_bytes: usage.app_data_bytes,
        log_bytes: usage.log_bytes,
        system_data_bytes,
        total_bytes: app_bytes
            .saturating_add(usage.total_bytes)
            .saturating_add(system_data_bytes),
        mounted_volumes,
    })
}

#[cfg(target_os = "macos")]
pub fn begin_uninstall(paths: &MaintenancePaths) -> CoreResult<()> {
    let app = current_app_bundle()?.ok_or_else(|| {
        CoreError::Other("uninstall is only available from a packaged macOS app".into())
    })?;
    let volumes = mounted_product_volumes()?;
    let running_from_volume = app.starts_with("/Volumes");

    if !running_from_volume {
        let trash = paths.home.join(".Trash");
        fs::create_dir_all(&trash)?;
        let mut target = trash.join(APP_NAME);
        if target.exists() {
            target = trash.join(format!(
                "PowerPico Client-{}.app",
                chrono::Utc::now().timestamp()
            ));
        }
        fs::rename(&app, target).map_err(|error| {
            CoreError::Other(format!("unable to move application to Trash: {error}"))
        })?;
    }

    let helper = std::env::temp_dir().join(format!(
        "powerpico-uninstall-{}-{}.sh",
        std::process::id(),
        chrono::Utc::now().timestamp()
    ));
    fs::write(
        &helper,
        "#!/bin/sh\npid=\"$1\"\nshift\nwhile kill -0 \"$pid\" 2>/dev/null; do sleep 0.2; done\nwhile [ \"$1\" != \"--volumes\" ]; do rm -rf -- \"$1\"; shift; done\nshift\nfor volume in \"$@\"; do /usr/bin/hdiutil detach \"$volume\" >/dev/null 2>&1 || true; done\nrm -f -- \"$0\"\n",
    )?;
    let mut command = Command::new("/bin/sh");
    command.arg(&helper).arg(std::process::id().to_string());
    for path in paths.all_data() {
        command.arg(path);
    }
    command.arg("--volumes");
    for volume in volumes {
        command.arg(volume);
    }
    command.spawn()?;
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn begin_uninstall(_paths: &MaintenancePaths) -> CoreResult<()> {
    Err(CoreError::Other(
        "application uninstall is only supported on macOS".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_size_counts_files_without_following_symlinks() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a"), b"1234").unwrap();
        fs::create_dir(dir.path().join("nested")).unwrap();
        fs::write(dir.path().join("nested/b"), b"12").unwrap();
        assert_eq!(path_size(dir.path()).unwrap(), 6);
    }

    #[test]
    fn clear_contents_preserves_root() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("record.bin"), b"data").unwrap();
        clear_directory_contents(dir.path()).unwrap();
        assert!(dir.path().is_dir());
        assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
    }

    #[test]
    fn maintenance_paths_only_include_exact_bundle_identifier() {
        let paths = MaintenancePaths {
            app_data: PathBuf::from("/data/app"),
            cache: PathBuf::from("/cache/app"),
            logs: PathBuf::from("/logs/app"),
            home: PathBuf::from("/home/user"),
        };
        for path in paths.all_data().into_iter().skip(3) {
            assert!(path.to_string_lossy().contains(BUNDLE_ID));
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn parses_duplicate_disk_image_volume_names() {
        let output = "/dev/disk5s1 APFS /Volumes/PowerPico Client\n/dev/disk6s1 HFS /Volumes/PowerPico Client 1\n/dev/disk7s1 HFS /Volumes/Other\n";
        assert_eq!(
            mount_points_from_hdiutil(output),
            vec![
                PathBuf::from("/Volumes/Other"),
                PathBuf::from("/Volumes/PowerPico Client"),
                PathBuf::from("/Volumes/PowerPico Client 1"),
            ]
        );
    }
}
