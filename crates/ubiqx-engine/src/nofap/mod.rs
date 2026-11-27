//! The runtime of the `nofap` module: one [`NofapService`], built from ports only, so
//! ubiqX's engine and the standalone app host the same service. Every decision lives in
//! the pure crate `ubiqx-nofap`; this module wires those decisions to the platform ports,
//! the repository, the card window and the event sink.
//!
//! | File | What it holds |
//! |---|---|
//! | `guard.rs` | the pass that runs every two seconds: match the tab in front, enforce |
//! | `vision.rs` | readiness of the AI check, one check, what an answer does |
//! | `system.rs` | the status, the verify pass, the system tiers, the primary lock |
//! | `service.rs` | what the page and the card window ask for |
//! | `cards.rs` | the cards and what their buttons need, in memory only |
//! | `ipc.rs` | the one command both shells expose |
//! | `env.rs` | ubiqX's settings and licence as the module reads them |
//!
//! # Rules every file keeps
//!
//! * **The runtime mutex is never held across a platform call or an await** (`resolve`,
//!   `resolve_fresh`, an `Enforcer` method, `frame`, `check`, `observe`, `apply`,
//!   `remove`, `show`, `emit`). `apply` blocks on the password prompt for as long as the
//!   user takes; a guard pass must not wait for it. Each method copies what it needs,
//!   releases the lock, calls out and locks again to record the result. Repository calls
//!   are short and may run under the lock.
//! * **A change to the configuration is stored before it counts.** Every change is made
//!   on a copy, the copy is written, and only then does it replace the configuration in
//!   memory (`update_config`). A write that fails leaves the module as it was: one that
//!   could not be stored as "off" keeps running, and a request that could not be stored
//!   is not waiting.
//! * **Nothing names what was seen.** No host, title, URL or application leaves the
//!   service in a card, an event, a notification, a counter or a log line. Log lines are
//!   written for state changes only (enabled, disabled, a tier applied, the AI check
//!   degraded) and never for a hit, a verdict or a check.
//! * **Off means untouched.** With the module off (the default), or while the other
//!   product on this machine runs the passes, a pass returns before any platform call.

pub mod cards;
pub mod env;
mod guard;
pub mod ipc;
mod service;
mod system;
mod vision;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Weak};
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use tokio_util::sync::CancellationToken;
use ubiqx_core::nofap::{
    DayKind, ExplicitDetector, FrameSource, GuardLimit, NofapConfig, NofapEnv, NofapEvent,
    NofapPresenter, NofapRepo, NofapSink, NofapSite, NofapStatus, Os, ResolverObservation,
    SiteOrigin, SystemObservation, SystemShield,
};
use ubiqx_core::ports::{
    ActivitySource, BrowserUrlResolver, Enforcer, Notifier, PermissionChecker, UsageRepo,
};
use ubiqx_core::{Clock, CoreError, CoreResult, ProductIdentity};
use ubiqx_nofap::episode::{EpisodeTracker, PassConfirmer};
use ubiqx_nofap::matcher::{Matcher, MatcherData};
use ubiqx_nofap::plan::SystemAssessment;
use ubiqx_nofap::primary::Role;
use ubiqx_nofap::sampling::{
    SamplerMemory, WindowKind, CHECK_TIMEOUT_SECS, CONFIRM_AFTER_SECS, HIT_MEMORY_SECS, SNOOZE_SECS,
};
use ubiqx_nofap::{progress, sampling};

use crate::deps::NofapPorts;
use cards::{CardStore, Outbox};

pub use guard::GuardOutcome;
pub use vision::{VisionJob, VisionOutcome};

/// How the service runs work that must not hold up an answer (the system read behind
/// `view`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Background {
    /// On a thread of its own.
    #[default]
    Thread,
    /// Before the answer, on the caller's thread: deterministic, for tests.
    Inline,
}

