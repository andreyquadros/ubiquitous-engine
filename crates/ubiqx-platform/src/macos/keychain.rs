//! API keys in the macOS Keychain through the `keyring` crate.

use ubiqx_core::ports::SecretStore;
use ubiqx_core::{CoreError, CoreResult};

/// Keychain service name. Fixed on purpose: it must not change with the bundle id, otherwise
/// a rename would orphan the stored key.
pub const SERVICE: &str = "ai.ubiqx";

#[derive(Debug, Clone)]
pub struct KeychainSecretStore {
    service: String,
}

impl Default for KeychainSecretStore {
    fn default() -> Self {
        Self {
            service: SERVICE.into(),
        }
    }
}

impl KeychainSecretStore {
    fn entry(&self, key: &str) -> CoreResult<keyring::Entry> {
        keyring::Entry::new(&self.service, key)
            .map_err(|e| CoreError::Platform(format!("keychain: {e}")))
    }
}

impl SecretStore for KeychainSecretStore {
    fn get(&self, key: &str) -> CoreResult<Option<String>> {
        match self.entry(key)?.get_password() {
            Ok(v) => Ok(Some(v)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(CoreError::Platform(format!("keychain read: {e}"))),
        }
    }

    fn set(&self, key: &str, value: &str) -> CoreResult<()> {
        self.entry(key)?
            .set_password(value)
            .map_err(|e| CoreError::Platform(format!("keychain write: {e}")))
    }

    fn delete(&self, key: &str) -> CoreResult<()> {
        match self.entry(key)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(CoreError::Platform(format!("keychain delete: {e}"))),
        }
    }
}
