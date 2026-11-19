//! The `nofap` module's shared vocabulary: configuration, status, the system plan, cards,
//! events and the ports its runtime is built from. Types and traits only; the decisions live
//! in the pure crate `ubiqx-nofap` and the runtime in `ubiqx_engine::nofap`.
//!
//! Three names, three rings (see `docs/nofap/DESIGN.md`, section 0.1):
//!
//! * **source**: `nofap` (crates, modules, tables, IPC commands, i18n namespaces);
//! * **machine**: [`SYSTEM_ID`], everything written to the user's machine or published
//!   (hosts-file marker, state directory, helper flag, bundle identifier, artefact names);
//! * **display**: [`DISPLAY_NAME`], the single place the user-visible name is defined.
//!
//! Privacy rule enforced by the tests at the bottom: nothing that reaches a window, a
//! notification or an event ([`NofapCard`], [`NofapEvent`], [`NofapStatus`]) has a field for
//! a host, a URL, a window title or an application.

use std::collections::BTreeMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::CoreResult;
use crate::lang::UiLanguage;
use crate::model::{AiProvider, Id};
use crate::ports::EncodedImage;
use crate::product::ProductId;

/// The name users see: sidebar label, the standalone product's name, window titles and
/// every sentence that names the feature (through the `{name}` placeholder).
pub const DISPLAY_NAME: &str = "NoFap";

/// The neutral name the operating system sees: hosts-file marker, system state directory,
/// policy and packaging file names, the helper flag, the card window's label, export names.
pub const SYSTEM_ID: &str = "ubiqx-shield";

/// Version of [`NofapConfig`] and of [`SystemPlan`].
pub const CONFIG_VERSION: u8 = 1;

/// Commitment delay: default and upper bound, in seconds.
pub const DELAY_DEFAULT_SECS: u32 = 600;
pub const DELAY_MAX_SECS: u32 = 86_400;
/// Most loosening requests that may wait at once.
pub const MAX_PENDING: usize = 20;
/// Longest values sentence, in characters.
pub const VALUES_MAX_CHARS: usize = 280;
/// Most "if… then…" plans, and the longest half of one, in characters.
pub const MAX_PLANS: usize = 3;
pub const PLAN_TEXT_MAX_CHARS: usize = 140;
/// Most user-typed hostnames a [`SystemPlan`] may carry.
pub const MAX_PLAN_EXTRA: usize = 400;

/// Vision caps: defaults and ranges.
pub const VISION_PER_HOUR_DEFAULT: u32 = 20;
pub const VISION_PER_HOUR_RANGE: (u32, u32) = (1, 60);
pub const VISION_PER_DAY_DEFAULT: u32 = 120;
pub const VISION_PER_DAY_RANGE: (u32, u32) = (1, 1_000);
pub const VISION_MONTHLY_USD_DEFAULT: f64 = 2.0;
pub const VISION_MONTHLY_USD_RANGE: (f64, f64) = (0.25, 20.0);

// ---------------------------------------------------------------------------------------------
// Operating systems, tiers, resolvers
// ---------------------------------------------------------------------------------------------

/// The three systems the system layer knows. Every back-end compiles on every OS; this
/// value picks one at run time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Os {
    Macos,
    Windows,
    Linux,
}

impl Os {
    pub const ALL: [Os; 3] = [Os::Macos, Os::Windows, Os::Linux];

    /// The system this binary was built for; `None` on anything else.
    pub fn current() -> Option<Os> {
        match std::env::consts::OS {
            "macos" => Some(Os::Macos),
            "windows" => Some(Os::Windows),
            "linux" => Some(Os::Linux),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Os::Macos => "macos",
            Os::Windows => "windows",
            Os::Linux => "linux",
        }
    }
}

/// A system tier: a change to the machine that keeps working while the app is closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tier {
    /// L2: the marked block in the hosts file.
    Hosts,
    /// L3: system DNS pointed at the family resolver.
    Resolver,
    /// L4: browser policies (browser DoH off, SafeSearch on).
    Policies,
}

impl Tier {
    pub const ALL: [Tier; 3] = [Tier::Hosts, Tier::Resolver, Tier::Policies];

    pub fn as_str(self) -> &'static str {
        match self {
            Tier::Hosts => "hosts",
            Tier::Resolver => "resolver",
            Tier::Policies => "policies",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "hosts" => Some(Tier::Hosts),
            "resolver" => Some(Tier::Resolver),
            "policies" => Some(Tier::Policies),
            _ => None,
        }
    }
}

/// A filtering resolver from the compiled table (`ubiqx_nofap::resolvers`). The helper
/// accepts an id, never an address.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolverId {
    CloudflareFamily,
}

impl ResolverId {
    pub const ALL: [ResolverId; 1] = [ResolverId::CloudflareFamily];

    pub fn as_str(self) -> &'static str {
        match self {
            ResolverId::CloudflareFamily => "cloudflare_family",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "cloudflare_family" => Some(ResolverId::CloudflareFamily),
            _ => None,
        }
    }
}

/// Browser families the policy tier knows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserFamily {
    Chrome,
    Chromium,
    Edge,
    Brave,
    Firefox,
}

impl BrowserFamily {
    pub const ALL: [BrowserFamily; 5] = [
        BrowserFamily::Chrome,
        BrowserFamily::Chromium,
        BrowserFamily::Edge,
        BrowserFamily::Brave,
        BrowserFamily::Firefox,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            BrowserFamily::Chrome => "chrome",
            BrowserFamily::Chromium => "chromium",
            BrowserFamily::Edge => "edge",
            BrowserFamily::Brave => "brave",
            BrowserFamily::Firefox => "firefox",
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Configuration (one JSON document in the single row of `nofap_state`)
// ---------------------------------------------------------------------------------------------

/// The module's configuration. Not part of `Settings`, so the UI's whole-object settings
/// save, "restaurar padrões" and "Apagar todos os dados" cannot flip it; it changes only
/// through typed requests (`NofapChange`). `{}` deserialises to the default, so a new field
/// needs no migration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct NofapConfig {
    pub v: u8,
    pub enabled: bool,
    pub enabled_at: Option<DateTime<Utc>>,
    /// The wizard's "é para mim, no meu computador" tick.
    pub self_use_at: Option<DateTime<Utc>>,
    pub system: SystemPrefs,
    pub vision: VisionPrefs,
    /// How long a request that weakens protection waits, in seconds (`0..=DELAY_MAX_SECS`).
    pub delay_secs: u32,
    /// Loosening requests waiting out the delay (at most [`MAX_PENDING`]).
    pub pending: Vec<PendingChange>,
    pub ui: UiPrefs,
    /// The user's own sentence (at most [`VALUES_MAX_CHARS`] characters).
    pub values: String,
    /// "Se… então…" plans (at most [`MAX_PLANS`]).
    pub plans: Vec<IfThenPlan>,
    /// Bookkeeping that never leaves the database: not exported, not sent to the UI.
    pub internal: Internal,
}

impl Default for NofapConfig {
    fn default() -> Self {
        Self {
            v: CONFIG_VERSION,
            enabled: false,
            enabled_at: None,
            self_use_at: None,
            system: SystemPrefs::default(),
            vision: VisionPrefs::default(),
            delay_secs: DELAY_DEFAULT_SECS,
            pending: Vec::new(),
            ui: UiPrefs::default(),
            values: String::new(),
            plans: Vec::new(),
            internal: Internal::default(),
        }
    }
}

impl NofapConfig {
    /// Clamps every range and length to its documented bound. Applied after every load and
    /// before every save, so a hand-edited row cannot carry an out-of-range value.
    pub fn sanitize(&mut self) {
        self.v = CONFIG_VERSION;
        self.delay_secs = self.delay_secs.min(DELAY_MAX_SECS);
        self.pending.truncate(MAX_PENDING);
        self.values = truncate_chars(self.values.trim(), VALUES_MAX_CHARS);
        self.plans.truncate(MAX_PLANS);
        for plan in &mut self.plans {
            plan.when = truncate_chars(plan.when.trim(), PLAN_TEXT_MAX_CHARS);
            plan.then = truncate_chars(plan.then.trim(), PLAN_TEXT_MAX_CHARS);
        }
        self.vision.sanitize();
    }
}

fn truncate_chars(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

/// Which system tiers the user asked for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SystemPrefs {
    pub hosts: bool,
    pub resolver: Option<ResolverId>,
    pub policies: bool,
    /// Whether sites the user adds also go into the (world-readable) hosts block. `false` is
    /// "guardar só neste app".
    pub hosts_user_sites: bool,
    /// The user pressed "Restaurar minha rede": the resolver stays chosen but is not applied
    /// until "Religar".
    pub network_restored: bool,
}

impl Default for SystemPrefs {
    fn default() -> Self {
        Self {
            hosts: false,
            resolver: None,
            policies: false,
            hosts_user_sites: true,
            network_restored: false,
        }
    }
}

/// The AI check's switches and caps.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct VisionPrefs {
    pub enabled: bool,
    /// What the user agreed to; a different provider selected later pauses vision
    /// ([`Issue::ConsentOutdated`]).
    pub consent: Option<VisionConsent>,
    pub unknown_sites: bool,
    pub media_apps: bool,
    /// Off by default: other people's messages.
    pub messaging_apps: bool,
    pub private_windows: bool,
    pub max_per_hour: u32,
    pub max_per_day: u32,
    pub monthly_cap_usd: f64,
}

