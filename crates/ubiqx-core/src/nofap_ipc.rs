//! The wire contract of the `nofap` module: the one command both shells expose
//! (`nofap(command: NofapCommand)`), and every type that crosses the boundary to the
//! frontend.
//!
//! The contract is fixed here, in Rust. [`sample`] builds one representative value of every
//! request and of every reply, and the test `nofap_ipc_golden` compares their serialised
//! form with two committed files that the frontend type-checks against and the engine's
//! handler test replays:
//!
//! * `apps/desktop/src/nofap/fixtures/commands.json`: one request per `cmd` (and one per
//!   kind of change and of user event);
//! * `apps/desktop/src/nofap/fixtures/views.json`: one value per return type, one event per
//!   [`NofapEvent`] variant, one card per kind, statuses covering every tier state and
//!   issue, every line a preview can hold, and under `enums` every value of every closed
//!   set on the wire, so the frontend can check its union types in both directions;
//! * `apps/desktop/src/nofap/fixtures/golden.ts`: the same two values as TypeScript
//!   literals (`as const`). A JSON import widens every string to `string`, so it cannot be
//!   checked against a union of literals; this module can
//!   (`commands satisfies DeepReadonly<NofapCommand[]>`).
//!
//! Regenerate them with `UBIQX_BLESS=1 cargo test -p ubiqx-core nofap_ipc_golden`. Nobody
//! writes a fixture by hand.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::model::{AiProvider, Id};
use crate::nofap::{
    AiReadiness, ApplyOutcome, CardKind, ChangeKind, GuardLimit, IfThenPlan, Issue, NofapCard,
    NofapChange, NofapConfig, NofapEvent, NofapStatus, Os, Overall, PendingChange, ResolverId,
    SystemCapabilities, SystemPrefs, Tier, UiPrefs, VisionOff, VisionPrefs,
};
use crate::product::ProductId;

pub use crate::nofap::{CardAction, PendingView, PlannedChange};

/// Name of the Tauri command and of the Tauri event both shells use.
pub const COMMAND: &str = "nofap";
pub const EVENT: &str = "nofap";

/// A request from the frontend. Wire form: `{ "cmd": "<snake_case>", "args": … }`; `args`
/// is absent for commands without arguments. The reply type of each command is in its
/// documentation; the handler (`ubiqx_engine::nofap::ipc::handle`) returns it as JSON.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "cmd", content = "args", rename_all = "snake_case")]
pub enum NofapCommand {
    /// → [`NofapView`].
    View,
    /// The wizard's last button; may raise the OS prompt. → [`NofapView`].
    Enable(EnableOptions),
    /// Records consent for the AI check with the provider now selected. → [`NofapView`].
    ConsentVision(ConsentArgs),
    /// → [`ChangeOutcome`].
    Request(NofapChange),
    /// → [`NofapView`].
    ConfirmPending { id: Id },
    /// → [`NofapView`].
    CancelPending { id: Id },
    /// → `Vec<`[`PlannedChange`]`>`.
    SystemPreview,
    /// "Aplicar", "Restaurar", "Religar"; OS prompt. → [`ApplyOutcome`].
    SystemApply,
    /// Only while disabled; OS prompt. → [`ApplyOutcome`].
    SystemRemove,
    /// "Restaurar minha rede": never delayed; OS prompt. → [`ApplyOutcome`].
    RestoreNetwork,
    /// → [`SitesView`].
    Sites,
    /// → [`SupportView`].
    Support,
    /// Counters only. → `null`.
    Record(UserEvent),
    /// The floating window asks for its card; an unknown id gives the neutral discreet
    /// card. → [`NofapCard`].
    Card { id: Id },
    /// → `null`.
    CardAction { id: Id, action: CardAction },
    /// "Ver um exemplo do que é enviado": the last candidate window, never sent, never
    /// stored. → [`SampleFrame`], or `null` when no candidate window is remembered.
    SampleFrame,
    /// "Apagar meu histórico": never delayed. → `null`.
    EraseHistory,
    /// "Apagar tudo": only while disabled. → `null`.
    Reset,
    /// → [`ExportResult`].
    Export,
    /// A preview card that counts nothing. → `null`.
    TestCard,
}

impl NofapCommand {
    /// Every `cmd` string, in declaration order.
    pub const NAMES: [&'static str; 20] = [
        "view",
        "enable",
        "consent_vision",
        "request",
        "confirm_pending",
        "cancel_pending",
        "system_preview",
        "system_apply",
        "system_remove",
        "restore_network",
        "sites",
        "support",
        "record",
        "card",
        "card_action",
        "sample_frame",
        "erase_history",
        "reset",
        "export",
        "test_card",
    ];

    /// The `cmd` string of this request.
    pub fn name(&self) -> &'static str {
        match self {
            NofapCommand::View => "view",
            NofapCommand::Enable(_) => "enable",
            NofapCommand::ConsentVision(_) => "consent_vision",
            NofapCommand::Request(_) => "request",
            NofapCommand::ConfirmPending { .. } => "confirm_pending",
            NofapCommand::CancelPending { .. } => "cancel_pending",
            NofapCommand::SystemPreview => "system_preview",
            NofapCommand::SystemApply => "system_apply",
            NofapCommand::SystemRemove => "system_remove",
            NofapCommand::RestoreNetwork => "restore_network",
            NofapCommand::Sites => "sites",
            NofapCommand::Support => "support",
            NofapCommand::Record(_) => "record",
            NofapCommand::Card { .. } => "card",
            NofapCommand::CardAction { .. } => "card_action",
            NofapCommand::SampleFrame => "sample_frame",
            NofapCommand::EraseHistory => "erase_history",
            NofapCommand::Reset => "reset",
            NofapCommand::Export => "export",
            NofapCommand::TestCard => "test_card",
        }
    }
}

