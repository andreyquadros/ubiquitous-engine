//! [`CorrectionRepo`] implementation.

use rusqlite::{named_params, params, Row};
use ubiqx_core::ports::CorrectionRepo;
use ubiqx_core::{CoreResult, Correction};

use crate::convert::{count, dt, ms, sql_limit};
use crate::error::StorageResult;
use crate::store::{execute, query_list, query_one, SqliteStore};

macro_rules! correction_cols {
    () => {
        "id, block_id, from_category_id, to_category_id, app_id, app_name, title_key, domain, \
         note, at"
    };
}

const INSERT: &str = concat!(
    "INSERT INTO corrections (",
    correction_cols!(),
    ") VALUES (:id, :block_id, :from_category_id, :to_category_id, :app_id, :app_name, \
     :title_key, :domain, :note, :at)"
);

const SELECT_RECENT: &str = concat!(
    "SELECT ",
    correction_cols!(),
    " FROM corrections ORDER BY at DESC, id DESC LIMIT ?1"
);

const COUNT: &str = "SELECT COUNT(*) FROM corrections";

fn row_to_correction(row: &Row<'_>) -> StorageResult<Correction> {
    Ok(Correction {
        id: row.get("id")?,
        block_id: row.get("block_id")?,
        from_category_id: row.get("from_category_id")?,
        to_category_id: row.get("to_category_id")?,
        app_id: row.get("app_id")?,
        app_name: row.get("app_name")?,
        title_key: row.get("title_key")?,
        domain: row.get("domain")?,
        note: row.get("note")?,
        at: dt(row.get("at")?)?,
    })
}

impl CorrectionRepo for SqliteStore {
    fn insert(&self, correction: &Correction) -> CoreResult<()> {
        self.with(|conn| {
            execute(
                conn,
                INSERT,
                named_params! {
                    ":id": correction.id,
                    ":block_id": correction.block_id,
                    ":from_category_id": correction.from_category_id,
                    ":to_category_id": correction.to_category_id,
                    ":app_id": correction.app_id,
                    ":app_name": correction.app_name,
                    ":title_key": correction.title_key,
                    ":domain": correction.domain,
                    ":note": correction.note,
                    ":at": ms(correction.at),
                },
            )?;
            Ok(())
        })
    }

    fn list_recent(&self, limit: usize) -> CoreResult<Vec<Correction>> {
        self.with(|conn| {
            query_list(
                conn,
                SELECT_RECENT,
                params![sql_limit(limit)],
                row_to_correction,
            )
        })
    }

    fn count(&self) -> CoreResult<u64> {
        self.with(|conn| query_one(conn, COUNT, [], |row| count(row, 0)))
    }
}
