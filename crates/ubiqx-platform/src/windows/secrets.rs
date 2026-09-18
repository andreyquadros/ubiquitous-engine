//! API keys in the Windows Credential Manager (`keyring` with `windows-native`), with the
//! JSON file fallback of [`crate::secrets_file`] when the Credential Manager errors.

use std::path::Path;

use crate::secrets_file::{default_dir, FallbackSecretStore, FileSecretStore, KeyringStore};

/// The Credential Manager first, `<data_dir>/secrets.json` when it fails.
pub fn secret_store(data_dir: Option<&Path>) -> FallbackSecretStore {
    let dir = data_dir.map(Path::to_path_buf).unwrap_or_else(default_dir);
    FallbackSecretStore::new(
        Box::new(KeyringStore::default()),
        FileSecretStore::in_dir(&dir),
    )
}
