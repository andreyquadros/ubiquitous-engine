//! [`BlockRepo`] implementation.

use chrono::{DateTime, Utc};
use rusqlite::{named_params, params, Connection, Row, ToSql};
use ubiqx_core::ports::BlockRepo;
use ubiqx_core::{
    new_id, ActivityBlock, AppTotal, CategoryTotal, ClassificationSource, CoreResult, Id, TimeRange,
};

use crate::convert::{dt, ms, opt_dt, opt_enum, opt_ms, sql_limit};
use crate::error::{StorageError, StorageResult};
use crate::store::{execute, execute_expecting_row, query_list, query_opt, SqliteStore};

macro_rules! block_cols {
    () => {
        "id, started_at, ended_at, app_name, app_id, title, title_key, url, domain, \
         category_id, confidence, source, description, screenshot_id, sample_count, is_open, \
         classify_attempts, next_attempt_at, needs_review, ai_payload, ai_sent_at, is_manual, \
         note"
    };
}

const SELECT_BY_ID: &str = concat!("SELECT ", block_cols!(), " FROM blocks WHERE id = ?1");

const SELECT_OPEN: &str = concat!(
    "SELECT ",
    block_cols!(),
    " FROM blocks WHERE is_open = 1 ORDER BY started_at DESC LIMIT 1"
);

/// Closed blocks overlapping the half-open range `[from, to)`.
const SELECT_IN_RANGE: &str = concat!(
    "SELECT ",
    block_cols!(),
    " FROM blocks WHERE is_open = 0 AND started_at < :to AND ended_at > :from \
     ORDER BY started_at ASC"
);

const SELECT_PENDING_REMOTE: &str = concat!(
    "SELECT ",
    block_cols!(),
    " FROM blocks WHERE is_open = 0 AND category_id IS NULL AND needs_review = 0 \
     AND (next_attempt_at IS NULL OR next_attempt_at <= :now) \
     AND (source IS NULL OR source != 'user') \
     ORDER BY started_at ASC LIMIT :limit"
);

const SELECT_UNCLASSIFIED: &str = concat!(
    "SELECT ",
    block_cols!(),
    " FROM blocks WHERE is_open = 0 AND category_id IS NULL ORDER BY started_at ASC LIMIT ?1"
);

const SELECT_NEEDS_REVIEW: &str = concat!(
    "SELECT ",
    block_cols!(),
    " FROM blocks WHERE needs_review = 1 ORDER BY started_at DESC LIMIT ?1"
);

const SELECT_USER_CLASSIFIED: &str = concat!(
    "SELECT ",
    block_cols!(),
    " FROM blocks WHERE source = 'user' AND category_id IS NOT NULL \
     ORDER BY started_at DESC LIMIT ?1"
);

const INSERT: &str = concat!(
    "INSERT INTO blocks (",
    block_cols!(),
    ") VALUES (:id, :started_at, :ended_at, :app_name, :app_id, :title, :title_key, :url, \
     :domain, :category_id, :confidence, :source, :description, :screenshot_id, :sample_count, \
     :is_open, :classify_attempts, :next_attempt_at, :needs_review, :ai_payload, :ai_sent_at, \
     :is_manual, :note)"
);

const UPDATE: &str = "UPDATE blocks SET started_at = :started_at, ended_at = :ended_at, \
     app_name = :app_name, app_id = :app_id, title = :title, title_key = :title_key, \
     url = :url, domain = :domain, category_id = :category_id, confidence = :confidence, \
     source = :source, description = :description, screenshot_id = :screenshot_id, \
     sample_count = :sample_count, is_open = :is_open, classify_attempts = :classify_attempts, \
     next_attempt_at = :next_attempt_at, needs_review = :needs_review, \
     ai_payload = :ai_payload, ai_sent_at = :ai_sent_at, is_manual = :is_manual, note = :note \
     WHERE id = :id";

