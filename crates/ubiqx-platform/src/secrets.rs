//! Secret stores that work everywhere. The Keychain store lives in [`crate::macos`].

use std::collections::HashMap;

use parking_lot::RwLock;
use ubiqx_core::ports::{secret_keys, SecretStore};
use ubiqx_core::CoreResult;

/// In-memory store seeded from environment variables (`UBIQX_ANTHROPIC_API_KEY` /
/// `ANTHROPIC_API_KEY`, `UBIQX_OPENAI_API_KEY` / `OPENAI_API_KEY`, `UBIQX_XAI_API_KEY` /
/// `XAI_API_KEY`). Values set at runtime live until the process exits. Intended for the CLI,
/// CI and tests — never for the desktop app.
#[derive(Debug, Default)]
pub struct EnvOrMemorySecretStore {
    values: RwLock<HashMap<String, String>>,
}

impl EnvOrMemorySecretStore {
    pub fn with(key: &str, value: &str) -> Self {
        let store = Self::default();
        store
            .values
            .write()
            .insert(key.to_string(), value.to_string());
        store
    }
}

impl SecretStore for EnvOrMemorySecretStore {
    fn get(&self, key: &str) -> CoreResult<Option<String>> {
        if let Some(v) = self.values.read().get(key) {
            return Ok(Some(v.clone()));
        }
        let vars: &[&str] = match key {
            k if k == secret_keys::ANTHROPIC_API_KEY => {
                &["UBIQX_ANTHROPIC_API_KEY", "ANTHROPIC_API_KEY"]
            }
            k if k == secret_keys::OPENAI_API_KEY => &["UBIQX_OPENAI_API_KEY", "OPENAI_API_KEY"],
            k if k == secret_keys::XAI_API_KEY => &["UBIQX_XAI_API_KEY", "XAI_API_KEY"],
            _ => &[],
        };
        for var in vars {
            if let Ok(v) = std::env::var(var) {
                if !v.trim().is_empty() {
                    return Ok(Some(v.trim().to_string()));
                }
            }
        }
        Ok(None)
    }

    fn set(&self, key: &str, value: &str) -> CoreResult<()> {
        self.values
            .write()
            .insert(key.to_string(), value.to_string());
        Ok(())
    }

    fn delete(&self, key: &str) -> CoreResult<()> {
        self.values.write().remove(key);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_round_trip() {
        let s = EnvOrMemorySecretStore::default();
        assert_eq!(s.get("x").unwrap(), None);
        s.set("x", "1").unwrap();
        assert_eq!(s.get("x").unwrap().as_deref(), Some("1"));
        s.delete("x").unwrap();
        assert_eq!(s.get("x").unwrap(), None);
    }
}
