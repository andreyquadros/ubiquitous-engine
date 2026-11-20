//! Integration tests of the `nofap` module's tables: migration V3 on a database that
//! already holds data, the [`NofapRepo`] implementation (checked step by step against a
//! model of the reference, `ubiqx_engine::MemoryNofapRepo`), retention, the two erasures
//! and what stays readable in the file afterwards.

use std::path::Path;

use chrono::{DateTime, Duration, TimeZone, Utc};
use ubiqx_core::nofap::{
    DayCount, DayKind, NofapChange, NofapConfig, NofapRepo, NofapSite, SiteList, SiteOrigin,
};
use ubiqx_core::nofap_ipc::sample;
use ubiqx_core::ports::{
    AiUsage, AiUsageKind, FocusRepo, KvRepo, MaintenanceRepo, SettingsRepo, UsageRepo,
};
use ubiqx_core::{CoreError, Settings, TimeRange};
use ubiqx_storage::{Db, SqliteStore};

// ---------------------------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------------------------

fn store() -> SqliteStore {
    SqliteStore::new(Db::open_in_memory().expect("in-memory database"))
}

/// Minutes from 2026-09-17 15:00 UTC.
fn at(mins: i64) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 9, 17, 15, 0, 0)
        .single()
        .expect("valid instant")
        + Duration::minutes(mins)
}

fn key(n: u8) -> [u8; 16] {
    [n; 16]
}

fn user_site(id: &str, list: SiteList, host: &str, n: u8) -> NofapSite {
    NofapSite {
        id: id.into(),
        list,
        origin: SiteOrigin::User,
        host: Some(host.into()),
        host_key: key(n),
        created_at: at(0),
        expires_at: None,
    }
}

fn learned_site(id: &str, n: u8) -> NofapSite {
    NofapSite {
        id: id.into(),
        list: SiteList::Block,
        origin: SiteOrigin::Learned,
        host: None,
        host_key: key(n),
        created_at: at(1),
        expires_at: None,
    }
}

fn snooze(id: &str, n: u8, expires_at: DateTime<Utc>) -> NofapSite {
    NofapSite {
        id: id.into(),
        list: SiteList::Allow,
        origin: SiteOrigin::Snooze,
        host: None,
        host_key: key(n),
        created_at: at(2),
        expires_at: Some(expires_at),
    }
}

fn ids(sites: &[NofapSite]) -> Vec<&str> {
    sites.iter().map(|s| s.id.as_str()).collect()
}

fn count(day: &str, hour: u8, kind: DayKind, n: u32) -> DayCount {
    DayCount {
        day: day.into(),
        hour,
        kind,
        n,
    }
}

fn raw(store: &SqliteStore, sql: &str) {
    store
        .db()
        .with(|c| {
            c.execute_batch(sql)?;
            Ok(())
        })
        .expect("raw statement");
}

fn rows(store: &SqliteStore, table: &str) -> i64 {
    store
        .db()
        .with(|c| Ok(c.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))?))
        .expect("count")
}

/// Whether `needle` occurs in the database file or in its write-ahead log.
fn on_disk(path: &Path, needle: &[u8]) -> bool {
    let wal = path.with_extension("sqlite-wal");
    [path.to_path_buf(), wal].iter().any(|file| {
        std::fs::read(file)
            .map(|bytes| bytes.windows(needle.len()).any(|w| w == needle))
            .unwrap_or(false)
    })
}

// ---------------------------------------------------------------------------------------------
// Migration
// ---------------------------------------------------------------------------------------------

#[test]
fn v3_applies_on_a_v2_database_with_data() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("ubiqx.sqlite");
    let settings = Settings {
        user_profile: Some("Ana, professora".into()),
        ..Settings::default()
    };

    // A database as the previous release left it: schema version 2, with rows.
    {
        let mut conn = rusqlite::Connection::open(&path).expect("open");
        ubiqx_storage::migrations::migrations()
            .to_version(&mut conn, 2)
            .expect("up to V2");
        conn.execute(
            "INSERT INTO settings (id, json, updated_at) VALUES (1, ?1, 1)",
            [serde_json::to_string(&settings).expect("json")],
        )
        .expect("settings");
        conn.execute_batch(
            "INSERT INTO kv (key, value, updated_at) VALUES ('last_report_check', '2026-09-17', 1);
             INSERT INTO focus_targets (id, kind, name, key, created_at)
                 VALUES ('t1', 'site', 'Exemplo', 'example.com', 1);",
        )
        .expect("V2 rows");
        conn.execute(
            "INSERT INTO ai_usage (at, kind, model, input_tokens, cost_usd) \
             VALUES (?1, 'classify', 'm', 10, 0.5)",
            [at(0).timestamp_millis()],
        )
        .expect("usage");
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .expect("version");
        assert_eq!(version, 2);
    }

    let db = Db::open(&path).expect("open migrates");
    assert_eq!(db.schema_version().expect("version"), 3);
    let s = SqliteStore::new(db);

    // What was there is still there.
    assert_eq!(SettingsRepo::load(&s).expect("settings"), settings);
    assert_eq!(
        KvRepo::get(&s, "last_report_check").expect("kv").as_deref(),
        Some("2026-09-17")
    );
    let targets = FocusRepo::list_targets(&s).expect("targets");
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].key, "example.com");
    assert_eq!(
        UsageRepo::totals(&s, TimeRange::new(at(-600), at(600)))
            .expect("usage")
            .calls,
        1
    );

    // The new tables are there, empty, and usable.
    let repo: &dyn NofapRepo = &s;
    assert_eq!(repo.load_config().expect("config"), NofapConfig::default());
    assert!(repo.list_sites().expect("sites").is_empty());
    assert!(repo
        .days_between("0000-01-01", "9999-12-31")
        .expect("days")
        .is_empty());
    repo.save_config(&sample::config()).expect("save");
    repo.upsert_site(&learned_site("l1", 1)).expect("site");
    repo.bump_day("2026-09-17", 15, DayKind::BlockList, 2)
        .expect("bump");
    drop(s);

    // And they persist across a reopen, which runs no migration.
    let db = Db::open(&path).expect("reopen");
    assert_eq!(db.schema_version().expect("version"), 3);
    let s = SqliteStore::new(db);
    let repo: &dyn NofapRepo = &s;
    assert_eq!(repo.load_config().expect("config"), sample::config());
    assert_eq!(
        repo.list_sites().expect("sites"),
        vec![learned_site("l1", 1)]
    );
    assert_eq!(
        repo.days_between("2026-09-17", "2026-09-17").expect("days"),
        vec![count("2026-09-17", 15, DayKind::BlockList, 2)]
    );
}

