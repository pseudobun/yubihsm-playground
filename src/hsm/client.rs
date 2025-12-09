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
