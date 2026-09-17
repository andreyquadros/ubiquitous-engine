//! Schema migrations, embedded as SQL and applied through `rusqlite_migration`.
//!
//! The schema version lives in SQLite's `user_version` pragma. To evolve the schema append a new
//! [`M::up`] to [`migrations`]; never edit a migration that has shipped.

use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};

use crate::error::StorageResult;

/// Version 1: the complete schema plus the four system categories.
///
/// Timestamps are unix milliseconds, dates `YYYY-MM-DD`, times `HH:MM`, JSON columns are TEXT.
const V1: &str = r##"
-- Categories ------------------------------------------------------------------------------
CREATE TABLE categories (
    id              TEXT PRIMARY KEY NOT NULL,
    name            TEXT NOT NULL,
    color           TEXT NOT NULL,
    icon            TEXT NOT NULL,
    description     TEXT NOT NULL DEFAULT '',
    keywords        TEXT NOT NULL DEFAULT '[]',   -- JSON array of strings
    report_time     TEXT,                         -- HH:MM local time, NULL = global default
    report_template TEXT,
    is_productive   INTEGER NOT NULL DEFAULT 1,
    is_system       INTEGER NOT NULL DEFAULT 0,
    archived        INTEGER NOT NULL DEFAULT 0,
    sort_order      INTEGER NOT NULL DEFAULT 0,
    created_at      INTEGER NOT NULL
);

-- Activity blocks -------------------------------------------------------------------------
CREATE TABLE blocks (
    id                TEXT PRIMARY KEY NOT NULL,
    started_at        INTEGER NOT NULL,
    ended_at          INTEGER NOT NULL,
    app_name          TEXT NOT NULL,
    app_id            TEXT NOT NULL,
    title             TEXT NOT NULL DEFAULT '',
    title_key         TEXT NOT NULL DEFAULT '',
    url               TEXT,
    domain            TEXT,
    category_id       TEXT REFERENCES categories(id) ON DELETE SET NULL,
    confidence        REAL NOT NULL DEFAULT 0,
    source            TEXT,                       -- rule | memory | llm | vision | user
    description       TEXT,
    screenshot_id     TEXT,
    sample_count      INTEGER NOT NULL DEFAULT 0,
    is_open           INTEGER NOT NULL DEFAULT 0,
    classify_attempts INTEGER NOT NULL DEFAULT 0,
    next_attempt_at   INTEGER,
    needs_review      INTEGER NOT NULL DEFAULT 0,
    ai_payload        TEXT,
    ai_sent_at        INTEGER,
    is_manual         INTEGER NOT NULL DEFAULT 0,
    note              TEXT
);
CREATE INDEX idx_blocks_started_at   ON blocks(started_at);
CREATE INDEX idx_blocks_ended_at     ON blocks(ended_at);
CREATE INDEX idx_blocks_category     ON blocks(category_id);
CREATE INDEX idx_blocks_open         ON blocks(started_at) WHERE is_open = 1;
CREATE INDEX idx_blocks_pending      ON blocks(started_at) WHERE is_open = 0 AND category_id IS NULL;
CREATE INDEX idx_blocks_needs_review ON blocks(started_at) WHERE needs_review = 1;
CREATE INDEX idx_blocks_user         ON blocks(started_at) WHERE source = 'user';

-- Rules -----------------------------------------------------------------------------------
CREATE TABLE rules (
    id                   TEXT PRIMARY KEY NOT NULL,
    category_id          TEXT NOT NULL REFERENCES categories(id) ON DELETE CASCADE,
    matcher              TEXT NOT NULL,           -- app | domain | title_contains | regex
    pattern              TEXT NOT NULL,
    priority             INTEGER NOT NULL DEFAULT 0,
    origin               TEXT NOT NULL,           -- user | learned
    enabled              INTEGER NOT NULL DEFAULT 1,
    created_at           INTEGER NOT NULL,
    hit_count            INTEGER NOT NULL DEFAULT 0,
    miss_count           INTEGER NOT NULL DEFAULT 0,
    last_contradicted_at INTEGER
);
CREATE INDEX idx_rules_category ON rules(category_id);

-- Corrections (learning signal; kept even when the block is purged) -----------------------
CREATE TABLE corrections (
    id               TEXT PRIMARY KEY NOT NULL,
    block_id         TEXT NOT NULL,
    from_category_id TEXT,
    to_category_id   TEXT NOT NULL REFERENCES categories(id) ON DELETE CASCADE,
    app_id           TEXT NOT NULL,
    app_name         TEXT NOT NULL,
    title_key        TEXT NOT NULL,
    domain           TEXT,
    note             TEXT,
    at               INTEGER NOT NULL
);
CREATE INDEX idx_corrections_at ON corrections(at);

