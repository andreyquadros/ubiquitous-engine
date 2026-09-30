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
-- Every payment platform retries a webhook it did not see acknowledged, so the same event
-- arrives more than once. Without this table a retry issued a second recorded licence and,
-- worse, reinstated a subscriber who had been cancelled in between.
CREATE TABLE IF NOT EXISTS webhook_events (
    dedup_key TEXT PRIMARY KEY,
    sub       TEXT NOT NULL,
    event     TEXT NOT NULL,
    at        TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS webhook_events_at ON webhook_events(at);
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

/// One subscriber as the panel lists them: the latest issuance, plus whether they are revoked.
/// `email_hash` is all there is -- the address itself never reaches this service.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SubscriberRow {
    pub sub: String,
    pub plan: String,
    pub email_hash: String,
    /// The subscription's id on the payment platform, for reconciling the two sides.
    pub external_id: Option<String>,
    pub key_hint: String,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    /// How many times a key was issued for this subscriber (first sale plus renewals).
    pub events: u64,
    pub revoked_at: Option<DateTime<Utc>>,
    pub revoked_reason: Option<String>,
}

/// What one subscriber spent in one month.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct MonthUsage {
    pub month: String,
    pub requests: u64,
    pub cost_usd: f64,
}

/// Counts for the panel's header. Cheap to compute in Rust over the subscriber list: this is a
/// single-operator product, not a data warehouse.
#[derive(Debug, Clone, Serialize, PartialEq, Default)]
pub struct Stats {
    pub subscribers: u64,
    /// Neither revoked nor past `expires_at`.
    pub active: u64,
    pub expired: u64,
    pub revoked: u64,
    /// Active, but close enough to expiry to be worth chasing. "Close" depends on the plan: a
    /// month is the whole life of a monthly licence, so thirty days would flag every one of
    /// them, for ever, and the warning would mean nothing.
    pub expiring_soon: u64,
    pub annual: u64,
    pub monthly: u64,
}

