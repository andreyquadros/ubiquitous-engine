//! Integration tests for every repository of `ubiqx-storage`, against an in-memory database
//! (plus one file-backed test for migrations on reopen).

use std::collections::BTreeSet;

use chrono::{DateTime, NaiveDate, NaiveTime, TimeZone, Utc};
use ubiqx_core::ports::{
    AiUsage, AiUsageKind, BlockRepo, CategoryRepo, CorrectionRepo, KvRepo, MaintenanceRepo,
    NudgeRepo, ReportRepo, RuleRepo, ScreenshotRepo, SettingsRepo, UsageRepo,
};
use ubiqx_core::{
    system_categories, ActivityBlock, ActivityKind, Category, ClassificationSource, CoreError,
    Correction, DailyReport, Nudge, NudgeKind, ReportItem, Rule, RuleMatcher, RuleOrigin,
    Screenshot, Settings, TimeRange, VisionPolicy,
};
use ubiqx_storage::{Db, SqliteStore};

// ---------------------------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------------------------

fn store() -> SqliteStore {
    SqliteStore::new(Db::open_in_memory().expect("in-memory database"))
}

/// A fixed instant on 2026-09-17 (UTC) at second precision.
fn at(hour: u32, min: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 9, 17, hour, min, 0)
        .single()
        .expect("valid instant")
}

fn day() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 9, 17).expect("valid date")
}

fn range(from: DateTime<Utc>, to: DateTime<Utc>) -> TimeRange {
    TimeRange::new(from, to)
}

fn block(id: &str, from: DateTime<Utc>, to: DateTime<Utc>) -> ActivityBlock {
    ActivityBlock {
        id: id.into(),
        started_at: from,
        ended_at: to,
        app_name: "Google Chrome".into(),
        app_id: "com.google.Chrome".into(),
        title: "Edital 12/2026 - SEI".into(),
        title_key: "edital 12/2026 - sei".into(),
        url: Some("https://sei.ifro.edu.br/processo/42".into()),
        domain: Some("sei.ifro.edu.br".into()),
        category_id: None,
        confidence: 0.0,
        source: None,
        description: None,
        screenshot_id: None,
        sample_count: 12,
        is_open: false,
        classify_attempts: 0,
        next_attempt_at: None,
        needs_review: false,
        ai_payload: None,
        ai_sent_at: None,
        is_manual: false,
        note: None,
    }
}

fn classified(
    id: &str,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    cat: &str,
    source: ClassificationSource,
) -> ActivityBlock {
    ActivityBlock {
        category_id: Some(cat.into()),
        confidence: 0.75,
        source: Some(source),
        ..block(id, from, to)
    }
}

fn category(id: &str, name: &str) -> Category {
    Category {
        id: id.into(),
        name: name.into(),
        color: "#2563EB".into(),
        icon: "graduation-cap".into(),
        description: "Trabalho institucional".into(),
        keywords: vec!["ifro".into(), "sei".into()],
        report_time: Some(NaiveTime::from_hms_opt(17, 30, 0).expect("valid time")),
        report_template: Some("Voz institucional".into()),
        is_productive: true,
        is_system: false,
        archived: false,
        sort_order: 1,
        created_at: at(8, 0),
    }
}

fn seed_categories(store: &SqliteStore, ids: &[&str]) {
    for id in ids {
        CategoryRepo::upsert(store, &category(id, id)).expect("upsert category");
    }
}

fn insert_blocks(store: &SqliteStore, blocks: &[ActivityBlock]) {
    for b in blocks {
        BlockRepo::insert(store, b).expect("insert block");
    }
}

fn ids(blocks: &[ActivityBlock]) -> Vec<&str> {
    blocks.iter().map(|b| b.id.as_str()).collect()
}

fn rule(id: &str, cat: &str, priority: i32) -> Rule {
    Rule {
        id: id.into(),
        category_id: cat.into(),
        matcher: RuleMatcher::Domain,
        pattern: "ifro.edu.br".into(),
        priority,
        origin: RuleOrigin::Learned,
        enabled: true,
        created_at: at(8, 0),
        hit_count: 0,
        miss_count: 0,
        last_contradicted_at: None,
    }
}

fn screenshot(id: &str, when: DateTime<Utc>, block_id: Option<&str>) -> Screenshot {
    Screenshot {
        id: id.into(),
        at: when,
        path: format!("/tmp/ubiqx/{id}.webp"),
        width: 1280,
        height: 800,
        app_id: "com.google.Chrome".into(),
        block_id: block_id.map(Into::into),
        sent_to_ai: false,
    }
}

fn report(id: &str, cat: &str, date: NaiveDate, summary: &str) -> DailyReport {
    DailyReport {
        id: id.into(),
        date,
        category_id: cat.into(),
        generated_at: at(18, 0),
        summary_md: summary.into(),
        items: vec![ReportItem {
            activity: "Analisei o edital 12/2026".into(),
            kind: ActivityKind::Gestao,
            minutes: 45,
            evidence: vec!["SEI".into()],
            time_range: "09:00–09:45".into(),
            continuation_of: None,
        }],
        highlights: vec!["Edital revisado".into()],
        total_secs: 2700,
        model: "claude-sonnet-5".into(),
        input_tokens: 6000,
        output_tokens: 1500,
        stale: false,
        edited: false,
    }
}

fn nudge(id: &str, when: DateTime<Utc>, kind: NudgeKind) -> Nudge {
    Nudge {
        id: id.into(),
        at: when,
        kind,
        title: "UBI".into(),
        message: "Que tal uma pausa?".into(),
        seen: false,
    }
}

// ---------------------------------------------------------------------------------------------
// Db & migrations
// ---------------------------------------------------------------------------------------------

#[test]
fn system_categories_are_seeded() {
    let s = store();
    let cats = CategoryRepo::list(&s, true).expect("list");
    let by_id = |id: &str| {
        cats.iter()
            .find(|c| c.id == id)
            .unwrap_or_else(|| panic!("{id}"))
    };

    let expected = [
        (
            system_categories::UNCATEGORIZED,
            "Sem categoria",
            "#94A3B8",
            "help-circle",
        ),
        (
            system_categories::DISTRACTION,
            "Distração",
            "#F97316",
            "coffee",
        ),
        (system_categories::BREAK, "Pausa", "#A3E635", "pause"),
        (system_categories::PRIVATE, "Privado", "#64748B", "lock"),
    ];
    for (id, name, color, icon) in expected {
        let c = by_id(id);
        assert_eq!(
            (c.name.as_str(), c.color.as_str(), c.icon.as_str()),
            (name, color, icon)
        );
        assert!(c.is_system && !c.is_productive && !c.archived);
        assert!(c.sort_order >= 900, "system categories sort last");
        assert!(c.keywords.is_empty());
        assert_eq!(c.report_time, None);
    }
    assert_eq!(cats.iter().filter(|c| c.is_system).count(), 4);
}

