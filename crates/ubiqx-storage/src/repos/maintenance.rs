//! [`MaintenanceRepo`] implementation: the "delete all data" path.

use ubiqx_core::ports::MaintenanceRepo;
use ubiqx_core::{CoreResult, RuleOrigin};

use crate::store::SqliteStore;

/// Screenshots go first so their `block_id` reference never dangles mid-transaction.
/// `blocks` is cleared without an `is_open` filter, which also removes any orphaned open row.
const WIPE: [&str; 7] = [
    "DELETE FROM screenshots",
    "DELETE FROM blocks",
    "DELETE FROM corrections",
    "DELETE FROM reports",
    "DELETE FROM nudges",
    "DELETE FROM ai_usage",
    "DELETE FROM kv",
];

const DELETE_LEARNED_RULES: &str = "DELETE FROM rules WHERE origin = ?1";

/// Freed pages still hold the deleted text until the file is rebuilt; the checkpoint folds the
/// WAL back into the main file first so the old rows do not linger there either.
const COMPACT: &str = "PRAGMA wal_checkpoint(TRUNCATE); VACUUM;";

impl MaintenanceRepo for SqliteStore {
    fn wipe_user_data(&self) -> CoreResult<()> {
        self.with(|conn| {
            let tx = conn.transaction()?;
            for sql in WIPE {
                tx.execute(sql, [])?;
            }
            tx.execute(DELETE_LEARNED_RULES, [RuleOrigin::Learned.as_str()])?;
            tx.commit()?;
            conn.execute_batch(COMPACT)?;
            tracing::info!("user data wiped");
            Ok(())
        })
    }
}