/// What the desktop session lets the guard read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DisplaySession {
    /// The window in front and, for a supported browser, its address.
    #[default]
    Full,
    /// Linux X11: the address is known only when the title prints it.
    TitleOnly,
    /// Linux Wayland: no window in front and no capture.
    Wayland,
}

impl DisplaySession {
    /// The session this process runs in, from the operating system and, on Linux, the
    /// session type the desktop announces.
    pub fn detect() -> Self {
        if Os::current() != Some(Os::Linux) {
            return DisplaySession::Full;
        }
        let wayland = std::env::var("XDG_SESSION_TYPE")
            .is_ok_and(|kind| kind.eq_ignore_ascii_case("wayland"))
            || std::env::var_os("WAYLAND_DISPLAY").is_some_and(|name| !name.is_empty());
        if wayland {
            DisplaySession::Wayland
        } else {
            DisplaySession::TitleOnly
        }
    }
}

/// The few knobs of the service that are not decisions: waits a test shortens, and what
/// the composition root knows about the session.
#[derive(Debug, Clone)]
pub struct Tuning {
    /// Wait before the second frame of a `Confirm` (design 4.6).
    pub confirm_after: Duration,
    /// The bound around one call to the detector (the client retries on its own).
    pub check_timeout: Duration,
    pub background: Background,
    pub display: DisplaySession,
}

impl Default for Tuning {
    fn default() -> Self {
        Self {
            confirm_after: Duration::from_secs(CONFIRM_AFTER_SECS),
            check_timeout: Duration::from_secs(CHECK_TIMEOUT_SECS),
            background: Background::default(),
            display: DisplaySession::detect(),
        }
    }
}

/// What the host of the service supplies besides [`NofapPorts`]: ports it already has, the
/// clock, its identity and where exports go.
#[derive(Clone)]
pub struct NofapHost {
    pub env: Arc<dyn NofapEnv>,
    pub activity: Arc<dyn ActivitySource>,
    pub urls: Arc<dyn BrowserUrlResolver>,
    pub enforcer: Arc<dyn Enforcer>,
    pub permissions: Arc<dyn PermissionChecker>,
    pub notifier: Arc<dyn Notifier>,
    pub usage: Arc<dyn UsageRepo>,
    pub clock: Arc<dyn Clock>,
    pub identity: &'static ProductIdentity,
    /// Directory "Exportar dados" writes into (`<data_dir>/exports`).
    pub export_dir: PathBuf,
}

/// Everything the service is built from. Ports only: no `Engine`, no engine state.
#[derive(Clone)]
pub struct NofapDeps {
    pub system: Arc<dyn SystemShield>,
    pub frames: Arc<dyn FrameSource>,
    pub detector: Option<Arc<dyn ExplicitDetector>>,
    pub presenter: Arc<dyn NofapPresenter>,
    pub sink: Arc<dyn NofapSink>,
    pub repo: Arc<dyn NofapRepo>,
    pub env: Arc<dyn NofapEnv>,
    pub activity: Arc<dyn ActivitySource>,
    pub urls: Arc<dyn BrowserUrlResolver>,
    pub enforcer: Arc<dyn Enforcer>,
    pub permissions: Arc<dyn PermissionChecker>,
    pub notifier: Arc<dyn Notifier>,
    pub usage: Arc<dyn UsageRepo>,
    pub clock: Arc<dyn Clock>,
    pub identity: &'static ProductIdentity,
    /// Directory of the primary lock both products share; `None`: always primary.
    pub shared_dir: Option<PathBuf>,
    pub export_dir: PathBuf,
    pub tuning: Tuning,
}

impl NofapDeps {
    /// The module's own ports plus what the host has.
    pub fn assemble(ports: NofapPorts, host: NofapHost) -> Self {
        Self {
            system: ports.system,
            frames: ports.frames,
            detector: ports.detector,
            presenter: ports.presenter,
            sink: ports.sink,
            repo: ports.repo,
            shared_dir: ports.shared_dir,
            tuning: ports.tuning,
            env: host.env,
            activity: host.activity,
            urls: host.urls,
            enforcer: host.enforcer,
            permissions: host.permissions,
            notifier: host.notifier,
            usage: host.usage,
            clock: host.clock,
            identity: host.identity,
            export_dir: host.export_dir,
        }
    }
}