/// The columns `Segmenter::feed`/`close` mutate on an existing block; everything else
/// (classification, screenshot link, retry state) is owned by other writers.
const TOUCH: &str = "UPDATE blocks SET ended_at = :ended_at, sample_count = :sample_count, \
     title = :title, url = :url, is_open = :is_open WHERE id = :id";

const SET_SCREENSHOT: &str = "UPDATE blocks SET screenshot_id = :screenshot_id WHERE id = :id";

const RECORD_ATTEMPT: &str = "UPDATE blocks SET classify_attempts = :attempts, \
     next_attempt_at = :next_attempt_at, needs_review = :needs_review WHERE id = :id";

const SET_AI_PAYLOAD: &str =
    "UPDATE blocks SET ai_payload = :payload, ai_sent_at = :at WHERE id = :id";

/// `description` is only overwritten when a new one is given; a classification that assigns a
/// category also resolves the "needs review" flag.
const SET_CLASSIFICATION: &str = "UPDATE blocks SET category_id = :category_id, \
     confidence = :confidence, source = :source, \
     description = COALESCE(:description, description), \
     needs_review = CASE WHEN :category_id IS NULL THEN needs_review ELSE 0 END \
     WHERE id = :id";

const TRIM_HEAD: &str =
    "UPDATE blocks SET ended_at = :ended_at, is_open = 0, sample_count = :sample_count \
     WHERE id = :id";

const BACKFILL: &str = "UPDATE blocks SET category_id = :category_id, source = :source, \
     confidence = :confidence, needs_review = 0, next_attempt_at = NULL \
     WHERE is_open = 0 AND started_at < :to AND ended_at > :from \
     AND lower(app_id) = lower(:app_id) \
     AND ((:domain IS NULL AND domain IS NULL) OR lower(domain) = lower(:domain)) \
     AND (source IS NULL OR source != 'user') \
     AND (category_id IS NULL OR category_id != :category_id)";

/// Seconds are summed after clipping every block to the range, in milliseconds.
const TOTALS_BY_CATEGORY: &str = "SELECT category_id, \
     SUM(MIN(ended_at, :to) - MAX(started_at, :from)) AS clipped_ms, COUNT(*) AS n \
     FROM blocks WHERE is_open = 0 AND started_at < :to AND ended_at > :from \
     GROUP BY category_id ORDER BY clipped_ms DESC";

const TOTALS_BY_APP: &str = "SELECT app_id, MAX(app_name) AS app_name, \
     SUM(MIN(ended_at, :to) - MAX(started_at, :from)) AS clipped_ms \
     FROM blocks WHERE is_open = 0 AND started_at < :to AND ended_at > :from \
     GROUP BY app_id ORDER BY clipped_ms DESC LIMIT :limit";

const DELETE: &str = "DELETE FROM blocks WHERE id = ?1";

const DELETE_BEFORE: &str = "DELETE FROM blocks WHERE is_open = 0 AND ended_at < ?1";

/// Values of a block that need an owned SQL representation (timestamps, enum tags).
struct Scratch {
    started_at: i64,
    ended_at: i64,
    source: Option<&'static str>,
    next_attempt_at: Option<i64>,
    ai_sent_at: Option<i64>,
}

impl Scratch {
    fn of(b: &ActivityBlock) -> Self {
        Self {
            started_at: ms(b.started_at),
            ended_at: ms(b.ended_at),
            source: b.source.map(|s| s.as_str()),
            next_attempt_at: opt_ms(b.next_attempt_at),
            ai_sent_at: opt_ms(b.ai_sent_at),
        }
    }
}