impl Default for VisionPrefs {
    fn default() -> Self {
        Self {
            enabled: false,
            consent: None,
            unknown_sites: true,
            media_apps: true,
            messaging_apps: false,
            private_windows: true,
            max_per_hour: VISION_PER_HOUR_DEFAULT,
            max_per_day: VISION_PER_DAY_DEFAULT,
            monthly_cap_usd: VISION_MONTHLY_USD_DEFAULT,
        }
    }
}

impl VisionPrefs {
    /// Clamps the caps to their ranges; a cap that is not a number becomes the default.
    pub fn sanitize(&mut self) {
        let (lo, hi) = VISION_PER_HOUR_RANGE;
        self.max_per_hour = self.max_per_hour.clamp(lo, hi);
        let (lo, hi) = VISION_PER_DAY_RANGE;
        self.max_per_day = self.max_per_day.clamp(lo, hi);
        let (lo, hi) = VISION_MONTHLY_USD_RANGE;
        self.monthly_cap_usd = if self.monthly_cap_usd.is_finite() {
            self.monthly_cap_usd.clamp(lo, hi)
        } else {
            VISION_MONTHLY_USD_DEFAULT
        };
    }
}

/// The consent the user gave for the AI check: which provider, and whether the managed
/// route ("IA do Ubi", through the operator's proxy) was ticked too.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisionConsent {
    pub provider: AiProvider,
    pub managed: bool,
    pub at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct UiPrefs {
    /// The first thing on screen after a block says nothing about why.
    pub discreet: bool,
    pub show_streak: bool,
    pub daily_checkin: bool,
}

impl Default for UiPrefs {
    fn default() -> Self {
        Self {
            discreet: true,
            show_streak: false,
            daily_checkin: false,
        }
    }
}

/// "Se eu sentir vontade quando `when`, então eu vou `then`."
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct IfThenPlan {
    pub when: String,
    pub then: String,
}

/// Bookkeeping kept beside the configuration. Never exported and never sent to the UI.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Internal {
    /// 16 random bytes in hex, created at first enable: the salt of learned-site hashes.
    pub salt_hex: String,
    /// When each entry of ubiqX's privacy lists was first seen (see
    /// `ubiqx_nofap::exclusions`).
    pub exclusions_seen: Vec<ExclusionSeen>,
    pub last_help_at: Option<DateTime<Utc>>,
    pub last_reminder_at: Option<DateTime<Utc>>,
    pub last_drift_card_at: Option<DateTime<Utc>>,
    /// The DNS servers in place before the resolver tier was applied. For display only:
    /// nothing here is ever written back to the system.
    pub previous_dns: Vec<PreviousDns>,
}

/// First sighting of one privacy-list entry. `key` is `app:<id>` or `domain:<name>`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExclusionSeen {
    pub key: String,
    pub first_seen_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreviousDns {
    pub interface: String,
    pub servers: Vec<String>,
}

/// A request that weakens protection, waiting out the delay.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PendingChange {
    pub id: Id,
    pub change: NofapChange,
    pub requested_at: DateTime<Utc>,
    /// Fixed at request time (`requested_at` plus the delay then in force).
    pub ready_at: DateTime<Utc>,
    /// Seconds the app was observed running since the request (grows by at most 150 s per
    /// verify pass), so a clock moved forward still costs real time.
    #[serde(default)]
    pub observed_secs: u32,
    /// When the request first became ready; it expires 24 hours later without a
    /// confirmation.
    #[serde(default)]
    pub ready_since: Option<DateTime<Utc>>,
}

/// Every way the configuration or the lists can change. `ubiqx_nofap::commitment::classify`
/// decides, per change, whether it takes effect now or after the delay.
///
/// Wire form: `{ "kind": "<snake_case variant>", …fields }`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NofapChange {
    Disable,
    AddBlock {
        host: String,
    },
    RemoveBlock {
        id: Id,
    },
    AddAllow {
        host: String,
    },
    RemoveAllow {
        id: Id,
    },
    ForgetLearned,
    SetHosts {
        on: bool,
    },
    SetResolver {
        resolver: Option<ResolverId>,
    },
    SetPolicies {
        on: bool,
    },
    SetHostsUserSites {
        on: bool,
    },
    SetVision {
        on: bool,
    },
    SetVisionScope {
        unknown_sites: bool,
        media_apps: bool,
        messaging_apps: bool,
        private_windows: bool,
    },
    SetCaps {
        max_per_hour: u32,
        max_per_day: u32,
        monthly_cap_usd: f64,
    },
    SetDelay {
        secs: u32,
    },
    SetPrefs {
        prefs: UiPrefs,
    },
    SetValues {
        text: String,
    },
    SetPlans {
        plans: Vec<IfThenPlan>,
    },
}

impl NofapChange {
    /// The discriminant, which is all a status or an event may say about a change.
    pub fn kind(&self) -> ChangeKind {
        match self {
            NofapChange::Disable => ChangeKind::Disable,
            NofapChange::AddBlock { .. } => ChangeKind::AddBlock,
            NofapChange::RemoveBlock { .. } => ChangeKind::RemoveBlock,
            NofapChange::AddAllow { .. } => ChangeKind::AddAllow,
            NofapChange::RemoveAllow { .. } => ChangeKind::RemoveAllow,
            NofapChange::ForgetLearned => ChangeKind::ForgetLearned,
            NofapChange::SetHosts { .. } => ChangeKind::SetHosts,
            NofapChange::SetResolver { .. } => ChangeKind::SetResolver,
            NofapChange::SetPolicies { .. } => ChangeKind::SetPolicies,
            NofapChange::SetHostsUserSites { .. } => ChangeKind::SetHostsUserSites,
            NofapChange::SetVision { .. } => ChangeKind::SetVision,
            NofapChange::SetVisionScope { .. } => ChangeKind::SetVisionScope,
            NofapChange::SetCaps { .. } => ChangeKind::SetCaps,
            NofapChange::SetDelay { .. } => ChangeKind::SetDelay,
            NofapChange::SetPrefs { .. } => ChangeKind::SetPrefs,
            NofapChange::SetValues { .. } => ChangeKind::SetValues,
            NofapChange::SetPlans { .. } => ChangeKind::SetPlans,
        }
    }
}

/// The kind of a [`NofapChange`], without its payload. Serialises to the same string as the
/// change's `kind` tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    Disable,
    AddBlock,
    RemoveBlock,
    AddAllow,
    RemoveAllow,
    ForgetLearned,
    SetHosts,
    SetResolver,
    SetPolicies,
    SetHostsUserSites,
    SetVision,
    SetVisionScope,
    SetCaps,
    SetDelay,
    SetPrefs,
    SetValues,
    SetPlans,
}

impl ChangeKind {
    pub const ALL: [ChangeKind; 17] = [
        ChangeKind::Disable,
        ChangeKind::AddBlock,
        ChangeKind::RemoveBlock,
        ChangeKind::AddAllow,
        ChangeKind::RemoveAllow,
        ChangeKind::ForgetLearned,
        ChangeKind::SetHosts,
        ChangeKind::SetResolver,
        ChangeKind::SetPolicies,
        ChangeKind::SetHostsUserSites,
        ChangeKind::SetVision,
        ChangeKind::SetVisionScope,
        ChangeKind::SetCaps,
        ChangeKind::SetDelay,
        ChangeKind::SetPrefs,
        ChangeKind::SetValues,
        ChangeKind::SetPlans,
    ];
}

// ---------------------------------------------------------------------------------------------
// Status (one state per layer; never a host)
// ---------------------------------------------------------------------------------------------

