//! Adapter error type and its mapping onto [`CoreError`].

use thiserror::Error;
use ubiqx_core::CoreError;

/// Everything that can go wrong inside the storage adapter.
#[derive(Debug, Error)]
pub enum StorageError {
    /// The SQLite driver reported an error (constraint violation, I/O, busy…).
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    /// A schema migration failed, or the database was written by a newer build.
    #[error("migration: {0}")]
    Migration(#[from] rusqlite_migration::Error),
    /// A JSON column could not be (de)serialised.
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    /// Creating the database directory failed.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    /// A stored value could not be decoded into its domain type (unknown enum tag, bad date…).
    #[error("corrupt value: {0}")]
    Decode(String),
    /// The caller asked for something that violates a repository invariant.
    #[error("invalid input: {0}")]
    Invalid(String),
    /// The row targeted by an update does not exist.
    #[error("not found: {0}")]
    NotFound(String),
    /// The blocking task running the query panicked or was cancelled.
    #[error("background task failed: {0}")]
    Join(String),
}

/// Result alias used throughout the crate.
pub type StorageResult<T> = Result<T, StorageError>;

impl From<StorageError> for CoreError {
    /// `Invalid` and `NotFound` keep their meaning so the application layer can react to them;
    /// everything else becomes [`CoreError::Storage`].
    fn from(err: StorageError) -> Self {
        match err {
            StorageError::Invalid(msg) => CoreError::Invalid(msg),
            StorageError::NotFound(msg) => CoreError::NotFound(msg),
            other => CoreError::Storage(other.to_string()),
        }
    }
}
