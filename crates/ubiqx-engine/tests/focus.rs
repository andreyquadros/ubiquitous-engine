//! The focus guard end to end: blocked targets are held (with the per-key cooldown), sites
//! match parent domains, distraction rules only count during a session, exempt apps are left
//! alone, sessions hide windows and run Shortcuts, their end stores the praise nudge, and
//! the "a lot of windows" prompt fires at ten switches and respects its cooldown.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Duration as ChronoDuration, TimeZone, Utc};
use parking_lot::Mutex;
use ubiqx_core::clock::FixedClock;
use ubiqx_core::ports::*;
use ubiqx_core::*;
use ubiqx_engine::engine::LoopConfig;
use ubiqx_engine::*;
use ubiqx_platform::mock::{
    EnforcerCall, MockAppCatalog, MockEnforcer, StaticUpdateFeed, SyntheticCapturer,
};
use ubiqx_storage::{Db, SqliteStore};

// ---------------------------------------------------------------------------------------------
// Test doubles
// ---------------------------------------------------------------------------------------------

/// A foreground the test sets by hand, with the URL a browser would report.
#[derive(Default)]
struct Foreground {
    window: Mutex<Option<ForegroundWindow>>,
    url: Mutex<Option<String>>,
}

impl Foreground {
    fn set(&self, app_name: &str, app_id: &str, title: &str, url: Option<&str>) {
        *self.window.lock() = Some(ForegroundWindow {
            app_name: app_name.into(),
            app_id: app_id.into(),
            window_title: title.into(),
            pid: Some(1),
            window_id: Some(1),
            bounds: None,
        });
        *self.url.lock() = url.map(String::from);
    }
}

impl ActivitySource for Foreground {
    fn foreground(&self) -> CoreResult<Option<ForegroundWindow>> {
        Ok(self.window.lock().clone())
    }
}

impl BrowserUrlResolver for Foreground {
    fn supports(&self, app_id: &str) -> bool {
        ubiqx_platform::browser::script_for(app_id).is_some()
    }

    fn resolve(&self, _window: &ForegroundWindow) -> CoreResult<Option<String>> {
        Ok(self.url.lock().clone())
    }
}

impl IdleDetector for Foreground {
    fn idle_secs(&self) -> CoreResult<Option<f64>> {
        Ok(Some(0.0))
    }
}

impl PermissionChecker for Foreground {
    fn status(&self) -> PermissionStatus {
        PermissionStatus {
            screen_recording: PermissionState::Granted,
            automation: PermissionState::Granted,
            accessibility: PermissionState::Granted,
        }
    }

    fn request(&self, _kind: PermissionKind) -> CoreResult<()> {
        Ok(())
    }
}

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

/// Records notifications (title, body).
#[derive(Default)]
struct RecordingNotifier(Mutex<Vec<(String, String)>>);

impl Notifier for RecordingNotifier {
    fn notify(&self, title: &str, body: &str) -> CoreResult<()> {
        self.0.lock().push((title.into(), body.into()));
        Ok(())
    }
}

/// Records the interventions it was asked to show; `shown` decides the answer.
struct RecordingPresenter {
    shown: std::sync::atomic::AtomicBool,
    seen: Mutex<Vec<Intervention>>,
}

impl RecordingPresenter {
    fn new(shown: bool) -> Self {
        Self {
            shown: std::sync::atomic::AtomicBool::new(shown),
            seen: Mutex::new(vec![]),
        }
    }
}

impl InterventionPresenter for RecordingPresenter {
    fn show(&self, intervention: &Intervention) -> CoreResult<bool> {
        self.seen.lock().push(intervention.clone());
        Ok(self.shown.load(std::sync::atomic::Ordering::SeqCst))
    }
}

struct CollectSink(Mutex<Vec<EngineEvent>>);

impl EventSink for CollectSink {
    fn emit(&self, event: EngineEvent) {
        self.0.lock().push(event);
    }
}

// ---------------------------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------------------------

struct Harness {
    handle: EngineHandle,
    store: Arc<SqliteStore>,
    foreground: Arc<Foreground>,
    enforcer: Arc<MockEnforcer>,
    presenter: Arc<RecordingPresenter>,
    notifier: Arc<RecordingNotifier>,
    sink: Arc<CollectSink>,
    clock: FixedClock,
    _tmp: tempfile::TempDir,
}