// ---------------------------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------------------------

#[test]
fn config_is_the_default_until_saved_and_round_trips() {
    let s = store();
    let repo: &dyn NofapRepo = &s;
    assert_eq!(
        repo.load_config().expect("missing row"),
        NofapConfig::default()
    );
    assert_eq!(rows(&s, "nofap_state"), 0, "reading writes nothing");

    let config = sample::config();
    assert!(config.enabled && !config.pending.is_empty());
    repo.save_config(&config).expect("save");
    assert_eq!(repo.load_config().expect("load"), config);

    let mut changed = config.clone();
    changed.enabled = false;
    changed.values = "Presente com a minha família.".into();
    changed.pending.clear();
    repo.save_config(&changed).expect("overwrite");
    assert_eq!(repo.load_config().expect("load"), changed);
    assert_eq!(rows(&s, "nofap_state"), 1, "one row, replaced in place");
}

#[test]
fn a_document_from_an_older_build_takes_the_defaults_for_what_it_lacks() {
    let s = store();
    raw(
        &s,
        r#"INSERT INTO nofap_state (id, config, updated_at)
           VALUES (1, '{"enabled": true, "delay_secs": 900}', 1)"#,
    );
    let config = NofapRepo::load_config(&s).expect("load");
    assert_eq!(
        config,
        NofapConfig {
            enabled: true,
            delay_secs: 900,
            ..NofapConfig::default()
        }
    );
}

#[test]
fn an_unreadable_config_is_an_error_never_the_default() {
    let s = store();
    let repo: &dyn NofapRepo = &s;

    // Not JSON at all.
    raw(
        &s,
        "INSERT INTO nofap_state (id, config, updated_at) VALUES (1, 'enabled: yes', 1)",
    );
    assert!(matches!(repo.load_config(), Err(CoreError::Storage(_))));

    // A pending change of a kind this build does not know fails the whole document. The
    // module is on in it: answering with the default would switch the protection off.
    raw(
        &s,
        r#"UPDATE nofap_state SET config = '{"enabled": true, "pending": [{"id": "p1",
           "change": {"kind": "from_the_future", "host": "segredo.example"},
           "requested_at": "2026-09-17T15:00:00Z", "ready_at": "2026-09-17T15:10:00Z"}]}'"#,
    );
    let err = repo.load_config().expect_err("unknown change kind");
    let message = err.to_string();
    assert!(matches!(err, CoreError::Storage(_)), "{message}");
    assert!(message.contains("does not parse"), "{message}");

    // A value of the wrong type, which serde would quote in its own message.
    raw(
        &s,
        r#"UPDATE nofap_state SET config = '{"enabled": true, "delay_secs": "segredo.example"}'"#,
    );
    let message = repo.load_config().expect_err("wrong type").to_string();
    assert!(
        !message.contains("segredo"),
        "the error says where, not what: {message}"
    );
    assert!(message.contains("line 1"), "{message}");

    // Saving a good configuration repairs the row.
    repo.save_config(&sample::config()).expect("save");
    assert_eq!(repo.load_config().expect("load"), sample::config());
}

#[test]
fn a_stored_document_that_is_not_an_object_is_an_error() {
    let s = store();
    let repo: &dyn NofapRepo = &s;
    raw(
        &s,
        "INSERT INTO nofap_state (id, config, updated_at) VALUES (1, '{}', 1)",
    );
    assert_eq!(
        repo.load_config().expect("an empty object"),
        NofapConfig::default()
    );

    // serde reads an array as the fields in order: `[]` would be the default, which is
    // "module off", and `[1, true]` a configuration nobody wrote.
    for document in [
        "[]",
        " [ ] ",
        "[1, true]",
        "null",
        "true",
        "7",
        "\"{}\"",
        "",
    ] {
        s.db()
            .with(|c| {
                c.execute("UPDATE nofap_state SET config = ?1", [document])?;
                Ok(())
            })
            .expect("write the document");
        let err = repo
            .load_config()
            .expect_err("only an object is a configuration");
        assert!(matches!(err, CoreError::Storage(_)), "{document:?}: {err}");
        assert!(err.to_string().contains("does not parse"), "{document:?}");
    }

    // Whitespace before the object is still an object.
    raw(
        &s,
        r#"UPDATE nofap_state SET config = '
  {"enabled": true}'"#,
    );
    assert!(repo.load_config().expect("an object").enabled);
}

