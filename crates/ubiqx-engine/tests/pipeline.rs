//! End-to-end test of the engine with the in-memory SQLite store, the scripted platform and
//! fake AI: samples flow in, blocks are closed, classified, corrected and reported.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Datelike, Duration as ChronoDuration, Local, TimeZone, Utc};
use parking_lot::Mutex;
use ubiqx_ai::fake::{FakeAdvisor, FakeClassifier, FakeReportWriter, FakeVisionClassifier};
use ubiqx_core::clock::FixedClock;
use ubiqx_core::ports::*;
use ubiqx_core::*;
use ubiqx_engine::engine::LoopConfig;
use ubiqx_engine::*;
use ubiqx_platform::mock::{Scenario, ScriptedPlatform, Step};
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

/// In-memory secret store that never falls back to environment variables, so the tests keep
/// their outcome on a developer machine with real vendor keys exported.
#[derive(Default)]
struct MemorySecrets(Mutex<HashMap<String, String>>);

impl SecretStore for MemorySecrets {
    fn get(&self, key: &str) -> CoreResult<Option<String>> {
        Ok(self.0.lock().get(key).cloned())
    }

    fn set(&self, key: &str, value: &str) -> CoreResult<()> {
        self.0.lock().insert(key.into(), value.into());
        Ok(())
    }

    fn delete(&self, key: &str) -> CoreResult<()> {
        self.0.lock().remove(key);
        Ok(())
    }
}

struct Harness {
    handle: EngineHandle,
    tx: tokio::sync::mpsc::Sender<ActivitySample>,
    store: Arc<SqliteStore>,
    sink: Arc<CollectSink>,
    platform: Arc<ScriptedPlatform>,
    clock: FixedClock,
    tmp: tempfile::TempDir,
}

/// A remote classifier that always fails with the given error (built per call).
struct FailingClassifier(fn() -> CoreError);

#[async_trait]
impl RemoteClassifier for FailingClassifier {
    fn name(&self) -> &'static str {
        "failing"
    }

    async fn classify_batch(
        &self,
        _blocks: &[ActivityBlock],
        _ctx: &ClassificationContext,
    ) -> CoreResult<Vec<Classification>> {
        Err((self.0)())
    }
}

fn fake_ai() -> AiPorts {
    AiPorts {
        remote: Some(Arc::new(FakeClassifier::new([
            ("sei.ifro.edu.br", "cat-ifro"),
            ("visual studio code", "cat-inc"),
        ]))),
        vision: Some(Arc::new(FakeVisionClassifier::new(
            Some("cat-inc"),
            "Trabalhou no código",
        ))),
        report_writer: Some(Arc::new(FakeReportWriter::new())),
        advisor: Some(Arc::new(FakeAdvisor::new())),
        requires_api_key: false,
    }
}

fn failing_ai(err: fn() -> CoreError) -> AiPorts {
    AiPorts {
        remote: Some(Arc::new(FailingClassifier(err))),
        ..fake_ai()
    }
}

/// Mid-afternoon UTC, so "today" is the same local day for any plausible test timezone.
fn fixed_now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 9, 17, 15, 0, 0).unwrap()
}