#[test]
fn migrations_are_idempotent_on_reopen() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("nested").join("data").join("ubiqx.sqlite");

    {
        let db = Db::open(&path).expect("first open creates parent dirs");
        assert_eq!(db.schema_version().expect("version"), 1);
        let s = SqliteStore::new(db);
        KvRepo::set(&s, "last_report_check", "2026-09-17").expect("set");
        // A user category must survive alongside the seed on reopen.
        CategoryRepo::upsert(&s, &category("cat-ifro", "IFRO")).expect("upsert");
    }

    let db = Db::open(&path).expect("second open");
    assert_eq!(db.schema_version().expect("version"), 1);
    let mode: String = db
        .with(|c| Ok(c.query_row("PRAGMA journal_mode", [], |r| r.get(0))?))
        .expect("pragma");
    assert_eq!(mode, "wal");
    let fk: i64 = db
        .with(|c| Ok(c.query_row("PRAGMA foreign_keys", [], |r| r.get(0))?))
        .expect("pragma");
    assert_eq!(fk, 1);

    let s = SqliteStore::new(db);
    assert_eq!(
        KvRepo::get(&s, "last_report_check")
            .expect("get")
            .as_deref(),
        Some("2026-09-17")
    );
    let cats = CategoryRepo::list(&s, true).expect("list");
    assert_eq!(
        cats.iter().filter(|c| c.is_system).count(),
        4,
        "seed not duplicated"
    );
    assert!(cats.iter().any(|c| c.id == "cat-ifro"));
}

#[tokio::test]
async fn db_run_executes_on_the_blocking_pool() {
    let db = Db::open_in_memory().expect("db");
    let n: i64 = db
        .run(|c| Ok(c.query_row("SELECT COUNT(*) FROM categories", [], |r| r.get(0))?))
        .await
        .expect("run");
    assert_eq!(n, 4);

    let cloned = db.clone();
    cloned
        .run(|c| {
            c.execute(
                "INSERT INTO kv (key, value, updated_at) VALUES ('a', 'b', 0)",
                [],
            )?;
            Ok(())
        })
        .await
        .expect("insert through clone");
    let v = KvRepo::get(&SqliteStore::new(db), "a").expect("get");
    assert_eq!(v.as_deref(), Some("b"));
}

// ---------------------------------------------------------------------------------------------
// BlockRepo
// ---------------------------------------------------------------------------------------------

#[test]
fn block_round_trip_update_and_delete() {
    let s = store();
    seed_categories(&s, &["cat-a"]);
    let blocks: &dyn BlockRepo = &s;

    let mut b = ActivityBlock {
        category_id: Some("cat-a".into()),
        confidence: 0.75,
        source: Some(ClassificationSource::Vision),
        description: Some("Respondeu ao ofício".into()),
        screenshot_id: Some("shot-1".into()),
        classify_attempts: 2,
        next_attempt_at: Some(at(10, 0)),
        needs_review: true,
        ai_payload: Some("Google Chrome | Edital".into()),
        ai_sent_at: Some(at(9, 5)),
        is_manual: true,
        note: Some("reunião".into()),
        ..block("b1", at(9, 0), at(9, 30))
    };
    blocks.insert(&b).expect("insert");
    assert_eq!(blocks.get("b1").expect("get"), Some(b.clone()));
    assert_eq!(blocks.get("nope").expect("get"), None);

    b.title = "Novo título".into();
    b.ended_at = at(9, 45);
    b.source = Some(ClassificationSource::User);
    b.next_attempt_at = None;
    blocks.update(&b).expect("update");
    assert_eq!(blocks.get("b1").expect("get"), Some(b.clone()));

    blocks.delete("b1").expect("delete");
    assert_eq!(blocks.get("b1").expect("get"), None);
    blocks.delete("b1").expect("delete is idempotent");
    assert!(matches!(blocks.update(&b), Err(CoreError::NotFound(_))));
}

#[test]
fn foreign_keys_are_enforced() {
    let s = store();
    let b = classified(
        "b1",
        at(9, 0),
        at(9, 10),
        "does-not-exist",
        ClassificationSource::Llm,
    );
    assert!(matches!(
        BlockRepo::insert(&s, &b),
        Err(CoreError::Storage(_))
    ));
}

#[test]
fn open_block_is_the_newest_open_one() {
    let s = store();
    let blocks: &dyn BlockRepo = &s;
    assert_eq!(blocks.open_block().expect("none"), None);

    let closed = block("closed", at(9, 0), at(9, 10));
    let older_open = ActivityBlock {
        is_open: true,
        ..block("open-old", at(9, 10), at(9, 20))
    };
    let newer_open = ActivityBlock {
        is_open: true,
        ..block("open-new", at(9, 20), at(9, 25))
    };
    insert_blocks(&s, &[closed, older_open, newer_open]);

    let open = blocks.open_block().expect("open").expect("some");
    assert_eq!(open.id, "open-new");
    assert!(open.is_open);
}

#[test]
fn list_in_range_returns_overlapping_closed_blocks_in_order() {
    let s = store();
    insert_blocks(
        &s,
        &[
            block("after", at(10, 0), at(10, 10)), // starts exactly at `to`: excluded
            block("inside", at(9, 20), at(9, 40)),
            block("straddles-end", at(9, 50), at(10, 30)),
            block("before", at(8, 0), at(9, 0)), // ends exactly at `from`: excluded
            block("straddles-start", at(8, 45), at(9, 5)),
            ActivityBlock {
                is_open: true,
                ..block("open", at(9, 30), at(9, 35))
            },
            block("covers-all", at(8, 0), at(11, 0)),
        ],
    );
    let got = BlockRepo::list_in_range(&s, range(at(9, 0), at(10, 0))).expect("list");
    assert_eq!(
        ids(&got),
        ["covers-all", "straddles-start", "inside", "straddles-end"]
    );
}

