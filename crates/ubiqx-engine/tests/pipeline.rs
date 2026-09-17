//! End-to-end test of the engine with the in-memory SQLite store, the scripted platform and
//! fake AI: samples flow in, blocks are closed, classified, corrected and reported.

use std::sync::Arc;
use std::time::Duration;

use chrono::{Datelike, Duration as ChronoDuration, Local, Utc};
use parking_lot::Mutex;
use ubiqx_ai::fake::{FakeAdvisor, FakeClassifier, FakeReportWriter, FakeVisionClassifier};
use ubiqx_core::ports::*;
use ubiqx_core::*;
use ubiqx_engine::engine::LoopConfig;
use ubiqx_engine::*;
use ubiqx_platform::mock::{Scenario, Step};
use ubiqx_platform::PlatformServices;
use ubiqx_storage::{Db, SqliteStore};

struct CollectSink(Mutex<Vec<EngineEvent>>);

impl EventSink for CollectSink {
    fn emit(&self, event: EngineEvent) {
        self.0.lock().push(event);
    }
}

fn category(id: &str, name: &str, productive: bool) -> Category {
    Category {
        id: id.into(),
        name: name.into(),
        color: "#2563EB".into(),
        icon: "folder".into(),
        description: format!("Trabalho de {name}"),
        keywords: vec![],
        report_time: None,
        report_template: None,
        is_productive: productive,
        is_system: false,
        archived: false,
        sort_order: 0,
        created_at: Utc::now(),
    }
}

struct Harness {
    handle: EngineHandle,
    tx: tokio::sync::mpsc::Sender<ActivitySample>,
    store: Arc<SqliteStore>,
    sink: Arc<CollectSink>,
    _tmp: tempfile::TempDir,
}

async fn harness() -> Harness {
    let tmp = tempfile::tempdir().unwrap();
    let store = Arc::new(SqliteStore::new(Db::open_in_memory().unwrap()));
    CategoryRepo::upsert(store.as_ref(), &category("cat-ifro", "IFRO", true)).unwrap();
    CategoryRepo::upsert(store.as_ref(), &category("cat-inc", "Incubadora", true)).unwrap();
    let (platform, _) = PlatformServices::scripted(Scenario::new(vec![Step::app("A", "a", "t", 1)]));
    let sink = Arc::new(CollectSink(Mutex::new(vec![])));
    let deps = EngineDeps {
        platform: PlatformPorts {
            activity: platform.activity.clone(),
            urls: platform.urls.clone(),
            idle: platform.idle.clone(),
            capturer: platform.capturer.clone(),
            permissions: platform.permissions.clone(),
            secrets: platform.secrets.clone(),
            notifier: platform.notifier.clone(),
        },
        repos: Repos::from_store(store.clone()),
        ai: AiPorts {
            remote: Some(Arc::new(FakeClassifier::new([
                ("sei.ifro.edu.br", "cat-ifro"),
                ("visual studio code", "cat-inc"),
            ]))),
            vision: Some(Arc::new(FakeVisionClassifier::new(Some("cat-inc"), "Trabalhou no código"))),
            report_writer: Some(Arc::new(FakeReportWriter::new())),
            advisor: Some(Arc::new(FakeAdvisor::new())),
            requires_api_key: false,
        },
        sink: sink.clone(),
        clock: Arc::new(SystemClock),
        data_dir: tmp.path().to_path_buf(),
    };
    let cfg = LoopConfig {
        classify_every: Duration::from_millis(200),
        reports_every: Duration::from_secs(3600),
        nudges_every: Duration::from_secs(3600),
        retention_every: Duration::from_secs(3600),
        without_sampler: true,
    };
    let (handle, tx) = Engine::start_with(deps, cfg).unwrap();
    let mut s = handle.settings();
    s.sample_interval_secs = 5;
    s.min_block_secs = 5;
    s.classify_batch_min = 1;
    s.classify_max_wait_secs = 0;
    s.screenshot_interval_secs = 0;
    handle.update_settings(s).unwrap();
    Harness { handle, tx, store, sink, _tmp: tmp }
}

