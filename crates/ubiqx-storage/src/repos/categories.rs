//! [`CategoryRepo`] implementation.

use rusqlite::{named_params, params, Row};
use ubiqx_core::ports::CategoryRepo;
use ubiqx_core::{Category, CoreResult};

use crate::convert::{dt, from_json, json, ms, opt_time, time_str};
use crate::error::{StorageError, StorageResult};
use crate::store::{execute, query_list, query_opt, SqliteStore};

macro_rules! category_cols {
    () => {
        "id, name, color, icon, description, keywords, report_time, report_template, \
         is_productive, is_system, archived, sort_order, created_at"
    };
}

const SELECT_ALL: &str = concat!(
    "SELECT ",
    category_cols!(),
    " FROM categories ORDER BY sort_order ASC, name ASC"
);

const SELECT_ACTIVE: &str = concat!(
    "SELECT ",
    category_cols!(),
    " FROM categories WHERE archived = 0 ORDER BY sort_order ASC, name ASC"
);

const SELECT_BY_ID: &str = concat!(
    "SELECT ",
    category_cols!(),
    " FROM categories WHERE id = ?1"
);

/// `created_at` is kept from the first insert.
const UPSERT: &str = concat!(
    "INSERT INTO categories (",
    category_cols!(),
    ") VALUES (:id, :name, :color, :icon, :description, :keywords, :report_time, \
     :report_template, :is_productive, :is_system, :archived, :sort_order, :created_at) \
     ON CONFLICT(id) DO UPDATE SET name = excluded.name, color = excluded.color, \
     icon = excluded.icon, description = excluded.description, keywords = excluded.keywords, \
     report_time = excluded.report_time, report_template = excluded.report_template, \
     is_productive = excluded.is_productive, is_system = excluded.is_system, \
     archived = excluded.archived, sort_order = excluded.sort_order"
);

const SELECT_IS_SYSTEM: &str = "SELECT is_system FROM categories WHERE id = ?1";

const DELETE: &str = "DELETE FROM categories WHERE id = ?1";

fn row_to_category(row: &Row<'_>) -> StorageResult<Category> {
    let keywords: String = row.get("keywords")?;
    Ok(Category {
        id: row.get("id")?,
        name: row.get("name")?,
        color: row.get("color")?,
        icon: row.get("icon")?,
        description: row.get("description")?,
        keywords: from_json(&keywords)?,
        report_time: opt_time(row.get("report_time")?)?,
        report_template: row.get("report_template")?,
        is_productive: row.get("is_productive")?,
        is_system: row.get("is_system")?,
        archived: row.get("archived")?,
        sort_order: row.get("sort_order")?,
        created_at: dt(row.get("created_at")?)?,
    })
}

impl CategoryRepo for SqliteStore {
    fn list(&self, include_archived: bool) -> CoreResult<Vec<Category>> {
        let sql = if include_archived {
            SELECT_ALL
        } else {
            SELECT_ACTIVE
        };
        self.with(|conn| query_list(conn, sql, [], row_to_category))
    }

    fn get(&self, id: &str) -> CoreResult<Option<Category>> {
        self.with(|conn| query_opt(conn, SELECT_BY_ID, params![id], row_to_category))
    }

    fn upsert(&self, category: &Category) -> CoreResult<()> {
        self.with(|conn| {
            execute(
                conn,
                UPSERT,
                named_params! {
                    ":id": category.id,
                    ":name": category.name,
                    ":color": category.color,
                    ":icon": category.icon,
                    ":description": category.description,
                    ":keywords": json(&category.keywords)?,
                    ":report_time": category.report_time.map(time_str),
                    ":report_template": category.report_template,
                    ":is_productive": category.is_productive,
                    ":is_system": category.is_system,
                    ":archived": category.archived,
                    ":sort_order": category.sort_order,
                    ":created_at": ms(category.created_at),
                },
            )?;
            Ok(())
        })
    }

    /// System categories are never deleted (`CoreError::Invalid`). Deleting a user category
    /// cascades to its rules, corrections and reports; its blocks fall back to "no category".
    fn delete(&self, id: &str) -> CoreResult<()> {
        self.with(|conn| {
            let is_system: Option<bool> =
                query_opt(conn, SELECT_IS_SYSTEM, params![id], |row| Ok(row.get(0)?))?;
            match is_system {
                Some(true) => Err(StorageError::Invalid(format!(
                    "system category {id} cannot be deleted"
                ))),
                Some(false) => execute(conn, DELETE, params![id]).map(|_| ()),
                None => Ok(()),
            }
        })
    }
}
