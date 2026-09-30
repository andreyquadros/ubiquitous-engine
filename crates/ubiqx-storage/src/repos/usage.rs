//! [`UsageRepo`] implementation: the AI usage ledger.

use rusqlite::{named_params, params, Row};
use ubiqx_core::ports::{AiUsage, AiUsageTotals, UsageRepo};
use ubiqx_core::{CoreResult, TimeRange};

use crate::convert::{count, ms};
use crate::error::StorageResult;
use crate::store::{execute, query_one, SqliteStore};

const INSERT: &str = "INSERT INTO ai_usage (at, kind, model, input_tokens, output_tokens, \
     cache_read_tokens, cache_write_tokens, cost_usd) VALUES (:at, :kind, :model, \
     :input_tokens, :output_tokens, :cache_read_tokens, :cache_write_tokens, :cost_usd)";

const TOTALS: &str = "SELECT COUNT(*) AS calls, \
     COALESCE(SUM(input_tokens), 0) AS input_tokens, \
     COALESCE(SUM(output_tokens), 0) AS output_tokens, \
     COALESCE(SUM(cost_usd), 0.0) AS cost_usd \
     FROM ai_usage WHERE at >= ?1 AND at < ?2";

fn row_to_totals(row: &Row<'_>) -> StorageResult<AiUsageTotals> {
    Ok(AiUsageTotals {
        calls: count(row, "calls")?,
        input_tokens: count(row, "input_tokens")?,
        output_tokens: count(row, "output_tokens")?,
        cost_usd: row.get("cost_usd")?,
    })
}

impl UsageRepo for SqliteStore {
    fn record(&self, usage: &AiUsage) -> CoreResult<()> {
        self.with(|conn| {
            execute(
                conn,
                INSERT,
                named_params! {
                    ":at": ms(usage.at),
                    ":kind": usage.kind.as_str(),
                    ":model": usage.model,
                    ":input_tokens": usage.input_tokens,
                    ":output_tokens": usage.output_tokens,
                    ":cache_read_tokens": usage.cache_read_tokens,
                    ":cache_write_tokens": usage.cache_write_tokens,
                    ":cost_usd": usage.cost_usd,
                },
            )?;
            Ok(())
        })
    }

    fn totals(&self, range: TimeRange) -> CoreResult<AiUsageTotals> {
        self.with(|conn| {
            query_one(
                conn,
                TOTALS,
                params![ms(range.from), ms(range.to)],
                row_to_totals,
            )
        })
    }
}
