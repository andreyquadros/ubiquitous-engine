//! `BlockRepo::fold_log` (integrator decision 36): after a block is scrubbed, and after
//! "delete all data", no older copy of a row is left in the write-ahead log or in the
//! database file. Against a database file in a temporary directory.

use std::path::{Path, PathBuf};

use chrono::{TimeZone, Utc};
use ubiqx_core::ports::{BlockRepo, MaintenanceRepo};
use ubiqx_core::{ActivityBlock, CoreError};
use ubiqx_storage::{Db, SqliteStore};

const TITLE: &str = "Um titulo de pagina que nao deve ficar no disco";
const ADDRESS: &str = "https://pagina-que-nao-deve-ficar.example/caminho";
const PRIVATE: &str = "Navegação privada";

fn block(id: &str) -> ActivityBlock {
    let from = Utc.with_ymd_and_hms(2026, 3, 9, 14, 0, 0).unwrap();
    ActivityBlock {
        id: id.into(),
        started_at: from,
        ended_at: from + chrono::Duration::minutes(5),
        app_name: "Google Chrome".into(),
        app_id: "com.google.Chrome".into(),
        title: TITLE.into(),
        title_key: TITLE.to_lowercase(),
        url: Some(ADDRESS.into()),
        domain: Some("pagina-que-nao-deve-ficar.example".into()),
        category_id: None,
        confidence: 0.0,
        source: None,
        description: None,
        screenshot_id: None,
        sample_count: 10,
        is_open: true,
        classify_attempts: 0,
        next_attempt_at: None,
        needs_review: false,
        ai_payload: None,
        ai_sent_at: None,
        is_manual: false,
        note: None,
    }
}

fn wal_of(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push("-wal");
    PathBuf::from(name)
}

fn in_file(path: &Path, needle: &str) -> bool {
    std::fs::read(path)
        .map(|bytes| {
            bytes
                .windows(needle.len())
                .any(|window| window == needle.as_bytes())
        })
        .unwrap_or(false)
}

/// Whether `needle` is in the database file (`db`) or in its log (`wal`).
fn on_disk(path: &Path, needle: &str) -> (bool, bool) {
    (in_file(path, needle), in_file(&wal_of(path), needle))
}

fn wal_len(path: &Path) -> u64 {
    std::fs::metadata(wal_of(path))
        .map(|m| m.len())
        .unwrap_or(0)
}

/// A store with one block whose title and address are on disk (in the log, and in the file
/// after a checkpoint the test runs when `checkpointed`), then scrubbed the way the
/// tracker scrubs: the row rewritten with the private title and no address.
fn scrubbed(dir: &Path, checkpointed: bool) -> (SqliteStore, PathBuf) {
    let path = dir.join("ubiqx.db");
    let store = SqliteStore::new(Db::open(&path).unwrap());
    let mut row = block("b1");
    BlockRepo::insert(&store, &row).unwrap();
    if checkpointed {
        // The row reaches the main file, as an automatic checkpoint would take it there.
        store.fold_log().unwrap();
        assert_eq!(on_disk(&path, TITLE), (true, false));
    }
    // A few more versions of the row in the log, as the tracker's touches leave them.
    for n in 0..3 {
        row.sample_count += n;
        BlockRepo::touch(&store, &row).unwrap();
    }
    assert!(
        on_disk(&path, TITLE).1,
        "the title is in the log before the scrub"
    );
    row.is_open = false;
    row.title = PRIVATE.into();
    row.title_key = PRIVATE.to_lowercase();
    row.url = None;
    row.domain = None;
    BlockRepo::update(&store, &row).unwrap();
    (store, path)
}

#[test]
fn a_rewritten_row_stays_in_the_log_until_it_is_folded() {
    let dir = tempfile::tempdir().unwrap();
    let (store, path) = scrubbed(dir.path(), false);
    assert!(
        on_disk(&path, TITLE).1,
        "without the fold the older versions stay in the log"
    );

    store.fold_log().unwrap();
    assert_eq!(on_disk(&path, TITLE), (false, false));
    assert_eq!(on_disk(&path, ADDRESS), (false, false));
    assert_eq!(wal_len(&path), 0, "the log is truncated");
    let row = BlockRepo::get(&store, "b1").unwrap().unwrap();
    assert_eq!((row.title.as_str(), row.url), (PRIVATE, None));
}