/// What the verify pass remembers about the machine between two looks.
#[derive(Debug, Default)]
pub(crate) struct SystemMemory {
    /// The last look, with the carried canary answer and resolver part in it.
    pub observation: Option<SystemObservation>,
    pub assessment: Option<SystemAssessment>,
    pub extras_left_out: u32,
    /// The last canary answer and when it was asked for (integrator decision 11).
    pub canary: Option<Vec<String>>,
    pub canary_at: Option<DateTime<Utc>>,
    /// Windows: the resolver part of the last full read and when it was made.
    pub full_resolver: Option<ResolverObservation>,
    pub full_at: Option<DateTime<Utc>>,
    /// Consecutive looks in which the probe name did not resolve.
    pub probe_fail_streak: u32,
    /// A chosen tier is changed or missing (the drift was counted).
    pub in_drift: bool,
    /// The card of the present drift was shown, or the six quiet hours spared it.
    pub drift_carded: bool,
    /// The "names are not resolving" card was shown for the present outage.
    pub unreachable_carded: bool,
    /// When the system was last read for `view` (at most once every 60 seconds).
    pub view_read_at: Option<DateTime<Utc>>,
}

/// Where a hit was: the application and, for a browser tab whose address is an ordinary
/// one, the host. Never serialised, never logged.
#[derive(Debug, Clone)]
pub(crate) struct HitMark {
    pub app_id: String,
    pub host: Option<String>,
    pub at: DateTime<Utc>,
}

/// Hit marks kept at most (a mark is only needed for the moments after its hit).
const HIT_MARKS_KEPT: usize = 16;

/// The whole in-memory state of the service, behind one mutex. Gone at exit.
pub(crate) struct Runtime {
    pub config: NofapConfig,
    /// The stored configuration could not be read: nothing is assumed about it, and
    /// nothing overwrites it except the wizard and "Apagar tudo".
    pub config_error: bool,
    pub sites: Vec<NofapSite>,
    /// The site lists could not be read (at the start, or after a change): what is in
    /// memory may miss a site the user blocked. Said in the status and read again by the
    /// verify pass until it works.
    pub sites_error: bool,
    /// `None` only while the embedded tables do not parse (a broken build).
    pub matcher: Option<Arc<Matcher>>,
    /// The earliest moment a snooze ends, when the matcher has to be rebuilt.
    pub snooze_ends: Option<DateTime<Utc>>,
    pub confirmer: PassConfirmer,
    pub episodes: EpisodeTracker,
    pub sampler: SamplerMemory,
    /// Keys whose next check is the one "Não era isso" bought.
    pub recheck: HashSet<u64>,
    pub cards: CardStore,
    /// Position in the rotation of support messages.
    pub rotation: usize,
    /// The "repeat" card of the running episode could not be shown yet.
    pub repeat_owed: bool,
    /// Context keys that had a hit, and when (the tracker forgets those windows).
    pub hits: HashMap<u64, DateTime<Utc>>,
    /// The windows of the latest hits, for the tracker's question "is the block I have
    /// open the one that hit was about?". In memory only, like a card's context.
    pub hit_marks: Vec<HitMark>,
    /// Context keys that carried a local signal, and when.
    pub signals: HashMap<u64, DateTime<Utc>>,
    /// The last window that was a candidate for the AI check: its id and kind.
    pub last_candidate: Option<(u32, WindowKind)>,
    /// What keeps the guard from reading the window in front, if anything.
    pub guard_limit: Option<GuardLimit>,
    /// When a host of the applied hosts block was last seen in front.
    pub bypass_at: Option<DateTime<Utc>>,
    pub role: Role,
    /// This process wrote the last heartbeat it knows of.
    pub holding: bool,
    pub heartbeat_at: Option<DateTime<Utc>>,
    pub system: SystemMemory,
    pub status: NofapStatus,
    /// The provider the sampler's memory belongs to; a change clears it.
    pub provider_seen: Option<(ubiqx_core::AiProvider, ubiqx_core::nofap::AiReadiness)>,
    /// The local day the counters were last pruned on.
    pub pruned_on: Option<String>,
    /// When the provider was first seen rejecting the key (it is tried again later).
    pub rejected_at: Option<DateTime<Utc>>,
    /// When the AI check was last found not ready by the guard (asked again a little
    /// later, not in every pass).
    pub not_ready_at: Option<DateTime<Utc>>,
    /// Waiting "turn it off" requests whose "pending ready" card was shown in this run.
    pub ready_carded: HashSet<ubiqx_core::Id>,
}

