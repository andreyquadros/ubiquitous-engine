//! The `nofap` module end to end, over a mock platform and a fake AI: the scenarios of
//! `docs/nofap/DESIGN.md` section 11.2, in its order, followed by the integrator's
//! decisions that concern the service.
//!
//! Every loop of the engine is parked; passes are driven by hand on a fixed clock. Nothing
//! here touches the real system: the system layer is `MockSystemShield`, which keeps a
//! pretend machine in memory. All names and data are fictional.

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use chrono::{DateTime, Duration as ChronoDuration, TimeZone, Utc};
use parking_lot::Mutex;
use tracing_subscriber::fmt::MakeWriter;
use ubiqx_ai::client::LlmContent;
use ubiqx_ai::explicit::LlmExplicitDetector;
use ubiqx_ai::fake::{FakeExplicitDetector, ScriptedLlmClient};
use ubiqx_ai::prompts::{EXPLICIT_SYSTEM_PROMPT, EXPLICIT_USER_TEXT};
use ubiqx_core::clock::FixedClock;
use ubiqx_core::nofap::*;
use ubiqx_core::nofap_ipc::{
    ChangeOutcome, ConsentArgs, EnableOptions, Mood, NofapView, Trigger, UserEvent, VisionScope,
};
use ubiqx_core::ports::*;
use ubiqx_core::segmenter::PRIVATE_TITLE;
use ubiqx_core::*;
use ubiqx_engine::engine::LoopConfig;
use ubiqx_engine::nofap::{Background, DisplaySession, GuardOutcome, Tuning, VisionJob};
use ubiqx_engine::*;
use ubiqx_nofap::matcher::{BlockSource, HostClass, Matcher, MatcherData, Verdict};
use ubiqx_nofap::primary::{self, PrimaryLock};
use ubiqx_platform::mock::{
    EnforcerCall, MockAppCatalog, MockEnforcer, MockSystemShield, ScriptedFrameSource,
    StaticUpdateFeed, SyntheticCapturer, SystemCall,
};
use ubiqx_storage::{Db, SqliteStore};

// ---------------------------------------------------------------------------------------------
// Names the scenarios use (all checked against the embedded lists by `the_names_mean_what_
// the_scenarios_assume`)
// ---------------------------------------------------------------------------------------------

const CHROME: &str = "com.google.Chrome";
/// The title of the browser window the scenarios put in front.
const TAB_TITLE: &str = "Receitas da semana";
const FIREFOX: &str = "org.mozilla.firefox";
const VLC: &str = "org.videolan.vlc";
const SLACK: &str = "com.tinyspeck.slackmacgap";

/// On no list and under no rule: an unknown site.
const UNKNOWN: &str = "sitio-desconhecido.example";
const UNKNOWN_2: &str = "outro-sitio.example";
/// Unknown as well; the scenarios put it on ubiqX's privacy list.
const EXCLUDED: &str = "banco-de-exemplo.example";
/// Unknown, with a signal-grade word in its name.
const SIGNAL: &str = "sexy-portal.example";
const POPULAR: &str = "wikipedia.org";
const MIXED: &str = "reddit.com";
const NEVER_CAPTURE: &str = "mail.google.com";
const BUILT_IN: &str = "github.com";
const KEYWORD: &str = "videosdeputaria.com.br";
const TLD: &str = "anything.xxx";

/// A name of the embedded list (the first canary of the build script).
fn listed() -> &'static str {
    ubiqx_nofap::list::canary_block()[0]
}

fn url_of(host: &str) -> String {
    format!("https://{host}/pagina")
}

/// Mid-afternoon UTC on the repository's fixed test day.
fn fixed_now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 9, 17, 15, 0, 0).unwrap()
}

// ---------------------------------------------------------------------------------------------
// Test doubles
// ---------------------------------------------------------------------------------------------

/// The window in front and the address its browser reports, set by hand. Counts every
/// call, so a test can show that a pass touched no platform port.
struct Foreground {
    window: Mutex<Option<ForegroundWindow>>,
    url: Mutex<Option<String>>,
    /// What `resolve_fresh` answers instead of `url` (a stale cache).
    fresh: Mutex<Option<Option<String>>>,
    /// Answers `peek_foreground` gives first, one per call.
    peeks: Mutex<VecDeque<Option<ForegroundWindow>>>,
    /// Runs once inside the next `resolve_fresh`: something that happens while a pass is
    /// between its first read and its action.
    during_fresh: Mutex<Option<Box<dyn FnOnce() + Send>>>,
    calls: AtomicUsize,
    idle_secs: Mutex<f64>,
    permissions: Mutex<PermissionStatus>,
}

impl Default for Foreground {
    fn default() -> Self {
        Self {
            window: Mutex::new(None),
            url: Mutex::new(None),
            fresh: Mutex::new(None),
            peeks: Mutex::new(VecDeque::new()),
            during_fresh: Mutex::new(None),
            calls: AtomicUsize::new(0),
            idle_secs: Mutex::new(0.0),
            permissions: Mutex::new(PermissionStatus {
                screen_recording: PermissionState::Granted,
                automation: PermissionState::Granted,
                accessibility: PermissionState::Granted,
            }),
        }
    }
}

fn window(app_name: &str, app_id: &str, title: &str, window_id: Option<u32>) -> ForegroundWindow {
    ForegroundWindow {
        app_name: app_name.into(),
        app_id: app_id.into(),
        window_title: title.into(),
        pid: Some(1),
        window_id,
        bounds: None,
    }
}

impl Foreground {
    fn set(&self, window: ForegroundWindow, url: Option<&str>) {
        *self.window.lock() = Some(window);
        *self.url.lock() = url.map(String::from);
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }

    fn count(&self) {
        self.calls.fetch_add(1, Ordering::SeqCst);
    }
}

impl ActivitySource for Foreground {
    fn foreground(&self) -> CoreResult<Option<ForegroundWindow>> {
        self.count();
        Ok(self.window.lock().clone())
    }

    fn peek_foreground(&self) -> CoreResult<Option<ForegroundWindow>> {
        self.count();
        if let Some(scripted) = self.peeks.lock().pop_front() {
            return Ok(scripted);
        }
        Ok(self.window.lock().clone())
    }
}

impl BrowserUrlResolver for Foreground {
    fn supports(&self, app_id: &str) -> bool {
        self.count();
        ubiqx_platform::browser::script_for(app_id).is_some()
    }

    fn resolve(&self, _window: &ForegroundWindow) -> CoreResult<Option<String>> {
        self.count();
        Ok(self.url.lock().clone())
    }

    fn resolve_fresh(&self, _window: &ForegroundWindow) -> CoreResult<Option<String>> {
        self.count();
        let meanwhile = self.during_fresh.lock().take();
        if let Some(meanwhile) = meanwhile {
            meanwhile();
        }
        match self.fresh.lock().clone() {
            Some(fresh) => Ok(fresh),
            None => Ok(self.url.lock().clone()),
        }
    }
}

impl IdleDetector for Foreground {
    fn idle_secs(&self) -> CoreResult<Option<f64>> {
        Ok(Some(*self.idle_secs.lock()))
    }
}

impl PermissionChecker for Foreground {
    fn status(&self) -> PermissionStatus {
        self.count();
        self.permissions.lock().clone()
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

#[derive(Default)]
struct RecordingNotifier(Mutex<Vec<(String, String)>>);

impl Notifier for RecordingNotifier {
    fn notify(&self, title: &str, body: &str) -> CoreResult<()> {
        self.0.lock().push((title.into(), body.into()));
        Ok(())
    }
}

/// The card window: records what it was asked to show; `shows` decides its answer.
struct RecordingPresenter {
    shows: AtomicBool,
    seen: Mutex<Vec<NofapCard>>,
}

impl NofapPresenter for RecordingPresenter {
    fn show(&self, card: &NofapCard) -> CoreResult<bool> {
        self.seen.lock().push(card.clone());
        Ok(self.shows.load(Ordering::SeqCst))
    }
}

#[derive(Default)]
struct RecordingSink(Mutex<Vec<NofapEvent>>);

impl NofapSink for RecordingSink {
    fn emit(&self, event: NofapEvent) {
        self.0.lock().push(event);
    }
}

struct NullInterventions;

impl InterventionPresenter for NullInterventions {
    fn show(&self, _intervention: &Intervention) -> CoreResult<bool> {
        Ok(true)
    }
}

/// Every line any test of this binary logs, at any level. Installed once for the process:
/// the lines of code that runs on other threads (the blocking pool) land here too.
#[derive(Clone)]
struct Capture(Arc<Mutex<Vec<u8>>>);

impl std::io::Write for Capture {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for Capture {
    type Writer = Capture;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

fn captured_log() -> Arc<Mutex<Vec<u8>>> {
    static LOG: OnceLock<Arc<Mutex<Vec<u8>>>> = OnceLock::new();
    LOG.get_or_init(|| {
        let buffer = Arc::new(Mutex::new(Vec::new()));
        let subscriber = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::TRACE)
            .with_ansi(false)
            .with_writer(Capture(buffer.clone()))
            .finish();
        // Another test may have installed it a moment ago; either way there is one.
        let _ = tracing::subscriber::set_global_default(subscriber);
        buffer
    })
    .clone()
}

// ---------------------------------------------------------------------------------------------
// Harness
// ---------------------------------------------------------------------------------------------

/// How a harness differs from the default one.
struct Setup {
    /// The pretend machine of the system layer.
    os: Os,
    /// A scripted LLM behind the real detector instead of the fake detector.
    llm: Option<Vec<String>>,
    /// No detector at all.
    no_detector: bool,
    /// A directory for the primary lock.
    shared: bool,
    /// The engine needs a key in the secret store (and has one for Anthropic).
    needs_key: bool,
    presenter_shows: bool,
}

impl Default for Setup {
    fn default() -> Self {
        Self {
            os: Os::Macos,
            llm: None,
            no_detector: false,
            shared: false,
            needs_key: false,
            presenter_shows: true,
        }
    }
}

struct Harness {
    handle: EngineHandle,
    service: Arc<NofapService>,
    store: Arc<SqliteStore>,
    foreground: Arc<Foreground>,
    enforcer: Arc<MockEnforcer>,
    frames: Arc<ScriptedFrameSource>,
    detector: Arc<FakeExplicitDetector>,
    llm: Option<Arc<ScriptedLlmClient>>,
    shield: Arc<MockSystemShield>,
    presenter: Arc<RecordingPresenter>,
    sink: Arc<RecordingSink>,
    notifier: Arc<RecordingNotifier>,
    secrets: Arc<MemorySecrets>,
    clock: FixedClock,
    tmp: tempfile::TempDir,
}

fn parked() -> LoopConfig {
    let never = Duration::from_secs(3600);
    LoopConfig {
        classify_every: never,
        reports_every: never,
        nudges_every: never,
        retention_every: never,
        update_initial_delay: never,
        update_every: never,
        focus_every: never,
        license_initial_delay: never,
        license_every: never,
        nofap_every: never,
        nofap_verify_every: never,
        without_sampler: true,
    }
}

async fn harness() -> Harness {
    harness_with(Setup::default()).await
}

async fn harness_with(setup: Setup) -> Harness {
    captured_log();
    let tmp = tempfile::tempdir().unwrap();
    let store = Arc::new(SqliteStore::new(
        Db::open(&tmp.path().join("ubiqx.db")).unwrap(),
    ));
    let foreground = Arc::new(Foreground::default());
    let enforcer = Arc::new(MockEnforcer::default());
    let frames = Arc::new(ScriptedFrameSource::new());
    let detector = Arc::new(FakeExplicitDetector::new());
    let llm = setup
        .llm
        .map(|texts| Arc::new(ScriptedLlmClient::new(texts)));
    let shield = Arc::new(MockSystemShield::new(setup.os));
    // Every tier is open on the pretend machine, as on a tester's own (the gate table of
    // a real build decides in production).
    shield.set_capabilities(SystemCapabilities {
        os: Some(setup.os),
        hosts: TierCapability::Available,
        resolver: TierCapability::Available,
        policies: TierCapability::Available,
        unverified: Tier::ALL.to_vec(),
    });
    let presenter = Arc::new(RecordingPresenter {
        shows: AtomicBool::new(setup.presenter_shows),
        seen: Mutex::new(Vec::new()),
    });
    let sink = Arc::new(RecordingSink::default());
    let notifier = Arc::new(RecordingNotifier::default());
    let secrets = Arc::new(MemorySecrets::default());
    if setup.needs_key {
        secrets
            .set(AiProvider::Anthropic.secret_key(), "sk-test-0000")
            .unwrap();
    }
    let clock = FixedClock::new(fixed_now());
    let active: Option<Arc<dyn ExplicitDetector>> = if setup.no_detector {
        None
    } else if let Some(llm) = &llm {
        Some(Arc::new(LlmExplicitDetector::new(llm.clone())))
    } else {
        Some(detector.clone())
    };
    let deps = EngineDeps {
        platform: PlatformPorts {
            activity: foreground.clone(),
            urls: foreground.clone(),
            idle: foreground.clone(),
            capturer: Arc::new(SyntheticCapturer),
            permissions: foreground.clone(),
            secrets: secrets.clone(),
            notifier: notifier.clone(),
            update_feed: Arc::new(StaticUpdateFeed::new(None)),
            apps: Arc::new(MockAppCatalog),
            enforcer: enforcer.clone(),
            presenter: Arc::new(NullInterventions),
        },
        repos: Repos::from_store(store.clone()),
        ai: AiPorts {
            requires_api_key: setup.needs_key,
            ..AiPorts::default()
        },
        sink: Arc::new(NullEventSink),
        clock: Arc::new(clock.clone()),
        data_dir: tmp.path().join("data"),
        build: BuildInfo::dev(),
        update_feed_url: String::new(),
        license: LicenseDeps::default(),
        nofap: NofapPorts {
            system: shield.clone(),
            frames: frames.clone(),
            detector: active,
            presenter: presenter.clone(),
            sink: sink.clone(),
            repo: store.clone(),
            shared_dir: setup.shared.then(|| tmp.path().join("shared")),
            tuning: Tuning {
                confirm_after: Duration::from_millis(5),
                check_timeout: Duration::from_secs(5),
                background: Background::Inline,
                display: DisplaySession::Full,
            },
        },
        identity: &ubiqx_core::product::UBIQX,
    };
    let (handle, _samples) = Engine::start_with(deps, parked()).unwrap();
    let mut settings = handle.settings();
    settings.onboarding_done = true;
    settings.quiet_hours.enabled = false;
    handle.update_settings(settings).unwrap();
    Harness {
        service: handle.nofap().clone(),
        handle,
        store,
        foreground,
        enforcer,
        frames,
        detector,
        llm,
        shield,
        presenter,
        sink,
        notifier,
        secrets,
        clock,
        tmp,
    }
}

fn options(vision: bool) -> EnableOptions {
    EnableOptions {
        self_use: true,
        hosts: false,
        resolver: None,
        policies: false,
        vision,
        vision_consent: vision.then_some(ConsentArgs {
            provider: AiProvider::Anthropic,
            managed: false,
        }),
        vision_scope: VisionScope {
            unknown_sites: true,
            media_apps: true,
            messaging_apps: false,
            private_windows: true,
        },
        delay_secs: 600,
        values: "Quero estar presente com quem eu amo.".into(),
        discreet: true,
    }
}

fn explicit(confidence: Confidence) -> ExplicitCheck {
    FakeExplicitDetector::verdict(ExplicitVerdict::Explicit, confidence)
}

fn clean() -> ExplicitCheck {
    FakeExplicitDetector::verdict(ExplicitVerdict::None, Confidence::High)
}

impl Harness {
    fn state(&self) -> &Arc<EngineState> {
        self.handle.state()
    }

    fn enable(&self, vision: bool) -> NofapView {
        self.service.enable(options(vision)).unwrap()
    }

    /// A browser tab on `host` in front (window 7).
    fn browse(&self, host: &str) {
        self.foreground.set(
            window("Google Chrome", CHROME, TAB_TITLE, Some(7)),
            Some(&url_of(host)),
        );
    }

    /// An application window in front (window 9).
    fn open_app(&self, name: &str, app_id: &str, title: &str) {
        self.foreground
            .set(window(name, app_id, title, Some(9)), None);
    }

    fn advance(&self, secs: i64) {
        self.clock.advance(ChronoDuration::seconds(secs));
    }

    fn pass(&self) -> GuardOutcome {
        self.service.guard_pass().unwrap()
    }

    /// Two passes on what is in front: the second is the one that may act.
    fn two_passes(&self) -> bool {
        self.pass();
        self.pass().hit
    }

    /// Passes every five seconds until the sampler asks for a check, at most `within`
    /// seconds from now. The job is returned, not run.
    fn job_within(&self, within: i64) -> Option<VisionJob> {
        let mut waited = 0;
        loop {
            if let Some(job) = self.pass().job {
                return Some(job);
            }
            if waited >= within {
                return None;
            }
            self.advance(5);
            waited += 5;
        }
    }

    /// Runs the next check the sampler asks for within `within` seconds; `None` when it
    /// asked for none. `Some(hit)` otherwise.
    async fn check_within(&self, within: i64) -> Option<bool> {
        let job = self.job_within(within)?;
        Some(self.service.vision_check(job).await.unwrap().hit)
    }

    fn cards(&self) -> Vec<NofapCard> {
        self.presenter.seen.lock().clone()
    }

    fn cards_of(&self, kind: CardKind) -> Vec<NofapCard> {
        self.cards()
            .into_iter()
            .filter(|card| card.kind == kind)
            .collect()
    }

    fn events(&self) -> Vec<NofapEvent> {
        self.sink.0.lock().clone()
    }

    fn closes(&self) -> usize {
        self.enforcer
            .calls()
            .iter()
            .filter(|call| matches!(call, EnforcerCall::CloseActiveTab(_)))
            .count()
    }

    fn days(&self) -> Vec<DayCount> {
        NofapRepo::days_between(self.store.as_ref(), "0000-01-01", "9999-12-31").unwrap()
    }

    fn counter(&self, kind: DayKind) -> u32 {
        self.days()
            .iter()
            .filter(|count| count.kind == kind)
            .map(|count| count.n)
            .sum()
    }

    fn stored_config(&self) -> NofapConfig {
        NofapRepo::load_config(self.store.as_ref()).unwrap()
    }

    fn stored_sites(&self) -> Vec<NofapSite> {
        NofapRepo::list_sites(self.store.as_ref()).unwrap()
    }

    fn learned(&self) -> u32 {
        self.service.sites().learned_count
    }

    fn settings_mut(&self, change: impl FnOnce(&mut Settings)) {
        let mut settings = self.handle.settings();
        change(&mut settings);
        self.handle.update_settings(settings).unwrap();
    }

    fn pending_ids(&self) -> Vec<Id> {
        self.stored_config()
            .pending
            .into_iter()
            .map(|waiting| waiting.id)
            .collect()
    }

    /// Verify passes that credit `secs` of running time in steps the commitment accepts.
    fn observe_running(&self, secs: u64) {
        let mut left = secs;
        while left > 0 {
            let step = left.min(150);
            self.service
                .verify_pass_after(Duration::from_secs(step))
                .unwrap();
            left -= step;
        }
    }

    /// Every file under the temporary directory, with its bytes.
    fn files(&self) -> Vec<(PathBuf, Vec<u8>)> {
        fn walk(dir: &Path, out: &mut Vec<(PathBuf, Vec<u8>)>) {
            for entry in std::fs::read_dir(dir).unwrap().flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, out);
                } else {
                    out.push((path.clone(), std::fs::read(&path).unwrap_or_default()));
                }
            }
        }
        let mut out = Vec::new();
        walk(self.tmp.path(), &mut out);
        out
    }
}

