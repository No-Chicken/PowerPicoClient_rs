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
    select_rediscovered_device(original, previous_paths, &devices)
        .ok_or_else(|| CoreError::DeviceNotFound(original.display_name.clone()))
}

fn select_unique<'a>(devices: impl Iterator<Item = &'a SerialDevice>) -> Option<SerialDevice> {
    let mut matches = devices;
    let first = matches.next()?.clone();
    matches.next().is_none().then_some(first)
}

fn select_rediscovered_device(
    original: &SerialDevice,
    previous_paths: &[String],
    devices: &[SerialDevice],
) -> Option<SerialDevice> {
    if let Some(serial) = &original.serial_number {
        if let Some(device) = select_unique(
            devices
                .iter()
                .filter(|device| device.serial_number.as_ref() == Some(serial)),
        ) {
            return Some(device);
        }
    }

    let new_usb_devices: Vec<_> = devices
        .iter()
        .filter(|device| !previous_paths.contains(&device.system_path) && device.vid.is_some())
        .collect();
    if let Some(device) = select_unique(new_usb_devices.iter().copied().filter(|device| {
        device.vid == original.vid && device.pid == original.pid && original.vid.is_some()
    })) {
        return Some(device);
    }
    if let Some(device) = select_unique(new_usb_devices.iter().copied().filter(|device| {
        let same_product = original.product.is_some() && device.product == original.product;
        let same_manufacturer =
            original.manufacturer.is_some() && device.manufacturer == original.manufacturer;
        same_product || same_manufacturer
    })) {
        return Some(device);
    }
    if new_usb_devices.len() == 1 {
        return Some(new_usb_devices[0].clone());
    }

    if let Some(device) = devices
        .iter()
        .find(|device| device.system_path == original.system_path)
    {
        return Some(device.clone());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn device(
        path: &str,
        vid: Option<u16>,
        pid: Option<u16>,
        serial: Option<&str>,
        manufacturer: Option<&str>,
        product: Option<&str>,
    ) -> SerialDevice {
        SerialDevice {
            id: format!("path:{path}"),
            display_name: path.into(),
            system_path: path.into(),
            vid,
            pid,
            serial_number: serial.map(Into::into),
            manufacturer: manufacturer.map(Into::into),
            product: product.map(Into::into),
        }
    }

    #[test]
    fn rediscovery_prefers_a_unique_serial_number() {
        let original = device(
            "/dev/ttyACM0",
            Some(0x1209),
            Some(0x0001),
            Some("powerpico-1"),
            Some("OpenFeastTech"),
            Some("PowerPico"),
        );
        let found = device(
            "/dev/ttyACM2",
            Some(0x1209),
            Some(0x0002),
            Some("powerpico-1"),
            Some("OpenFeastTech"),
            Some("PowerPico Boot"),
        );
        let unrelated = device(
            "/dev/ttyUSB0",
            Some(0x1a86),
            Some(0x7523),
            Some("other"),
            Some("Other"),
            Some("Adapter"),
        );

        assert_eq!(
            select_rediscovered_device(
                &original,
                &[original.system_path.clone()],
                &[unrelated, found.clone()],
            ),
            Some(found)
        );
    }

    #[test]
    fn rediscovery_uses_the_only_new_usb_device() {
        let original = device(
            "/dev/ttyACM0",
            Some(0x1209),
            Some(0x0001),
            None,
            Some("OpenFeastTech"),
            Some("PowerPico"),
        );
        let bootloader = device(
            "/dev/ttyACM1",
            Some(0x1209),
            Some(0x0002),
            None,
            Some("Bootloader"),
            Some("Pico Boot"),
        );

        assert_eq!(
            select_rediscovered_device(
                &original,
                &[original.system_path.clone()],
                &[bootloader.clone()],
            ),
            Some(bootloader)
        );
    }

    #[test]
    fn rediscovery_rejects_ambiguous_new_usb_devices() {
        let original = device("/dev/ttyACM0", None, None, None, None, None);
        let first = device("/dev/ttyACM1", Some(0x1209), Some(0x0002), None, None, None);
        let second = device("/dev/ttyUSB0", Some(0x1a86), Some(0x7523), None, None, None);

        assert_eq!(
            select_rediscovered_device(
                &original,
                &[original.system_path.clone()],
                &[first, second],
            ),
            None
        );
    }

    #[test]
    fn rediscovery_accepts_a_reused_system_path() {
        let original = device("/dev/ttyACM0", None, None, None, None, None);

        assert_eq!(
            select_rediscovered_device(
                &original,
                &[original.system_path.clone()],
                &[original.clone()],
            ),
            Some(original)
        );
    }
}