/// The full named-parameter set shared by [`INSERT`] and [`UPDATE`].
fn block_params<'a>(b: &'a ActivityBlock, s: &'a Scratch) -> [(&'static str, &'a dyn ToSql); 23] {
    [
        (":id", &b.id),
        (":started_at", &s.started_at),
        (":ended_at", &s.ended_at),
        (":app_name", &b.app_name),
        (":app_id", &b.app_id),
        (":title", &b.title),
        (":title_key", &b.title_key),
        (":url", &b.url),
        (":domain", &b.domain),
        (":category_id", &b.category_id),
        (":confidence", &b.confidence),
        (":source", &s.source),
        (":description", &b.description),
        (":screenshot_id", &b.screenshot_id),
        (":sample_count", &b.sample_count),
        (":is_open", &b.is_open),
        (":classify_attempts", &b.classify_attempts),
        (":next_attempt_at", &s.next_attempt_at),
        (":needs_review", &b.needs_review),
        (":ai_payload", &b.ai_payload),
        (":ai_sent_at", &s.ai_sent_at),
        (":is_manual", &b.is_manual),
        (":note", &b.note),
    ]
}

fn row_to_block(row: &Row<'_>) -> StorageResult<ActivityBlock> {
    Ok(ActivityBlock {
        id: row.get("id")?,
        started_at: dt(row.get("started_at")?)?,
        ended_at: dt(row.get("ended_at")?)?,
        app_name: row.get("app_name")?,
        app_id: row.get("app_id")?,
        title: row.get("title")?,
        title_key: row.get("title_key")?,
        url: row.get("url")?,
        domain: row.get("domain")?,
        category_id: row.get("category_id")?,
        confidence: row.get("confidence")?,
        source: opt_enum(
            "classification source",
            row.get("source")?,
            ClassificationSource::parse,
        )?,
        description: row.get("description")?,
        screenshot_id: row.get("screenshot_id")?,
        sample_count: row.get("sample_count")?,
        is_open: row.get("is_open")?,
        classify_attempts: row.get("classify_attempts")?,
        next_attempt_at: opt_dt(row.get("next_attempt_at")?)?,
        needs_review: row.get("needs_review")?,
        ai_payload: row.get("ai_payload")?,
        ai_sent_at: opt_dt(row.get("ai_sent_at")?)?,
        is_manual: row.get("is_manual")?,
        note: row.get("note")?,
    })
}

fn row_to_category_total(row: &Row<'_>) -> StorageResult<CategoryTotal> {
    let clipped_ms: i64 = row.get("clipped_ms")?;
    Ok(CategoryTotal {
        category_id: row.get("category_id")?,
        secs: clipped_ms / 1000,
        block_count: row.get("n")?,
    })
}

fn row_to_app_total(row: &Row<'_>) -> StorageResult<AppTotal> {
    let clipped_ms: i64 = row.get("clipped_ms")?;
    Ok(AppTotal {
        app_id: row.get("app_id")?,
        app_name: row.get("app_name")?,
        secs: clipped_ms / 1000,
    })
}

fn insert_block(conn: &Connection, block: &ActivityBlock) -> StorageResult<()> {
    let scratch = Scratch::of(block);
    execute(conn, INSERT, &block_params(block, &scratch)[..])?;
    Ok(())
}

fn get_block(conn: &Connection, id: &str) -> StorageResult<Option<ActivityBlock>> {
    query_opt(conn, SELECT_BY_ID, params![id], row_to_block)
}

/// Confidence written by a backfill: user decisions are certain, everything else is a strong
/// but not absolute signal (same cap as the memory classifier).
fn backfill_confidence(source: ClassificationSource) -> f32 {
    match source {
        ClassificationSource::User => 1.0,
        _ => 0.9,
    }
}

impl BlockRepo for SqliteStore {
    fn insert(&self, block: &ActivityBlock) -> CoreResult<()> {
        self.with(|conn| insert_block(conn, block))
    }

    fn update(&self, block: &ActivityBlock) -> CoreResult<()> {
        self.with(|conn| {
            let scratch = Scratch::of(block);
            execute_expecting_row(
                conn,
                UPDATE,
                &block_params(block, &scratch)[..],
                "block",
                &block.id,
            )
        })
    }