fn contains(haystack: &[u8], needle: &str) -> bool {
    let needle = needle.as_bytes();
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

// ---------------------------------------------------------------------------------------------
// The names
// ---------------------------------------------------------------------------------------------

#[test]
fn the_names_mean_what_the_scenarios_assume() {
    let matcher = Matcher::new(MatcherData::embedded(), &[], &[0u8; 16], fixed_now()).unwrap();
    let plain = Verdict::Unknown(HostClass::Plain { signal: false });
    assert_eq!(matcher.verdict(UNKNOWN), plain);
    assert_eq!(matcher.verdict(UNKNOWN_2), plain);
    assert_eq!(matcher.verdict(EXCLUDED), plain);
    assert_eq!(
        ubiqx_nofap::sampling::window_kind(VLC, "VLC", false, false),
        ubiqx_nofap::sampling::WindowKind::Media
    );
    assert_eq!(
        matcher.verdict(SIGNAL),
        Verdict::Unknown(HostClass::Plain { signal: true })
    );
    assert_eq!(
        matcher.verdict(POPULAR),
        Verdict::Unknown(HostClass::Popular)
    );
    assert_eq!(matcher.verdict(MIXED), Verdict::Unknown(HostClass::Mixed));
    assert_eq!(
        matcher.verdict(NEVER_CAPTURE),
        Verdict::Unknown(HostClass::NeverCapture)
    );
    assert!(matches!(matcher.verdict(BUILT_IN), Verdict::Allowed(_)));
    assert_eq!(
        matcher.verdict(KEYWORD),
        Verdict::Blocked(BlockSource::Keyword)
    );
    assert_eq!(matcher.verdict(TLD), Verdict::Blocked(BlockSource::Tld));
    assert_eq!(
        matcher.verdict(listed()),
        Verdict::Blocked(BlockSource::List)
    );
    assert!(matcher.in_hosts_tier(listed()));
    assert!(ubiqx_platform::browser::script_for(CHROME).is_some());
    assert!(ubiqx_platform::browser::script_for(FIREFOX).is_none());
}

// ---------------------------------------------------------------------------------------------
// 1 to 8: the guard
// ---------------------------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn s01_off_by_default_no_platform_call_no_card_no_row() {
    let h = harness().await;
    h.browse(listed());
    let before = h.foreground.calls();
    for _ in 0..3 {
        let outcome = h.pass();
        assert!(!outcome.hit && outcome.job.is_none());
    }
    h.service.verify_pass().unwrap();
    assert_eq!(
        h.foreground.calls(),
        before,
        "a pass touched a platform port"
    );
    assert!(h.enforcer.calls().is_empty());
    assert!(h.frames.calls().is_empty());
    assert!(h.shield.calls().is_empty());
    assert_eq!(h.detector.calls(), 0);
    assert!(h.cards().is_empty() && h.events().is_empty());
    assert!(h.notifier.0.lock().is_empty());
    assert!(h.days().is_empty() && h.stored_sites().is_empty());
    assert_eq!(h.stored_config(), NofapConfig::default());
    assert_eq!(h.service.status(), NofapStatus::off());
    // The tracker's two questions are answered "no" without looking anything up.
    let sample = ActivitySample {
        at: fixed_now(),
        app_name: "Google Chrome".into(),
        app_id: CHROME.into(),
        window_title: "Página".into(),
        url: Some(url_of(listed())),
        idle_secs: Some(0.0),
        window_id: Some(7),
    };
    assert!(!h.service.redacts(&sample));
    assert!(!h.service.capture_veto(CHROME, Some(listed()), "Página"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn s02_listed_host_on_two_passes_is_closed_once_seen_twice() {
    let h = harness().await;
    h.enable(false);

    // One pass only, then another site: nothing.
    h.browse(listed());
    assert!(!h.pass().hit);
    h.browse(UNKNOWN);
    assert!(!h.pass().hit);
    h.browse(listed());
    assert!(!h.pass().hit);
    assert!(h.enforcer.calls().is_empty());
    assert!(h.cards().is_empty());
    assert_eq!(h.counter(DayKind::BlockList), 0);

    // The second pass in a row acts.
    assert!(h.pass().hit);
    assert_eq!(
        h.enforcer.calls(),
        vec![EnforcerCall::CloseActiveTab(CHROME.into())]
    );
    assert_eq!(h.counter(DayKind::BlockList), 1);
    let cards = h.cards();
    assert_eq!(cards.len(), 1);
    let card = &cards[0];
    assert_eq!(card.kind, CardKind::Blocked);
    assert!(card.discreet);
    assert_eq!(card.message_key, "block.discreet");
    assert_eq!(card.actions, vec![CardAction::Support, CardAction::Ok]);
    assert_eq!(card.auto_close_secs, Some(12));
    assert!(h
        .events()
        .iter()
        .any(|event| matches!(event, NofapEvent::Card { card: shown } if shown.id == card.id)));
    // The window asks for its card by id; an id nobody knows gets the neutral one.
    assert_eq!(&h.service.card(&card.id), card);
    let neutral = h.service.card("nobody-knows-this-id");
    assert_eq!((neutral.kind, neutral.discreet), (CardKind::Blocked, true));
    assert_eq!(neutral.message_key, "block.discreet");
    // The window was shown, so no notification; and the focus guard's table is untouched.
    assert!(h.notifier.0.lock().is_empty());
    assert!(h.handle.interventions(10).unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn s03_close_refused_blanks_and_both_refused_leaves_the_card() {
    let h = harness_with(Setup {
        presenter_shows: false,
        ..Setup::default()
    })
    .await;
    h.enable(false);
    h.browse(listed());
    h.enforcer.refuse_close_tab.store(true, Ordering::SeqCst);
    assert!(h.two_passes());
    assert_eq!(
        h.enforcer.calls(),
        vec![
            EnforcerCall::CloseActiveTab(CHROME.into()),
            EnforcerCall::BlankActiveTab(CHROME.into()),
        ]
    );

    h.enforcer.clear();
    h.enforcer.refuse_blank_tab.store(true, Ordering::SeqCst);
    h.advance(200);
    assert!(
        h.pass().hit,
        "the hit still counts when nothing could be done"
    );
    assert_eq!(h.enforcer.calls().len(), 2);
    assert!(!h
        .enforcer
        .calls()
        .iter()
        .any(|call| matches!(call, EnforcerCall::QuitApp(_) | EnforcerCall::HideOthers(_))));
    // A card each time (two episodes), and, with no window to show it in, the neutral
    // notification in its place: once per episode.
    assert_eq!(h.cards_of(CardKind::Blocked).len(), 2);
    let notes = h.notifier.0.lock().clone();
    assert_eq!(notes.len(), 2);
    assert_eq!(
        notes[0],
        ("UBI".to_string(), "Página bloqueada pelo UBI.".to_string())
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn s04_five_hits_in_one_episode_five_enforcements_one_count_two_cards() {
    let h = harness().await;
    h.enable(false);
    h.browse(listed());
    h.pass();
    for hit in 1..=5 {
        assert!(h.pass().hit, "hit {hit}");
        h.advance(35);
    }
    assert_eq!(h.closes(), 5, "no enforcement cooldown");
    assert_eq!(h.counter(DayKind::BlockList), 1, "one count per episode");
    let kinds: Vec<CardKind> = h.cards().iter().map(|card| card.kind).collect();
    assert_eq!(kinds, vec![CardKind::Blocked, CardKind::BlockedRepeat]);
    let repeat = &h.cards()[1];
    assert_eq!(repeat.message_key, "block.repeat");
    assert_eq!(repeat.actions, vec![CardAction::Surf, CardAction::Ok]);

    // A new episode after two quiet minutes counts again.
    h.advance(200);
    assert!(h.pass().hit);
    assert_eq!(h.counter(DayKind::BlockList), 2);
    assert_eq!(h.cards().len(), 3);

    // Hits two seconds apart, as the loop makes them: the third comes inside the minute
    // the first card holds, so the "repeat" card is shown once that minute is over.
    for _ in 0..25 {
        h.advance(2);
        assert!(h.pass().hit);
    }
    assert_eq!(h.cards().len(), 3, "at most one card a minute");
    for _ in 0..10 {
        h.advance(2);
        assert!(h.pass().hit);
    }
    let kinds: Vec<CardKind> = h.cards().iter().map(|card| card.kind).collect();
    assert_eq!(kinds.len(), 4, "{kinds:?}");
    assert_eq!(kinds[3], CardKind::BlockedRepeat);
    for _ in 0..40 {
        h.advance(2);
        assert!(h.pass().hit);
    }
    assert_eq!(h.cards().len(), 4, "once per episode");
    assert_eq!(h.counter(DayKind::BlockList), 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn s05_a_stale_cached_address_closes_nothing() {
    let h = harness().await;
    h.enable(false);
    h.browse(listed());
    // The cache still says the listed host; the tab has moved on.
    *h.foreground.fresh.lock() = Some(Some(url_of(UNKNOWN)));
    assert!(!h.two_passes());
    assert!(!h.pass().hit);
    assert!(h.enforcer.calls().is_empty());
    assert!(h.cards().is_empty());
    assert_eq!(h.counter(DayKind::BlockList), 0);

    // No address at all on the fresh read: nothing either.
    *h.foreground.fresh.lock() = Some(None);
    assert!(!h.two_passes());
    assert!(h.enforcer.calls().is_empty());

    // With the fresh read agreeing, the same tab is closed (the pass before this one
    // already saw the host, so this is the second in a row).
    *h.foreground.fresh.lock() = None;
    assert!(h.pass().hit);
    assert_eq!(h.closes(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn s06_another_app_in_front_between_detection_and_action_closes_nothing() {
    let h = harness().await;
    h.enable(false);
    h.browse(listed());
    assert!(!h.pass().hit);
    let chrome = window("Google Chrome", CHROME, TAB_TITLE, Some(7));
    // The pass reads the browser, and the look right before acting finds another app.
    h.foreground.peeks.lock().extend([
        Some(chrome.clone()),
        Some(window(
            "Planilhas",
            "com.example.planilhas",
            "Custos",
            Some(3),
        )),
    ]);
    assert!(!h.pass().hit);
    assert!(h.enforcer.calls().is_empty());

    // The same browser, another window of it: nothing either.
    h.pass();
    h.foreground.peeks.lock().extend([
        Some(chrome.clone()),
        Some(window("Google Chrome", CHROME, "Outra janela", Some(8))),
    ]);
    assert!(!h.pass().hit);
    assert!(h.enforcer.calls().is_empty());
    assert!(h.cards().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn s07_keyword_and_tld_hosts_block_and_the_trap_hosts_do_not() {
    let h = harness().await;
    h.enable(false);
    for (host, closes) in [
        (KEYWORD, 1),
        (TLD, 2),
        ("porno365.golf", 3),
        ("missav123.com", 4),
    ] {
        h.browse(host);
        assert!(h.two_passes(), "{host}");
        assert_eq!(h.closes(), closes, "{host}");
    }
    assert_eq!(h.counter(DayKind::BlockKeyword), 1, "one episode");
    // A keyword block carries the correction on its card. With nothing left to look at
    // the page (the AI check is off) it is the delayed request, not a snooze.
    let card = &h.cards()[0];
    assert_eq!(
        card.actions,
        vec![
            CardAction::Support,
            CardAction::Ok,
            CardAction::RequestException
        ]
    );

    h.enforcer.clear();
    let traps = [
        "essex.gov.uk",
        "sussex.ac.uk",
        "sussexexpress.co.uk",
        "planalto.gov.br",
        "canaltech.com.br",
        "motorola.com.br",
        "drogariasaopaulo.com.br",
        "peacocktv.com",
        "emilfrey.de",
        "milfordasset.com",
        "lichngaytot.com",
        "coroasparavelorio.com.br",
        "nakedwines.com",
        "skylinewebcams.com",
        "iowasexoffender.gov",
        "pussy888.com",
        "cambridge.org",
        "xxxlutz.de",
        "scunthorpe.co.uk",
        "penistone.co.uk",
        "ninjavan.co",
        "fapesp.br",
        "disputas.example",
        "pornichet.example",
        "aeromexico.com",
        "192.168.0.1",
        "nas.local",
    ];
    for host in traps {
        h.browse(host);
        assert!(!h.two_passes(), "{host}");
    }
    // A block word in a path is a signal, never a block.
    h.foreground.set(
        window("Google Chrome", CHROME, "Fotos", Some(7)),
        Some("https://www.reddit.com/r/EarthPorn/"),
    );
    assert!(!h.two_passes());
    assert!(h.enforcer.calls().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn s08_user_allow_beats_the_list_user_block_beats_allow_and_a_snooze_expires() {
    let h = harness().await;
    // While the module is off nothing is being weakened, so the allow is immediate.
    let allowed = h
        .service
        .request(NofapChange::AddAllow {
            host: format!("https://www.{}/x", listed()),
        })
        .unwrap();
    assert!(matches!(allowed, ChangeOutcome::Applied(_)));
    h.enable(true);
    assert_eq!(h.service.sites().allowed[0].host, listed());

    h.browse(listed());
    assert!(!h.two_passes(), "the user's allow beats the list");
    assert!(h.enforcer.calls().is_empty());

    let blocked = h
        .service
        .request(NofapChange::AddBlock {
            host: listed().into(),
        })
        .unwrap();
    assert!(
        matches!(blocked, ChangeOutcome::Applied(_)),
        "tightening is immediate"
    );
    assert!(h.two_passes(), "the user's block beats the user's allow");
    assert_eq!(h.counter(DayKind::BlockUser), 1);

    // A snooze: "Não era isso" on a keyword block, while the AI check can keep looking.
    h.advance(200);
    h.browse(KEYWORD);
    assert!(h.two_passes());
    let card = h.cards().last().unwrap().clone();
    assert_eq!(
        card.actions,
        vec![CardAction::Support, CardAction::Ok, CardAction::NotIt]
    );
    h.service.card_action(&card.id, CardAction::NotIt).unwrap();
    assert_eq!(h.counter(DayKind::NotIt), 1);
    let snooze = h
        .stored_sites()
        .into_iter()
        .find(|site| site.origin == SiteOrigin::Snooze)
        .expect("a snooze row");
    assert_eq!((snooze.list, snooze.host.clone()), (SiteList::Allow, None));
    assert_eq!(
        snooze.expires_at,
        Some(h.clock.now() + ChronoDuration::seconds(86_400))
    );
    assert!(h.pending_ids().is_empty(), "a snooze is not a request");
    let closes = h.closes();
    assert!(!h.two_passes(), "snoozed");
    assert_eq!(h.closes(), closes);

    // A day later the snooze is over and the rule blocks again.
    h.advance(86_401);
    assert!(h.two_passes());
    assert_eq!(h.closes(), closes + 1);
}

// ---------------------------------------------------------------------------------------------
// 9 to 18: the AI check
// ---------------------------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn s09_unknown_host_is_checked_after_15s_and_a_positive_closes_and_learns_it() {
    let h = harness().await;
    h.enable(true);
    h.browse(UNKNOWN);
    assert!(h.pass().job.is_none(), "not in front long enough");
    h.advance(14);
    assert!(h.pass().job.is_none());
    h.advance(1);
    let job = h.pass().job.expect("due after 15 s in front");
    h.detector.push(Ok(explicit(Confidence::High)));
    assert!(h.service.vision_check(job).await.unwrap().hit);

    // One call, with one reduced frame of that window (640 px, the browser's top cropped).
    assert_eq!(h.detector.calls(), 1);
    assert_eq!(h.detector.images()[0].width, 640);
    let (window_id, opts) = h.frames.calls()[0];
    assert_eq!((window_id, opts.max_edge), (7, 640));
    assert!((opts.crop_top_frac - 0.12).abs() < f32::EPSILON);
    assert_eq!(h.detector.models(), vec![h.handle.settings().models.vision]);

    // The tab is closed, the host is learned, the card says so and offers the correction.
    assert_eq!(
        h.enforcer.calls(),
        vec![EnforcerCall::CloseActiveTab(CHROME.into())]
    );
    assert_eq!(h.learned(), 1);
    assert_eq!(h.counter(DayKind::VisionCheck), 1);
    assert_eq!(h.counter(DayKind::VisionHit), 1);
    assert_eq!(
        h.counter(DayKind::Lapse),
        0,
        "a vision hit is never a lapse"
    );
    let card = h.cards_of(CardKind::VisionClosed).pop().expect("the card");
    assert_eq!(card.message_key, "vision.closed");
    assert_eq!(card.note_key.as_deref(), Some("vision.added"));
    assert_eq!(
        card.actions,
        vec![
            CardAction::Ok,
            CardAction::NotIt,
            CardAction::Surf,
            CardAction::AddToList
        ]
    );
    assert_eq!(card.auto_close_secs, None, "stays until answered");

    // From now on the list decides: blocked as learned, no further look.
    assert!(h.two_passes());
    assert_eq!(h.closes(), 2);
    assert_eq!(h.detector.calls(), 1);
    assert_eq!(h.frames.calls().len(), 1);
    assert_eq!(h.counter(DayKind::BlockList), 1);

    // The learned site is its salted hash: the name is in no byte of the database.
    let learned = h
        .stored_sites()
        .into_iter()
        .find(|site| site.origin == SiteOrigin::Learned)
        .unwrap();
    assert_eq!(learned.host, None);
    let files = h.files();
    let database: Vec<_> = files
        .iter()
        .filter(|(path, _)| {
            path.file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with("ubiqx.db"))
        })
        .collect();
    assert!(!database.is_empty());
    for (path, bytes) in database {
        assert!(
            !contains(bytes, UNKNOWN),
            "{} names the host",
            path.display()
        );
        assert!(!contains(bytes, "sitio-desconhecido"));
    }
    // And the sites view shows a count, never a name.
    let sites = serde_json::to_string(&h.service.sites()).unwrap();
    assert!(!sites.contains("desconhecido"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn s10_a_positive_for_a_window_or_host_that_changed_meanwhile_is_dropped() {
    let h = harness().await;
    h.enable(true);

    // Another window of the browser is in front when the answer arrives.
    h.browse(UNKNOWN);
    let job = h.job_within(30).unwrap();
    h.detector.push(Ok(explicit(Confidence::High)));
    h.foreground.set(
        window("Google Chrome", CHROME, "Outra", Some(8)),
        Some(&url_of(UNKNOWN)),
    );
    assert!(!h.service.vision_check(job).await.unwrap().hit);

    // The same window, but the tab went to another site.
    h.browse(UNKNOWN_2);
    let job = h.job_within(30).unwrap();
    h.detector.push(Ok(explicit(Confidence::High)));
    *h.foreground.fresh.lock() = Some(Some(url_of(POPULAR)));
    assert!(!h.service.vision_check(job).await.unwrap().hit);

    assert_eq!(h.detector.calls(), 2);
    assert!(h.enforcer.calls().is_empty(), "nothing closed");
    assert_eq!(h.learned(), 0, "nothing learned");
    assert!(h.cards().is_empty());
    assert_eq!(h.counter(DayKind::VisionHit), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn s11_medium_then_none_asks_a_refusal_shows_nothing_two_refusals_with_a_signal_ask() {
    let h = harness().await;
    h.enable(true);

    // `explicit` at medium confidence: a second frame decides; `none` there is "unsure".
    h.browse(UNKNOWN);
    h.detector.push(Ok(explicit(Confidence::Medium)));
    h.detector.push(Ok(clean()));
    assert_eq!(h.check_within(30).await, Some(false));
    assert_eq!(h.detector.calls(), 2, "two frames");
    assert_eq!(h.counter(DayKind::VisionCheck), 2);
    let asks = h.cards_of(CardKind::VisionAsk);
    assert_eq!(asks.len(), 1);
    assert_eq!(asks[0].message_key, "vision.soft");
    assert_eq!(
        asks[0].actions,
        vec![CardAction::CloseWindow, CardAction::NotIt]
    );
    assert_eq!(asks[0].auto_close_secs, Some(60));

    // A refusal alone: nothing shown.
    h.advance(120);
    h.browse(UNKNOWN_2);
    h.detector.push(Ok(FakeExplicitDetector::refusal()));
    assert_eq!(h.check_within(30).await, Some(false));
    assert_eq!(h.cards().len(), 1);

    // Two refusals in a row on a window that carries a signal of its own: the soft card.
    h.advance(120);
    h.browse(SIGNAL);
    h.detector.push(Ok(FakeExplicitDetector::refusal()));
    h.detector.push(Ok(FakeExplicitDetector::refusal()));
    assert_eq!(h.check_within(30).await, Some(false));
    assert_eq!(h.cards().len(), 1, "one refusal is not enough");
    assert_eq!(h.check_within(200).await, Some(false));
    assert_eq!(h.cards_of(CardKind::VisionAsk).len(), 2);

    // Medium twice is a positive; never before the second frame agreed.
    h.advance(120);
    h.browse("mais-um-sitio.example");
    h.detector.push(Ok(explicit(Confidence::Medium)));
    h.detector.push(Ok(explicit(Confidence::Medium)));
    assert_eq!(h.closes(), 0, "no enforcement so far");
    assert_eq!(h.check_within(30).await, Some(true));
    assert_eq!(h.closes(), 1);
    assert_eq!(h.learned(), 1);
    assert!(!h
        .enforcer
        .calls()
        .iter()
        .any(|call| matches!(call, EnforcerCall::QuitApp(_))));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn s12_another_app_is_hidden_and_quit_only_on_the_button() {
    let h = harness().await;
    h.enable(true);
    h.open_app("VLC", VLC, "ferias.mp4");
    assert!(h.pass().job.is_none());
    h.detector.push(Ok(explicit(Confidence::High)));
    assert_eq!(h.check_within(30).await, Some(true));

    // The frame of another app is not cropped.
    let (window_id, opts) = h.frames.calls()[0];
    assert_eq!(window_id, 9);
    assert!(opts.crop_top_frac.abs() < f32::EPSILON);
    // Reversible first: the other windows are hidden, keeping this product's.
    assert_eq!(
        h.enforcer.calls(),
        vec![EnforcerCall::HideOthers("ai.ubiqx.app".into())]
    );
    assert_eq!(h.learned(), 0);
    let card = h
        .cards_of(CardKind::VisionAsk)
        .pop()
        .expect("the soft card");
    assert_eq!(card.message_key, "vision.soft");
    assert_eq!(
        card.actions,
        vec![CardAction::CloseWindow, CardAction::NotIt]
    );

    // Only the click quits the application.
    h.service
        .card_action(&card.id, CardAction::CloseWindow)
        .unwrap();
    assert_eq!(
        h.enforcer.calls(),
        vec![
            EnforcerCall::HideOthers("ai.ubiqx.app".into()),
            EnforcerCall::QuitApp(VLC.into())
        ]
    );
    assert!(h
        .events()
        .iter()
        .any(|event| matches!(event, NofapEvent::CardClosed { id } if *id == card.id)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn s13_what_is_never_captured() {
    let h = harness().await;
    // On ubiqX's privacy lists when the module is enabled: honoured at once. A player and
    // an unknown site would both be looked at if they were not on them.
    h.settings_mut(|settings| {
        settings.blocked_apps.push("Bloco de Notas".into());
        settings.blocked_apps.push("VLC".into());
        settings.blocked_domains.push(EXCLUDED.into());
    });
    h.service
        .request(NofapChange::AddAllow {
            host: UNKNOWN_2.into(),
        })
        .unwrap();
    h.enable(true);

    let chrome = |title: &str, id: Option<u32>| window("Google Chrome", CHROME, title, id);
    let cases: Vec<(&str, ForegroundWindow, Option<String>)> = vec![
        (
            "popular without a signal",
            chrome("Enciclopédia", Some(7)),
            Some(url_of(POPULAR)),
        ),
        (
            "never-capture",
            chrome("Caixa de entrada", Some(7)),
            Some(url_of(NEVER_CAPTURE)),
        ),
        (
            "built-in allow",
            chrome("Código", Some(7)),
            Some(url_of(BUILT_IN)),
        ),
        (
            "user allow",
            chrome("Página", Some(7)),
            Some(url_of(UNKNOWN_2)),
        ),
        ("blocked", chrome("Página", Some(7)), Some(url_of(listed()))),
        (
            "local",
            chrome("Roteador", Some(7)),
            Some("http://192.168.0.1/admin".into()),
        ),
        (
            "messaging app",
            window("Slack", SLACK, "geral", Some(9)),
            None,
        ),
        (
            "no window id",
            chrome("Página", None),
            Some(url_of(UNKNOWN)),
        ),
        (
            "excluded app",
            window("Bloco de Notas", "com.example.notas", "lista", Some(9)),
            None,
        ),
        (
            "excluded player",
            window("VLC", VLC, "ferias.mp4", Some(9)),
            None,
        ),
        (
            "excluded domain",
            chrome("Extrato", Some(7)),
            Some(url_of(EXCLUDED)),
        ),
        (
            "a page under an excluded domain",
            chrome("Extrato", Some(7)),
            Some(format!("https://conta.{EXCLUDED}/extrato")),
        ),
        (
            "any other app",
            window("Planilhas", "com.example.planilhas", "custos", Some(9)),
            None,
        ),
        (
            "password manager",
            window("1Password", "com.1password.1password", "cofre", Some(9)),
            None,
        ),
        (
            "this product",
            window("ubiqX", "ai.ubiqx.app", "ubiqX", Some(9)),
            None,
        ),
        (
            "the other product",
            window("NoFap", "ai.ubiqx.shield", "NoFap", Some(9)),
            None,
        ),
    ];
    for (name, front, url) in cases {
        h.foreground.set(front, url.as_deref());
        for _ in 0..40 {
            assert!(h.pass().job.is_none(), "{name}");
            h.advance(30);
        }
    }
    assert!(h.frames.calls().is_empty());
    assert_eq!(h.detector.calls(), 0);

    // The same popular site with a signal in the address is looked at.
    h.foreground.set(
        chrome("Fotos", Some(7)),
        Some(&format!("https://{POPULAR}/wiki/Pornography")),
    );
    let job = h.job_within(30).expect("a signal in the address");
    let closed_before = h.closes();

    // A positive there closes the tab and teaches nothing: a well-known site is never
    // put on the list by one picture.
    h.detector.push(Ok(explicit(Confidence::High)));
    assert!(h.service.vision_check(job).await.unwrap().hit);
    assert_eq!(h.closes(), closed_before + 1);
    assert_eq!(h.learned(), 0);
    let card = h.cards_of(CardKind::VisionClosed).pop().expect("the card");
    assert_eq!(card.note_key, None, "nothing was added");
    h.browse(POPULAR);
    assert!(!h.two_passes());
    assert_eq!(h.closes(), closed_before + 1);

    // The same on a platform that cannot be split by domain: one page of it is closed,
    // the platform is not learned.
    h.advance(700);
    h.frames
        .set_fallback(Some(ScriptedFrameSource::sample_frame(4)));
    h.browse(MIXED);
    let job = h.job_within(60).expect("a mixed platform is looked at");
    h.detector.push(Ok(explicit(Confidence::High)));
    assert!(h.service.vision_check(job).await.unwrap().hit);
    assert_eq!(h.closes(), closed_before + 2);
    assert_eq!(h.learned(), 0);
    assert!(!h.two_passes());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn s14_no_idle_gate_and_an_unchanged_frame_is_neither_sent_nor_counted() {
    let h = harness().await;
    h.enable(true);
    // Ten minutes without input: watching a video looks exactly like this.
    *h.foreground.idle_secs.lock() = 600.0;
    h.browse(UNKNOWN);
    h.detector.push(Ok(clean()));
    assert_eq!(h.check_within(30).await, Some(false));
    assert_eq!(h.detector.calls(), 1);
    assert_eq!(h.counter(DayKind::VisionCheck), 1);

    // The same picture again: a frame is taken, found unchanged, and nothing is sent.
    assert_eq!(h.check_within(200).await, Some(false));
    assert_eq!(h.frames.calls().len(), 2);
    assert_eq!(h.detector.calls(), 1, "not sent");
    assert_eq!(h.counter(DayKind::VisionCheck), 1, "not counted");

    // Another picture in the same window is sent.
    h.frames
        .set_fallback(Some(ScriptedFrameSource::sample_frame(4)));
    h.detector.push(Ok(clean()));
    assert_eq!(h.check_within(400).await, Some(false));
    assert_eq!(h.detector.calls(), 2);
    assert_eq!(h.counter(DayKind::VisionCheck), 2);
}

/// One way the AI check is not ready: how to get there, and the reason it must give.
struct NotReady {
    name: &'static str,
    setup: Setup,
    before_enable: fn(&Harness),
    vision: bool,
    after_enable: fn(&Harness),
    reason: VisionOff,
}

fn nothing(_: &Harness) {}

fn vision_spend(h: &Harness, kind: AiUsageKind, cost_usd: f64) {
    UsageRepo::record(
        h.store.as_ref(),
        &AiUsage {
            at: h.clock.now(),
            kind,
            model: "fake".into(),
            input_tokens: 600,
            output_tokens: 15,
            cache_read_tokens: 0,
            cache_write_tokens: 0,
            cost_usd,
        },
    )
    .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn s15_readiness_every_reason_stops_the_check_and_leaves_the_ai_health_alone() {
    let cases = vec![
        NotReady {
            name: "local only",
            setup: Setup::default(),
            before_enable: nothing,
            vision: true,
            after_enable: |h| h.settings_mut(|settings| settings.local_only = true),
            reason: VisionOff::LocalOnly,
        },
        NotReady {
            name: "no key",
            setup: Setup {
                needs_key: true,
                ..Setup::default()
            },
            before_enable: nothing,
            vision: true,
            after_enable: |h| {
                h.secrets
                    .delete(AiProvider::Anthropic.secret_key())
                    .unwrap();
                // The secret store is asked again after a few seconds.
                h.advance(20);
            },
            reason: VisionOff::NoKey,
        },
        NotReady {
            name: "no consent",
            setup: Setup::default(),
            before_enable: nothing,
            vision: false,
            after_enable: |h| {
                let on = h
                    .service
                    .request(NofapChange::SetVision { on: true })
                    .unwrap();
                assert!(matches!(on, ChangeOutcome::Applied(_)));
            },
            reason: VisionOff::ConsentOutdated,
        },
        NotReady {
            name: "consent for another provider",
            setup: Setup::default(),
            before_enable: nothing,
            vision: true,
            after_enable: |h| h.settings_mut(|settings| settings.ai_provider = AiProvider::OpenAi),
            reason: VisionOff::ConsentOutdated,
        },
        NotReady {
            name: "no detector",
            setup: Setup {
                no_detector: true,
                ..Setup::default()
            },
            before_enable: nothing,
            vision: true,
            after_enable: nothing,
            reason: VisionOff::NoProvider,
        },
        NotReady {
            name: "the module's own budget is spent",
            setup: Setup::default(),
            before_enable: nothing,
            vision: true,
            after_enable: |h| vision_spend(h, AiUsageKind::NofapVision, 2.0),
            reason: VisionOff::BudgetSpent,
        },
        NotReady {
            name: "screen recording denied",
            setup: Setup::default(),
            before_enable: nothing,
            vision: true,
            after_enable: |h| {
                h.foreground.permissions.lock().screen_recording = PermissionState::Denied;
            },
            reason: VisionOff::ScreenRecording,
        },
        NotReady {
            name: "switched off",
            setup: Setup::default(),
            before_enable: nothing,
            vision: false,
            after_enable: nothing,
            reason: VisionOff::Disabled,
        },
    ];
    for case in cases {
        let h = harness_with(case.setup).await;
        (case.before_enable)(&h);
        h.enable(case.vision);
        let health = h.state().ai_health();
        (case.after_enable)(&h);
        assert_eq!(h.service.vision_ready(), Err(case.reason), "{}", case.name);
        h.browse(UNKNOWN);
        assert!(h.job_within(400).is_none(), "{}", case.name);
        assert!(h.frames.calls().is_empty(), "{}", case.name);
        assert_eq!(h.detector.calls(), 0, "{}", case.name);
        assert_eq!(h.state().ai_health(), health, "{}", case.name);
        let status = h.service.view().unwrap().status;
        assert_eq!(
            status.vision,
            VisionState::Off {
                reason: case.reason
            },
            "{}",
            case.name
        );
    }

    // With a key it is ready, and the key's presence is all that differs.
    let h = harness_with(Setup {
        needs_key: true,
        ..Setup::default()
    })
    .await;
    h.enable(true);
    assert_eq!(h.service.vision_ready(), Ok(()));
    assert_eq!(h.service.view().unwrap().status.vision, VisionState::On);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn s15_the_caps_and_three_errors_pause_the_check_without_touching_the_ai_health() {
    // The hourly cap, set before the module was switched on.
    let h = harness().await;
    let caps = NofapChange::SetCaps {
        max_per_hour: 1,
        max_per_day: 5,
        monthly_cap_usd: 2.0,
    };
    assert!(matches!(
        h.service.request(caps).unwrap(),
        ChangeOutcome::Applied(_)
    ));
    h.enable(true);
    let health = h.state().ai_health();
    h.browse(UNKNOWN);
    h.detector.push(Ok(clean()));
    assert_eq!(h.check_within(30).await, Some(false));
    assert_eq!(h.service.vision_ready(), Err(VisionOff::CapReached));
    h.browse(UNKNOWN_2);
    assert!(h.job_within(600).is_none());
    assert_eq!(h.detector.calls(), 1);
    assert_eq!(h.state().ai_health(), health);
    assert!(h
        .service
        .view()
        .unwrap()
        .status
        .issues
        .contains(&Issue::VisionCapReached));
    // The hour rolls and the cap with it.
    h.advance(3_600);
    assert_eq!(h.service.vision_ready(), Ok(()));

    // Three failed calls in a row: the layer backs off; the productivity AI is not told.
    let h = harness().await;
    h.enable(true);
    let health = h.state().ai_health();
    for (index, host) in [UNKNOWN, UNKNOWN_2, "terceiro-sitio.example"]
        .into_iter()
        .enumerate()
    {
        h.browse(host);
        h.detector.push(Err(CoreError::Ai("outage".into())));
        assert_eq!(h.check_within(30).await, Some(false), "{index}");
    }
    assert_eq!(h.detector.calls(), 3);
    assert_eq!(
        h.counter(DayKind::VisionCheck),
        0,
        "a failed call is not a check"
    );
    let status = h.service.view().unwrap().status;
    assert!(matches!(status.vision, VisionState::Degraded { .. }));
    assert_eq!(h.state().ai_health(), health);
    h.browse("quarto-sitio.example");
    assert!(h.job_within(50).is_none(), "in backoff");
    assert!(h.enforcer.calls().is_empty() && h.cards().is_empty());
    // After the backoff the next answer brings it back.
    h.advance(120);
    h.detector.push(Ok(clean()));
    assert_eq!(h.check_within(30).await, Some(false));
    assert_eq!(h.service.view().unwrap().status.vision, VisionState::On);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn s16_the_two_budgets_are_separate() {
    // The module's checks over their cap do not pause classification.
    let h = harness().await;
    h.settings_mut(|settings| settings.ai_monthly_budget_usd = 1.0);
    h.enable(true);
    vision_spend(&h, AiUsageKind::NofapVision, 5.0);
    let health = h.state().ai_health();
    assert!(ubiqx_engine::classify::budget_allows(h.state()).unwrap());
    assert_eq!(h.state().ai_health(), health);
    assert_eq!(h.service.vision_ready(), Err(VisionOff::BudgetSpent));
    let view = h.service.view().unwrap();
    assert!((view.cost_line.month_usd - 5.0).abs() < 1e-9);
    assert_eq!(view.cost_line.month_checks, 1);
    assert!((view.cost_line.cap_usd - 2.0).abs() < 1e-9);

    // A spent productivity budget does not stop the checks.
    let h = harness().await;
    h.settings_mut(|settings| settings.ai_monthly_budget_usd = 1.0);
    h.enable(true);
    vision_spend(&h, AiUsageKind::Classify, 5.0);
    assert!(!ubiqx_engine::classify::budget_allows(h.state()).unwrap());
    assert!(matches!(h.state().ai_health(), AiHealth::Paused { .. }));
    assert_eq!(h.service.vision_ready(), Ok(()));
    h.browse(UNKNOWN);
    h.detector.push(Ok(clean()));
    assert_eq!(h.check_within(30).await, Some(false));
    assert_eq!(h.detector.calls(), 1);
    assert!(matches!(h.state().ai_health(), AiHealth::Paused { .. }));
}

/// A positive on `UNKNOWN`, and the card it showed.
async fn positive_on_unknown(h: &Harness) -> NofapCard {
    h.browse(UNKNOWN);
    h.detector.push(Ok(explicit(Confidence::High)));
    assert_eq!(h.check_within(30).await, Some(true));
    assert_eq!(h.learned(), 1);
    h.cards_of(CardKind::VisionClosed).pop().unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn s17_not_it_on_a_vision_card_buys_one_independent_look() {
    // The second look agrees.
    let h = harness().await;
    h.enable(true);
    let card = positive_on_unknown(&h).await;
    h.service.card_action(&card.id, CardAction::NotIt).unwrap();
    assert_eq!(h.learned(), 0, "unlearned");
    assert_eq!(h.counter(DayKind::NotIt), 1);
    assert!(h.pending_ids().is_empty());
    assert_eq!(h.closes(), 1, "nothing more is done to the window for now");
    h.advance(10);
    assert!(h.pass().job.is_none(), "the second look comes 20 s later");
    assert_eq!(h.closes(), 1);
    h.advance(60);
    h.detector.push(Ok(explicit(Confidence::High)));
    let job = h.pass().job.expect("the look the button bought");
    assert!(h.service.vision_check(job).await.unwrap().hit);
    assert_eq!(h.closes(), 2, "closed again");
    assert_eq!(h.learned(), 1, "learned again");
    // For the rest of the day the button on this window asks for a delayed exception.
    let again = h.cards_of(CardKind::VisionClosed).pop().unwrap();
    assert_ne!(again.id, card.id);
    assert_eq!(
        again.actions,
        vec![
            CardAction::Ok,
            CardAction::RequestException,
            CardAction::Surf,
            CardAction::AddToList
        ]
    );
    h.service
        .card_action(&again.id, CardAction::RequestException)
        .unwrap();
    let pending = h.stored_config().pending;
    assert_eq!(pending.len(), 1);
    assert_eq!(
        pending[0].change,
        NofapChange::AddAllow {
            host: UNKNOWN.into()
        }
    );
    assert_eq!(h.learned(), 1, "still blocked while the request waits");
    assert!(h.two_passes());

    // The second look disagrees: nothing closed, and the window stays under watch.
    let h = harness().await;
    h.enable(true);
    let card = positive_on_unknown(&h).await;
    h.service.card_action(&card.id, CardAction::NotIt).unwrap();
    h.advance(20);
    h.detector.push(Ok(clean()));
    let job = h.pass().job.expect("the second look");
    assert!(!h.service.vision_check(job).await.unwrap().hit);
    assert_eq!(h.closes(), 1);
    assert_eq!(h.learned(), 0);
    assert_eq!(h.detector.calls(), 2);
    // Signal cadence: the next look comes well before the five minutes of a quiet site.
    h.detector.push(Ok(clean()));
    h.frames
        .set_fallback(Some(ScriptedFrameSource::sample_frame(3)));
    assert_eq!(h.check_within(120).await, Some(false));
    assert_eq!(h.detector.calls(), 3);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn s17_not_it_on_a_keyword_block_is_a_request_without_the_ai_check_and_after_three() {
    // Nothing is left to look at the page: a snooze would be a free pass for a day.
    let h = harness().await;
    h.enable(false);
    h.browse(KEYWORD);
    assert!(h.two_passes());
    let card = h.cards().pop().unwrap();
    h.service.card_action(&card.id, CardAction::NotIt).unwrap();
    assert!(!h
        .stored_sites()
        .iter()
        .any(|site| site.origin == SiteOrigin::Snooze));
    let pending = h.stored_config().pending;
    assert_eq!(pending.len(), 1);
    assert_eq!(
        pending[0].change,
        NofapChange::AddAllow {
            host: KEYWORD.into()
        }
    );
    assert_eq!(h.counter(DayKind::OverrideRequest), 1);
    assert!(h.pass().hit, "still blocked while the request waits");

    // With the AI check ready: three snoozes a day, the fourth is a request.
    let h = harness().await;
    h.enable(true);
    for (index, host) in [KEYWORD, TLD, "terceira-coisa.xxx"].into_iter().enumerate() {
        h.advance(200);
        h.browse(host);
        assert!(h.two_passes());
        let card = h.cards().pop().unwrap();
        assert!(card.actions.contains(&CardAction::NotIt), "{index}");
        h.service.card_action(&card.id, CardAction::NotIt).unwrap();
        assert!(!h.two_passes(), "{host} is snoozed");
    }
    let snoozes = |h: &Harness| {
        h.stored_sites()
            .iter()
            .filter(|site| site.origin == SiteOrigin::Snooze)
            .count()
    };
    assert_eq!(snoozes(&h), 3);
    assert!(h.pending_ids().is_empty());

    h.advance(200);
    h.browse("quarta-coisa.xxx");
    assert!(h.two_passes());
    let card = h.cards().pop().unwrap();
    assert!(
        card.actions.contains(&CardAction::RequestException),
        "the button says what it does"
    );
    // The window may still send the old name of the button: it is a request either way.
    h.service.card_action(&card.id, CardAction::NotIt).unwrap();
    assert_eq!(snoozes(&h), 3, "no fourth snooze");
    assert_eq!(h.stored_config().pending.len(), 1);
    assert_eq!(h.counter(DayKind::NotIt), 4);
    assert!(h.pass().hit);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn s18_an_app_excluded_after_enable_is_looked_at_until_its_delay_has_passed() {
    let h = harness().await;
    h.enable(true);
    h.open_app("VLC", VLC, "ferias.mp4");
    // A minute after the module was switched on, the app goes on ubiqX's privacy list.
    h.advance(60);
    h.settings_mut(|settings| settings.blocked_apps.push(VLC.into()));
    h.service.verify_pass_after(Duration::ZERO).unwrap();
    let seen = h.stored_config().internal.exclusions_seen;
    let entry = seen
        .iter()
        .find(|entry| entry.key == format!("app:{VLC}"))
        .expect("the new entry is remembered");
    assert_eq!(entry.first_seen_at, h.clock.now());
    assert_eq!(entry.observed_secs, 0);

    // Still a candidate: the list is not an instant off switch.
    h.detector.push(Ok(clean()));
    assert_eq!(h.check_within(30).await, Some(false));
    assert_eq!(h.detector.calls(), 1);

    // The clock alone does not end the wait (it can be set forward)...
    h.advance(601);
    h.observe_running(60);
    h.frames
        .set_fallback(Some(ScriptedFrameSource::sample_frame(2)));
    h.detector.push(Ok(clean()));
    assert_eq!(h.check_within(700).await, Some(false), "still looked at");
    assert_eq!(h.detector.calls(), 2);

    // ...the running time does. From then on the app is never looked at.
    h.observe_running(600);
    let frames = h.frames.calls().len();
    for _ in 0..80 {
        assert!(h.pass().job.is_none());
        h.advance(30);
    }
    assert_eq!(h.frames.calls().len(), frames);
    assert_eq!(h.detector.calls(), 2);
}

// ---------------------------------------------------------------------------------------------
// 19 to 21: commitment and the system tiers
// ---------------------------------------------------------------------------------------------

fn with_hosts(vision: bool) -> EnableOptions {
    EnableOptions {
        hosts: true,
        ..options(vision)
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn s19_turning_it_off_waits_the_delay_and_real_running_time_then_a_confirmation() {
    let h = harness_with(Setup {
        os: Os::Linux,
        ..Setup::default()
    })
    .await;
    let view = h.service.enable(with_hosts(false)).unwrap();
    assert_eq!(view.status.hosts, TierState::Applied);
    assert_eq!(view.status.overall, Overall::Protected);
    assert!(h
        .shield
        .calls()
        .iter()
        .any(|call| matches!(call, SystemCall::Apply(plan) if plan.hosts && plan.owner == ProductId::Ubiqx)));

    let requested = h.service.request(NofapChange::Disable).unwrap();
    let ChangeOutcome::Pending(waiting) = requested else {
        panic!("turning it off is never immediate");
    };
    assert_eq!(waiting.kind, ChangeKind::Disable);
    assert!(!waiting.ready);
    assert_eq!(waiting.remaining_secs, 600);
    assert_eq!(h.counter(DayKind::OverrideRequest), 1);
    assert!(h.events().contains(&NofapEvent::PendingChanged));
    assert_eq!(h.service.status().overall, Overall::PendingOff);

    // It keeps enforcing while the request waits.
    h.browse(listed());
    assert!(h.two_passes());

    // The clock says ten minutes, the app saw one: not ready.
    h.advance(600);
    h.observe_running(60);
    let view = h.service.view().unwrap();
    assert!(!view.status.pending[0].ready);
    assert!(h.cards_of(CardKind::PendingReady).is_empty());
    assert!(h.service.confirm_pending(&waiting.id).is_err());
    assert!(h.service.enabled());

    // Nine minutes of running time are still not ten.
    h.observe_running(480);
    assert!(!h.service.view().unwrap().status.pending[0].ready);
    assert!(h.cards_of(CardKind::PendingReady).is_empty());

    // Ten minutes of running time: ready, and the card asks (it does not apply).
    h.observe_running(60);
    let view = h.service.view().unwrap();
    assert!(view.status.pending[0].ready);
    let card = h.cards_of(CardKind::PendingReady).pop().expect("the card");
    assert_eq!(card.message_key, "pending.ready");
    assert_eq!(
        card.actions,
        vec![CardAction::KeepProtection, CardAction::TurnOff]
    );
    assert!(h.service.enabled(), "ready is not applied");
    assert!(h.pass().hit);

    // "Manter proteção": still on.
    h.service
        .card_action(&card.id, CardAction::KeepProtection)
        .unwrap();
    assert!(h.pending_ids().is_empty());
    assert!(h.service.enabled());
    assert_eq!(h.counter(DayKind::OverrideCancel), 1);

    // Asked again, ready, and then nobody confirms for a day: it lapses.
    let ChangeOutcome::Pending(second) = h.service.request(NofapChange::Disable).unwrap() else {
        panic!("pending");
    };
    h.advance(600);
    h.observe_running(600);
    assert!(h.service.view().unwrap().status.pending[0].ready);
    h.advance(86_400);
    h.observe_running(1);
    assert!(h.pending_ids().is_empty(), "expired");
    assert!(matches!(
        h.service.confirm_pending(&second.id),
        Err(CoreError::NotFound(_))
    ));
    assert!(h.service.enabled());

    // Asked a third time and confirmed: off, and the system is asked once to remove.
    let ChangeOutcome::Pending(third) = h.service.request(NofapChange::Disable).unwrap() else {
        panic!("pending");
    };
    h.advance(600);
    h.observe_running(600);
    h.shield.clear_calls();
    let view = h.service.confirm_pending(&third.id).unwrap();
    assert!(!view.config.enabled);
    assert_eq!(view.status.overall, Overall::Off);
    let removes: Vec<_> = h
        .shield
        .calls()
        .into_iter()
        .filter(|call| matches!(call, SystemCall::Remove(..)))
        .collect();
    assert_eq!(
        removes,
        vec![SystemCall::Remove(ProductId::Ubiqx, RemoveScope::All)]
    );
    assert_eq!(
        view.status.hosts,
        TierState::NotApplied,
        "taken off the machine"
    );
    assert!(!h
        .shield
        .observation()
        .hosts
        .text
        .unwrap()
        .contains("ubiqx-shield"));
    let before = h.foreground.calls();
    assert!(!h.pass().hit);
    assert_eq!(h.foreground.calls(), before, "off: no platform call");
    assert_eq!(h.counter(DayKind::OverrideRequest), 3);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn s20_settings_and_delete_all_data_cannot_flip_it() {
    let h = harness().await;
    let old_settings = h.handle.settings();
    h.enable(false);
    h.service
        .request(NofapChange::AddBlock {
            host: UNKNOWN.into(),
        })
        .unwrap();
    h.browse(listed());
    assert!(h.two_passes());
    let config = h.stored_config();
    let sites = h.stored_sites();
    assert!(config.enabled && sites.len() == 1);

    // A whole old settings object written back (the page's last-writer-wins save).
    h.handle.update_settings(old_settings).unwrap();
    // "Apagar todos os dados".
    h.handle.delete_all_data().unwrap();

    assert_eq!(h.stored_config(), config);
    assert_eq!(h.stored_sites(), sites);
    assert!(h.service.enabled());
    assert!(h.days().is_empty(), "the counters are activity data and go");
    h.advance(200);
    assert!(h.two_passes(), "still enforcing");
    h.browse(UNKNOWN);
    assert!(h.two_passes(), "the user's list is still there");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn s21_drift_is_said_once_and_restoring_the_network_removes_only_the_resolver() {
    let h = harness().await;
    let view = h
        .service
        .enable(EnableOptions {
            resolver: Some(ResolverId::CloudflareFamily),
            ..with_hosts(false)
        })
        .unwrap();
    assert_eq!(view.status.hosts, TierState::Applied);
    assert_eq!(view.status.resolver, TierState::Applied);
    assert_eq!(view.status.overall, Overall::Protected);
    // What was there before is remembered for display, also "automatic".
    assert_eq!(
        h.stored_config().internal.previous_dns,
        vec![PreviousDns {
            interface: "Wi-Fi".into(),
            servers: vec![]
        }]
    );

    // Somebody takes the block out of the hosts file.
    h.shield
        .edit_observation(|machine| machine.hosts.text = Some("127.0.0.1 localhost\n".into()));
    h.service
        .verify_pass_after(Duration::from_secs(120))
        .unwrap();
    let status = h.service.status();
    assert_eq!(status.hosts, TierState::Missing);
    assert_eq!(status.overall, Overall::Reduced);
    assert!(h.events().iter().any(
        |event| matches!(event, NofapEvent::Status { status } if status.overall == Overall::Reduced)
    ));
    let drift = h.cards_of(CardKind::Drift);
    assert_eq!(drift.len(), 1);
    assert_eq!(drift[0].message_key, "drift.card");
    assert_eq!(
        drift[0].actions,
        vec![CardAction::Restore, CardAction::Later]
    );
    assert_eq!(h.counter(DayKind::Drift), 1);
    assert!(
        !h.shield
            .calls()
            .iter()
            .any(|call| matches!(call, SystemCall::Apply(_)))
            || h.shield
                .calls()
                .iter()
                .filter(|call| matches!(call, SystemCall::Apply(_)))
                .count()
                == 1,
        "nothing is repaired silently"
    );

    // The next passes say nothing more.
    h.advance(120);
    h.service
        .verify_pass_after(Duration::from_secs(120))
        .unwrap();
    h.advance(120);
    h.service
        .verify_pass_after(Duration::from_secs(120))
        .unwrap();
    assert_eq!(h.cards_of(CardKind::Drift).len(), 1);
    assert_eq!(h.counter(DayKind::Drift), 1);

    // "Restaurar" on the card applies again, through the prompt.
    h.service
        .card_action(&drift[0].id, CardAction::Restore)
        .unwrap();
    assert_eq!(h.service.status().hosts, TierState::Applied);
    assert_eq!(h.service.status().overall, Overall::Protected);

    // The block is taken out again within the six hours two such cards keep between
    // them: counted, shown on the page, no card.
    h.advance(3600);
    h.shield
        .edit_observation(|machine| machine.hosts.text = Some("127.0.0.1 localhost\n".into()));
    h.service
        .verify_pass_after(Duration::from_secs(120))
        .unwrap();
    assert_eq!(h.service.status().hosts, TierState::Missing);
    assert_eq!(h.counter(DayKind::Drift), 2);
    assert_eq!(h.cards_of(CardKind::Drift).len(), 1);
    assert_eq!(h.service.system_apply().unwrap(), ApplyOutcome::Applied);

    // A later drift has its card, and the card is once per drift: the same drift does
    // not say it again when the six hours have passed.
    h.advance(7 * 3600);
    h.shield
        .edit_observation(|machine| machine.hosts.text = Some("127.0.0.1 localhost\n".into()));
    h.service
        .verify_pass_after(Duration::from_secs(120))
        .unwrap();
    assert_eq!(h.cards_of(CardKind::Drift).len(), 2);
    h.advance(7 * 3600);
    h.service
        .verify_pass_after(Duration::from_secs(120))
        .unwrap();
    assert_eq!(h.service.status().hosts, TierState::Missing);
    assert_eq!(h.cards_of(CardKind::Drift).len(), 2);
    assert_eq!(h.counter(DayKind::Drift), 3);
    assert_eq!(h.service.system_apply().unwrap(), ApplyOutcome::Applied);
    assert_eq!(h.service.status().overall, Overall::Protected);

    // "Restaurar minha rede": the resolver goes, the hosts block stays, nothing waits.
    h.shield.clear_calls();
    assert_eq!(h.service.restore_network().unwrap(), ApplyOutcome::Applied);
    let removes: Vec<_> = h
        .shield
        .calls()
        .into_iter()
        .filter(|call| matches!(call, SystemCall::Remove(..) | SystemCall::Apply(_)))
        .collect();
    assert_eq!(
        removes,
        vec![SystemCall::Remove(ProductId::Ubiqx, RemoveScope::Resolver)]
    );
    let status = h.service.status();
    assert_eq!(status.hosts, TierState::Applied);
    assert_eq!(status.resolver, TierState::NotApplied);
    assert!(h.stored_config().system.network_restored);
    assert_eq!(h.counter(DayKind::NetworkRestore), 1);
    assert_eq!(h.counter(DayKind::OverrideRequest), 0, "not an override");
    assert!(h.pending_ids().is_empty());

    // "Religar" is an apply: the resolver comes back.
    assert_eq!(h.service.system_apply().unwrap(), ApplyOutcome::Applied);
    assert_eq!(h.service.status().resolver, TierState::Applied);
    assert!(!h.stored_config().system.network_restored);
}

// ---------------------------------------------------------------------------------------------
// 22 to 24: the tracker, two products, the privacy sweep
// ---------------------------------------------------------------------------------------------

fn sample_of(h: &Harness, front: &ForegroundWindow, url: Option<String>) -> ActivitySample {
    ActivitySample {
        at: h.clock.now(),
        app_name: front.app_name.clone(),
        app_id: front.app_id.clone(),
        window_title: front.window_title.clone(),
        url,
        idle_secs: Some(0.0),
        window_id: front.window_id,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn s22_the_tracker_forgets_a_hit() {
    let h = harness().await;
    h.enable(true);
    let state = h.state().clone();

    // A sample of a blocked page reaches the segmenter as private browsing does.
    let chrome = window(
        "Google Chrome",
        CHROME,
        "Um título que não deve ficar",
        Some(7),
    );
    let sample = sample_of(&h, &chrome, Some(url_of(listed())));
    ubiqx_engine::tracker::handle_sample(&state, sample).unwrap();
    let open = BlockRepo::open_block(h.store.as_ref()).unwrap().unwrap();
    assert_eq!(open.title, PRIVATE_TITLE);
    assert_eq!((open.url, open.domain), (None, None));
    assert_eq!(
        open.category_id.as_deref(),
        Some(system_categories::PRIVATE)
    );

    // A page nobody knows is tracked as usual, with its title and address, until the AI
    // check answers positive for it: then the block the tracker had open is scrubbed.
    let tab = window(
        "Google Chrome",
        CHROME,
        "Outro título que não deve ficar",
        Some(7),
    );
    for _ in 0..4 {
        h.advance(30);
        let sample = sample_of(&h, &tab, Some(url_of(UNKNOWN)));
        ubiqx_engine::tracker::handle_sample(&state, sample).unwrap();
    }
    let open = BlockRepo::open_block(h.store.as_ref()).unwrap().unwrap();
    assert_eq!(open.url, Some(url_of(UNKNOWN)));
    assert_eq!(open.domain.as_deref(), Some(UNKNOWN));
    h.foreground.set(tab.clone(), Some(&url_of(UNKNOWN)));
    h.detector.push(Ok(explicit(Confidence::High)));
    assert_eq!(h.check_within(30).await, Some(true));
    ubiqx_engine::tracker::scrub_open_block(&state).unwrap();
    let block = BlockRepo::get(h.store.as_ref(), &open.id).unwrap().unwrap();
    assert!(!block.is_open);
    assert_eq!(block.title, PRIVATE_TITLE);
    assert_eq!((block.url.clone(), block.domain.clone()), (None, None));
    assert_eq!(
        block.category_id.as_deref(),
        Some(system_categories::PRIVATE)
    );

    // Another application, tracked with a screenshot.
    h.advance(200);
    let player = window("VLC", VLC, "ferias.mp4", Some(9));
    for _ in 0..4 {
        h.advance(30);
        ubiqx_engine::tracker::handle_sample(&state, sample_of(&h, &player, None)).unwrap();
    }
    let open = BlockRepo::open_block(h.store.as_ref()).unwrap().unwrap();
    assert_eq!(open.title, "ferias.mp4");
    let shot = ubiqx_engine::screenshots::capture_for_block(&state, &open, Some(9), None).unwrap();
    BlockRepo::set_screenshot(h.store.as_ref(), &open.id, &shot.id).unwrap();
    assert!(Path::new(&shot.path).exists());

    // The AI check answers positive for that window; the engine's loop then scrubs.
    h.foreground.set(player.clone(), None);
    h.detector.push(Ok(explicit(Confidence::High)));
    assert_eq!(h.check_within(30).await, Some(true));
    ubiqx_engine::tracker::scrub_open_block(&state).unwrap();

    let block = BlockRepo::get(h.store.as_ref(), &open.id).unwrap().unwrap();
    assert!(!block.is_open);
    assert_eq!(block.title, PRIVATE_TITLE);
    assert_eq!((block.url.clone(), block.domain.clone()), (None, None));
    assert_eq!(
        (block.description.clone(), block.ai_payload.clone()),
        (None, None)
    );
    assert_eq!(
        block.category_id.as_deref(),
        Some(system_categories::PRIVATE)
    );
    assert_eq!(block.screenshot_id, None);
    assert_eq!(
        ScreenshotRepo::get(h.store.as_ref(), &shot.id).unwrap(),
        None
    );
    assert!(!Path::new(&shot.path).exists(), "the file is gone too");
    assert!(BlockRepo::open_block(h.store.as_ref()).unwrap().is_none());

    // For ten minutes no productivity screenshot of that window, and its samples are
    // private; another window is captured as usual.
    let later = h.clock.now();
    let mut same = ActivityBlock::new_manual(
        later - ChronoDuration::seconds(60),
        later,
        String::new(),
        None,
    );
    same.is_manual = false;
    same.category_id = None;
    same.app_id = VLC.into();
    same.app_name = "VLC".into();
    same.title = "ferias.mp4".into();
    let mut other = same.clone();
    other.id = new_id();
    other.app_id = "com.example.planilhas".into();
    other.app_name = "Planilhas".into();
    other.title = "custos".into();
    assert!(!ubiqx_engine::screenshots::should_capture(
        &state, &same, later
    ));
    assert!(ubiqx_engine::screenshots::should_capture(
        &state, &other, later
    ));
    assert!(h.service.redacts(&sample_of(&h, &player, None)));
    h.advance(601);
    let later = h.clock.now();
    assert!(ubiqx_engine::screenshots::should_capture(
        &state, &same, later
    ));
    assert!(!h.service.redacts(&sample_of(&h, &player, None)));

    // A mixed platform is never photographed for productivity while the module is on.
    let mut mixed = other.clone();
    mixed.app_id = CHROME.into();
    mixed.domain = Some(MIXED.into());
    assert!(!ubiqx_engine::screenshots::should_capture(
        &state, &mixed, later
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn s23_the_other_product_runs_the_passes_while_its_heartbeat_is_fresh() {
    let h = harness_with(Setup {
        shared: true,
        ..Setup::default()
    })
    .await;
    let dir = h.tmp.path().join("shared");
    let lock = dir.join(primary::LOCK_FILE);
    std::fs::create_dir_all(&dir).unwrap();
    let heartbeat = |at: DateTime<Utc>| {
        primary::render_lock(&PrimaryLock {
            product: ProductId::Shield,
            pid: 4_242,
            heartbeat_at: at,
        })
    };
    std::fs::write(&lock, heartbeat(h.clock.now())).unwrap();
    h.enable(false);
    h.browse(listed());

    let before = h.foreground.calls();
    assert!(!h.two_passes());
    assert_eq!(h.foreground.calls(), before, "the secondary runs no pass");
    assert!(h.enforcer.calls().is_empty());
    let status = h.service.view().unwrap().status;
    assert_eq!(status.overall, Overall::Secondary);
    assert!(status.issues.contains(&Issue::OtherProductPrimary));
    assert_eq!(
        std::fs::read_to_string(&lock).unwrap(),
        heartbeat(fixed_now())
    );

    // The other app stops beating: after 90 s this one takes over and says so in the lock.
    h.advance(95);
    assert!(h.two_passes());
    assert_eq!(h.closes(), 1);
    let mine = primary::parse_lock(&std::fs::read_to_string(&lock).unwrap()).unwrap();
    assert_eq!(mine.product, ProductId::Ubiqx);
    assert_eq!(mine.heartbeat_at, h.clock.now());
    assert_eq!(h.service.status().overall, Overall::Protected);

    // A heartbeat far in the future (a clock change, a file edited by hand) is stale too.
    std::fs::write(&lock, heartbeat(h.clock.now() + ChronoDuration::hours(5))).unwrap();
    h.advance(35);
    assert!(h.pass().hit);

    // Quitting counts, and gives the lock up so the other product can take over.
    h.service.on_quit();
    assert!(!lock.exists());
    assert_eq!(h.counter(DayKind::QuitOn), 1);
}

fn answer(verdict: &str, confidence: &str) -> String {
    format!("{{\"verdict\":\"{verdict}\",\"confidence\":\"{confidence}\"}}")
}

/// A line-counting subscriber for the thread that drives a scenario.
#[derive(Clone, Default)]
struct Lines(Arc<Mutex<Vec<String>>>);

impl std::io::Write for Lines {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .push(String::from_utf8_lossy(bytes).into_owned());
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for Lines {
    type Writer = Lines;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn s24_privacy_sweep_twenty_checks_three_hits_leave_no_trace() {
    let hits = [3usize, 9, 15];
    let texts: Vec<String> = (0..20)
        .map(|index| {
            if hits.contains(&index) {
                answer("explicit", "high")
            } else if index % 4 == 1 {
                answer("suggestive", "medium")
            } else {
                answer("none", "high")
            }
        })
        .collect();
    let h = harness_with(Setup {
        llm: Some(texts),
        presenter_shows: false,
        ..Setup::default()
    })
    .await;
    h.enable(true);
    let hosts: Vec<String> = (0..20)
        .map(|index| format!("pagina-{index}.example"))
        .collect();

    // Everything this thread logs from here on, at any level.
    let lines = Lines::default();
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::TRACE)
        .with_ansi(false)
        .with_writer(lines.clone())
        .finish();
    let scoped = tracing::subscriber::set_default(subscriber);
    // The capture works: a line written now is seen.
    tracing::info!("marker before the checks");
    assert_eq!(lines.0.lock().len(), 1);
    let mut after_five = 0;
    for (index, host) in hosts.iter().enumerate() {
        h.browse(host);
        let hit = h.check_within(60).await.expect("a check");
        assert_eq!(hit, hits.contains(&index), "{index}");
        if hit {
            // The page stays in front for a moment: the guard holds it as learned.
            assert!(h.two_passes());
        }
        h.advance(100);
        if index == 4 {
            after_five = lines.0.lock().len();
        }
    }
    let logged = lines.0.lock().clone();
    drop(scoped);
    assert_eq!(
        logged.len(),
        after_five,
        "the log grew with the number of checks: {logged:?}"
    );

    // A while later one of the learned pages is opened again. This block has its card
    // (the earlier ones came within the minute of the card of the check), and that card
    // is part of everything looked through below.
    h.advance(700);
    h.browse(&hosts[3]);
    assert!(h.two_passes());
    assert_eq!(h.cards_of(CardKind::Blocked).len(), 1);

    assert_eq!(h.counter(DayKind::VisionCheck), 20);
    assert_eq!(h.counter(DayKind::VisionHit), 3);
    assert_eq!(h.learned(), 3);

    // Every request carried one image and the two constant strings, nothing else.
    let requests = h.llm.as_ref().unwrap().requests();
    assert_eq!(requests.len(), 20);
    for request in &requests {
        let system: Vec<&str> = request
            .system
            .iter()
            .map(|block| block.text.as_str())
            .collect();
        assert_eq!(system, vec![EXPLICIT_SYSTEM_PROMPT]);
        assert_eq!(request.usage_kind, AiUsageKind::NofapVision);
        assert_eq!(request.messages.len(), 1);
        let content = &request.messages[0].content;
        assert_eq!(content.len(), 2);
        assert!(
            matches!(&content[0], LlmContent::Image { media_type, .. } if media_type == "image/jpeg")
        );
        assert_eq!(content[1], LlmContent::Text(EXPLICIT_USER_TEXT.into()));
        let whole = format!("{request:?}");
        for host in &hosts {
            assert!(!whole.contains(host.as_str()));
        }
        assert!(!whole.contains(TAB_TITLE) && !whole.contains("Chrome"));
    }

    // No image was written anywhere.
    let files = h.files();
    for (path, bytes) in &files {
        assert!(
            !bytes.starts_with(&[0xFF, 0xD8, 0xFF]),
            "{} is a JPEG",
            path.display()
        );
    }
    // No card, event or notification names a host, a title or an application; nor does
    // any file (the database included), nor any log line of any thread of this binary.
    let cards = serde_json::to_string(&h.cards()).unwrap();
    let events = serde_json::to_string(&h.events()).unwrap();
    let notes = format!("{:?}", h.notifier.0.lock());
    let log = String::from_utf8_lossy(&captured_log().lock()).into_owned();
    let scoped_log = logged.join("");
    assert!(!h.cards().is_empty() && !h.events().is_empty());
    for text in [&cards, &events, &notes, &log, &scoped_log] {
        for word in ["pagina-", ".example", TAB_TITLE, "Google Chrome", CHROME] {
            assert!(!text.contains(word), "{word} in {text}");
        }
    }
    for (path, bytes) in &files {
        assert!(
            !contains(bytes, "pagina-"),
            "{} names a host",
            path.display()
        );
    }
    for text in [&log, &scoped_log] {
        for word in [
            "nofap_vision",
            "explicit",
            "suggestive",
            "verdict",
            UNKNOWN,
            KEYWORD,
        ] {
            assert!(!text.contains(word), "{word} in the log");
        }
        assert!(!text.contains(listed()));
    }
    // The stand-in notification was sent, and says nothing about the subject.
    assert!(h
        .notifier
        .0
        .lock()
        .iter()
        .all(|(title, body)| title == "UBI" && body == "Página bloqueada pelo UBI."));
    // What is stored about the checks: a counter per hour and the learned hashes.
    let stored: Vec<DayKind> = h.days().iter().map(|count| count.kind).collect();
    assert!(stored.iter().all(|kind| matches!(
        kind,
        DayKind::VisionCheck | DayKind::VisionHit | DayKind::BlockList
    )));
}

// ---------------------------------------------------------------------------------------------
// The service's own rules and the integrator's decisions
// ---------------------------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_password_prompt_that_stays_open_does_not_stop_the_guard() {
    let h = harness_with(Setup {
        os: Os::Linux,
        ..Setup::default()
    })
    .await;
    h.enable(false);
    let on = h
        .service
        .request(NofapChange::SetHosts { on: true })
        .unwrap();
    assert!(
        matches!(on, ChangeOutcome::Applied(_)),
        "tightening is immediate"
    );
    assert_eq!(h.service.status().hosts, TierState::NotApplied);

    // The operating system's prompt is open and nobody answers it.
    let release = h.shield.hold_apply();
    let service = h.service.clone();
    let applying = std::thread::spawn(move || service.system_apply());
    let started = std::time::Instant::now();
    while !h
        .shield
        .calls()
        .iter()
        .any(|call| matches!(call, SystemCall::Apply(_)))
    {
        assert!(
            started.elapsed() < Duration::from_secs(20),
            "apply never started"
        );
        std::thread::sleep(Duration::from_millis(5));
    }

    // Meanwhile a listed tab is still closed and the page still gets its answer. The
    // passes run on their own thread: a guard that waited for the prompt would otherwise
    // hang the suite instead of failing it.
    h.browse(listed());
    let (answered, answer) = std::sync::mpsc::channel();
    let service = h.service.clone();
    std::thread::spawn(move || {
        service.guard_pass().unwrap();
        let hit = service.guard_pass().unwrap().hit;
        let enabled = service.view().unwrap().config.enabled;
        let _ = answered.send((hit, enabled));
    });
    let meanwhile = answer.recv_timeout(Duration::from_secs(20));
    let still_open = !applying.is_finished();
    release.send(()).unwrap();
    assert_eq!(
        meanwhile,
        Ok((true, true)),
        "the guard and the page waited for the prompt"
    );
    assert_eq!(h.closes(), 1);
    assert!(still_open);

    assert_eq!(applying.join().unwrap().unwrap(), ApplyOutcome::Applied);
    assert_eq!(h.service.status().hosts, TierState::Applied);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_declined_prompt_leaves_the_module_on_and_the_tier_not_applied() {
    let h = harness_with(Setup {
        os: Os::Linux,
        ..Setup::default()
    })
    .await;
    h.shield.set_apply_outcome(Some(ApplyOutcome::Declined));
    let view = h.service.enable(with_hosts(false)).unwrap();
    assert!(view.config.enabled);
    assert_eq!(view.status.hosts, TierState::NotApplied);
    assert_eq!(
        view.status.overall,
        Overall::Reduced,
        "\"Proteção parcial\""
    );
    h.browse(listed());
    assert!(h.two_passes(), "L1 runs all the same");

    // The preview lists what "Aplicar" would do, and "Aplicar" then does it.
    let preview = h.service.system_preview().unwrap();
    assert!(preview.iter().any(|line| line.tier == Tier::Hosts));
    assert!(preview.iter().all(|line| {
        preview::LINES
            .iter()
            .any(|(key, tier, _)| *key == line.text_key && *tier == line.tier)
    }));
    h.shield.set_apply_outcome(None);
    assert_eq!(h.service.system_apply().unwrap(), ApplyOutcome::Applied);
    assert_eq!(h.service.status().hosts, TierState::Applied);
    assert!(h.service.system_applied());
    assert!(h.service.quit_dialog().message.contains("continua valendo"));

    // While the module is on, removing from the system is refused: that is what waits.
    assert!(matches!(
        h.service.system_remove(),
        Err(CoreError::Invalid(_))
    ));
    assert!(matches!(h.service.reset(), Err(CoreError::Invalid(_))));
    assert!(matches!(
        h.service.enable(options(false)),
        Err(CoreError::Invalid(_))
    ));
    assert!(h.service.enabled());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_gated_tier_is_shown_as_such_and_never_sent() {
    // A pair the gate table of this build still keeps closed (the table changes as the
    // systems' checks pass; when nothing is closed any more there is nothing gated to show).
    let closed = ubiqx_nofap::gate::TIER_GATE
        .iter()
        .find(|(_, _, open)| !open)
        .map(|(os, tier, _)| (*os, *tier));
    let Some((os, tier)) = closed else {
        return;
    };
    let h = harness_with(Setup {
        os,
        ..Setup::default()
    })
    .await;
    // What the real system layer reports for a closed tier: gated, and not overridden.
    let capability = |of: Tier| {
        if of == tier {
            TierCapability::Gated
        } else {
            TierCapability::Available
        }
    };
    h.shield.set_capabilities(SystemCapabilities {
        os: Some(os),
        hosts: capability(Tier::Hosts),
        resolver: capability(Tier::Resolver),
        policies: capability(Tier::Policies),
        unverified: Tier::ALL
            .into_iter()
            .filter(|other| *other != tier)
            .collect(),
    });
    let view = h
        .service
        .enable(EnableOptions {
            hosts: tier == Tier::Hosts,
            resolver: (tier == Tier::Resolver).then_some(ResolverId::CloudflareFamily),
            policies: tier == Tier::Policies,
            ..options(false)
        })
        .unwrap();
    let state = match tier {
        Tier::Hosts => view.status.hosts.clone(),
        Tier::Resolver => view.status.resolver.clone(),
        Tier::Policies => view.status.policies.clone(),
    };
    assert_eq!(state, TierState::Gated, "{os:?} {tier:?}");
    assert_eq!(
        view.status.overall,
        Overall::Protected,
        "L1 carries it; nothing chosen is broken"
    );
    assert!(!h
        .shield
        .calls()
        .iter()
        .any(|call| matches!(call, SystemCall::Apply(_))));
    assert_eq!(
        h.service.system_apply().unwrap(),
        ApplyOutcome::Unavailable(reason::GATED.into())
    );
    assert!(h.service.system_preview().unwrap().is_empty());
    assert!(!h
        .shield
        .calls()
        .iter()
        .any(|call| matches!(call, SystemCall::Apply(_) | SystemCall::Remove(..))));
    assert!(!h.service.system_applied());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn enabling_needs_the_tick_and_the_consent() {
    let h = harness().await;
    let no_tick = EnableOptions {
        self_use: false,
        ..options(false)
    };
    assert!(matches!(
        h.service.enable(no_tick),
        Err(CoreError::Invalid(_))
    ));
    let no_consent = EnableOptions {
        vision_consent: None,
        ..options(true)
    };
    assert!(matches!(
        h.service.enable(no_consent),
        Err(CoreError::Invalid(_))
    ));
    assert!(!h.service.enabled());
    assert_eq!(h.stored_config(), NofapConfig::default());

    let view = h.enable(true);
    assert!(view.config.enabled);
    assert_eq!(view.config.enabled_at, Some(fixed_now()));
    assert_eq!(view.config.self_use_at, Some(fixed_now()));
    assert_eq!(view.config.values, "Quero estar presente com quem eu amo.");
    assert_eq!(view.display_name, DISPLAY_NAME);
    assert_eq!(view.product, ProductId::Ubiqx);
    assert_eq!(
        (view.provider, view.ai),
        (AiProvider::Anthropic, AiReadiness::Ready)
    );
    let stored = h.stored_config();
    assert_eq!(stored.internal.salt_hex.len(), 32);
    // The salt never reaches the page.
    let wire = serde_json::to_string(&view).unwrap();
    assert!(!wire.contains(&stored.internal.salt_hex) && !wire.contains("internal"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn d22_the_same_request_is_not_queued_twice_and_tightening_never_waits() {
    let h = harness().await;
    h.enable(true);
    let ChangeOutcome::Pending(first) = h.service.request(NofapChange::Disable).unwrap() else {
        panic!("pending");
    };
    h.advance(120);
    let ChangeOutcome::Pending(again) = h.service.request(NofapChange::Disable).unwrap() else {
        panic!("pending");
    };
    assert_eq!(again.id, first.id, "the waiting request is answered");
    assert_eq!(
        again.ready_at, first.ready_at,
        "its clock did not start over"
    );
    assert_eq!(h.stored_config().pending.len(), 1);
    assert_eq!(h.counter(DayKind::OverrideRequest), 1, "counted once");

    // Each loosening waits; each tightening is immediate.
    for change in [
        NofapChange::SetVision { on: false },
        NofapChange::SetDelay { secs: 60 },
        NofapChange::AddAllow {
            host: UNKNOWN.into(),
        },
        NofapChange::ForgetLearned,
    ] {
        assert!(
            matches!(
                h.service.request(change.clone()).unwrap(),
                ChangeOutcome::Pending(_)
            ),
            "{change:?}"
        );
    }
    for change in [
        NofapChange::SetDelay { secs: 1_800 },
        NofapChange::AddBlock {
            host: UNKNOWN.into(),
        },
        NofapChange::SetValues {
            text: "Outra frase.".into(),
        },
    ] {
        assert!(
            matches!(
                h.service.request(change.clone()).unwrap(),
                ChangeOutcome::Applied(_)
            ),
            "{change:?}"
        );
    }
    let config = h.stored_config();
    assert!(config.vision.enabled && config.enabled);
    assert_eq!(config.delay_secs, 1_800);
    assert_eq!(config.pending.len(), 5);
    assert!(matches!(
        h.service.request(NofapChange::AddBlock {
            host: "não é um site".into()
        }),
        Err(CoreError::Invalid(_))
    ));
    assert!(matches!(
        h.service.request(NofapChange::RemoveBlock {
            id: "nobody".into()
        }),
        Err(CoreError::NotFound(_))
    ));
    assert!(matches!(
        h.service.cancel_pending("nobody"),
        Err(CoreError::NotFound(_))
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn d23_a_snoozed_host_is_looked_at_at_signal_cadence() {
    let h = harness().await;
    h.enable(true);
    h.browse(KEYWORD);
    assert!(h.two_passes());
    let card = h.cards().pop().unwrap();
    h.service.card_action(&card.id, CardAction::NotIt).unwrap();
    assert!(!h.pass().hit, "snoozed");
    h.advance(5);
    assert!(
        h.pass().job.is_some(),
        "five seconds in front are enough with a signal"
    );

    // An unknown site without a signal waits its fifteen seconds.
    h.advance(900);
    h.browse(UNKNOWN);
    h.pass();
    h.advance(5);
    assert!(h.pass().job.is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn d14_d21_view_answers_from_memory_and_reads_the_system_once_a_minute() {
    // A module nobody ever enabled: the page's question reads nothing at all.
    let h = harness_with(Setup {
        os: Os::Windows,
        ..Setup::default()
    })
    .await;
    let before = h.foreground.calls();
    let view = h.service.view().unwrap();
    assert_eq!(view.status, NofapStatus::off());
    assert!(h.shield.calls().is_empty());
    assert_eq!(h.foreground.calls(), before);

    h.service.enable(with_hosts(false)).unwrap();
    let observes = |h: &Harness| -> Vec<ObserveOpts> {
        h.shield
            .calls()
            .into_iter()
            .filter_map(|call| match call {
                SystemCall::Observe(opts) => Some(opts),
                _ => None,
            })
            .collect()
    };
    h.shield.clear_calls();
    assert_eq!(h.service.view().unwrap().status.hosts, TierState::Applied);
    let first = observes(&h);
    assert!(!first.is_empty(), "the page opening is a full read");
    assert!(first[0].full);
    for _ in 0..10 {
        h.service.view().unwrap();
    }
    assert_eq!(observes(&h), first, "at most one read a minute");
    h.advance(61);
    h.service.view().unwrap();
    assert!(observes(&h).len() > first.len());

    // The verify pass makes the quick read; the full one only every thirty minutes.
    h.shield.clear_calls();
    h.advance(120);
    h.service
        .verify_pass_after(Duration::from_secs(120))
        .unwrap();
    assert!(observes(&h).iter().all(|opts| !opts.full));
    h.shield.clear_calls();
    h.advance(1_800);
    h.service
        .verify_pass_after(Duration::from_secs(120))
        .unwrap();
    assert!(observes(&h).iter().any(|opts| opts.full));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn d11_the_canary_answer_is_carried_between_lookups() {
    let h = harness().await;
    let lookups = |h: &Harness| {
        h.shield
            .calls()
            .iter()
            .filter(|call| matches!(call, SystemCall::Observe(opts) if opts.canary))
            .count()
    };
    // Before the block is in the file the reserved name is never looked up.
    h.shield.clear_calls();
    h.service.enable(with_hosts(false)).unwrap();
    let observed: Vec<_> = h
        .shield
        .calls()
        .into_iter()
        .take_while(|call| !matches!(call, SystemCall::Apply(_)))
        .collect();
    assert!(!observed
        .iter()
        .any(|call| matches!(call, SystemCall::Observe(opts) if opts.canary)));
    // Right after the apply it is, and the block is honoured.
    assert_eq!(lookups(&h), 1);
    assert!(!h.service.status().issues.contains(&Issue::HostsNotHonoured));

    // Something resolves the canary to a real address (a browser's own DNS, say).
    h.shield.set_canary(Some(vec!["93.184.216.34".into()]));
    h.shield.clear_calls();
    h.advance(120);
    h.service
        .verify_pass_after(Duration::from_secs(120))
        .unwrap();
    assert_eq!(
        lookups(&h),
        0,
        "looked up every thirty minutes, not every pass"
    );
    assert!(!h.service.status().issues.contains(&Issue::HostsNotHonoured));
    h.advance(1_800);
    h.service
        .verify_pass_after(Duration::from_secs(120))
        .unwrap();
    assert_eq!(lookups(&h), 1);
    let status = h.service.status();
    assert!(status.issues.contains(&Issue::HostsNotHonoured));
    assert_eq!(status.overall, Overall::Reduced);

    // The passes in between do not look, and the issue does not come and go.
    h.shield.clear_calls();
    for _ in 0..3 {
        h.advance(120);
        h.service
            .verify_pass_after(Duration::from_secs(120))
            .unwrap();
        assert!(h.service.status().issues.contains(&Issue::HostsNotHonoured));
    }
    assert_eq!(lookups(&h), 0);

    // The next lookup finds it honoured again.
    h.shield.set_canary(None);
    h.advance(1_800);
    h.service
        .verify_pass_after(Duration::from_secs(120))
        .unwrap();
    assert!(!h.service.status().issues.contains(&Issue::HostsNotHonoured));
    assert_eq!(h.service.status().overall, Overall::Protected);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_host_of_the_applied_hosts_block_seen_in_front_is_a_bypass_suspicion() {
    let h = harness().await;
    h.service.enable(with_hosts(true)).unwrap();
    assert!(!h
        .service
        .status()
        .issues
        .contains(&Issue::DnsBypassSuspected));
    h.browse(listed());
    assert!(h.two_passes());
    let status = h.service.status();
    assert!(status.issues.contains(&Issue::DnsBypassSuspected));
    assert_eq!(status.overall, Overall::Reduced);
    // It also heightens the AI check: an unknown site is looked at after five seconds.
    h.browse(UNKNOWN);
    h.pass();
    h.advance(5);
    assert!(h.pass().job.is_some());
    // Fifteen minutes later the suspicion is over.
    h.advance(901);
    assert!(!h
        .service
        .view()
        .unwrap()
        .status
        .issues
        .contains(&Issue::DnsBypassSuspected));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_flows_are_counters_and_support_is_derived_from_them() {
    let h = harness().await;
    h.enable(true);
    h.service
        .request(NofapChange::SetPlans {
            plans: vec![IfThenPlan {
                when: "estiver sozinho à noite".into(),
                then: "sair para caminhar".into(),
            }],
        })
        .unwrap();
    let support = h.service.support().unwrap();
    assert_eq!(support.values, "Quero estar presente com quem eu amo.");
    assert_eq!(support.plans.len(), 1);
    assert_eq!(support.progress.window_days, 30);
    assert_eq!(
        support.progress.days, 1,
        "today, so far, went the way they wanted"
    );
    assert_eq!(
        support.progress.streak, None,
        "the streak is off by default"
    );
    assert!(!support.progress.after_reset && !support.help_due);
    assert_eq!(support.checkin_today, None);
    let ids: Vec<&str> = support
        .resources
        .iter()
        .map(|resource| resource.id.as_str())
        .collect();
    assert!(ids.contains(&"cvv") && ids.contains(&"samu"));

    h.service
        .record(UserEvent::Lapse {
            triggers: vec![Trigger::Tired, Trigger::Lonely],
        })
        .unwrap();
    h.service
        .record(UserEvent::Checkin {
            mood: Mood::Hard,
            triggers: vec![Trigger::Stressed],
        })
        .unwrap();
    for event in [
        UserEvent::SurfStarted,
        UserEvent::SurfDone,
        UserEvent::Panic,
    ] {
        h.service.record(event).unwrap();
    }
    for (kind, n) in [
        (DayKind::Lapse, 1),
        (DayKind::TriggerTired, 1),
        (DayKind::TriggerLonely, 1),
        (DayKind::CheckinHard, 1),
        (DayKind::TriggerStressed, 1),
        (DayKind::SurfStarted, 1),
        (DayKind::SurfDone, 1),
        (DayKind::Panic, 1),
    ] {
        assert_eq!(h.counter(kind), n, "{kind:?}");
    }
    let support = h.service.support().unwrap();
    assert_eq!(support.checkin_today, Some(Mood::Hard));
    assert_eq!(support.progress.days, 0);
    assert!(support.progress.after_reset);

    // "Sair da tela por 10 min" hides the other windows; nothing is locked or closed.
    h.service.record(UserEvent::StepAway).unwrap();
    assert_eq!(
        h.enforcer.calls(),
        vec![EnforcerCall::HideOthers("ai.ubiqx.app".into())]
    );
    // While that flow runs in the window a block does not replace it with a card.
    h.browse(listed());
    assert!(h.two_passes());
    assert!(h.cards().is_empty());
    h.advance(601);
    assert!(h.pass().hit);
    assert_eq!(h.cards_of(CardKind::Blocked).len(), 1);

    // Five lapses in a week: the help card is due, shown once, and counted.
    for _ in 0..4 {
        h.service
            .record(UserEvent::Lapse { triggers: vec![] })
            .unwrap();
    }
    assert!(h.service.support().unwrap().help_due);
    h.advance(120);
    h.service
        .verify_pass_after(Duration::from_secs(120))
        .unwrap();
    let help = h.cards_of(CardKind::Help);
    assert_eq!(help.len(), 1);
    assert_eq!(help[0].message_key, "help.pattern");
    assert_eq!(h.counter(DayKind::HelpShown), 1);
    assert!(
        !h.service.support().unwrap().help_due,
        "not again this week"
    );
    h.advance(120);
    h.service
        .verify_pass_after(Duration::from_secs(120))
        .unwrap();
    assert_eq!(h.cards_of(CardKind::Help).len(), 1);

    // Thirty days on: the reminder that the module is on, once.
    h.advance(30 * 86_400);
    h.service
        .verify_pass_after(Duration::from_secs(120))
        .unwrap();
    h.service
        .verify_pass_after(Duration::from_secs(120))
        .unwrap();
    let reminders = h.cards_of(CardKind::Reminder);
    assert_eq!(reminders.len(), 1);
    assert_eq!(reminders[0].actions, vec![CardAction::Ok, CardAction::Help]);

    // The preview card counts nothing.
    let days = h.days();
    h.service.test_card();
    assert_eq!(h.days(), days);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn d17_erasing_the_history_clears_counters_snoozes_and_what_the_person_wrote() {
    let h = harness().await;
    h.enable(true);
    h.service
        .request(NofapChange::SetPlans {
            plans: vec![IfThenPlan {
                when: "estiver cansado".into(),
                then: "dormir cedo".into(),
            }],
        })
        .unwrap();
    h.service
        .request(NofapChange::AddBlock {
            host: UNKNOWN_2.into(),
        })
        .unwrap();
    let card = positive_on_unknown(&h).await;
    assert_eq!(card.kind, CardKind::VisionClosed);
    h.advance(200);
    h.browse(KEYWORD);
    assert!(h.two_passes());
    let card = h.cards().pop().unwrap();
    h.service.card_action(&card.id, CardAction::NotIt).unwrap();
    h.service
        .record(UserEvent::Lapse { triggers: vec![] })
        .unwrap();
    let ChangeOutcome::Pending(waiting) = h.service.request(NofapChange::Disable).unwrap() else {
        panic!("pending");
    };
    assert!(!h.days().is_empty());

    h.service.erase_history().unwrap();

    assert!(h.days().is_empty(), "every counter");
    let sites = h.stored_sites();
    assert!(
        !sites.iter().any(|site| site.origin == SiteOrigin::Snooze),
        "every snooze"
    );
    assert!(
        sites.iter().any(|site| site.origin == SiteOrigin::User),
        "the user's list stays"
    );
    assert!(
        sites.iter().any(|site| site.origin == SiteOrigin::Learned),
        "learned sites stay"
    );
    let config = h.stored_config();
    assert_eq!(config.values, "");
    assert!(config.plans.is_empty());
    // Protection settings stay, and so does a waiting request: nothing got weaker.
    assert!(config.enabled && config.vision.enabled);
    assert_eq!(config.delay_secs, 600);
    assert_eq!(config.pending.len(), 1);
    assert_eq!(config.pending[0].id, waiting.id);
    assert!(h.two_passes(), "the snooze is gone: the rule blocks again");
    // The words are in no byte of the database any more.
    for (path, bytes) in h.files() {
        assert!(
            !contains(&bytes, "Quero estar presente"),
            "{}",
            path.display()
        );
        assert!(!contains(&bytes, "dormir cedo"), "{}", path.display());
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn export_and_reset() {
    let h = harness().await;
    h.enable(true);
    h.service
        .request(NofapChange::AddBlock {
            host: UNKNOWN_2.into(),
        })
        .unwrap();
    positive_on_unknown(&h).await;
    let export = h.service.export().unwrap();
    let path = PathBuf::from(&export.path);
    let name = path.file_name().unwrap().to_string_lossy().into_owned();
    assert!(
        name.starts_with("shield-") && name.ends_with(".json"),
        "{name}"
    );
    assert!(path.starts_with(h.tmp.path().join("data").join("exports")));
    let text = std::fs::read_to_string(&path).unwrap();
    let document: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(document["config"]["enabled"], true);
    assert_eq!(document["sites"]["blocked"][0]["host"], UNKNOWN_2);
    assert_eq!(document["sites"]["learned_count"], 1);
    assert!(document["days"]
        .as_array()
        .is_some_and(|days| !days.is_empty()));
    // No bookkeeping, no salt, and not the learned site.
    let salt = h.stored_config().internal.salt_hex;
    assert!(!text.contains("internal") && !text.contains(&salt));
    assert!(!text.contains(UNKNOWN));

    // "Apagar tudo" needs the module off; turning it off is what waits.
    assert!(h.service.reset().is_err());
    let ChangeOutcome::Pending(waiting) = h.service.request(NofapChange::Disable).unwrap() else {
        panic!("pending");
    };
    h.advance(600);
    h.observe_running(600);
    h.service.confirm_pending(&waiting.id).unwrap();
    h.service.reset().unwrap();
    assert_eq!(h.stored_config(), NofapConfig::default());
    assert!(h.stored_sites().is_empty() && h.days().is_empty());
    assert!(h.events().contains(&NofapEvent::SitesChanged));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_example_frame_is_the_last_candidate_window_never_the_apps_own() {
    let h = harness().await;
    h.enable(true);
    assert_eq!(h.service.sample_frame().unwrap(), None, "no candidate yet");
    h.browse(UNKNOWN);
    h.pass();
    // The page is in front now (the user pressed the button there).
    h.foreground
        .set(window("ubiqX", "ai.ubiqx.app", "ubiqX", Some(1)), None);
    h.pass();
    let frame = h
        .service
        .sample_frame()
        .unwrap()
        .expect("the browser window");
    assert_eq!((frame.mime.as_str(), frame.width), ("image/jpeg", 640));
    assert!(!frame.base64.is_empty());
    let (window_id, opts) = *h.frames.calls().last().unwrap();
    assert_eq!(window_id, 7, "the candidate, not the app's own window");
    assert!((opts.crop_top_frac - 0.12).abs() < f32::EPSILON);
    assert_eq!(h.detector.calls(), 0, "shown, never sent");
    assert_eq!(h.counter(DayKind::VisionCheck), 0);
    // The window is gone: the page says to open a site and come back.
    h.frames.set_fallback(None);
    assert_eq!(h.service.sample_frame().unwrap(), None);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_browser_without_a_readable_address_is_said_and_still_looked_at() {
    let h = harness().await;
    h.enable(true);
    h.foreground
        .set(window("Firefox", FIREFOX, "Uma página", Some(11)), None);
    h.pass();
    let status = h.service.status();
    assert_eq!(
        status.guard,
        GuardState::Limited {
            reason: GuardLimit::BrowserWithoutUrl
        }
    );
    assert_eq!(
        status.overall,
        Overall::Protected,
        "the AI check covers that window"
    );
    // Positive there: no tab can be named, so the window is hidden and the card asks.
    h.detector.push(Ok(explicit(Confidence::High)));
    assert_eq!(h.check_within(30).await, Some(true));
    assert_eq!(
        h.enforcer.calls(),
        vec![EnforcerCall::HideOthers("ai.ubiqx.app".into())]
    );
    assert_eq!(h.cards_of(CardKind::VisionAsk).len(), 1);

    // With the AI check off and no system tier, nothing protects that window: reduced.
    let h = harness().await;
    h.enable(false);
    h.foreground
        .set(window("Firefox", FIREFOX, "Uma página", Some(11)), None);
    h.pass();
    assert_eq!(h.service.status().overall, Overall::Reduced);
    h.browse(UNKNOWN);
    h.pass();
    assert_eq!(h.service.status().guard, GuardState::On);
    assert_eq!(h.service.status().overall, Overall::Protected);
}

// ---------------------------------------------------------------------------------------------
// The service without an engine (the standalone app's shape)
// ---------------------------------------------------------------------------------------------

struct StaticEnv;

impl NofapEnv for StaticEnv {
    fn snapshot(&self) -> NofapEnvSnapshot {
        NofapEnvSnapshot {
            language: UiLanguage::En,
            provider: AiProvider::OpenAi,
            vision_model: "a-vision-model".into(),
            ai: AiReadiness::Ready,
            excluded_apps: Vec::new(),
            excluded_domains: Vec::new(),
        }
    }
}

/// A repository whose stored configuration cannot be read, or whose writes fail, or
/// whose lists or counters cannot be reached. Counts every call it gets.
#[derive(Default)]
struct Flaky {
    inner: MemoryNofapRepo,
    unreadable: AtomicBool,
    fail_saves: AtomicBool,
    fail_lists: AtomicBool,
    fail_counters: AtomicBool,
    saves: AtomicUsize,
    calls: AtomicUsize,
}

impl Flaky {
    fn called(&self) {
        self.calls.fetch_add(1, Ordering::SeqCst);
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl NofapRepo for Flaky {
    fn load_config(&self) -> CoreResult<NofapConfig> {
        self.called();
        if self.unreadable.load(Ordering::SeqCst) {
            return Err(CoreError::Storage("line 1 column 9".into()));
        }
        self.inner.load_config()
    }

    fn save_config(&self, config: &NofapConfig) -> CoreResult<()> {
        self.called();
        if self.fail_saves.load(Ordering::SeqCst) {
            return Err(CoreError::Storage("disk full".into()));
        }
        self.saves.fetch_add(1, Ordering::SeqCst);
        self.inner.save_config(config)
    }

    fn list_sites(&self) -> CoreResult<Vec<NofapSite>> {
        self.called();
        if self.fail_lists.load(Ordering::SeqCst) {
            return Err(CoreError::Storage("database is locked".into()));
        }
        self.inner.list_sites()
    }

    fn upsert_site(&self, site: &NofapSite) -> CoreResult<()> {
        self.called();
        self.inner.upsert_site(site)
    }

    fn delete_site(&self, id: &str) -> CoreResult<()> {
        self.called();
        self.inner.delete_site(id)
    }

    fn delete_sites_by_origin(&self, origin: SiteOrigin) -> CoreResult<u64> {
        self.called();
        self.inner.delete_sites_by_origin(origin)
    }

    fn purge_expired_sites(&self, now: DateTime<Utc>) -> CoreResult<u64> {
        self.called();
        self.inner.purge_expired_sites(now)
    }

    fn bump_day(&self, day: &str, hour: u8, kind: DayKind, by: u32) -> CoreResult<()> {
        self.called();
        if self.fail_counters.load(Ordering::SeqCst) {
            return Err(CoreError::Storage("database is locked".into()));
        }
        self.inner.bump_day(day, hour, kind, by)
    }

    fn days_between(&self, from: &str, to: &str) -> CoreResult<Vec<DayCount>> {
        self.called();
        if self.fail_counters.load(Ordering::SeqCst) {
            return Err(CoreError::Storage("database is locked".into()));
        }
        self.inner.days_between(from, to)
    }

    fn delete_days_before(&self, day: &str) -> CoreResult<u64> {
        self.called();
        self.inner.delete_days_before(day)
    }

    fn erase_history(&self) -> CoreResult<()> {
        self.called();
        self.inner.erase_history()
    }
}

struct Standalone {
    service: Arc<NofapService>,
    repo: Arc<Flaky>,
    foreground: Arc<Foreground>,
    enforcer: Arc<MockEnforcer>,
    detector: Arc<FakeExplicitDetector>,
    notifier: Arc<RecordingNotifier>,
    clock: FixedClock,
    _tmp: tempfile::TempDir,
}

/// One `NofapService` built from ports alone, as the standalone app builds it: no
/// `Engine`, no engine state, no tracker.
fn standalone(repo: Arc<Flaky>) -> Standalone {
    standalone_with(repo, None, Duration::from_secs(5))
}

/// The same with another detector (the fake one when `None`) and another bound on its
/// answer.
fn standalone_with(
    repo: Arc<Flaky>,
    other_detector: Option<Arc<dyn ExplicitDetector>>,
    check_timeout: Duration,
) -> Standalone {
    captured_log();
    let tmp = tempfile::tempdir().unwrap();
    let foreground = Arc::new(Foreground::default());
    let enforcer = Arc::new(MockEnforcer::default());
    let detector = Arc::new(FakeExplicitDetector::new());
    let notifier = Arc::new(RecordingNotifier::default());
    let clock = FixedClock::new(fixed_now());
    let ports = NofapPorts {
        system: Arc::new(NoSystemShield),
        frames: Arc::new(ScriptedFrameSource::new()),
        detector: Some(other_detector.unwrap_or_else(|| detector.clone())),
        presenter: Arc::new(LogNofapPresenter),
        sink: Arc::new(NullNofapSink),
        repo: repo.clone(),
        shared_dir: None,
        tuning: Tuning {
            confirm_after: Duration::from_millis(5),
            check_timeout,
            background: Background::Inline,
            display: DisplaySession::Full,
        },
    };
    let host = NofapHost {
        env: Arc::new(StaticEnv),
        activity: foreground.clone(),
        urls: foreground.clone(),
        enforcer: enforcer.clone(),
        permissions: foreground.clone(),
        notifier: notifier.clone(),
        usage: Arc::new(ubiqx_ai::fake::MemoryUsageRepo::new()),
        clock: Arc::new(clock.clone()),
        identity: &ubiqx_core::product::SHIELD,
        export_dir: tmp.path().join("exports"),
    };
    Standalone {
        service: NofapService::new(NofapDeps::assemble(ports, host)),
        repo,
        foreground,
        enforcer,
        detector,
        notifier,
        clock,
        _tmp: tmp,
    }
}

fn standalone_options() -> EnableOptions {
    EnableOptions {
        vision_consent: Some(ConsentArgs {
            provider: AiProvider::OpenAi,
            managed: false,
        }),
        ..options(true)
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_service_runs_from_ports_alone_as_the_standalone_product() {
    let s = standalone(Arc::new(Flaky::default()));
    assert!(!s.service.enabled());
    let view = s.service.enable(standalone_options()).unwrap();
    assert_eq!(view.product, ProductId::Shield);
    assert_eq!(view.provider, AiProvider::OpenAi);
    // No system layer in this host: the tiers are off, and nothing pretends otherwise.
    assert_eq!(view.status.hosts, TierState::Off);
    assert_eq!(view.status.overall, Overall::Protected);

    let chrome = window("Google Chrome", CHROME, TAB_TITLE, Some(7));
    s.foreground.set(chrome.clone(), Some(&url_of(listed())));
    s.service.guard_pass().unwrap();
    assert!(s.service.guard_pass().unwrap().hit);
    assert_eq!(
        s.enforcer.calls(),
        vec![EnforcerCall::CloseActiveTab(CHROME.into())]
    );
    // No window: the neutral notification stands in, in the host's language.
    assert_eq!(
        s.notifier.0.lock().clone(),
        vec![("UBI".to_string(), "Page blocked by UBI.".to_string())]
    );

    // The AI check, with the standalone host's provider and model, and its own identity.
    s.foreground
        .set(window("VLC", VLC, "ferias.mp4", Some(9)), None);
    s.service.guard_pass().unwrap();
    s.clock.advance(ChronoDuration::seconds(25));
    let job = s.service.guard_pass().unwrap().job.expect("a check");
    s.detector.push(Ok(explicit(Confidence::High)));
    assert!(s.service.vision_check(job).await.unwrap().hit);
    assert_eq!(s.detector.models(), vec!["a-vision-model".to_string()]);
    assert!(s
        .enforcer
        .calls()
        .contains(&EnforcerCall::HideOthers("ai.ubiqx.shield".into())));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn d10_an_unreadable_configuration_is_a_degraded_state_never_default_off() {
    let repo = Arc::new(Flaky::default());
    repo.unreadable.store(true, Ordering::SeqCst);
    let s = standalone(repo);

    // Not "off": the page must not read as a module nobody enabled.
    assert_eq!(s.service.status().overall, Overall::Reduced);
    assert_eq!(s.service.view().unwrap().status.overall, Overall::Reduced);
    // Nothing is assumed about it: no pass, and nothing overwrites the stored row.
    s.foreground.set(
        window("Google Chrome", CHROME, TAB_TITLE, Some(7)),
        Some(&url_of(listed())),
    );
    let before = s.foreground.calls();
    s.service.guard_pass().unwrap();
    s.service.guard_pass().unwrap();
    s.service.verify_pass().unwrap();
    assert_eq!(s.foreground.calls(), before);
    for refused in [
        s.service
            .request(NofapChange::SetValues { text: "x".into() })
            .map(|_| ()),
        s.service.request(NofapChange::Disable).map(|_| ()),
        s.service
            .consent_vision(ConsentArgs {
                provider: AiProvider::OpenAi,
                managed: false,
            })
            .map(|_| ()),
        s.service.confirm_pending("x").map(|_| ()),
        s.service.cancel_pending("x").map(|_| ()),
        s.service.system_apply().map(|_| ()),
    ] {
        assert!(matches!(refused, Err(CoreError::Storage(_))), "{refused:?}");
    }
    assert_eq!(s.repo.saves.load(Ordering::SeqCst), 0);

    // The wizard is the way out: it writes a fresh configuration and the guard runs.
    s.service.enable(standalone_options()).unwrap();
    assert_eq!(s.repo.saves.load(Ordering::SeqCst), 1);
    assert!(s.service.guard_pass().is_ok());
    assert!(s.service.guard_pass().unwrap().hit);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn d10_a_configuration_that_cannot_be_stored_keeps_the_last_good_one_running() {
    let s = standalone(Arc::new(Flaky::default()));
    s.service.enable(standalone_options()).unwrap();
    s.repo.fail_saves.store(true, Ordering::SeqCst);
    assert!(matches!(
        s.service.request(NofapChange::Disable),
        Err(CoreError::Storage(_))
    ));
    assert!(s.service.enabled());
    s.foreground.set(
        window("Google Chrome", CHROME, TAB_TITLE, Some(7)),
        Some(&url_of(listed())),
    );
    s.service.guard_pass().unwrap();
    assert!(s.service.guard_pass().unwrap().hit, "still enforcing");
    // What is on disk is still the configuration that was last stored: on.
    assert!(s.repo.inner.load_config().unwrap().enabled);
}

// ---------------------------------------------------------------------------------------------
// The engine's loops
// ---------------------------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_engine_loop_enforces_spawns_the_check_and_scrubs_the_open_block() {
    captured_log();
    let tmp = tempfile::tempdir().unwrap();
    let store = Arc::new(SqliteStore::new(Db::open_in_memory().unwrap()));
    let foreground = Arc::new(Foreground::default());
    let enforcer = Arc::new(MockEnforcer::default());
    let detector = Arc::new(FakeExplicitDetector::new());
    let clock = FixedClock::new(fixed_now());
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
            enforcer: enforcer.clone(),
            presenter: Arc::new(NullInterventions),
        },
        repos: Repos::from_store(store.clone()),
        ai: AiPorts {
            requires_api_key: false,
            ..AiPorts::default()
        },
        sink: Arc::new(NullEventSink),
        clock: Arc::new(clock.clone()),
        data_dir: tmp.path().to_path_buf(),
        build: BuildInfo::dev(),
        update_feed_url: String::new(),
        license: LicenseDeps::default(),
        nofap: NofapPorts {
            frames: Arc::new(ScriptedFrameSource::new()),
            detector: Some(detector.clone()),
            repo: store.clone(),
            tuning: Tuning {
                confirm_after: Duration::from_millis(5),
                check_timeout: Duration::from_secs(5),
                background: Background::Inline,
                display: DisplaySession::Full,
            },
            ..NofapPorts::inert()
        },
        identity: &ubiqx_core::product::UBIQX,
    };
    let config = LoopConfig {
        nofap_every: Duration::from_millis(15),
        ..parked()
    };
    let (handle, _samples) = Engine::start_with(deps, config).unwrap();
    let mut settings = handle.settings();
    settings.onboarding_done = true;
    handle.update_settings(settings).unwrap();
    let wait_for = |what: &str, done: &dyn Fn() -> bool| {
        let started = std::time::Instant::now();
        while !done() {
            assert!(started.elapsed() < Duration::from_secs(20), "{what}");
            std::thread::sleep(Duration::from_millis(10));
        }
    };

    // Off: the loop ticks and touches nothing.
    foreground.set(
        window("Google Chrome", CHROME, TAB_TITLE, Some(7)),
        Some(&url_of(listed())),
    );
    std::thread::sleep(Duration::from_millis(120));
    assert_eq!(foreground.calls(), 0);

    // On: the loop closes the listed tab by itself.
    handle.nofap().enable(options(true)).unwrap();
    wait_for("the loop never closed the tab", &|| {
        enforcer
            .calls()
            .contains(&EnforcerCall::CloseActiveTab(CHROME.into()))
    });

    // The tracker has a block open on a player (fed while a window the guard leaves alone
    // is in front, so no pass acts meanwhile).
    foreground.set(window("Finder", "com.apple.finder", "", Some(2)), None);
    std::thread::sleep(Duration::from_millis(200));
    let player = window("VLC", VLC, "ferias.mp4", Some(9));
    let state = handle.state().clone();
    for _ in 0..3 {
        clock.advance(ChronoDuration::seconds(30));
        let sample = ActivitySample {
            at: clock.now(),
            app_name: player.app_name.clone(),
            app_id: player.app_id.clone(),
            window_title: player.window_title.clone(),
            url: None,
            idle_secs: Some(0.0),
            window_id: player.window_id,
        };
        ubiqx_engine::tracker::handle_sample(&state, sample).unwrap();
    }
    let open = BlockRepo::open_block(store.as_ref()).unwrap().unwrap();
    assert_eq!((open.title.as_str(), open.sample_count), ("ferias.mp4", 3));

    // The player comes to the front; the loop's check answers positive, runs on a task of
    // its own, and the loop scrubs that block.
    foreground.set(player.clone(), None);
    detector.set_fallback(Some(explicit(Confidence::High)));
    wait_for("the loop never ran the check", &|| {
        clock.advance(ChronoDuration::seconds(5));
        detector.calls() >= 1
    });
    wait_for("the open block was never scrubbed", &|| {
        BlockRepo::get(store.as_ref(), &open.id)
            .unwrap()
            .is_some_and(|block| block.title == PRIVATE_TITLE && !block.is_open)
    });
    assert!(enforcer
        .calls()
        .contains(&EnforcerCall::HideOthers("ai.ubiqx.app".into())));
    handle.shutdown();
}

/// A detector whose answer does not come until the test lets it.
struct HeldDetector {
    asked: AtomicUsize,
    answer: tokio::sync::Semaphore,
    /// What it answers once the test lets it.
    verdict: ExplicitCheck,
}

#[async_trait::async_trait]
impl ExplicitDetector for HeldDetector {
    async fn check(&self, _image: &EncodedImage, _model: &str) -> CoreResult<ExplicitCheck> {
        self.asked.fetch_add(1, Ordering::SeqCst);
        let _released = self.answer.acquire().await;
        Ok(self.verdict)
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_loop_keeps_closing_tabs_while_a_check_waits_for_its_answer() {
    let held = Arc::new(HeldDetector {
        asked: AtomicUsize::new(0),
        answer: tokio::sync::Semaphore::new(0),
        verdict: clean(),
    });
    let s = standalone_with(
        Arc::new(Flaky::default()),
        Some(held.clone()),
        Duration::from_secs(120),
    );
    s.service.enable(standalone_options()).unwrap();
    let wait_for = |what: &str, done: &dyn Fn() -> bool| {
        let started = std::time::Instant::now();
        while !done() {
            assert!(started.elapsed() < Duration::from_secs(20), "{what}");
            std::thread::sleep(Duration::from_millis(10));
        }
    };

    // A player in front long enough: the loop starts a check, and the provider is slow.
    s.foreground
        .set(window("VLC", VLC, "ferias.mp4", Some(9)), None);
    let cancel = tokio_util::sync::CancellationToken::new();
    let running = tokio::spawn(ubiqx_engine::nofap::run_guard_loop(
        s.service.clone(),
        Duration::from_millis(15),
        cancel.clone(),
        Arc::new(|| {}),
    ));
    wait_for("the loop never started a check", &|| {
        s.clock.advance(ChronoDuration::seconds(5));
        held.asked.load(Ordering::SeqCst) >= 1
    });

    // The answer is still out and a listed tab comes to the front: closed all the same.
    s.foreground.set(
        window("Google Chrome", CHROME, TAB_TITLE, Some(7)),
        Some(&url_of(listed())),
    );
    wait_for("the guard waited for the check", &|| {
        s.enforcer
            .calls()
            .contains(&EnforcerCall::CloseActiveTab(CHROME.into()))
    });
    assert_eq!(held.asked.load(Ordering::SeqCst), 1, "one check at a time");

    held.answer.add_permits(1);
    cancel.cancel();
    tokio::time::timeout(Duration::from_secs(20), running)
        .await
        .expect("the loop ends when cancelled")
        .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_rejected_key_switches_the_check_off_until_the_key_changes_or_a_while_passed() {
    let h = harness_with(Setup {
        needs_key: true,
        ..Setup::default()
    })
    .await;
    h.enable(true);
    let health = h.state().ai_health();
    for host in [UNKNOWN, UNKNOWN_2, "terceiro-sitio.example"] {
        h.browse(host);
        h.detector
            .push(Err(CoreError::AiRejected("no credit".into())));
        assert_eq!(h.check_within(30).await, Some(false));
    }
    assert_eq!(h.service.vision_ready(), Err(VisionOff::NoKey));
    let status = h.service.view().unwrap().status;
    assert!(
        status.issues.contains(&Issue::VisionNoProvider),
        "the calm note on the page"
    );
    assert_eq!(
        h.state().ai_health(),
        health,
        "the productivity AI is not told"
    );
    h.browse("quarto-sitio.example");
    assert!(h.job_within(60).is_none());
    assert!(h.cards().is_empty() && h.enforcer.calls().is_empty());

    // A new key through ubiqX's settings: ready again at once.
    h.handle
        .set_api_key(AiProvider::Anthropic, Some("sk-test-1111"))
        .unwrap();
    assert_eq!(h.service.vision_ready(), Ok(()));

    // Without a new key (the user fixed it at the provider's): tried again after a while.
    for host in [
        "quinto-sitio.example",
        "sexto-sitio.example",
        "setimo-sitio.example",
    ] {
        h.browse(host);
        h.detector
            .push(Err(CoreError::AiRejected("no credit".into())));
        assert_eq!(h.check_within(30).await, Some(false));
    }
    assert_eq!(h.service.vision_ready(), Err(VisionOff::NoKey));
    h.browse("oitavo-sitio.example");
    assert!(h.job_within(600).is_none());
    h.advance(400);
    h.detector.push(Ok(clean()));
    assert_eq!(h.check_within(60).await, Some(false), "tried again");
    assert_eq!(h.service.vision_ready(), Ok(()));

    // Removing the key stops the check at once.
    h.handle.set_api_key(AiProvider::Anthropic, None).unwrap();
    assert_eq!(h.service.vision_ready(), Err(VisionOff::NoKey));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn after_a_restart_everything_that_was_stored_is_back() {
    let h = harness_with(Setup {
        os: Os::Linux,
        ..Setup::default()
    })
    .await;
    h.service.enable(with_hosts(true)).unwrap();
    h.service
        .request(NofapChange::AddBlock {
            host: UNKNOWN_2.into(),
        })
        .unwrap();
    positive_on_unknown(&h).await;
    let ChangeOutcome::Pending(waiting) = h.service.request(NofapChange::Disable).unwrap() else {
        panic!("pending");
    };

    // A second service over the same database and the same machine: the next start.
    let enforcer = Arc::new(MockEnforcer::default());
    let ports = NofapPorts {
        system: h.shield.clone(),
        frames: h.frames.clone(),
        detector: Some(h.detector.clone()),
        presenter: h.presenter.clone(),
        sink: h.sink.clone(),
        repo: h.store.clone(),
        shared_dir: None,
        tuning: Tuning {
            confirm_after: Duration::from_millis(5),
            check_timeout: Duration::from_secs(5),
            background: Background::Inline,
            display: DisplaySession::Full,
        },
    };
    let host = NofapHost {
        env: Arc::new(StaticEnv),
        activity: h.foreground.clone(),
        urls: h.foreground.clone(),
        enforcer: enforcer.clone(),
        permissions: h.foreground.clone(),
        notifier: h.notifier.clone(),
        usage: h.store.clone(),
        clock: Arc::new(h.clock.clone()),
        identity: &ubiqx_core::product::UBIQX,
        export_dir: h.tmp.path().join("exports"),
    };
    let restarted = NofapService::new(NofapDeps::assemble(ports, host));
    assert!(restarted.enabled());
    // The quit dialog knows what is applied before any pass ran.
    h.shield.clear_calls();
    assert!(restarted.system_applied());
    assert!(h
        .shield
        .calls()
        .iter()
        .all(|call| matches!(call, SystemCall::Observe(_))));
    // The lists: embedded, the user's and the learned one.
    for host in [listed(), UNKNOWN_2, UNKNOWN] {
        h.browse(host);
        restarted.guard_pass().unwrap();
        assert!(restarted.guard_pass().unwrap().hit, "{host}");
    }
    assert_eq!(enforcer.calls().len(), 3);
    // The waiting request and the day's checks.
    let view = restarted.view().unwrap();
    assert_eq!(view.status.pending.len(), 1);
    assert_eq!(view.status.pending[0].id, waiting.id);
    assert_eq!(view.status.overall, Overall::PendingOff);
    assert_eq!(view.today.checks, 1);
    assert_eq!(restarted.sites().learned_count, 1);
    // The running time a request needs is counted again from what was stored.
    assert!(restarted.confirm_pending(&waiting.id).is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_reader_or_source_view_of_a_blocked_page_is_closed_and_names_no_host() {
    let h = harness().await;
    h.enable(false);
    let views = [
        format!("view-source:https://{}/pagina", listed()),
        format!("chrome-distiller://abc123/?url=https://{}/pagina", listed()),
        // The address bar shows it without a scheme, with another address in its query.
        format!("{}/entrar?next=https://{UNKNOWN}/", listed()),
        format!("view-source:https://{KEYWORD}/"),
    ];
    for (index, url) in views.iter().enumerate() {
        h.advance(200);
        h.foreground.set(
            window("Google Chrome", CHROME, TAB_TITLE, Some(7)),
            Some(url),
        );
        assert!(h.two_passes(), "{url}");
        assert_eq!(h.closes(), index + 1, "{url}");
    }
    // The card of the keyword block in a source view has no correction button: the text
    // that stands where a host would is not one, and no request may name it.
    let card = h.cards().pop().unwrap();
    assert_eq!(card.actions, vec![CardAction::Support, CardAction::Ok]);
    // The tracker records such a sample as private, like any blocked page.
    let sample = ActivitySample {
        at: h.clock.now(),
        app_name: "Google Chrome".into(),
        app_id: CHROME.into(),
        window_title: TAB_TITLE.into(),
        url: Some(views[0].clone()),
        idle_secs: Some(0.0),
        window_id: Some(7),
    };
    assert!(h.service.redacts(&sample));

    // A stale fresh read protects these too: the view moved on, nothing is closed.
    h.advance(200);
    h.foreground.set(
        window("Google Chrome", CHROME, TAB_TITLE, Some(7)),
        Some(&views[0]),
    );
    *h.foreground.fresh.lock() = Some(Some(format!("view-source:https://{UNKNOWN}/")));
    let closes = h.closes();
    assert!(!h.two_passes());
    assert_eq!(h.closes(), closes);

    // Browser-internal pages are not sites.
    *h.foreground.fresh.lock() = None;
    for url in [
        "about:blank",
        "chrome://settings/",
        "file:///home/ana/notas.html",
    ] {
        h.foreground.set(
            window("Google Chrome", CHROME, TAB_TITLE, Some(7)),
            Some(url),
        );
        assert!(!h.two_passes(), "{url}");
    }
    assert_eq!(h.closes(), closes);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn names_that_stop_resolving_offer_to_give_the_network_back_and_change_nothing() {
    let h = harness().await;
    h.service
        .enable(EnableOptions {
            resolver: Some(ResolverId::CloudflareFamily),
            ..options(false)
        })
        .unwrap();
    assert_eq!(h.service.status().resolver, TierState::Applied);

    // The probe name fails on two looks: not yet an outage (a laptop lid, a tunnel).
    h.shield.set_probe_ok(false);
    for _ in 0..2 {
        h.advance(120);
        h.service
            .verify_pass_after(Duration::from_secs(120))
            .unwrap();
    }
    assert!(!h
        .service
        .status()
        .issues
        .contains(&Issue::ResolverUnreachable));
    assert!(h.cards().is_empty());

    // The third in a row: the issue, and one card that offers the way out.
    h.shield.clear_calls();
    for _ in 0..3 {
        h.advance(120);
        h.service
            .verify_pass_after(Duration::from_secs(120))
            .unwrap();
    }
    let status = h.service.status();
    assert!(status.issues.contains(&Issue::ResolverUnreachable));
    assert_eq!(status.overall, Overall::Reduced);
    let cards = h.cards_of(CardKind::ResolverUnreachable);
    assert_eq!(cards.len(), 1, "once per outage");
    assert_eq!(cards[0].message_key, "resolver.unreachable");
    assert_eq!(
        cards[0].actions,
        vec![CardAction::RestoreNetwork, CardAction::Later]
    );
    assert!(
        h.shield
            .calls()
            .iter()
            .all(|call| matches!(call, SystemCall::Observe(_))),
        "nothing changes by itself"
    );
    assert_eq!(h.service.status().resolver, TierState::Applied);

    // The button: the resolver goes, undelayed, and the card closes.
    h.service
        .card_action(&cards[0].id, CardAction::RestoreNetwork)
        .unwrap();
    assert!(h
        .shield
        .calls()
        .contains(&SystemCall::Remove(ProductId::Ubiqx, RemoveScope::Resolver)));
    let status = h.service.status();
    assert_eq!(status.resolver, TierState::NotApplied);
    assert!(!status.issues.contains(&Issue::ResolverUnreachable));
    assert!(h.events().contains(&NofapEvent::CardClosed {
        id: cards[0].id.clone()
    }));
}

// ---------------------------------------------------------------------------------------------
// Review: what the plan does not list (a failing store, a module switched off in the middle of
// a pass, an answer that comes late, a clock that jumps, the same thing asked twice)
// ---------------------------------------------------------------------------------------------

/// The host's settings as a test changes them.
struct SwitchEnv {
    ai: Mutex<AiReadiness>,
    calls: AtomicUsize,
}

impl NofapEnv for SwitchEnv {
    fn snapshot(&self) -> NofapEnvSnapshot {
        self.calls.fetch_add(1, Ordering::SeqCst);
        NofapEnvSnapshot {
            language: UiLanguage::En,
            provider: AiProvider::OpenAi,
            vision_model: "a-vision-model".into(),
            ai: *self.ai.lock(),
            excluded_apps: Vec::new(),
            excluded_domains: Vec::new(),
        }
    }
}

/// The pretend machine, except that taking the tiers off fails (the helper is gone, the
/// prompt cannot be raised).
struct FailingRemove(Arc<MockSystemShield>);

impl SystemShield for FailingRemove {
    fn capabilities(&self) -> SystemCapabilities {
        self.0.capabilities()
    }

    fn observe(&self, opts: ObserveOpts) -> CoreResult<SystemObservation> {
        self.0.observe(opts)
    }

    fn preview(&self, plan: &SystemPlan) -> CoreResult<Vec<PlannedChange>> {
        self.0.preview(plan)
    }

    fn apply(&self, plan: &SystemPlan) -> CoreResult<ApplyOutcome> {
        self.0.apply(plan)
    }

    fn remove(&self, _owner: ProductId, _scope: RemoveScope) -> CoreResult<ApplyOutcome> {
        Err(CoreError::Platform(
            "the helper could not be started".into(),
        ))
    }
}

#[derive(Default)]
struct RigSetup {
    repo: Arc<Flaky>,
    /// Another detector than the fake one.
    detector: Option<Arc<dyn ExplicitDetector>>,
    failing_remove: bool,
    /// A directory for the primary lock.
    shared: bool,
}

/// The service built from ports alone, with every port a review test scripts or reads.
struct Rig {
    service: Arc<NofapService>,
    repo: Arc<Flaky>,
    foreground: Arc<Foreground>,
    enforcer: Arc<MockEnforcer>,
    frames: Arc<ScriptedFrameSource>,
    detector: Arc<FakeExplicitDetector>,
    shield: Arc<MockSystemShield>,
    presenter: Arc<RecordingPresenter>,
    sink: Arc<RecordingSink>,
    env: Arc<SwitchEnv>,
    clock: FixedClock,
    tmp: tempfile::TempDir,
}

fn rig(setup: RigSetup) -> Rig {
    captured_log();
    let tmp = tempfile::tempdir().unwrap();
    let foreground = Arc::new(Foreground::default());
    let enforcer = Arc::new(MockEnforcer::default());
    let frames = Arc::new(ScriptedFrameSource::new());
    let detector = Arc::new(FakeExplicitDetector::new());
    let shield = Arc::new(MockSystemShield::new(Os::Linux));
    shield.set_capabilities(SystemCapabilities {
        os: Some(Os::Linux),
        hosts: TierCapability::Available,
        resolver: TierCapability::Available,
        policies: TierCapability::Available,
        unverified: Tier::ALL.to_vec(),
    });
    let system: Arc<dyn SystemShield> = if setup.failing_remove {
        Arc::new(FailingRemove(shield.clone()))
    } else {
        shield.clone()
    };
    let presenter = Arc::new(RecordingPresenter {
        shows: AtomicBool::new(true),
        seen: Mutex::new(Vec::new()),
    });
    let sink = Arc::new(RecordingSink::default());
    let env = Arc::new(SwitchEnv {
        ai: Mutex::new(AiReadiness::Ready),
        calls: AtomicUsize::new(0),
    });
    let clock = FixedClock::new(fixed_now());
    let ports = NofapPorts {
        system,
        frames: frames.clone(),
        detector: Some(setup.detector.unwrap_or_else(|| detector.clone())),
        presenter: presenter.clone(),
        sink: sink.clone(),
        repo: setup.repo.clone(),
        shared_dir: setup.shared.then(|| tmp.path().join("shared")),
        tuning: Tuning {
            confirm_after: Duration::from_millis(5),
            check_timeout: Duration::from_secs(60),
            background: Background::Inline,
            display: DisplaySession::Full,
        },
    };
    let host = NofapHost {
        env: env.clone(),
        activity: foreground.clone(),
        urls: foreground.clone(),
        enforcer: enforcer.clone(),
        permissions: foreground.clone(),
        notifier: Arc::new(RecordingNotifier::default()),
        usage: Arc::new(ubiqx_ai::fake::MemoryUsageRepo::new()),
        clock: Arc::new(clock.clone()),
        identity: &ubiqx_core::product::SHIELD,
        export_dir: tmp.path().join("exports"),
    };
    Rig {
        service: NofapService::new(NofapDeps::assemble(ports, host)),
        repo: setup.repo,
        foreground,
        enforcer,
        frames,
        detector,
        shield,
        presenter,
        sink,
        env,
        clock,
        tmp,
    }
}

impl Rig {
    fn browse(&self, host: &str) {
        self.foreground.set(
            window("Google Chrome", CHROME, TAB_TITLE, Some(7)),
            Some(&url_of(host)),
        );
    }

    fn player(&self) {
        self.foreground
            .set(window("VLC", VLC, "ferias.mp4", Some(9)), None);
    }

    fn pass(&self) -> GuardOutcome {
        self.service.guard_pass().unwrap()
    }

    /// A pass every five seconds until the sampler asks for a check.
    fn job_within(&self, within: i64) -> Option<VisionJob> {
        let mut waited = 0;
        loop {
            if let Some(job) = self.pass().job {
                return Some(job);
            }
            if waited >= within {
                return None;
            }
            self.clock.advance(ChronoDuration::seconds(5));
            waited += 5;
        }
    }

    fn cards_of(&self, kind: CardKind) -> Vec<NofapCard> {
        self.presenter
            .seen
            .lock()
            .iter()
            .filter(|card| card.kind == kind)
            .cloned()
            .collect()
    }

    fn counter(&self, kind: DayKind) -> u32 {
        self.repo
            .inner
            .days_between("0000-01-01", "9999-12-31")
            .unwrap()
            .iter()
            .filter(|count| count.kind == kind)
            .map(|count| count.n)
            .sum()
    }

    /// Asks to turn the module off; with no delay the request is ready at once and still
    /// waits for its confirmation.
    fn ask_off(&self) -> PendingView {
        match self.service.request(NofapChange::Disable).unwrap() {
            ChangeOutcome::Pending(waiting) => waiting,
            ChangeOutcome::Applied(_) => panic!("turning it off is never immediate"),
        }
    }
}

fn no_delay() -> EnableOptions {
    EnableOptions {
        delay_secs: 0,
        ..standalone_options()
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_change_that_cannot_be_stored_is_not_made_in_memory_either() {
    let r = rig(RigSetup::default());
    r.service.enable(no_delay()).unwrap();
    let waiting = r.ask_off();
    assert!(waiting.ready);

    // The disk refuses: the confirmation did not happen, in memory either. The request
    // still waits and the module is still on.
    r.repo.fail_saves.store(true, Ordering::SeqCst);
    assert!(matches!(
        r.service.confirm_pending(&waiting.id),
        Err(CoreError::Storage(_))
    ));
    assert!(r.service.enabled());
    // Neither did a sentence, a request that would wait, or a cancellation.
    for refused in [
        r.service
            .request(NofapChange::SetValues {
                text: "outra frase".into(),
            })
            .map(|_| ()),
        r.service
            .request(NofapChange::SetVision { on: false })
            .map(|_| ()),
        r.service.cancel_pending(&waiting.id).map(|_| ()),
        r.service
            .consent_vision(ConsentArgs {
                provider: AiProvider::Anthropic,
                managed: false,
            })
            .map(|_| ()),
    ] {
        assert!(matches!(refused, Err(CoreError::Storage(_))), "{refused:?}");
    }
    let view = r.service.view().unwrap();
    assert_eq!(view.config.values, no_delay().values);
    assert_eq!(view.config.pending.len(), 1);
    assert_eq!(
        view.config
            .vision
            .consent
            .as_ref()
            .map(|consent| consent.provider),
        Some(AiProvider::OpenAi)
    );
    assert_eq!(view.status.overall, Overall::PendingOff);
    assert_eq!(
        ubiqx_core::nofap_ipc::ConfigView::from(&r.repo.inner.load_config().unwrap()),
        view.config,
        "memory and disk say the same"
    );
    r.browse(listed());
    r.pass();
    assert!(r.pass().hit, "still enforcing");

    // The disk works again: the same confirmation goes through.
    r.repo.fail_saves.store(false, Ordering::SeqCst);
    let view = r.service.confirm_pending(&waiting.id).unwrap();
    assert!(!view.config.enabled);
    assert!(!r.repo.inner.load_config().unwrap().enabled);
    assert!(!r.pass().hit);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_verify_pass_that_cannot_store_still_reads_the_machine_and_shows_its_card() {
    let r = rig(RigSetup::default());
    let view = r
        .service
        .enable(EnableOptions {
            hosts: true,
            ..standalone_options()
        })
        .unwrap();
    assert_eq!(view.status.hosts, TierState::Applied);
    // Something waits, so every pass has running time to credit: a write.
    r.service
        .request(NofapChange::SetVision { on: false })
        .unwrap();
    r.shield
        .edit_observation(|machine| machine.hosts.text = Some("127.0.0.1 localhost\n".into()));

    r.repo.fail_saves.store(true, Ordering::SeqCst);
    let pass = r.service.verify_pass_after(Duration::from_secs(120));
    assert!(
        matches!(pass, Err(CoreError::Storage(_))),
        "the failure is reported: {pass:?}"
    );
    // The pass went on all the same: the machine was read, the page is told, the card is
    // shown (a card that was kept and never shown would hold back the next one).
    let status = r.service.status();
    assert_eq!(status.hosts, TierState::Missing);
    assert_eq!(status.overall, Overall::Reduced);
    assert!(r.sink.0.lock().iter().any(
        |event| matches!(event, NofapEvent::Status { status } if status.hosts == TierState::Missing)
    ));
    assert_eq!(r.cards_of(CardKind::Drift).len(), 1);
    assert_eq!(r.counter(DayKind::Drift), 1);
    // Running time that could not be stored was not credited in memory only.
    assert_eq!(status.pending[0].remaining_secs, 600);

    // The same drift does not say it again, also while its time cannot be stored.
    r.clock.advance(ChronoDuration::seconds(120));
    assert!(r
        .service
        .verify_pass_after(Duration::from_secs(120))
        .is_err());
    assert_eq!(r.cards_of(CardKind::Drift).len(), 1);

    // The disk works again: the pass succeeds and the wait is credited.
    r.repo.fail_saves.store(false, Ordering::SeqCst);
    r.service
        .verify_pass_after(Duration::from_secs(120))
        .unwrap();
    assert_eq!(
        r.repo.inner.load_config().unwrap().pending[0].observed_secs,
        120
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn turning_it_off_is_done_whatever_the_removal_answers() {
    let r = rig(RigSetup {
        failing_remove: true,
        ..RigSetup::default()
    });
    let view = r
        .service
        .enable(EnableOptions {
            hosts: true,
            ..no_delay()
        })
        .unwrap();
    assert_eq!(view.status.hosts, TierState::Applied);
    let waiting = r.ask_off();
    r.sink.0.lock().clear();

    // The removal fails outright. The module is off all the same, the page is told, and
    // the tier that is still on the machine keeps showing so it can be removed later.
    let view = r.service.confirm_pending(&waiting.id).unwrap();
    assert!(!view.config.enabled);
    assert_eq!(view.status.overall, Overall::Off);
    assert_eq!(view.status.hosts, TierState::Applied);
    let events = r.sink.0.lock().clone();
    assert!(events.contains(&NofapEvent::PendingChanged));
    assert!(events.iter().any(
        |event| matches!(event, NofapEvent::Status { status } if status.overall == Overall::Off)
    ));
    assert!(!r.repo.inner.load_config().unwrap().enabled);
    r.browse(listed());
    let before = r.foreground.calls();
    assert!(!r.pass().hit && !r.pass().hit);
    assert_eq!(r.foreground.calls(), before, "off: no platform call");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switched_off_in_the_middle_of_a_pass_nothing_is_closed_and_nothing_is_shown() {
    let r = rig(RigSetup {
        shared: true,
        ..RigSetup::default()
    });
    let lock = r.tmp.path().join("shared").join(primary::LOCK_FILE);
    r.service.enable(no_delay()).unwrap();
    r.browse(listed());
    assert!(!r.pass().hit, "the first of two passes");
    assert!(lock.exists(), "this product runs the passes");

    // The second pass has read the window and is about to act when the confirmation of
    // "turn it off" arrives.
    let waiting = r.ask_off();
    let service = r.service.clone();
    *r.foreground.during_fresh.lock() = Some(Box::new(move || {
        service.confirm_pending(&waiting.id).unwrap();
    }));
    let outcome = r.pass();
    assert!(!r.service.enabled());
    assert!(!outcome.hit && outcome.job.is_none());
    assert!(
        r.enforcer.calls().is_empty(),
        "an off module closes nothing"
    );
    assert!(r.presenter.seen.lock().is_empty(), "and shows nothing");
    assert_eq!(r.counter(DayKind::BlockList), 0);
    assert!(!lock.exists(), "and leaves no heartbeat behind");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn switched_on_and_off_over_and_over_while_the_passes_run() {
    let r = rig(RigSetup {
        shared: true,
        ..RigSetup::default()
    });
    let lock = r.tmp.path().join("shared").join(primary::LOCK_FILE);
    r.browse(listed());

    // One thread runs the two passes as fast as it can, as the loops do.
    let stop = Arc::new(AtomicBool::new(false));
    let passes = {
        let (service, stop) = (r.service.clone(), stop.clone());
        std::thread::spawn(move || {
            let mut made = 0u32;
            while !stop.load(Ordering::SeqCst) {
                service.guard_pass().unwrap();
                service.verify_pass_after(Duration::from_secs(1)).unwrap();
                made += 1;
            }
            made
        })
    };
    // The other switches the module on and off, forty times.
    for _ in 0..40 {
        r.service.enable(no_delay()).unwrap();
        let waiting = r.ask_off();
        let view = r.service.confirm_pending(&waiting.id).unwrap();
        assert!(!view.config.enabled);
    }
    stop.store(true, Ordering::SeqCst);
    let made = passes.join().expect("no pass panicked");
    assert!(made > 0);

    // It ends off, and off means untouched: no pass, no lock, nothing waiting.
    assert!(!r.service.enabled());
    assert_eq!(r.service.status().overall, Overall::Off);
    assert!(!lock.exists(), "an off module holds no lock");
    let before = (r.foreground.calls(), r.enforcer.calls().len());
    assert!(!r.pass().hit);
    r.service.verify_pass().unwrap();
    assert_eq!((r.foreground.calls(), r.enforcer.calls().len()), before);
    assert!(r.repo.inner.load_config().unwrap().pending.is_empty());

    // And on again it works as on the first day.
    r.service.enable(no_delay()).unwrap();
    r.pass();
    assert!(r.pass().hit);
    assert!(lock.exists());
}

/// A detector that answers `explicit` at medium confidence (so a second frame is asked
/// for) and, during its first call, lets the test change something.
struct MediumDetector {
    calls: AtomicUsize,
    during_first: Mutex<Option<Box<dyn FnOnce() + Send>>>,
}

#[async_trait::async_trait]
impl ExplicitDetector for MediumDetector {
    async fn check(&self, _image: &EncodedImage, _model: &str) -> CoreResult<ExplicitCheck> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let meanwhile = self.during_first.lock().take();
        if let Some(meanwhile) = meanwhile {
            meanwhile();
        }
        Ok(explicit(Confidence::Medium))
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn nothing_is_captured_or_sent_once_the_user_withdrew_the_check() {
    // "Local only" switched on between the two frames of one check.
    let medium = Arc::new(MediumDetector {
        calls: AtomicUsize::new(0),
        during_first: Mutex::new(None),
    });
    let r = rig(RigSetup {
        detector: Some(medium.clone()),
        ..RigSetup::default()
    });
    r.service.enable(standalone_options()).unwrap();
    r.player();
    let job = r.job_within(30).expect("a check");
    let env = r.env.clone();
    *medium.during_first.lock() = Some(Box::new(move || {
        *env.ai.lock() = AiReadiness::LocalOnly;
    }));
    assert!(!r.service.vision_check(job).await.unwrap().hit);
    assert_eq!(r.frames.calls().len(), 1, "no second frame was taken");
    assert_eq!(medium.calls.load(Ordering::SeqCst), 1, "and none was sent");
    assert!(r.enforcer.calls().is_empty());
    assert!(r.presenter.seen.lock().is_empty());

    // The module switched off between the pass that asked for a check and the check.
    let r = rig(RigSetup::default());
    r.service.enable(no_delay()).unwrap();
    r.player();
    let job = r.job_within(30).expect("a check");
    let waiting = r.ask_off();
    r.service.confirm_pending(&waiting.id).unwrap();
    r.detector.push(Ok(explicit(Confidence::High)));
    assert!(!r.service.vision_check(job).await.unwrap().hit);
    assert!(
        r.frames.calls().is_empty(),
        "an off module captures nothing"
    );
    assert_eq!(r.detector.calls(), 0);
    assert!(r.enforcer.calls().is_empty());

    // With everything in place the same two frames are taken and the second decides.
    let medium = Arc::new(MediumDetector {
        calls: AtomicUsize::new(0),
        during_first: Mutex::new(None),
    });
    let r = rig(RigSetup {
        detector: Some(medium.clone()),
        ..RigSetup::default()
    });
    r.service.enable(standalone_options()).unwrap();
    r.player();
    let job = r.job_within(30).expect("a check");
    assert!(r.service.vision_check(job).await.unwrap().hit);
    assert_eq!(r.frames.calls().len(), 2);
    assert_eq!(medium.calls.load(Ordering::SeqCst), 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_hit_scrubs_only_the_block_of_its_own_window() {
    let h = harness().await;
    h.enable(true);
    let state = h.state().clone();
    let feed = |front: &ForegroundWindow, url: Option<String>| {
        for _ in 0..4 {
            h.advance(30);
            ubiqx_engine::tracker::handle_sample(&state, sample_of(&h, front, url.clone()))
                .unwrap();
        }
        BlockRepo::open_block(h.store.as_ref()).unwrap().unwrap()
    };

    // The tracker has a block open on an editor. A listed tab comes to the front and the
    // guard closes it before the tracker samples again: the editor's block is not the
    // hit's, and stays as it is.
    let editor = window("Editor", "com.example.editor", "relatorio.md", Some(3));
    let work = feed(&editor, None);
    h.browse(listed());
    assert!(h.two_passes());
    ubiqx_engine::tracker::scrub_open_block(&state).unwrap();
    let after = BlockRepo::get(h.store.as_ref(), &work.id).unwrap().unwrap();
    assert!(after.is_open, "the work before the hit was closed");
    assert_eq!(after.title, "relatorio.md");
    assert_eq!(after.category_id, work.category_id);
    assert!(!h.service.covers_open_block("com.example.editor", None));

    // The same browser, with another site in the tracker's open block: not the hit's.
    let docs = window("Google Chrome", CHROME, "Enciclopédia", Some(7));
    let reading = feed(&docs, Some(url_of(POPULAR)));
    assert_eq!(reading.domain.as_deref(), Some(POPULAR));
    h.browse(listed());
    assert!(h.two_passes());
    ubiqx_engine::tracker::scrub_open_block(&state).unwrap();
    let after = BlockRepo::get(h.store.as_ref(), &reading.id)
        .unwrap()
        .unwrap();
    assert!(after.is_open);
    assert_eq!(after.title, reading.title);
    assert_eq!(after.url, Some(url_of(POPULAR)));
    assert!(!h.service.covers_open_block(CHROME, Some(&url_of(POPULAR))));

    // A block of the page the hit was about goes: by its host, and, for a block without
    // an address, by its application.
    assert!(h.service.covers_open_block(CHROME, Some(&url_of(listed()))));
    assert!(h.service.covers_open_block(CHROME, None));
    let player = window("VLC", VLC, "ferias.mp4", Some(9));
    let watching = feed(&player, None);
    h.foreground
        .set(window("VLC", VLC, "ferias (2).mp4", Some(9)), None);
    h.detector.push(Ok(explicit(Confidence::High)));
    assert_eq!(h.check_within(30).await, Some(true));
    ubiqx_engine::tracker::scrub_open_block(&state).unwrap();
    let after = BlockRepo::get(h.store.as_ref(), &watching.id)
        .unwrap()
        .unwrap();
    assert!(!after.is_open, "the title changed, the application did not");
    assert_eq!(after.title, PRIVATE_TITLE);

    // Ten minutes later a hit is not remembered any more.
    h.advance(601);
    assert!(!h.service.covers_open_block(VLC, None));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn site_lists_that_cannot_be_read_are_said_and_read_again() {
    // The person blocked a site of their own; then the app starts while the lists cannot
    // be read.
    let repo = Arc::new(Flaky::default());
    let first = rig(RigSetup {
        repo: repo.clone(),
        ..RigSetup::default()
    });
    first.service.enable(standalone_options()).unwrap();
    first
        .service
        .request(NofapChange::AddBlock {
            host: UNKNOWN.into(),
        })
        .unwrap();
    drop(first);

    repo.fail_lists.store(true, Ordering::SeqCst);
    let r = rig(RigSetup {
        repo: repo.clone(),
        ..RigSetup::default()
    });
    assert!(r.service.enabled());
    // Not "protected": a site the person blocked may be missing, and the page says so.
    assert_eq!(r.service.status().overall, Overall::Reduced);
    assert_eq!(r.service.view().unwrap().status.overall, Overall::Reduced);
    // The embedded list still works meanwhile.
    r.browse(listed());
    r.pass();
    assert!(r.pass().hit);

    // The verify pass reads them again; from then on the person's own list is back.
    repo.fail_lists.store(false, Ordering::SeqCst);
    r.service.verify_pass_after(Duration::from_secs(1)).unwrap();
    assert_eq!(r.service.status().overall, Overall::Protected);
    assert_eq!(r.service.sites().blocked.len(), 1);
    r.browse(UNKNOWN);
    r.pass();
    assert!(r.pass().hit);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_correction_counts_once_however_often_its_button_is_pressed() {
    let h = harness().await;
    h.enable(true);
    h.browse(KEYWORD);
    assert!(h.two_passes());
    let card = h.cards().pop().unwrap();
    assert!(card.actions.contains(&CardAction::NotIt));
    for _ in 0..3 {
        h.service.card_action(&card.id, CardAction::NotIt).unwrap();
    }
    assert_eq!(
        h.counter(DayKind::NotIt),
        1,
        "one of the day's three, not three"
    );
    let snoozes = h
        .stored_sites()
        .iter()
        .filter(|site| site.origin == SiteOrigin::Snooze)
        .count();
    assert_eq!(snoozes, 1);

    // The same on a card of the AI check: one independent look is bought, once.
    h.advance(200);
    let card = positive_on_unknown(&h).await;
    for _ in 0..3 {
        h.service.card_action(&card.id, CardAction::NotIt).unwrap();
    }
    assert_eq!(h.counter(DayKind::NotIt), 2);
    // Another button of the same card still works.
    h.service
        .card_action(&card.id, CardAction::AddToList)
        .unwrap();
    assert_eq!(h.service.sites().blocked.len(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn two_hits_in_the_same_second() {
    let h = harness().await;
    h.enable(true);
    // A player in front long enough for a check to be due; its answer is still out.
    h.open_app("VLC", VLC, "ferias.mp4");
    let job = h.job_within(30).expect("a check");

    // A listed tab: three passes inside one second act twice, count once, show one card.
    h.browse(listed());
    assert!(!h.pass().hit);
    assert!(h.pass().hit);
    assert!(h.pass().hit);
    assert_eq!(h.closes(), 2);
    assert_eq!(h.counter(DayKind::BlockList), 1);
    assert_eq!(h.cards().len(), 1);

    // In that same second the player is back in front and its answer arrives, positive:
    // acted on and counted, and no second card inside the minute.
    let player = window("VLC", VLC, "ferias.mp4", Some(9));
    h.foreground.set(player.clone(), None);
    h.detector.push(Ok(explicit(Confidence::High)));
    assert!(h.service.vision_check(job).await.unwrap().hit);
    assert!(h
        .enforcer
        .calls()
        .contains(&EnforcerCall::HideOthers("ai.ubiqx.app".into())));
    assert_eq!(h.counter(DayKind::VisionHit), 1);
    assert_eq!(h.counter(DayKind::BlockList), 1);
    assert_eq!(h.cards().len(), 1, "one card a minute");
    // The tracker forgets both windows.
    assert!(h.service.redacts(&sample_of(&h, &player, None)));
    let tab = window("Google Chrome", CHROME, TAB_TITLE, Some(7));
    assert!(h
        .service
        .redacts(&sample_of(&h, &tab, Some(url_of(listed())))));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_request_confirmed_twice_is_applied_once() {
    let h = harness_with(Setup {
        os: Os::Linux,
        ..Setup::default()
    })
    .await;
    h.service.enable(with_hosts(false)).unwrap();
    let ChangeOutcome::Pending(waiting) = h.service.request(NofapChange::Disable).unwrap() else {
        panic!("pending");
    };
    h.advance(600);
    h.observe_running(600);
    let card = h.cards_of(CardKind::PendingReady).pop().expect("the card");
    h.shield.clear_calls();

    // The page and the card window confirm at the same moment.
    let confirm = |service: Arc<NofapService>, id: Id| {
        std::thread::spawn(move || service.confirm_pending(&id).map(|view| view.config.enabled))
    };
    let (page, window) = (
        confirm(h.service.clone(), waiting.id.clone()),
        confirm(h.service.clone(), waiting.id.clone()),
    );
    let answers = [page.join().unwrap(), window.join().unwrap()];
    assert_eq!(
        answers.iter().filter(|answer| answer.is_ok()).count(),
        1,
        "{answers:?}"
    );
    assert!(answers
        .iter()
        .any(|answer| matches!(answer, Err(CoreError::NotFound(_)))));
    let removes = |h: &Harness| {
        h.shield
            .calls()
            .into_iter()
            .filter(|call| matches!(call, SystemCall::Remove(..)))
            .count()
    };
    assert_eq!(removes(&h), 1, "the system is asked once");
    assert!(!h.service.enabled());

    // Late presses change nothing more.
    h.service
        .card_action(&card.id, CardAction::TurnOff)
        .unwrap();
    assert!(matches!(
        h.service.confirm_pending(&waiting.id),
        Err(CoreError::NotFound(_))
    ));
    assert_eq!(removes(&h), 1);
    assert_eq!(h.counter(DayKind::OverrideRequest), 1);
    assert_eq!(h.stored_config().pending.len(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_clock_jumps_while_a_check_waits_for_its_answer() {
    let held = Arc::new(HeldDetector {
        asked: AtomicUsize::new(0),
        answer: tokio::sync::Semaphore::new(0),
        verdict: explicit(Confidence::High),
    });
    let r = rig(RigSetup {
        detector: Some(held.clone()),
        ..RigSetup::default()
    });
    r.service.enable(standalone_options()).unwrap();
    let asked = |times: usize| {
        let started = std::time::Instant::now();
        while held.asked.load(Ordering::SeqCst) < times {
            assert!(
                started.elapsed() < Duration::from_secs(20),
                "the check never asked"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    };
    let run = |job: VisionJob| {
        let service = r.service.clone();
        tokio::spawn(async move { service.vision_check(job).await.unwrap().hit })
    };

    // The clock is set back three hours while the provider thinks. The answer is acted on
    // all the same: what counts is that the window and the host are the ones captured.
    r.browse(UNKNOWN);
    let check = run(r.job_within(30).expect("a check"));
    asked(1);
    r.clock.advance(ChronoDuration::hours(-3));
    held.answer.add_permits(1);
    assert!(tokio::time::timeout(Duration::from_secs(20), check)
        .await
        .expect("the answer was handled")
        .unwrap());
    assert_eq!(
        r.enforcer.calls(),
        vec![EnforcerCall::CloseActiveTab(CHROME.into())]
    );
    assert_eq!(r.service.sites().learned_count, 1);
    assert_eq!(r.counter(DayKind::VisionCheck), 1);
    // The service itself hands the repository a hash and no name (the database drops a
    // learned name too; a repository that keeps what it is given shows which of the two
    // did it).
    let learned = r.repo.inner.list_sites().unwrap();
    assert_eq!(learned.len(), 1);
    assert_eq!(
        (learned[0].origin, learned[0].host.clone()),
        (SiteOrigin::Learned, None)
    );
    // The guard goes on by the clock as it is now: the learned page is closed again...
    r.pass();
    assert!(r.pass().hit);
    // ...and another unknown page is still looked at in its time, not three hours later.
    r.browse(UNKNOWN_2);
    let check = run(r.job_within(60).expect("a check within a minute"));
    asked(2);

    // This time the clock goes forward two days while the answer is out.
    r.clock.advance(ChronoDuration::days(2));
    held.answer.add_permits(1);
    assert!(tokio::time::timeout(Duration::from_secs(20), check)
        .await
        .expect("the answer was handled")
        .unwrap());
    assert_eq!(r.service.sites().learned_count, 2);
    assert_eq!(r.counter(DayKind::VisionCheck), 2);
    assert_eq!(r.service.status().vision, VisionState::On);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_clock_set_back_does_not_end_a_wait_and_set_forward_does_not_shorten_it() {
    let h = harness().await;
    h.enable(false);
    let ChangeOutcome::Pending(waiting) = h.service.request(NofapChange::Disable).unwrap() else {
        panic!("pending");
    };

    // Back one day, with all the running time the request needs: the clock says no.
    h.advance(-86_400);
    h.observe_running(600);
    let view = h.service.view().unwrap();
    assert!(!view.status.pending[0].ready);
    assert_eq!(view.status.pending[0].remaining_secs, 86_400 + 600);
    assert!(matches!(
        h.service.confirm_pending(&waiting.id),
        Err(CoreError::Invalid(_))
    ));
    assert!(h.service.enabled());
    h.browse(listed());
    assert!(h.two_passes(), "still enforcing");

    // Forward again to the moment it was promised for: ready, and it needs the confirmation.
    h.advance(86_400 + 600);
    assert!(h.service.view().unwrap().status.pending[0].ready);
    assert!(h.service.enabled());
    assert!(
        !h.service
            .confirm_pending(&waiting.id)
            .unwrap()
            .config
            .enabled
    );

    // A second request, and the clock jumps a month ahead with a minute of running time.
    h.enable(false);
    let ChangeOutcome::Pending(second) = h.service.request(NofapChange::Disable).unwrap() else {
        panic!("pending");
    };
    h.advance(30 * 86_400);
    h.observe_running(60);
    assert!(!h.service.view().unwrap().status.pending[0].ready);
    assert!(h.service.confirm_pending(&second.id).is_err());
    assert!(h.service.enabled());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_store_failing_in_the_middle_of_a_pass_does_not_stop_the_guard() {
    let r = rig(RigSetup::default());
    r.service.enable(standalone_options()).unwrap();

    // The counters cannot be reached: the tab is closed and the card shown all the same,
    // and the card does not hand out a snooze it cannot count.
    r.repo.fail_counters.store(true, Ordering::SeqCst);
    r.browse(KEYWORD);
    assert!(!r.pass().hit);
    assert!(r.pass().hit);
    assert_eq!(
        r.enforcer.calls(),
        vec![EnforcerCall::CloseActiveTab(CHROME.into())]
    );
    let card = r.cards_of(CardKind::Blocked).pop().expect("the card");
    assert!(card.actions.contains(&CardAction::RequestException));
    assert!(!card.actions.contains(&CardAction::NotIt));

    // An AI check runs and acts though it cannot count itself.
    r.clock.advance(ChronoDuration::seconds(200));
    r.player();
    let job = r.job_within(30).expect("a check");
    r.detector.push(Ok(explicit(Confidence::High)));
    assert!(r.service.vision_check(job).await.unwrap().hit);
    assert!(r
        .enforcer
        .calls()
        .contains(&EnforcerCall::HideOthers("ai.ubiqx.shield".into())));

    // A positive that cannot be stored as a learned site still closes the tab.
    r.repo.fail_counters.store(false, Ordering::SeqCst);
    r.repo.fail_lists.store(true, Ordering::SeqCst);
    r.clock.advance(ChronoDuration::seconds(200));
    r.enforcer.clear();
    r.browse(UNKNOWN);
    let job = r.job_within(30).expect("a check");
    r.detector.push(Ok(explicit(Confidence::High)));
    assert!(r.service.vision_check(job).await.unwrap().hit);
    assert_eq!(
        r.enforcer.calls(),
        vec![EnforcerCall::CloseActiveTab(CHROME.into())]
    );
    // The page says that the lists could not be read.
    assert_eq!(r.service.view().unwrap().status.overall, Overall::Reduced);
}

/// Every port of the service. Any call fails the test.
struct Untouchable;

impl SystemShield for Untouchable {
    fn capabilities(&self) -> SystemCapabilities {
        panic!("the system layer was asked for its capabilities")
    }

    fn observe(&self, _opts: ObserveOpts) -> CoreResult<SystemObservation> {
        panic!("the system was read")
    }

    fn preview(&self, _plan: &SystemPlan) -> CoreResult<Vec<PlannedChange>> {
        panic!("a plan was previewed")
    }

    fn apply(&self, _plan: &SystemPlan) -> CoreResult<ApplyOutcome> {
        panic!("a plan was applied")
    }

    fn remove(&self, _owner: ProductId, _scope: RemoveScope) -> CoreResult<ApplyOutcome> {
        panic!("the tiers were removed")
    }
}

impl FrameSource for Untouchable {
    fn frame(&self, _window_id: u32, _opts: FrameOpts) -> CoreResult<Option<Frame>> {
        panic!("a frame was taken")
    }
}

#[async_trait::async_trait]
impl ExplicitDetector for Untouchable {
    async fn check(&self, _image: &EncodedImage, _model: &str) -> CoreResult<ExplicitCheck> {
        panic!("an image was sent")
    }
}

impl NofapPresenter for Untouchable {
    fn show(&self, _card: &NofapCard) -> CoreResult<bool> {
        panic!("a card was shown")
    }

    fn dismiss(&self, _id: &str) {
        panic!("a card was dismissed")
    }
}

impl NofapSink for Untouchable {
    fn emit(&self, _event: NofapEvent) {
        panic!("an event was emitted")
    }
}

impl NofapEnv for Untouchable {
    fn snapshot(&self) -> NofapEnvSnapshot {
        panic!("the host's settings were read")
    }
}

impl ActivitySource for Untouchable {
    fn foreground(&self) -> CoreResult<Option<ForegroundWindow>> {
        panic!("the window in front was read")
    }

    fn peek_foreground(&self) -> CoreResult<Option<ForegroundWindow>> {
        panic!("the window in front was read")
    }
}

impl BrowserUrlResolver for Untouchable {
    fn supports(&self, _app_id: &str) -> bool {
        panic!("the URL resolver was asked")
    }

    fn resolve(&self, _window: &ForegroundWindow) -> CoreResult<Option<String>> {
        panic!("an address was read")
    }

    fn resolve_fresh(&self, _window: &ForegroundWindow) -> CoreResult<Option<String>> {
        panic!("an address was read")
    }
}

impl Enforcer for Untouchable {
    fn quit_app(&self, _bundle_id: &str) -> CoreResult<bool> {
        panic!("an application was quit")
    }

    fn close_active_tab(&self, _browser_bundle_id: &str) -> CoreResult<bool> {
        panic!("a tab was closed")
    }

    fn blank_active_tab(&self, _browser_bundle_id: &str) -> CoreResult<bool> {
        panic!("a tab was blanked")
    }

    fn hide_others(&self, _keep_bundle_id: &str) -> CoreResult<bool> {
        panic!("windows were hidden")
    }

    fn run_shortcut(&self, _name: &str) -> CoreResult<bool> {
        panic!("a shortcut was run")
    }
}

impl PermissionChecker for Untouchable {
    fn status(&self) -> PermissionStatus {
        panic!("the permissions were read")
    }

    fn request(&self, _kind: PermissionKind) -> CoreResult<()> {
        panic!("a permission was asked for")
    }
}

impl Notifier for Untouchable {
    fn notify(&self, _title: &str, _body: &str) -> CoreResult<()> {
        panic!("a notification was sent")
    }
}

impl UsageRepo for Untouchable {
    fn record(&self, _usage: &AiUsage) -> CoreResult<()> {
        panic!("the ledger was written")
    }

    fn totals(&self, _range: TimeRange) -> CoreResult<AiUsageTotals> {
        panic!("the ledger was read")
    }

    fn totals_by_kind(&self, _range: TimeRange) -> CoreResult<Vec<(AiUsageKind, AiUsageTotals)>> {
        panic!("the ledger was read")
    }
}

/// The service over ports that must not be touched, and the repository it may read once,
/// when it is built.
fn untouchable(repo: Arc<Flaky>) -> Arc<NofapService> {
    let port = Arc::new(Untouchable);
    let ports = NofapPorts {
        system: port.clone(),
        frames: port.clone(),
        detector: Some(port.clone()),
        presenter: port.clone(),
        sink: port.clone(),
        repo,
        shared_dir: None,
        tuning: Tuning {
            confirm_after: Duration::from_millis(5),
            check_timeout: Duration::from_secs(5),
            background: Background::Inline,
            display: DisplaySession::Full,
        },
    };
    let host = NofapHost {
        env: port.clone(),
        activity: port.clone(),
        urls: port.clone(),
        enforcer: port.clone(),
        permissions: port.clone(),
        notifier: port.clone(),
        usage: port,
        clock: Arc::new(FixedClock::new(fixed_now())),
        identity: &ubiqx_core::product::SHIELD,
        export_dir: std::env::temp_dir().join("never-written"),
    };
    NofapService::new(NofapDeps::assemble(ports, host))
}

#[test]
fn with_the_module_off_no_port_is_called() {
    let sample = ActivitySample {
        at: fixed_now(),
        app_name: "Google Chrome".into(),
        app_id: CHROME.into(),
        window_title: TAB_TITLE.into(),
        url: Some(url_of(listed())),
        idle_secs: Some(0.0),
        window_id: Some(7),
    };
    let everything_a_host_calls_by_itself = |service: &NofapService, repo: &Flaky| {
        let stored = repo.calls();
        for _ in 0..3 {
            let outcome = service.guard_pass().unwrap();
            assert!(!outcome.hit && outcome.job.is_none());
        }
        service.verify_pass().unwrap();
        service.verify_pass_after(Duration::from_secs(120)).unwrap();
        assert!(!service.redacts(&sample));
        assert!(!service.capture_veto(CHROME, Some(listed()), TAB_TITLE));
        assert!(!service.covers_open_block(CHROME, Some(&url_of(listed()))));
        assert!(!service.enabled());
        service.status();
        service.on_quit();
        service.release_primary();
        assert_eq!(repo.calls(), stored, "the database was asked");
    };

    // Never enabled.
    let repo = Arc::new(Flaky::default());
    let service = untouchable(repo.clone());
    assert_eq!(service.status(), NofapStatus::off());
    everything_a_host_calls_by_itself(&service, &repo);

    // A stored configuration that cannot be read: nothing is assumed, nothing is touched.
    let repo = Arc::new(Flaky::default());
    repo.unreadable.store(true, Ordering::SeqCst);
    let service = untouchable(repo.clone());
    assert_eq!(service.status().overall, Overall::Reduced);
    everything_a_host_calls_by_itself(&service, &repo);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_pass_over_a_page_that_is_not_blocked_asks_the_database_nothing() {
    // With the AI check on, between two checks: the pass reads the window, its address and
    // the host's settings, and that is all. No list is read again, no counter, no ledger.
    let r = rig(RigSetup::default());
    r.service.enable(standalone_options()).unwrap();
    r.browse(UNKNOWN);
    r.pass();
    let stored = r.repo.calls();
    for _ in 0..200 {
        let outcome = r.pass();
        assert!(!outcome.hit && outcome.job.is_none());
    }
    assert_eq!(r.repo.calls(), stored);
    assert!(r.frames.calls().is_empty());

    // With the AI check off the host's settings are not read either.
    let r = rig(RigSetup::default());
    r.service
        .enable(EnableOptions {
            vision: false,
            vision_consent: None,
            ..standalone_options()
        })
        .unwrap();
    r.browse(UNKNOWN);
    r.pass();
    let before = (r.repo.calls(), r.env.calls.load(Ordering::SeqCst));
    for _ in 0..200 {
        r.pass();
    }
    assert_eq!((r.repo.calls(), r.env.calls.load(Ordering::SeqCst)), before);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn d10_the_ways_out_of_an_unreadable_configuration() {
    let repo = Arc::new(Flaky::default());
    repo.unreadable.store(true, Ordering::SeqCst);
    let r = rig(RigSetup {
        repo: repo.clone(),
        ..RigSetup::default()
    });
    assert_eq!(r.service.status().overall, Overall::Reduced);
    // No file is written that would show the defaults as the person's configuration.
    assert!(matches!(r.service.export(), Err(CoreError::Storage(_))));
    assert!(!r.tmp.path().join("exports").exists());
    // Erasing one's history stays possible and replaces nothing.
    r.service.erase_history().unwrap();
    assert_eq!(repo.saves.load(Ordering::SeqCst), 0);
    assert_eq!(r.service.status().overall, Overall::Reduced);

    // "Apagar tudo" is a way out: a fresh configuration, off, stored.
    r.service.reset().unwrap();
    assert_eq!(repo.saves.load(Ordering::SeqCst), 1);
    assert_eq!(r.service.status(), NofapStatus::off());
    assert_eq!(repo.inner.load_config().unwrap(), NofapConfig::default());
    assert!(r.service.export().is_ok());

    // A reset that cannot be stored leaves the unreadable state as it was.
    let repo = Arc::new(Flaky::default());
    repo.unreadable.store(true, Ordering::SeqCst);
    repo.fail_saves.store(true, Ordering::SeqCst);
    let r = rig(RigSetup {
        repo,
        ..RigSetup::default()
    });
    assert!(r.service.reset().is_err());
    assert!(r.service.enable(standalone_options()).is_err());
    assert_eq!(r.service.status().overall, Overall::Reduced);
    assert!(!r.service.enabled());
}
