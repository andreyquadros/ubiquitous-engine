//! [`NofapRepo`] implementation: the configuration row, the sites and the hourly counters of
//! the `nofap` module.
//!
//! The behaviour is that of `ubiqx_engine::MemoryNofapRepo`, the reference the engine's
//! tests run against, with the differences a file needs:
//!
//! * a stored configuration that does not parse is an error, never the default: the default
//!   is "module off", so reading it would switch protection off silently;
//! * a site that is not the user's own is stored as its hash only, whatever the caller
//!   passed, and the schema refuses a name for it;
//! * a counter needs a real calendar day and an hour of the day, because the retention
//!   prune compares days as text;
//! * rows a newer build wrote with a value this build does not know (a counter kind, a site
//!   list or origin) are skipped instead of failing the read;
//! * erasing the history overwrites the freed pages, so the counters do not stay readable
//!   in the file.

use chrono::{DateTime, Utc};
use rusqlite::{named_params, params, Connection, Row};
use ubiqx_core::nofap::{
    DayCount, DayKind, NofapConfig, NofapRepo, NofapSite, SiteList, SiteOrigin,
};
use ubiqx_core::CoreResult;

use crate::convert::{date, date_str, dt, json, ms, opt_dt, opt_ms};
use crate::error::{StorageError, StorageResult};
use crate::store::{execute, query_list, query_one, query_opt, SqliteStore};

const SELECT_CONFIG: &str = "SELECT config FROM nofap_state WHERE id = 1";

const UPSERT_CONFIG: &str = "INSERT INTO nofap_state (id, config, updated_at) \
     VALUES (1, ?1, ?2) \
     ON CONFLICT(id) DO UPDATE SET config = excluded.config, updated_at = excluded.updated_at";

/// Insertion order: a replaced site moves to the end, like in the reference.
const SELECT_SITES: &str = "SELECT id, list, origin, host, host_key, created_at, expires_at \
     FROM nofap_sites ORDER BY rowid";

const DELETE_SITE_FOR_UPSERT: &str =
    "DELETE FROM nofap_sites WHERE id = :id OR (list = :list AND host_key = :host_key)";

const INSERT_SITE: &str = "INSERT INTO nofap_sites \
     (id, list, origin, host, host_key, created_at, expires_at) \
     VALUES (:id, :list, :origin, :host, :host_key, :created_at, :expires_at)";

const DELETE_SITE: &str = "DELETE FROM nofap_sites WHERE id = ?1";

const DELETE_SITES_BY_ORIGIN: &str = "DELETE FROM nofap_sites WHERE origin = ?1";

const DELETE_EXPIRED_SITES: &str =
    "DELETE FROM nofap_sites WHERE expires_at IS NOT NULL AND expires_at <= ?1";

/// The counter saturates at `u32::MAX`, like the reference's `saturating_add`.
const BUMP_DAY: &str = "INSERT INTO nofap_day (day, hour, kind, n) VALUES (?1, ?2, ?3, ?4) \
     ON CONFLICT(day, hour, kind) DO UPDATE SET n = MIN(n + excluded.n, 4294967295)";

const SELECT_DAYS: &str = "SELECT day, hour, kind, n FROM nofap_day \
     WHERE day >= ?1 AND day <= ?2 ORDER BY day, hour, kind";

const DELETE_DAYS_BEFORE: &str = "DELETE FROM nofap_day WHERE day < ?1";

const DELETE_ALL_DAYS: &str = "DELETE FROM nofap_day";

const SECURE_DELETE: &str = "PRAGMA secure_delete";

/// Folds the write-ahead log back into the main file, so the pages it still holds from
/// before the erase are gone too.
const CHECKPOINT: &str = "PRAGMA wal_checkpoint(TRUNCATE)";

/// A site row whose list or origin this build does not know is `None` (skipped).
fn row_to_site(row: &Row<'_>) -> StorageResult<Option<NofapSite>> {
    let list: String = row.get("list")?;
    let origin: String = row.get("origin")?;
    let (Some(list), Some(origin)) = (SiteList::parse(&list), SiteOrigin::parse(&origin)) else {
        return Ok(None);
    };
    let id: String = row.get("id")?;
    let key: Vec<u8> = row.get("host_key")?;
    let host_key: [u8; 16] = key.as_slice().try_into().map_err(|_| {
        StorageError::Decode(format!(
            "nofap site {id}: host key of {} bytes, expected 16",
            key.len()
        ))
    })?;
    let created_at: i64 = row.get("created_at")?;
    Ok(Some(NofapSite {
        id,
        list,
        origin,
        host: row.get("host")?,
        host_key,
        created_at: dt(created_at)?,
        expires_at: opt_dt(row.get("expires_at")?)?,
    }))
}

/// A counter row whose kind this build does not know, or whose hour is not an hour of the
/// day, is `None` (skipped). A count outside `u32` is clamped.
fn row_to_day(row: &Row<'_>) -> StorageResult<Option<DayCount>> {
    let kind: String = row.get("kind")?;
    let hour: i64 = row.get("hour")?;
    let (Some(kind), Some(hour)) = (
        DayKind::parse(&kind),
        u8::try_from(hour).ok().filter(|h| *h <= 23),
    ) else {
        return Ok(None);
    };
    let n: i64 = row.get("n")?;
    Ok(Some(DayCount {
        day: row.get("day")?,
        hour,
        kind,
        n: u32::try_from(n.max(0)).unwrap_or(u32::MAX),
    }))
}

/// Says where a stored configuration stopped parsing, and nothing of what it holds: serde's
/// own message quotes the offending value, which can be a site the user asked to allow.
fn unreadable_config(err: &serde_json::Error) -> StorageError {
    StorageError::Decode(format!(
        "the stored nofap configuration does not parse ({:?} error at line {}, column {})",
        err.classify(),
        err.line(),
        err.column()
    ))
}

