//! [`ReportRepo`] implementation.

use chrono::NaiveDate;
use rusqlite::{params, Row, ToSql};
use ubiqx_core::ports::ReportRepo;
use ubiqx_core::{CoreResult, DailyReport};

use crate::convert::{date, date_str, dt, from_json, json, ms};
use crate::error::StorageResult;
use crate::store::{execute, query_list, query_opt, SqliteStore};

macro_rules! report_cols {
    () => {
        "id, date, category_id, generated_at, summary_md, items, highlights, total_secs, model, \
         input_tokens, output_tokens, stale, edited"
    };
}

const SELECT_BY_KEY: &str = concat!(
    "SELECT ",
    report_cols!(),
    " FROM reports WHERE date = ?1 AND category_id = ?2"
);

const SELECT_FOR_DATE: &str = concat!(
    "SELECT ",
    report_cols!(),
    " FROM reports WHERE date = ?1 ORDER BY category_id ASC"
);

/// Both bounds inclusive: `list_between(1st, 31st)` is the whole month.
const SELECT_BETWEEN: &str = concat!(
    "SELECT ",
    report_cols!(),
    " FROM reports WHERE date >= ?1 AND date <= ?2 ORDER BY date ASC, category_id ASC"
);

const SELECT_ID_BY_KEY: &str = "SELECT id FROM reports WHERE date = ?1 AND category_id = ?2";

const INSERT: &str = concat!(
    "INSERT INTO reports (",
    report_cols!(),
    ") VALUES (:id, :date, :category_id, :generated_at, :summary_md, :items, :highlights, \
     :total_secs, :model, :input_tokens, :output_tokens, :stale, :edited)"
);

const UPDATE: &str = "UPDATE reports SET date = :date, category_id = :category_id, \
     generated_at = :generated_at, summary_md = :summary_md, items = :items, \
     highlights = :highlights, total_secs = :total_secs, model = :model, \
     input_tokens = :input_tokens, output_tokens = :output_tokens, stale = :stale, \
     edited = :edited WHERE id = :id";

const MARK_STALE: &str = "UPDATE reports SET stale = 1 WHERE date = ?1";

const DELETE: &str = "DELETE FROM reports WHERE id = ?1";

fn row_to_report(row: &Row<'_>) -> StorageResult<DailyReport> {
    let date_raw: String = row.get("date")?;
    let items: String = row.get("items")?;
    let highlights: String = row.get("highlights")?;
    Ok(DailyReport {
        id: row.get("id")?,
        date: date(&date_raw)?,
        category_id: row.get("category_id")?,
        generated_at: dt(row.get("generated_at")?)?,
        summary_md: row.get("summary_md")?,
        items: from_json(&items)?,
        highlights: from_json(&highlights)?,
        total_secs: row.get("total_secs")?,
        model: row.get("model")?,
        input_tokens: row.get("input_tokens")?,
        output_tokens: row.get("output_tokens")?,
        stale: row.get("stale")?,
        edited: row.get("edited")?,
    })
}

/// Owned SQL representations of a report's encoded columns, plus the id of the row written.
struct Scratch {
    id: String,
    date: String,
    generated_at: i64,
    items: String,
    highlights: String,
}

impl Scratch {
    fn of(r: &DailyReport, id: String) -> StorageResult<Self> {
        Ok(Self {
            id,
            date: date_str(r.date),
            generated_at: ms(r.generated_at),
            items: json(&r.items)?,
            highlights: json(&r.highlights)?,
        })
    }
}

/// Named parameters shared by [`INSERT`] and [`UPDATE`].
fn report_params<'a>(r: &'a DailyReport, s: &'a Scratch) -> [(&'static str, &'a dyn ToSql); 13] {
    [
        (":id", &s.id),
        (":date", &s.date),
        (":category_id", &r.category_id),
        (":generated_at", &s.generated_at),
        (":summary_md", &r.summary_md),
        (":items", &s.items),
        (":highlights", &s.highlights),
        (":total_secs", &r.total_secs),
        (":model", &r.model),
        (":input_tokens", &r.input_tokens),
        (":output_tokens", &r.output_tokens),
        (":stale", &r.stale),
        (":edited", &r.edited),
    ]
}

impl ReportRepo for SqliteStore {
    /// Keyed by `(date, category_id)`: an existing report is replaced but keeps its id, so
    /// links held by the UI stay valid across regenerations.
    fn upsert(&self, report: &DailyReport) -> CoreResult<()> {
        self.with(|conn| {
            let tx = conn.transaction()?;
            let existing: Option<String> = query_opt(
                &tx,
                SELECT_ID_BY_KEY,
                params![date_str(report.date), report.category_id],
                |row| Ok(row.get(0)?),
            )?;
            let sql = if existing.is_some() { UPDATE } else { INSERT };
            let scratch = Scratch::of(report, existing.unwrap_or_else(|| report.id.clone()))?;
            execute(&tx, sql, &report_params(report, &scratch)[..])?;
            tx.commit()?;
            Ok(())
        })
    }

    fn get(&self, date: NaiveDate, category_id: &str) -> CoreResult<Option<DailyReport>> {
        self.with(|conn| {
            query_opt(
                conn,
                SELECT_BY_KEY,
                params![date_str(date), category_id],
                row_to_report,
            )
        })
    }

    fn list_for_date(&self, date: NaiveDate) -> CoreResult<Vec<DailyReport>> {
        self.with(|conn| {
            query_list(
                conn,
                SELECT_FOR_DATE,
                params![date_str(date)],
                row_to_report,
            )
        })
    }

    fn list_between(&self, from: NaiveDate, to: NaiveDate) -> CoreResult<Vec<DailyReport>> {
        self.with(|conn| {
            query_list(
                conn,
                SELECT_BETWEEN,
                params![date_str(from), date_str(to)],
                row_to_report,
            )
        })
    }

    fn mark_stale(&self, date: NaiveDate) -> CoreResult<()> {
        self.with(|conn| execute(conn, MARK_STALE, params![date_str(date)]).map(|_| ()))
    }

    fn delete(&self, id: &str) -> CoreResult<()> {
        self.with(|conn| execute(conn, DELETE, params![id]).map(|_| ()))
    }
}