#[test]
fn pending_remote_honours_retry_and_review_state() {
    let s = store();
    seed_categories(&s, &["cat-a"]);
    insert_blocks(
        &s,
        &[
            ActivityBlock {
                source: Some(ClassificationSource::User),
                ..block("user-unknown", at(8, 0), at(8, 10))
            },
            block("ready", at(9, 0), at(9, 10)),
            ActivityBlock {
                classify_attempts: 1,
                next_attempt_at: Some(at(10, 0)),
                ..block("backoff", at(9, 10), at(9, 20))
            },
            ActivityBlock {
                needs_review: true,
                ..block("review", at(9, 20), at(9, 30))
            },
            classified(
                "done",
                at(9, 30),
                at(9, 40),
                "cat-a",
                ClassificationSource::Llm,
            ),
            ActivityBlock {
                is_open: true,
                ..block("open", at(9, 40), at(9, 50))
            },
        ],
    );
    let blocks: &dyn BlockRepo = &s;

    let pending = blocks.list_pending_remote(at(9, 30), 10).expect("pending");
    assert_eq!(
        ids(&pending),
        ["ready"],
        "backoff still in the future, review/user/done/open excluded"
    );

    let pending = blocks.list_pending_remote(at(10, 0), 10).expect("pending");
    assert_eq!(
        ids(&pending),
        ["ready", "backoff"],
        "next_attempt_at <= now is due; oldest first"
    );

    let pending = blocks.list_pending_remote(at(10, 0), 1).expect("pending");
    assert_eq!(ids(&pending), ["ready"], "limit respected");

    let unclassified = blocks.list_unclassified(10).expect("unclassified");
    assert_eq!(
        ids(&unclassified),
        ["user-unknown", "ready", "backoff", "review"]
    );
    assert_eq!(
        ids(&blocks.list_unclassified(2).expect("limited")),
        ["user-unknown", "ready"]
    );

    let review = blocks.list_needs_review(10).expect("review");
    assert_eq!(ids(&review), ["review"]);
}

#[test]
fn record_attempt_and_ai_payload() {
    let s = store();
    insert_blocks(&s, &[block("b1", at(9, 0), at(9, 10))]);
    let blocks: &dyn BlockRepo = &s;

    blocks
        .record_attempt("b1", 3, Some(at(9, 45)), false)
        .expect("attempt");
    let b = blocks.get("b1").expect("get").expect("some");
    assert_eq!(
        (b.classify_attempts, b.next_attempt_at, b.needs_review),
        (3, Some(at(9, 45)), false)
    );

    blocks.record_attempt("b1", 5, None, true).expect("attempt");
    let b = blocks.get("b1").expect("get").expect("some");
    assert_eq!(
        (b.classify_attempts, b.next_attempt_at, b.needs_review),
        (5, None, true)
    );

    blocks
        .set_ai_payload("b1", "Chrome | edital", at(9, 46))
        .expect("payload");
    let b = blocks.get("b1").expect("get").expect("some");
    assert_eq!(b.ai_payload.as_deref(), Some("Chrome | edital"));
    assert_eq!(b.ai_sent_at, Some(at(9, 46)));

    assert!(matches!(
        blocks.record_attempt("ghost", 1, None, false),
        Err(CoreError::NotFound(_))
    ));
    assert!(matches!(
        blocks.set_ai_payload("ghost", "x", at(9, 0)),
        Err(CoreError::NotFound(_))
    ));
}

#[test]
fn set_classification_keeps_description_unless_given() {
    let s = store();
    seed_categories(&s, &["cat-a", "cat-b"]);
    insert_blocks(
        &s,
        &[
            ActivityBlock {
                description: Some("antiga".into()),
                needs_review: true,
                ..block("b1", at(9, 0), at(9, 10))
            },
            ActivityBlock {
                needs_review: true,
                ..block("b2", at(9, 10), at(9, 20))
            },
        ],
    );
    let blocks: &dyn BlockRepo = &s;

    blocks
        .set_classification("b1", Some("cat-a"), 0.75, ClassificationSource::Llm, None)
        .expect("classify");
    let b = blocks.get("b1").expect("get").expect("some");
    assert_eq!(b.category_id.as_deref(), Some("cat-a"));
    assert_eq!(b.confidence, 0.75);
    assert_eq!(b.source, Some(ClassificationSource::Llm));
    assert_eq!(
        b.description.as_deref(),
        Some("antiga"),
        "None keeps the description"
    );
    assert!(
        !b.needs_review,
        "assigning a category resolves the review flag"
    );

    blocks
        .set_classification(
            "b1",
            Some("cat-b"),
            1.0,
            ClassificationSource::User,
            Some("nova"),
        )
        .expect("classify");
    let b = blocks.get("b1").expect("get").expect("some");
    assert_eq!(b.category_id.as_deref(), Some("cat-b"));
    assert_eq!(b.description.as_deref(), Some("nova"));
    assert_eq!(b.source, Some(ClassificationSource::User));

    blocks
        .set_classification("b2", None, 0.0, ClassificationSource::Llm, None)
        .expect("unknown classification");
    let b = blocks.get("b2").expect("get").expect("some");
    assert_eq!(b.category_id, None);
    assert!(b.needs_review, "no category: review flag untouched");

    assert!(matches!(
        blocks.set_classification(
            "ghost",
            Some("cat-a"),
            1.0,
            ClassificationSource::Rule,
            None
        ),
        Err(CoreError::NotFound(_))
    ));
}

#[test]
fn split_creates_tail_and_trims_head() {
    let s = store();
    seed_categories(&s, &["cat-a"]);
    insert_blocks(
        &s,
        &[ActivityBlock {
            sample_count: 120,
            screenshot_id: Some("shot".into()),
            ai_payload: Some("payload".into()),
            description: Some("desc".into()),
            ..classified(
                "b1",
                at(9, 0),
                at(10, 0),
                "cat-a",
                ClassificationSource::Llm,
            )
        }],
    );
    let blocks: &dyn BlockRepo = &s;

    let tail_id = blocks.split("b1", at(9, 45)).expect("split");
    assert_ne!(tail_id, "b1");

    let head = blocks.get("b1").expect("get").expect("head");
    assert_eq!((head.started_at, head.ended_at), (at(9, 0), at(9, 45)));
    assert_eq!(head.sample_count, 90);
    assert_eq!(head.screenshot_id.as_deref(), Some("shot"));

    let tail = blocks.get(&tail_id).expect("get").expect("tail");
    assert_eq!((tail.started_at, tail.ended_at), (at(9, 45), at(10, 0)));
    assert_eq!(tail.sample_count, 30);
    assert_eq!(tail.app_id, head.app_id);
    assert_eq!(tail.title, head.title);
    assert_eq!(tail.url, head.url);
    assert_eq!(tail.domain, head.domain);
    assert_eq!(tail.category_id.as_deref(), Some("cat-a"));
    assert_eq!(tail.source, Some(ClassificationSource::Llm));
    assert_eq!(tail.confidence, 0.75);
    assert_eq!(tail.description.as_deref(), Some("desc"));
    assert_eq!(tail.screenshot_id, None);
    assert_eq!(tail.ai_payload, None);
    assert!(!tail.is_open);

    let in_range = blocks
        .list_in_range(range(at(9, 0), at(10, 0)))
        .expect("list");
    assert_eq!(ids(&in_range), ["b1", tail_id.as_str()]);

    assert!(
        matches!(blocks.split("b1", at(9, 0)), Err(CoreError::Invalid(_))),
        "at == started_at"
    );
    assert!(
        matches!(blocks.split("b1", at(9, 45)), Err(CoreError::Invalid(_))),
        "at == ended_at"
    );
    assert!(
        matches!(blocks.split("b1", at(11, 0)), Err(CoreError::Invalid(_))),
        "outside"
    );
    assert!(matches!(
        blocks.split("ghost", at(9, 30)),
        Err(CoreError::NotFound(_))
    ));
    assert_eq!(
        blocks
            .list_in_range(range(at(0, 0), at(23, 59)))
            .expect("list")
            .len(),
        2,
        "failed splits change nothing"
    );
}