/// Runs `f` with SQLite overwriting every page it frees, then restores the connection's
/// setting and checkpoints the log.
fn with_secure_delete(
    conn: &mut Connection,
    f: impl FnOnce(&mut Connection) -> StorageResult<()>,
) -> StorageResult<()> {
    let before: i64 = query_one(conn, SECURE_DELETE, [], |row| Ok(row.get(0)?))?;
    conn.pragma_update(None, "secure_delete", 1)?;
    let outcome = f(conn);
    conn.pragma_update(None, "secure_delete", before)?;
    outcome?;
    // The checkpoint answers with a row (busy, log frames, checkpointed frames).
    query_one(conn, CHECKPOINT, [], |_| Ok(()))
}

impl NofapRepo for SqliteStore {
    /// [`NofapConfig::default`] until the first save. Fields missing from a stored document
    /// (written by an older build) take their serde defaults. A document that does not
    /// parse is an error, never the default.
    fn load_config(&self) -> CoreResult<NofapConfig> {
        self.with(|conn| {
            let raw: Option<String> = query_opt(conn, SELECT_CONFIG, [], |row| Ok(row.get(0)?))?;
            match raw {
                Some(raw) => serde_json::from_str(&raw).map_err(|e| unreadable_config(&e)),
                None => Ok(NofapConfig::default()),
            }
        })
    }

    fn save_config(&self, config: &NofapConfig) -> CoreResult<()> {
        self.with(|conn| {
            execute(conn, UPSERT_CONFIG, params![json(config)?, ms(Utc::now())])?;
            Ok(())
        })
    }

    fn list_sites(&self) -> CoreResult<Vec<NofapSite>> {
        self.with(|conn| {
            let rows = query_list(conn, SELECT_SITES, [], row_to_site)?;
            Ok(rows.into_iter().flatten().collect())
        })
    }

    /// Replaces the row with the same id and the row with the same `(list, host_key)`.
    /// Only a [`SiteOrigin::User`] row keeps its `host`: a learned site or a snooze is
    /// written as its hash alone.
    fn upsert_site(&self, site: &NofapSite) -> CoreResult<()> {
        self.with(|conn| {
            let host = match site.origin {
                SiteOrigin::User => site.host.as_deref(),
                SiteOrigin::Learned | SiteOrigin::Snooze => None,
            };
            let tx = conn.transaction()?;
            execute(
                &tx,
                DELETE_SITE_FOR_UPSERT,
                named_params! {
                    ":id": site.id,
                    ":list": site.list.as_str(),
                    ":host_key": site.host_key.as_slice(),
                },
            )?;
            execute(
                &tx,
                INSERT_SITE,
                named_params! {
                    ":id": site.id,
                    ":list": site.list.as_str(),
                    ":origin": site.origin.as_str(),
                    ":host": host,
                    ":host_key": site.host_key.as_slice(),
                    ":created_at": ms(site.created_at),
                    ":expires_at": opt_ms(site.expires_at),
                },
            )?;
            tx.commit()?;
            Ok(())
        })
    }

    fn delete_site(&self, id: &str) -> CoreResult<()> {
        self.with(|conn| {
            execute(conn, DELETE_SITE, params![id])?;
            Ok(())
        })
    }

    fn delete_sites_by_origin(&self, origin: SiteOrigin) -> CoreResult<u64> {
        self.with(|conn| execute(conn, DELETE_SITES_BY_ORIGIN, params![origin.as_str()]))
    }

    fn purge_expired_sites(&self, now: DateTime<Utc>) -> CoreResult<u64> {
        self.with(|conn| execute(conn, DELETE_EXPIRED_SITES, params![ms(now)]))
    }

    /// `day` must be a calendar day (`YYYY-MM-DD`) and `hour` an hour of the day:
    /// [`ubiqx_core::CoreError::Invalid`] otherwise, and nothing is written.
    fn bump_day(&self, day: &str, hour: u8, kind: DayKind, by: u32) -> CoreResult<()> {
        self.with(|conn| {
            let canonical = date(day)
                .map(date_str)
                .map_err(|_| StorageError::Invalid(format!("nofap counter day `{day}`")))?;
            if canonical != day {
                return Err(StorageError::Invalid(format!("nofap counter day `{day}`")));
            }
            if hour > 23 {
                return Err(StorageError::Invalid(format!("nofap counter hour {hour}")));
            }
            execute(conn, BUMP_DAY, params![day, hour, kind.as_str(), by])?;
            Ok(())
        })
    }

    fn days_between(&self, from: &str, to: &str) -> CoreResult<Vec<DayCount>> {
        self.with(|conn| {
            let rows = query_list(conn, SELECT_DAYS, params![from, to], row_to_day)?;
            Ok(rows.into_iter().flatten().collect())
        })
    }

    fn delete_days_before(&self, day: &str) -> CoreResult<u64> {
        self.with(|conn| execute(conn, DELETE_DAYS_BEFORE, params![day]))
    }

    /// Every counter and every snooze, in one transaction, with the freed pages
    /// overwritten. The configuration and the user's and learned sites stay.
    fn erase_history(&self) -> CoreResult<()> {
        self.with(|conn| {
            with_secure_delete(conn, |conn| {
                let tx = conn.transaction()?;
                tx.execute(DELETE_ALL_DAYS, [])?;
                tx.execute(DELETE_SITES_BY_ORIGIN, [SiteOrigin::Snooze.as_str()])?;
                tx.commit()?;
                Ok(())
            })?;
            tracing::debug!("nofap history erased");
            Ok(())
        })
    }
}
