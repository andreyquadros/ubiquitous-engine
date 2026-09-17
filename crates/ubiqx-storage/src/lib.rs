//! # ubiqx-storage
//!
//! SQLite persistence adapter for ubiqX. It implements every persistence port declared in
//! [`ubiqx_core::ports`] on top of a single [`rusqlite`] connection guarded by a mutex.
//!
//! * [`Db`] — cheap-to-clone connection handle. Opens the file, applies the connection pragmas
//!   and runs the embedded migrations.
//! * [`SqliteStore`] — implements `BlockRepo`, `CategoryRepo`, `RuleRepo`, `CorrectionRepo`,
//!   `ScreenshotRepo`, `ReportRepo`, `NudgeRepo`, `SettingsRepo`, `UsageRepo`, `KvRepo` and
//!   `MaintenanceRepo`.
//! * [`StorageError`] — the adapter's error type, convertible into [`ubiqx_core::CoreError`].
//!
//! ## Storage conventions
//!
//! * Timestamps are `INTEGER` unix milliseconds (UTC). Sub-millisecond precision is dropped.
//! * Calendar dates are `TEXT` `YYYY-MM-DD`; times of day are `TEXT` `HH:MM`.
//! * Enums are stored through their `as_str()` / `parse()` helpers from `ubiqx-core`.
//! * Lists and nested structs (`Category::keywords`, report items and highlights, the whole
//!   [`ubiqx_core::Settings`] struct) are JSON `TEXT` columns, so new fields with serde
//!   defaults need no migration.
//! * Booleans are `INTEGER` `0`/`1`.
//!
//! ```no_run
//! use std::path::Path;
//! use ubiqx_core::ports::KvRepo;
//! use ubiqx_storage::{Db, SqliteStore};
//!
//! # fn main() -> Result<(), ubiqx_core::CoreError> {
//! let db = Db::open(Path::new("/tmp/ubiqx/ubiqx.sqlite"))?;
//! let store = SqliteStore::new(db);
//! store.set("last_report_check", "2026-09-17T18:00:00Z")?;
//! # Ok(())
//! # }
//! ```

mod convert;
mod db;
mod error;
pub mod migrations;
mod repos;
mod store;

pub use db::Db;
pub use error::{StorageError, StorageResult};
pub use store::SqliteStore;