#[test]
fn backfill_skips_user_blocks_and_matches_key_case_insensitively() {
    let s = store();
    seed_categories(&s, &["cat-a", "cat-b"]);
    let same_domain = |id: &str, from, to| block(id, from, to);
    insert_blocks(
        &s,
        &[
            same_domain("plain", at(9, 0), at(9, 10)),
            ActivityBlock {
                app_id: "COM.GOOGLE.CHROME".into(),
                domain: Some("SEI.ifro.edu.br".into()),
                needs_review: true,
                next_attempt_at: Some(at(12, 0)),
                ..classified(
                    "llm",
                    at(9, 10),
                    at(9, 20),
                    "cat-b",
                    ClassificationSource::Llm,
                )
            },
            classified(
                "user",
                at(9, 20),
                at(9, 30),
                "cat-b",
                ClassificationSource::User,
            ),
            ActivityBlock {
                domain: Some("github.com".into()),
                ..block("other-domain", at(9, 30), at(9, 40))
            },
            ActivityBlock {
                url: None,
                domain: None,
                ..block("no-domain", at(9, 55), at(9, 58))
            },
            same_domain("outside-range", at(12, 0), at(12, 10)),
            ActivityBlock {
                is_open: true,
                ..block("open", at(9, 40), at(9, 50))
            },
            ActivityBlock {
                app_id: "com.apple.mail".into(),
                ..block("other-app", at(9, 50), at(9, 55))
            },
        ],
    );
    let blocks: &dyn BlockRepo = &s;

    let changed = blocks
        .backfill_category(
            "com.google.chrome",
            Some("sei.ifro.edu.br"),
            range(at(9, 0), at(10, 0)),
            "cat-a",
            ClassificationSource::User,
        )
        .expect("backfill");
    assert_eq!(changed, 2);

    for id in ["plain", "llm"] {
        let b = blocks.get(id).expect("get").expect("some");
        assert_eq!(b.category_id.as_deref(), Some("cat-a"), "{id}");
        assert_eq!(b.source, Some(ClassificationSource::User), "{id}");
        assert_eq!(b.confidence, 1.0, "{id}");
        assert!(
            !b.needs_review && b.next_attempt_at.is_none(),
            "{id} no longer pending"
        );
    }
    for id in [
        "user",
        "other-domain",
        "no-domain",
        "outside-range",
        "open",
        "other-app",
    ] {
        let b = blocks.get(id).expect("get").expect("some");
        assert_ne!(
            b.category_id.as_deref(),
            Some("cat-a"),
            "{id} must be untouched"
        );
    }
    let user = blocks.get("user").expect("get").expect("some");
    assert_eq!(user.category_id.as_deref(), Some("cat-b"));

    // A `None` key matches only blocks without a domain: it is not a wildcard over every
    // domain of the app, so a domain-less browser block never drags other sites along.
    let changed = blocks
        .backfill_category(
            "com.google.chrome",
            None,
            range(at(9, 0), at(10, 0)),
            "cat-a",
            ClassificationSource::Memory,
        )
        .expect("backfill");
    assert_eq!(changed, 1);
    let b = blocks.get("no-domain").expect("get").expect("some");
    assert_eq!(b.category_id.as_deref(), Some("cat-a"));
    assert_eq!(b.source, Some(ClassificationSource::Memory));
    assert!(b.confidence < 1.0);
    let other = blocks.get("other-domain").expect("get").expect("some");
    assert_eq!(other.category_id, None, "other domains are untouched");
}

#[test]
fn touch_and_set_screenshot_leave_other_columns_alone() {
    let s = store();
    seed_categories(&s, &["cat-a"]);
    insert_blocks(
        &s,
        &[ActivityBlock {
            is_open: true,
            ..block("b1", at(9, 0), at(9, 5))
        }],
    );
    let blocks: &dyn BlockRepo = &s;

    // Concurrent writers: a user reclassification and a screenshot link.
    blocks
        .set_classification("b1", Some("cat-a"), 1.0, ClassificationSource::User, None)
        .expect("classify");
    blocks.set_screenshot("b1", "shot-1").expect("screenshot");

    // The segmenter's stale in-memory copy only carries the columns it owns.
    let stale = ActivityBlock {
        ended_at: at(9, 10),
        sample_count: 24,
        title: "Edital 12/2026 - SEI (v2)".into(),
        is_open: false,
        ..block("b1", at(9, 0), at(9, 5))
    };
    blocks.touch(&stale).expect("touch");

    let b = blocks.get("b1").expect("get").expect("some");
    assert_eq!(b.ended_at, at(9, 10));
    assert_eq!(b.sample_count, 24);
    assert_eq!(b.title, "Edital 12/2026 - SEI (v2)");
    assert!(!b.is_open);
    assert_eq!(
        b.category_id.as_deref(),
        Some("cat-a"),
        "classification kept"
    );
    assert_eq!(b.source, Some(ClassificationSource::User));
    assert_eq!(
        b.screenshot_id.as_deref(),
        Some("shot-1"),
        "screenshot kept"
    );

    assert!(matches!(
        blocks.touch(&block("ghost", at(9, 0), at(9, 5))),
        Err(CoreError::NotFound(_))
    ));
    assert!(matches!(
        blocks.set_screenshot("ghost", "shot-2"),
        Err(CoreError::NotFound(_))
    ));
}