#[test]
fn a_pending_change_with_a_host_round_trips() {
    let s = store();
    let repo: &dyn NofapRepo = &s;
    let mut config = sample::config();
    config.pending[0].change = NofapChange::AddAllow {
        host: "exemplo.example".into(),
    };
    config.pending[0].observed_secs = 120;
    config.pending[0].ready_since = Some(at(5));
    repo.save_config(&config).expect("save");
    assert_eq!(repo.load_config().expect("load"), config);
}

// ---------------------------------------------------------------------------------------------
// Sites
// ---------------------------------------------------------------------------------------------

#[test]
fn sites_round_trip_in_insertion_order() {
    let s = store();
    let repo: &dyn NofapRepo = &s;
    assert!(repo.list_sites().expect("empty").is_empty());

    let sites = vec![
        user_site("u1", SiteList::Block, "bloqueado.example", 1),
        learned_site("l1", 2),
        snooze("z1", 3, at(24 * 60)),
        user_site("u2", SiteList::Allow, "liberado.example", 4),
    ];
    for site in &sites {
        repo.upsert_site(site).expect("upsert");
    }
    assert_eq!(repo.list_sites().expect("list"), sites);
}

#[test]
fn upsert_replaces_by_id_and_by_list_and_key() {
    let s = store();
    let repo: &dyn NofapRepo = &s;
    repo.upsert_site(&user_site("a", SiteList::Block, "um.example", 1))
        .expect("a");
    repo.upsert_site(&user_site("b", SiteList::Block, "dois.example", 2))
        .expect("b");
    repo.upsert_site(&learned_site("c", 3)).expect("c");

    // Same (list, key) under a new id: the old row goes, the new one is last.
    let replacement = user_site("d", SiteList::Block, "um.example", 1);
    repo.upsert_site(&replacement).expect("d");
    let sites = repo.list_sites().expect("list");
    assert_eq!(ids(&sites), ["b", "c", "d"]);
    assert_eq!(sites[2], replacement);

    // Same id with another key: replaced in the same way.
    let moved = user_site("b", SiteList::Block, "tres.example", 9);
    repo.upsert_site(&moved).expect("b again");
    let sites = repo.list_sites().expect("list");
    assert_eq!(ids(&sites), ["c", "d", "b"]);
    assert_eq!(sites[2], moved);

    // One upsert can displace two rows: the one with its id and the one with its key.
    repo.upsert_site(&NofapSite {
        id: "c".into(),
        ..user_site("x", SiteList::Block, "um.example", 1)
    })
    .expect("c takes d's key");
    assert_eq!(ids(&repo.list_sites().expect("list")), ["b", "c"]);

    // The same key on the other list is another row.
    repo.upsert_site(&user_site("e", SiteList::Allow, "tres.example", 9))
        .expect("e");
    assert_eq!(ids(&repo.list_sites().expect("list")), ["b", "c", "e"]);
}

#[test]
fn only_a_user_site_keeps_its_name() {
    let s = store();
    let repo: &dyn NofapRepo = &s;
    repo.upsert_site(&NofapSite {
        host: Some("aprendido.example".into()),
        ..learned_site("l1", 1)
    })
    .expect("learned");
    repo.upsert_site(&NofapSite {
        host: Some("adiado.example".into()),
        ..snooze("z1", 2, at(60))
    })
    .expect("snooze");
    repo.upsert_site(&user_site("u1", SiteList::Block, "meu.example", 3))
        .expect("user");

    let sites = repo.list_sites().expect("list");
    assert_eq!(
        sites
            .iter()
            .map(|s| (s.id.as_str(), s.host.as_deref()))
            .collect::<Vec<_>>(),
        [("l1", None), ("z1", None), ("u1", Some("meu.example"))]
    );
    // Everything else about the rows is what was passed.
    assert_eq!(sites[0], learned_site("l1", 1));
    assert_eq!(sites[1], snooze("z1", 2, at(60)));
}

#[test]
fn delete_by_id_by_origin_and_by_expiry() {
    let s = store();
    let repo: &dyn NofapRepo = &s;
    for site in [
        user_site("u1", SiteList::Block, "um.example", 1),
        learned_site("l1", 2),
        learned_site("l2", 3),
        snooze("z-past", 4, at(-1)),
        snooze("z-now", 5, at(0)),
        snooze("z-later", 6, at(1)),
    ] {
        repo.upsert_site(&site).expect("upsert");
    }

    repo.delete_site("u1").expect("delete");
    repo.delete_site("no-such-id")
        .expect("a missing id is not an error");
    assert_eq!(
        ids(&repo.list_sites().expect("list")),
        ["l1", "l2", "z-past", "z-now", "z-later"]
    );

    // A snooze ends at its expiry: `expires_at <= now` goes, a later one stays, and a row
    // without an expiry is never purged.
    assert_eq!(repo.purge_expired_sites(at(0)).expect("purge"), 2);
    assert_eq!(
        ids(&repo.list_sites().expect("list")),
        ["l1", "l2", "z-later"]
    );
    assert_eq!(repo.purge_expired_sites(at(0)).expect("again"), 0);

    assert_eq!(
        repo.delete_sites_by_origin(SiteOrigin::Learned)
            .expect("forget"),
        2
    );
    assert_eq!(ids(&repo.list_sites().expect("list")), ["z-later"]);
    assert_eq!(
        repo.delete_sites_by_origin(SiteOrigin::User).expect("none"),
        0
    );
}

