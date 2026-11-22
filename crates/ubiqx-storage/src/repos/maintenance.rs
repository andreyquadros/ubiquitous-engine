//! [`MaintenanceRepo`] implementation: the "delete all data" path.

use ubiqx_core::ports::MaintenanceRepo;
use ubiqx_core::{CoreResult, RuleOrigin};

use crate::store::SqliteStore;

/// Screenshots go first so their `block_id` reference never dangles mid-transaction.
/// `blocks` is cleared without an `is_open` filter, which also removes any orphaned open row.
/// Focus targets are configuration, not activity, and stay.
///
/// Of the `nofap` module only the counters go. Its configuration (`nofap_state`) and its
/// sites (`nofap_sites`) stay on purpose: wiping them would be a way to switch the
/// protection off without the delay every other way has to wait.
const WIPE: [&str; 10] = [
    "DELETE FROM screenshots",
    "DELETE FROM blocks",
    "DELETE FROM corrections",
    "DELETE FROM reports",
    "DELETE FROM nudges",
    "DELETE FROM ai_usage",
    "DELETE FROM kv",
    "DELETE FROM interventions",
    "DELETE FROM focus_sessions",
    "DELETE FROM nofap_day",
];

const DELETE_LEARNED_RULES: &str = "DELETE FROM rules WHERE origin = ?1";

/// Rebuilds the file without the freed pages. The write-ahead log is folded before (so the old
/// rows do not linger there) and after (`VACUUM` writes the rebuilt pages through the log).
const VACUUM: &str = "VACUUM";

impl MaintenanceRepo for SqliteStore {
    fn wipe_user_data(&self) -> CoreResult<()> {
        self.with(|conn| {
            let tx = conn.transaction()?;
            for sql in WIPE {
                tx.execute(sql, [])?;
            }
            tx.execute(DELETE_LEARNED_RULES, [RuleOrigin::Learned.as_str()])?;
            tx.commit()?;
            SqliteStore::fold_log_now(conn)?;
            conn.execute_batch(VACUUM)?;
            SqliteStore::fold_log_now(conn)?;
            tracing::info!("user data wiped");
            Ok(())
        })
    }
}