#[test]
fn split_rejects_open_blocks() {
    let s = store();
    insert_blocks(
        &s,
        &[
            block("closed", at(8, 0), at(9, 0)),
            ActivityBlock {
                is_open: true,
                ..block("open", at(9, 0), at(10, 0))
            },
        ],
    );
    let blocks: &dyn BlockRepo = &s;

    assert!(matches!(
        blocks.split("open", at(9, 30)),
        Err(CoreError::Invalid(_))
    ));
    let open = blocks.open_block().expect("open").expect("some");
    assert_eq!(open.id, "open");
    assert_eq!((open.started_at, open.ended_at), (at(9, 0), at(10, 0)));
    assert_eq!(
        ids(&blocks
            .list_in_range(range(at(0, 0), at(23, 59)))
            .expect("list")),
        ["closed"]
    );
}

#[test]
fn totals_clip_blocks_to_the_range() {
    let s = store();
    seed_categories(&s, &["cat-a", "cat-b"]);
    let app = |id: &str, from, to, app_id: &str, cat: Option<&str>| ActivityBlock {
        app_id: app_id.into(),
        app_name: app_id.to_uppercase(),
        category_id: cat.map(Into::into),
        ..block(id, from, to)
    };
    insert_blocks(
        &s,
        &[
            app("straddles-start", at(8, 30), at(9, 30), "a", Some("cat-a")), // 30 min inside
            app("inside", at(9, 30), at(9, 45), "b", Some("cat-b")),          // 15 min
            app("straddles-end", at(9, 50), at(10, 30), "a", Some("cat-a")),  // 10 min inside
            app("uncategorised", at(9, 45), at(9, 50), "c", None),            // 5 min
            ActivityBlock {
                is_open: true,
                ..app("open", at(9, 0), at(9, 10), "a", Some("cat-a"))
            },
            app("outside", at(11, 0), at(11, 30), "a", Some("cat-a")),
        ],
    );
    let blocks: &dyn BlockRepo = &s;
    let r = range(at(9, 0), at(10, 0));

    let by_cat = blocks.totals_by_category(r).expect("totals");
    let rows: Vec<(Option<&str>, i64, u32)> = by_cat
        .iter()
        .map(|t| (t.category_id.as_deref(), t.secs, t.block_count))
        .collect();
    assert_eq!(
        rows,
        [
            (Some("cat-a"), 2400, 2),
            (Some("cat-b"), 900, 1),
            (None, 300, 1)
        ]
    );

    let by_app = blocks.totals_by_app(r, 10).expect("totals");
    let rows: Vec<(&str, &str, i64)> = by_app
        .iter()
        .map(|t| (t.app_id.as_str(), t.app_name.as_str(), t.secs))
        .collect();
    assert_eq!(rows, [("a", "A", 2400), ("b", "B", 900), ("c", "C", 300)]);

    let top = blocks.totals_by_app(r, 1).expect("totals");
    assert_eq!(top.len(), 1);
    assert_eq!(top[0].app_id, "a");

    assert!(blocks
        .totals_by_category(range(at(13, 0), at(14, 0)))
        .expect("empty")
        .is_empty());
}

#[test]
fn user_classified_blocks_newest_first() {
    let s = store();
    seed_categories(&s, &["cat-a"]);
    insert_blocks(
        &s,
        &[
            classified(
                "u-old",
                at(8, 0),
                at(8, 10),
                "cat-a",
                ClassificationSource::User,
            ),
            classified(
                "llm",
                at(8, 10),
                at(8, 20),
                "cat-a",
                ClassificationSource::Llm,
            ),
            classified(
                "u-new",
                at(9, 0),
                at(9, 10),
                "cat-a",
                ClassificationSource::User,
            ),
            ActivityBlock {
                source: Some(ClassificationSource::User),
                ..block("u-no-cat", at(9, 10), at(9, 20))
            },
        ],
    );
    let got = BlockRepo::list_user_classified(&s, 10).expect("list");
    assert_eq!(ids(&got), ["u-new", "u-old"]);
    assert_eq!(
        ids(&BlockRepo::list_user_classified(&s, 1).expect("list")),
        ["u-new"]
    );
}

#[test]
fn delete_before_purges_closed_blocks_and_detaches_screenshots() {
    let s = store();
    insert_blocks(
        &s,
        &[
            block("old", at(7, 0), at(7, 30)),
            ActivityBlock {
                is_open: true,
                ..block("old-open", at(7, 30), at(7, 40))
            },
            block("ends-at-cutoff", at(7, 40), at(8, 0)), // not strictly before: kept
            block("recent", at(9, 0), at(9, 30)),
        ],
    );
    ScreenshotRepo::insert(&s, &screenshot("shot", at(7, 10), Some("old"))).expect("shot");

    let deleted = BlockRepo::delete_before(&s, at(8, 0)).expect("purge");
    assert_eq!(deleted, 1);
    let remaining = BlockRepo::list_in_range(&s, range(at(0, 0), at(23, 0))).expect("list");
    assert_eq!(ids(&remaining), ["ends-at-cutoff", "recent"]);
    assert!(
        BlockRepo::get(&s, "old-open").expect("get").is_some(),
        "open blocks survive"
    );

    let shot = ScreenshotRepo::get(&s, "shot").expect("get").expect("some");
    assert_eq!(shot.block_id, None, "FK ON DELETE SET NULL");
}

// ---------------------------------------------------------------------------------------------
// CategoryRepo
// ---------------------------------------------------------------------------------------------

#[test]
fn categories_crud_and_system_protection() {
    let s = store();
    let cats: &dyn CategoryRepo = &s;

    let mut c = category("cat-ifro", "IFRO");
    cats.upsert(&c).expect("insert");
    assert_eq!(cats.get("cat-ifro").expect("get"), Some(c.clone()));
    assert_eq!(cats.get("nope").expect("get"), None);

    let archived = Category {
        archived: true,
        sort_order: 2,
        ..category("cat-old", "Antiga")
    };
    cats.upsert(&archived).expect("insert archived");

    let active: Vec<String> = cats
        .list(false)
        .expect("list")
        .into_iter()
        .map(|c| c.id)
        .collect();
    assert_eq!(
        active[0], "cat-ifro",
        "user categories sort before system ones"
    );
    assert!(!active.contains(&"cat-old".to_string()));
    let all: Vec<String> = cats
        .list(true)
        .expect("list")
        .into_iter()
        .map(|c| c.id)
        .collect();
    assert_eq!(&all[..2], ["cat-ifro", "cat-old"]);
    assert_eq!(all.len(), 6);

    c.name = "IFRO Campus".into();
    c.keywords.push("campus".into());
    c.report_time = None;
    c.created_at = at(23, 0); // must not overwrite the original creation time
    cats.upsert(&c).expect("update");
    let stored = cats.get("cat-ifro").expect("get").expect("some");
    assert_eq!(stored.name, "IFRO Campus");
    assert_eq!(stored.keywords, vec!["ifro", "sei", "campus"]);
    assert_eq!(stored.report_time, None);
    assert_eq!(stored.created_at, at(8, 0));

    // Deleting a user category cascades to rules and detaches blocks.
    RuleRepo::upsert(&s, &rule("r1", "cat-ifro", 1)).expect("rule");
    insert_blocks(
        &s,
        &[classified(
            "b1",
            at(9, 0),
            at(9, 10),
            "cat-ifro",
            ClassificationSource::Rule,
        )],
    );
    cats.delete("cat-ifro").expect("delete");
    assert_eq!(cats.get("cat-ifro").expect("get"), None);
    assert!(RuleRepo::list(&s).expect("rules").is_empty());
    let b = BlockRepo::get(&s, "b1").expect("get").expect("some");
    assert_eq!(b.category_id, None);

    cats.delete("cat-ifro").expect("delete unknown is a no-op");
    assert!(matches!(
        cats.delete(system_categories::UNCATEGORIZED),
        Err(CoreError::Invalid(_))
    ));
    assert!(cats
        .get(system_categories::UNCATEGORIZED)
        .expect("get")
        .is_some());
}

