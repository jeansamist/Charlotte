//! BlueZ pairing agent (`org.bluez.Agent1`), served by this process.
//!
//! Behaves like a Bluetooth speaker (capability `NoInputNoOutput`, "Just Works"):
//! - while the adapter is **discoverable** (pairing window open), new devices are
//!   accepted and marked **trusted**;
//! - outside the window, unknown devices are rejected;
//! - trusted devices can always (re)connect and use the audio profiles.
//!
//! Runs on its own thread: zbus drives the D-Bus connection with `block_on`.

use super::Adapter1Proxy;
use std::thread::JoinHandle;
use tracing::{error, info, warn};
use zbus::{interface, Connection, ObjectPath, OwnedObjectPath};

pub const AGENT_PATH: &str = "/org/charlotte/audio/agent";
const CAPABILITY: &str = "NoInputNoOutput";

#[zbus::proxy(
    interface = "org.bluez.AgentManager1",
    default_service = "org.bluez",
    default_path = "/org/bluez"
)]
trait AgentManager1 {
    fn register_agent(&self, agent: &ObjectPath<'_>, capability: &str) -> zbus::Result<()>;
    fn request_default_agent(&self, agent: &ObjectPath<'_>) -> zbus::Result<()>;
}

#[zbus::proxy(interface = "org.bluez.Device1", default_service = "org.bluez")]
trait Device1 {
    #[zbus(property)]
    fn alias(&self) -> zbus::Result<String>;
    #[zbus(property)]
    fn address(&self) -> zbus::Result<String>;
    #[zbus(property)]
    fn trusted(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn paired(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn set_trusted(&self, value: bool) -> zbus::Result<()>;
}

/// Errors returned to BlueZ (names must be `org.bluez.Error.*`).
#[derive(Debug, zbus::DBusError)]
#[zbus(prefix = "org.bluez.Error")]
pub enum AgentError {
    #[zbus(error)]
    ZBus(zbus::Error),
    Rejected(String),
}

struct Agent {
    adapter_path: String,
    accept_trusted: bool,
}

impl Agent {
    async fn device(&self, conn: &Connection, path: &OwnedObjectPath) -> zbus::Result<Device1Proxy<'static>> {
        Device1Proxy::builder(conn).path(path.clone()).build().await
    }

    async fn pairing_window_open(&self, conn: &Connection) -> bool {
        match Adapter1Proxy::builder(conn).path(self.adapter_path.as_str()).build().await {
            Ok(a) => a.discoverable().await.unwrap_or(false),
            Err(_) => false,
        }
    }

    /// Accepts a new device only while the pairing window is open, and trusts it.
    async fn accept_new(&self, conn: &Connection, path: &OwnedObjectPath, step: &str) -> Result<(), AgentError> {
        let dev = self.device(conn, path).await?;
        let name = dev.alias().await.unwrap_or_default();
        let addr = dev.address().await.unwrap_or_default();
        if self.pairing_window_open(conn).await {
            dev.set_trusted(true).await?;
            info!(device = %name, address = %addr, step, "pairing accepted, device trusted");
            Ok(())
        } else {
            warn!(device = %name, address = %addr, step, "pairing rejected: pairing window is closed (run scripts/bt-sink.sh)");
            Err(AgentError::Rejected("pairing window closed".into()))
        }
    }
}

#[interface(name = "org.bluez.Agent1")]
impl Agent {
    async fn release(&self) {
        warn!("pairing agent released by BlueZ");
    }

    async fn request_pin_code(
        &self,
        #[zbus(connection)] conn: &Connection,
        device: OwnedObjectPath,
    ) -> Result<String, AgentError> {
        // Legacy (pre-2.1) devices: speakers conventionally use 0000.
        self.accept_new(conn, &device, "pin-code").await?;
        Ok("0000".into())
    }

    async fn display_pin_code(&self, _device: OwnedObjectPath, pincode: String) {
        info!(%pincode, "pairing PIN");
    }

    async fn request_passkey(
        &self,
        #[zbus(connection)] conn: &Connection,
        device: OwnedObjectPath,
    ) -> Result<u32, AgentError> {
        self.accept_new(conn, &device, "passkey").await?;
        Ok(0)
    }

    async fn display_passkey(&self, _device: OwnedObjectPath, passkey: u32, _entered: u16) {
        info!(passkey = format!("{passkey:06}"), "pairing passkey");
    }

    async fn request_confirmation(
        &self,
        #[zbus(connection)] conn: &Connection,
        device: OwnedObjectPath,
        passkey: u32,
    ) -> Result<(), AgentError> {
        info!(passkey = format!("{passkey:06}"), "pairing confirmation requested");
        self.accept_new(conn, &device, "confirmation").await
    }

    async fn request_authorization(
        &self,
        #[zbus(connection)] conn: &Connection,
        device: OwnedObjectPath,
    ) -> Result<(), AgentError> {
        self.accept_new(conn, &device, "authorization").await
    }

    async fn authorize_service(
        &self,
        #[zbus(connection)] conn: &Connection,
        device: OwnedObjectPath,
        uuid: String,
    ) -> Result<(), AgentError> {
        let dev = self.device(conn, &device).await?;
        let trusted = dev.trusted().await.unwrap_or(false);
        // Pairing is only possible while the pairing window is open (Pairable has the
        // same timeout), so a paired device was accepted then: trust it.
        let paired = dev.paired().await.unwrap_or(false);
        let name = dev.alias().await.unwrap_or_default();
        if paired && !trusted {
            dev.set_trusted(true).await?;
            info!(device = %name, "paired device trusted");
        }
        if ((trusted || paired) && self.accept_trusted) || self.pairing_window_open(conn).await {
            info!(device = %name, %uuid, "service authorized");
            Ok(())
        } else {
            warn!(device = %name, %uuid, "service rejected (untrusted device)");
            Err(AgentError::Rejected("untrusted device".into()))
        }
    }

    async fn cancel(&self) {
        info!("pairing cancelled by the remote device");
    }
}

/// Starts the agent thread. It registers as BlueZ's default agent and lives
/// until the process exits (BlueZ drops the agent when our connection closes).
pub fn spawn_agent(adapter: &str, accept_trusted: bool) -> std::io::Result<JoinHandle<()>> {
    let adapter_path = format!("/org/bluez/{adapter}");
    std::thread::Builder::new().name("bt-agent".into()).spawn(move || {
        let result = zbus::block_on(async {
            let conn = zbus::connection::Builder::system()
                .serve_at(AGENT_PATH, Agent { adapter_path: adapter_path.clone(), accept_trusted })
                .build()
                .await?;
            let path = ObjectPath::from_static_str_unchecked(AGENT_PATH);
            let mgr = AgentManager1Proxy::new(&conn).await?;
            mgr.register_agent(&path, CAPABILITY).await?;
            mgr.request_default_agent(&path).await?;
            // New devices may pair only during the pairing window (see set_pairing_window).
            let adapter = Adapter1Proxy::builder(&conn).path(adapter_path.as_str()).build().await?;
            if !adapter.discoverable().await? && adapter.pairable().await? {
                adapter.set_pairable(false).await?;
            }
            info!(capability = CAPABILITY, "pairing agent registered (default agent)");
            std::future::pending::<()>().await;
            Ok::<(), zbus::Error>(())
        });
        if let Err(e) = result {
            error!(error = %e, "pairing agent failed — new devices cannot pair");
        }
    })
}
