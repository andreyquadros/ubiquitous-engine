//! [`RuleRepo`] implementation.

use chrono::{DateTime, Utc};
use rusqlite::{named_params, params, Row};
use ubiqx_core::ports::RuleRepo;
use ubiqx_core::{CoreResult, Rule, RuleMatcher, RuleOrigin};

use crate::convert::{dt, ms, opt_dt, opt_ms, parse_enum};
use crate::error::StorageResult;
use crate::store::{execute, execute_expecting_row, query_list, SqliteStore};

macro_rules! rule_cols {
    () => {
        "id, category_id, matcher, pattern, priority, origin, enabled, created_at, hit_count, \
         miss_count, last_contradicted_at"
    };
}

const SELECT_ALL: &str = concat!(
    "SELECT ",
    rule_cols!(),
    " FROM rules ORDER BY priority DESC, created_at ASC, id ASC"
);

/// `created_at` is kept from the first insert.
const UPSERT: &str = concat!(
    "INSERT INTO rules (",
    rule_cols!(),
    ") VALUES (:id, :category_id, :matcher, :pattern, :priority, :origin, :enabled, \
     :created_at, :hit_count, :miss_count, :last_contradicted_at) \
     ON CONFLICT(id) DO UPDATE SET category_id = excluded.category_id, \
     matcher = excluded.matcher, pattern = excluded.pattern, priority = excluded.priority, \
     origin = excluded.origin, enabled = excluded.enabled, hit_count = excluded.hit_count, \
     miss_count = excluded.miss_count, last_contradicted_at = excluded.last_contradicted_at"
);

const DELETE: &str = "DELETE FROM rules WHERE id = ?1";

const INCREMENT_HITS: &str = "UPDATE rules SET hit_count = hit_count + 1 WHERE id = ?1";

const RECORD_MISS: &str =
    "UPDATE rules SET miss_count = miss_count + 1, last_contradicted_at = ?2 WHERE id = ?1";

fn row_to_rule(row: &Row<'_>) -> StorageResult<Rule> {
    let matcher: String = row.get("matcher")?;
    let origin: String = row.get("origin")?;
    Ok(Rule {
        id: row.get("id")?,
        category_id: row.get("category_id")?,
        matcher: parse_enum("rule matcher", &matcher, RuleMatcher::parse)?,
        pattern: row.get("pattern")?,
        priority: row.get("priority")?,
        origin: parse_enum("rule origin", &origin, RuleOrigin::parse)?,
        enabled: row.get("enabled")?,
        created_at: dt(row.get("created_at")?)?,
        hit_count: row.get("hit_count")?,
        miss_count: row.get("miss_count")?,
        last_contradicted_at: opt_dt(row.get("last_contradicted_at")?)?,
    })
}

impl RuleRepo for SqliteStore {
    fn list(&self) -> CoreResult<Vec<Rule>> {
        self.with(|conn| query_list(conn, SELECT_ALL, [], row_to_rule))
    }

    fn upsert(&self, rule: &Rule) -> CoreResult<()> {
        self.with(|conn| {
            execute(
                conn,
                UPSERT,
                named_params! {
                    ":id": rule.id,
                    ":category_id": rule.category_id,
                    ":matcher": rule.matcher.as_str(),
                    ":pattern": rule.pattern,
                    ":priority": rule.priority,
                    ":origin": rule.origin.as_str(),
                    ":enabled": rule.enabled,
                    ":created_at": ms(rule.created_at),
                    ":hit_count": rule.hit_count,
                    ":miss_count": rule.miss_count,
                    ":last_contradicted_at": opt_ms(rule.last_contradicted_at),
                },
            )?;
            Ok(())
        })
    }

    fn delete(&self, id: &str) -> CoreResult<()> {
        self.with(|conn| execute(conn, DELETE, params![id]).map(|_| ()))
    }

    fn increment_hits(&self, id: &str) -> CoreResult<()> {
        self.with(|conn| execute_expecting_row(conn, INCREMENT_HITS, params![id], "rule", id))
    }

    fn record_miss(&self, id: &str, at: DateTime<Utc>) -> CoreResult<()> {
        self.with(|conn| execute_expecting_row(conn, RECORD_MISS, params![id, ms(at)], "rule", id))
    }
}