#[test]
fn a_row_already_in_the_file_is_overwritten_there_by_the_fold() {
    let dir = tempfile::tempdir().unwrap();
    let (store, path) = scrubbed(dir.path(), true);
    assert!(on_disk(&path, TITLE).0, "the old version is in the file");

    store.fold_log().unwrap();
    assert_eq!(on_disk(&path, TITLE), (false, false));
    assert_eq!(on_disk(&path, ADDRESS), (false, false));
    // Still a working database afterwards.
    let mut next = block("b2");
    next.title = "Outra janela".into();
    BlockRepo::insert(&store, &next).unwrap();
    assert!(BlockRepo::get(&store, "b2").unwrap().is_some());
}

#[test]
fn deleting_all_data_leaves_no_row_in_the_file_or_the_log() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ubiqx.db");
    let store = SqliteStore::new(Db::open(&path).unwrap());
    let mut row = block("b1");
    row.is_open = false;
    BlockRepo::insert(&store, &row).unwrap();
    store.fold_log().unwrap();
    row.sample_count += 1;
    BlockRepo::update(&store, &row).unwrap();
    assert_eq!(on_disk(&path, TITLE), (true, true));

    store.wipe_user_data().unwrap();
    assert_eq!(on_disk(&path, TITLE), (false, false));
    assert_eq!(on_disk(&path, ADDRESS), (false, false));
    assert_eq!(wal_len(&path), 0, "VACUUM's pages are folded too");
    assert_eq!(BlockRepo::get(&store, "b1").unwrap(), None);
}

/// "Delete all data" while another process is in the middle of a read (the CLI): the
/// rows are deleted and the call succeeds, so the engine goes on to delete the screenshot
/// files; the log keeps the older versions only until the next fold.
#[test]
fn deleting_all_data_while_another_process_reads_still_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ubiqx.db");
    let store = SqliteStore::new(Db::open(&path).unwrap());
    let mut row = block("b1");
    row.is_open = false;
    BlockRepo::insert(&store, &row).unwrap();
    store.fold_log().unwrap();
    row.sample_count += 1;
    BlockRepo::update(&store, &row).unwrap();
    store
        .db()
        .with(|conn| Ok(conn.execute_batch("PRAGMA busy_timeout = 0")?))
        .unwrap();
    let reader = rusqlite::Connection::open(&path).unwrap();
    reader
        .execute_batch("BEGIN; SELECT count(*) FROM blocks;")
        .unwrap();
    // The reader really blocks a fold: the wipe below meets the same.
    assert!(store.fold_log().is_err());

    store.wipe_user_data().unwrap();
    assert_eq!(BlockRepo::get(&store, "b1").unwrap(), None);

    reader.execute_batch("COMMIT").unwrap();
    store.fold_log().unwrap();
    assert_eq!(on_disk(&path, TITLE), (false, false));
    assert_eq!(on_disk(&path, ADDRESS), (false, false));
    assert_eq!(wal_len(&path), 0);
}

#[test]
fn a_fold_that_cannot_finish_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ubiqx.db");
    let store = SqliteStore::new(Db::open(&path).unwrap());
    BlockRepo::insert(&store, &block("b1")).unwrap();
    // Not waiting five seconds for the reader below.
    store
        .db()
        .with(|conn| Ok(conn.execute_batch("PRAGMA busy_timeout = 0")?))
        .unwrap();
    // Another process in the middle of a read holds the log.
    let reader = rusqlite::Connection::open(&path).unwrap();
    reader
        .execute_batch("BEGIN; SELECT count(*) FROM blocks;")
        .unwrap();
    BlockRepo::touch(&store, &block("b1")).unwrap();

    let error = store.fold_log().unwrap_err();
    assert!(matches!(error, CoreError::Storage(_)), "{error:?}");
    reader.execute_batch("COMMIT").unwrap();
    store.fold_log().unwrap();
    assert_eq!(wal_len(&path), 0);
}

#[test]
fn an_in_memory_store_has_nothing_to_fold() {
    let store = SqliteStore::new(Db::open_in_memory().unwrap());
    BlockRepo::insert(&store, &block("b1")).unwrap();
    store.fold_log().unwrap();
}