/// What the module is doing right now, layer by layer. Carried by [`NofapEvent::Status`]
/// and by the page's view.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NofapStatus {
    pub overall: Overall,
    /// L1.
    pub guard: GuardState,
    /// L2.
    pub hosts: TierState,
    /// L3.
    pub resolver: TierState,
    /// L4.
    pub policies: TierState,
    /// L5.
    pub vision: VisionState,
    /// Each issue has copy and at most one action in the UI.
    pub issues: Vec<Issue>,
    pub pending: Vec<PendingView>,
    /// Names waiting to enter the hosts block at the next apply the user starts
    /// ([`TierState::UpdatePending`]).
    pub hosts_waiting: u32,
    /// User sites that do not fit the plan argument on this system and stay in the in-app
    /// guard only.
    pub extras_left_out: u32,
}

impl NofapStatus {
    /// The status of a module that is switched off.
    pub fn off() -> Self {
        Self {
            overall: Overall::Off,
            guard: GuardState::Off,
            hosts: TierState::Off,
            resolver: TierState::Off,
            policies: TierState::Off,
            vision: VisionState::Off {
                reason: VisionOff::Disabled,
            },
            issues: Vec::new(),
            pending: Vec::new(),
            hosts_waiting: 0,
            extras_left_out: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Overall {
    Off,
    /// Shown only with the list of layers that are on.
    Protected,
    /// Something chosen is not in place, or nothing is stopping sites in the window in
    /// front.
    Reduced,
    /// A confirmed or waiting `Disable` request exists.
    PendingOff,
    /// The other product on this machine runs the passes.
    Secondary,
}

impl Overall {
    pub const ALL: [Overall; 5] = [
        Overall::Off,
        Overall::Protected,
        Overall::Reduced,
        Overall::PendingOff,
        Overall::Secondary,
    ];
}

/// L1, the guard. Wire form: `{ "state": "on" }`, `{ "state": "limited", "reason": … }`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum GuardState {
    On,
    Off,
    Limited { reason: GuardLimit },
}

/// Why the guard cannot read the address of the window in front.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GuardLimit {
    /// macOS: Automation denied for the browser in front.
    AutomationDenied,
    /// A browser the URL resolver does not support.
    BrowserWithoutUrl,
    /// Linux X11: the host is known only when the title prints it.
    TitleOnly,
    /// Linux Wayland: no foreground window and no capture.
    Wayland,
}

impl GuardLimit {
    pub const ALL: [GuardLimit; 4] = [
        GuardLimit::AutomationDenied,
        GuardLimit::BrowserWithoutUrl,
        GuardLimit::TitleOnly,
        GuardLimit::Wayland,
    ];
}

/// State of one system tier. Wire form: `{ "state": "applied" }`,
/// `{ "state": "unsupported", "reason": "write_refused" }`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum TierState {
    /// Not chosen.
    Off,
    Applied,
    /// In place and intact, but the app would now write something else (a site the user
    /// added, a new app version). Never a prompt by itself.
    UpdatePending,
    /// In place but not what was written (drift).
    Changed,
    /// The state file lists it for this product and it is gone (drift).
    Missing,
    /// Chosen, never applied or the OS prompt was declined.
    NotApplied,
    /// Closed for this operating system until its checks pass.
    Gated,
    /// Cannot be done on this machine; `reason` is a short code (`write_refused`,
    /// `appimage`, `no_resolved`, `flatpak`, `not_built`, …).
    Unsupported {
        reason: String,
    },
    /// Applied for the other product only.
    InUseByOther,
}

/// L5, the AI check.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum VisionState {
    Off {
        reason: VisionOff,
    },
    On,
    /// The provider failed repeatedly; checks resume at `until`.
    Degraded {
        until: DateTime<Utc>,
    },
}

/// Why the AI check is not running, in the order the readiness check tests them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VisionOff {
    Disabled,
    ConsentOutdated,
    NoProvider,
    NoKey,
    LocalOnly,
    NotOnboarded,
    Licence,
    ScreenRecording,
    Wayland,
    BudgetSpent,
    CapReached,
}

impl VisionOff {
    pub const ALL: [VisionOff; 11] = [
        VisionOff::Disabled,
        VisionOff::ConsentOutdated,
        VisionOff::NoProvider,
        VisionOff::NoKey,
        VisionOff::LocalOnly,
        VisionOff::NotOnboarded,
        VisionOff::Licence,
        VisionOff::ScreenRecording,
        VisionOff::Wayland,
        VisionOff::BudgetSpent,
        VisionOff::CapReached,
    ];
}

/// Something the page tells the user about, each with its own copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Issue {
    /// The canary did not resolve to the null address although the hosts block is applied.
    HostsNotHonoured,
    /// A host that is in the applied hosts block was seen in front.
    DnsBypassSuspected,
    /// Names stopped resolving while the resolver tier is applied.
    ResolverUnreachable,
    /// A connection that appeared after the apply does not use the filtered resolver.
    NewConnectionUnfiltered,
    /// A browser's policy location holds someone else's value; left alone.
    PolicyConflict,
    /// macOS: Chromium-family browsers get no policy.
    PolicyAdvisoryOnly,
    AutomationDenied,
    ScreenRecordingDenied,
    VisionBudgetSpent,
    VisionCapReached,
    VisionNoProvider,
    /// The AI provider changed since the consent was given.
    ConsentOutdated,
    /// The other product on this machine runs the passes.
    OtherProductPrimary,
}

impl Issue {
    pub const ALL: [Issue; 13] = [
        Issue::HostsNotHonoured,
        Issue::DnsBypassSuspected,
        Issue::ResolverUnreachable,
        Issue::NewConnectionUnfiltered,
        Issue::PolicyConflict,
        Issue::PolicyAdvisoryOnly,
        Issue::AutomationDenied,
        Issue::ScreenRecordingDenied,
        Issue::VisionBudgetSpent,
        Issue::VisionCapReached,
        Issue::VisionNoProvider,
        Issue::ConsentOutdated,
        Issue::OtherProductPrimary,
    ];
}

/// A waiting request as the status shows it: its kind and its clock, never its payload
/// (an allow request names a host; the page reads that from the configuration it asked
/// for, not from an event).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingView {
    pub id: Id,
    pub kind: ChangeKind,
    pub requested_at: DateTime<Utc>,
    pub ready_at: DateTime<Utc>,
    /// The delay has passed and enough running time was observed: the request waits for
    /// the user's confirmation.
    pub ready: bool,
    /// Seconds until it can be confirmed (0 once ready).
    pub remaining_secs: u32,
    /// When a ready request lapses without a confirmation.
    pub expires_at: Option<DateTime<Utc>>,
}

// ---------------------------------------------------------------------------------------------
// The system layer (L2, L3, L4)
// ---------------------------------------------------------------------------------------------

/// What one product wants applied to the machine. Travels to the elevated helper as a
/// single argument (`--plan-b64`), so the helper reads no file from a user-writable place.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemPlan {
    /// [`CONFIG_VERSION`].
    pub v: u8,
    pub owner: ProductId,
    pub hosts: bool,
    /// Hostnames the user typed: lower-case, without `www.` (the helper writes the `www.`
    /// twin itself), at most [`MAX_PLAN_EXTRA`]. Learned sites are never here.
    pub extra: Vec<String>,
    pub resolver: Option<ResolverId>,
    pub policies: bool,
}

/// What `remove` drops for an owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RemoveScope {
    All,
    /// Only the resolver ("Restaurar minha rede"); the other tiers stay.
    Resolver,
}

impl RemoveScope {
    pub fn as_str(self) -> &'static str {
        match self {
            RemoveScope::All => "all",
            RemoveScope::Resolver => "resolver",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "all" => Some(RemoveScope::All),
            "resolver" => Some(RemoveScope::Resolver),
            _ => None,
        }
    }
}