#[test]
fn a_site_row_this_build_does_not_understand_is_skipped() {
    let s = store();
    let repo: &dyn NofapRepo = &s;
    repo.upsert_site(&learned_site("l1", 1)).expect("upsert");
    raw(
        &s,
        "INSERT INTO nofap_sites (id, list, origin, host, host_key, created_at)
         VALUES ('n1', 'block', 'from_the_future', NULL, zeroblob(16), 1),
                ('n2', 'watch', 'user', 'novo.example', randomblob(16), 1)",
    );
    assert_eq!(ids(&repo.list_sites().expect("list")), ["l1"]);
}

// ---------------------------------------------------------------------------------------------
// Counters
// ---------------------------------------------------------------------------------------------

#[test]
fn bump_day_is_an_upsert_per_day_hour_and_kind() {
    let s = store();
    let repo: &dyn NofapRepo = &s;
    repo.bump_day("2026-09-17", 15, DayKind::BlockList, 1)
        .expect("first");
    repo.bump_day("2026-09-17", 15, DayKind::BlockList, 2)
        .expect("same counter");
    repo.bump_day("2026-09-17", 15, DayKind::VisionCheck, 1)
        .expect("another kind");
    repo.bump_day("2026-09-17", 16, DayKind::BlockList, 1)
        .expect("another hour");
    repo.bump_day("2026-09-18", 0, DayKind::BlockList, 5)
        .expect("another day");
    repo.bump_day("2026-09-16", 23, DayKind::Lapse, 1)
        .expect("the day before");

    assert_eq!(rows(&s, "nofap_day"), 5);
    assert_eq!(
        repo.days_between("2026-09-16", "2026-09-18").expect("all"),
        vec![
            count("2026-09-16", 23, DayKind::Lapse, 1),
            count("2026-09-17", 15, DayKind::BlockList, 3),
            count("2026-09-17", 15, DayKind::VisionCheck, 1),
            count("2026-09-17", 16, DayKind::BlockList, 1),
            count("2026-09-18", 0, DayKind::BlockList, 5),
        ],
        "ordered by day, hour and kind"
    );
    // Both ends are inclusive.
    assert_eq!(
        repo.days_between("2026-09-17", "2026-09-17")
            .expect("one day")
            .len(),
        3
    );
    assert!(repo
        .days_between("2026-09-19", "2026-09-30")
        .expect("later")
        .is_empty());
    assert!(repo
        .days_between("2026-09-18", "2026-09-16")
        .expect("inverted range")
        .is_empty());
}

#[test]
fn every_kind_round_trips_and_a_counter_saturates() {
    let s = store();
    let repo: &dyn NofapRepo = &s;
    for kind in DayKind::ALL {
        repo.bump_day("2026-09-17", 9, kind, 1).expect("bump");
    }
    let stored = repo.days_between("2026-09-17", "2026-09-17").expect("read");
    let mut kinds: Vec<&str> = stored.iter().map(|c| c.kind.as_str()).collect();
    assert_eq!(kinds.len(), DayKind::ALL.len());
    let mut expected: Vec<&str> = DayKind::ALL.iter().map(|k| k.as_str()).collect();
    expected.sort_unstable();
    assert_eq!(kinds, expected, "sorted by the stored kind");
    kinds.dedup();
    assert_eq!(kinds.len(), DayKind::ALL.len());

    repo.bump_day("2026-09-17", 10, DayKind::Panic, u32::MAX - 1)
        .expect("large");
    repo.bump_day("2026-09-17", 10, DayKind::Panic, 5)
        .expect("past the top");
    repo.bump_day("2026-09-17", 10, DayKind::Panic, u32::MAX)
        .expect("far past the top");
    let panic = repo
        .days_between("2026-09-17", "2026-09-17")
        .expect("read")
        .into_iter()
        .find(|c| c.hour == 10)
        .expect("counter");
    assert_eq!(panic.n, u32::MAX);
}

#[test]
fn a_counter_needs_a_calendar_day_and_an_hour_of_the_day() {
    let s = store();
    let repo: &dyn NofapRepo = &s;
    for (day, hour) in [
        ("17/09/2026", 9),
        ("2026-9-17", 9),
        ("2026-02-30", 9),
        ("", 9),
        ("2026-09-17 ", 9),
        ("2026-09-17", 24),
        ("2026-09-17", 255),
    ] {
        assert!(
            matches!(
                repo.bump_day(day, hour, DayKind::BlockList, 1),
                Err(CoreError::Invalid(_))
            ),
            "`{day}` hour {hour}"
        );
    }
    assert_eq!(rows(&s, "nofap_day"), 0, "nothing was written");
    repo.bump_day("2026-09-17", 23, DayKind::BlockList, 1)
        .expect("the last hour of the day");
    repo.bump_day("2028-02-29", 0, DayKind::BlockList, 1)
        .expect("a leap day");
}

