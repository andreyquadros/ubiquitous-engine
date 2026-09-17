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
const PRAGMAS: &str = "PRAGMA journal_mode = WAL; \
                       PRAGMA synchronous = NORMAL; \
                       PRAGMA busy_timeout = 5000; \
                       PRAGMA foreign_keys = ON;";

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