/// The module's runtime. See the module documentation for the rules it keeps.
pub struct NofapService {
    pub(crate) deps: NofapDeps,
    pub(crate) me: Weak<NofapService>,
    pub(crate) rt: Mutex<Runtime>,
    /// One AI check at a time; cleared when its job is dropped.
    pub(crate) in_flight: Arc<AtomicBool>,
    /// One background system read at a time.
    pub(crate) reading: AtomicBool,
    /// When the verify pass last ran, on the monotonic clock.
    pub(crate) verified: Mutex<Instant>,
    /// Serialises the reading and writing of the primary lock (the guard loop and the
    /// verify loop both ask for the role). Never taken while the runtime lock is held.
    pub(crate) role_lock: Mutex<()>,
    /// One status at a time is computed, stored and announced, so a status computed
    /// before a change can never be stored or sent after the one computed behind it
    /// (every change is followed by its own refresh). Never taken while the runtime lock
    /// is held.
    pub(crate) status_gate: Mutex<()>,
    /// Held while the operating system's prompt is open (apply, remove, restore the
    /// network): a second click is refused instead of raising a second prompt behind the
    /// first. Never taken while the runtime lock is held.
    pub(crate) elevating: Mutex<()>,
    pub(crate) matcher_data: MatcherData,
}

impl NofapService {
    /// Builds the service and reads what it stored. Nothing here can fail in a way the
    /// caller could act on: a stored configuration that cannot be read is a degraded
    /// state of the service (integrator decision 10), never "off by default" and never a
    /// reason not to start the app.
    pub fn new(deps: NofapDeps) -> Arc<Self> {
        let now = deps.clock.now();
        let (mut config, config_error) = match deps.repo.load_config() {
            Ok(config) => (config, false),
            Err(error) => {
                tracing::warn!(
                    class = error_class(&error),
                    "nofap: the stored configuration cannot be read; protection state unknown"
                );
                (NofapConfig::default(), true)
            }
        };
        config.sanitize();
        let (sites, sites_error) = match deps.repo.list_sites() {
            Ok(sites) => (sites, false),
            Err(error) => {
                tracing::warn!(
                    class = error_class(&error),
                    "nofap: the site lists were not read"
                );
                (Vec::new(), true)
            }
        };
        let matcher_data = MatcherData::embedded();
        let salt = salt_of(&config);
        let matcher = Matcher::new(matcher_data.clone(), &live_sites(&sites, now), &salt, now)
            .map(Arc::new)
            .map_err(|error| {
                tracing::error!(
                    class = error_class(&error),
                    "nofap: the embedded lists cannot be read"
                );
            })
            .ok();
        let runtime = Runtime {
            snooze_ends: next_snooze_end(&sites, now),
            config,
            config_error,
            sites,
            sites_error,
            matcher,
            confirmer: PassConfirmer::default(),
            episodes: EpisodeTracker::default(),
            sampler: SamplerMemory::default(),
            recheck: HashSet::new(),
            cards: CardStore::default(),
            rotation: 0,
            repeat_owed: false,
            hits: HashMap::new(),
            hit_marks: Vec::new(),
            signals: HashMap::new(),
            last_candidate: None,
            guard_limit: None,
            bypass_at: None,
            role: Role::Primary,
            holding: false,
            heartbeat_at: None,
            system: SystemMemory::default(),
            status: NofapStatus::off(),
            provider_seen: None,
            pruned_on: None,
            rejected_at: None,
            not_ready_at: None,
            ready_carded: HashSet::new(),
        };
        let service = Arc::new_cyclic(|me| Self {
            deps,
            me: me.clone(),
            rt: Mutex::new(runtime),
            in_flight: Arc::new(AtomicBool::new(false)),
            reading: AtomicBool::new(false),
            verified: Mutex::new(Instant::now()),
            role_lock: Mutex::new(()),
            status_gate: Mutex::new(()),
            elevating: Mutex::new(()),
            matcher_data,
        });
        service.seed_day_count();
        if !config_error {
            // The retention of the counters holds from the start, also for a module that
            // stays off (the verify pass prunes once a day from then on).
            let mut rt = service.rt.lock();
            service.prune_daily(&mut rt, now);
        }
        // The first status is computed from memory alone: no platform port is touched
        // while the module is off.
        let status = service.compute_status();
        service.rt.lock().status = status;
        service
    }

