use serialport::SerialPortType;

use crate::{
    error::{CoreError, CoreResult},
    types::SerialDevice,
};

pub fn list_devices() -> CoreResult<Vec<SerialDevice>> {
    let mut devices: Vec<_> = serialport::available_ports()?
        .into_iter()
        .map(|port| {
            let (vid, pid, serial_number, manufacturer, product) = match port.port_type {
                SerialPortType::UsbPort(info) => (
                    Some(info.vid),
                    Some(info.pid),
                    info.serial_number,
                    info.manufacturer,
                    info.product,
                ),
                _ => (None, None, None, None, None),
            };
            let description = product
                .clone()
                .or_else(|| manufacturer.clone())
                .unwrap_or_else(|| "Serial device".into());
            let id = serial_number
                .clone()
                .map(|serial| {
                    format!(
                        "usb:{vid:04x}:{pid:04x}:{serial}",
                        vid = vid.unwrap_or_default(),
                        pid = pid.unwrap_or_default()
                    )
                })
                .unwrap_or_else(|| format!("path:{}", port.port_name));
            SerialDevice {
                id,
                display_name: format!("{} · {}", description, port.port_name),
                system_path: port.port_name,
                vid,
                pid,
                serial_number,
                manufacturer,
                product,
            }
        })
        .collect();
    devices.sort_by(|a, b| a.display_name.cmp(&b.display_name));
    Ok(devices)
}

pub fn find_device(id: &str) -> CoreResult<SerialDevice> {
    list_devices()?
        .into_iter()
        .find(|device| device.id == id)
        .ok_or_else(|| CoreError::DeviceNotFound(id.into()))
}

pub fn rediscover_device(
    original: &SerialDevice,
    previous_paths: &[String],
) -> CoreResult<SerialDevice> {
    let devices = list_devices()?;
    if let Some(serial) = &original.serial_number {
        if let Some(device) = devices
            .iter()
            .find(|device| device.serial_number.as_ref() == Some(serial))
        {
            return Ok(device.clone());
        }
    }
    if let Some(device) = devices
        .iter()
        .find(|device| !previous_paths.contains(&device.system_path) && device.vid.is_some())
    {
        return Ok(device.clone());
    }
    if let Some(device) = devices
        .iter()
        .find(|device| device.system_path == original.system_path)
    {
        return Ok(device.clone());
    }
    Err(CoreError::DeviceNotFound(original.display_name.clone()))
}