// ---------------------------------------------------------------------------------------------
// RuleRepo
// ---------------------------------------------------------------------------------------------

#[test]
fn rules_crud_hits_and_misses() {
    let s = store();
    seed_categories(&s, &["cat-a"]);
    let rules: &dyn RuleRepo = &s;

    rules.upsert(&rule("low", "cat-a", 1)).expect("insert");
    rules
        .upsert(&Rule {
            matcher: RuleMatcher::Regex,
            origin: RuleOrigin::User,
            ..rule("high", "cat-a", 10)
        })
        .expect("insert");
    let listed = rules.list().expect("list");
    assert_eq!(
        listed.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
        ["high", "low"]
    );
    assert_eq!(listed[0].matcher, RuleMatcher::Regex);
    assert_eq!(listed[0].origin, RuleOrigin::User);
    assert_eq!(listed[1], rule("low", "cat-a", 1));

    rules.increment_hits("low").expect("hit");
    rules.increment_hits("low").expect("hit");
    rules.record_miss("low", at(9, 0)).expect("miss");
    let low = rules
        .list()
        .expect("list")
        .into_iter()
        .find(|r| r.id == "low")
        .expect("low");
    assert_eq!(
        (low.hit_count, low.miss_count, low.last_contradicted_at),
        (2, 1, Some(at(9, 0)))
    );

    rules
        .upsert(&Rule {
            enabled: false,
            ..low.clone()
        })
        .expect("update");
    let low = rules
        .list()
        .expect("list")
        .into_iter()
        .find(|r| r.id == "low")
        .expect("low");
    assert!(!low.enabled);
    assert_eq!(low.hit_count, 2);

    rules.delete("low").expect("delete");
    assert_eq!(rules.list().expect("list").len(), 1);
    assert!(matches!(
        rules.increment_hits("low"),
        Err(CoreError::NotFound(_))
    ));
    assert!(matches!(
        rules.record_miss("low", at(9, 0)),
        Err(CoreError::NotFound(_))
    ));
}

// ---------------------------------------------------------------------------------------------
// CorrectionRepo
// ---------------------------------------------------------------------------------------------

#[test]
fn corrections_insert_list_recent_and_count() {
    let s = store();
    seed_categories(&s, &["cat-a", "cat-b"]);
    let corrections: &dyn CorrectionRepo = &s;
    assert_eq!(corrections.count().expect("count"), 0);

    let make = |id: &str, when| Correction {
        id: id.into(),
        block_id: "b1".into(),
        from_category_id: Some("cat-a".into()),
        to_category_id: "cat-b".into(),
        app_id: "com.google.Chrome".into(),
        app_name: "Google Chrome".into(),
        title_key: "edital".into(),
        domain: Some("sei.ifro.edu.br".into()),
        note: None,
        at: when,
    };
    let c1 = make("c1", at(9, 0));
    let c2 = Correction {
        from_category_id: None,
        note: Some("era incubadora".into()),
        ..make("c2", at(10, 0))
    };
    corrections.insert(&c1).expect("insert");
    corrections.insert(&c2).expect("insert");

    assert_eq!(corrections.count().expect("count"), 2);
    assert_eq!(
        corrections.list_recent(10).expect("list"),
        vec![c2.clone(), c1]
    );
    assert_eq!(corrections.list_recent(1).expect("list"), vec![c2]);
}

// ---------------------------------------------------------------------------------------------
// ScreenshotRepo
// ---------------------------------------------------------------------------------------------

#[test]
fn screenshots_full_lifecycle() {
    let s = store();
    insert_blocks(
        &s,
        &[
            block("b1", at(9, 0), at(9, 30)),
            block("b2", at(9, 30), at(10, 0)),
        ],
    );
    let shots: &dyn ScreenshotRepo = &s;

    let s1 = screenshot("s1", at(9, 5), Some("b1"));
    let s2 = screenshot("s2", at(9, 15), Some("b1"));
    let s3 = screenshot("s3", at(9, 40), None);
    for shot in [&s1, &s2, &s3] {
        shots.insert(shot).expect("insert");
    }
    assert_eq!(shots.get("s1").expect("get"), Some(s1.clone()));
    assert_eq!(shots.get("nope").expect("get"), None);

    assert_eq!(
        shots.latest_for_block("b1").expect("latest").map(|s| s.id),
        Some("s2".into())
    );
    assert_eq!(shots.latest_for_block("b2").expect("latest"), None);

    shots.attach_to_block("s3", "b2").expect("attach");
    assert_eq!(
        shots.latest_for_block("b2").expect("latest").map(|s| s.id),
        Some("s3".into())
    );
    assert!(matches!(
        shots.attach_to_block("nope", "b2"),
        Err(CoreError::NotFound(_))
    ));

    shots.mark_sent("s2").expect("sent");
    shots.mark_sent("s3").expect("sent");
    assert!(shots.get("s2").expect("get").expect("some").sent_to_ai);
    assert_eq!(shots.count_sent_since(at(9, 0)).expect("count"), 2);
    assert_eq!(shots.count_sent_since(at(9, 20)).expect("count"), 1);
    assert_eq!(shots.count_sent_since(at(11, 0)).expect("count"), 0);
    assert!(matches!(
        shots.mark_sent("nope"),
        Err(CoreError::NotFound(_))
    ));

    let deleted = shots.delete_before(at(9, 20)).expect("delete");
    let deleted_ids: BTreeSet<&str> = deleted.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(deleted_ids, BTreeSet::from(["s1", "s2"]));
    let s1_deleted = deleted.iter().find(|s| s.id == "s1").expect("s1");
    assert_eq!(
        s1_deleted.path, s1.path,
        "rows carry the path so files can be unlinked"
    );
    assert_eq!(shots.get("s1").expect("get"), None);
    assert!(shots.get("s3").expect("get").is_some());
    assert!(shots
        .delete_before(at(9, 20))
        .expect("nothing left")
        .is_empty());
}

