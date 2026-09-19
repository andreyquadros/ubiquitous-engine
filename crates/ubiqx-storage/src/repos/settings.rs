//! [`SettingsRepo`] implementation: one row holding the whole struct as JSON.

use chrono::Utc;
use rusqlite::params;
use ubiqx_core::ports::SettingsRepo;
use ubiqx_core::{CoreResult, Settings};

use crate::convert::{from_json, json, ms};
use crate::store::{execute, query_opt, SqliteStore};

const SELECT: &str = "SELECT json FROM settings WHERE id = 1";

const UPSERT: &str = "INSERT INTO settings (id, json, updated_at) VALUES (1, ?1, ?2) \
     ON CONFLICT(id) DO UPDATE SET json = excluded.json, updated_at = excluded.updated_at";

impl SettingsRepo for SqliteStore {
    /// Returns [`Settings::default`] until the first save. Fields missing from a stored JSON
    /// (written by an older build) take their serde defaults.
    fn load(&self) -> CoreResult<Settings> {
        self.with(|conn| {
            let raw: Option<String> = query_opt(conn, SELECT, [], |row| Ok(row.get(0)?))?;
            match raw {
                Some(raw) => from_json(&raw),
                None => Ok(Settings::default()),
            }
        })
    }

    fn save(&self, settings: &Settings) -> CoreResult<()> {
        self.with(|conn| {
            execute(conn, UPSERT, params![json(settings)?, ms(Utc::now())])?;
            Ok(())
        })
    }
}