#[test]
fn an_unknown_kind_does_not_fail_a_read() {
    let s = store();
    let repo: &dyn NofapRepo = &s;
    repo.bump_day("2026-09-17", 15, DayKind::BlockList, 1)
        .expect("bump");
    // Rows a newer build wrote, and one nobody could have written.
    raw(
        &s,
        "INSERT INTO nofap_day (day, hour, kind, n) VALUES
            ('2026-09-17', 15, 'from_the_future', 7),
            ('2026-09-17', 16, 'another_new_kind', 1),
            ('2026-09-17', 99, 'block_list', 1),
            ('2026-09-17', 17, 'lapse', -3),
            ('2026-09-17', 18, 'lapse', 99999999999)",
    );

    assert_eq!(
        repo.days_between("2026-09-17", "2026-09-17").expect("read"),
        vec![
            count("2026-09-17", 15, DayKind::BlockList, 1),
            count("2026-09-17", 17, DayKind::Lapse, 0),
            count("2026-09-17", 18, DayKind::Lapse, u32::MAX),
        ],
        "unknown kinds and an impossible hour are skipped; a count is clamped"
    );
    // Writing next to them still works, and retention removes them like any other row.
    repo.bump_day("2026-09-17", 15, DayKind::BlockList, 1)
        .expect("bump");
    assert_eq!(repo.delete_days_before("2026-09-18").expect("prune"), 6);
    assert_eq!(rows(&s, "nofap_day"), 0);
}

#[test]
fn retention_deletes_the_days_before_the_given_one() {
    let s = store();
    let repo: &dyn NofapRepo = &s;
    for (day, hour) in [
        ("2026-06-18", 9),
        ("2026-06-19", 9),
        ("2026-06-19", 10),
        ("2026-06-20", 0),
        ("2026-09-17", 15),
    ] {
        repo.bump_day(day, hour, DayKind::VisionCheck, 1)
            .expect("bump");
    }
    assert_eq!(repo.delete_days_before("2026-06-20").expect("prune"), 3);
    let left = repo.days_between("0000-01-01", "9999-12-31").expect("read");
    assert_eq!(
        left.iter().map(|c| c.day.as_str()).collect::<Vec<_>>(),
        ["2026-06-20", "2026-09-17"],
        "the day itself is kept"
    );
    assert_eq!(repo.delete_days_before("2026-06-20").expect("again"), 0);
}

#[test]
fn the_cutoff_of_the_prune_must_be_a_calendar_day() {
    let s = store();
    let repo: &dyn NofapRepo = &s;
    repo.bump_day("2026-09-17", 15, DayKind::VisionCheck, 1)
        .expect("bump");
    repo.bump_day("2026-12-31", 23, DayKind::Lapse, 1)
        .expect("bump");

    // Compared as text, each of these would delete days it should keep: `2026-9-18` sorts
    // after every day of 2026, `zzzz` after every day there is.
    for cutoff in [
        "2026-9-18",
        "2026-09-18T00:00:00Z",
        "2026-09-18 ",
        "18/09/2026",
        "2026-02-30",
        "zzzz",
        "",
    ] {
        assert!(
            matches!(repo.delete_days_before(cutoff), Err(CoreError::Invalid(_))),
            "{cutoff:?}"
        );
        assert_eq!(rows(&s, "nofap_day"), 2, "{cutoff:?} deleted nothing");
    }
    assert_eq!(repo.delete_days_before("2026-09-18").expect("prune"), 1);
    assert_eq!(rows(&s, "nofap_day"), 1);
}

// ---------------------------------------------------------------------------------------------
// The two erasures
// ---------------------------------------------------------------------------------------------

fn seed_everything(s: &SqliteStore) -> (NofapConfig, Vec<NofapSite>) {
    let repo: &dyn NofapRepo = s;
    let config = sample::config();
    repo.save_config(&config).expect("config");
    let kept = vec![
        user_site("u1", SiteList::Block, "bloqueado.example", 1),
        user_site("u2", SiteList::Allow, "liberado.example", 2),
        learned_site("l1", 3),
    ];
    for site in &kept {
        repo.upsert_site(site).expect("site");
    }
    repo.upsert_site(&snooze("z1", 4, at(24 * 60)))
        .expect("snooze");
    repo.bump_day("2026-09-16", 22, DayKind::Lapse, 1)
        .expect("bump");
    repo.bump_day("2026-09-17", 15, DayKind::VisionCheck, 4)
        .expect("bump");
    UsageRepo::record(
        s,
        &AiUsage {
            at: at(0),
            kind: AiUsageKind::NofapVision,
            model: "claude-haiku-4-5".into(),
            input_tokens: 600,
            output_tokens: 15,
            cache_read_tokens: 0,
            cache_write_tokens: 0,
            cost_usd: 0.0007,
        },
    )
    .expect("usage");
    (config, kept)
}

fn nofap_calls(s: &SqliteStore) -> u64 {
    UsageRepo::totals_by_kind(s, TimeRange::new(at(-600), at(600)))
        .expect("totals")
        .into_iter()
        .find(|(kind, _)| *kind == AiUsageKind::NofapVision)
        .map_or(0, |(_, totals)| totals.calls)
}