    fn touch(&self, block: &ActivityBlock) -> CoreResult<()> {
        self.with(|conn| {
            execute_expecting_row(
                conn,
                TOUCH,
                named_params! {
                    ":id": block.id,
                    ":ended_at": ms(block.ended_at),
                    ":sample_count": block.sample_count,
                    ":title": block.title,
                    ":url": block.url,
                    ":is_open": block.is_open,
                },
                "block",
                &block.id,
            )
        })
    }

    fn set_screenshot(&self, id: &str, screenshot_id: &str) -> CoreResult<()> {
        self.with(|conn| {
            execute_expecting_row(
                conn,
                SET_SCREENSHOT,
                named_params! { ":id": id, ":screenshot_id": screenshot_id },
                "block",
                id,
            )
        })
    }

    fn get(&self, id: &str) -> CoreResult<Option<ActivityBlock>> {
        self.with(|conn| get_block(conn, id))
    }

    fn open_block(&self) -> CoreResult<Option<ActivityBlock>> {
        self.with(|conn| query_opt(conn, SELECT_OPEN, [], row_to_block))
    }

    fn list_in_range(&self, range: TimeRange) -> CoreResult<Vec<ActivityBlock>> {
        self.with(|conn| {
            query_list(
                conn,
                SELECT_IN_RANGE,
                named_params! { ":from": ms(range.from), ":to": ms(range.to) },
                row_to_block,
            )
        })
    }

    fn list_pending_remote(
        &self,
        now: DateTime<Utc>,
        limit: usize,
    ) -> CoreResult<Vec<ActivityBlock>> {
        self.with(|conn| {
            query_list(
                conn,
                SELECT_PENDING_REMOTE,
                named_params! { ":now": ms(now), ":limit": sql_limit(limit) },
                row_to_block,
            )
        })
    }

    fn list_unclassified(&self, limit: usize) -> CoreResult<Vec<ActivityBlock>> {
        self.with(|conn| {
            query_list(
                conn,
                SELECT_UNCLASSIFIED,
                params![sql_limit(limit)],
                row_to_block,
            )
        })
    }

    fn list_needs_review(&self, limit: usize) -> CoreResult<Vec<ActivityBlock>> {
        self.with(|conn| {
            query_list(
                conn,
                SELECT_NEEDS_REVIEW,
                params![sql_limit(limit)],
                row_to_block,
            )
        })
    }

    fn record_attempt(
        &self,
        id: &str,
        attempts: u32,
        next_attempt_at: Option<DateTime<Utc>>,
        needs_review: bool,
    ) -> CoreResult<()> {
        self.with(|conn| {
            execute_expecting_row(
                conn,
                RECORD_ATTEMPT,
                named_params! {
                    ":id": id,
                    ":attempts": attempts,
                    ":next_attempt_at": opt_ms(next_attempt_at),
                    ":needs_review": needs_review,
                },
                "block",
                id,
            )
        })
    }

    fn set_ai_payload(&self, id: &str, payload: &str, at: DateTime<Utc>) -> CoreResult<()> {
        self.with(|conn| {
            execute_expecting_row(
                conn,
                SET_AI_PAYLOAD,
                named_params! { ":id": id, ":payload": payload, ":at": ms(at) },
                "block",
                id,
            )
        })
    }

