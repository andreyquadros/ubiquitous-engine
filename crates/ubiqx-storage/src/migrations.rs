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

/// Version 2: the focus guard. Blocked targets (unique per kind + key), the interventions
/// UBI made and the focus sessions. Interventions keep no foreign key to their target so the
/// history survives a removed target.
const V2: &str = r##"
-- Focus targets (blocked apps and sites) ---------------------------------------------------
CREATE TABLE focus_targets (
    id              TEXT PRIMARY KEY NOT NULL,
    kind            TEXT NOT NULL,                -- app | site
    name            TEXT NOT NULL,
    key             TEXT NOT NULL,                -- bundle id | registrable domain
    enabled         INTEGER NOT NULL DEFAULT 1,
    created_at      INTEGER NOT NULL,
    last_blocked_at INTEGER,
    blocked_count   INTEGER NOT NULL DEFAULT 0,
    UNIQUE (kind, key)
);

-- Interventions (what the guard held, and what UBI said) ----------------------------------
CREATE TABLE interventions (
    id         TEXT PRIMARY KEY NOT NULL,
    at         INTEGER NOT NULL,
    target_id  TEXT,
    kind       TEXT NOT NULL,                     -- app | site
    name       TEXT NOT NULL,
    key        TEXT NOT NULL,
    action     TEXT NOT NULL,                     -- app_quit | tab_closed | tab_blanked | notified
    session_id TEXT,
    message    TEXT NOT NULL DEFAULT ''
);
CREATE INDEX idx_interventions_at ON interventions(at);

-- Focus sessions --------------------------------------------------------------------------
CREATE TABLE focus_sessions (
    id            TEXT PRIMARY KEY NOT NULL,
    task          TEXT NOT NULL,
    started_at    INTEGER NOT NULL,
    ends_at       INTEGER NOT NULL,
    ended_at      INTEGER,
    interventions INTEGER NOT NULL DEFAULT 0,
    hid_windows   INTEGER NOT NULL DEFAULT 0,
    ran_shortcut  INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_focus_sessions_started ON focus_sessions(started_at);
CREATE INDEX idx_focus_sessions_active  ON focus_sessions(started_at) WHERE ended_at IS NULL;
"##;

/// Version 3: the `nofap` module. One configuration row, the user's and the learned sites,
/// and hourly counters. There is no row per event, no exact time, no host of a learned site
/// and no app: a learned site or a snooze is its salted hash only, which the first `CHECK`
/// makes a property of the file, not only of the code that writes it. Both products apply
/// this same list, each to its own database file.
const V3: &str = r##"
-- Configuration (a single row; never part of `settings`) ---------------------------------
CREATE TABLE nofap_state (
    id         INTEGER PRIMARY KEY CHECK (id = 1),
    config     TEXT NOT NULL,                 -- NofapConfig as JSON
    updated_at INTEGER NOT NULL               -- unix milliseconds
);

-- Sites (the user's lists, sites learned from a screen check, snoozes) --------------------
CREATE TABLE nofap_sites (
    id         TEXT PRIMARY KEY NOT NULL,
    list       TEXT NOT NULL,                 -- block | allow
    origin     TEXT NOT NULL,                 -- user | learned | snooze
    host       TEXT,                          -- NULL unless origin = user
    host_key   BLOB NOT NULL,                 -- 16 bytes, SHA-256(salt || 0x00 || host)[..16]
    created_at INTEGER NOT NULL,
    expires_at INTEGER,                       -- snooze only
    UNIQUE (list, host_key),
    CHECK (origin = 'user' OR host IS NULL),
    CHECK (length(host_key) = 16)
);

-- Counters per local day and hour ---------------------------------------------------------
CREATE TABLE nofap_day (
    day  TEXT NOT NULL,                       -- local YYYY-MM-DD
    hour INTEGER NOT NULL,                    -- local hour 0..23
    kind TEXT NOT NULL,                       -- ubiqx_core::nofap::DayKind, read leniently
    n    INTEGER NOT NULL,
    PRIMARY KEY (day, hour, kind)
);
"##;

/// All migrations, oldest first.
pub fn migrations() -> Migrations<'static> {
    Migrations::new(vec![M::up(V1), M::up(V2), M::up(V3)])
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
        assert_eq!(current_version(&conn).expect("version"), 3);
    }

    fn table_names(conn: &Connection) -> Vec<String> {
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
            .expect("prepare");
        let names = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .expect("query")
            .collect::<Result<Vec<_>, _>>()
            .expect("names");
        names
    }

    #[test]
    fn v3_adds_the_three_tables_to_a_v2_database_and_keeps_its_rows() {
        let mut conn = Connection::open_in_memory().expect("memory db");
        migrations().to_version(&mut conn, 2).expect("up to V2");
        assert_eq!(current_version(&conn).expect("version"), 2);
        assert!(!table_names(&conn).iter().any(|t| t.starts_with("nofap_")));
        conn.execute_batch(
            "INSERT INTO kv (key, value, updated_at) VALUES ('k', 'v', 1);
             INSERT INTO focus_targets (id, kind, name, key, created_at)
                 VALUES ('t1', 'site', 'Exemplo', 'example.com', 1);
             INSERT INTO ai_usage (at, kind, model) VALUES (1, 'classify', 'm');",
        )
        .expect("V2 rows");

        apply(&mut conn).expect("V3");

        assert_eq!(current_version(&conn).expect("version"), 3);
        let tables = table_names(&conn);
        for table in ["nofap_state", "nofap_sites", "nofap_day"] {
            assert!(tables.iter().any(|t| t == table), "{table} missing");
            let rows: i64 = conn
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
                .expect("count");
            assert_eq!(rows, 0, "{table} starts empty");
        }
        for table in ["kv", "focus_targets", "ai_usage"] {
            let rows: i64 = conn
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
                .expect("count");
            assert_eq!(rows, 1, "{table} kept its row");
        }
        assert_eq!(
            conn.query_row("SELECT COUNT(*) FROM categories", [], |r| r
                .get::<_, i64>(0))
                .expect("count"),
            4,
            "the seeded categories are not seeded twice"
        );
    }

    #[test]
    fn the_schema_refuses_a_name_for_a_learned_site_and_a_short_key() {
        let mut conn = Connection::open_in_memory().expect("memory db");
        apply(&mut conn).expect("apply");
        let insert = "INSERT INTO nofap_sites (id, list, origin, host, host_key, created_at) \
                      VALUES (?1, 'block', ?2, ?3, ?4, 1)";
        let key = vec![7u8; 16];
        conn.execute(insert, rusqlite::params!["a", "user", "example.com", key])
            .expect("a user row carries its name");
        conn.execute(
            insert,
            rusqlite::params!["b", "learned", None::<String>, vec![8u8; 16]],
        )
        .expect("a learned row without a name");
        assert!(conn
            .execute(
                insert,
                rusqlite::params!["c", "learned", "example.org", vec![9u8; 16]]
            )
            .is_err());
        assert!(conn
            .execute(
                insert,
                rusqlite::params!["d", "snooze", "example.org", vec![10u8; 16]]
            )
            .is_err());
        assert!(conn
            .execute(
                insert,
                rusqlite::params!["e", "user", "example.net", vec![11u8; 15]]
            )
            .is_err());
        assert!(
            conn.execute(insert, rusqlite::params!["f", "user", "other.example", key])
                .is_err(),
            "one row per (list, host_key)"
        );
        assert!(conn
            .execute(
                "INSERT INTO nofap_state (id, config, updated_at) VALUES (2, '{}', 1)",
                []
            )
            .is_err());
    }
}