#[test]
fn erase_history_deletes_counters_and_snoozes_only() {
    let s = store();
    let repo: &dyn NofapRepo = &s;
    let (config, kept) = seed_everything(&s);

    repo.erase_history().expect("erase");

    assert_eq!(rows(&s, "nofap_day"), 0);
    assert_eq!(
        repo.list_sites().expect("sites"),
        kept,
        "the snooze is gone; the user's and the learned sites stay"
    );
    assert_eq!(repo.load_config().expect("config"), config);
    assert_eq!(
        nofap_calls(&s),
        1,
        "the cost ledger is not history of use by site"
    );

    // The store keeps working, and erasing an empty history is fine.
    repo.bump_day("2026-09-17", 16, DayKind::BlockList, 1)
        .expect("bump");
    repo.erase_history().expect("again");
    assert_eq!(rows(&s, "nofap_day"), 0);
}

#[test]
fn wipe_deletes_the_counters_and_keeps_the_config_and_the_sites() {
    let s = store();
    let repo: &dyn NofapRepo = &s;
    let (config, kept) = seed_everything(&s);
    let all_sites = repo.list_sites().expect("sites");
    assert_eq!(all_sites.len(), kept.len() + 1);

    MaintenanceRepo::wipe_user_data(&s).expect("wipe");

    assert_eq!(rows(&s, "nofap_day"), 0, "the counters are user data");
    assert_eq!(nofap_calls(&s), 0, "and so is the cost ledger");
    assert_eq!(
        repo.load_config().expect("config"),
        config,
        "wiping the configuration would be an undelayed off switch"
    );
    assert!(repo.load_config().expect("config").enabled);
    assert_eq!(
        repo.list_sites().expect("sites"),
        all_sites,
        "every site stays, the snooze included"
    );
}

// ---------------------------------------------------------------------------------------------
// What stays readable in the file
// ---------------------------------------------------------------------------------------------

#[test]
fn a_learned_site_leaves_no_name_in_the_file() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("shield.sqlite");
    let s = SqliteStore::new(Db::open(&path).expect("open"));
    let repo: &dyn NofapRepo = &s;

    repo.upsert_site(&user_site(
        "u1",
        SiteList::Block,
        "digitado-pela-pessoa.example",
        1,
    ))
    .expect("user");
    // Even a caller that passes the name by mistake cannot get it written.
    repo.upsert_site(&NofapSite {
        host: Some("aprendido-pela-verificacao.example".into()),
        ..learned_site("l1", 2)
    })
    .expect("learned");
    repo.upsert_site(&NofapSite {
        host: Some("adiado-por-um-dia.example".into()),
        ..snooze("z1", 3, at(24 * 60))
    })
    .expect("snooze");

    assert!(
        on_disk(&path, b"digitado-pela-pessoa.example"),
        "the search finds what is stored as text"
    );
    assert!(!on_disk(&path, b"aprendido-pela-verificacao.example"));
    assert!(!on_disk(&path, b"adiado-por-um-dia.example"));
    assert!(on_disk(&path, &key(2)), "the learned row is its hash");
}

#[test]
fn erased_history_does_not_stay_in_the_file() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("shield.sqlite");
    let s = SqliteStore::new(Db::open(&path).expect("open"));
    let repo: &dyn NofapRepo = &s;

    let snooze_key = *b"SNOOZE-KEY-16-B!";
    repo.upsert_site(&NofapSite {
        host_key: snooze_key,
        ..snooze("z1", 0, at(24 * 60))
    })
    .expect("snooze");
    repo.upsert_site(&learned_site("l1", 0xAB))
        .expect("learned");
    // The day text of a counter row is the marker of the counters.
    repo.bump_day("2031-12-25", 7, DayKind::Lapse, 1)
        .expect("bump");
    assert!(on_disk(&path, &snooze_key));
    assert!(on_disk(&path, b"2031-12-25"));

    repo.erase_history().expect("erase");

    assert!(
        !on_disk(&path, &snooze_key),
        "freed pages are overwritten and the log is folded back"
    );
    assert!(!on_disk(&path, b"2031-12-25"));
    assert!(on_disk(&path, &key(0xAB)), "what was kept is still there");

    // The connection's own setting is back to what it was: on, as for every connection.
    let secure: i64 = s
        .db()
        .with(|c| Ok(c.query_row("PRAGMA secure_delete", [], |r| r.get(0))?))
        .expect("pragma");
    assert_eq!(secure, 1);
}

// ---------------------------------------------------------------------------------------------
// The reference
// ---------------------------------------------------------------------------------------------

/// The sites and counters of `ubiqx_engine::MemoryNofapRepo`, statement for statement. The
/// storage crate cannot depend on the engine, so the reference's rules are restated here
/// and the SQLite implementation is compared with them after every step of a long random
/// walk.
#[derive(Default)]
struct Reference {
    sites: Vec<NofapSite>,
    days: Vec<DayCount>,
}

impl Reference {
    fn upsert_site(&mut self, site: &NofapSite) {
        self.sites
            .retain(|s| s.id != site.id && !(s.list == site.list && s.host_key == site.host_key));
        self.sites.push(site.clone());
    }

    fn delete_site(&mut self, id: &str) {
        self.sites.retain(|s| s.id != id);
    }

