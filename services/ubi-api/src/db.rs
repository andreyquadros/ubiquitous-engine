//! SQLite ledger: usage per subscriber and month, revocations and issued licenses.
//!
//! One connection behind a mutex is plenty: every statement here is a point lookup or a
//! single insert, and the proxy's real latency is the vendor call.

use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use chrono::{DateTime, Datelike, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

use crate::config::TokenUsage;

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS usage (
    id            INTEGER PRIMARY KEY,
    sub           TEXT    NOT NULL,
    month         TEXT    NOT NULL,
    model         TEXT    NOT NULL,
    input_tokens  INTEGER NOT NULL,
    output_tokens INTEGER NOT NULL,
    cost_usd      REAL    NOT NULL,
    at            TEXT    NOT NULL
);
CREATE INDEX IF NOT EXISTS usage_sub_month ON usage(sub, month);
CREATE TABLE IF NOT EXISTS revoked (
    sub    TEXT PRIMARY KEY,
    at     TEXT NOT NULL,
    reason TEXT
);
CREATE TABLE IF NOT EXISTS licenses (
    id          INTEGER PRIMARY KEY,
    sub         TEXT NOT NULL,
    plan        TEXT NOT NULL,
    email_hash  TEXT NOT NULL,
    external_id TEXT,
    event       TEXT NOT NULL,
    key_hint    TEXT NOT NULL,
    issued_at   TEXT NOT NULL,
    expires_at  TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS licenses_sub ON licenses(sub);
"#;

/// `YYYY-MM` in UTC: the accounting period of the managed plan.
pub fn month_of(at: DateTime<Utc>) -> String {
    format!("{:04}-{:02}", at.year(), at.month())
}

/// Validates a `YYYY-MM` string from a query parameter.
pub fn is_month(s: &str) -> bool {
    s.len() == 7
        && s.as_bytes()[4] == b'-'
        && s[..4].chars().all(|c| c.is_ascii_digit())
        && s[5..].chars().all(|c| c.is_ascii_digit())
        && matches!(s[5..].parse::<u32>(), Ok(1..=12))
}

#[derive(Debug, Clone)]
pub struct UsageRow {
    pub sub: String,
    pub model: String,
    pub usage: TokenUsage,
    pub cost_usd: f64,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SubscriberUsage {
    pub sub: String,
    pub requests: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_usd: f64,
}

#[derive(Debug, Clone)]
pub struct IssuedLicense {
    pub sub: String,
    pub plan: String,
    pub email_hash: String,
    pub external_id: Option<String>,
    pub event: String,
    pub key_hint: String,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct Db {
    conn: Arc<Mutex<Connection>>,
}

impl Db {
    /// Opens (and migrates) the database at `path`; `":memory:"` for tests.
    pub fn open(path: &str) -> Result<Self> {
        let conn = if path == ":memory:" {
            Connection::open_in_memory()?
        } else {
            let conn = Connection::open(path).with_context(|| format!("opening {path}"))?;
            conn.pragma_update(None, "journal_mode", "WAL")?;
            conn
        };
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.execute_batch(SCHEMA)?;
        Ok(Db {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    fn with<T>(&self, f: impl FnOnce(&Connection) -> rusqlite::Result<T>) -> Result<T> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| anyhow::anyhow!("db mutex poisoned"))?;
        Ok(f(&conn)?)
    }

    pub fn record_usage(&self, row: &UsageRow) -> Result<()> {
        self.with(|c| {
            c.execute(
                "INSERT INTO usage (sub, month, model, input_tokens, output_tokens, cost_usd, at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    row.sub,
                    month_of(row.at),
                    row.model,
                    row.usage.total_input() as i64,
                    row.usage.output_tokens as i64,
                    row.cost_usd,
                    row.at.to_rfc3339(),
                ],
            )
            .map(|_| ())
        })
    }

    /// USD spent by `sub` in `month`.
    pub fn spent_usd(&self, sub: &str, month: &str) -> Result<f64> {
        self.with(|c| {
            c.query_row(
                "SELECT COALESCE(SUM(cost_usd), 0.0) FROM usage WHERE sub = ?1 AND month = ?2",
                params![sub, month],
                |r| r.get::<_, f64>(0),
            )
        })
    }

    /// Per-subscriber totals for `month`, biggest spender first.
    pub fn usage_by_sub(&self, month: &str) -> Result<Vec<SubscriberUsage>> {
        self.with(|c| {
            let mut stmt = c.prepare(
                "SELECT sub, COUNT(*), COALESCE(SUM(input_tokens), 0), COALESCE(SUM(output_tokens), 0),
                        COALESCE(SUM(cost_usd), 0.0)
                 FROM usage WHERE month = ?1 GROUP BY sub ORDER BY 5 DESC, sub",
            )?;
            let rows = stmt.query_map(params![month], |r| {
                Ok(SubscriberUsage {
                    sub: r.get(0)?,
                    requests: r.get::<_, i64>(1)? as u64,
                    input_tokens: r.get::<_, i64>(2)? as u64,
                    output_tokens: r.get::<_, i64>(3)? as u64,
                    cost_usd: r.get(4)?,
                })
            })?;
            rows.collect()
        })
    }

    pub fn revoke(&self, sub: &str, reason: Option<&str>, at: DateTime<Utc>) -> Result<()> {
        self.with(|c| {
            c.execute(
                "INSERT INTO revoked (sub, at, reason) VALUES (?1, ?2, ?3)
                 ON CONFLICT(sub) DO UPDATE SET at = excluded.at, reason = excluded.reason",
                params![sub, at.to_rfc3339(), reason],
            )
            .map(|_| ())
        })
    }

    pub fn unrevoke(&self, sub: &str) -> Result<bool> {
        self.with(|c| {
            c.execute("DELETE FROM revoked WHERE sub = ?1", params![sub])
                .map(|n| n > 0)
        })
    }

    pub fn is_revoked(&self, sub: &str) -> Result<bool> {
        self.with(|c| {
            c.query_row("SELECT 1 FROM revoked WHERE sub = ?1", params![sub], |_| {
                Ok(())
            })
            .optional()
            .map(|r| r.is_some())
        })
    }

    pub fn record_license(&self, lic: &IssuedLicense) -> Result<()> {
        self.with(|c| {
            c.execute(
                "INSERT INTO licenses (sub, plan, email_hash, external_id, event, key_hint, issued_at, expires_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    lic.sub,
                    lic.plan,
                    lic.email_hash,
                    lic.external_id,
                    lic.event,
                    lic.key_hint,
                    lic.issued_at.to_rfc3339(),
                    lic.expires_at.to_rfc3339(),
                ],
            )
            .map(|_| ())
        })
    }

    /// Number of issuance events recorded for `sub` (tests, admin listing).
    pub fn license_events(&self, sub: &str) -> Result<u64> {
        self.with(|c| {
            c.query_row(
                "SELECT COUNT(*) FROM licenses WHERE sub = ?1",
                params![sub],
                |r| r.get::<_, i64>(0),
            )
            .map(|n| n as u64)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(sub: &str, cost: f64, at: DateTime<Utc>) -> UsageRow {
        UsageRow {
            sub: sub.into(),
            model: "m".into(),
            usage: TokenUsage {
                input_tokens: 10,
                output_tokens: 5,
                cache_read_tokens: 2,
                cache_write_tokens: 1,
            },
            cost_usd: cost,
            at,
        }
    }

    #[test]
    fn month_helpers() {
        let at = DateTime::parse_from_rfc3339("2026-09-18T23:59:59Z")
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(month_of(at), "2026-09");
        assert!(is_month("2026-01"));
        assert!(is_month("2026-12"));
        assert!(!is_month("2026-13"));
        assert!(!is_month("2026-1"));
        assert!(!is_month("2026/01"));
        assert!(!is_month("abcd-01"));
    }

    #[test]
    fn usage_is_summed_per_sub_and_month() {
        let db = Db::open(":memory:").unwrap();
        let sep = DateTime::parse_from_rfc3339("2026-09-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let oct = DateTime::parse_from_rfc3339("2026-10-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        db.record_usage(&row("a", 1.5, sep)).unwrap();
        db.record_usage(&row("a", 2.0, sep)).unwrap();
        db.record_usage(&row("b", 0.25, sep)).unwrap();
        db.record_usage(&row("a", 9.0, oct)).unwrap();
        assert_eq!(db.spent_usd("a", "2026-09").unwrap(), 3.5);
        assert_eq!(db.spent_usd("a", "2026-10").unwrap(), 9.0);
        assert_eq!(db.spent_usd("zzz", "2026-09").unwrap(), 0.0);
        let by_sub = db.usage_by_sub("2026-09").unwrap();
        assert_eq!(by_sub.len(), 2);
        assert_eq!(by_sub[0].sub, "a");
        assert_eq!(by_sub[0].requests, 2);
        assert_eq!(by_sub[0].input_tokens, 26);
        assert_eq!(by_sub[0].output_tokens, 10);
        assert_eq!(by_sub[1].sub, "b");
    }

    #[test]
    fn revocation_round_trip() {
        let db = Db::open(":memory:").unwrap();
        assert!(!db.is_revoked("s").unwrap());
        db.revoke("s", Some("cancelled"), Utc::now()).unwrap();
        assert!(db.is_revoked("s").unwrap());
        db.revoke("s", None, Utc::now()).unwrap();
        assert!(db.unrevoke("s").unwrap());
        assert!(!db.unrevoke("s").unwrap());
        assert!(!db.is_revoked("s").unwrap());
    }
}
