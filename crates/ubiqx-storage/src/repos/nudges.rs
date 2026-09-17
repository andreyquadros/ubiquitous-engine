//! [`NudgeRepo`] implementation.

use chrono::{DateTime, Utc};
use rusqlite::{named_params, params, Row};
use ubiqx_core::ports::NudgeRepo;
use ubiqx_core::{CoreResult, Nudge, NudgeKind};

use crate::convert::{count, dt, ms, opt_dt, parse_enum, sql_limit};
use crate::error::StorageResult;
use crate::store::{execute, execute_expecting_row, query_list, query_one, SqliteStore};

macro_rules! nudge_cols {
    () => {
        "id, at, kind, title, message, seen"
    };
}

const INSERT: &str = concat!(
    "INSERT INTO nudges (",
    nudge_cols!(),
    ") VALUES (:id, :at, :kind, :title, :message, :seen)"
);

const SELECT_RECENT: &str = concat!(
    "SELECT ",
    nudge_cols!(),
    " FROM nudges ORDER BY at DESC, id DESC LIMIT ?1"
);

const MARK_SEEN: &str = "UPDATE nudges SET seen = 1 WHERE id = ?1";

const MARK_ALL_SEEN: &str = "UPDATE nudges SET seen = 1 WHERE seen = 0";

const LAST_OF_KIND: &str = "SELECT MAX(at) FROM nudges WHERE kind = ?1";

const COUNT_SINCE: &str = "SELECT COUNT(*) FROM nudges WHERE at >= ?1";

fn row_to_nudge(row: &Row<'_>) -> StorageResult<Nudge> {
    let kind: String = row.get("kind")?;
    Ok(Nudge {
        id: row.get("id")?,
        at: dt(row.get("at")?)?,
        kind: parse_enum("nudge kind", &kind, NudgeKind::parse)?,
        title: row.get("title")?,
        message: row.get("message")?,
        seen: row.get("seen")?,
    })
}

impl NudgeRepo for SqliteStore {
    fn insert(&self, nudge: &Nudge) -> CoreResult<()> {
        self.with(|conn| {
            execute(
                conn,
                INSERT,
                named_params! {
                    ":id": nudge.id,
                    ":at": ms(nudge.at),
                    ":kind": nudge.kind.as_str(),
                    ":title": nudge.title,
                    ":message": nudge.message,
                    ":seen": nudge.seen,
                },
            )?;
            Ok(())
        })
    }

    fn list_recent(&self, limit: usize) -> CoreResult<Vec<Nudge>> {
        self.with(|conn| query_list(conn, SELECT_RECENT, params![sql_limit(limit)], row_to_nudge))
    }

    fn mark_seen(&self, id: &str) -> CoreResult<()> {
        self.with(|conn| execute_expecting_row(conn, MARK_SEEN, params![id], "nudge", id))
    }

    fn mark_all_seen(&self) -> CoreResult<()> {
        self.with(|conn| execute(conn, MARK_ALL_SEEN, []).map(|_| ()))
    }

    fn last_of_kind(&self, kind: NudgeKind) -> CoreResult<Option<DateTime<Utc>>> {
        self.with(|conn| {
            query_one(conn, LAST_OF_KIND, params![kind.as_str()], |row| {
                opt_dt(row.get(0)?)
            })
        })
    }

    fn count_since(&self, since: DateTime<Utc>) -> CoreResult<u64> {
        self.with(|conn| query_one(conn, COUNT_SINCE, params![ms(since)], |row| count(row, 0)))
    }
}
