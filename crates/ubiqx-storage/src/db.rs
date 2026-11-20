//! Connection handle: opening, pragmas, migrations and serialised access.

use std::fmt;
use std::path::Path;
use std::sync::Arc;

use parking_lot::Mutex;
use rusqlite::Connection;

use crate::error::{StorageError, StorageResult};
use crate::migrations;

/// Pragmas applied to every connection before migrations run.
///
/// * WAL lets readers proceed while a write is in flight and recovers well from crashes.
/// * `synchronous = NORMAL` is durable enough under WAL and avoids an fsync per transaction.
/// * `busy_timeout` makes a second process (the CLI next to the app) wait instead of failing.
/// * `foreign_keys` enforces the `REFERENCES` clauses of the schema.
/// * `secure_delete` makes SQLite overwrite with zeros whatever it frees: a deleted row,
///   the old version of a replaced one, a page that is no longer used. Without it that
///   text stays readable in the file until something else happens to be written over it.
///   The setting belongs to the connection, not to the file, which is why it is here. The
///   overwritten page reaches the main file with the next checkpoint of the write-ahead
///   log; the erasures that must leave nothing behind run that checkpoint themselves.
const PRAGMAS: &str = "PRAGMA journal_mode = WAL; \
                       PRAGMA synchronous = NORMAL; \
                       PRAGMA busy_timeout = 5000; \
                       PRAGMA foreign_keys = ON; \
                       PRAGMA secure_delete = ON;";

/// A cheap-to-clone handle to one SQLite connection.
///
/// All access is serialised through a mutex: SQLite is fast enough for this workload and a
/// single writer avoids `SQLITE_BUSY` entirely. Use [`Db::with`] from synchronous code and
/// [`Db::run`] from async code; the guard is never held across an `.await`.
#[derive(Clone)]
pub struct Db {
    conn: Arc<Mutex<Connection>>,
}

impl fmt::Debug for Db {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Db").finish_non_exhaustive()
    }
}

impl Db {
    /// Opens (or creates) the database at `path`: creates parent directories, applies the
    /// connection pragmas and runs any pending migration.
    pub fn open(path: &Path) -> StorageResult<Self> {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        tracing::debug!(path = %path.display(), "opening sqlite database");
        Self::init(Connection::open(path)?)
    }