    /// Whether the module is switched on.
    pub fn enabled(&self) -> bool {
        self.rt.lock().config.enabled
    }

    /// The status as last computed.
    pub fn status(&self) -> NofapStatus {
        self.rt.lock().status.clone()
    }

    pub(crate) fn now(&self) -> DateTime<Utc> {
        self.deps.clock.now()
    }

    /// The local day and hour of `now`: the key of a counter.
    pub(crate) fn day_hour(&self, now: DateTime<Utc>) -> (String, u8) {
        let offset = self.deps.clock.now_local().offset().local_minus_utc();
        progress::local_day_hour(now, offset)
    }

    /// Adds one to a counter of the present hour. A counter that cannot be stored is not
    /// worth failing a pass for, and its failure is not logged with what it counted.
    pub(crate) fn bump(&self, kind: DayKind) {
        let (day, hour) = self.day_hour(self.now());
        if let Err(error) = self.deps.repo.bump_day(&day, hour, kind, 1) {
            tracing::debug!(
                class = error_class(&error),
                "nofap: a counter was not stored"
            );
        }
    }

    /// Changes the configuration: `change` works on a copy, the copy is stored, and only
    /// then does it become the configuration in memory. A write that fails (or a change
    /// that is refused) therefore leaves memory and disk saying the same thing: a module
    /// that could not be stored as "off" keeps running, and one that could not be stored
    /// as "on" does not pretend to be. Refused while the stored configuration could not be
    /// read, so an unreadable row is never replaced by defaults behind the user's back.
    pub(crate) fn update_config<T>(
        &self,
        rt: &mut Runtime,
        change: impl FnOnce(&mut NofapConfig) -> CoreResult<T>,
    ) -> CoreResult<T> {
        if rt.config_error {
            return Err(unreadable_config());
        }
        let mut next = rt.config.clone();
        let value = change(&mut next)?;
        next.sanitize();
        if next != rt.config {
            self.deps.repo.save_config(&next)?;
            rt.config = next;
        }
        Ok(value)
    }

    /// Replaces the whole configuration (the wizard, "Apagar tudo"): stored first, then
    /// taken into memory. These two are also the way out of a stored configuration that
    /// cannot be read.
    pub(crate) fn replace_config(
        &self,
        rt: &mut Runtime,
        mut config: NofapConfig,
    ) -> CoreResult<()> {
        config.sanitize();
        self.deps.repo.save_config(&config)?;
        rt.config = config;
        rt.config_error = false;
        Ok(())
    }