/// What the wizard collected. Nothing is applied before this request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnableOptions {
    /// The "é para mim, no meu computador" tick; the request is refused without it.
    pub self_use: bool,
    pub hosts: bool,
    pub resolver: Option<ResolverId>,
    pub policies: bool,
    pub vision: bool,
    /// Consent for the AI check, given in the wizard; required when `vision` is true.
    pub vision_consent: Option<ConsentArgs>,
    pub vision_scope: VisionScope,
    pub delay_secs: u32,
    pub values: String,
    pub discreet: bool,
}

/// Which windows the AI check may look at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisionScope {
    pub unknown_sites: bool,
    pub media_apps: bool,
    pub messaging_apps: bool,
    pub private_windows: bool,
}

impl From<&VisionPrefs> for VisionScope {
    fn from(prefs: &VisionPrefs) -> Self {
        Self {
            unknown_sites: prefs.unknown_sites,
            media_apps: prefs.media_apps,
            messaging_apps: prefs.messaging_apps,
            private_windows: prefs.private_windows,
        }
    }
}

/// The provider the user read about and agreed to; `managed` is the second tick of "IA do
/// Ubi" (images pass through the operator's proxy).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsentArgs {
    pub provider: AiProvider,
    pub managed: bool,
}

/// Reply to `request`: the change took effect, or it waits out the delay. Wire form:
/// `{ "applied": NofapView }` or `{ "pending": PendingView }`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeOutcome {
    Applied(Box<NofapView>),
    Pending(PendingView),
}

/// Everything the module's page shows.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NofapView {
    pub status: NofapStatus,
    pub config: ConfigView,
    pub capabilities: SystemCapabilities,
    pub today: TodayCounts,
    /// `ubiqx_core::nofap::DISPLAY_NAME`, for the `{name}` placeholder.
    pub display_name: String,
    /// Which product this process is.
    pub product: ProductId,
    /// The AI provider now selected, for the `{provider}` placeholder.
    pub provider: AiProvider,
    /// Whether that provider is usable (a key, a licence, not local-only).
    pub ai: AiReadiness,
    pub cost_line: CostLine,
}

/// [`NofapConfig`] without its `internal` bookkeeping (the salt never leaves the engine).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfigView {
    pub v: u8,
    pub enabled: bool,
    pub enabled_at: Option<DateTime<Utc>>,
    pub self_use_at: Option<DateTime<Utc>>,
    pub system: SystemPrefs,
    pub vision: VisionPrefs,
    pub delay_secs: u32,
    pub pending: Vec<PendingChange>,
    pub ui: UiPrefs,
    pub values: String,
    pub plans: Vec<IfThenPlan>,
}

impl From<&NofapConfig> for ConfigView {
    fn from(config: &NofapConfig) -> Self {
        Self {
            v: config.v,
            enabled: config.enabled,
            enabled_at: config.enabled_at,
            self_use_at: config.self_use_at,
            system: config.system.clone(),
            vision: config.vision.clone(),
            delay_secs: config.delay_secs,
            pending: config.pending.clone(),
            ui: config.ui.clone(),
            values: config.values.clone(),
            plans: config.plans.clone(),
        }
    }
}

/// Today's counters (local day): pages held and AI checks made. Counts, never names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct TodayCounts {
    pub blocks: u32,
    pub checks: u32,
}

/// The module's own AI spend this month, from the usage ledger's `nofap_vision` rows.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct CostLine {
    pub month_usd: f64,
    pub month_checks: u64,
    pub cap_usd: f64,
}

/// The lists the user can read and edit. Learned sites are a count, never a name.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SitesView {
    pub blocked: Vec<SiteEntry>,
    pub allowed: Vec<SiteEntry>,
    pub learned_count: u32,
}

/// A site the user typed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SiteEntry {
    pub id: Id,
    pub host: String,
    pub created_at: DateTime<Utc>,
}

/// The "Apoio" tab.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupportView {
    pub values: String,
    pub plans: Vec<IfThenPlan>,
    pub progress: ProgressView,
    /// Today's check-in answer, when one was given.
    pub checkin_today: Option<Mood>,
    /// The professional-help card is due (a product heuristic, not a clinical cut-off).
    pub help_due: bool,
    pub resources: Vec<HelpResource>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgressView {
    /// Size of the window in days (30).
    pub window_days: u32,
    /// Days of the window that went the way the user wanted.
    pub days: u32,
    /// Current streak in days; `None` while the streak is switched off.
    pub streak: Option<u32>,
    /// A lapse was recorded inside the window (`streak.after_reset`).
    pub after_reset: bool,
}

/// One help resource from `resources.json`. Phone numbers and addresses come from here,
/// never from a message string.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HelpResource {
    pub id: String,
    /// ISO 3166-1 alpha-2, or empty for a resource that covers several countries.
    pub country: String,
    pub name: String,
    pub phone: Option<String>,
    pub url: Option<String>,
    pub note_pt: String,
    pub note_en: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mood {
    Ok,
    Meh,
    Hard,
    VeryBad,
}

