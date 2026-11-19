//! Two products on one machine: only one process runs the guard and the AI check (design
//! section 1.6).
//!
//! The primary rewrites `<data_local_dir>/ubiqx-shield/primary.lock` every
//! [`HEARTBEAT_EVERY_SECS`]. A product writes the lock only while its own module is enabled
//! and its guard loop is running, and deletes it on disable and on quit. A heartbeat from
//! the other product that is at most [`STALE_AFTER_SECS`] old makes this process
//! `Secondary`; a heartbeat more than [`STALE_AFTER_SECS`] in the future is stale too. The
//! standalone app wins a tie.
//!
//! Seeded by the foundation unit: complete types and constants, signatures only.
#![allow(unused_variables)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ubiqx_core::ProductId;

/// Directory (under the user's local data directory) both products share, and the lock
/// file inside it.
pub const SHARED_DIR: &str = "ubiqx-shield";
pub const LOCK_FILE: &str = "primary.lock";
pub const HEARTBEAT_EVERY_SECS: i64 = 30;
pub const STALE_AFTER_SECS: i64 = 90;

/// Content of the lock file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrimaryLock {
    pub product: ProductId,
    pub pid: u32,
    pub heartbeat_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    /// This process runs the passes (and writes the heartbeat).
    Primary,
    /// The other product runs them; this process shows `overall: secondary`.
    Secondary,
}

/// The role of product `me` given the lock it read (`None`: no file, or unreadable).
pub fn decide(lock: Option<&PrimaryLock>, me: ProductId, now: DateTime<Utc>) -> Role {
    todo!()
}

/// Parses the lock file; anything that is not a valid lock is `None`.
pub fn parse_lock(text: &str) -> Option<PrimaryLock> {
    todo!()
}

/// The lock file's content for `lock` (one line of JSON with a final newline).
pub fn render_lock(lock: &PrimaryLock) -> String {
    todo!()
}
