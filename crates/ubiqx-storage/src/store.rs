//! [`SqliteStore`] and the small query helpers shared by the repository implementations.

use rusqlite::{Connection, Params, Row};
use ubiqx_core::CoreResult;

use crate::db::Db;
use crate::error::{StorageError, StorageResult};

/// Implements every persistence port of `ubiqx_core::ports` (including `FocusRepo`) on top
/// of a [`Db`].
///
/// Clone it freely: all clones share the same connection. Because several ports declare
/// methods with the same name (`insert`, `get`, `upsert`, `delete`, …), call them through the
/// trait (`BlockRepo::get(&store, id)`) or through a `&dyn BlockRepo` coercion.
#[derive(Clone, Debug)]
pub struct SqliteStore {
    db: Db,
}

impl SqliteStore {
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    /// The underlying connection handle, for ad-hoc queries and maintenance.
    pub fn db(&self) -> &Db {
        &self.db
    }

    /// Runs `f` on the connection and maps the error onto `CoreError`.
    pub(crate) fn with<R>(
        &self,
        f: impl FnOnce(&mut Connection) -> StorageResult<R>,
    ) -> CoreResult<R> {
        self.db.with(f).map_err(Into::into)
    }
}

/// Runs a query and maps every row.
pub(crate) fn query_list<T, P: Params>(
    conn: &Connection,
    sql: &str,
    params: P,
    map: impl FnMut(&Row<'_>) -> StorageResult<T>,
) -> StorageResult<Vec<T>> {
    let mut stmt = conn.prepare_cached(sql)?;
    let rows = stmt.query_and_then(params, map)?;
    rows.collect()
}

/// Runs a query and maps its first row, if any.
pub(crate) fn query_opt<T, P: Params>(
    conn: &Connection,
    sql: &str,
    params: P,
    map: impl FnMut(&Row<'_>) -> StorageResult<T>,
) -> StorageResult<Option<T>> {
    let mut stmt = conn.prepare_cached(sql)?;
    let mut rows = stmt.query_and_then(params, map)?;
    rows.next().transpose()
}

/// Runs a query that always yields exactly one row (aggregates).
pub(crate) fn query_one<T, P: Params>(
    conn: &Connection,
    sql: &str,
    params: P,
    map: impl FnMut(&Row<'_>) -> StorageResult<T>,
) -> StorageResult<T> {
    query_opt(conn, sql, params, map)?
        .ok_or_else(|| StorageError::Decode(format!("query returned no row: {sql}")))
}

/// Executes a statement and returns the number of affected rows.
pub(crate) fn execute<P: Params>(conn: &Connection, sql: &str, params: P) -> StorageResult<u64> {
    let changed = conn.prepare_cached(sql)?.execute(params)?;
    Ok(changed as u64)
}

/// Executes an update that must touch the row identified by `id`, failing with
/// [`StorageError::NotFound`] when it does not exist.
pub(crate) fn execute_expecting_row<P: Params>(
    conn: &Connection,
    sql: &str,
    params: P,
    what: &str,
    id: &str,
) -> StorageResult<()> {
    if execute(conn, sql, params)? == 0 {
        return Err(StorageError::NotFound(format!("{what} {id}")));
    }
    Ok(())
}