/// Whether a tier can be applied on this machine at all.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum TierCapability {
    Available,
    /// Closed by the per-OS gate.
    Gated,
    Unsupported {
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemCapabilities {
    /// `None` on a system without a back-end.
    pub os: Option<Os>,
    pub hosts: TierCapability,
    pub resolver: TierCapability,
    pub policies: TierCapability,
    /// Tiers a tester opened with `UBIQX_SHIELD_UNVERIFIED_TIERS`; the page labels them
    /// "em teste".
    pub unverified: Vec<Tier>,
}

impl SystemCapabilities {
    /// No tier can be applied (no back-end, or a shell without a system layer).
    pub fn none(reason: &str) -> Self {
        let unsupported = || TierCapability::Unsupported {
            reason: reason.to_string(),
        };
        Self {
            os: Os::current(),
            hosts: unsupported(),
            resolver: unsupported(),
            policies: unsupported(),
            unverified: Vec::new(),
        }
    }

    pub fn of(&self, tier: Tier) -> &TierCapability {
        match tier {
            Tier::Hosts => &self.hosts,
            Tier::Resolver => &self.resolver,
            Tier::Policies => &self.policies,
        }
    }
}

/// What an unprivileged look at the machine found: raw text and lists, no judgement.
/// `ubiqx_nofap::plan::assess` turns it into [`TierState`]s, so the adapter holds no
/// decision. Printed as JSON by the helper's `status` verb.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SystemObservation {
    pub hosts: HostsObservation,
    pub resolver: ResolverObservation,
    pub policies: Vec<PolicyObservation>,
    /// The helper's state file, when present and readable.
    pub state: Option<SystemState>,
    /// Short code when the state file exists but could not be read or parsed.
    pub state_error: Option<String>,
    /// Addresses the hosts canary resolved to through the OS: `None` when this observation
    /// was not asked to look it up ([`ObserveOpts::canary`]), empty when it did not resolve.
    pub canary: Option<Vec<String>>,
    /// Whether the probe name resolved through the OS; `None` when this observation was not
    /// asked to look it up ([`ObserveOpts::probe`]).
    pub probe_ok: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct HostsObservation {
    /// The hosts file as text (invalid UTF-8 replaced); `None` when it could not be read.
    pub text: Option<String>,
    /// Short code when the tier cannot work here (`unreadable`, `appimage`,
    /// `write_refused`, …).
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ResolverObservation {
    /// macOS network services and Windows adapters. Empty on Linux.
    pub interfaces: Vec<InterfaceDns>,
    /// Linux: the content of our systemd-resolved drop-in, when the file exists.
    pub dropin: Option<String>,
    /// Linux: whether systemd-resolved is active.
    pub resolved_active: Option<bool>,
    /// Linux: whether `/etc/resolv.conf` points at systemd-resolved.
    pub resolv_conf_uses_resolved: Option<bool>,
    /// Windows: `true` for the full read (PowerShell), `false` for the quick registry read
    /// ([`ObserveOpts::full`]). A quick read that found nothing usable sets `error`, and the
    /// verify pass then asks for the full read every 10 minutes.
    pub full: bool,
    /// Short code when DNS could not be read at all.
    pub error: Option<String>,
}

/// The DNS servers configured by hand on one interface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterfaceDns {
    /// macOS: the service name. Windows: the interface index or GUID.
    pub id: String,
    /// Name shown to the user in the preview.
    pub name: String,
    /// Enabled (macOS) or up (Windows).
    pub active: bool,
    /// Statically configured servers of both address families; empty means automatic.
    pub servers: Vec<String>,
}

/// One policy value of a browser: a name and its JSON value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PolicyValue {
    pub key: String,
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PolicyObservation {
    pub browser: BrowserFamily,
    /// Whether the browser's policy location exists on this machine.
    pub present: bool,
    /// Values of the keys this tier manages, as found in the registry, in our own policy
    /// file or in the preferences domain.
    #[serde(default)]
    pub values: Vec<PolicyValue>,
    /// Linux: the same keys set by a policy file that is not ours.
    #[serde(default)]
    pub foreign: Vec<PolicyValue>,
    /// Short code when the browser cannot be handled (`flatpak`, `unreadable`, …).
    #[serde(default)]
    pub error: Option<String>,
}

/// The helper's state file (world-readable). **Nothing read back from it becomes an
/// address, a path or a command**; the only values that reach a system file again are the
/// other owner's `extra` hostnames, re-validated like a plan.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SystemState {
    pub v: u8,
    /// Unix milliseconds of the last apply.
    pub applied_at: i64,
    pub owners: BTreeMap<ProductId, OwnerWants>,
    pub hosts: Option<HostsStamp>,
    pub policies_created: Vec<PolicyCreated>,
    /// Per-tier results of the last run (what a partial apply left behind).
    pub report: Vec<TierReport>,
}

/// What one product asked the helper for.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct OwnerWants {
    pub hosts: bool,
    pub resolver: Option<ResolverId>,
    pub policies: bool,
    pub extra: Vec<String>,
}

/// Hash and size of the hosts block the helper last wrote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostsStamp {
    pub sha256: String,
    pub names: u32,
}

/// A policy value the helper created (so undo deletes only what is ours).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PolicyCreated {
    pub browser: BrowserFamily,
    pub key: String,
}

/// One line of the preview the user sees before the OS prompt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlannedChange {
    pub tier: Tier,
    /// Key in the `nofap` i18n namespace.
    pub text_key: String,
    /// What the sentence is about, computed locally: a count of names, the DNS servers that
    /// will be replaced, a browser name.
    pub detail: Option<String>,
}

/// What happened to one tier in a helper run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TierReport {
    pub tier: Tier,
    pub ok: bool,
    /// Short code when `ok` is false (`unreachable`, `write_refused`, `conflict`, `gated`,
    /// …).
    pub reason: Option<String>,
}

/// Result of an apply or a remove. Wire form: `{ "outcome": "applied" }`,
/// `{ "outcome": "partial", "detail": [TierReport] }`,
/// `{ "outcome": "unavailable", "detail": "no_polkit_agent" }`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", content = "detail", rename_all = "snake_case")]
pub enum ApplyOutcome {
    Applied,
    Partial(Vec<TierReport>),
    /// The user declined the OS prompt.
    Declined,
    /// Elevation cannot work here; the string is a short code (`no_polkit_agent`,
    /// `appimage`, `unsupported_os`, …).
    Unavailable(String),
}

/// How far an unprivileged look at the machine goes. The caller decides, on every call:
/// the adapter keeps no clock and judges nothing, so "only while the block is applied" and
/// "every 30 minutes" are rules of the verify pass, not of the adapter. The default is the
/// cheapest look: files, the quick DNS read, no name lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ObserveOpts {
    /// Windows: read DNS with the full read (PowerShell, every adapter that is up) instead
    /// of the quick registry read of the adapters seen at the last full read. Asked for at
    /// apply, when the page opens, on "Verificar agora" and every 30 minutes. The other
    /// systems have one read and ignore it.
    pub full: bool,
    /// Resolve the hosts canary through the OS ([`SystemObservation::canary`]). Asked for
    /// only while the hosts block is applied, so the reserved name never reaches a
    /// resolver from a machine without the block.
    pub canary: bool,
    /// Resolve the probe name through the OS ([`SystemObservation::probe_ok`]). Asked for
    /// only while the resolver tier is applied.
    pub probe: bool,
}

impl ObserveOpts {
    /// Files and the quick DNS read; no name lookup.
    pub const QUICK: ObserveOpts = ObserveOpts {
        full: false,
        canary: false,
        probe: false,
    };
    /// The full DNS read; no name lookup.
    pub const FULL: ObserveOpts = ObserveOpts {
        full: true,
        canary: false,
        probe: false,
    };
}

/// The system layer: three tiers behind one port. `apply` and `remove` block on the OS
/// password prompt; nothing else elevates.
pub trait SystemShield: Send + Sync {
    /// Per tier: available, gated or unsupported.
    fn capabilities(&self) -> SystemCapabilities;
    /// Unprivileged: hosts text, DNS per interface, the drop-in, policy values and the
    /// state file; the canary and the probe when `opts` asks for them.
    fn observe(&self, opts: ObserveOpts) -> CoreResult<SystemObservation>;
    /// The exact list of changes `apply` would make, for the user to read first.
    fn preview(&self, plan: &SystemPlan) -> CoreResult<Vec<PlannedChange>>;
    /// Blocks on the OS prompt.
    fn apply(&self, plan: &SystemPlan) -> CoreResult<ApplyOutcome>;
    /// Blocks on the OS prompt.
    fn remove(&self, owner: ProductId, scope: RemoveScope) -> CoreResult<ApplyOutcome>;
}

// ---------------------------------------------------------------------------------------------
// Vision (L5)
// ---------------------------------------------------------------------------------------------

/// How a frame is taken: longest edge after downscaling, and the fraction cropped from the
/// top (a browser's tab strip and address bar; `0.0` for other apps).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameOpts {
    pub max_edge: u32,
    pub crop_top_frac: f32,
}

/// One reduced image of one window, in memory only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub image: EncodedImage,
    /// 64-bit difference hash, used to skip an unchanged frame. Never stored.
    pub dhash: u64,
}

/// Captures one window. There is no display or region fallback.
pub trait FrameSource: Send + Sync {
    /// `Ok(None)` when there is nothing usable to send: the window is gone or the capture
    /// is near-uniform.
    fn frame(&self, window_id: u32, opts: FrameOpts) -> CoreResult<Option<Frame>>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExplicitVerdict {
    Explicit,
    Suggestive,
    None,
    Unsure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    High,
    Medium,
    Low,
}

/// The model's answer: a class, never a sentence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExplicitCheck {
    pub verdict: ExplicitVerdict,
    pub confidence: Confidence,
    /// The provider refused to look. A refusal alone is never a positive.
    pub refused: bool,
}