-- Screenshots (metadata only; the image lives at `path`) ----------------------------------
CREATE TABLE screenshots (
    id         TEXT PRIMARY KEY NOT NULL,
    at         INTEGER NOT NULL,
    path       TEXT NOT NULL,
    width      INTEGER NOT NULL,
    height     INTEGER NOT NULL,
    app_id     TEXT NOT NULL,
    block_id   TEXT REFERENCES blocks(id) ON DELETE SET NULL,
    sent_to_ai INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_screenshots_at    ON screenshots(at);
CREATE INDEX idx_screenshots_block ON screenshots(block_id, at);

-- Daily reports ---------------------------------------------------------------------------
CREATE TABLE reports (
    id            TEXT PRIMARY KEY NOT NULL,
    date          TEXT NOT NULL,                  -- YYYY-MM-DD (local calendar day)
    category_id   TEXT NOT NULL REFERENCES categories(id) ON DELETE CASCADE,
    generated_at  INTEGER NOT NULL,
    summary_md    TEXT NOT NULL DEFAULT '',
    items         TEXT NOT NULL DEFAULT '[]',     -- JSON array of ReportItem
    highlights    TEXT NOT NULL DEFAULT '[]',     -- JSON array of strings
    total_secs    INTEGER NOT NULL DEFAULT 0,
    model         TEXT NOT NULL DEFAULT '',
    input_tokens  INTEGER NOT NULL DEFAULT 0,
    output_tokens INTEGER NOT NULL DEFAULT 0,
    stale         INTEGER NOT NULL DEFAULT 0,
    edited        INTEGER NOT NULL DEFAULT 0,
    UNIQUE (date, category_id)
);

-- Nudges ----------------------------------------------------------------------------------
CREATE TABLE nudges (
    id      TEXT PRIMARY KEY NOT NULL,
    at      INTEGER NOT NULL,
    kind    TEXT NOT NULL,
    title   TEXT NOT NULL DEFAULT '',
    message TEXT NOT NULL DEFAULT '',
    seen    INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_nudges_at      ON nudges(at);
CREATE INDEX idx_nudges_kind_at ON nudges(kind, at);

-- Settings: a single row holding the whole struct as JSON ---------------------------------
CREATE TABLE settings (
    id         INTEGER PRIMARY KEY CHECK (id = 1),
    json       TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);

-- AI usage ledger -------------------------------------------------------------------------
CREATE TABLE ai_usage (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    at                 INTEGER NOT NULL,
    kind               TEXT NOT NULL,             -- classify | vision | report | advice
    model              TEXT NOT NULL,
    input_tokens       INTEGER NOT NULL DEFAULT 0,
    output_tokens      INTEGER NOT NULL DEFAULT 0,
    cache_read_tokens  INTEGER NOT NULL DEFAULT 0,
    cache_write_tokens INTEGER NOT NULL DEFAULT 0,
    cost_usd           REAL NOT NULL DEFAULT 0
);
CREATE INDEX idx_ai_usage_at ON ai_usage(at);

-- Engine key/value state ------------------------------------------------------------------
CREATE TABLE kv (
    key        TEXT PRIMARY KEY NOT NULL,
    value      TEXT NOT NULL,
    updated_at INTEGER NOT NULL
);

-- System categories (ids from ubiqx_core::system_categories) ------------------------------
INSERT OR IGNORE INTO categories
    (id, name, color, icon, description, keywords, is_productive, is_system, archived,
     sort_order, created_at)
VALUES
    ('sys-uncategorized', 'Sem categoria', '#94A3B8', 'help-circle',
     'Atividades que ainda não foram enquadradas em nenhuma categoria.',
     '[]', 0, 1, 0, 900, CAST(strftime('%s', 'now') AS INTEGER) * 1000),
    ('sys-distraction', 'Distração', '#F97316', 'coffee',
     'Redes sociais, entretenimento e outras atividades sem relação com o trabalho.',
     '[]', 0, 1, 0, 910, CAST(strftime('%s', 'now') AS INTEGER) * 1000),
    ('sys-break', 'Pausa', '#A3E635', 'pause',
     'Intervalos e pausas ao longo do dia.',
     '[]', 0, 1, 0, 920, CAST(strftime('%s', 'now') AS INTEGER) * 1000),
    ('sys-private', 'Privado', '#64748B', 'lock',
     'Tempo em apps bloqueados ou em modo privado. Nunca é enviado à IA.',
     '[]', 0, 1, 0, 930, CAST(strftime('%s', 'now') AS INTEGER) * 1000);
"##;

/// All migrations, oldest first.
pub fn migrations() -> Migrations<'static> {
    Migrations::new(vec![M::up(V1)])
}

/// Brings `conn` to the latest schema version. Idempotent: reopening an up-to-date database
/// runs nothing.
pub fn apply(conn: &mut Connection) -> StorageResult<()> {
    let migrations = migrations();
    let before: usize = migrations.current_version(conn)?.into();
    migrations.to_latest(conn)?;
    let after: usize = migrations.current_version(conn)?.into();
    if before == after {
        tracing::debug!(version = after, "schema is up to date");
    } else {
        tracing::info!(from = before, to = after, "applied schema migrations");
    }
    Ok(())
}

/// The schema version recorded in the database (`0` when no migration ran yet).
pub fn current_version(conn: &Connection) -> StorageResult<usize> {
    Ok(migrations().current_version(conn)?.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_are_well_formed() {
        migrations().validate().expect("valid migrations");
    }

    #[test]
    fn applying_twice_is_a_no_op() {
        let mut conn = Connection::open_in_memory().expect("memory db");
        apply(&mut conn).expect("first apply");
        apply(&mut conn).expect("second apply");
        assert_eq!(current_version(&conn).expect("version"), 1);
    }
}