impl Harness {
    fn state(&self) -> &Arc<EngineState> {
        self.handle.state()
    }

    fn guard(&self) -> Option<Intervention> {
        ubiqx_engine::focus::guard_once(self.state()).unwrap()
    }

    fn events(&self) -> Vec<EngineEvent> {
        self.sink.0.lock().clone()
    }

    fn settings_mut(&self, f: impl FnOnce(&mut Settings)) {
        let mut s = self.handle.settings();
        f(&mut s);
        self.handle.update_settings(s).unwrap();
    }

    fn nudges(&self, kind: NudgeKind) -> Vec<Nudge> {
        NudgeRepo::list_recent(self.store.as_ref(), 50)
            .unwrap()
            .into_iter()
            .filter(|n| n.kind == kind)
            .collect()
    }
}

/// Mid-afternoon UTC, outside the default quiet hours in any plausible test timezone.
fn fixed_now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 9, 17, 15, 0, 0).unwrap()
}

async fn harness() -> Harness {
    harness_with(true).await
}

async fn harness_with(presenter_shows: bool) -> Harness {
    let tmp = tempfile::tempdir().unwrap();
    let store = Arc::new(SqliteStore::new(Db::open_in_memory().unwrap()));
    let foreground = Arc::new(Foreground::default());
    let enforcer = Arc::new(MockEnforcer::default());
    let presenter = Arc::new(RecordingPresenter::new(presenter_shows));
    let notifier = Arc::new(RecordingNotifier::default());
    let sink = Arc::new(CollectSink(Mutex::new(vec![])));
    let clock = FixedClock::new(fixed_now());
    let deps = EngineDeps {
        platform: PlatformPorts {
            activity: foreground.clone(),
            urls: foreground.clone(),
            idle: foreground.clone(),
            capturer: Arc::new(SyntheticCapturer),
            permissions: foreground.clone(),
            secrets: Arc::new(MemorySecrets::default()),
            notifier: notifier.clone(),
            update_feed: Arc::new(StaticUpdateFeed::new(None)),
            apps: Arc::new(MockAppCatalog),
            enforcer: enforcer.clone(),
            presenter: presenter.clone(),
        },
        repos: Repos::from_store(store.clone()),
        ai: AiPorts {
            requires_api_key: false,
            ..AiPorts::default()
        },
        sink: sink.clone(),
        clock: Arc::new(clock.clone()),
        data_dir: tmp.path().to_path_buf(),
        build: ubiqx_core::BuildInfo::dev(),
        update_feed_url: String::new(),
        license: LicenseDeps::default(),
    };
    let cfg = LoopConfig {
        classify_every: Duration::from_secs(3600),
        reports_every: Duration::from_secs(3600),
        nudges_every: Duration::from_secs(3600),
        retention_every: Duration::from_secs(3600),
        update_initial_delay: Duration::from_secs(3600),
        update_every: Duration::from_secs(3600),
        focus_every: Duration::from_secs(3600),
        license_initial_delay: Duration::from_secs(3600),
        license_every: Duration::from_secs(3600),
        without_sampler: true,
    };
    let (handle, _tx) = Engine::start_with(deps, cfg).unwrap();
    let mut s = handle.settings();
    s.onboarding_done = true;
    s.quiet_hours.enabled = false;
    handle.update_settings(s).unwrap();
    Harness {
        handle,
        store,
        foreground,
        enforcer,
        presenter,
        notifier,
        sink,
        clock,
        _tmp: tmp,
    }
}

const SLACK: &str = "com.tinyspeck.slackmacgap";
const CHROME: &str = "com.google.Chrome";

fn distraction_rule(pattern: &str, matcher: RuleMatcher) -> Rule {
    Rule {
        id: new_id(),
        category_id: system_categories::DISTRACTION.into(),
        matcher,
        pattern: pattern.into(),
        priority: 0,
        origin: RuleOrigin::User,
        enabled: true,
        created_at: fixed_now(),
        hit_count: 0,
        miss_count: 0,
        last_contradicted_at: None,
    }
}