/// Asks the configured AI provider whether one image is explicit. The request carries the
/// image and two constant strings: no URL, title, app name, language or identifier.
#[async_trait]
pub trait ExplicitDetector: Send + Sync {
    async fn check(&self, image: &EncodedImage, model: &str) -> CoreResult<ExplicitCheck>;
}

// ---------------------------------------------------------------------------------------------
// Cards (the floating window)
// ---------------------------------------------------------------------------------------------

/// What the card window shows. **No host, title, URL or application field**: the engine
/// keeps what a button needs in memory, keyed by the card id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NofapCard {
    pub id: Id,
    pub kind: CardKind,
    /// Key in the `nofap` i18n namespace.
    pub message_key: String,
    /// A second line shown under the message (for example `vision.added`).
    pub note_key: Option<String>,
    pub discreet: bool,
    pub actions: Vec<CardAction>,
    /// The window closes by itself after this many seconds; `None` stays until answered.
    pub auto_close_secs: Option<u32>,
    /// After this the engine has forgotten what the buttons need.
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardKind {
    Blocked,
    BlockedRepeat,
    VisionClosed,
    VisionAsk,
    Drift,
    PendingReady,
    ResolverUnreachable,
    Reminder,
    Help,
}

impl CardKind {
    pub const ALL: [CardKind; 9] = [
        CardKind::Blocked,
        CardKind::BlockedRepeat,
        CardKind::VisionClosed,
        CardKind::VisionAsk,
        CardKind::Drift,
        CardKind::PendingReady,
        CardKind::ResolverUnreachable,
        CardKind::Reminder,
        CardKind::Help,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardAction {
    Ok,
    /// "Abrir apoio".
    Support,
    /// "Respirar 3 min".
    Surf,
    /// "Fechar a janela" (a non-browser app; the only path to `quit_app`).
    CloseWindow,
    /// "Não era isso".
    NotIt,
    /// "Pedir exceção": what "Não era isso" becomes once it would be a free pass.
    RequestException,
    /// "Adicionar à minha lista".
    AddToList,
    /// "Restaurar" (drift).
    Restore,
    /// "Manter proteção".
    KeepProtection,
    /// "Desligar" (a ready request).
    TurnOff,
    /// "Restaurar minha rede".
    RestoreNetwork,
    /// "Agora não".
    Later,
    Help,
}

impl CardAction {
    pub const ALL: [CardAction; 13] = [
        CardAction::Ok,
        CardAction::Support,
        CardAction::Surf,
        CardAction::CloseWindow,
        CardAction::NotIt,
        CardAction::RequestException,
        CardAction::AddToList,
        CardAction::Restore,
        CardAction::KeepProtection,
        CardAction::TurnOff,
        CardAction::RestoreNetwork,
        CardAction::Later,
        CardAction::Help,
    ];
}

/// Shows the card window. Implemented by the shells; `Ok(false)` means the window could not
/// be shown and the service falls back to one neutral OS notification.
pub trait NofapPresenter: Send + Sync {
    fn show(&self, card: &NofapCard) -> CoreResult<bool>;
    fn dismiss(&self, _id: &str) {}
}

// ---------------------------------------------------------------------------------------------
// Events, host environment
// ---------------------------------------------------------------------------------------------

/// What the module tells its shell. No variant has a host. Wire form:
/// `{ "type": "status", "status": … }`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NofapEvent {
    Status { status: NofapStatus },
    Card { card: NofapCard },
    CardClosed { id: Id },
    PendingChanged,
    SitesChanged,
}

/// Outbound event channel of the module (its own sink, so `EngineEvent` gains no variant).
pub trait NofapSink: Send + Sync {
    fn emit(&self, event: NofapEvent);
}

/// Whether the host has a usable AI provider for the check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiReadiness {
    Ready,
    NoKey,
    LocalOnly,
    NotOnboarded,
    LicenceRequired,
}

impl AiReadiness {
    pub const ALL: [AiReadiness; 5] = [
        AiReadiness::Ready,
        AiReadiness::NoKey,
        AiReadiness::LocalOnly,
        AiReadiness::NotOnboarded,
        AiReadiness::LicenceRequired,
    ];
}

/// What the module reads from its host, read-only: ubiqX's settings and licence, or the
/// standalone app's settings row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NofapEnvSnapshot {
    pub language: UiLanguage,
    pub provider: AiProvider,
    pub vision_model: String,
    pub ai: AiReadiness,
    /// ubiqX: `blocked_apps` and `vision_denied_apps`. Standalone: empty.
    pub excluded_apps: Vec<String>,
    /// ubiqX: `blocked_domains`. Standalone: empty.
    pub excluded_domains: Vec<String>,
}

pub trait NofapEnv: Send + Sync {
    fn snapshot(&self) -> NofapEnvSnapshot;
}

// ---------------------------------------------------------------------------------------------
// Persistence
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SiteList {
    Block,
    Allow,
}

impl SiteList {
    pub fn as_str(self) -> &'static str {
        match self {
            SiteList::Block => "block",
            SiteList::Allow => "allow",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "block" => Some(SiteList::Block),
            "allow" => Some(SiteList::Allow),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SiteOrigin {
    /// Typed by the user: stored as plain text.
    User,
    /// Learned from a vision positive: salted hash only.
    Learned,
    /// "Não era isso": salted hash, expires.
    Snooze,
}

impl SiteOrigin {
    pub const ALL: [SiteOrigin; 3] = [SiteOrigin::User, SiteOrigin::Learned, SiteOrigin::Snooze];

    pub fn as_str(self) -> &'static str {
        match self {
            SiteOrigin::User => "user",
            SiteOrigin::Learned => "learned",
            SiteOrigin::Snooze => "snooze",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "user" => Some(SiteOrigin::User),
            "learned" => Some(SiteOrigin::Learned),
            "snooze" => Some(SiteOrigin::Snooze),
            _ => None,
        }
    }
}

/// One row of `nofap_sites`. Deliberately not serialisable: a learned site must never be
/// written anywhere but the database, and there only as its hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NofapSite {
    pub id: Id,
    pub list: SiteList,
    pub origin: SiteOrigin,
    /// `None` unless `origin` is [`SiteOrigin::User`].
    pub host: Option<String>,
    /// `SHA-256(salt ‖ 0x00 ‖ host)[..16]` (`ubiqx_nofap::matcher::site_key`).
    pub host_key: [u8; 16],
    pub created_at: DateTime<Utc>,
    /// Snooze only.
    pub expires_at: Option<DateTime<Utc>>,
}

/// The counters of `nofap_day`. There is no row per event, no exact time, no host, no app.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DayKind {
    BlockList,
    BlockUser,
    BlockKeyword,
    VisionCheck,
    VisionHit,
    NotIt,
    SurfStarted,
    SurfDone,
    Panic,
    Lapse,
    CheckinOk,
    CheckinMeh,
    CheckinHard,
    CheckinVeryBad,
    TriggerTired,
    TriggerBored,
    TriggerStressed,
    TriggerLonely,
    TriggerHungry,
    TriggerAngry,
    TriggerOther,
    OverrideRequest,
    OverrideCancel,
    QuitOn,
    NetworkRestore,
    Drift,
    HelpShown,
}

impl DayKind {
    pub const ALL: [DayKind; 27] = [
        DayKind::BlockList,
        DayKind::BlockUser,
        DayKind::BlockKeyword,
        DayKind::VisionCheck,
        DayKind::VisionHit,
        DayKind::NotIt,
        DayKind::SurfStarted,
        DayKind::SurfDone,
        DayKind::Panic,
        DayKind::Lapse,
        DayKind::CheckinOk,
        DayKind::CheckinMeh,
        DayKind::CheckinHard,
        DayKind::CheckinVeryBad,
        DayKind::TriggerTired,
        DayKind::TriggerBored,
        DayKind::TriggerStressed,
        DayKind::TriggerLonely,
        DayKind::TriggerHungry,
        DayKind::TriggerAngry,
        DayKind::TriggerOther,
        DayKind::OverrideRequest,
        DayKind::OverrideCancel,
        DayKind::QuitOn,
        DayKind::NetworkRestore,
        DayKind::Drift,
        DayKind::HelpShown,
    ];