    /// Reads the sites again and rebuilds the matcher (after any change to `nofap_sites`
    /// and when a snooze ends). A matcher that cannot be built keeps the previous one.
    pub(crate) fn reload_sites(&self, rt: &mut Runtime) {
        let now = self.now();
        match self.deps.repo.list_sites() {
            Ok(sites) => {
                rt.sites = sites;
                rt.sites_error = false;
            }
            Err(error) => {
                rt.sites_error = true;
                tracing::warn!(
                    class = error_class(&error),
                    "nofap: the site lists were not read"
                );
            }
        }
        let salt = salt_of(&rt.config);
        match Matcher::new(
            self.matcher_data.clone(),
            &live_sites(&rt.sites, now),
            &salt,
            now,
        ) {
            Ok(matcher) => rt.matcher = Some(Arc::new(matcher)),
            Err(error) => {
                tracing::error!(
                    class = error_class(&error),
                    "nofap: the lists cannot be read"
                );
            }
        }
        rt.snooze_ends = next_snooze_end(&rt.sites, now);
    }

    /// Tells the sampler how many checks were already made today, so the daily cap
    /// survives a restart.
    pub(crate) fn seed_day_count(&self) {
        let (day, _) = self.day_hour(self.now());
        let checks = self
            .deps
            .repo
            .days_between(&day, &day)
            .map(|counts| progress::total_on(&counts, &day, DayKind::VisionCheck))
            .unwrap_or(0);
        self.rt.lock().sampler.set_day_count(&day, checks);
    }

    /// Forgets everything kept per window: the sampler's memory, hits, signals, cards and
    /// the running episode ("Apagar todos os dados" and "Apagar meu histórico").
    pub fn clear_memory(&self) {
        {
            let mut rt = self.rt.lock();
            rt.sampler.clear();
            rt.recheck.clear();
            rt.hits.clear();
            rt.hit_marks.clear();
            rt.signals.clear();
            rt.cards = CardStore::default();
            rt.episodes = EpisodeTracker::default();
            rt.confirmer.clear();
            rt.last_candidate = None;
            // The cards are gone, and with them what their buttons needed: a waiting
            // request that is ready gets a card of its own again.
            rt.ready_carded.clear();
        }
        self.seed_day_count();
    }

    /// Emits what a method queued while it held the lock, and shows its cards.
    pub(crate) fn flush(&self, outbox: Outbox) {
        for (card, notice) in outbox.cards {
            self.present(&card, notice);
        }
        for event in outbox.events {
            self.deps.sink.emit(event);
        }
        if outbox.status {
            self.refresh_status();
        }
    }

    /// Recomputes the status and tells the shell when it changed.
    pub(crate) fn refresh_status(&self) -> NofapStatus {
        // Held until the status is stored and sent: a pass that read the configuration
        // just before the module was switched off must not write "protected" over the
        // "off" of the switch, nor send it afterwards.
        let _one_at_a_time = self.status_gate.lock();
        let status = self.compute_status();
        let changed = {
            let mut rt = self.rt.lock();
            let changed = !same_status(&rt.status, &status);
            rt.status = status.clone();
            changed
        };
        if changed {
            self.deps.sink.emit(NofapEvent::Status {
                status: status.clone(),
            });
        }
        status
    }
}

/// Whether two statuses say the same thing. The seconds a waiting request still has left
/// run down by themselves (the page counts them locally): they are not news.
fn same_status(before: &NofapStatus, after: &NofapStatus) -> bool {
    let settled = |status: &NofapStatus| {
        let mut status = status.clone();
        for waiting in &mut status.pending {
            waiting.remaining_secs = 0;
        }
        status
    };
    settled(before) == settled(after)
}

/// The salt learned and snoozed sites are hashed with; zeros before the first enable
/// (nothing is hashed with it then: a learned site needs the module on).
pub(crate) fn salt_of(config: &NofapConfig) -> [u8; 16] {
    let mut salt = [0u8; 16];
    let hex = config.internal.salt_hex.as_bytes();
    if hex.len() == 32 {
        for (index, pair) in hex.chunks(2).enumerate() {
            let digit = |byte: u8| (byte as char).to_digit(16).unwrap_or(0);
            // Two hexadecimal digits are at most 255.
            salt[index] = u8::try_from(digit(pair[0]) * 16 + digit(pair[1])).unwrap_or(0);
        }
    }
    salt
}