impl Mood {
    pub const ALL: [Mood; 4] = [Mood::Ok, Mood::Meh, Mood::Hard, Mood::VeryBad];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Trigger {
    Tired,
    Bored,
    Stressed,
    Lonely,
    Hungry,
    Angry,
    Other,
}

impl Trigger {
    pub const ALL: [Trigger; 7] = [
        Trigger::Tired,
        Trigger::Bored,
        Trigger::Stressed,
        Trigger::Lonely,
        Trigger::Hungry,
        Trigger::Angry,
        Trigger::Other,
    ];
}

/// Something the user did in a flow. Counted per day and hour, never stored as an event.
/// Wire form: `{ "event": "lapse", "triggers": ["tired"] }`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum UserEvent {
    /// "Aconteceu um deslize": only the user starts it.
    Lapse {
        triggers: Vec<Trigger>,
    },
    Checkin {
        mood: Mood,
        triggers: Vec<Trigger>,
    },
    SurfStarted,
    SurfDone,
    Panic,
    /// "Sair da tela por 10 min": hides the other windows, locks nothing.
    StepAway,
}

/// The reduced image "Ver um exemplo do que é enviado" shows in the page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SampleFrame {
    pub mime: String,
    pub base64: String,
    pub width: u32,
    pub height: u32,
}

/// Where "Exportar dados" wrote its file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportResult {
    pub path: String,
}

/// Shape of `fixtures/views.json`: one value per reply type, per event variant and per
/// card kind, and statuses that cover every tier state and issue.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GoldenViews {
    pub view: NofapView,
    /// `applied`, then `pending`.
    pub change_outcomes: Vec<ChangeOutcome>,
    pub planned_changes: Vec<PlannedChange>,
    /// `applied`, `partial`, `declined`, `unavailable`.
    pub apply_outcomes: Vec<ApplyOutcome>,
    pub sites: SitesView,
    pub support: SupportView,
    /// One per card kind.
    pub cards: Vec<NofapCard>,
    /// A frame, then the `null` of "no candidate window".
    pub sample_frames: Vec<Option<SampleFrame>>,
    pub export: ExportResult,
    /// One per variant.
    pub events: Vec<NofapEvent>,
    pub statuses: Vec<NofapStatus>,
    /// Every line `system_preview` can return (`ubiqx_core::nofap::preview::LINES`): the
    /// frontend needs a sentence for each key and fills its one placeholder with `detail`.
    pub preview_lines: Vec<PreviewLine>,
    pub enums: GoldenEnums,
}

/// One entry of `ubiqx_core::nofap::preview::LINES`, as the fixture carries it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreviewLine {
    pub key: String,
    pub tier: Tier,
    /// Name of the placeholder `detail` fills; `None` for a line without one.
    pub placeholder: Option<String>,
}

/// Every value of every closed set that crosses the boundary as a bare string, each list
/// being the type's `ALL`. The other fixtures only show the values their samples happen to
/// use; with these lists the frontend checks its unions both ways: a value Rust sends that
/// the union lacks fails `satisfies`, and a member of the union that Rust never sends fails
/// an `Exclude<Union, (typeof list)[number]>` that must be `never`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoldenEnums {
    pub overall: Vec<Overall>,
    pub guard_limit: Vec<GuardLimit>,
    pub vision_off: Vec<VisionOff>,
    pub issue: Vec<Issue>,
    pub change_kind: Vec<ChangeKind>,
    pub card_kind: Vec<CardKind>,
    pub card_action: Vec<CardAction>,
    pub mood: Vec<Mood>,
    pub trigger: Vec<Trigger>,
    pub ai_readiness: Vec<AiReadiness>,
    pub ai_provider: Vec<AiProvider>,
    pub product: Vec<ProductId>,
    pub os: Vec<Os>,
    pub tier: Vec<Tier>,
    pub resolver: Vec<ResolverId>,
    /// `ubiqx_core::nofap::reason::ALL`. Unlike the other lists this one is not the whole
    /// type (a reason travels as a string): a code outside it gets the neutral sentence.
    pub reason: Vec<String>,
}

/// Representative values of every wire type. They are the source of the two fixture files;
/// tests in other crates build on them too. All data is fictional.
pub mod sample {
    use chrono::TimeZone;

    use super::*;
    use crate::nofap::{
        preview, reason, GuardState, TierCapability, TierReport, TierState, VisionConsent,
        VisionState, DISPLAY_NAME,
    };