    fn delete_sites_by_origin(&mut self, origin: SiteOrigin) -> u64 {
        let before = self.sites.len();
        self.sites.retain(|s| s.origin != origin);
        (before - self.sites.len()) as u64
    }

    fn purge_expired_sites(&mut self, now: DateTime<Utc>) -> u64 {
        let before = self.sites.len();
        self.sites
            .retain(|s| s.expires_at.is_none_or(|at| at > now));
        (before - self.sites.len()) as u64
    }

    fn bump_day(&mut self, day: &str, hour: u8, kind: DayKind, by: u32) {
        match self
            .days
            .iter_mut()
            .find(|c| c.day == day && c.hour == hour && c.kind == kind)
        {
            Some(count) => count.n = count.n.saturating_add(by),
            None => self.days.push(count(day, hour, kind, by)),
        }
    }

    fn days_between(&self, from: &str, to: &str) -> Vec<DayCount> {
        let mut out: Vec<DayCount> = self
            .days
            .iter()
            .filter(|c| c.day.as_str() >= from && c.day.as_str() <= to)
            .cloned()
            .collect();
        out.sort_by(|a, b| {
            (&a.day, a.hour, a.kind.as_str()).cmp(&(&b.day, b.hour, b.kind.as_str()))
        });
        out
    }

    fn delete_days_before(&mut self, day: &str) -> u64 {
        let before = self.days.len();
        self.days.retain(|c| c.day.as_str() >= day);
        (before - self.days.len()) as u64
    }

    fn erase_history(&mut self) {
        self.days.clear();
        self.sites.retain(|s| s.origin != SiteOrigin::Snooze);
    }
}

/// A small deterministic generator (no dependency, same walk on every run).
struct Walk(u64);

impl Walk {
    fn next(&mut self, below: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 33) as usize) % below
    }

    fn pick<T: Copy>(&mut self, of: &[T]) -> T {
        of[self.next(of.len())]
    }
}

#[test]
fn the_store_behaves_like_the_reference_step_by_step() {
    const IDS: [&str; 6] = ["s1", "s2", "s3", "s4", "s5", "s6"];
    const DAYS: [&str; 5] = [
        "2026-09-15",
        "2026-09-16",
        "2026-09-17",
        "2026-09-18",
        "2026-10-01",
    ];
    const KINDS: [DayKind; 4] = [
        DayKind::BlockList,
        DayKind::VisionCheck,
        DayKind::Lapse,
        DayKind::HelpShown,
    ];
    const ORIGINS: [SiteOrigin; 3] = SiteOrigin::ALL;
    const LISTS: [SiteList; 2] = [SiteList::Block, SiteList::Allow];
    const AMOUNTS: [u32; 5] = [0, 1, 2, 7, u32::MAX - 3];

    let s = store();
    let repo: &dyn NofapRepo = &s;
    let mut reference = Reference::default();
    let mut walk = Walk(0x5EED_2026);
    let mut seen = [0usize; 8];

    for step in 0..1500 {
        let op = walk.next(8);
        seen[op] += 1;
        match op {
            0 | 1 => {
                let origin = walk.pick(&ORIGINS);
                let n = walk.next(4) as u8;
                let site = NofapSite {
                    id: walk.pick(&IDS).into(),
                    list: walk.pick(&LISTS),
                    origin,
                    host: (origin == SiteOrigin::User).then(|| format!("site-{n}.example")),
                    host_key: key(n),
                    created_at: at(walk.next(10) as i64),
                    expires_at: (walk.next(2) == 0).then(|| at(walk.next(6) as i64)),
                };
                repo.upsert_site(&site).expect("upsert");
                reference.upsert_site(&site);
            }
            2 => {
                let id = walk.pick(&IDS);
                repo.delete_site(id).expect("delete");
                reference.delete_site(id);
            }
            3 => {
                // Rare, or the lists would be empty most of the time.
                if walk.next(6) == 0 {
                    let origin = walk.pick(&ORIGINS);
                    assert_eq!(
                        repo.delete_sites_by_origin(origin).expect("by origin"),
                        reference.delete_sites_by_origin(origin),
                        "step {step}"
                    );
                } else {
                    let now = at(walk.next(6) as i64);
                    assert_eq!(
                        repo.purge_expired_sites(now).expect("purge"),
                        reference.purge_expired_sites(now),
                        "step {step}"
                    );
                }
            }
            4..=6 => {
                let (day, hour, kind, by) = (
                    walk.pick(&DAYS),
                    walk.pick(&[0u8, 9, 23]),
                    walk.pick(&KINDS),
                    walk.pick(&AMOUNTS),
                );
                repo.bump_day(day, hour, kind, by).expect("bump");
                reference.bump_day(day, hour, kind, by);
            }
            _ => {
                if walk.next(12) == 0 {
                    repo.erase_history().expect("erase");
                    reference.erase_history();
                } else if walk.next(4) == 0 {
                    let day = walk.pick(&DAYS);
                    assert_eq!(
                        repo.delete_days_before(day).expect("prune"),
                        reference.delete_days_before(day),
                        "step {step}"
                    );
                }
            }
        }

        assert_eq!(
            repo.list_sites().expect("sites"),
            reference.sites,
            "sites after step {step}"
        );
        let (from, to) = (walk.pick(&DAYS), walk.pick(&DAYS));
        assert_eq!(
            repo.days_between(from, to).expect("days"),
            reference.days_between(from, to),
            "counters between {from} and {to} after step {step}"
        );
    }

    assert!(
        seen.iter().all(|n| *n > 50),
        "every operation was exercised: {seen:?}"
    );
    assert_eq!(
        repo.days_between("0000-01-01", "9999-12-31").expect("all"),
        reference.days_between("0000-01-01", "9999-12-31")
    );
}

