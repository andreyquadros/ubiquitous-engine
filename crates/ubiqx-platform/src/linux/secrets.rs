//! API keys in the Secret Service (`keyring` with `sync-secret-service`: GNOME Keyring,
//! KWallet through its portal), with the JSON file fallback of [`crate::secrets_file`]
//! when no `org.freedesktop.secrets` daemon answers.

use std::path::Path;

use crate::secrets_file::{default_dir, FallbackSecretStore, FileSecretStore, KeyringStore};

/// The Secret Service first, `<data_dir>/secrets.json` when it fails.
pub fn secret_store(data_dir: Option<&Path>) -> FallbackSecretStore {
    let dir = data_dir.map(Path::to_path_buf).unwrap_or_else(default_dir);
    FallbackSecretStore::new(
        Box::new(KeyringStore::default()),
        FileSecretStore::in_dir(&dir),
    )
}