struct HarnessOptions {
    ai: AiPorts,
    /// Runs against the store before the engine starts (e.g. leave an open block behind).
    seed: fn(&SqliteStore),
    onboarding_done: bool,
    /// Keys stored before the engine starts, per provider.
    keys: Vec<(AiProvider, &'static str)>,
}

impl Default for HarnessOptions {
    fn default() -> Self {
        Self {
            ai: fake_ai(),
            seed: |_| {},
            onboarding_done: true,
            keys: vec![],
        }
    }
}

async fn harness() -> Harness {
    harness_with(HarnessOptions::default()).await
}

async fn harness_with(opts: HarnessOptions) -> Harness {
    let tmp = tempfile::tempdir().unwrap();
    let store = Arc::new(SqliteStore::new(Db::open_in_memory().unwrap()));
    CategoryRepo::upsert(store.as_ref(), &category("cat-ifro", "IFRO", true)).unwrap();
    CategoryRepo::upsert(store.as_ref(), &category("cat-inc", "Incubadora", true)).unwrap();
    (opts.seed)(store.as_ref());
    let (platform, scripted) =
        PlatformServices::scripted(Scenario::new(vec![Step::app("A", "a", "t", 1)]));
    let secrets: Arc<dyn SecretStore> = Arc::new(MemorySecrets::default());
    for (provider, key) in &opts.keys {
        secrets.set(provider.secret_key(), key).unwrap();
    }
    let sink = Arc::new(CollectSink(Mutex::new(vec![])));
    let clock = FixedClock::new(fixed_now());
    let deps = EngineDeps {
        platform: PlatformPorts {
            activity: platform.activity.clone(),
            urls: platform.urls.clone(),
            idle: platform.idle.clone(),
            capturer: platform.capturer.clone(),
            permissions: platform.permissions.clone(),
            secrets,
            notifier: platform.notifier.clone(),
        },
        repos: Repos::from_store(store.clone()),
        ai: opts.ai,
        sink: sink.clone(),
        clock: Arc::new(clock.clone()),
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
    s.onboarding_done = opts.onboarding_done;
    s.sample_interval_secs = 5;
    s.min_block_secs = 5;
    s.classify_batch_min = 1;
    s.classify_max_wait_secs = 0;
    s.screenshot_interval_secs = 0;
    handle.update_settings(s).unwrap();
    Harness {
        handle,
        tx,
        store,
        sink,
        platform: scripted,
        clock,
        tmp,
    }
}

/// A closed, unclassified SEI block ending `mins_ago` minutes before the fixed clock.
fn pending_block(id: &str, mins_ago: i64) -> ActivityBlock {
    let mut b = ActivityBlock::new_manual(
        fixed_now() - ChronoDuration::minutes(mins_ago + 10),
        fixed_now() - ChronoDuration::minutes(mins_ago),
        "cat-ifro".into(),
        None,
    );
    b.id = id.into();
    b.is_manual = false;
    b.category_id = None;
    b.confidence = 0.0;
    b.source = None;
    b.app_id = "com.google.Chrome".into();
    b.app_name = "Google Chrome".into();
    b.title = "SEI - Processo 1".into();
    b.title_key = "sei - processo 1".into();
    b.url = Some("https://sei.ifro.edu.br/x".into());
    b.domain = Some("sei.ifro.edu.br".into());
    b
}

fn get_block(h: &Harness, id: &str) -> ActivityBlock {
    BlockRepo::get(h.store.as_ref(), id).unwrap().unwrap()
}

fn nudges_of(h: &Harness, kind: NudgeKind) -> usize {
    NudgeRepo::list_recent(h.store.as_ref(), 50)
        .unwrap()
        .iter()
        .filter(|n| n.kind == kind)
        .count()
}

fn sample(
    at: chrono::DateTime<Utc>,
    app: &str,
    app_id: &str,
    title: &str,
    url: Option<&str>,
) -> ActivitySample {
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
    let t0 = fixed_now() - ChronoDuration::minutes(30);
    // 6 samples of SEI (30 s), then 6 samples of VS Code, then a switch to Slack (closes VS Code).
    for i in 0..6 {
        h.tx.send(sample(
            t0 + ChronoDuration::seconds(i * 5),
            "Google Chrome",
            "com.google.Chrome",
            "SEI - Processo 1",
            Some("https://sei.ifro.edu.br/x?token=1"),
        ))
        .await
        .unwrap();
    }
    for i in 6..12 {
        h.tx.send(sample(
            t0 + ChronoDuration::seconds(i * 5),
            "Visual Studio Code",
            "com.microsoft.VSCode",
            "api — main.rs",
            None,
        ))
        .await
        .unwrap();
    }
    h.tx.send(sample(
        t0 + ChronoDuration::seconds(60),
        "Slack",
        "com.tinyspeck.slackmacgap",
        "general",
        None,
    ))
    .await
    .unwrap();
    settle().await;

    let today = fixed_now().with_timezone(&Local).date_naive();
    let blocks = timeline(h.handle.state(), today).unwrap();
    let closed: Vec<_> = blocks.iter().filter(|b| !b.is_open).collect();
    assert_eq!(closed.len(), 2, "two closed blocks: {blocks:#?}");
    let sei = closed
        .iter()
        .find(|b| b.app_id == "com.google.Chrome")
        .unwrap();
    assert_eq!(sei.domain.as_deref(), Some("sei.ifro.edu.br"));
    assert_eq!(
        sei.category_id.as_deref(),
        Some("cat-ifro"),
        "fake remote classifier by domain"
    );
    assert_eq!(sei.source, Some(ClassificationSource::Llm));
    let payload = sei.ai_payload.as_deref().expect("payload recorded");
    assert!(
        !payload.contains("token=1"),
        "query string must be redacted: {payload}"
    );
    let code = closed
        .iter()
        .find(|b| b.app_id == "com.microsoft.VSCode")
        .unwrap();
    assert_eq!(code.category_id.as_deref(), Some("cat-inc"));
    assert!(blocks
        .iter()
        .any(|b| b.is_open && b.app_id == "com.tinyspeck.slackmacgap"));

    // Events were emitted for the UI.
    let events = h.sink.0.lock().clone();
    assert!(events
        .iter()
        .any(|e| matches!(e, EngineEvent::BlockClosed { .. })));
    assert!(events
        .iter()
        .any(|e| matches!(e, EngineEvent::BlocksClassified { .. })));

    // Dashboard aggregates.
    let d = dashboard(h.handle.state(), today).unwrap();
    assert!(d.stats.productive_secs >= 55, "{:?}", d.stats);
    assert!(d
        .totals
        .iter()
        .any(|t| t.category_id.as_deref() == Some("cat-ifro")));

    // Correction: move the SEI block to Incubadora, day scope; a rule suggestion is offered.
    let out = h
        .handle
        .reclassify(
            &sei.id,
            "cat-inc",
            Some("era da incubadora".into()),
            ReclassifyScope::Day,
        )
        .unwrap();
    assert!(
        out.suggestions
            .iter()
            .any(|s| s.pattern == "sei.ifro.edu.br"),
        "{out:?}"
    );
    let corrected = BlockRepo::get(h.store.as_ref(), &sei.id).unwrap().unwrap();
    assert_eq!(corrected.category_id.as_deref(), Some("cat-inc"));
    assert_eq!(corrected.source, Some(ClassificationSource::User));
    assert_eq!(CorrectionRepo::count(h.store.as_ref()).unwrap(), 1);
    // Accepting the suggestion creates a learned rule.
    let rule = h
        .handle
        .accept_rule_suggestion(&out.suggestions[0])
        .unwrap();
    assert_eq!(rule.origin, RuleOrigin::Learned);

    // Reports: the fake writer produces items from the blocks.
    let report = h.handle.generate_report(today, "cat-inc").await.unwrap();
    assert!(!report.items.is_empty(), "{report:?}");
    assert!(report.summary_md.contains("Incubadora"));
    let monthly =
        monthly_report_md(h.handle.state(), "cat-inc", today.year(), today.month()).unwrap();
    assert!(monthly.contains("Incubadora"));

    // Manual entry and split.
    let manual = h
        .handle
        .add_manual_entry(
            t0 - ChronoDuration::hours(2),
            t0 - ChronoDuration::hours(1),
            "cat-ifro",
            Some("Reunião presencial".into()),
        )
        .unwrap();
    assert!(manual.is_manual);
    let new_id = h
        .handle
        .split_block(&manual.id, t0 - ChronoDuration::minutes(90))
        .unwrap();
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
    let t0 = fixed_now() - ChronoDuration::minutes(10);
    for i in 0..4 {
        h.tx.send(sample(
            t0 + ChronoDuration::seconds(i * 5),
            "1Password",
            "com.1password.1password",
            "Vault",
            None,
        ))
        .await
        .unwrap();
    }
    settle().await;
    h.handle
        .set_private_mode(PrivateModeDuration::Minutes30)
        .unwrap();
    assert_eq!(h.handle.tracker_state(), TrackerState::Private);
    for i in 5..9 {
        h.tx.send(sample(
            t0 + ChronoDuration::seconds(i * 5),
            "Google Chrome",
            "com.google.Chrome",
            "Banco - conta",
            Some("https://bank.example/secret"),
        ))
        .await
        .unwrap();
    }
    h.tx.send(sample(
        t0 + ChronoDuration::seconds(60),
        "Slack",
        "com.tinyspeck.slackmacgap",
        "general",
        None,
    ))
    .await
    .unwrap();
    settle().await;
    let today = fixed_now().with_timezone(&Local).date_naive();
    let blocks = timeline(h.handle.state(), today).unwrap();
    let private: Vec<_> = blocks
        .iter()
        .filter(|b| b.category_id.as_deref() == Some(system_categories::PRIVATE))
        .collect();
    assert!(private.len() >= 2, "{blocks:#?}");
    assert!(private
        .iter()
        .all(|b| b.url.is_none() && b.title == ubiqx_core::segmenter::PRIVATE_TITLE));
    assert!(
        private.iter().all(|b| b.ai_payload.is_none()),
        "private blocks never go to the AI"
    );
    h.handle.set_private_mode(PrivateModeDuration::Off).unwrap();
    assert_ne!(h.handle.tracker_state(), TrackerState::Paused);
    h.handle.shutdown();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn nudges_fire_on_long_distraction() {
    let h = harness().await;
    let state = h.handle.state().clone();
    let now = fixed_now();
    // Report/attention nudges are exempt from the daily cap and must not starve policy nudges.
    for i in 0..h.handle.settings().nudges.max_per_day {
        NudgeRepo::insert(
            h.store.as_ref(),
            &Nudge {
                id: format!("rr-{i}"),
                at: now - ChronoDuration::minutes(5),
                kind: NudgeKind::ReportReady,
                title: "Relatório pronto".into(),
                message: String::new(),
                seen: false,
            },
        )
        .unwrap();
    }
    // 40 minutes of YouTube classified as distraction, ending now.
    let mut b = ActivityBlock::new_manual(
        now - ChronoDuration::minutes(40),
        now - ChronoDuration::seconds(30),
        system_categories::DISTRACTION.into(),
        None,
    );
    b.is_manual = false;
    b.app_id = "com.google.Chrome".into();
    b.app_name = "Google Chrome".into();
    b.source = Some(ClassificationSource::Rule);
    BlockRepo::insert(h.store.as_ref(), &b).unwrap();
    let mut s = h.handle.settings();
    s.quiet_hours.enabled = false;
    h.handle.update_settings(s).unwrap();
    let nudges = ubiqx_engine::nudges::run_once(&state).unwrap();
    assert!(
        nudges.iter().any(|n| n.kind == NudgeKind::Unproductive),
        "{nudges:?}"
    );
    // Second evaluation within the cooldown yields nothing.
    assert!(ubiqx_engine::nudges::run_once(&state).unwrap().is_empty());
    h.handle.shutdown();
}

// ---------------------------------------------------------------------------------------------
// Regression tests for the review findings
// ---------------------------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn screen_recording_prompt_fires_even_when_window_id_is_known() {
    let h = harness().await;
    h.platform.set_permissions(PermissionStatus {
        screen_recording: PermissionState::Denied,
        automation: PermissionState::Granted,
        accessibility: PermissionState::Granted,
    });
    let t0 = fixed_now() - ChronoDuration::minutes(5);
    for i in 0..3 {
        let mut s = sample(
            t0 + ChronoDuration::seconds(i * 5),
            "Xcode",
            "com.apple.dt.Xcode",
            "",
            None,
        );
        s.window_id = Some(77); // macOS reports the id without the permission
        h.tx.send(s).await.unwrap();
    }
    settle().await;
    let events = h.sink.0.lock().clone();
    let prompts = events
        .iter()
        .filter(|e| {
            matches!(e, EngineEvent::PermissionRequired { permission } if permission == "screen_recording")
        })
        .count();
    assert_eq!(prompts, 1, "{events:#?}");
    h.handle.shutdown();
}

fn record_usage(h: &Harness, cost_usd: f64) {
    UsageRepo::record(
        h.store.as_ref(),
        &AiUsage {
            at: h.clock.now(),
            kind: AiUsageKind::Classify,
            model: "fake".into(),
            input_tokens: 1000,
            output_tokens: 100,
            cache_read_tokens: 0,
            cache_write_tokens: 0,
            cost_usd,
        },
    )
    .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn budget_pause_lifts_on_month_rollover() {
    let h = harness().await;
    record_usage(&h, 6.0); // over the default US$ 5 budget
    BlockRepo::insert(h.store.as_ref(), &pending_block("p1", 5)).unwrap();

    let r = h.handle.classify_now().await.unwrap();
    assert!(r.skipped_remote);
    assert!(
        matches!(h.handle.ai_health(), AiHealth::Paused { ref reason } if reason.contains("Orçamento")),
        "{:?}",
        h.handle.ai_health()
    );
    assert_eq!(get_block(&h, "p1").category_id, None);

    // Next month: the ledger of the new month is empty, so the pause lifts by itself.
    h.clock.advance(ChronoDuration::days(40));
    let r = h.handle.classify_now().await.unwrap();
    assert!(!r.skipped_remote, "{r:?}");
    assert_eq!(h.handle.ai_health(), AiHealth::Ok);
    assert_eq!(get_block(&h, "p1").category_id.as_deref(), Some("cat-ifro"));
    h.handle.shutdown();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn budget_pause_lifts_when_budget_is_raised() {
    let h = harness().await;
    record_usage(&h, 6.0);
    BlockRepo::insert(h.store.as_ref(), &pending_block("p1", 5)).unwrap();
    assert!(h.handle.classify_now().await.unwrap().skipped_remote);
    assert!(matches!(h.handle.ai_health(), AiHealth::Paused { .. }));

    // Unrelated settings changes leave the pause alone…
    let mut s = h.handle.settings();
    s.nudges.snoozed_until = Some(h.clock.now() + ChronoDuration::minutes(30));
    h.handle.update_settings(s).unwrap();
    assert!(matches!(h.handle.ai_health(), AiHealth::Paused { .. }));

    // …raising the budget resumes immediately, before any classification pass.
    let mut s = h.handle.settings();
    s.ai_monthly_budget_usd = 20.0;
    h.handle.update_settings(s).unwrap();
    assert_eq!(h.handle.ai_health(), AiHealth::Ok);
    let r = h.handle.classify_now().await.unwrap();
    assert_eq!(r.remote, 1, "{r:?}");
    assert_eq!(get_block(&h, "p1").category_id.as_deref(), Some("cat-ifro"));
    h.handle.shutdown();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn network_failures_degrade_health_but_never_charge_blocks() {
    let h = harness_with(HarnessOptions {
        ai: failing_ai(|| CoreError::Ai("network error: connection reset".into())),
        ..Default::default()
    })
    .await;
    BlockRepo::insert(h.store.as_ref(), &pending_block("p1", 5)).unwrap();
    let r = h.handle.classify_now().await.unwrap();
    assert!(r.skipped_remote);
    assert!(matches!(h.handle.ai_health(), AiHealth::Degraded { .. }));
    let b = get_block(&h, "p1");
    assert_eq!(b.classify_attempts, 0);
    assert!(!b.needs_review);
    assert_eq!(b.next_attempt_at, None);
    h.handle.shutdown();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rejected_account_pauses_ai_until_a_new_key_is_saved() {
    let h = harness_with(HarnessOptions {
        ai: failing_ai(|| CoreError::AiRejected("créditos esgotados".into())),
        ..Default::default()
    })
    .await;
    BlockRepo::insert(h.store.as_ref(), &pending_block("p1", 5)).unwrap();
    assert!(h.handle.classify_now().await.unwrap().skipped_remote);
    match h.handle.ai_health() {
        AiHealth::Paused { reason } => assert_eq!(reason, "IA indisponível: créditos esgotados"),
        other => panic!("{other:?}"),
    }
    let b = get_block(&h, "p1");
    assert_eq!(b.classify_attempts, 0, "not the blocks' fault");
    assert!(!b.needs_review);
    assert_eq!(nudges_of(&h, NudgeKind::Attention), 1);

    // Still paused on the next pass, and no second nudge.
    assert!(h.handle.classify_now().await.unwrap().skipped_remote);
    assert_eq!(nudges_of(&h, NudgeKind::Attention), 1);

    // Saving a key re-arms the AI path.
    h.handle
        .set_api_key(AiProvider::Anthropic, Some("sk-ant-new"))
        .unwrap();
    assert_eq!(h.handle.ai_health(), AiHealth::Ok);
    h.handle.shutdown();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn refusal_sends_the_batch_to_review_without_touching_health() {
    let h = harness_with(HarnessOptions {
        ai: failing_ai(|| CoreError::AiRefused),
        ..Default::default()
    })
    .await;
    BlockRepo::insert(h.store.as_ref(), &pending_block("p1", 5)).unwrap();
    let r = h.handle.classify_now().await.unwrap();
    assert_eq!(r.needs_review, 1, "{r:?}");
    assert!(!r.skipped_remote, "a refusal does not abort the pass");
    assert_eq!(h.handle.ai_health(), AiHealth::Ok);
    let b = get_block(&h, "p1");
    assert!(b.needs_review);
    assert_eq!(b.classify_attempts, ubiqx_engine::classify::MAX_ATTEMPTS);
    assert!(
        BlockRepo::list_pending_remote(h.store.as_ref(), h.clock.now(), 10)
            .unwrap()
            .is_empty(),
        "never re-sent"
    );
    assert_eq!(nudges_of(&h, NudgeKind::Attention), 0);
    h.handle.shutdown();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn scheduled_run_never_overwrites_an_edited_report() {
    let h = harness().await;
    let state = h.handle.state().clone();
    let now = h.clock.now();
    // Reports of both categories became due one hour ago; the last check was three hours ago.
    let due_at = (now - ChronoDuration::hours(1)).with_timezone(&Local);
    let date = due_at.date_naive();
    let mut s = h.handle.settings();
    s.report_default_time = due_at.time();
    h.handle.update_settings(s).unwrap();
    KvRepo::set(
        h.store.as_ref(),
        "last_report_check",
        &(now - ChronoDuration::hours(3)).to_rfc3339(),
    )
    .unwrap();

    let mut report = h.handle.generate_report(date, "cat-inc").await.unwrap();
    report.summary_md = "Texto revisado à mão".into();
    report.edited = true;
    ReportRepo::upsert(h.store.as_ref(), &report).unwrap();

    let generated = ubiqx_engine::reports::run_due(&state).await.unwrap();
    assert_eq!(generated, 1, "only the untouched category is regenerated");
    let stored = ReportRepo::get(h.store.as_ref(), date, "cat-inc")
        .unwrap()
        .unwrap();
    assert_eq!(stored.summary_md, "Texto revisado à mão");
    assert!(stored.edited);
    assert!(ReportRepo::get(h.store.as_ref(), date, "cat-ifro")
        .unwrap()
        .is_some());

    // An explicit user regeneration is still allowed to replace the text.
    let again = h.handle.generate_report(date, "cat-inc").await.unwrap();
    assert!(!again.edited);
    h.handle.shutdown();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn report_payload_is_recorded_for_locally_classified_blocks() {
    let h = harness().await;
    let today = h.clock.now().with_timezone(&Local).date_naive();
    RuleRepo::upsert(
        h.store.as_ref(),
        &Rule {
            id: "r-sei".into(),
            category_id: "cat-ifro".into(),
            matcher: RuleMatcher::Domain,
            pattern: "sei.ifro.edu.br".into(),
            priority: 10,
            origin: RuleOrigin::User,
            enabled: true,
            created_at: h.clock.now(),
            hit_count: 0,
            miss_count: 0,
            last_contradicted_at: None,
        },
    )
    .unwrap();
    BlockRepo::insert(h.store.as_ref(), &pending_block("p1", 30)).unwrap();

    let r = h.handle.classify_now().await.unwrap();
    assert_eq!(r.local, 1, "{r:?}");
    let b = get_block(&h, "p1");
    assert_eq!(b.source, Some(ClassificationSource::Rule));
    assert!(b.ai_payload.is_none() && b.ai_sent_at.is_none());
    assert!(ai_sent_blocks(h.handle.state(), today).unwrap().is_empty());

    h.handle.generate_report(today, "cat-ifro").await.unwrap();
    let b = get_block(&h, "p1");
    assert!(b.ai_sent_at.is_some(), "the report sent this block's text");
    let payload = b.ai_payload.as_deref().unwrap();
    assert!(payload.starts_with("[relatório]"), "{payload}");
    assert!(payload.contains("SEI"), "{payload}");
    assert_eq!(
        ai_sent_blocks(h.handle.state(), today)
            .unwrap()
            .iter()
            .map(|b| b.id.as_str())
            .collect::<Vec<_>>(),
        ["p1"]
    );
    h.handle.shutdown();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn generated_text_follows_the_settings_language() {
    let h = harness().await;
    let state = h.handle.state().clone();
    let today = h.clock.now().with_timezone(&Local).date_naive();

    // Any spelling of English is stored as the canonical tag.
    let mut s = h.handle.settings();
    assert_eq!(s.language, "pt-BR");
    s.language = "en_US".into();
    h.handle.update_settings(s).unwrap();
    assert_eq!(h.handle.settings().language, "en");
    assert_eq!(h.handle.settings().ui_language(), UiLanguage::En);

    // Report and payload label in English.
    BlockRepo::insert(h.store.as_ref(), &pending_block("p1", 30)).unwrap();
    RuleRepo::upsert(
        h.store.as_ref(),
        &Rule {
            id: "r-sei".into(),
            category_id: "cat-ifro".into(),
            matcher: RuleMatcher::Domain,
            pattern: "sei.ifro.edu.br".into(),
            priority: 10,
            origin: RuleOrigin::User,
            enabled: true,
            created_at: h.clock.now(),
            hit_count: 0,
            miss_count: 0,
            last_contradicted_at: None,
        },
    )
    .unwrap();
    h.handle.classify_now().await.unwrap();
    let report = h.handle.generate_report(today, "cat-ifro").await.unwrap();
    assert!(
        report.summary_md.starts_with("# Report — IFRO — "),
        "{}",
        report.summary_md
    );
    assert!(report.summary_md.contains("## Activities"));
    let payload = get_block(&h, "p1").ai_payload.unwrap();
    assert!(payload.starts_with("[report]"), "{payload}");

    // The "report ready" notification of the scheduler.
    let due_at = (h.clock.now() - ChronoDuration::hours(1)).with_timezone(&Local);
    let mut s = h.handle.settings();
    s.report_default_time = due_at.time();
    h.handle.update_settings(s).unwrap();
    KvRepo::set(
        h.store.as_ref(),
        "last_report_check",
        &(h.clock.now() - ChronoDuration::hours(3)).to_rfc3339(),
    )
    .unwrap();
    ubiqx_engine::reports::run_due(&state).await.unwrap();
    let ready: Vec<Nudge> = NudgeRepo::list_recent(h.store.as_ref(), 10)
        .unwrap()
        .into_iter()
        .filter(|n| n.kind == NudgeKind::ReportReady)
        .collect();
    assert!(!ready.is_empty());
    assert!(
        ready.iter().all(|n| n.title.ends_with(" is ready")),
        "{ready:?}"
    );

    // Back to Portuguese: the same paths speak Portuguese again.
    let mut s = h.handle.settings();
    s.language = "pt".into();
    h.handle.update_settings(s).unwrap();
    assert_eq!(h.handle.settings().language, "pt-BR");
    let report = h.handle.generate_report(today, "cat-ifro").await.unwrap();
    assert!(
        report.summary_md.starts_with("# Relatório — IFRO — "),
        "{}",
        report.summary_md
    );
    h.handle.shutdown();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tracker_keeps_user_reclassification_and_screenshot_of_the_open_block() {
    let h = harness().await;
    let mut s = h.handle.settings();
    s.screenshot_interval_secs = 60;
    h.handle.update_settings(s).unwrap();
    let t0 = fixed_now() - ChronoDuration::minutes(30);
    let send = |i: i64| {
        h.tx.send(sample(
            t0 + ChronoDuration::seconds(i * 5),
            "Xcode",
            "com.apple.dt.Xcode",
            "App.swift",
            None,
        ))
    };
    for i in 0..3 {
        send(i).await.unwrap();
    }
    settle().await;
    let open = BlockRepo::open_block(h.store.as_ref()).unwrap().unwrap();
    assert!(
        open.screenshot_id.is_some(),
        "captured once the block is old enough"
    );

    // The user reclassifies the running block from the timeline.
    h.handle
        .reclassify(&open.id, "cat-inc", None, ReclassifyScope::Block)
        .unwrap();
    for i in 3..6 {
        send(i).await.unwrap();
    }
    settle().await;
    let b = get_block(&h, &open.id);
    assert!(b.is_open);
    assert_eq!(b.sample_count, 6, "the segmenter's columns still flow");
    assert_eq!(b.category_id.as_deref(), Some("cat-inc"), "not reverted");
    assert_eq!(b.source, Some(ClassificationSource::User));
    assert_eq!(b.screenshot_id, open.screenshot_id, "not reverted");

    // Pausing persists the close through the same narrow update.
    h.handle.pause();
    let b = get_block(&h, &open.id);
    assert!(!b.is_open);
    assert_eq!(b.category_id.as_deref(), Some("cat-inc"));
    h.handle.shutdown();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn disabling_tracking_closes_the_open_block_and_drops_late_samples() {
    let h = harness().await;
    let t0 = fixed_now() - ChronoDuration::minutes(30);
    let mk = |i: i64| {
        sample(
            t0 + ChronoDuration::seconds(i * 5),
            "Xcode",
            "com.apple.dt.Xcode",
            "App.swift",
            None,
        )
    };
    for i in 0..3 {
        h.tx.send(mk(i)).await.unwrap();
    }
    settle().await;
    let open = BlockRepo::open_block(h.store.as_ref()).unwrap().unwrap();

    let mut s = h.handle.settings();
    s.tracking_enabled = false;
    h.handle.update_settings(s).unwrap();
    assert_eq!(h.handle.tracker_state(), TrackerState::Paused);
    assert!(BlockRepo::open_block(h.store.as_ref()).unwrap().is_none());
    assert!(!get_block(&h, &open.id).is_open);

    // A sample that was already queued must not reopen a block.
    h.tx.send(mk(3)).await.unwrap();
    settle().await;
    assert!(BlockRepo::open_block(h.store.as_ref()).unwrap().is_none());
    let today = fixed_now().with_timezone(&Local).date_naive();
    assert_eq!(timeline(h.handle.state(), today).unwrap().len(), 1);
    h.handle.shutdown();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn day_scope_does_not_spread_a_domainless_browser_block() {
    let h = harness().await;
    let mut no_domain = pending_block("chrome-nodomain", 40);
    no_domain.url = None;
    no_domain.domain = None;
    no_domain.title = "YouTube".into();
    no_domain.title_key = "youtube".into();
    let mut github = pending_block("chrome-github", 20);
    github.url = Some("https://github.com/x".into());
    github.domain = Some("github.com".into());
    let mut code_a = pending_block("code-a", 60);
    code_a.app_id = "com.microsoft.VSCode".into();
    code_a.app_name = "Visual Studio Code".into();
    code_a.url = None;
    code_a.domain = None;
    let mut code_b = code_a.clone();
    code_b.id = "code-b".into();
    code_b.started_at -= ChronoDuration::hours(1);
    code_b.ended_at -= ChronoDuration::hours(1);
    for b in [&no_domain, &github, &code_a, &code_b] {
        BlockRepo::insert(h.store.as_ref(), b).unwrap();
    }

    let out = h
        .handle
        .reclassify("chrome-nodomain", "cat-inc", None, ReclassifyScope::Day)
        .unwrap();
    assert_eq!(
        out.backfilled, 0,
        "no trustworthy key: nothing else is touched"
    );
    assert_eq!(get_block(&h, "chrome-github").category_id, None);
    assert_eq!(
        get_block(&h, "chrome-nodomain").category_id.as_deref(),
        Some("cat-inc")
    );

    // Native apps keep the documented "same app" day backfill.
    let out = h
        .handle
        .reclassify("code-a", "cat-inc", None, ReclassifyScope::Day)
        .unwrap();
    assert_eq!(out.backfilled, 1);
    assert_eq!(
        get_block(&h, "code-b").category_id.as_deref(),
        Some("cat-inc")
    );
    h.handle.shutdown();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delete_all_data_wipes_derived_data_but_keeps_settings_and_key() {
    let h = harness().await;
    let today = h.clock.now().with_timezone(&Local).date_naive();
    h.handle
        .set_api_key(AiProvider::Anthropic, Some("sk-ant-keep"))
        .unwrap();
    let mut s = h.handle.settings();
    s.user_profile = Some("Servidor do IFRO".into());
    h.handle.update_settings(s).unwrap();
    BlockRepo::insert(h.store.as_ref(), &pending_block("p1", 30)).unwrap();
    h.handle
        .reclassify("p1", "cat-inc", Some("nota".into()), ReclassifyScope::Block)
        .unwrap();
    h.handle.generate_report(today, "cat-inc").await.unwrap();
    h.handle.advice(false).await.unwrap();
    let export = h.handle.export_json().unwrap();
    assert!(export.exists());
    assert_eq!(CorrectionRepo::count(h.store.as_ref()).unwrap(), 1);
    assert!(KvRepo::get(h.store.as_ref(), &format!("advice_{today}"))
        .unwrap()
        .is_some());

    h.handle.delete_all_data().unwrap();

    assert!(CorrectionRepo::list_recent(h.store.as_ref(), 10)
        .unwrap()
        .is_empty());
    assert!(timeline(h.handle.state(), today).unwrap().is_empty());
    assert!(ReportRepo::list_for_date(h.store.as_ref(), today)
        .unwrap()
        .is_empty());
    assert!(NudgeRepo::list_recent(h.store.as_ref(), 10)
        .unwrap()
        .is_empty());
    assert_eq!(
        KvRepo::get(h.store.as_ref(), &format!("advice_{today}")).unwrap(),
        None
    );
    assert!(!export.exists(), "exports folder removed");
    assert!(!h.tmp.path().join("exports").exists());
    assert_eq!(
        h.handle
            .api_key_hint(AiProvider::Anthropic)
            .unwrap()
            .as_deref(),
        Some("…keep"),
        "the key is kept, as the dialog promises"
    );
    assert_eq!(
        h.handle.settings().user_profile.as_deref(),
        Some("Servidor do IFRO")
    );
    assert_eq!(
        CategoryRepo::list(h.store.as_ref(), false).unwrap().len(),
        { CategoryRepo::list(h.store.as_ref(), false).unwrap().len() }
    );
    assert!(CategoryRepo::get(h.store.as_ref(), "cat-inc")
        .unwrap()
        .is_some());
    h.handle.shutdown();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn nothing_is_recorded_or_sent_before_onboarding() {
    let h = harness_with(HarnessOptions {
        onboarding_done: false,
        ..Default::default()
    })
    .await;
    assert_eq!(h.handle.tracker_state(), TrackerState::Paused);
    let t0 = fixed_now() - ChronoDuration::minutes(30);
    for i in 0..4 {
        h.tx.send(sample(
            t0 + ChronoDuration::seconds(i * 5),
            "Google Chrome",
            "com.google.Chrome",
            "SEI - Processo 1",
            Some("https://sei.ifro.edu.br/x"),
        ))
        .await
        .unwrap();
    }
    settle().await;
    let today = fixed_now().with_timezone(&Local).date_naive();
    assert!(timeline(h.handle.state(), today).unwrap().is_empty());
    assert!(BlockRepo::open_block(h.store.as_ref()).unwrap().is_none());

    // A pending block (e.g. from a previous install) is not sent either.
    BlockRepo::insert(h.store.as_ref(), &pending_block("p1", 5)).unwrap();
    let r = h.handle.classify_now().await.unwrap();
    assert!(r.skipped_remote, "{r:?}");
    assert_eq!(get_block(&h, "p1").category_id, None);
    assert!(get_block(&h, "p1").ai_sent_at.is_none());

    // Finishing onboarding arms both paths.
    let mut s = h.handle.settings();
    s.onboarding_done = true;
    h.handle.update_settings(s).unwrap();
    assert_eq!(h.handle.tracker_state(), TrackerState::Running);
    let r = h.handle.classify_now().await.unwrap();
    assert_eq!(r.remote, 1, "{r:?}");
    h.handle.shutdown();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn shutdown_persists_the_open_block_before_returning() {
    let h = harness().await;
    let t0 = fixed_now() - ChronoDuration::minutes(30);
    for i in 0..3 {
        h.tx.send(sample(
            t0 + ChronoDuration::seconds(i * 5),
            "Xcode",
            "com.apple.dt.Xcode",
            "App.swift",
            None,
        ))
        .await
        .unwrap();
    }
    settle().await;
    let open = BlockRepo::open_block(h.store.as_ref()).unwrap().unwrap();

    h.handle.shutdown();
    // Synchronously visible: no sleep, no polling.
    assert!(BlockRepo::open_block(h.store.as_ref()).unwrap().is_none());
    let b = get_block(&h, &open.id);
    assert!(!b.is_open);
    assert!(b.ended_at <= fixed_now());
    assert!(!h.handle.is_running());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stale_open_block_from_an_abrupt_exit_is_closed_at_start() {
    let h = harness_with(HarnessOptions {
        seed: |store| {
            let mut b = pending_block("stale", 60);
            b.is_open = true;
            BlockRepo::insert(store, &b).unwrap();
        },
        ..Default::default()
    })
    .await;
    assert!(BlockRepo::open_block(h.store.as_ref()).unwrap().is_none());
    let b = get_block(&h, "stale");
    assert!(!b.is_open);
    assert_eq!(
        b.ended_at,
        fixed_now() - ChronoDuration::minutes(60),
        "not extended"
    );
    let today = fixed_now().with_timezone(&Local).date_naive();
    let d = dashboard(h.handle.state(), today).unwrap();
    assert!(d.open_block.is_none());
    h.handle.shutdown();
}

/// Fake AI that still requires a key, so health follows the secret store like in production.
fn keyed_fake_ai() -> AiPorts {
    AiPorts {
        requires_api_key: true,
        ..fake_ai()
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switching_provider_flips_health_by_stored_keys() {
    let h = harness_with(HarnessOptions {
        ai: keyed_fake_ai(),
        keys: vec![(AiProvider::OpenAi, "sk-openai-1234")],
        ..Default::default()
    })
    .await;
    // Anthropic is selected and has no key.
    assert_eq!(h.handle.ai_health(), AiHealth::NotConfigured);

    let mut s = h.handle.settings();
    s.ai_provider = AiProvider::OpenAi;
    h.handle.update_settings(s).unwrap();
    assert_eq!(h.handle.ai_health(), AiHealth::Ok, "OpenAI has a key");
    assert!(h
        .sink
        .0
        .lock()
        .iter()
        .any(|e| matches!(e, EngineEvent::AiHealth { health } if *health == AiHealth::Ok)));

    let mut s = h.handle.settings();
    s.ai_provider = AiProvider::Xai;
    h.handle.update_settings(s).unwrap();
    assert_eq!(
        h.handle.ai_health(),
        AiHealth::NotConfigured,
        "xAI has no key"
    );

    // A pause caused by the previous vendor ends with the switch.
    let mut s = h.handle.settings();
    s.ai_provider = AiProvider::OpenAi;
    h.handle.update_settings(s).unwrap();
    h.handle.state().set_ai_health(AiHealth::Paused {
        reason: "IA indisponível: créditos esgotados".into(),
    });
    let mut s = h.handle.settings();
    s.ai_provider = AiProvider::Anthropic;
    h.handle.update_settings(s).unwrap();
    assert_eq!(h.handle.ai_health(), AiHealth::NotConfigured);

    // Saving other settings does not recompute health.
    h.handle.state().set_ai_health(AiHealth::Ok);
    let mut s = h.handle.settings();
    s.user_profile = Some("x".into());
    h.handle.update_settings(s).unwrap();
    assert_eq!(h.handle.ai_health(), AiHealth::Ok);
    h.handle.shutdown();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn key_of_a_non_selected_provider_never_changes_health() {
    let h = harness_with(HarnessOptions {
        ai: keyed_fake_ai(),
        ..Default::default()
    })
    .await;
    assert_eq!(h.handle.ai_health(), AiHealth::NotConfigured);

    h.handle
        .set_api_key(AiProvider::Xai, Some("xai-abcd1234"))
        .unwrap();
    assert_eq!(h.handle.ai_health(), AiHealth::NotConfigured);
    assert_eq!(
        h.handle.api_key_hint(AiProvider::Xai).unwrap().as_deref(),
        Some("…1234")
    );
    assert_eq!(h.handle.api_key_hint(AiProvider::Anthropic).unwrap(), None);
    assert_eq!(
        h.handle.api_key_status().unwrap(),
        vec![
            (AiProvider::Anthropic, None),
            (AiProvider::OpenAi, None),
            (AiProvider::Xai, Some("…1234".into())),
        ]
    );

    // The selected provider's key arms and disarms the AI path.
    h.handle
        .set_api_key(AiProvider::Anthropic, Some("sk-ant-9999"))
        .unwrap();
    assert_eq!(h.handle.ai_health(), AiHealth::Ok);
    h.handle.set_api_key(AiProvider::Xai, None).unwrap();
    assert_eq!(h.handle.ai_health(), AiHealth::Ok, "xAI is not selected");
    assert_eq!(h.handle.api_key_hint(AiProvider::Xai).unwrap(), None);
    h.handle
        .set_api_key(AiProvider::Anthropic, Some("  "))
        .unwrap();
    assert_eq!(h.handle.ai_health(), AiHealth::NotConfigured);
    h.handle.shutdown();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn models_are_reconciled_when_the_provider_changes() {
    let h = harness().await;
    let mut s = h.handle.settings();
    assert_eq!(s.models, AiModels::for_provider(AiProvider::Anthropic));
    s.ai_provider = AiProvider::Xai;
    s.models.report = "my-custom-model".into();
    h.handle.update_settings(s).unwrap();

    let s = h.handle.settings();
    let xai = AiModels::for_provider(AiProvider::Xai);
    assert_eq!(s.models.classify, xai.classify, "claude-* id replaced");
    assert_eq!(s.models.vision, xai.vision);
    assert_eq!(s.models.report, "my-custom-model", "unknown ids are kept");
    // The reconciled models are what got persisted too.
    assert_eq!(
        SettingsRepo::load(h.store.as_ref()).unwrap().models,
        s.models
    );

    let mut s = h.handle.settings();
    s.ai_provider = AiProvider::OpenAi;
    h.handle.update_settings(s).unwrap();
    let s = h.handle.settings();
    assert_eq!(s.models.classify, "gpt-5-mini");
    assert_eq!(s.models.report, "my-custom-model");
    h.handle.shutdown();
}
