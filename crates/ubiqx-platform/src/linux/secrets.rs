//! API keys in the Secret Service (`keyring` with `sync-secret-service`: GNOME Keyring,
//! KWallet through its portal), with the JSON file fallback of [`crate::secrets_file`]
//! when no `org.freedesktop.secrets` daemon answers.

use std::path::Path;

use crate::secrets_file::{default_dir, FallbackSecretStore, FileSecretStore, KeyringStore};

/// The Secret Service first, `<data_dir>/secrets.json` when it fails. `service` is the keyring
/// service name of the product (`ProductIdentity::keyring_service`).
pub fn secret_store(data_dir: Option<&Path>, service: &str) -> FallbackSecretStore {
    let dir = data_dir.map(Path::to_path_buf).unwrap_or_else(default_dir);
    FallbackSecretStore::new(
        Box::new(KeyringStore::with_service(service)),
        FileSecretStore::in_dir(&dir),
    )
}