// ---------------------------------------------------------------------------------------------
// Targets and enforcement
// ---------------------------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn blocked_app_is_quit_once_per_cooldown() {
    let h = harness().await;
    let target = h
        .handle
        .add_focus_target(FocusTargetKind::App, "Slack", SLACK)
        .unwrap();
    assert!(target.enabled);
    h.foreground.set("Slack", SLACK, "general", None);

    let i = h.guard().expect("intervention");
    assert_eq!(i.action, InterventionAction::AppQuit);
    assert_eq!(i.target_id.as_deref(), Some(target.id.as_str()));
    assert_eq!(i.kind, FocusTargetKind::App);
    assert_eq!(i.name, "Slack");
    assert_eq!(i.key, SLACK);
    assert_eq!(i.session_id, None);
    assert_eq!(i.message, ubiqx_core::focus::MESSAGES_PT[0]);
    assert_eq!(
        h.enforcer.calls(),
        vec![EnforcerCall::QuitApp(SLACK.into())]
    );

    // Recorded, counted on the target, announced, shown (so no notification).
    assert_eq!(h.handle.interventions(10).unwrap(), vec![i.clone()]);
    let t = FocusRepo::get_target(h.store.as_ref(), &target.id)
        .unwrap()
        .unwrap();
    assert_eq!(t.blocked_count, 1);
    assert_eq!(t.last_blocked_at, Some(fixed_now()));
    assert!(h.events().iter().any(
        |e| matches!(e, EngineEvent::Intervention { intervention } if intervention.id == i.id)
    ));
    assert_eq!(h.presenter.seen.lock().len(), 1);
    assert!(h.notifier.0.lock().is_empty());

    // Inside the cooldown nothing happens; after it the app is quit again with the next
    // message of the rotation.
    h.clock.advance(ChronoDuration::seconds(10));
    assert_eq!(h.guard(), None);
    assert_eq!(h.enforcer.calls().len(), 1);
    h.clock.advance(ChronoDuration::seconds(11));
    let again = h.guard().expect("second intervention");
    assert_eq!(again.message, ubiqx_core::focus::MESSAGES_PT[1]);
    assert_eq!(h.enforcer.calls().len(), 2);

    let status = h.handle.focus_status().unwrap();
    assert_eq!(status.targets_enabled, 1);
    assert_eq!(status.interventions_today, 2);
    assert!(status.guard_enabled);
    assert_eq!(status.session, None);

    // A disabled target no longer hits; the guard switch stops everything.
    h.handle
        .set_focus_target_enabled(&target.id, false)
        .unwrap();
    h.clock.advance(ChronoDuration::minutes(1));
    assert_eq!(h.guard(), None);
    h.handle.set_focus_target_enabled(&target.id, true).unwrap();
    h.settings_mut(|s| s.focus.guard_enabled = false);
    assert_eq!(h.guard(), None);
    h.handle.shutdown();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn blocked_site_matches_parent_domain_and_falls_back_to_blanking() {
    let h = harness().await;
    h.handle
        .add_focus_target(
            FocusTargetKind::Site,
            "YouTube",
            "https://www.YouTube.com/feed",
        )
        .unwrap();
    let targets = h.handle.focus_targets().unwrap();
    assert_eq!(targets[0].key, "youtube.com", "key normalised");

    h.foreground.set(
        "Google Chrome",
        CHROME,
        "Trailer - YouTube",
        Some("https://m.youtube.com/watch?v=x"),
    );
    let i = h.guard().expect("intervention");
    assert_eq!(i.action, InterventionAction::TabClosed);
    assert_eq!(i.kind, FocusTargetKind::Site);
    assert_eq!(i.name, "YouTube");
    assert_eq!(
        h.enforcer.calls(),
        vec![EnforcerCall::CloseActiveTab(CHROME.into())]
    );

    // Another site is left alone.
    h.clock.advance(ChronoDuration::minutes(1));
    h.foreground.set(
        "Google Chrome",
        CHROME,
        "SEI",
        Some("https://sei.ifro.edu.br/x"),
    );
    assert_eq!(h.guard(), None);

    // Closing fails → the tab is blanked; both fail → only notified.
    h.enforcer.clear();
    h.enforcer
        .refuse_close_tab
        .store(true, std::sync::atomic::Ordering::SeqCst);
    h.foreground.set(
        "Google Chrome",
        CHROME,
        "YouTube",
        Some("https://youtube.com/"),
    );
    let i = h.guard().expect("intervention");
    assert_eq!(i.action, InterventionAction::TabBlanked);
    assert_eq!(
        h.enforcer.calls(),
        vec![
            EnforcerCall::CloseActiveTab(CHROME.into()),
            EnforcerCall::BlankActiveTab(CHROME.into())
        ]
    );
    h.enforcer
        .refuse_blank_tab
        .store(true, std::sync::atomic::Ordering::SeqCst);
    h.clock.advance(ChronoDuration::minutes(1));
    let i = h.guard().expect("intervention");
    assert_eq!(i.action, InterventionAction::Notified);
    h.handle.shutdown();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn adding_an_existing_target_re_enables_it_and_inputs_are_validated() {
    let h = harness().await;
    let first = h
        .handle
        .add_focus_target(FocusTargetKind::Site, "YouTube", "youtube.com")
        .unwrap();
    h.handle.set_focus_target_enabled(&first.id, false).unwrap();
    let again = h
        .handle
        .add_focus_target(FocusTargetKind::Site, "", "www.youtube.com/")
        .unwrap();
    assert_eq!(again.id, first.id);
    assert!(again.enabled);
    assert_eq!(h.handle.focus_targets().unwrap().len(), 1);
    // A blank key is refused; a blank name falls back to the key.
    assert!(matches!(
        h.handle
            .add_focus_target(FocusTargetKind::Site, "x", "https://"),
        Err(CoreError::Invalid(_))
    ));
    let named = h
        .handle
        .add_focus_target(FocusTargetKind::App, "  ", "com.hnc.Discord")
        .unwrap();
    assert_eq!(named.name, "com.hnc.Discord");
    h.handle.remove_focus_target(&named.id).unwrap();
    assert_eq!(h.handle.focus_targets().unwrap().len(), 1);
    assert!(matches!(
        h.handle.set_focus_target_enabled(&named.id, true),
        Err(CoreError::NotFound(_))
    ));
    // The catalogue and known domains are served too.
    let apps = h.handle.installed_apps().unwrap();
    assert_eq!(apps.len(), 6);
    assert!(h.handle.known_domains(30).unwrap().is_empty());
    h.handle.shutdown();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn distraction_rules_only_count_during_a_session() {
    let h = harness().await;
    RuleRepo::upsert(
        h.store.as_ref(),
        &distraction_rule("reddit.com", RuleMatcher::Domain),
    )
    .unwrap();
    RuleRepo::upsert(
        h.store.as_ref(),
        &distraction_rule("com.hnc.Discord", RuleMatcher::App),
    )
    .unwrap();
    h.foreground.set(
        "Google Chrome",
        CHROME,
        "reddit",
        Some("https://www.reddit.com/r/x"),
    );
    assert_eq!(h.guard(), None, "no session, no target: nothing to hold");

    let session = h.handle.start_focus_session("relatório", 45).unwrap();
    let i = h.guard().expect("intervention from the rule");
    assert_eq!(i.target_id, None);
    assert_eq!(i.kind, FocusTargetKind::Site);
    assert_eq!(i.key, "reddit.com");
    assert_eq!(i.session_id.as_deref(), Some(session.id.as_str()));
    let status = h.handle.focus_status().unwrap();
    assert_eq!(status.session.as_ref().unwrap().interventions, 1);
    assert!(h.events().iter().any(|e| matches!(
        e,
        EngineEvent::FocusSession { session: Some(s) } if s.interventions == 1 && s.ended_at.is_none()
    )));

    // An app rule during the session, and the option that turns rules off.
    h.clock.advance(ChronoDuration::minutes(2));
    h.foreground
        .set("Discord", "com.hnc.Discord", "general", None);
    let i = h.guard().expect("intervention from the app rule");
    assert_eq!(i.kind, FocusTargetKind::App);
    assert_eq!(i.action, InterventionAction::AppQuit);
    h.settings_mut(|s| s.focus.block_distraction_in_session = false);
    h.clock.advance(ChronoDuration::minutes(2));
    assert_eq!(h.guard(), None);
    h.handle.shutdown();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn exempt_apps_are_never_enforced() {
    let h = harness().await;
    for (name, id) in [
        ("Finder", "com.apple.finder"),
        ("ubiqX", "ai.ubiqx.app"),
        ("Terminal", "com.apple.Terminal"),
    ] {
        h.handle
            .add_focus_target(FocusTargetKind::App, name, id)
            .unwrap();
    }
    h.foreground.set("Finder", "com.apple.finder", "", None);
    assert_eq!(h.guard(), None);
    h.foreground.set("ubiqX", "ai.ubiqx.app", "ubiqX", None);
    assert_eq!(h.guard(), None);
    h.foreground
        .set("System Settings", "com.apple.systempreferences", "", None);
    assert_eq!(h.guard(), None);

    // The Terminal is spared for the first minute of a session only.
    h.handle.start_focus_session("deploy", 30).unwrap();
    h.foreground
        .set("Terminal", "com.apple.Terminal", "zsh", None);
    assert_eq!(h.guard(), None);
    h.clock.advance(ChronoDuration::seconds(59));
    assert_eq!(h.guard(), None);
    h.clock.advance(ChronoDuration::seconds(2));
    assert!(h.guard().is_some());
    assert!(h.enforcer.calls().iter().any(|c| matches!(
        c,
        EnforcerCall::QuitApp(id) if id == "com.apple.Terminal"
    )));
    h.handle.shutdown();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn presenter_failure_falls_back_to_a_notification() {
    let h = harness_with(false).await;
    h.handle
        .add_focus_target(FocusTargetKind::App, "Slack", SLACK)
        .unwrap();
    h.foreground.set("Slack", SLACK, "general", None);
    let i = h.guard().expect("intervention");
    assert_eq!(
        h.notifier.0.lock().clone(),
        vec![("Slack".to_string(), i.message.clone())]
    );
    // The test button shows the sample and records nothing.
    h.handle.test_intervention().unwrap();
    assert_eq!(h.presenter.seen.lock().len(), 2);
    assert_eq!(h.presenter.seen.lock()[1].id, "test");
    assert_eq!(h.handle.interventions(10).unwrap().len(), 1);
    assert_eq!(h.notifier.0.lock().len(), 2);
    h.handle.shutdown();
}

// ---------------------------------------------------------------------------------------------
// Sessions
// ---------------------------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn session_start_hides_others_and_runs_the_shortcut() {
    let h = harness().await;
    h.settings_mut(|s| {
        s.focus.macos_focus_shortcut_on = Some(" Foco ".into());
        s.focus.macos_focus_shortcut_off = Some("Foco off".into());
    });
    h.foreground.set(
        "Visual Studio Code",
        "com.microsoft.VSCode",
        "main.rs",
        None,
    );

    assert!(matches!(
        h.handle.start_focus_session("   ", 45),
        Err(CoreError::Invalid(_))
    ));
    assert!(matches!(
        h.handle.start_focus_session("x", 4),
        Err(CoreError::Invalid(_))
    ));
    assert!(matches!(
        h.handle.start_focus_session("x", 241),
        Err(CoreError::Invalid(_))
    ));

    let s = h
        .handle
        .start_focus_session("  terminar o relatório ", 45)
        .unwrap();
    assert_eq!(s.task, "terminar o relatório");
    assert_eq!(s.started_at, fixed_now());
    assert_eq!(s.ends_at, fixed_now() + ChronoDuration::minutes(45));
    assert!(s.hid_windows);
    assert!(s.ran_shortcut);
    assert_eq!(
        h.enforcer.calls(),
        vec![
            EnforcerCall::HideOthers("com.microsoft.VSCode".into()),
            EnforcerCall::RunShortcut("Foco".into())
        ]
    );
    assert!(h.events().iter().any(|e| matches!(
        e,
        EngineEvent::FocusSession { session: Some(x) } if x.id == s.id && x.ended_at.is_none()
    )));
    let status = h.handle.focus_status().unwrap();
    assert_eq!(status.remaining_secs, Some(45 * 60));
    h.clock.advance(ChronoDuration::minutes(10));
    assert_eq!(
        h.handle.focus_status().unwrap().remaining_secs,
        Some(35 * 60)
    );

    // Starting another session ends the first (by hand, over five minutes: praised).
    let second = h.handle.start_focus_session("outra", 25).unwrap();
    assert_ne!(second.id, s.id);
    let sessions = FocusRepo::list_sessions(h.store.as_ref(), 10).unwrap();
    assert_eq!(sessions.len(), 2);
    assert_eq!(
        sessions[1].ended_at,
        Some(fixed_now() + ChronoDuration::minutes(10))
    );
    assert!(h
        .enforcer
        .calls()
        .contains(&EnforcerCall::RunShortcut("Foco off".into())));
    assert_eq!(h.nudges(NudgeKind::Praise).len(), 1);

    // Windows are not hidden when the option is off.
    h.settings_mut(|s| s.focus.hide_others_on_start = false);
    h.enforcer.clear();
    let third = h.handle.start_focus_session("terceira", 25).unwrap();
    assert!(!third.hid_windows);
    assert!(!h
        .enforcer
        .calls()
        .iter()
        .any(|c| matches!(c, EnforcerCall::HideOthers(_))));
    h.handle.shutdown();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn session_end_stores_the_praise_nudge() {
    let h = harness().await;
    let s = h.handle.start_focus_session("relatório", 45).unwrap();
    h.handle
        .add_focus_target(FocusTargetKind::App, "Slack", SLACK)
        .unwrap();
    h.foreground.set("Slack", SLACK, "general", None);
    h.guard().expect("held once");
    h.clock.advance(ChronoDuration::minutes(30));
    h.guard().expect("held twice");
    // Not a hit any more once the timer passes: the guard ends the session first.
    h.foreground.set(
        "Visual Studio Code",
        "com.microsoft.VSCode",
        "main.rs",
        None,
    );
    h.clock.advance(ChronoDuration::minutes(15));
    assert_eq!(h.guard(), None);

    let status = h.handle.focus_status().unwrap();
    assert_eq!(status.session, None);
    assert_eq!(status.remaining_secs, None);
    let stored = FocusRepo::list_sessions(h.store.as_ref(), 10).unwrap();
    assert_eq!(stored[0].id, s.id);
    assert_eq!(stored[0].ended_at, Some(s.ends_at));
    assert_eq!(stored[0].interventions, 2);
    let praise = h.nudges(NudgeKind::Praise);
    assert_eq!(praise.len(), 1);
    assert_eq!(
        praise[0].message,
        "Sessão de foco concluída: 45 min em \"relatório\", 2 distrações seguradas."
    );
    assert!(h.events().iter().any(|e| matches!(
        e,
        EngineEvent::FocusSession { session: Some(x) } if x.id == s.id && x.ended_at.is_some()
    )));
    assert!(h
        .notifier
        .0
        .lock()
        .iter()
        .any(|(t, _)| t == "Sessão de foco concluída"));

    // English texts, and no praise for a session stopped within five minutes.
    h.settings_mut(|s| s.language = "en".into());
    let short = h.handle.start_focus_session("report", 25).unwrap();
    h.clock.advance(ChronoDuration::minutes(3));
    let stopped = h.handle.stop_focus_session().unwrap().unwrap();
    assert_eq!(stopped.id, short.id);
    assert!(stopped.ended_at.is_some());
    assert_eq!(h.nudges(NudgeKind::Praise).len(), 1);
    assert_eq!(h.handle.stop_focus_session().unwrap(), None);
    let long = h.handle.start_focus_session("report", 25).unwrap();
    h.clock.advance(ChronoDuration::minutes(6));
    h.handle.stop_focus_session().unwrap().unwrap();
    let praise = h.nudges(NudgeKind::Praise);
    assert_eq!(praise.len(), 2);
    assert_eq!(
        praise[0].message,
        format!(
            "Focus session done: 6 min on \"{}\", 0 distractions held.",
            long.task
        )
    );
    h.handle.shutdown();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn session_end_praise_follows_the_nudge_toggles() {
    let h = harness().await;
    // The praise kind switched off: the timer still ends the session, but nothing is stored,
    // announced or notified.
    h.settings_mut(|s| s.nudges.praise = false);
    let s = h.handle.start_focus_session("relatório", 25).unwrap();
    h.clock.advance(ChronoDuration::minutes(25));
    assert_eq!(h.guard(), None);
    assert_eq!(h.handle.focus_status().unwrap().session, None);
    let stored = FocusRepo::list_sessions(h.store.as_ref(), 10).unwrap();
    assert_eq!(stored[0].id, s.id);
    assert_eq!(stored[0].ended_at, Some(s.ends_at));
    assert!(h.events().iter().any(|e| matches!(
        e,
        EngineEvent::FocusSession { session: Some(x) } if x.id == s.id && x.ended_at.is_some()
    )));
    assert!(h.nudges(NudgeKind::Praise).is_empty());
    assert!(h.notifier.0.lock().is_empty());
    assert!(!h
        .events()
        .iter()
        .any(|e| matches!(e, EngineEvent::Nudge { .. })));

    // All nudges off: the same for a session stopped by hand past the praise threshold.
    h.settings_mut(|s| {
        s.nudges.praise = true;
        s.nudges.enabled = false;
    });
    h.handle.start_focus_session("relatório", 25).unwrap();
    h.clock.advance(ChronoDuration::minutes(10));
    h.handle.stop_focus_session().unwrap().unwrap();
    assert!(h.nudges(NudgeKind::Praise).is_empty());
    assert!(h.notifier.0.lock().is_empty());

    // Both on again: praised and, at the planned end, notified.
    h.settings_mut(|s| s.nudges.enabled = true);
    h.handle.start_focus_session("relatório", 25).unwrap();
    h.clock.advance(ChronoDuration::minutes(25));
    assert_eq!(h.guard(), None);
    assert_eq!(h.nudges(NudgeKind::Praise).len(), 1);
    assert_eq!(h.notifier.0.lock().len(), 1);
    h.handle.shutdown();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_session_left_by_the_previous_run_is_restored() {
    let tmp = tempfile::tempdir().unwrap();
    let store = Arc::new(SqliteStore::new(Db::open_in_memory().unwrap()));
    let session = FocusSession {
        id: "left-behind".into(),
        task: "continuar".into(),
        started_at: fixed_now() - ChronoDuration::minutes(5),
        ends_at: fixed_now() + ChronoDuration::minutes(40),
        ended_at: None,
        interventions: 1,
        hid_windows: false,
        ran_shortcut: false,
    };
    FocusRepo::insert_session(store.as_ref(), &session).unwrap();
    let foreground = Arc::new(Foreground::default());
    let deps = EngineDeps {
        platform: PlatformPorts {
            activity: foreground.clone(),
            urls: foreground.clone(),
            idle: foreground.clone(),
            capturer: Arc::new(SyntheticCapturer),
            permissions: foreground.clone(),
            secrets: Arc::new(MemorySecrets::default()),
            notifier: Arc::new(RecordingNotifier::default()),
            update_feed: Arc::new(StaticUpdateFeed::new(None)),
            apps: Arc::new(MockAppCatalog),
            enforcer: Arc::new(MockEnforcer::default()),
            presenter: Arc::new(LogInterventionPresenter),
        },
        repos: Repos::from_store(store.clone()),
        ai: AiPorts::default(),
        sink: Arc::new(NullEventSink),
        clock: Arc::new(FixedClock::new(fixed_now())),
        data_dir: tmp.path().to_path_buf(),
        build: ubiqx_core::BuildInfo::dev(),
        update_feed_url: String::new(),
        license: LicenseDeps::default(),
    };
    let state = EngineState::new(deps, Settings::default(), None);
    let status = ubiqx_engine::focus::status(&state).unwrap();
    assert_eq!(status.session, Some(session));
    assert_eq!(status.remaining_secs, Some(40 * 60));
}

// ---------------------------------------------------------------------------------------------
// The "a lot of windows" prompt
// ---------------------------------------------------------------------------------------------

fn switching_blocks(count: usize, now: DateTime<Utc>) -> Vec<ActivityBlock> {
    (0..count)
        .map(|i| {
            let app = if i % 2 == 0 {
                ("Slack", SLACK)
            } else {
                ("Google Chrome", CHROME)
            };
            let start = now - ChronoDuration::minutes(count as i64 - i as i64);
            let mut b = ActivityBlock::new_manual(
                start,
                start + ChronoDuration::seconds(55),
                "cat-x".into(),
                None,
            );
            b.is_manual = false;
            b.category_id = None;
            b.source = None;
            b.app_name = app.0.into();
            b.app_id = app.1.into();
            b
        })
        .collect()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn focus_prompt_fires_at_ten_switches_and_respects_its_cooldown() {
    let h = harness().await;
    let state = h.state().clone();
    // Nine switches (ten blocks) are not enough.
    for b in switching_blocks(10, fixed_now()) {
        BlockRepo::insert(h.store.as_ref(), &b).unwrap();
    }
    let out = ubiqx_engine::nudges::run_once(&state).unwrap();
    assert!(
        out.iter().all(|n| n.kind != NudgeKind::FocusPrompt),
        "{out:?}"
    );

    // One more block (an open one counts too) makes ten switches.
    let mut open = switching_blocks(1, fixed_now() + ChronoDuration::minutes(1))
        .pop()
        .unwrap();
    open.app_id = "com.microsoft.VSCode".into();
    open.app_name = "Visual Studio Code".into();
    open.is_open = true;
    BlockRepo::insert(h.store.as_ref(), &open).unwrap();
    let out = ubiqx_engine::nudges::run_once(&state).unwrap();
    let prompt = out
        .iter()
        .find(|n| n.kind == NudgeKind::FocusPrompt)
        .expect("focus prompt");
    assert_eq!(prompt.title, "Muitas janelas");
    assert_eq!(
        prompt.message,
        "Você tem alternado entre muitas janelas, que tal focar mais? Que tarefa você precisa fazer agora e quer que eu te ajude com um foco maior?"
    );
    assert_eq!(h.nudges(NudgeKind::FocusPrompt).len(), 1);
    assert!(h
        .notifier
        .0
        .lock()
        .iter()
        .any(|(t, _)| t == "Muitas janelas"));
    assert!(h.events().iter().any(
        |e| matches!(e, EngineEvent::Nudge { nudge } if nudge.kind == NudgeKind::FocusPrompt)
    ));

    // Cooldown: 45 minutes.
    h.clock.advance(ChronoDuration::minutes(30));
    let more: Vec<ActivityBlock> = switching_blocks(12, h.clock.now());
    for b in &more {
        BlockRepo::insert(h.store.as_ref(), b).unwrap();
    }
    let out = ubiqx_engine::nudges::run_once(&state).unwrap();
    assert!(out.iter().all(|n| n.kind != NudgeKind::FocusPrompt));
    h.clock.advance(ChronoDuration::minutes(16));
    for b in switching_blocks(12, h.clock.now()) {
        BlockRepo::insert(h.store.as_ref(), &b).unwrap();
    }
    let out = ubiqx_engine::nudges::run_once(&state).unwrap();
    assert!(out.iter().any(|n| n.kind == NudgeKind::FocusPrompt));

    // Never during a session, and in English when the UI is.
    h.settings_mut(|s| s.language = "en".into());
    h.clock.advance(ChronoDuration::minutes(50));
    for b in switching_blocks(12, h.clock.now()) {
        BlockRepo::insert(h.store.as_ref(), &b).unwrap();
    }
    h.handle.start_focus_session("report", 45).unwrap();
    let out = ubiqx_engine::nudges::run_once(&state).unwrap();
    assert!(out.iter().all(|n| n.kind != NudgeKind::FocusPrompt));
    h.handle.stop_focus_session().unwrap();
    let out = ubiqx_engine::nudges::run_once(&state).unwrap();
    let prompt = out
        .iter()
        .find(|n| n.kind == NudgeKind::FocusPrompt)
        .expect("focus prompt in English");
    assert_eq!(prompt.title, "A lot of windows");
    h.handle.shutdown();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn known_domains_come_from_the_users_blocks() {
    let h = harness().await;
    let now = fixed_now();
    for (i, (domain, mins)) in [
        ("youtube.com", 30),
        ("sei.ifro.edu.br", 90),
        ("youtube.com", 15),
    ]
    .into_iter()
    .enumerate()
    {
        let start = now - ChronoDuration::hours(i as i64 + 1);
        let mut b = ActivityBlock::new_manual(
            start,
            start + ChronoDuration::minutes(mins),
            "cat-x".into(),
            None,
        );
        b.is_manual = false;
        b.category_id = None;
        b.source = None;
        b.domain = Some(domain.into());
        BlockRepo::insert(h.store.as_ref(), &b).unwrap();
    }
    let domains = h.handle.known_domains(30).unwrap();
    assert_eq!(
        domains,
        vec![
            KnownDomain {
                domain: "sei.ifro.edu.br".into(),
                seconds: 90 * 60
            },
            KnownDomain {
                domain: "youtube.com".into(),
                seconds: 45 * 60
            },
        ]
    );
    assert_eq!(h.handle.known_domains(1).unwrap().len(), 1);
    h.handle.shutdown();
}