    /// 2026-09-17 15:00 UTC plus `mins` minutes (the repository's fixed test clock).
    pub fn at(mins: i64) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 9, 17, 15, 0, 0).unwrap() + chrono::Duration::minutes(mins)
    }

    pub fn enable_options() -> EnableOptions {
        EnableOptions {
            self_use: true,
            hosts: true,
            resolver: Some(ResolverId::CloudflareFamily),
            policies: false,
            vision: true,
            vision_consent: Some(ConsentArgs {
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

    /// One change of every kind, in the order of [`ChangeKind::ALL`].
    pub fn changes() -> Vec<NofapChange> {
        vec![
            NofapChange::Disable,
            NofapChange::AddBlock {
                host: "example.com".into(),
            },
            NofapChange::RemoveBlock {
                id: "site-1".into(),
            },
            NofapChange::AddAllow {
                host: "example.org".into(),
            },
            NofapChange::RemoveAllow {
                id: "site-2".into(),
            },
            NofapChange::ForgetLearned,
            NofapChange::SetHosts { on: true },
            NofapChange::SetResolver {
                resolver: Some(ResolverId::CloudflareFamily),
            },
            NofapChange::SetPolicies { on: false },
            NofapChange::SetHostsUserSites { on: false },
            NofapChange::SetVision { on: true },
            NofapChange::SetVisionScope {
                unknown_sites: true,
                media_apps: true,
                messaging_apps: false,
                private_windows: true,
            },
            NofapChange::SetCaps {
                max_per_hour: 20,
                max_per_day: 120,
                monthly_cap_usd: 2.0,
            },
            NofapChange::SetDelay { secs: 1_800 },
            NofapChange::SetPrefs {
                prefs: UiPrefs {
                    discreet: true,
                    show_streak: true,
                    daily_checkin: false,
                },
            },
            NofapChange::SetValues {
                text: "Quero estar presente com quem eu amo.".into(),
            },
            NofapChange::SetPlans {
                plans: vec![plan()],
            },
        ]
    }

    pub fn user_events() -> Vec<UserEvent> {
        vec![
            UserEvent::Lapse {
                triggers: vec![Trigger::Tired, Trigger::Lonely],
            },
            UserEvent::Checkin {
                mood: Mood::Hard,
                triggers: vec![Trigger::Stressed],
            },
            UserEvent::SurfStarted,
            UserEvent::SurfDone,
            UserEvent::Panic,
            UserEvent::StepAway,
        ]
    }

    /// Every request of `commands.json`: each `cmd` at least once, `request` once per kind
    /// of change and `record` once per user event.
    pub fn commands() -> Vec<NofapCommand> {
        let mut out = vec![
            NofapCommand::View,
            NofapCommand::Enable(enable_options()),
            NofapCommand::ConsentVision(ConsentArgs {
                provider: AiProvider::Ubi,
                managed: true,
            }),
        ];
        out.extend(changes().into_iter().map(NofapCommand::Request));
        out.extend([
            NofapCommand::ConfirmPending {
                id: "pending-1".into(),
            },
            NofapCommand::CancelPending {
                id: "pending-1".into(),
            },
            NofapCommand::SystemPreview,
            NofapCommand::SystemApply,
            NofapCommand::SystemRemove,
            NofapCommand::RestoreNetwork,
            NofapCommand::Sites,
            NofapCommand::Support,
        ]);
        out.extend(user_events().into_iter().map(NofapCommand::Record));
        out.extend([
            NofapCommand::Card {
                id: "card-1".into(),
            },
            NofapCommand::CardAction {
                id: "card-1".into(),
                action: CardAction::NotIt,
            },
            NofapCommand::SampleFrame,
            NofapCommand::EraseHistory,
            NofapCommand::Reset,
            NofapCommand::Export,
            NofapCommand::TestCard,
        ]);
        out
    }

    pub fn plan() -> IfThenPlan {
        IfThenPlan {
            when: "estiver sem sono de madrugada".into(),
            then: "levantar e beber água".into(),
        }
    }

    pub fn pending_change() -> PendingChange {
        PendingChange {
            id: "pending-1".into(),
            change: NofapChange::AddAllow {
                host: "example.org".into(),
            },
            requested_at: at(0),
            ready_at: at(10),
            observed_secs: 120,
            ready_since: None,
        }
    }

    pub fn pending_view() -> PendingView {
        PendingView {
            id: "pending-1".into(),
            kind: ChangeKind::AddAllow,
            requested_at: at(0),
            ready_at: at(10),
            ready: false,
            remaining_secs: 480,
            expires_at: None,
        }
    }

    pub fn config() -> NofapConfig {
        let mut config = NofapConfig {
            enabled: true,
            enabled_at: Some(at(-60)),
            self_use_at: Some(at(-61)),
            pending: vec![pending_change()],
            values: "Quero estar presente com quem eu amo.".into(),
            plans: vec![plan()],
            ..NofapConfig::default()
        };
        config.system.hosts = true;
        config.system.resolver = Some(ResolverId::CloudflareFamily);
        config.vision.enabled = true;
        config.vision.consent = Some(VisionConsent {
            provider: AiProvider::Anthropic,
            managed: false,
            at: at(-60),
        });
        config.internal.salt_hex = "000102030405060708090a0b0c0d0e0f".into();
        config
    }

    pub fn status() -> NofapStatus {
        NofapStatus {
            overall: Overall::Protected,
            guard: GuardState::On,
            hosts: TierState::Applied,
            resolver: TierState::Applied,
            policies: TierState::Gated,
            vision: VisionState::On,
            issues: vec![],
            pending: vec![pending_view()],
            hosts_waiting: 0,
            extras_left_out: 0,
        }
    }

    pub fn capabilities() -> SystemCapabilities {
        SystemCapabilities {
            os: Some(Os::Linux),
            hosts: TierCapability::Available,
            resolver: TierCapability::Unsupported {
                reason: "no_resolved".into(),
            },
            policies: TierCapability::Gated,
            unverified: vec![Tier::Hosts],
        }
    }

    pub fn view() -> NofapView {
        NofapView {
            status: status(),
            config: ConfigView::from(&config()),
            capabilities: capabilities(),
            today: TodayCounts {
                blocks: 2,
                checks: 14,
            },
            display_name: DISPLAY_NAME.to_string(),
            product: ProductId::Ubiqx,
            provider: AiProvider::Anthropic,
            ai: AiReadiness::Ready,
            cost_line: CostLine {
                month_usd: 0.21,
                month_checks: 300,
                cap_usd: 2.0,
            },
        }
    }

    /// A first apply of all three tiers on a machine with a DNS set by hand.
    pub fn planned_changes() -> Vec<PlannedChange> {
        let line = |tier, key: &str, detail: Option<&str>| PlannedChange {
            tier,
            text_key: key.into(),
            detail: detail.map(Into::into),
        };
        vec![
            line(Tier::Hosts, preview::HOSTS_ADD, Some("8003")),
            line(Tier::Hosts, preview::HOSTS_READABLE, None),
            line(
                Tier::Resolver,
                preview::RESOLVER_SET,
                Some("Cloudflare for Families"),
            ),
            line(Tier::Resolver, preview::DNS_PREVIOUS, Some("192.168.0.2")),
            line(
                Tier::Policies,
                preview::POLICIES_SET,
                Some("Chrome, Firefox"),
            ),
            line(Tier::Policies, preview::POLICIES_DISCLOSURE, None),
        ]
    }

    pub fn preview_lines() -> Vec<PreviewLine> {
        preview::LINES
            .into_iter()
            .map(|(key, tier, placeholder)| PreviewLine {
                key: key.into(),
                tier,
                placeholder: placeholder.map(Into::into),
            })
            .collect()
    }

    pub fn apply_outcomes() -> Vec<ApplyOutcome> {
        vec![
            ApplyOutcome::Applied,
            ApplyOutcome::Partial(vec![
                TierReport {
                    tier: Tier::Hosts,
                    ok: true,
                    reason: None,
                },
                TierReport {
                    tier: Tier::Resolver,
                    ok: false,
                    reason: Some("unreachable".into()),
                },
            ]),
            ApplyOutcome::Declined,
            ApplyOutcome::Unavailable("no_polkit_agent".into()),
        ]
    }

    pub fn sites() -> SitesView {
        SitesView {
            blocked: vec![SiteEntry {
                id: "site-1".into(),
                host: "example.com".into(),
                created_at: at(-120),
            }],
            allowed: vec![SiteEntry {
                id: "site-2".into(),
                host: "example.org".into(),
                created_at: at(-30),
            }],
            learned_count: 3,
        }
    }

    pub fn support() -> SupportView {
        SupportView {
            values: "Quero estar presente com quem eu amo.".into(),
            plans: vec![plan()],
            progress: ProgressView {
                window_days: 30,
                days: 26,
                streak: Some(5),
                after_reset: true,
            },
            checkin_today: Some(Mood::Meh),
            help_due: false,
            resources: vec![
                HelpResource {
                    id: "cvv".into(),
                    country: "BR".into(),
                    name: "CVV".into(),
                    phone: Some("188".into()),
                    url: Some("https://cvv.org.br/chat".into()),
                    note_pt: "Grátis, 24 horas.".into(),
                    note_en: "Free, 24 hours.".into(),
                },
                HelpResource {
                    id: "crp".into(),
                    country: "BR".into(),
                    name: "CRP".into(),
                    phone: None,
                    url: None,
                    note_pt: "Procure um psicólogo com registro ativo no CRP.".into(),
                    note_en: "Look for a psychologist with an active CRP registration.".into(),
                },
            ],
        }
    }

    /// The card of one kind, with the message and buttons section 5.3 gives it. The one
    /// exception is the vision-ask card, shown here as it reads once "Não era isso" has
    /// become "Pedir exceção" (section 5.4); its usual second button is `not_it`.
    pub fn card(kind: CardKind) -> NofapCard {
        use CardAction as A;
        let (message_key, note_key, actions, auto_close_secs): (&str, Option<&str>, Vec<A>, _) =
            match kind {
                CardKind::Blocked => ("block.discreet", None, vec![A::Support, A::Ok], Some(12)),
                CardKind::BlockedRepeat => ("block.repeat", None, vec![A::Surf, A::Ok], None),
                CardKind::VisionClosed => (
                    "vision.closed",
                    Some("vision.added"),
                    vec![A::Ok, A::NotIt, A::Surf, A::AddToList],
                    None,
                ),
                CardKind::VisionAsk => (
                    "vision.soft",
                    None,
                    vec![A::CloseWindow, A::RequestException],
                    Some(60),
                ),
                CardKind::Drift => ("drift.card", None, vec![A::Restore, A::Later], None),
                CardKind::PendingReady => (
                    "pending.ready",
                    None,
                    vec![A::KeepProtection, A::TurnOff],
                    None,
                ),
                CardKind::ResolverUnreachable => (
                    "resolver.unreachable",
                    None,
                    vec![A::RestoreNetwork, A::Later],
                    None,
                ),
                CardKind::Reminder => ("reminder.on", None, vec![A::Ok, A::Help], None),
                CardKind::Help => ("help.pattern", None, vec![A::Help, A::Ok], None),
            };
        NofapCard {
            id: "card-1".into(),
            kind,
            message_key: message_key.into(),
            note_key: note_key.map(Into::into),
            discreet: kind == CardKind::Blocked,
            actions,
            auto_close_secs,
            expires_at: at(10),
        }
    }

    pub fn events() -> Vec<NofapEvent> {
        vec![
            NofapEvent::Status { status: status() },
            NofapEvent::Card {
                card: card(CardKind::Blocked),
            },
            NofapEvent::CardClosed {
                id: "card-1".into(),
            },
            NofapEvent::PendingChanged,
            NofapEvent::SitesChanged,
        ]
    }

    /// Statuses that together show every [`TierState`], [`Issue`], [`GuardState`],
    /// [`VisionState`] and [`Overall`]: status `i` carries issue `i` and steps through the
    /// other lists.
    pub fn statuses() -> Vec<NofapStatus> {
        let tiers = [
            TierState::Off,
            TierState::Applied,
            TierState::UpdatePending,
            TierState::Changed,
            TierState::Missing,
            TierState::NotApplied,
            TierState::Gated,
            TierState::Unsupported {
                reason: "write_refused".into(),
            },
            TierState::InUseByOther,
        ];
        let mut guards = vec![GuardState::On, GuardState::Off];
        guards.extend(
            GuardLimit::ALL
                .into_iter()
                .map(|reason| GuardState::Limited { reason }),
        );
        let mut visions = vec![VisionState::On, VisionState::Degraded { until: at(15) }];
        visions.extend(
            VisionOff::ALL
                .into_iter()
                .map(|reason| VisionState::Off { reason }),
        );
        let overalls = Overall::ALL;
        Issue::ALL
            .into_iter()
            .enumerate()
            .map(|(i, issue)| NofapStatus {
                overall: overalls[i % overalls.len()],
                guard: guards[i % guards.len()],
                hosts: tiers[i % tiers.len()].clone(),
                resolver: tiers[(i + 3) % tiers.len()].clone(),
                policies: tiers[(i + 6) % tiers.len()].clone(),
                vision: visions[i % visions.len()].clone(),
                issues: vec![issue],
                pending: if i % 4 == 0 {
                    vec![PendingView {
                        ready: true,
                        remaining_secs: 0,
                        expires_at: Some(at(24 * 60 + 10)),
                        kind: ChangeKind::Disable,
                        ..pending_view()
                    }]
                } else {
                    vec![]
                },
                hosts_waiting: (i % 3) as u32,
                extras_left_out: (i % 2) as u32,
            })
            .collect()
    }

    pub fn views() -> GoldenViews {
        GoldenViews {
            view: view(),
            change_outcomes: vec![
                ChangeOutcome::Applied(Box::new(view())),
                ChangeOutcome::Pending(pending_view()),
            ],
            planned_changes: planned_changes(),
            apply_outcomes: apply_outcomes(),
            sites: sites(),
            support: support(),
            cards: CardKind::ALL.into_iter().map(card).collect(),
            sample_frames: vec![
                Some(SampleFrame {
                    mime: "image/jpeg".into(),
                    base64: "/9j/4AAQSkZJRg==".into(),
                    width: 640,
                    height: 400,
                }),
                None,
            ],
            export: ExportResult {
                path: "/home/ana/.local/share/ubiqx/exports/shield-2026-09-17.json".into(),
            },
            events: events(),
            statuses: statuses(),
            preview_lines: preview_lines(),
            enums: enums(),
        }
    }

    pub fn enums() -> GoldenEnums {
        GoldenEnums {
            overall: Overall::ALL.to_vec(),
            guard_limit: GuardLimit::ALL.to_vec(),
            vision_off: VisionOff::ALL.to_vec(),
            issue: Issue::ALL.to_vec(),
            change_kind: ChangeKind::ALL.to_vec(),
            card_kind: CardKind::ALL.to_vec(),
            card_action: CardAction::ALL.to_vec(),
            mood: Mood::ALL.to_vec(),
            trigger: Trigger::ALL.to_vec(),
            ai_readiness: AiReadiness::ALL.to_vec(),
            ai_provider: AiProvider::ALL.to_vec(),
            product: ProductId::ALL.to_vec(),
            os: Os::ALL.to_vec(),
            tier: Tier::ALL.to_vec(),
            resolver: ResolverId::ALL.to_vec(),
            reason: reason::ALL.into_iter().map(Into::into).collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::PathBuf;

    use serde::de::DeserializeOwned;
    use serde::Serialize;
    use serde_json::{json, Value};

    use super::*;
    use crate::nofap::{assert_all_listed, TierState};

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../apps/desktop/src/nofap/fixtures")
            .join(name)
    }

    fn pretty<T: Serialize>(value: &T) -> String {
        let mut text = serde_json::to_string_pretty(value).unwrap();
        text.push('\n');
        text
    }

    /// A Windows checkout may hand the fixtures over with CRLF line endings
    /// (`core.autocrlf`); the generated text always uses LF. No fixture value holds a line
    /// break, so this changes nothing but the line endings.
    fn unix_newlines(text: &str) -> String {
        text.replace("\r\n", "\n")
    }

    /// Compares `expected` with the committed fixture; `UBIQX_BLESS=1` rewrites the file.
    fn committed(name: &str, expected: &str) -> String {
        let path = fixture(name);
        if std::env::var("UBIQX_BLESS").is_ok_and(|v| v == "1") {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, expected).unwrap();
        }
        let committed = std::fs::read_to_string(&path).unwrap_or_else(|e| {
            panic!(
                "{}: {e}. Run `UBIQX_BLESS=1 cargo test -p ubiqx-core nofap_ipc_golden`.",
                path.display()
            )
        });
        let committed = unix_newlines(&committed);
        assert!(
            committed == expected,
            "{name} differs from what the Rust types serialise to. If the contract change is \
             intended, run `UBIQX_BLESS=1 cargo test -p ubiqx-core nofap_ipc_golden` and commit \
             the file together with the frontend types."
        );
        committed
    }

    fn golden<T>(name: &str, value: &T)
    where
        T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug,
    {
        let text = committed(name, &pretty(value));
        // The committed file also reads back into the same values.
        let parsed: T = serde_json::from_str(&text).unwrap();
        assert_eq!(&parsed, value);
    }

    /// The TypeScript twin of the two JSON files: the same values as literal types.
    fn golden_ts() -> String {
        format!(
            "// Generated by `UBIQX_BLESS=1 cargo test -p ubiqx-core nofap_ipc_golden` from\n\
             // crates/ubiqx-core/src/nofap_ipc.rs. Do not edit: the test fails when this file\n\
             // differs from what the Rust types serialise to.\n\
             //\n\
             // The same values as commands.json and views.json, as literal types. A JSON import\n\
             // widens every string to `string`, so only this module can be checked against the\n\
             // unions of types.ts: `commands satisfies DeepReadonly<NofapCommand[]>`.\n\
             \n\
             /** `T` with every array and object read-only, which is what `as const` produces. */\n\
             export type DeepReadonly<T> = T extends readonly (infer U)[]\n  \
             ? readonly DeepReadonly<U>[]\n  \
             : T extends object\n    \
             ? {{ readonly [K in keyof T]: DeepReadonly<T[K]> }}\n    \
             : T;\n\
             \n\
             /** One request per `cmd`, per kind of change and per user event. */\n\
             export const commands = {} as const;\n\
             \n\
             /** One value per reply type, event variant and card kind; statuses for every state. */\n\
             export const views = {} as const;\n",
            pretty(&sample::commands()).trim_end(),
            pretty(&sample::views()).trim_end(),
        )
    }

    #[test]
    fn nofap_ipc_golden() {
        golden("commands.json", &sample::commands());
        golden("views.json", &sample::views());
        committed("golden.ts", &golden_ts());
    }

    #[test]
    fn golden_comparison_ignores_checkout_line_endings_only() {
        let lf = pretty(&sample::commands());
        let crlf = lf.replace('\n', "\r\n");
        assert_ne!(crlf, lf);
        assert_eq!(unix_newlines(&crlf), lf);
        assert_eq!(unix_newlines(&lf), lf);
        // The generated text holds no carriage return of its own, escaped or raw, that the
        // normalisation could hide a difference behind.
        for text in [lf, pretty(&sample::views()), golden_ts()] {
            assert!(!text.contains('\r') && !text.contains("\\r"));
        }
    }

    #[test]
    fn commands_cover_every_cmd_change_and_event() {
        let commands = serde_json::to_value(sample::commands()).unwrap();
        let commands = commands.as_array().unwrap();
        let cmds: BTreeSet<&str> = commands
            .iter()
            .map(|c| c["cmd"].as_str().unwrap())
            .collect();
        let expected: BTreeSet<&str> = NofapCommand::NAMES.into_iter().collect();
        assert_eq!(cmds, expected);
        assert_eq!(expected.len(), NofapCommand::NAMES.len(), "duplicate name");
        for command in sample::commands() {
            let value = serde_json::to_value(&command).unwrap();
            assert_eq!(value["cmd"], command.name());
        }

        let kinds: BTreeSet<String> = commands
            .iter()
            .filter(|c| c["cmd"] == "request")
            .map(|c| c["args"]["kind"].as_str().unwrap().to_string())
            .collect();
        let all_kinds: BTreeSet<String> = ChangeKind::ALL
            .into_iter()
            .map(|k| {
                serde_json::to_value(k)
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .to_string()
            })
            .collect();
        assert_eq!(kinds, all_kinds);

        let events: BTreeSet<&str> = commands
            .iter()
            .filter(|c| c["cmd"] == "record")
            .map(|c| c["args"]["event"].as_str().unwrap())
            .collect();
        assert_eq!(
            events,
            BTreeSet::from([
                "lapse",
                "checkin",
                "surf_started",
                "surf_done",
                "panic",
                "step_away"
            ])
        );
    }

    #[test]
    fn command_wire_form() {
        assert_eq!(
            serde_json::to_value(NofapCommand::View).unwrap(),
            json!({ "cmd": "view" })
        );
        assert_eq!(
            serde_json::to_value(NofapCommand::Request(NofapChange::SetHosts { on: false }))
                .unwrap(),
            json!({ "cmd": "request", "args": { "kind": "set_hosts", "on": false } })
        );
        assert_eq!(
            serde_json::to_value(NofapCommand::CardAction {
                id: "c".into(),
                action: CardAction::CloseWindow
            })
            .unwrap(),
            json!({ "cmd": "card_action", "args": { "id": "c", "action": "close_window" } })
        );
        assert_eq!(
            serde_json::to_value(NofapCommand::Record(UserEvent::Checkin {
                mood: Mood::VeryBad,
                triggers: vec![Trigger::Hungry]
            }))
            .unwrap(),
            json!({ "cmd": "record", "args": { "event": "checkin", "mood": "very_bad", "triggers": ["hungry"] } })
        );
        // What the frontend sends for a command without arguments, with or without `args`.
        for text in [r#"{"cmd":"sites"}"#, r#"{"cmd":"sites","args":null}"#] {
            assert_eq!(
                serde_json::from_str::<NofapCommand>(text).unwrap(),
                NofapCommand::Sites
            );
        }
        assert!(serde_json::from_str::<NofapCommand>(r#"{"cmd":"run_this"}"#).is_err());
        assert!(serde_json::from_str::<NofapCommand>(r#"{"cmd":"request"}"#).is_err());
    }

    #[test]
    fn change_outcome_is_keyed_by_its_variant() {
        let outcomes = serde_json::to_value(sample::views().change_outcomes).unwrap();
        assert!(outcomes[0]["applied"]["status"].is_object());
        assert_eq!(outcomes[1]["pending"]["kind"], "add_allow");
    }

    #[test]
    fn view_config_never_carries_the_internal_bookkeeping() {
        let config = sample::config();
        assert!(!config.internal.salt_hex.is_empty());
        let view = serde_json::to_value(sample::view()).unwrap();
        assert!(view["config"].get("internal").is_none());
        let text = view.to_string();
        assert!(!text.contains(&config.internal.salt_hex));
        assert!(!text.contains("salt"));
        // Everything else of the configuration is there.
        let full = serde_json::to_value(&config).unwrap();
        for (key, value) in full.as_object().unwrap() {
            if key != "internal" {
                assert_eq!(&view["config"][key], value, "config.{key}");
            }
        }
    }

    #[test]
    fn views_cover_every_state_issue_card_and_event() {
        let views = sample::views();
        let statuses = serde_json::to_value(&views.statuses).unwrap();
        let mut tier_states = BTreeSet::new();
        let mut issues = BTreeSet::new();
        let mut guards = BTreeSet::new();
        let mut visions = BTreeSet::new();
        let mut overalls = BTreeSet::new();
        for status in statuses.as_array().unwrap() {
            for tier in ["hosts", "resolver", "policies"] {
                tier_states.insert(status[tier]["state"].as_str().unwrap().to_string());
            }
            for issue in status["issues"].as_array().unwrap() {
                issues.insert(issue.as_str().unwrap().to_string());
            }
            let reason = |v: &Value| v["reason"].as_str().unwrap_or("").to_string();
            guards.insert((
                status["guard"]["state"].as_str().unwrap().to_string(),
                reason(&status["guard"]),
            ));
            visions.insert((
                status["vision"]["state"].as_str().unwrap().to_string(),
                reason(&status["vision"]),
            ));
            overalls.insert(status["overall"].as_str().unwrap().to_string());
        }
        assert_eq!(tier_states.len(), 9, "{tier_states:?}");
        assert_eq!(issues.len(), Issue::ALL.len());
        assert_eq!(guards.len(), 6, "{guards:?}");
        assert_eq!(visions.len(), 13, "{visions:?}");
        assert_eq!(overalls.len(), 5);
        assert!(matches!(
            views.statuses[7].hosts,
            TierState::Unsupported { .. }
        ));

        let kinds: std::collections::HashSet<CardKind> =
            views.cards.iter().map(|c| c.kind).collect();
        assert_eq!(kinds.len(), CardKind::ALL.len());
        let event_types: BTreeSet<String> = serde_json::to_value(&views.events)
            .unwrap()
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e["type"].as_str().unwrap().to_string())
            .collect();
        assert_eq!(event_types.len(), 5);
        assert_eq!(views.apply_outcomes.len(), 4);
        assert_eq!(views.sample_frames[1], None);
    }

    #[test]
    fn enum_lists_hold_every_value_once() {
        assert_all_listed!(Mood: Ok, Meh, Hard, VeryBad);
        assert_all_listed!(Trigger: Tired, Bored, Stressed, Lonely, Hungry, Angry, Other);
        // `AiProvider::ALL` belongs to the model; checked here so that a new provider reaches
        // the fixture.
        assert_all_listed!(AiProvider: Anthropic, OpenAi, Xai, Ubi);

        // Every list is non-empty, free of duplicates and made of bare strings.
        let enums = serde_json::to_value(sample::enums()).unwrap();
        let lists = enums.as_object().unwrap();
        assert_eq!(lists.len(), 16);
        for (name, list) in lists {
            let values: Vec<&str> = list
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_str().unwrap_or_else(|| panic!("{name}: not a string")))
                .collect();
            assert!(!values.is_empty(), "{name} is empty");
            let unique: BTreeSet<&str> = values.iter().copied().collect();
            assert_eq!(unique.len(), values.len(), "{name} repeats a value");
        }
        assert_eq!(enums["mood"], json!(["ok", "meh", "hard", "very_bad"]));
        assert_eq!(enums["ai_provider"][1], "openai");
        assert_eq!(enums["product"], json!(["ubiqx", "shield"]));
    }

    /// Every reason code a serialised value carries: the `reason` of an unsupported state
    /// and of a tier report, and the `detail` of an unavailable outcome.
    fn reason_codes(value: &Value, out: &mut Vec<String>) {
        match value {
            Value::Object(map) => {
                let unsupported = map.get("state").is_some_and(|s| s == "unsupported");
                let report = map.contains_key("tier") && map.contains_key("ok");
                if unsupported || report {
                    if let Some(code) = map.get("reason").and_then(Value::as_str) {
                        out.push(code.to_string());
                    }
                }
                if map.get("outcome").is_some_and(|o| o == "unavailable") {
                    out.push(map["detail"].as_str().unwrap().to_string());
                }
                map.values().for_each(|inner| reason_codes(inner, out));
            }
            Value::Array(items) => items.iter().for_each(|inner| reason_codes(inner, out)),
            _ => {}
        }
    }

    #[test]
    fn samples_use_only_listed_preview_lines_and_reason_codes() {
        let views = sample::views();
        assert_eq!(
            views.preview_lines.len(),
            crate::nofap::preview::LINES.len()
        );
        for change in &views.planned_changes {
            let line = views
                .preview_lines
                .iter()
                .find(|l| l.key == change.text_key)
                .unwrap_or_else(|| panic!("{} is not a preview line", change.text_key));
            assert_eq!(line.tier, change.tier, "{}", change.text_key);
            assert_eq!(
                line.placeholder.is_some(),
                change.detail.is_some(),
                "{}: a detail exactly when the line has a placeholder",
                change.text_key
            );
        }

        let mut codes = Vec::new();
        reason_codes(&serde_json::to_value(&views).unwrap(), &mut codes);
        // The view, the statuses, a partial apply and an unavailable one all carry a code.
        assert!(codes.len() >= 4, "{codes:?}");
        for code in &codes {
            assert!(
                views.enums.reason.contains(code),
                "{code} is not a listed reason"
            );
        }
        // The walker sees a code that is not listed.
        let mut stray = Vec::new();
        reason_codes(
            &json!({ "hosts": { "state": "unsupported", "reason": "made_up" } }),
            &mut stray,
        );
        assert_eq!(stray, ["made_up"]);
    }

    #[test]
    fn sample_changes_follow_the_kind_order() {
        let kinds: Vec<ChangeKind> = sample::changes().iter().map(NofapChange::kind).collect();
        assert_eq!(kinds, ChangeKind::ALL.to_vec());
    }
}