/// Gives the configuration a salt when it has none: 16 random bytes in hex.
pub(crate) fn ensure_salt(config: &mut NofapConfig) -> bool {
    if config.internal.salt_hex.len() == 32 {
        return false;
    }
    let bytes: [u8; 16] = rand::random();
    config.internal.salt_hex = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    true
}

fn next_snooze_end(sites: &[NofapSite], now: DateTime<Utc>) -> Option<DateTime<Utc>> {
    sites
        .iter()
        .filter(|site| site.origin == SiteOrigin::Snooze)
        .filter_map(|site| site.expires_at)
        .filter(|ends| !snooze_over(*ends, now))
        .min()
}

/// Whether a snooze that ends at `ends` is over at `now`: it ended, or it ends further
/// ahead than a snooze lasts, which no snooze set under the present clock can (the clock
/// was set back since). A snooze is never stretched by a clock change.
pub(crate) fn snooze_over(ends: DateTime<Utc>, now: DateTime<Utc>) -> bool {
    ends <= now || (ends - now).num_seconds() > SNOOZE_SECS
}

/// The sites the matcher is built from: all of them but the snoozes that are over by
/// [`snooze_over`] (the matcher drops the ones that ended by itself; the rows of the
/// others stay until their time passes, and count towards the snoozes of the day).
fn live_sites(sites: &[NofapSite], now: DateTime<Utc>) -> Vec<NofapSite> {
    sites
        .iter()
        .filter(|site| {
            site.origin != SiteOrigin::Snooze
                || site.expires_at.is_some_and(|ends| !snooze_over(ends, now))
        })
        .cloned()
        .collect()
}

/// Whether `at` lies in the last [`HIT_MEMORY_SECS`] before `now`.
pub(crate) fn recent(at: Option<&DateTime<Utc>>, now: DateTime<Utc>) -> bool {
    at.is_some_and(|at| (0..HIT_MEMORY_SECS).contains(&(now - *at).num_seconds()))
}

/// Remembers where a hit was, for [`NofapService::covers_open_block`].
pub(crate) fn mark_hit(rt: &mut Runtime, app_id: &str, host: Option<&str>, now: DateTime<Utc>) {
    rt.hit_marks.retain(|mark| recent(Some(&mark.at), now));
    if rt.hit_marks.len() >= HIT_MARKS_KEPT {
        rt.hit_marks.remove(0);
    }
    rt.hit_marks.push(HitMark {
        app_id: app_id.to_owned(),
        host: host.map(str::to_owned),
        at: now,
    });
}

/// Drops remembered keys that are older than the tracker needs them.
pub(crate) fn forget_old(keys: &mut HashMap<u64, DateTime<Utc>>, now: DateTime<Utc>) {
    if keys.len() > 256 {
        keys.retain(|_, at| recent(Some(at), now));
    }
}

/// The page answers this error with one action, "Restaurar padrões" (a reset that stores a
/// fresh configuration, integrator decision 33), so the text names that way out.
pub(crate) fn unreadable_config() -> CoreError {
    CoreError::Storage(
        "the stored configuration of the module cannot be read; restore the defaults to set it up again"
            .into(),
    )
}

/// The kind of an error without its text. An error text can quote what a platform call
/// was about (an application, a path), so log lines of this module carry the kind only.
pub(crate) fn error_class(error: &CoreError) -> &'static str {
    match error {
        CoreError::Storage(_) => "storage",
        CoreError::Platform(_) => "platform",
        CoreError::Permission(_) => "permission",
        CoreError::Ai(_) => "ai",
        CoreError::AiNotConfigured => "ai_not_configured",
        CoreError::AiRejected(_) => "ai_rejected",
        CoreError::AiRefused => "ai_refused",
        CoreError::RateLimited { .. } => "rate_limited",
        CoreError::LicenseRequired => "license_required",
        CoreError::Invalid(_) => "invalid",
        CoreError::NotFound(_) => "not_found",
        _ => "other",
    }
}

