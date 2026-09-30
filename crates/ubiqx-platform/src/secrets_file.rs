//! Secret storage shared by Windows and Linux: the OS keyring (Credential Manager, Secret
//! Service) first, and a JSON file under the data directory when the keyring backend
//! errors (no `org.freedesktop.secrets` daemon on a bare X11 session, a locked or absent
//! collection, a Credential Manager policy).
//!
//! The file is `secrets.json`, a flat `{ "<key>": "<value>" }` map. On Unix it is created
//! with mode `0600`; on Windows it inherits the user's profile ACL (the data directory
//! lives under `%APPDATA%`, readable only by that user and administrators). It is a
//! fallback, not a vault: the Settings page tells the user where their key lives.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use parking_lot::Mutex;
use ubiqx_core::ports::SecretStore;
use ubiqx_core::{CoreError, CoreResult};

/// File name of the fallback store inside the data directory.
pub const FILE_NAME: &str = "secrets.json";

/// Keyring service name shared by every OS (see `macos::keychain::SERVICE`).
pub const SERVICE: &str = "ai.ubiqx";

/// Where the fallback file goes when the composition root gave no data directory: the
/// per-user data folder of the OS, or the working directory as a last resort.
pub fn default_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("XDG_DATA_HOME").filter(|d| !d.is_empty()) {
        return PathBuf::from(dir).join("ubiqx");
    }
    if let Some(dir) = std::env::var_os("APPDATA").filter(|d| !d.is_empty()) {
        return PathBuf::from(dir).join("ai.ubiqx.app");
    }
    if let Some(home) = std::env::var_os("HOME").filter(|d| !d.is_empty()) {
        return PathBuf::from(home).join(".local/share/ubiqx");
    }
    PathBuf::from(".ubiqx")
}

/// JSON-file secret store.
#[derive(Debug)]
pub struct FileSecretStore {
    path: PathBuf,
    lock: Mutex<()>,
}

impl FileSecretStore {
    /// A store at `<dir>/secrets.json`.
    pub fn in_dir(dir: &Path) -> Self {
        Self {
            path: dir.join(FILE_NAME),
            lock: Mutex::new(()),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn read(&self) -> CoreResult<BTreeMap<String, String>> {
        match std::fs::read(&self.path) {
            Ok(bytes) if bytes.iter().all(u8::is_ascii_whitespace) => Ok(BTreeMap::new()),
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|e| CoreError::Platform(format!("secrets file is not valid JSON: {e}"))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(BTreeMap::new()),
            Err(e) => Err(CoreError::Platform(format!("secrets file read: {e}"))),
        }
    }

    fn write(&self, map: &BTreeMap<String, String>) -> CoreResult<()> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)
                .map_err(|e| CoreError::Platform(format!("secrets dir: {e}")))?;
        }
        let json = serde_json::to_vec_pretty(map)
            .map_err(|e| CoreError::Platform(format!("secrets encode: {e}")))?;
        // Write next to the file and rename, so a crash never leaves a truncated store.
        let tmp = self.path.with_extension("json.tmp");
        write_private(&tmp, &json)?;
        std::fs::rename(&tmp, &self.path)
            .map_err(|e| CoreError::Platform(format!("secrets file rename: {e}")))?;
        Ok(())
    }
}

/// Writes `bytes` to `path`, creating it user-readable only where the OS has file modes.
fn write_private(path: &Path, bytes: &[u8]) -> CoreResult<()> {
    use std::io::Write;
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .map_err(|e| CoreError::Platform(format!("secrets file open: {e}")))?;
    #[cfg(unix)]
    {
        // An existing file keeps its old mode on open: tighten it explicitly.
        use std::os::unix::fs::PermissionsExt;
        let _ = file.set_permissions(std::fs::Permissions::from_mode(0o600));
    }
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|e| CoreError::Platform(format!("secrets file write: {e}")))
}

impl SecretStore for FileSecretStore {
    fn get(&self, key: &str) -> CoreResult<Option<String>> {
        let _guard = self.lock.lock();
        Ok(self.read()?.get(key).cloned())
    }

    fn set(&self, key: &str, value: &str) -> CoreResult<()> {
        let _guard = self.lock.lock();
        let mut map = self.read()?;
        map.insert(key.to_string(), value.to_string());
        self.write(&map)
    }

    fn delete(&self, key: &str) -> CoreResult<()> {
        let _guard = self.lock.lock();
        let mut map = self.read()?;
        if map.remove(key).is_some() {
            self.write(&map)?;
        }
        Ok(())
    }
}

/// The OS keyring through the `keyring` crate, as a [`SecretStore`]. Compiled wherever a
/// keyring backend is enabled (every OS the desktop app ships on).
#[cfg(any(target_os = "windows", target_os = "linux"))]
#[derive(Debug, Clone)]
pub struct KeyringStore {
    service: String,
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
impl Default for KeyringStore {
    fn default() -> Self {
        Self {
            service: SERVICE.into(),
        }
    }
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
impl KeyringStore {
    fn entry(&self, key: &str) -> CoreResult<keyring::Entry> {
        keyring::Entry::new(&self.service, key)
            .map_err(|e| CoreError::Platform(format!("keyring: {e}")))
    }
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
impl SecretStore for KeyringStore {
    fn get(&self, key: &str) -> CoreResult<Option<String>> {
        match self.entry(key)?.get_password() {
            Ok(v) => Ok(Some(v)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(CoreError::Platform(format!("keyring read: {e}"))),
        }
    }

    fn set(&self, key: &str, value: &str) -> CoreResult<()> {
        self.entry(key)?
            .set_password(value)
            .map_err(|e| CoreError::Platform(format!("keyring write: {e}")))
    }

    fn delete(&self, key: &str) -> CoreResult<()> {
        match self.entry(key)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(CoreError::Platform(format!("keyring delete: {e}"))),
        }
    }
}

/// A primary store with a file fallback: reads try the primary and, when it errors (or has
/// nothing), the file; writes go to the primary and, when it errors, to the file; deletes
/// clear both, so a key never lingers in the fallback once the keyring works again.
pub struct FallbackSecretStore {
    primary: Box<dyn SecretStore>,
    fallback: FileSecretStore,
}

impl std::fmt::Debug for FallbackSecretStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FallbackSecretStore")
            .field("fallback", &self.fallback.path)
            .finish_non_exhaustive()
    }
}

impl FallbackSecretStore {
    pub fn new(primary: Box<dyn SecretStore>, fallback: FileSecretStore) -> Self {
        Self { primary, fallback }
    }