// ---------------------------------------------------------------------------------------------
// ReportRepo
// ---------------------------------------------------------------------------------------------

#[test]
fn report_upsert_keeps_id_and_mark_stale() {
    let s = store();
    seed_categories(&s, &["cat-a", "cat-b"]);
    let reports: &dyn ReportRepo = &s;
    let next_day = day().succ_opt().expect("date");
    let far_day = NaiveDate::from_ymd_opt(2026, 10, 1).expect("date");

    let first = report("r-1", "cat-a", day(), "v1");
    reports.upsert(&first).expect("insert");
    assert_eq!(
        reports.get(day(), "cat-a").expect("get"),
        Some(first.clone())
    );
    assert_eq!(reports.get(day(), "cat-b").expect("get"), None);

    let regenerated = DailyReport {
        stale: true,
        edited: true,
        ..report("r-2", "cat-a", day(), "v2")
    };
    reports.upsert(&regenerated).expect("replace");
    let stored = reports.get(day(), "cat-a").expect("get").expect("some");
    assert_eq!(
        stored.id, "r-1",
        "same (date, category) keeps the original id"
    );
    assert_eq!(
        stored,
        DailyReport {
            id: "r-1".into(),
            ..regenerated
        }
    );

    reports
        .upsert(&report("r-3", "cat-b", day(), "b"))
        .expect("insert");
    reports
        .upsert(&report("r-4", "cat-a", next_day, "next"))
        .expect("insert");
    reports
        .upsert(&report("r-5", "cat-a", far_day, "far"))
        .expect("insert");

    let today: Vec<String> = reports
        .list_for_date(day())
        .expect("list")
        .into_iter()
        .map(|r| r.id)
        .collect();
    assert_eq!(today, ["r-1", "r-3"]);
    let between: Vec<String> = reports
        .list_between(day(), next_day)
        .expect("list")
        .into_iter()
        .map(|r| r.id)
        .collect();
    assert_eq!(between, ["r-1", "r-3", "r-4"], "both bounds inclusive");

    reports.mark_stale(next_day).expect("stale");
    assert!(
        reports
            .get(next_day, "cat-a")
            .expect("get")
            .expect("some")
            .stale
    );
    assert!(
        !reports
            .get(day(), "cat-b")
            .expect("get")
            .expect("some")
            .stale
    );

    reports.delete("r-3").expect("delete");
    assert_eq!(reports.get(day(), "cat-b").expect("get"), None);
    reports.delete("r-3").expect("idempotent");
}

// ---------------------------------------------------------------------------------------------
// NudgeRepo
// ---------------------------------------------------------------------------------------------

#[test]
fn nudges_lifecycle_and_rate_limit_queries() {
    let s = store();
    let nudges: &dyn NudgeRepo = &s;

    let n1 = nudge("n1", at(9, 0), NudgeKind::BreakSuggested);
    let n2 = nudge("n2", at(10, 0), NudgeKind::Praise);
    let n3 = nudge("n3", at(11, 0), NudgeKind::Praise);
    for n in [&n1, &n2, &n3] {
        nudges.insert(n).expect("insert");
    }

    assert_eq!(
        nudges.list_recent(10).expect("list"),
        vec![n3.clone(), n2.clone(), n1.clone()]
    );
    assert_eq!(nudges.list_recent(1).expect("list"), vec![n3.clone()]);

    assert_eq!(
        nudges.last_of_kind(NudgeKind::Praise).expect("last"),
        Some(at(11, 0))
    );
    assert_eq!(
        nudges
            .last_of_kind(NudgeKind::BreakSuggested)
            .expect("last"),
        Some(at(9, 0))
    );
    assert_eq!(nudges.last_of_kind(NudgeKind::Idle).expect("last"), None);

    assert_eq!(
        nudges.count_since(at(10, 0)).expect("count"),
        2,
        "inclusive lower bound"
    );
    assert_eq!(nudges.count_since(at(12, 0)).expect("count"), 0);

    nudges.mark_seen("n1").expect("seen");
    let seen: Vec<bool> = nudges
        .list_recent(10)
        .expect("list")
        .iter()
        .map(|n| n.seen)
        .collect();
    assert_eq!(seen, [false, false, true]);
    nudges.mark_all_seen().expect("all seen");
    assert!(nudges.list_recent(10).expect("list").iter().all(|n| n.seen));
    assert!(matches!(
        nudges.mark_seen("nope"),
        Err(CoreError::NotFound(_))
    ));

    // Report and attention nudges bypass the daily cap, so they are not counted.
    nudges
        .insert(&nudge("n4", at(10, 30), NudgeKind::ReportReady))
        .expect("insert");
    nudges
        .insert(&nudge("n5", at(10, 45), NudgeKind::Attention))
        .expect("insert");
    assert_eq!(nudges.count_since(at(10, 0)).expect("count"), 2);
    assert_eq!(nudges.list_recent(10).expect("list").len(), 5);
}

// ---------------------------------------------------------------------------------------------
// SettingsRepo
// ---------------------------------------------------------------------------------------------