    /// The value of the `nofap_day.kind` column.
    pub fn as_str(self) -> &'static str {
        match self {
            DayKind::BlockList => "block_list",
            DayKind::BlockUser => "block_user",
            DayKind::BlockKeyword => "block_keyword",
            DayKind::VisionCheck => "vision_check",
            DayKind::VisionHit => "vision_hit",
            DayKind::NotIt => "not_it",
            DayKind::SurfStarted => "surf_started",
            DayKind::SurfDone => "surf_done",
            DayKind::Panic => "panic",
            DayKind::Lapse => "lapse",
            DayKind::CheckinOk => "checkin_ok",
            DayKind::CheckinMeh => "checkin_meh",
            DayKind::CheckinHard => "checkin_hard",
            DayKind::CheckinVeryBad => "checkin_very_bad",
            DayKind::TriggerTired => "trigger_tired",
            DayKind::TriggerBored => "trigger_bored",
            DayKind::TriggerStressed => "trigger_stressed",
            DayKind::TriggerLonely => "trigger_lonely",
            DayKind::TriggerHungry => "trigger_hungry",
            DayKind::TriggerAngry => "trigger_angry",
            DayKind::TriggerOther => "trigger_other",
            DayKind::OverrideRequest => "override_request",
            DayKind::OverrideCancel => "override_cancel",
            DayKind::QuitOn => "quit_on",
            DayKind::NetworkRestore => "network_restore",
            DayKind::Drift => "drift",
            DayKind::HelpShown => "help_shown",
        }
    }

    /// Lenient: an unknown kind (written by a newer build) is `None` and the row is
    /// skipped.
    pub fn parse(s: &str) -> Option<Self> {
        DayKind::ALL.into_iter().find(|k| k.as_str() == s)
    }
}

/// One counter of one local hour.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DayCount {
    /// Local `YYYY-MM-DD`.
    pub day: String,
    /// Local hour, `0..=23`.
    pub hour: u8,
    pub kind: DayKind,
    pub n: u32,
}

/// Persistence of the module: one configuration row, the user's and the learned sites, and
/// hourly counters. Synchronous, like every persistence port.
pub trait NofapRepo: Send + Sync {
    /// The stored configuration, or the default when the row does not exist yet.
    fn load_config(&self) -> CoreResult<NofapConfig>;
    fn save_config(&self, config: &NofapConfig) -> CoreResult<()>;
    fn list_sites(&self) -> CoreResult<Vec<NofapSite>>;
    /// Inserts the site, or replaces the one with the same `(list, host_key)`.
    fn upsert_site(&self, site: &NofapSite) -> CoreResult<()>;
    fn delete_site(&self, id: &str) -> CoreResult<()>;
    fn delete_sites_by_origin(&self, origin: SiteOrigin) -> CoreResult<u64>;
    /// Deletes snoozes whose `expires_at` is at or before `now`.
    fn purge_expired_sites(&self, now: DateTime<Utc>) -> CoreResult<u64>;
    /// Adds `by` to the counter of that local day and hour (an upsert).
    fn bump_day(&self, day: &str, hour: u8, kind: DayKind, by: u32) -> CoreResult<()>;
    /// Every counter with `from <= day <= to` (local `YYYY-MM-DD`), ordered by day and hour.
    fn days_between(&self, from: &str, to: &str) -> CoreResult<Vec<DayCount>>;
    fn delete_days_before(&self, day: &str) -> CoreResult<u64>;
    /// "Apagar meu histórico": every counter and every snooze. The configuration and the
    /// user's and learned sites stay.
    fn erase_history(&self) -> CoreResult<()>;
}

