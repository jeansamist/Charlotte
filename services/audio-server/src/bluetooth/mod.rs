//! Bluetooth adapter identity, over BlueZ's D-Bus API (zbus).
//!
//! Sets the name other devices see (the adapter `Alias`), and checks the
//! Class of Device. A Mac or phone lists us as a speaker only if the major class
//! is Audio/Video. The class can only be set by root in /etc/bluetooth/main.conf
//! (see `scripts/setup-bluetooth.sh`), so we only report it here.

pub mod agent;

use thiserror::Error;

#[zbus::proxy(interface = "org.bluez.Adapter1", default_service = "org.bluez")]
trait Adapter1 {
    #[zbus(property)]
    fn alias(&self) -> zbus::Result<String>;
    #[zbus(property)]
    fn set_alias(&self, value: &str) -> zbus::Result<()>;
    #[zbus(property)]
    fn class(&self) -> zbus::Result<u32>;
    #[zbus(property)]
    fn address(&self) -> zbus::Result<String>;
    #[zbus(property)]
    fn powered(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn discoverable(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn pairable(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn set_pairable(&self, value: bool) -> zbus::Result<()>;
    #[zbus(property)]
    fn set_discoverable(&self, value: bool) -> zbus::Result<()>;
    #[zbus(property)]
    fn discoverable_timeout(&self) -> zbus::Result<u32>;
    #[zbus(property)]
    fn set_discoverable_timeout(&self, value: u32) -> zbus::Result<()>;
    fn remove_device(&self, device: &zbus::ObjectPath<'_>) -> zbus::Result<()>;
}

#[derive(Debug, Error)]
pub enum BluetoothError {
    #[error("D-Bus error talking to BlueZ: {0}")]
    DBus(#[from] zbus::Error),
    #[error("invalid Bluetooth address {0:?}")]
    InvalidAddress(String),
}

#[derive(Debug, Clone)]
pub struct AdapterStatus {
    pub address: String,
    pub alias: String,
    pub class: u32,
    pub powered: bool,
    /// True if the alias was changed by this call.
    pub renamed: bool,
}

/// Sets the adapter alias to `name` (if different) and returns the adapter status.
pub fn apply_identity(adapter: &str, name: &str) -> Result<AdapterStatus, BluetoothError> {
    zbus::block_on(async {
        let conn = zbus::Connection::system().await?;
        let proxy = Adapter1Proxy::builder(&conn)
            .path(format!("/org/bluez/{adapter}"))
            .build()
            .await?;
        let mut renamed = false;
        if proxy.alias().await? != name {
            proxy.set_alias(name).await?;
            renamed = true;
        }
        Ok(AdapterStatus {
            address: proxy.address().await?,
            // BlueZ may still report the old alias right after the change.
            alias: if renamed { name.to_owned() } else { proxy.alias().await? },
            class: proxy.class().await?,
            powered: proxy.powered().await?,
            renamed,
        })
    })
}

/// Adapter + known devices, for the API.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BluetoothStatus {
    pub adapter: String,
    pub address: String,
    pub name: String,
    pub class: u32,
    pub class_name: &'static str,
    pub audio_class: bool,
    pub powered: bool,
    pub discoverable: bool,
    pub discoverable_timeout: u32,
    pub pairable: bool,
    pub devices: Vec<DeviceStatus>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DeviceStatus {
    pub address: String,
    pub name: String,
    pub icon: String,
    pub paired: bool,
    pub trusted: bool,
    pub connected: bool,
}

async fn adapter_proxy<'a>(conn: &'a zbus::Connection, adapter: &str) -> zbus::Result<Adapter1Proxy<'a>> {
    Adapter1Proxy::builder(conn).path(format!("/org/bluez/{adapter}")).build().await
}

/// Reads the adapter state and every device BlueZ knows on this adapter.
pub fn status(adapter: &str) -> Result<BluetoothStatus, BluetoothError> {
    zbus::block_on(async {
        let conn = zbus::Connection::system().await?;
        let a = adapter_proxy(&conn, adapter).await?;
        let om = zbus::fdo::ObjectManagerProxy::builder(&conn)
            .destination("org.bluez")
            .path("/")
            .build()
            .await?;
        let prefix = format!("/org/bluez/{adapter}/dev_");
        let mut devices = Vec::new();
        for (path, mut ifaces) in om.get_managed_objects().await? {
            if !path.as_str().starts_with(&prefix) {
                continue;
            }
            let Some(mut p) = ifaces.remove("org.bluez.Device1") else { continue };
            let s = |p: &mut std::collections::HashMap<String, zbus::OwnedVariant>, k: &str| {
                p.remove(k).and_then(|v| String::try_from(v).ok()).unwrap_or_default()
            };
            let b = |p: &std::collections::HashMap<String, zbus::OwnedVariant>, k: &str| {
                p.get(k).and_then(|v| bool::try_from(v).ok()).unwrap_or(false)
            };
            devices.push(DeviceStatus {
                address: s(&mut p, "Address"),
                name: s(&mut p, "Alias"),
                icon: s(&mut p, "Icon"),
                paired: b(&p, "Paired"),
                trusted: b(&p, "Trusted"),
                connected: b(&p, "Connected"),
            });
        }
        devices.sort_by(|x, y| y.connected.cmp(&x.connected).then(x.name.cmp(&y.name)));
        let class = a.class().await?;
        Ok(BluetoothStatus {
            adapter: adapter.to_owned(),
            address: a.address().await?,
            name: a.alias().await?,
            class,
            class_name: major_class_name(class),
            audio_class: is_audio_class(class),
            powered: a.powered().await?,
            discoverable: a.discoverable().await?,
            discoverable_timeout: a.discoverable_timeout().await?,
            pairable: a.pairable().await?,
            devices,
        })
    })
}

/// Opens (`Some(seconds)`) or closes (`None`) the pairing window.
pub fn set_pairing_window(adapter: &str, seconds: Option<u32>) -> Result<(), BluetoothError> {
    zbus::block_on(async {
        let conn = zbus::Connection::system().await?;
        let a = adapter_proxy(&conn, adapter).await?;
        match seconds {
            Some(s) => {
                a.set_pairable(true).await?;
                a.set_discoverable_timeout(s).await?;
                a.set_discoverable(true).await?;
            }
            None => a.set_discoverable(false).await?,
        }
        Ok(())
    })
}

/// Sets the name other devices see.
pub fn set_name(adapter: &str, name: &str) -> Result<(), BluetoothError> {
    zbus::block_on(async {
        let conn = zbus::Connection::system().await?;
        adapter_proxy(&conn, adapter).await?.set_alias(name).await?;
        Ok(())
    })
}

/// Unpairs and forgets a device (`AA:BB:CC:DD:EE:FF`).
pub fn forget_device(adapter: &str, address: &str) -> Result<(), BluetoothError> {
    let valid = address.len() == 17
        && address.split(':').all(|p| p.len() == 2 && p.chars().all(|c| c.is_ascii_hexdigit()));
    if !valid {
        return Err(BluetoothError::InvalidAddress(address.to_owned()));
    }
    let path = format!("/org/bluez/{adapter}/dev_{}", address.to_uppercase().replace(':', "_"));
    zbus::block_on(async {
        let conn = zbus::Connection::system().await?;
        let obj = zbus::ObjectPath::try_from(path.as_str()).map_err(zbus::Error::from)?;
        adapter_proxy(&conn, adapter).await?.remove_device(&obj).await?;
        Ok(())
    })
}

/// Major device class (bits 8..=12 of the Class of Device).
pub fn major_class(class: u32) -> u32 {
    (class >> 8) & 0x1f
}

/// True if other devices will see us as an audio device (speaker/headphones…).
pub fn is_audio_class(class: u32) -> bool {
    major_class(class) == 0x04
}

pub fn major_class_name(class: u32) -> &'static str {
    match major_class(class) {
        0x00 => "Miscellaneous",
        0x01 => "Computer",
        0x02 => "Phone",
        0x03 => "Network",
        0x04 => "Audio/Video",
        0x05 => "Peripheral",
        0x06 => "Imaging",
        0x07 => "Wearable",
        0x08 => "Toy",
        0x09 => "Health",
        _ => "Uncategorized",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_laptop_class() {
        // Value read from the dev box before setup: Computer / Laptop + service bits.
        assert_eq!(major_class_name(0x007c010c), "Computer");
        assert!(!is_audio_class(0x007c010c));
    }

    #[test]
    fn decodes_loudspeaker_class() {
        // Audio/Video, Loudspeaker, with Rendering + Audio service bits.
        assert!(is_audio_class(0x240414));
        assert_eq!(major_class_name(0x240414), "Audio/Video");
    }
}