#[test]
fn settings_default_then_round_trip_with_serde_defaults() {
    let s = store();
    let settings: &dyn SettingsRepo = &s;
    assert_eq!(settings.load().expect("load"), Settings::default());

    let custom = Settings {
        tracking_enabled: false,
        private_mode: true,
        private_until: Some(at(12, 0)),
        vision_policy: VisionPolicy::OnlyApps {
            apps: vec!["com.google.Chrome".into()],
        },
        blocked_domains: vec!["bank.example".into()],
        report_default_time: NaiveTime::from_hms_opt(17, 15, 0).expect("time"),
        user_profile: Some("Professor no IFRO".into()),
        ai_monthly_budget_usd: 12.5,
        ..Settings::default()
    };
    settings.save(&custom).expect("save");
    assert_eq!(settings.load().expect("load"), custom);

    let again = Settings {
        local_only: true,
        ..custom
    };
    settings
        .save(&again)
        .expect("save overwrites the single row");
    assert_eq!(settings.load().expect("load"), again);

    // A JSON written by an older build lacks fields: they take their serde defaults.
    s.db()
        .with(|c| {
            c.execute(
                "UPDATE settings SET json = ?1 WHERE id = 1",
                [r#"{"language":"en-US"}"#],
            )?;
            Ok(())
        })
        .expect("raw update");
    let loaded = settings.load().expect("load");
    assert_eq!(loaded.language, "en-US");
    assert_eq!(
        loaded,
        Settings {
            language: "en-US".into(),
            ..Settings::default()
        }
    );
}

// ---------------------------------------------------------------------------------------------
// UsageRepo
// ---------------------------------------------------------------------------------------------

#[test]
fn usage_totals_sum_calls_tokens_and_cost_in_range() {
    let s = store();
    let usage: &dyn UsageRepo = &s;
    let make = |when, kind, input: u32, output: u32, cost: f64| AiUsage {
        at: when,
        kind,
        model: "claude-haiku-4-5".into(),
        input_tokens: input,
        output_tokens: output,
        cache_read_tokens: 100,
        cache_write_tokens: 0,
        cost_usd: cost,
    };
    usage
        .record(&make(at(9, 0), AiUsageKind::Classify, 300, 40, 0.0005))
        .expect("record");
    usage
        .record(&make(at(9, 30), AiUsageKind::Vision, 1100, 60, 0.0014))
        .expect("record");
    usage
        .record(&make(at(10, 0), AiUsageKind::Report, 6000, 1500, 0.04))
        .expect("record");

    let t = usage.totals(range(at(9, 0), at(10, 0))).expect("totals");
    assert_eq!((t.calls, t.input_tokens, t.output_tokens), (2, 1400, 100));
    assert!(
        (t.cost_usd - 0.0019).abs() < 1e-9,
        "half-open range excludes the report"
    );

    let all = usage.totals(range(at(0, 0), at(23, 0))).expect("totals");
    assert_eq!(
        (all.calls, all.input_tokens, all.output_tokens),
        (3, 7400, 1600)
    );

    let none = usage.totals(range(at(12, 0), at(13, 0))).expect("totals");
    assert_eq!(none, Default::default());
}

// ---------------------------------------------------------------------------------------------
// KvRepo
// ---------------------------------------------------------------------------------------------

#[test]
fn kv_get_set_and_overwrite() {
    let s = store();
    let kv: &dyn KvRepo = &s;
    assert_eq!(kv.get("missing").expect("get"), None);
    kv.set("last_report_check", "2026-09-17T18:00:00Z")
        .expect("set");
    assert_eq!(
        kv.get("last_report_check").expect("get").as_deref(),
        Some("2026-09-17T18:00:00Z")
    );
    kv.set("last_report_check", "2026-09-18T18:00:00Z")
        .expect("overwrite");
    assert_eq!(
        kv.get("last_report_check").expect("get").as_deref(),
        Some("2026-09-18T18:00:00Z")
    );
    kv.set("empty", "").expect("empty value");
    assert_eq!(kv.get("empty").expect("get").as_deref(), Some(""));
}

// ---------------------------------------------------------------------------------------------
// MaintenanceRepo
// ---------------------------------------------------------------------------------------------

#[test]
fn wipe_user_data_keeps_categories_settings_and_user_rules() {
    let s = store();
    seed_categories(&s, &["cat-a"]);
    insert_blocks(
        &s,
        &[
            classified(
                "b1",
                at(9, 0),
                at(9, 30),
                "cat-a",
                ClassificationSource::Llm,
            ),
            ActivityBlock {
                is_open: true,
                ..block("orphan-open", at(9, 30), at(9, 40))
            },
        ],
    );
    ScreenshotRepo::insert(&s, &screenshot("s1", at(9, 10), Some("b1"))).expect("screenshot");
    CorrectionRepo::insert(
        &s,
        &Correction {
            id: "c1".into(),
            block_id: "b1".into(),
            from_category_id: None,
            to_category_id: "cat-a".into(),
            app_id: "com.google.Chrome".into(),
            app_name: "Google Chrome".into(),
            title_key: "edital 12/2026 - sei".into(),
            domain: Some("sei.ifro.edu.br".into()),
            note: Some("era do IFRO".into()),
            at: at(9, 35),
        },
    )
    .expect("correction");
    ReportRepo::upsert(&s, &report("r1", "cat-a", day(), "Resumo")).expect("report");
    NudgeRepo::insert(&s, &nudge("n1", at(10, 0), NudgeKind::Praise)).expect("nudge");
    UsageRepo::record(
        &s,
        &AiUsage {
            at: at(10, 0),
            kind: AiUsageKind::Classify,
            model: "m".into(),
            input_tokens: 10,
            output_tokens: 5,
            cache_read_tokens: 0,
            cache_write_tokens: 0,
            cost_usd: 0.01,
        },
    )
    .expect("usage");
    KvRepo::set(&s, "advice_2026-09-17", "{}").expect("kv");
    RuleRepo::upsert(
        &s,
        &Rule {
            origin: RuleOrigin::User,
            ..rule("user-rule", "cat-a", 1)
        },
    )
    .expect("rule");
    RuleRepo::upsert(&s, &rule("learned-rule", "cat-a", 0)).expect("rule");
    let settings = Settings {
        user_profile: Some("Servidor do IFRO".into()),
        ..Settings::default()
    };
    SettingsRepo::save(&s, &settings).expect("settings");

    MaintenanceRepo::wipe_user_data(&s).expect("wipe");

    let blocks: &dyn BlockRepo = &s;
    assert_eq!(blocks.get("b1").expect("get"), None);
    assert_eq!(
        blocks.open_block().expect("open"),
        None,
        "orphan open row gone"
    );
    assert_eq!(ScreenshotRepo::get(&s, "s1").expect("get"), None);
    assert_eq!(CorrectionRepo::count(&s).expect("count"), 0);
    assert!(CorrectionRepo::list_recent(&s, 10)
        .expect("list")
        .is_empty());
    assert!(ReportRepo::list_for_date(&s, day())
        .expect("list")
        .is_empty());
    assert!(NudgeRepo::list_recent(&s, 10).expect("list").is_empty());
    assert_eq!(
        UsageRepo::totals(&s, range(at(0, 0), at(23, 59)))
            .expect("totals")
            .calls,
        0
    );
    assert_eq!(KvRepo::get(&s, "advice_2026-09-17").expect("kv"), None);

    let rules = RuleRepo::list(&s).expect("rules");
    assert_eq!(
        rules.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
        ["user-rule"]
    );
    assert!(CategoryRepo::get(&s, "cat-a").expect("get").is_some());
    assert_eq!(
        SettingsRepo::load(&s)
            .expect("settings")
            .user_profile
            .as_deref(),
        Some("Servidor do IFRO")
    );

    // The store keeps working after the VACUUM.
    insert_blocks(&s, &[block("b2", at(11, 0), at(11, 30))]);
    assert!(blocks.get("b2").expect("get").is_some());
}
