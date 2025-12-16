use super::error::{HsmError, HsmResult};
use std::sync::{Arc, Mutex};
use yubihsm::{Client, Connector, Credentials, UsbConfig};

/// Configuration for HSM connection
#[derive(Clone)]
pub struct HsmConfig {
    pub auth_key_id: u16,
    pub auth_password: String,
}

impl Default for HsmConfig {
    fn default() -> Self {
        Self {
            auth_key_id: 1,
            auth_password: "password".to_string(),
        }
    }
}

/// HSM client wrapper that manages the connection to yubihsm2
pub struct HsmClient {
    client: Arc<Mutex<Client>>,
}

impl HsmClient {
    pub fn connect(config: HsmConfig) -> HsmResult<Self> {
        // create usb connector
        let serial_config = UsbConfig::default();
        let connector = Connector::usb(&serial_config);
        let credentials =
            Credentials::from_password(config.auth_key_id, config.auth_password.as_bytes());

        // open client sesh
        let client = Client::open(connector, credentials, true)
            .map_err(|e| HsmError::AuthenticationFailed(format!("{:?}", e)))?;

        Ok(Self {
            client: Arc::new(Mutex::new(client)),
        })
    }

    /// with this we can call any yubihsm client method directly
    pub fn client(&self) -> Arc<Mutex<Client>> {
        self.client.clone()
    }
}

impl Drop for HsmClient {
    fn drop(&mut self) {
        // The YubiHSM client will automatically close the session when dropped
    }
}

/// Manages an active logical session to the HSM (one set of credentials).
/// Can be extended later to handle multiple named sessions.
pub struct SessionManager {
    active_client: Option<HsmClient>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            active_client: None,
        }
    }

    /// Connect using the provided config and set it as the active session.
    pub fn connect(&mut self, config: HsmConfig) -> HsmResult<()> {
        let client = HsmClient::connect(config)?;
        self.active_client = Some(client);
        Ok(())
    }

    /// Returns true if there is an active authenticated session.
    pub fn is_authenticated(&self) -> bool {
        self.active_client.is_some()
    }

    /// Get a reference to the active client, or an authentication error if none.
    pub fn active_client(&self) -> HsmResult<&HsmClient> {
        self.active_client.as_ref().ok_or_else(|| {
            HsmError::AuthenticationFailed(
                "No active HSM session. Please authenticate first.".into(),
            )
        })
    }

    /// Disconnect the current session, if any.
    pub fn disconnect(&mut self) {
        self.active_client = None;
    }
}