fn sample(at: chrono::DateTime<Utc>, app: &str, app_id: &str, title: &str, url: Option<&str>) -> ActivitySample {
    ActivitySample {
        at,
        app_name: app.into(),
        app_id: app_id.into(),
        window_title: title.into(),
        url: url.map(String::from),
        idle_secs: Some(0.0),
        window_id: None,
    }
}

async fn settle() {
    tokio::time::sleep(Duration::from_millis(800)).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn samples_become_classified_blocks_and_reports() {
    let h = harness().await;
    let t0 = Utc::now() - ChronoDuration::minutes(30);
    // 6 samples of SEI (30 s), then 6 samples of VS Code, then a switch to Slack (closes VS Code).
    for i in 0..6 {
        h.tx.send(sample(t0 + ChronoDuration::seconds(i * 5), "Google Chrome", "com.google.Chrome", "SEI - Processo 1", Some("https://sei.ifro.edu.br/x?token=1"))).await.unwrap();
    }
    for i in 6..12 {
        h.tx.send(sample(t0 + ChronoDuration::seconds(i * 5), "Visual Studio Code", "com.microsoft.VSCode", "api — main.rs", None)).await.unwrap();
    }
    h.tx.send(sample(t0 + ChronoDuration::seconds(60), "Slack", "com.tinyspeck.slackmacgap", "general", None)).await.unwrap();
    settle().await;

    let today = Local::now().date_naive();
    let blocks = timeline(h.handle.state(), today).unwrap();
    let closed: Vec<_> = blocks.iter().filter(|b| !b.is_open).collect();
    assert_eq!(closed.len(), 2, "two closed blocks: {blocks:#?}");
    let sei = closed.iter().find(|b| b.app_id == "com.google.Chrome").unwrap();
    assert_eq!(sei.domain.as_deref(), Some("sei.ifro.edu.br"));
    assert_eq!(sei.category_id.as_deref(), Some("cat-ifro"), "fake remote classifier by domain");
    assert_eq!(sei.source, Some(ClassificationSource::Llm));
    let payload = sei.ai_payload.as_deref().expect("payload recorded");
    assert!(!payload.contains("token=1"), "query string must be redacted: {payload}");
    let code = closed.iter().find(|b| b.app_id == "com.microsoft.VSCode").unwrap();
    assert_eq!(code.category_id.as_deref(), Some("cat-inc"));
    assert!(blocks.iter().any(|b| b.is_open && b.app_id == "com.tinyspeck.slackmacgap"));

    // Events were emitted for the UI.
    let events = h.sink.0.lock().clone();
    assert!(events.iter().any(|e| matches!(e, EngineEvent::BlockClosed { .. })));
    assert!(events.iter().any(|e| matches!(e, EngineEvent::BlocksClassified { .. })));

    // Dashboard aggregates.
    let d = dashboard(h.handle.state(), today).unwrap();
    assert!(d.stats.productive_secs >= 55, "{:?}", d.stats);
    assert!(d.totals.iter().any(|t| t.category_id.as_deref() == Some("cat-ifro")));

    // Correction: move the SEI block to Incubadora, day scope; a rule suggestion is offered.
    let out = h.handle.reclassify(&sei.id, "cat-inc", Some("era da incubadora".into()), ReclassifyScope::Day).unwrap();
    assert!(out.suggestions.iter().any(|s| s.pattern == "sei.ifro.edu.br"), "{out:?}");
    let corrected = BlockRepo::get(h.store.as_ref(), &sei.id).unwrap().unwrap();
    assert_eq!(corrected.category_id.as_deref(), Some("cat-inc"));
    assert_eq!(corrected.source, Some(ClassificationSource::User));
    assert_eq!(CorrectionRepo::count(h.store.as_ref()).unwrap(), 1);
    // Accepting the suggestion creates a learned rule.
    let rule = h.handle.accept_rule_suggestion(&out.suggestions[0]).unwrap();
    assert_eq!(rule.origin, RuleOrigin::Learned);

    // Reports: the fake writer produces items from the blocks.
    let report = h.handle.generate_report(today, "cat-inc").await.unwrap();
    assert!(!report.items.is_empty(), "{report:?}");
    assert!(report.summary_md.contains("Incubadora"));
    let monthly = monthly_report_md(h.handle.state(), "cat-inc", today.year(), today.month()).unwrap();
    assert!(monthly.contains("Incubadora"));

    // Manual entry and split.
    let manual = h.handle.add_manual_entry(t0 - ChronoDuration::hours(2), t0 - ChronoDuration::hours(1), "cat-ifro", Some("Reunião presencial".into())).unwrap();
    assert!(manual.is_manual);
    let new_id = h.handle.split_block(&manual.id, t0 - ChronoDuration::minutes(90)).unwrap();
    assert_ne!(new_id, manual.id);

    // Pause closes the open block; export writes a file.
    h.handle.pause();
    assert_eq!(h.handle.tracker_state(), TrackerState::Paused);
    let path = h.handle.export_json().unwrap();
    assert!(path.exists());
    h.handle.shutdown();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn private_mode_and_blocked_apps_keep_time_without_content() {
    let h = harness().await;
    let t0 = Utc::now() - ChronoDuration::minutes(10);
    for i in 0..4 {
        h.tx.send(sample(t0 + ChronoDuration::seconds(i * 5), "1Password", "com.1password.1password", "Vault", None)).await.unwrap();
    }
    settle().await;
    h.handle.set_private_mode(PrivateModeDuration::Minutes30).unwrap();
    assert_eq!(h.handle.tracker_state(), TrackerState::Private);
    for i in 5..9 {
        h.tx.send(sample(t0 + ChronoDuration::seconds(i * 5), "Google Chrome", "com.google.Chrome", "Banco - conta", Some("https://bank.example/secret"))).await.unwrap();
    }
    h.tx.send(sample(t0 + ChronoDuration::seconds(60), "Slack", "com.tinyspeck.slackmacgap", "general", None)).await.unwrap();
    settle().await;
    let today = Local::now().date_naive();
    let blocks = timeline(h.handle.state(), today).unwrap();
    let private: Vec<_> = blocks.iter().filter(|b| b.category_id.as_deref() == Some(system_categories::PRIVATE)).collect();
    assert!(private.len() >= 2, "{blocks:#?}");
    assert!(private.iter().all(|b| b.url.is_none() && b.title == ubiqx_core::segmenter::PRIVATE_TITLE));
    assert!(private.iter().all(|b| b.ai_payload.is_none()), "private blocks never go to the AI");
    h.handle.set_private_mode(PrivateModeDuration::Off).unwrap();
    assert_ne!(h.handle.tracker_state(), TrackerState::Paused);
    h.handle.shutdown();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn nudges_fire_on_long_distraction() {
    let h = harness().await;
    let state = h.handle.state().clone();
    let now = Utc::now();
    // 40 minutes of YouTube classified as distraction, ending now.
    let mut b = ActivityBlock::new_manual(now - ChronoDuration::minutes(40), now - ChronoDuration::seconds(30), system_categories::DISTRACTION.into(), None);
    b.is_manual = false;
    b.app_id = "com.google.Chrome".into();
    b.app_name = "Google Chrome".into();
    b.source = Some(ClassificationSource::Rule);
    BlockRepo::insert(h.store.as_ref(), &b).unwrap();
    let mut s = h.handle.settings();
    s.quiet_hours.enabled = false;
    h.handle.update_settings(s).unwrap();
    let nudges = ubiqx_engine::nudges::run_once(&state).unwrap();
    assert!(nudges.iter().any(|n| n.kind == NudgeKind::Unproductive), "{nudges:?}");
    // Second evaluation within the cooldown yields nothing.
    assert!(ubiqx_engine::nudges::run_once(&state).unwrap().is_empty());
    h.handle.shutdown();
}