/// Asserts that `Type::ALL` is exactly the listed variants, in order. The list is also the
/// arms of a `match` without a wildcard, so a variant added to the enum does not compile
/// until it is listed here, and the assertion then fails until it is in `ALL` too. `ALL` is
/// what the fixtures, the lenient parsers and the frontend's union types are built from.
#[cfg(test)]
macro_rules! assert_all_listed {
    ($ty:ident: $($variant:ident),+ $(,)?) => {{
        #[allow(dead_code)]
        fn exhaustive(value: $ty) {
            match value {
                $($ty::$variant => {}),+
            }
        }
        assert_eq!(
            $ty::ALL.to_vec(),
            vec![$($ty::$variant),+],
            concat!(stringify!($ty), "::ALL does not list every variant in order")
        );
    }};
}
#[cfg(test)]
pub(crate) use assert_all_listed;

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use serde_json::{json, Value};

    fn at(h: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 9, 17, h, 0, 0).unwrap()
    }

    #[test]
    fn config_from_empty_object_is_the_default() {
        let config: NofapConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(config, NofapConfig::default());
        assert_eq!(config.v, 1);
        assert!(!config.enabled);
        assert_eq!(config.delay_secs, 600);
        assert!(config.system.hosts_user_sites);
        assert!(!config.system.hosts && !config.system.policies);
        assert_eq!(config.system.resolver, None);
        assert!(!config.vision.enabled);
        assert!(config.vision.unknown_sites && config.vision.media_apps);
        assert!(!config.vision.messaging_apps);
        assert!(config.vision.private_windows);
        assert_eq!(
            (config.vision.max_per_hour, config.vision.max_per_day),
            (20, 120)
        );
        assert_eq!(config.vision.monthly_cap_usd, 2.0);
        assert!(config.ui.discreet && !config.ui.show_streak && !config.ui.daily_checkin);
        assert!(config.internal.salt_hex.is_empty());
    }

    #[test]
    fn config_keeps_unknown_and_partial_documents_readable() {
        // A newer build wrote a field this one does not know, and left others out.
        let config: NofapConfig = serde_json::from_value(json!({
            "enabled": true,
            "system": { "hosts": true, "future_field": 1 },
            "vision": { "max_per_day": 300 },
            "from_the_future": { "a": [1, 2] }
        }))
        .unwrap();
        assert!(config.enabled);
        assert!(config.system.hosts);
        assert!(config.system.hosts_user_sites);
        assert_eq!(config.vision.max_per_day, 300);
        assert_eq!(config.vision.max_per_hour, 20);
    }

    #[test]
    fn config_round_trips_with_every_field_set() {
        let config = NofapConfig {
            v: 1,
            enabled: true,
            enabled_at: Some(at(9)),
            self_use_at: Some(at(8)),
            system: SystemPrefs {
                hosts: true,
                resolver: Some(ResolverId::CloudflareFamily),
                policies: true,
                hosts_user_sites: false,
                network_restored: true,
            },
            vision: VisionPrefs {
                enabled: true,
                consent: Some(VisionConsent {
                    provider: AiProvider::OpenAi,
                    managed: false,
                    at: at(9),
                }),
                messaging_apps: true,
                max_per_hour: 30,
                ..VisionPrefs::default()
            },
            delay_secs: 1_800,
            pending: vec![PendingChange {
                id: "p1".into(),
                change: NofapChange::AddAllow {
                    host: "example.com".into(),
                },
                requested_at: at(10),
                ready_at: at(11),
                observed_secs: 300,
                ready_since: None,
            }],
            ui: UiPrefs {
                discreet: false,
                show_streak: true,
                daily_checkin: true,
            },
            values: "Quero estar presente.".into(),
            plans: vec![IfThenPlan {
                when: "estiver cansado".into(),
                then: "sair para caminhar".into(),
            }],
            internal: Internal {
                salt_hex: "00112233445566778899aabbccddeeff".into(),
                exclusions_seen: vec![ExclusionSeen {
                    key: "app:com.example.bank".into(),
                    first_seen_at: at(9),
                }],
                last_help_at: Some(at(9)),
                last_reminder_at: None,
                last_drift_card_at: Some(at(10)),
                previous_dns: vec![PreviousDns {
                    interface: "Wi-Fi".into(),
                    servers: vec!["192.168.0.2".into()],
                }],
            },
        };
        let text = serde_json::to_string(&config).unwrap();
        assert_eq!(serde_json::from_str::<NofapConfig>(&text).unwrap(), config);
    }

    #[test]
    fn sanitize_clamps_every_range() {
        let mut config = NofapConfig {
            v: 9,
            delay_secs: 999_999,
            values: format!("  {}  ", "é".repeat(400)),
            plans: (0..5)
                .map(|i| IfThenPlan {
                    when: format!(" quando {i} {}", "x".repeat(300)),
                    then: " então ".into(),
                })
                .collect(),
            pending: (0..25)
                .map(|i| PendingChange {
                    id: format!("p{i}"),
                    change: NofapChange::Disable,
                    requested_at: at(10),
                    ready_at: at(11),
                    observed_secs: 0,
                    ready_since: None,
                })
                .collect(),
            ..NofapConfig::default()
        };
        config.vision.max_per_hour = 0;
        config.vision.max_per_day = 50_000;
        config.vision.monthly_cap_usd = f64::NAN;
        config.sanitize();
        assert_eq!(config.v, CONFIG_VERSION);
        assert_eq!(config.delay_secs, 86_400);
        assert_eq!(config.values.chars().count(), 280);
        assert!(config.values.starts_with('é'));
        assert_eq!(config.plans.len(), 3);
        assert_eq!(config.plans[0].when.chars().count(), 140);
        assert!(config.plans[0].when.starts_with("quando 0"));
        assert_eq!(config.plans[0].then, "então");
        assert_eq!(config.pending.len(), 20);
        assert_eq!(config.vision.max_per_hour, 1);
        assert_eq!(config.vision.max_per_day, 1_000);
        assert_eq!(config.vision.monthly_cap_usd, 2.0);

        config.vision.max_per_hour = 61;
        config.vision.monthly_cap_usd = 0.01;
        config.sanitize();
        assert_eq!(config.vision.max_per_hour, 60);
        assert_eq!(config.vision.monthly_cap_usd, 0.25);
        config.vision.monthly_cap_usd = 500.0;
        config.sanitize();
        assert_eq!(config.vision.monthly_cap_usd, 20.0);

        // In-range values are left alone, and a delay of zero is allowed.
        let mut ok = NofapConfig {
            delay_secs: 0,
            ..NofapConfig::default()
        };
        let before = ok.clone();
        ok.sanitize();
        assert_eq!(ok, before);
    }

    #[test]
    fn changes_serialise_flat_with_a_kind_tag() {
        let cases = [
            (NofapChange::Disable, json!({ "kind": "disable" })),
            (
                NofapChange::AddBlock {
                    host: "example.com".into(),
                },
                json!({ "kind": "add_block", "host": "example.com" }),
            ),
            (
                NofapChange::SetHosts { on: true },
                json!({ "kind": "set_hosts", "on": true }),
            ),
            (
                NofapChange::SetResolver { resolver: None },
                json!({ "kind": "set_resolver", "resolver": null }),
            ),
            (
                NofapChange::SetResolver {
                    resolver: Some(ResolverId::CloudflareFamily),
                },
                json!({ "kind": "set_resolver", "resolver": "cloudflare_family" }),
            ),
            (
                NofapChange::SetCaps {
                    max_per_hour: 10,
                    max_per_day: 60,
                    monthly_cap_usd: 1.5,
                },
                json!({ "kind": "set_caps", "max_per_hour": 10, "max_per_day": 60, "monthly_cap_usd": 1.5 }),
            ),
            (
                NofapChange::SetPrefs {
                    prefs: UiPrefs::default(),
                },
                json!({ "kind": "set_prefs", "prefs": { "discreet": true, "show_streak": false, "daily_checkin": false } }),
            ),
        ];
        for (change, expected) in cases {
            let value = serde_json::to_value(&change).unwrap();
            assert_eq!(value, expected);
            assert_eq!(
                serde_json::from_value::<NofapChange>(value).unwrap(),
                change
            );
            // The kind enum and the tag are the same string.
            assert_eq!(
                serde_json::to_value(change.kind()).unwrap(),
                expected["kind"]
            );
        }
    }

    #[test]
    fn enums_round_trip_through_their_string_forms() {
        for tier in Tier::ALL {
            assert_eq!(Tier::parse(tier.as_str()), Some(tier));
            assert_eq!(serde_json::to_value(tier).unwrap(), json!(tier.as_str()));
        }
        for os in Os::ALL {
            assert_eq!(serde_json::to_value(os).unwrap(), json!(os.as_str()));
        }
        for r in ResolverId::ALL {
            assert_eq!(ResolverId::parse(r.as_str()), Some(r));
            assert_eq!(serde_json::to_value(r).unwrap(), json!(r.as_str()));
        }
        for b in BrowserFamily::ALL {
            assert_eq!(serde_json::to_value(b).unwrap(), json!(b.as_str()));
        }
        for o in SiteOrigin::ALL {
            assert_eq!(SiteOrigin::parse(o.as_str()), Some(o));
        }
        for l in [SiteList::Block, SiteList::Allow] {
            assert_eq!(SiteList::parse(l.as_str()), Some(l));
        }
        for s in [RemoveScope::All, RemoveScope::Resolver] {
            assert_eq!(RemoveScope::parse(s.as_str()), Some(s));
        }
        let mut seen = std::collections::HashSet::new();
        for kind in DayKind::ALL {
            assert_eq!(DayKind::parse(kind.as_str()), Some(kind));
            assert_eq!(serde_json::to_value(kind).unwrap(), json!(kind.as_str()));
            assert!(seen.insert(kind.as_str()), "{} twice", kind.as_str());
        }
        assert_eq!(DayKind::parse("from_a_newer_build"), None);
        assert_eq!(Tier::parse("Hosts"), None);
        assert_eq!(ResolverId::parse("1.1.1.3"), None);
        assert_eq!(RemoveScope::parse(""), None);
    }

    #[test]
    fn tagged_states_have_the_documented_wire_form() {
        assert_eq!(
            serde_json::to_value(TierState::Applied).unwrap(),
            json!({ "state": "applied" })
        );
        assert_eq!(
            serde_json::to_value(TierState::Unsupported {
                reason: "write_refused".into()
            })
            .unwrap(),
            json!({ "state": "unsupported", "reason": "write_refused" })
        );
        assert_eq!(
            serde_json::to_value(GuardState::Limited {
                reason: GuardLimit::TitleOnly
            })
            .unwrap(),
            json!({ "state": "limited", "reason": "title_only" })
        );
        assert_eq!(
            serde_json::to_value(VisionState::Off {
                reason: VisionOff::BudgetSpent
            })
            .unwrap(),
            json!({ "state": "off", "reason": "budget_spent" })
        );
        assert_eq!(
            serde_json::to_value(ApplyOutcome::Applied).unwrap(),
            json!({ "outcome": "applied" })
        );
        assert_eq!(
            serde_json::to_value(ApplyOutcome::Partial(vec![TierReport {
                tier: Tier::Resolver,
                ok: false,
                reason: Some("unreachable".into()),
            }]))
            .unwrap(),
            json!({ "outcome": "partial", "detail": [{ "tier": "resolver", "ok": false, "reason": "unreachable" }] })
        );
        assert_eq!(
            serde_json::to_value(ApplyOutcome::Unavailable("appimage".into())).unwrap(),
            json!({ "outcome": "unavailable", "detail": "appimage" })
        );
        for outcome in [
            ApplyOutcome::Applied,
            ApplyOutcome::Declined,
            ApplyOutcome::Unavailable("no_polkit_agent".into()),
        ] {
            let text = serde_json::to_string(&outcome).unwrap();
            assert_eq!(
                serde_json::from_str::<ApplyOutcome>(&text).unwrap(),
                outcome
            );
        }
    }

    #[test]
    fn system_state_file_parses_the_documented_shape() {
        let state: SystemState = serde_json::from_value(json!({
            "v": 1, "applied_at": 1795100000000i64,
            "owners": {
                "ubiqx":  { "hosts": true, "resolver": "cloudflare_family", "policies": true, "extra": ["example.com"] },
                "shield": { "hosts": true, "resolver": null, "policies": false, "extra": [] }
            },
            "hosts": { "sha256": "ab", "names": 8003 },
            "policies_created": [ { "browser": "chrome", "key": "DnsOverHttpsMode" } ]
        }))
        .unwrap();
        assert_eq!(state.owners.len(), 2);
        let ubiqx = &state.owners[&ProductId::Ubiqx];
        assert_eq!(ubiqx.resolver, Some(ResolverId::CloudflareFamily));
        assert_eq!(ubiqx.extra, vec!["example.com".to_string()]);
        assert_eq!(state.owners[&ProductId::Shield].resolver, None);
        assert_eq!(state.hosts.as_ref().unwrap().names, 8003);
        assert_eq!(state.policies_created[0].browser, BrowserFamily::Chrome);
        assert!(state.report.is_empty());
        let text = serde_json::to_string(&state).unwrap();
        assert_eq!(serde_json::from_str::<SystemState>(&text).unwrap(), state);

        // An owner outside the closed set is not a state file this build accepts.
        assert!(serde_json::from_value::<SystemState>(
            json!({ "owners": { "someone": { "hosts": true } } })
        )
        .is_err());
        // An empty observation is what a machine without the module looks like.
        let observation: SystemObservation = serde_json::from_str("{}").unwrap();
        assert_eq!(observation, SystemObservation::default());
    }

    /// Keys that would name what the user was looking at.
    const FORBIDDEN_KEYS: [&str; 7] = [
        "host", "domain", "url", "title", "app", "app_name", "app_id",
    ];

    fn assert_no_forbidden_key(value: &Value, path: &str) {
        match value {
            Value::Object(map) => {
                for (key, inner) in map {
                    assert!(
                        !FORBIDDEN_KEYS.contains(&key.as_str()),
                        "{path}.{key} names what was on screen"
                    );
                    assert_no_forbidden_key(inner, &format!("{path}.{key}"));
                }
            }
            Value::Array(items) => {
                for (i, inner) in items.iter().enumerate() {
                    assert_no_forbidden_key(inner, &format!("{path}[{i}]"));
                }
            }
            _ => {}
        }
    }

    fn full_status() -> NofapStatus {
        NofapStatus {
            overall: Overall::Reduced,
            guard: GuardState::Limited {
                reason: GuardLimit::BrowserWithoutUrl,
            },
            hosts: TierState::UpdatePending,
            resolver: TierState::Unsupported {
                reason: "no_resolved".into(),
            },
            policies: TierState::Gated,
            vision: VisionState::Degraded { until: at(16) },
            issues: Issue::ALL.to_vec(),
            // Every kind of change, including the ones whose payload is a host.
            pending: ChangeKind::ALL
                .into_iter()
                .enumerate()
                .map(|(i, kind)| PendingView {
                    id: format!("p{i}"),
                    kind,
                    requested_at: at(10),
                    ready_at: at(11),
                    ready: i % 2 == 0,
                    remaining_secs: 60,
                    expires_at: Some(at(12)),
                })
                .collect(),
            hosts_waiting: 1,
            extras_left_out: 2,
        }
    }

    fn full_card(kind: CardKind) -> NofapCard {
        NofapCard {
            id: "c1".into(),
            kind,
            message_key: "block.discreet".into(),
            note_key: Some("vision.added".into()),
            discreet: true,
            actions: CardAction::ALL.to_vec(),
            auto_close_secs: Some(12),
            expires_at: at(16),
        }
    }

    #[test]
    fn nothing_shown_or_emitted_has_a_field_for_what_was_on_screen() {
        assert_no_forbidden_key(&serde_json::to_value(full_status()).unwrap(), "status");
        assert_no_forbidden_key(
            &serde_json::to_value(NofapStatus::off()).unwrap(),
            "status_off",
        );
        for kind in CardKind::ALL {
            assert_no_forbidden_key(&serde_json::to_value(full_card(kind)).unwrap(), "card");
        }
        let events = [
            NofapEvent::Status {
                status: full_status(),
            },
            NofapEvent::Card {
                card: full_card(CardKind::VisionClosed),
            },
            NofapEvent::CardClosed { id: "c1".into() },
            NofapEvent::PendingChanged,
            NofapEvent::SitesChanged,
        ];
        for event in events {
            let value = serde_json::to_value(&event).unwrap();
            assert_no_forbidden_key(&value, "event");
            assert!(value["type"].is_string());
            assert_eq!(serde_json::from_value::<NofapEvent>(value).unwrap(), event);
        }
    }

    #[test]
    fn the_forbidden_key_check_can_fail() {
        // The guard above is only worth something if it sees a nested offender.
        let offending =
            json!({ "status": { "pending": [{ "change": { "host": "example.com" } }] } });
        let caught = std::panic::catch_unwind(|| assert_no_forbidden_key(&offending, "x"));
        assert!(caught.is_err());
    }

    #[test]
    fn event_wire_form_is_tagged_by_type() {
        assert_eq!(
            serde_json::to_value(NofapEvent::PendingChanged).unwrap(),
            json!({ "type": "pending_changed" })
        );
        assert_eq!(
            serde_json::to_value(NofapEvent::CardClosed { id: "c1".into() }).unwrap(),
            json!({ "type": "card_closed", "id": "c1" })
        );
        let status = serde_json::to_value(NofapEvent::Status {
            status: NofapStatus::off(),
        })
        .unwrap();
        assert_eq!(status["type"], "status");
        assert_eq!(status["status"]["overall"], "off");
        assert_eq!(
            status["status"]["vision"],
            json!({ "state": "off", "reason": "disabled" })
        );
    }

    #[test]
    fn every_all_constant_lists_every_variant() {
        assert_all_listed!(Os: Macos, Windows, Linux);
        assert_all_listed!(Tier: Hosts, Resolver, Policies);
        assert_all_listed!(ResolverId: CloudflareFamily);
        assert_all_listed!(BrowserFamily: Chrome, Chromium, Edge, Brave, Firefox);
        assert_all_listed!(
            ChangeKind: Disable,
            AddBlock,
            RemoveBlock,
            AddAllow,
            RemoveAllow,
            ForgetLearned,
            SetHosts,
            SetResolver,
            SetPolicies,
            SetHostsUserSites,
            SetVision,
            SetVisionScope,
            SetCaps,
            SetDelay,
            SetPrefs,
            SetValues,
            SetPlans,
        );
        assert_all_listed!(Overall: Off, Protected, Reduced, PendingOff, Secondary);
        assert_all_listed!(GuardLimit: AutomationDenied, BrowserWithoutUrl, TitleOnly, Wayland);
        assert_all_listed!(
            VisionOff: Disabled,
            ConsentOutdated,
            NoProvider,
            NoKey,
            LocalOnly,
            NotOnboarded,
            Licence,
            ScreenRecording,
            Wayland,
            BudgetSpent,
            CapReached,
        );
        assert_all_listed!(
            Issue: HostsNotHonoured,
            DnsBypassSuspected,
            ResolverUnreachable,
            NewConnectionUnfiltered,
            PolicyConflict,
            PolicyAdvisoryOnly,
            AutomationDenied,
            ScreenRecordingDenied,
            VisionBudgetSpent,
            VisionCapReached,
            VisionNoProvider,
            ConsentOutdated,
            OtherProductPrimary,
        );
        assert_all_listed!(
            CardKind: Blocked,
            BlockedRepeat,
            VisionClosed,
            VisionAsk,
            Drift,
            PendingReady,
            ResolverUnreachable,
            Reminder,
            Help,
        );
        assert_all_listed!(
            CardAction: Ok,
            Support,
            Surf,
            CloseWindow,
            NotIt,
            RequestException,
            AddToList,
            Restore,
            KeepProtection,
            TurnOff,
            RestoreNetwork,
            Later,
            Help,
        );
        assert_all_listed!(
            AiReadiness: Ready,
            NoKey,
            LocalOnly,
            NotOnboarded,
            LicenceRequired,
        );
        assert_all_listed!(SiteOrigin: User, Learned, Snooze);
        assert_all_listed!(
            DayKind: BlockList,
            BlockUser,
            BlockKeyword,
            VisionCheck,
            VisionHit,
            NotIt,
            SurfStarted,
            SurfDone,
            Panic,
            Lapse,
            CheckinOk,
            CheckinMeh,
            CheckinHard,
            CheckinVeryBad,
            TriggerTired,
            TriggerBored,
            TriggerStressed,
            TriggerLonely,
            TriggerHungry,
            TriggerAngry,
            TriggerOther,
            OverrideRequest,
            OverrideCancel,
            QuitOn,
            NetworkRestore,
            Drift,
            HelpShown,
        );
        assert_all_listed!(ProductId: Ubiqx, Shield);
    }

    #[test]
    fn names_follow_the_three_rings() {
        assert_eq!(SYSTEM_ID, "ubiqx-shield");
        assert!(!SYSTEM_ID.to_ascii_lowercase().contains("nofap"));
        assert!(!DISPLAY_NAME.trim().is_empty());
        assert_eq!(crate::product::SHIELD.display_name, DISPLAY_NAME);
    }

    #[test]
    fn the_default_observation_is_the_cheapest_one() {
        // A caller that forgets to choose must not start PowerShell or send a lookup.
        assert_eq!(ObserveOpts::default(), ObserveOpts::QUICK);
        let quick = ObserveOpts::QUICK;
        assert!(!quick.full && !quick.canary && !quick.probe);
        let full = ObserveOpts::FULL;
        assert!(full.full && !full.canary && !full.probe);
    }

    #[test]
    fn capabilities_none_marks_every_tier_unsupported() {
        let caps = SystemCapabilities::none("not_built");
        for tier in Tier::ALL {
            assert_eq!(
                caps.of(tier),
                &TierCapability::Unsupported {
                    reason: "not_built".into()
                }
            );
        }
        assert!(caps.unverified.is_empty());
    }
}