/// The key of a window for the tracker's hooks and the sampler: the host when there is
/// one, otherwise the application and the title.
pub(crate) fn context_key(host: Option<&str>, app_id: &str, title: &str) -> u64 {
    sampling::sample_key(host, app_id, title)
}

// ---------------------------------------------------------------------------------------------
// The two loops a host runs
// ---------------------------------------------------------------------------------------------

/// The guard loop: every `every` one [`NofapService::guard_pass`] on the blocking pool
/// (the first tick is skipped). `on_hit` runs after a pass or a check that acted, also on
/// the blocking pool (ubiqX scrubs the tracker's open block there).
///
/// The AI check a pass asks for is **spawned, not awaited**: it can take half a minute,
/// and the guard keeps ticking meanwhile.
pub async fn run_guard_loop(
    service: Arc<NofapService>,
    every: Duration,
    cancel: CancellationToken,
    on_hit: Arc<dyn Fn() + Send + Sync>,
) {
    let mut tick = tokio::time::interval(every);
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    tick.tick().await;
    loop {
        tokio::select! {
            // A cancel already signalled wins over a tick due at the same moment: no pass
            // starts after the host asked the loop to stop (integrator decision 37).
            biased;
            _ = cancel.cancelled() => break,
            _ = tick.tick() => {}
        }
        let pass = {
            let service = service.clone();
            let on_hit = on_hit.clone();
            tokio::task::spawn_blocking(move || {
                let outcome = service.guard_pass();
                if matches!(&outcome, Ok(outcome) if outcome.hit) {
                    on_hit();
                }
                outcome
            })
            .await
        };
        match pass {
            Ok(Ok(outcome)) => {
                if let Some(job) = outcome.job {
                    let service = service.clone();
                    let on_hit = on_hit.clone();
                    tokio::spawn(async move {
                        if matches!(service.vision_check(job).await, Ok(outcome) if outcome.hit) {
                            let _ = tokio::task::spawn_blocking(move || on_hit()).await;
                        }
                    });
                }
            }
            Ok(Err(error)) => {
                tracing::debug!(class = error_class(&error), "nofap: a guard pass failed");
            }
            Err(_) => tracing::warn!("nofap: a guard pass panicked"),
        }
    }
    let _ = tokio::task::spawn_blocking(move || service.release_primary()).await;
}

/// The verify loop: every `every` one [`NofapService::verify_pass`] on the blocking pool
/// (the first tick is skipped, like the engine's other loops; the page asks for a read of
/// its own when it opens, and so does the quit dialog).
pub async fn run_verify_loop(
    service: Arc<NofapService>,
    every: Duration,
    cancel: CancellationToken,
) {
    let mut tick = tokio::time::interval(every);
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    tick.tick().await;
    loop {
        tokio::select! {
            // A cancel already signalled wins over a tick due at the same moment: no pass
            // starts after the host asked the loop to stop (integrator decision 37).
            biased;
            _ = cancel.cancelled() => break,
            _ = tick.tick() => {}
        }
        let service = service.clone();
        match tokio::task::spawn_blocking(move || service.verify_pass()).await {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                tracing::debug!(class = error_class(&error), "nofap: a verify pass failed");
            }
            Err(_) => tracing::warn!("nofap: a verify pass panicked"),
        }
    }
}

/// Marks one AI check as running; dropping it lets the next one start.
pub(crate) struct InFlight(pub(crate) Arc<AtomicBool>);

impl InFlight {
    /// `None` while another check runs.
    pub(crate) fn begin(flag: &Arc<AtomicBool>) -> Option<Self> {
        flag.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .ok()
            .map(|_| Self(flag.clone()))
    }
}

impl Drop for InFlight {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}