    /// Path of the fallback file.
    pub fn fallback_path(&self) -> &Path {
        self.fallback.path()
    }
}

impl SecretStore for FallbackSecretStore {
    fn get(&self, key: &str) -> CoreResult<Option<String>> {
        match self.primary.get(key) {
            Ok(Some(v)) => Ok(Some(v)),
            Ok(None) => self.fallback.get(key),
            Err(e) => {
                tracing::debug!(error = %e, "keyring read failed; using the secrets file");
                self.fallback.get(key)
            }
        }
    }

    fn set(&self, key: &str, value: &str) -> CoreResult<()> {
        match self.primary.set(key, value) {
            Ok(()) => {
                // The keyring holds it now: a stale copy in the file must not shadow it.
                let _ = self.fallback.delete(key);
                Ok(())
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    path = %self.fallback.path.display(),
                    "keyring write failed; storing the secret in the fallback file"
                );
                self.fallback.set(key, value)
            }
        }
    }

    fn delete(&self, key: &str) -> CoreResult<()> {
        let primary = self.primary.delete(key);
        let fallback = self.fallback.delete(key);
        match (primary, fallback) {
            (Ok(()), r) => r,
            (Err(e), Ok(())) => {
                tracing::debug!(error = %e, "keyring delete failed; the file copy is gone");
                Ok(())
            }
            (Err(e), Err(_)) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A primary store that always errors (a missing Secret Service daemon).
    struct Broken;

    impl SecretStore for Broken {
        fn get(&self, _: &str) -> CoreResult<Option<String>> {
            Err(CoreError::Platform("no keyring".into()))
        }
        fn set(&self, _: &str, _: &str) -> CoreResult<()> {
            Err(CoreError::Platform("no keyring".into()))
        }
        fn delete(&self, _: &str) -> CoreResult<()> {
            Err(CoreError::Platform("no keyring".into()))
        }
    }

    #[test]
    fn file_store_round_trip_and_mode() {
        let dir = tempfile::tempdir().unwrap();
        let store = FileSecretStore::in_dir(&dir.path().join("nested"));
        assert_eq!(store.get("a").unwrap(), None);
        store.set("a", "1").unwrap();
        store.set("b", "2").unwrap();
        assert_eq!(store.get("a").unwrap().as_deref(), Some("1"));
        let json: serde_json::Value =
            serde_json::from_slice(&std::fs::read(store.path()).unwrap()).unwrap();
        assert_eq!(json["b"], "2");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(store.path())
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600);
        }
        store.delete("a").unwrap();
        assert_eq!(store.get("a").unwrap(), None);
        assert_eq!(store.get("b").unwrap().as_deref(), Some("2"));
        assert!(!store.path().with_extension("json.tmp").exists());
    }

    #[test]
    fn file_store_rejects_garbage() {
        let dir = tempfile::tempdir().unwrap();
        let store = FileSecretStore::in_dir(dir.path());
        std::fs::write(store.path(), b"not json").unwrap();
        assert!(store.get("a").is_err());
        std::fs::write(store.path(), b"  \n").unwrap();
        assert_eq!(store.get("a").unwrap(), None);
    }

    #[test]
    fn falls_back_to_the_file_when_the_keyring_fails() {
        let dir = tempfile::tempdir().unwrap();
        let store = FallbackSecretStore::new(Box::new(Broken), FileSecretStore::in_dir(dir.path()));
        assert_eq!(store.get("k").unwrap(), None);
        store.set("k", "v").unwrap();
        assert_eq!(store.get("k").unwrap().as_deref(), Some("v"));
        assert!(store.fallback_path().is_file());
        store.delete("k").unwrap();
        assert_eq!(store.get("k").unwrap(), None);
    }

    #[test]
    fn a_working_keyring_wins_and_clears_the_file_copy() {
        let dir = tempfile::tempdir().unwrap();
        let file = FileSecretStore::in_dir(dir.path());
        file.set("k", "stale").unwrap();
        let primary = crate::secrets::EnvOrMemorySecretStore::default();
        let store = FallbackSecretStore::new(Box::new(primary), file);
        // Nothing in the keyring yet: the file answers.
        assert_eq!(store.get("k").unwrap().as_deref(), Some("stale"));
        store.set("k", "fresh").unwrap();
        assert_eq!(store.get("k").unwrap().as_deref(), Some("fresh"));
        let file = FileSecretStore::in_dir(dir.path());
        assert_eq!(file.get("k").unwrap(), None, "file copy removed");
    }

    #[test]
    fn default_dir_is_somewhere() {
        assert!(!default_dir().as_os_str().is_empty());
    }
}
