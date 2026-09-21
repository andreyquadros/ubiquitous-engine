//! [`ScreenshotRepo`] implementation.

use chrono::{DateTime, Utc};
use rusqlite::{named_params, params, Row};
use ubiqx_core::ports::ScreenshotRepo;
use ubiqx_core::{CoreResult, Screenshot};

use crate::convert::{count, dt, ms};
use crate::error::StorageResult;
use crate::store::{execute, execute_expecting_row, query_list, query_one, query_opt, SqliteStore};

macro_rules! screenshot_cols {
    () => {
        "id, at, path, width, height, app_id, block_id, sent_to_ai"
    };
}

const INSERT: &str = concat!(
    "INSERT INTO screenshots (",
    screenshot_cols!(),
    ") VALUES (:id, :at, :path, :width, :height, :app_id, :block_id, :sent_to_ai)"
);

const SELECT_BY_ID: &str = concat!(
    "SELECT ",
    screenshot_cols!(),
    " FROM screenshots WHERE id = ?1"
);

const SELECT_LATEST_FOR_BLOCK: &str = concat!(
    "SELECT ",
    screenshot_cols!(),
    " FROM screenshots WHERE block_id = ?1 ORDER BY at DESC LIMIT 1"
);

const ATTACH: &str = "UPDATE screenshots SET block_id = ?2 WHERE id = ?1";

const MARK_SENT: &str = "UPDATE screenshots SET sent_to_ai = 1 WHERE id = ?1";

const COUNT_SENT_SINCE: &str = "SELECT COUNT(*) FROM screenshots WHERE sent_to_ai = 1 AND at >= ?1";

/// `RETURNING` hands back the deleted rows in one round trip.
const DELETE_BEFORE: &str = concat!(
    "DELETE FROM screenshots WHERE at < ?1 RETURNING ",
    screenshot_cols!()
);

fn row_to_screenshot(row: &Row<'_>) -> StorageResult<Screenshot> {
    Ok(Screenshot {
        id: row.get("id")?,
        at: dt(row.get("at")?)?,
        path: row.get("path")?,
        width: row.get("width")?,
        height: row.get("height")?,
        app_id: row.get("app_id")?,
        block_id: row.get("block_id")?,
        sent_to_ai: row.get("sent_to_ai")?,
    })
}

impl ScreenshotRepo for SqliteStore {
    fn insert(&self, shot: &Screenshot) -> CoreResult<()> {
        self.with(|conn| {
            execute(
                conn,
                INSERT,
                named_params! {
                    ":id": shot.id,
                    ":at": ms(shot.at),
                    ":path": shot.path,
                    ":width": shot.width,
                    ":height": shot.height,
                    ":app_id": shot.app_id,
                    ":block_id": shot.block_id,
                    ":sent_to_ai": shot.sent_to_ai,
                },
            )?;
            Ok(())
        })
    }

    fn get(&self, id: &str) -> CoreResult<Option<Screenshot>> {
        self.with(|conn| query_opt(conn, SELECT_BY_ID, params![id], row_to_screenshot))
    }

    fn latest_for_block(&self, block_id: &str) -> CoreResult<Option<Screenshot>> {
        self.with(|conn| {
            query_opt(
                conn,
                SELECT_LATEST_FOR_BLOCK,
                params![block_id],
                row_to_screenshot,
            )
        })
    }

    fn attach_to_block(&self, id: &str, block_id: &str) -> CoreResult<()> {
        self.with(|conn| {
            execute_expecting_row(conn, ATTACH, params![id, block_id], "screenshot", id)
        })
    }

    fn mark_sent(&self, id: &str) -> CoreResult<()> {
        self.with(|conn| execute_expecting_row(conn, MARK_SENT, params![id], "screenshot", id))
    }

    fn count_sent_since(&self, since: DateTime<Utc>) -> CoreResult<u64> {
        self.with(|conn| {
            query_one(conn, COUNT_SENT_SINCE, params![ms(since)], |row| {
                count(row, 0)
            })
        })
    }

    fn delete_before(&self, before: DateTime<Utc>) -> CoreResult<Vec<Screenshot>> {
        self.with(|conn| {
            let deleted = query_list(conn, DELETE_BEFORE, params![ms(before)], row_to_screenshot)?;
            tracing::debug!(before = %before, deleted = deleted.len(), "purged old screenshots");
            Ok(deleted)
        })
    }
}
