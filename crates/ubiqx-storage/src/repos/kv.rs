//! [`KvRepo`] implementation: small engine state that must survive restarts.

use chrono::Utc;
use rusqlite::params;
use ubiqx_core::ports::KvRepo;
use ubiqx_core::CoreResult;

use crate::convert::ms;
use crate::store::{execute, query_opt, SqliteStore};

const SELECT: &str = "SELECT value FROM kv WHERE key = ?1";

const UPSERT: &str = "INSERT INTO kv (key, value, updated_at) VALUES (?1, ?2, ?3) \
     ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at";

impl KvRepo for SqliteStore {
    fn get(&self, key: &str) -> CoreResult<Option<String>> {
        self.with(|conn| query_opt(conn, SELECT, params![key], |row| Ok(row.get(0)?)))
    }

    fn set(&self, key: &str, value: &str) -> CoreResult<()> {
        self.with(|conn| {
            execute(conn, UPSERT, params![key, value, ms(Utc::now())])?;
            Ok(())
        })
    }
}