#[derive(Debug, Clone, Serialize)]
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

    /// Claims `dedup_key` for this delivery: `true` the first time it is seen, `false` when the
    /// platform is retrying one we already acted on. The insert is the claim, so two concurrent
    /// deliveries of the same event cannot both win.
    pub fn claim_webhook_event(
        &self,
        dedup_key: &str,
        sub: &str,
        event: &str,
        at: DateTime<Utc>,
    ) -> Result<bool> {
        self.with(|c| {
            c.execute(
                "INSERT OR IGNORE INTO webhook_events (dedup_key, sub, event, at)
                 VALUES (?1, ?2, ?3, ?4)",
                params![dedup_key, sub, event, at.to_rfc3339()],
            )
            .map(|n| n > 0)
        })
    }

    /// Drops claims older than `before`, so the table cannot grow without bound. A platform
    /// that retries later than this will be treated as a fresh event, which is the safe way
    /// round: a duplicate licence beats a customer with none.
    pub fn prune_webhook_events(&self, before: DateTime<Utc>) -> Result<u64> {
        self.with(|c| {
            c.execute(
                "DELETE FROM webhook_events WHERE at < ?1",
                params![before.to_rfc3339()],
            )
            .map(|n| n as u64)
        })
    }

    /// Every subscriber, newest issuance first: the latest row per `sub`, joined with the
    /// revocation list.
    pub fn list_subscribers(&self) -> Result<Vec<SubscriberRow>> {
        self.with(|c| {
            let mut stmt = c.prepare(
                "SELECT l.sub, l.plan, l.email_hash, l.external_id, l.key_hint, l.issued_at,
                        l.expires_at, (SELECT COUNT(*) FROM licenses WHERE sub = l.sub),
                        r.at, r.reason
                 FROM licenses l
                 JOIN (SELECT sub, MAX(id) AS id FROM licenses GROUP BY sub) last ON last.id = l.id
                 LEFT JOIN revoked r ON r.sub = l.sub
                 ORDER BY l.issued_at DESC, l.sub",
            )?;
            let rows = stmt.query_map([], |r| {
                Ok(SubscriberRow {
                    sub: r.get(0)?,
                    plan: r.get(1)?,
                    email_hash: r.get(2)?,
                    external_id: r.get(3)?,
                    key_hint: r.get(4)?,
                    issued_at: parse_ts(&r.get::<_, String>(5)?),
                    expires_at: parse_ts(&r.get::<_, String>(6)?),
                    events: r.get::<_, i64>(7)? as u64,
                    revoked_at: r.get::<_, Option<String>>(8)?.map(|s| parse_ts(&s)),
                    revoked_reason: r.get(9)?,
                })
            })?;
            rows.collect()
        })
    }

    /// Every key ever issued for `sub`, newest first.
    pub fn licenses_of(&self, sub: &str) -> Result<Vec<IssuedLicense>> {
        self.with(|c| {
            let mut stmt = c.prepare(
                "SELECT sub, plan, email_hash, external_id, event, key_hint, issued_at, expires_at
                 FROM licenses WHERE sub = ?1 ORDER BY id DESC",
            )?;
            let rows = stmt.query_map(params![sub], |r| {
                Ok(IssuedLicense {
                    sub: r.get(0)?,
                    plan: r.get(1)?,
                    email_hash: r.get(2)?,
                    external_id: r.get(3)?,
                    event: r.get(4)?,
                    key_hint: r.get(5)?,
                    issued_at: parse_ts(&r.get::<_, String>(6)?),
                    expires_at: parse_ts(&r.get::<_, String>(7)?),
                })
            })?;
            rows.collect()
        })
    }

    /// What `sub` spent, month by month, newest first.
    pub fn usage_of(&self, sub: &str) -> Result<Vec<MonthUsage>> {
        self.with(|c| {
            let mut stmt = c.prepare(
                "SELECT month, COUNT(*), COALESCE(SUM(cost_usd), 0.0)
                 FROM usage WHERE sub = ?1 GROUP BY month ORDER BY month DESC",
            )?;
            let rows = stmt.query_map(params![sub], |r| {
                Ok(MonthUsage {
                    month: r.get(0)?,
                    requests: r.get::<_, i64>(1)? as u64,
                    cost_usd: r.get(2)?,
                })
            })?;
            rows.collect()
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

/// Timestamps are written with `to_rfc3339` and only ever read back here; a row that somehow
/// is not parseable dates to the epoch rather than failing the whole listing.
fn parse_ts(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_else(|_| DateTime::<Utc>::UNIX_EPOCH)
}

/// How long before expiry a licence counts as expiring, by plan. A monthly licence lives about
/// thirty days, so it is only news in its last week; an annual one is worth chasing a month out.
pub fn expiring_window_days(plan: &str) -> i64 {
    if plan == "monthly_managed" {
        7
    } else {
        30
    }
}

/// Folds the subscriber list into the panel's header counts.
pub fn stats_of(rows: &[SubscriberRow], now: DateTime<Utc>) -> Stats {
    let mut s = Stats {
        subscribers: rows.len() as u64,
        ..Default::default()
    };
    for r in rows {
        match r.plan.as_str() {
            "annual_own_key" => s.annual += 1,
            "monthly_managed" => s.monthly += 1,
            _ => {}
        }
        if r.revoked_at.is_some() {
            s.revoked += 1;
        } else if r.expires_at <= now {
            s.expired += 1;
        } else {
            s.active += 1;
            if r.expires_at <= now + chrono::Duration::days(expiring_window_days(&r.plan)) {
                s.expiring_soon += 1;
            }
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

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
    fn a_delivery_is_claimed_once_and_the_claim_expires() {
        let db = Db::open(":memory:").unwrap();
        let t0 = DateTime::parse_from_rfc3339("2026-09-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        assert!(db
            .claim_webhook_event("id:evt_1", "s", "created", t0)
            .unwrap());
        assert!(
            !db.claim_webhook_event("id:evt_1", "s", "created", t0)
                .unwrap(),
            "the second delivery of the same event loses"
        );
        assert!(
            db.claim_webhook_event("id:evt_2", "s", "renewed", t0)
                .unwrap(),
            "a different event on the same subscriber still wins"
        );

        // Past the window the claim is gone, so a late retry counts as new -- a duplicate
        // licence is a nuisance, a customer left without one is a refund.
        assert_eq!(db.prune_webhook_events(t0).unwrap(), 0);
        assert_eq!(
            db.prune_webhook_events(t0 + Duration::seconds(1)).unwrap(),
            2
        );
        assert!(db
            .claim_webhook_event("id:evt_1", "s", "created", t0)
            .unwrap());
    }

    #[test]
    fn expiring_soon_is_read_against_the_plan_not_the_calendar() {
        let now = DateTime::parse_from_rfc3339("2026-09-21T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let row = |plan: &str, days: i64| SubscriberRow {
            sub: format!("{plan}-{days}"),
            plan: plan.into(),
            email_hash: "h".into(),
            external_id: None,
            key_hint: "ABCD".into(),
            issued_at: now,
            expires_at: now + Duration::days(days),
            events: 1,
            revoked_at: None,
            revoked_reason: None,
        };
        // A monthly licence 20 days out is simply mid-term; an annual one is worth a nudge.
        let rows = [
            row("monthly_managed", 20),
            row("monthly_managed", 5),
            row("annual_own_key", 20),
            row("annual_own_key", 200),
        ];
        let s = stats_of(&rows, now);
        assert_eq!(s.active, 4);
        assert_eq!(
            s.expiring_soon, 2,
            "the 5-day monthly and the 20-day annual"
        );
        assert_eq!(s.monthly, 2);
        assert_eq!(s.annual, 2);

        // Revoked and expired are counted apart from active, and revocation wins.
        let mut revoked = row("monthly_managed", 5);
        revoked.revoked_at = Some(now);
        let past = row("annual_own_key", -1);
        let s = stats_of(&[revoked, past], now);
        assert_eq!(
            (s.active, s.revoked, s.expired, s.expiring_soon),
            (0, 1, 1, 0)
        );
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
