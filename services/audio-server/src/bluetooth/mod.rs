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
}

#[derive(Debug, Error)]
pub enum BluetoothError {
    #[error("D-Bus error talking to BlueZ: {0}")]
    DBus(#[from] zbus::Error),
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