    /// The new block covers `[at, ended_at)` and inherits app, title, URL, domain,
    /// classification, description and manual flag/note; the original is trimmed to
    /// `[started_at, at)`. Sample counts are apportioned by duration. Screenshot link, AI
    /// payload and retry state are not copied. An open block is rejected with
    /// [`CoreError::Invalid`](ubiqx_core::CoreError::Invalid): the segmenter owns it in memory
    /// and would re-open the head on the next sample, leaving an orphaned open tail.
    fn split(&self, id: &str, at: DateTime<Utc>) -> CoreResult<Id> {
        self.with(|conn| {
            let tx = conn.transaction()?;
            let head =
                get_block(&tx, id)?.ok_or_else(|| StorageError::NotFound(format!("block {id}")))?;
            if head.is_open {
                return Err(StorageError::Invalid(format!(
                    "block {id} is still open and cannot be split"
                )));
            }
            if at <= head.started_at || at >= head.ended_at {
                return Err(StorageError::Invalid(format!(
                    "split point {at} is not strictly inside block {id} [{}, {})",
                    head.started_at, head.ended_at
                )));
            }
            let total_ms = (head.ended_at - head.started_at).num_milliseconds().max(1);
            let tail_ms = (head.ended_at - at).num_milliseconds();
            let tail_samples = (i64::from(head.sample_count) * tail_ms / total_ms) as u32;
            let tail = ActivityBlock {
                id: new_id(),
                started_at: at,
                screenshot_id: None,
                sample_count: tail_samples,
                is_open: false,
                classify_attempts: 0,
                next_attempt_at: None,
                ai_payload: None,
                ai_sent_at: None,
                ..head.clone()
            };
            insert_block(&tx, &tail)?;
            execute(
                &tx,
                TRIM_HEAD,
                named_params! {
                    ":id": id,
                    ":ended_at": ms(at),
                    ":sample_count": head.sample_count - tail_samples,
                },
            )?;
            tx.commit()?;
            tracing::debug!(block = id, new_block = %tail.id, at = %at, "split block");
            Ok(tail.id)
        })
    }

    fn backfill_category(
        &self,
        app_id: &str,
        domain: Option<&str>,
        range: TimeRange,
        category_id: &str,
        source: ClassificationSource,
    ) -> CoreResult<u64> {
        self.with(|conn| {
            let changed = execute(
                conn,
                BACKFILL,
                named_params! {
                    ":app_id": app_id,
                    ":domain": domain,
                    ":from": ms(range.from),
                    ":to": ms(range.to),
                    ":category_id": category_id,
                    ":source": source.as_str(),
                    ":confidence": backfill_confidence(source),
                },
            )?;
            tracing::debug!(app_id, ?domain, category_id, changed, "backfilled category");
            Ok(changed)
        })
    }

    fn delete(&self, id: &str) -> CoreResult<()> {
        self.with(|conn| execute(conn, DELETE, params![id]).map(|_| ()))
    }

    fn set_classification(
        &self,
        id: &str,
        category_id: Option<&str>,
        confidence: f32,
        source: ClassificationSource,
        description: Option<&str>,
    ) -> CoreResult<()> {
        self.with(|conn| {
            execute_expecting_row(
                conn,
                SET_CLASSIFICATION,
                named_params! {
                    ":id": id,
                    ":category_id": category_id,
                    ":confidence": confidence,
                    ":source": source.as_str(),
                    ":description": description,
                },
                "block",
                id,
            )
        })
    }

    fn totals_by_category(&self, range: TimeRange) -> CoreResult<Vec<CategoryTotal>> {
        self.with(|conn| {
            query_list(
                conn,
                TOTALS_BY_CATEGORY,
                named_params! { ":from": ms(range.from), ":to": ms(range.to) },
                row_to_category_total,
            )
        })
    }

    fn totals_by_app(&self, range: TimeRange, limit: usize) -> CoreResult<Vec<AppTotal>> {
        self.with(|conn| {
            query_list(
                conn,
                TOTALS_BY_APP,
                named_params! { ":from": ms(range.from), ":to": ms(range.to), ":limit": sql_limit(limit) },
                row_to_app_total,
            )
        })
    }

    fn list_user_classified(&self, limit: usize) -> CoreResult<Vec<ActivityBlock>> {
        self.with(|conn| {
            query_list(
                conn,
                SELECT_USER_CLASSIFIED,
                params![sql_limit(limit)],
                row_to_block,
            )
        })
    }

    fn delete_before(&self, before: DateTime<Utc>) -> CoreResult<u64> {
        self.with(|conn| {
            let deleted = execute(conn, DELETE_BEFORE, params![ms(before)])?;
            tracing::debug!(before = %before, deleted, "purged old blocks");
            Ok(deleted)
        })
    }
}