    /// Opens a fresh, private in-memory database (tests, CLI dry runs).
    pub fn open_in_memory() -> StorageResult<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(mut conn: Connection) -> StorageResult<Self> {
        conn.execute_batch(PRAGMAS)?;
        migrations::apply(&mut conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Runs `f` with exclusive access to the connection, then releases it.
    pub fn with<R>(&self, f: impl FnOnce(&mut Connection) -> StorageResult<R>) -> StorageResult<R> {
        let mut guard = self.conn.lock();
        f(&mut guard)
    }

    /// Runs `f` on tokio's blocking thread pool so an async task never blocks on the mutex or
    /// on disk I/O. Must be called from within a tokio runtime.
    pub async fn run<R: Send + 'static>(
        &self,
        f: impl FnOnce(&mut Connection) -> StorageResult<R> + Send + 'static,
    ) -> StorageResult<R> {
        let db = self.clone();
        tokio::task::spawn_blocking(move || db.with(f))
            .await
            .map_err(|e| StorageError::Join(e.to_string()))?
    }

    /// Current schema version (`0` before the first migration ran).
    pub fn schema_version(&self) -> StorageResult<usize> {
        self.with(|conn| migrations::current_version(conn))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const DELETED_KEY: &str = "chave-da-linha-apagada";
    const DELETED_VALUE: &str = "texto-da-linha-apagada.example";
    const REPLACED_VALUE: &str = "texto-antigo-substituido.example";
    const KEPT_VALUE: &str = "texto-da-linha-mantida.example";

    const CHECKPOINT: &str = "PRAGMA wal_checkpoint(TRUNCATE)";

    /// The write-ahead log SQLite keeps beside the database at `path`.
    fn wal_of(path: &Path) -> PathBuf {
        let mut name = path.as_os_str().to_owned();
        name.push("-wal");
        PathBuf::from(name)
    }

    /// Whether `needle` occurs in the database file or in its write-ahead log.
    fn on_disk(path: &Path, needle: &str) -> bool {
        [path.to_path_buf(), wal_of(path)].iter().any(|file| {
            std::fs::read(file)
                .map(|bytes| {
                    bytes
                        .windows(needle.len())
                        .any(|window| window == needle.as_bytes())
                })
                .unwrap_or(false)
        })
    }

    fn secure_delete(conn: &Connection) -> i64 {
        conn.query_row("PRAGMA secure_delete", [], |row| row.get(0))
            .expect("pragma")
    }

    /// Stores five rows, folds the log into the file, then replaces the value of one row
    /// and deletes another, and folds the log again. Nothing is rebuilt (no `VACUUM`).
    ///
    /// The new value is longer than the old one, so it cannot be written into the place
    /// the old one leaves free: whether the old text is still there afterwards depends on
    /// the connection overwriting it, not on what happens to be written next.
    fn store_delete_and_replace(conn: &Connection, path: &Path) {
        conn.execute_batch("CREATE TABLE IF NOT EXISTS notes (key TEXT PRIMARY KEY, value TEXT)")
            .expect("table");
        for (key, value) in [
            ("mantida-1", KEPT_VALUE),
            (DELETED_KEY, DELETED_VALUE),
            ("mantida-2", "outro texto mantido"),
            ("substituida", REPLACED_VALUE),
            ("mantida-3", "mais um texto mantido"),
        ] {
            conn.execute(
                "INSERT INTO notes (key, value) VALUES (?1, ?2)",
                [key, value],
            )
            .expect("insert");
        }
        conn.query_row(CHECKPOINT, [], |_| Ok(()))
            .expect("checkpoint");
        for stored in [DELETED_KEY, DELETED_VALUE, REPLACED_VALUE, KEPT_VALUE] {
            assert!(on_disk(path, stored), "the search finds `{stored}`");
        }

        let longer = format!("{REPLACED_VALUE} {REPLACED_VALUE}").replace("antigo", "novo");
        let replaced = conn
            .execute(
                "UPDATE notes SET value = ?1 WHERE key = 'substituida'",
                [&longer],
            )
            .expect("update");
        assert_eq!(replaced, 1);
        let deleted = conn
            .execute("DELETE FROM notes WHERE key = ?1", [DELETED_KEY])
            .expect("delete");
        assert_eq!(deleted, 1);
        conn.query_row(CHECKPOINT, [], |_| Ok(()))
            .expect("checkpoint");
        let left: i64 = conn
            .query_row("SELECT COUNT(*) FROM notes", [], |row| row.get(0))
            .expect("count");
        assert_eq!(left, 4);
    }

    #[test]
    fn every_connection_overwrites_what_it_deletes() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("ubiqx.sqlite");
        let first = Db::open(&path).expect("open");
        assert_eq!(first.with(|conn| Ok(secure_delete(conn))).expect("with"), 1);
        // The setting is the connection's: a second one on the same file, and one opened
        // after the first is gone, set it again.
        let second = Db::open(&path).expect("open again");
        assert_eq!(
            second.with(|conn| Ok(secure_delete(conn))).expect("with"),
            1
        );
        drop(first);
        drop(second);
        let reopened = Db::open(&path).expect("reopen");
        assert_eq!(
            reopened.with(|conn| Ok(secure_delete(conn))).expect("with"),
            1
        );
        let memory = Db::open_in_memory().expect("memory");
        assert_eq!(
            memory.with(|conn| Ok(secure_delete(conn))).expect("with"),
            1
        );
    }

    #[test]
    fn a_deleted_row_and_a_replaced_value_do_not_stay_in_the_file() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("ubiqx.sqlite");
        let db = Db::open(&path).expect("open");
        db.with(|conn| {
            store_delete_and_replace(conn, &path);
            Ok(())
        })
        .expect("with");

        for gone in [DELETED_KEY, DELETED_VALUE, REPLACED_VALUE] {
            assert!(!on_disk(&path, gone), "`{gone}` is still in the file");
        }
        assert!(on_disk(&path, KEPT_VALUE), "what was kept is still there");
        let kept: String = db
            .with(|conn| {
                Ok(conn.query_row(
                    "SELECT value FROM notes WHERE key = 'mantida-1'",
                    [],
                    |row| row.get(0),
                )?)
            })
            .expect("select");
        assert_eq!(kept, KEPT_VALUE);
    }

    #[test]
    fn without_the_pragma_the_deleted_text_stays_in_the_file() {
        // The same steps on a connection that does not overwrite: this is what the pragma
        // is there for, and what makes the test above able to fail.
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("plain.sqlite");
        let conn = Connection::open(&path).expect("open");
        conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA secure_delete = OFF;")
            .expect("pragmas");
        assert_eq!(secure_delete(&conn), 0);
        store_delete_and_replace(&conn, &path);

        for left_behind in [DELETED_KEY, DELETED_VALUE, REPLACED_VALUE] {
            assert!(
                on_disk(&path, left_behind),
                "`{left_behind}` was expected to stay without the pragma"
            );
        }
        assert!(on_disk(&path, KEPT_VALUE));
    }
}