/// Does `act` on a database that also holds rows the erase keeps (so no page is emptied by
/// the erase itself), erases the history, and says whether `marker` is still in the file.
fn left_behind_after_erase(
    marker: &[u8],
    seed: impl Fn(&dyn NofapRepo),
    act: impl Fn(&dyn NofapRepo),
) -> bool {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("shield.sqlite");
    let s = SqliteStore::new(Db::open(&path).expect("open"));
    let repo: &dyn NofapRepo = &s;
    for n in 1..=3u8 {
        repo.upsert_site(&user_site(
            &format!("u{n}"),
            SiteList::Block,
            &format!("mantido-{n}.example"),
            n,
        ))
        .expect("kept site");
    }
    repo.bump_day("2031-06-01", 9, DayKind::BlockList, 1)
        .expect("kept day");
    seed(repo);
    assert!(on_disk(&path, marker), "the search finds what is stored");
    act(repo);
    repo.erase_history().expect("erase");
    let kept = repo.list_sites().expect("sites");
    assert!(
        (1..=3).all(|n| kept.iter().any(|site| site.id == format!("u{n}"))),
        "the erase keeps the user's sites"
    );
    on_disk(&path, marker)
}

#[test]
fn what_was_deleted_before_the_erase_does_not_stay_in_the_file_either() {
    // Each of these left the tables before the person asked for the history to be erased.
    // An erase that only overwrote what it deletes itself would leave them in the file.
    let purged = *b"PURGED-SNOOZE-16";
    assert!(
        !left_behind_after_erase(
            &purged,
            |repo| {
                repo.upsert_site(&NofapSite {
                    host_key: purged,
                    ..snooze("z1", 0, at(10))
                })
                .expect("snooze");
            },
            |repo| assert_eq!(repo.purge_expired_sites(at(11)).expect("purge"), 1),
        ),
        "a snooze that expired and was purged"
    );

    let deleted = *b"DELETED-BY-ID-16";
    assert!(
        !left_behind_after_erase(
            &deleted,
            |repo| {
                repo.upsert_site(&NofapSite {
                    host_key: deleted,
                    ..learned_site("l1", 0)
                })
                .expect("learned");
            },
            |repo| repo.delete_site("l1").expect("delete"),
        ),
        "a site deleted by id"
    );

    let forgotten = *b"FORGOTTEN-SITE16";
    assert!(
        !left_behind_after_erase(
            &forgotten,
            |repo| {
                repo.upsert_site(&NofapSite {
                    host_key: forgotten,
                    ..learned_site("l1", 0)
                })
                .expect("learned");
            },
            |repo| {
                assert_eq!(
                    repo.delete_sites_by_origin(SiteOrigin::Learned)
                        .expect("forget"),
                    1
                );
            },
        ),
        "the learned sites the person asked to forget"
    );

    let replaced = *b"REPLACED-ROW--16";
    assert!(
        !left_behind_after_erase(
            &replaced,
            |repo| {
                repo.upsert_site(&NofapSite {
                    host_key: replaced,
                    ..learned_site("l1", 0)
                })
                .expect("learned");
                // A row written after it, so the old row is not at the edge of the page's
                // free space, where the next write would land on it anyway.
                repo.upsert_site(&user_site("u4", SiteList::Block, "depois.example", 4))
                    .expect("later row");
            },
            // The new row is longer than the old one, so it cannot land on its bytes.
            |repo| {
                repo.upsert_site(&NofapSite {
                    id: "l1".into(),
                    ..user_site(
                        "l1",
                        SiteList::Allow,
                        "um-nome-bem-mais-comprido-do-que-a-linha-antiga.example",
                        9,
                    )
                })
                .expect("replace");
            },
        ),
        "a row replaced by a newer one of the same id"
    );

    // A day the retention pruned: enough rows to fill pages of their own, which the prune
    // hands back to the file whole.
    assert!(
        !left_behind_after_erase(
            b"2031-03-09",
            |repo| {
                for hour in 0..24 {
                    for kind in DayKind::ALL {
                        repo.bump_day("2031-03-09", hour, kind, 1).expect("bump");
                    }
                }
            },
            |repo| {
                assert_eq!(
                    repo.delete_days_before("2031-03-10").expect("prune"),
                    24 * 27
                );
            },
        ),
        "a day the retention pruned"
    );

    // The person's own sentence, long enough to need pages of its own, then cleared.
    let sentence = "FRASE-QUE-A-PESSOA-ESCREVEU ".repeat(400);
    let with_sentence = NofapConfig {
        values: sentence,
        ..sample::config()
    };
    assert!(
        !left_behind_after_erase(
            b"FRASE-QUE-A-PESSOA-ESCREVEU",
            |repo| repo.save_config(&with_sentence).expect("config"),
            |repo| repo.save_config(&sample::config()).expect("cleared"),
        ),
        "a sentence the person cleared"
    );
}
