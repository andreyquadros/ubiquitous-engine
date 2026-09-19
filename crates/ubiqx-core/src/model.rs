//! Domain data types. All of them are plain, serialisable values so they can cross the
//! Tauri IPC boundary unchanged and be stored by any repository implementation.

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use serde::{Deserialize, Serialize};

pub type Id = String;

/// Generates a new random identifier (UUID v4, string form).
pub fn new_id() -> Id {
    uuid::Uuid::new_v4().to_string()
}

// ---------------------------------------------------------------------------------------------
// Capture
// ---------------------------------------------------------------------------------------------

/// What the platform layer sees in the foreground at a given instant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForegroundWindow {
    /// Human readable application name, e.g. "Google Chrome".
    pub app_name: String,
    /// Stable application identifier when the OS provides one (macOS bundle id such as
    /// `com.google.Chrome`). Falls back to the executable path or name.
    pub app_id: String,
    /// Window title as reported by the OS. May be empty when the permission to read it is missing.
    pub window_title: String,
    pub pid: Option<u32>,
    /// OS window identifier (macOS `kCGWindowNumber`), used for per-window screenshots.
    pub window_id: Option<u32>,
    /// Window bounds in screen points `(x, y, width, height)`, when known.
    pub bounds: Option<(i32, i32, u32, u32)>,
}

/// One raw observation of user activity. Produced every `sample_interval_secs` by the sampler.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActivitySample {
    pub at: DateTime<Utc>,
    pub app_name: String,
    pub app_id: String,
    pub window_title: String,
    /// Full URL for browsers (only resolved when the active app is a supported browser).
    pub url: Option<String>,
    /// Seconds since the last keyboard/mouse input. `None` when the platform cannot tell.
    pub idle_secs: Option<f64>,
    pub window_id: Option<u32>,
}

impl ActivitySample {
    /// Whether the user was away from the keyboard according to the configured threshold.
    pub fn is_idle(&self, threshold_secs: u32) -> bool {
        self.idle_secs
            .map(|s| s >= threshold_secs as f64)
            .unwrap_or(false)
    }
}

/// Where a block's category came from. Later entries in the chain override earlier ones only
/// when the user explicitly corrects a block (`User` always wins and is never overwritten).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ClassificationSource {
    Rule,
    Memory,
    Llm,
    Vision,
    User,
}

impl ClassificationSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            ClassificationSource::Rule => "rule",
            ClassificationSource::Memory => "memory",
            ClassificationSource::Llm => "llm",
            ClassificationSource::Vision => "vision",
            ClassificationSource::User => "user",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "rule" => Some(Self::Rule),
            "memory" => Some(Self::Memory),
            "llm" => Some(Self::Llm),
            "vision" => Some(Self::Vision),
            "user" => Some(Self::User),
            _ => None,
        }
    }
}

/// A contiguous span of time during which the user stayed in the same context
/// (same application and normalised window title / web domain).
///
/// Blocks are the unit of classification, reporting and correction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActivityBlock {
    pub id: Id,
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
    pub app_name: String,
    pub app_id: String,
    /// Representative (most frequent) raw window title of the block.
    pub title: String,
    /// Normalised title key used for grouping and similarity. See [`crate::normalize`].
    pub title_key: String,
    pub url: Option<String>,
    pub domain: Option<String>,
    pub category_id: Option<Id>,
    /// 0.0–1.0. `1.0` for user corrections and exact rule matches.
    pub confidence: f32,
    pub source: Option<ClassificationSource>,
    /// Short natural-language description of what the user was doing, produced by the vision
    /// or text classifier. Used verbatim in reports.
    pub description: Option<String>,
    pub screenshot_id: Option<Id>,
    /// Number of samples that were folded into this block.
    pub sample_count: u32,
    /// True while the sampler may still extend the block. Only closed blocks are classified.
    pub is_open: bool,
    /// How many times a remote classifier failed on this block (for exponential backoff).
    pub classify_attempts: u32,
    /// Do not send this block to a remote classifier before this instant.
    pub next_attempt_at: Option<DateTime<Utc>>,
    /// The block could not be classified automatically and waits for the user.
    pub needs_review: bool,
    /// Exact (redacted) text that was sent to the AI for this block, for transparency.
    pub ai_payload: Option<String>,
    pub ai_sent_at: Option<DateTime<Utc>>,
    /// Blocks created by hand ("reunião presencial 14:00–15:30") rather than by the sampler.
    pub is_manual: bool,
    pub note: Option<String>,
}

impl ActivityBlock {
    /// Creates an empty, closed block for the given span. Used by manual entries and tests.
    pub fn new_manual(
        started_at: DateTime<Utc>,
        ended_at: DateTime<Utc>,
        category_id: Id,
        note: Option<String>,
    ) -> Self {
        Self {
            id: new_id(),
            started_at,
            ended_at,
            app_name: "Manual".into(),
            app_id: "manual".into(),
            title: note.clone().unwrap_or_default(),
            title_key: note.clone().unwrap_or_default().to_lowercase(),
            url: None,
            domain: None,
            category_id: Some(category_id),
            confidence: 1.0,
            source: Some(ClassificationSource::User),
            description: note.clone(),
            screenshot_id: None,
            sample_count: 0,
            is_open: false,
            classify_attempts: 0,
            next_attempt_at: None,
            needs_review: false,
            ai_payload: None,
            ai_sent_at: None,
            is_manual: true,
            note,
        }
    }

    pub fn duration_secs(&self) -> i64 {
        (self.ended_at - self.started_at).num_seconds().max(0)
    }

    pub fn is_classified(&self) -> bool {
        self.category_id.is_some()
    }

    /// Whether an automatic classifier may (re)assign this block. User decisions are final.
    pub fn accepts_auto_classification(&self) -> bool {
        self.source != Some(ClassificationSource::User)
    }
}

/// Stored screenshot metadata. The image itself lives on disk at `path`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Screenshot {
    pub id: Id,
    pub at: DateTime<Utc>,
    pub path: String,
    pub width: u32,
    pub height: u32,
    pub app_id: String,
    pub block_id: Option<Id>,
    pub sent_to_ai: bool,
}

// ---------------------------------------------------------------------------------------------
// Categories, rules and learning
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Category {
    pub id: Id,
    pub name: String,
    /// Hex colour, e.g. `#2563EB`.
    pub color: String,
    /// Icon name from the UI icon set (lucide), e.g. `graduation-cap`.
    pub icon: String,
    /// Free-text description given to the AI so it understands what belongs here.
    pub description: String,
    /// Keywords that hint at this category (used by the AI prompt and rule suggestions).
    pub keywords: Vec<String>,
    /// Local time at which the daily report for this category is generated. `None` = use the
    /// global default from settings.
    pub report_time: Option<NaiveTime>,
    /// Free text describing how this institution wants activities written
    /// (voice, sections, what to emphasise). Fed to the report writer.
    pub report_template: Option<String>,
    /// Whether time spent here counts as productive for the focus score.
    pub is_productive: bool,
    /// Built-in categories (Uncategorized, Distraction) cannot be deleted.
    pub is_system: bool,
    pub archived: bool,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
}

/// Well-known system category identifiers.
pub mod system_categories {
    pub const UNCATEGORIZED: &str = "sys-uncategorized";
    pub const DISTRACTION: &str = "sys-distraction";
    pub const BREAK: &str = "sys-break";
    /// Time in blocked apps / private mode. Kept so daily totals stay honest, never sent to AI.
    pub const PRIVATE: &str = "sys-private";
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum RuleMatcher {
    /// Exact match on the application id (bundle id) or, if that is empty, the app name.
    App,
    /// Match on the web domain (suffix match: `ifro.edu.br` matches `sei.ifro.edu.br`).
    Domain,
    /// Case-insensitive substring match on the window title.
    TitleContains,
    /// Regular expression evaluated against `"<app_name> | <title> | <url>"`.
    Regex,
}

impl RuleMatcher {
    pub fn as_str(&self) -> &'static str {
        match self {
            RuleMatcher::App => "app",
            RuleMatcher::Domain => "domain",
            RuleMatcher::TitleContains => "title_contains",
            RuleMatcher::Regex => "regex",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "app" => Some(Self::App),
            "domain" => Some(Self::Domain),
            "title_contains" => Some(Self::TitleContains),
            "regex" => Some(Self::Regex),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum RuleOrigin {
    /// Created explicitly by the user.
    User,
    /// Suggested by the learning engine from corrections and confirmed (or auto-applied).
    Learned,
}

impl RuleOrigin {
    pub fn as_str(&self) -> &'static str {
        match self {
            RuleOrigin::User => "user",
            RuleOrigin::Learned => "learned",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "user" => Some(Self::User),
            "learned" => Some(Self::Learned),
            _ => None,
        }
    }
}

/// A deterministic classification rule. Rules are evaluated before any AI call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rule {
    pub id: Id,
    pub category_id: Id,
    pub matcher: RuleMatcher,
    pub pattern: String,
    /// Higher wins when several rules match.
    pub priority: i32,
    pub origin: RuleOrigin,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    /// How many blocks this rule has classified (for the UI and for pruning useless rules).
    pub hit_count: u32,
    /// How many times the user corrected a block this rule had classified.
    pub miss_count: u32,
    pub last_contradicted_at: Option<DateTime<Utc>>,
}

impl Rule {
    /// A rule that keeps being contradicted is disabled automatically.
    pub fn should_auto_disable(&self) -> bool {
        self.miss_count >= 2
            || (self.hit_count >= 5 && self.miss_count as f32 / self.hit_count as f32 > 0.3)
    }
}

/// A user correction of a block's category. This is the learning signal of the system.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Correction {
    pub id: Id,
    pub block_id: Id,
    pub from_category_id: Option<Id>,
    pub to_category_id: Id,
    pub app_id: String,
    pub app_name: String,
    pub title_key: String,
    pub domain: Option<String>,
    pub note: Option<String>,
    pub at: DateTime<Utc>,
}

/// A rule the learning engine proposes after observing consistent corrections.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuleSuggestion {
    pub category_id: Id,
    pub matcher: RuleMatcher,
    pub pattern: String,
    /// Number of corrections supporting this suggestion.
    pub support: u32,
    pub rationale: String,
    /// Safe to apply without confirmation (specific domain, not a multi-tenant service).
    pub auto_apply_safe: bool,
}

// ---------------------------------------------------------------------------------------------
// Reports and insights
// ---------------------------------------------------------------------------------------------

/// Kind of activity, used to group and phrase items in institutional reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ActivityKind {
    Desenvolvimento,
    Reuniao,
    Comunicacao,
    Documentacao,
    Ensino,
    Pesquisa,
    Extensao,
    Gestao,
    Outro,
}

impl ActivityKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ActivityKind::Desenvolvimento => "desenvolvimento",
            ActivityKind::Reuniao => "reuniao",
            ActivityKind::Comunicacao => "comunicacao",
            ActivityKind::Documentacao => "documentacao",
            ActivityKind::Ensino => "ensino",
            ActivityKind::Pesquisa => "pesquisa",
            ActivityKind::Extensao => "extensao",
            ActivityKind::Gestao => "gestao",
            ActivityKind::Outro => "outro",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "desenvolvimento" => Some(Self::Desenvolvimento),
            "reuniao" => Some(Self::Reuniao),
            "comunicacao" => Some(Self::Comunicacao),
            "documentacao" => Some(Self::Documentacao),
            "ensino" => Some(Self::Ensino),
            "pesquisa" => Some(Self::Pesquisa),
            "extensao" => Some(Self::Extensao),
            "gestao" => Some(Self::Gestao),
            "outro" => Some(Self::Outro),
            _ => None,
        }
    }

    pub fn label_pt(&self) -> &'static str {
        match self {
            ActivityKind::Desenvolvimento => "Desenvolvimento",
            ActivityKind::Reuniao => "Reunião",
            ActivityKind::Comunicacao => "Comunicação",
            ActivityKind::Documentacao => "Documentação",
            ActivityKind::Ensino => "Ensino",
            ActivityKind::Pesquisa => "Pesquisa",
            ActivityKind::Extensao => "Extensão",
            ActivityKind::Gestao => "Gestão",
            ActivityKind::Outro => "Outro",
        }
    }

    pub fn label_en(&self) -> &'static str {
        match self {
            ActivityKind::Desenvolvimento => "Development",
            ActivityKind::Reuniao => "Meetings",
            ActivityKind::Comunicacao => "Communication",
            ActivityKind::Documentacao => "Documentation",
            ActivityKind::Ensino => "Teaching",
            ActivityKind::Pesquisa => "Research",
            ActivityKind::Extensao => "Outreach",
            ActivityKind::Gestao => "Administration",
            ActivityKind::Outro => "Other",
        }
    }

    /// Heading of this kind in reports, in the UI language.
    pub fn label(&self, lang: crate::lang::UiLanguage) -> &'static str {
        match lang {
            crate::lang::UiLanguage::PtBr => self.label_pt(),
            crate::lang::UiLanguage::En => self.label_en(),
        }
    }
}

/// One line of a report: what was done, for how long, with which evidence. Items are the
/// product — the monthly report is assembled from them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReportItem {
    /// Activity in the past tense, first person, in the report language.
    pub activity: String,
    pub kind: ActivityKind,
    pub minutes: u32,
    /// Apps, documents, domains or project names that support the item (never people).
    pub evidence: Vec<String>,
    /// Local time range, e.g. "09:10–10:25", or empty when the item spans the day.
    pub time_range: String,
    /// When this continues an item from a previous day, its activity text.
    pub continuation_of: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DailyReport {
    pub id: Id,
    /// Local calendar day the report covers.
    pub date: NaiveDate,
    pub category_id: Id,
    pub generated_at: DateTime<Utc>,
    /// Markdown body ready to paste into a monthly activity report.
    pub summary_md: String,
    pub items: Vec<ReportItem>,
    pub highlights: Vec<String>,
    pub total_secs: i64,
    pub model: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    /// Blocks changed after generation; the UI offers "Regenerar".
    pub stale: bool,
    /// The user edited the items by hand; automatic regeneration must not overwrite them.
    pub edited: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum NudgeKind {
    /// Long stretch in non-productive categories.
    Unproductive,
    /// Many context switches in a short window.
    Distracted,
    /// Long focused session without a break.
    BreakSuggested,
    /// Positive reinforcement after a good focus streak.
    Praise,
    /// User has been idle for a long time while tracking is on.
    Idle,
    /// A daily report is ready.
    ReportReady,
    /// Something needs the user's attention (permission missing, API key invalid…).
    Attention,
    /// UBI noticed a lot of window switching and asks which task to focus on (the
    /// dashboard turns this one into a small form that starts a focus session).
    FocusPrompt,
}

impl NudgeKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            NudgeKind::Unproductive => "unproductive",
            NudgeKind::Distracted => "distracted",
            NudgeKind::BreakSuggested => "break_suggested",
            NudgeKind::Praise => "praise",
            NudgeKind::Idle => "idle",
            NudgeKind::ReportReady => "report_ready",
            NudgeKind::Attention => "attention",
            NudgeKind::FocusPrompt => "focus_prompt",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "unproductive" => Some(Self::Unproductive),
            "distracted" => Some(Self::Distracted),
            "break_suggested" => Some(Self::BreakSuggested),
            "praise" => Some(Self::Praise),
            "idle" => Some(Self::Idle),
            "report_ready" => Some(Self::ReportReady),
            "attention" => Some(Self::Attention),
            "focus_prompt" => Some(Self::FocusPrompt),
            _ => None,
        }
    }
}

/// A message from UBI (the mascot) to the user.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Nudge {
    pub id: Id,
    pub at: DateTime<Utc>,
    pub kind: NudgeKind,
    pub title: String,
    pub message: String,
    pub seen: bool,
}

/// UBI's mood, derived from the recent focus score. Drives the mascot animation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mood {
    Sleeping,
    Calm,
    Focused,
    Excited,
    Worried,
}

/// Aggregated numbers for a period, computed locally by the insight engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FocusStats {
    /// 0–100.
    pub focus_score: u8,
    pub productive_secs: i64,
    pub distraction_secs: i64,
    pub uncategorized_secs: i64,
    pub idle_secs: i64,
    pub total_secs: i64,
    /// Context switches per hour of active time.
    pub switches_per_hour: f32,
    /// Longest uninterrupted productive stretch in seconds.
    pub longest_focus_secs: i64,
    pub mood: Mood,
}

// ---------------------------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------------------------

/// Secret-store key under which the ubiqX license key is stored. It never sits in the
/// settings JSON row: the UI sees a [`crate::license::LicenseStatus`] instead.
pub const LICENSE_SECRET_KEY: &str = "ubiqx.license";

/// Where the plans are sold; the console link of the [`AiProvider::Ubi`] provider.
pub const PLANS_URL: &str = "https://andreyquadros.github.io/ubiquitous-engine/#planos";

/// Which hosted LLM vendor answers the remote calls. Every provider is reached through its
/// own HTTP client; the user picks one in onboarding / settings and pastes that vendor's key.
/// [`AiProvider::Ubi`] is the managed option: the calls go to the Ubi proxy, authenticated
/// with the license key of a `monthly_managed` subscription instead of a vendor key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AiProvider {
    #[default]
    Anthropic,
    #[serde(rename = "openai")]
    OpenAi,
    Xai,
    /// "IA do Ubi": Anthropic-compatible calls through the Ubi proxy, paid by the monthly
    /// plan. Needs a valid `monthly_managed` license (see [`crate::license`]).
    Ubi,
}

impl AiProvider {
    pub const ALL: [AiProvider; 4] = [
        AiProvider::Anthropic,
        AiProvider::OpenAi,
        AiProvider::Xai,
        AiProvider::Ubi,
    ];

    /// Stable id used in settings, secrets and the IPC contract.
    pub fn id(self) -> &'static str {
        match self {
            AiProvider::Anthropic => "anthropic",
            AiProvider::OpenAi => "openai",
            AiProvider::Xai => "xai",
            AiProvider::Ubi => "ubi",
        }
    }

    pub fn parse(id: &str) -> Option<Self> {
        match id.trim().to_ascii_lowercase().as_str() {
            "anthropic" => Some(AiProvider::Anthropic),
            "openai" => Some(AiProvider::OpenAi),
            "xai" | "grok" => Some(AiProvider::Xai),
            "ubi" => Some(AiProvider::Ubi),
            _ => None,
        }
    }

    /// Human label (vendor + product).
    pub fn label(self) -> &'static str {
        match self {
            AiProvider::Anthropic => "Anthropic Claude",
            AiProvider::OpenAi => "OpenAI",
            AiProvider::Xai => "xAI Grok",
            AiProvider::Ubi => "IA do Ubi",
        }
    }

    /// Where the user creates an API key (for Ubi: where the plan is bought).
    pub fn console_url(self) -> &'static str {
        match self {
            AiProvider::Anthropic => "https://console.anthropic.com/settings/keys",
            AiProvider::OpenAi => "https://platform.openai.com/api-keys",
            AiProvider::Xai => "https://console.x.ai",
            AiProvider::Ubi => PLANS_URL,
        }
    }

    /// Typical key prefix, for placeholders and a soft sanity check in the UI.
    pub fn key_prefix(self) -> &'static str {
        match self {
            AiProvider::Anthropic => "sk-ant-",
            AiProvider::OpenAi => "sk-",
            AiProvider::Xai => "xai-",
            AiProvider::Ubi => crate::license::KEY_PREFIX,
        }
    }

    /// Secret-store key under which this provider's API key is stored. For Ubi it is the
    /// license key itself ([`LICENSE_SECRET_KEY`]): the proxy authenticates with it.
    pub fn secret_key(self) -> &'static str {
        match self {
            AiProvider::Anthropic => crate::ports::secret_keys::ANTHROPIC_API_KEY,
            AiProvider::OpenAi => crate::ports::secret_keys::OPENAI_API_KEY,
            AiProvider::Xai => crate::ports::secret_keys::XAI_API_KEY,
            AiProvider::Ubi => LICENSE_SECRET_KEY,
        }
    }

    /// Whether the provider is the managed one (AI paid by the monthly plan, no vendor key,
    /// fixed model aliases).
    pub fn is_managed(self) -> bool {
        matches!(self, AiProvider::Ubi)
    }

    /// Whether a model id belongs to this vendor's naming scheme. Unknown ids belong to
    /// nobody (`None` from [`AiProvider::for_model`]), so custom ids are never overwritten.
    pub fn owns_model(self, model: &str) -> bool {
        AiProvider::for_model(model) == Some(self)
    }

    /// Vendor inferred from a model id (`claude-*`, `gpt-*`/`o*`/`chatgpt-*`/`ft:*`, `grok-*`,
    /// `ubi-*`).
    pub fn for_model(model: &str) -> Option<Self> {
        let m = model.trim().to_ascii_lowercase();
        if m.starts_with("claude") {
            Some(AiProvider::Anthropic)
        } else if m.starts_with("grok") {
            Some(AiProvider::Xai)
        } else if m.starts_with("ubi-") {
            Some(AiProvider::Ubi)
        } else if m.starts_with("gpt")
            || m.starts_with("chatgpt")
            || m.starts_with("ft:")
            || (m.starts_with('o') && m[1..].starts_with(|c: char| c.is_ascii_digit()))
        {
            Some(AiProvider::OpenAi)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AiModels {
    /// Cheap model for high-volume block classification.
    pub classify: String,
    /// Vision-capable model for ambiguous blocks (screenshot → description + category).
    pub vision: String,
    /// Better model for daily reports and weekly recommendations.
    pub report: String,
}

impl Default for AiModels {
    fn default() -> Self {
        Self::for_provider(AiProvider::Anthropic)
    }
}

impl AiModels {
    /// Recommended models per vendor: a cheap multimodal model for the high-volume
    /// classification and vision calls, a stronger one for the few daily reports.
    pub fn for_provider(provider: AiProvider) -> Self {
        match provider {
            AiProvider::Anthropic => Self {
                classify: "claude-haiku-4-5".into(),
                vision: "claude-haiku-4-5".into(),
                report: "claude-sonnet-5".into(),
            },
            AiProvider::OpenAi => Self {
                classify: "gpt-5-mini".into(),
                vision: "gpt-5-mini".into(),
                report: "gpt-5".into(),
            },
            AiProvider::Xai => Self {
                classify: "grok-4-1-fast-non-reasoning".into(),
                vision: "grok-4-1-fast-non-reasoning".into(),
                report: "grok-4-1-fast-reasoning".into(),
            },
            // The proxy only accepts its two aliases; the vendor models behind them are
            // chosen server-side.
            AiProvider::Ubi => Self {
                classify: crate::license::UBI_MODEL_FAST.into(),
                vision: crate::license::UBI_MODEL_FAST.into(),
                report: crate::license::UBI_MODEL_SMART.into(),
            },
        }
    }

    /// Replaces every model id that visibly belongs to another vendor (or is blank) with the
    /// provider's recommendation, keeping custom/unknown ids untouched. The managed provider
    /// has no custom ids: its aliases are fixed (`ubi-fast` / `ubi-smart`).
    pub fn reconciled_with(mut self, provider: AiProvider) -> Self {
        if provider.is_managed() {
            return Self::for_provider(provider);
        }
        let defaults = Self::for_provider(provider);
        let fix = |current: &mut String, default: String| {
            let foreign = AiProvider::for_model(current).is_some_and(|p| p != provider);
            if current.trim().is_empty() || foreign {
                *current = default;
            }
        };
        fix(&mut self.classify, defaults.classify);
        fix(&mut self.vision, defaults.vision);
        fix(&mut self.report, defaults.report);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuietHours {
    pub enabled: bool,
    pub start: NaiveTime,
    pub end: NaiveTime,
}

impl Default for QuietHours {
    fn default() -> Self {
        Self {
            enabled: true,
            start: NaiveTime::from_hms_opt(20, 0, 0).expect("valid"),
            end: NaiveTime::from_hms_opt(8, 0, 0).expect("valid"),
        }
    }
}

impl QuietHours {
    /// Whether `t` falls inside the quiet window (which may wrap past midnight).
    pub fn contains(&self, t: NaiveTime) -> bool {
        if !self.enabled {
            return false;
        }
        if self.start <= self.end {
            t >= self.start && t < self.end
        } else {
            t >= self.start || t < self.end
        }
    }
}

/// Which screenshots may be sent to the vision model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum VisionPolicy {
    /// Screenshots stay on this machine; the AI only ever sees text.
    Never,
    /// Only screenshots of these application ids may be sent.
    OnlyApps { apps: Vec<String> },
    /// Any app except `blocked_apps` and `vision_denied_apps`.
    AllExceptBlocked,
}

impl VisionPolicy {
    pub fn allows(
        &self,
        app_id: &str,
        app_name: &str,
        blocked: &[String],
        denied: &[String],
    ) -> bool {
        let is_in = |list: &[String]| {
            list.iter()
                .any(|b| b.eq_ignore_ascii_case(app_id) || b.eq_ignore_ascii_case(app_name))
        };
        match self {
            VisionPolicy::Never => false,
            VisionPolicy::OnlyApps { apps } => is_in(apps) && !is_in(blocked),
            VisionPolicy::AllExceptBlocked => !is_in(blocked) && !is_in(denied),
        }
    }
}

/// Per-kind switches for UBI's nudges.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct NudgeSettings {
    pub enabled: bool,
    pub unproductive: bool,
    pub distracted: bool,
    pub break_suggested: bool,
    pub praise: bool,
    pub idle: bool,
    /// Hard cap of nudges per local day (report-ready and attention nudges are exempt).
    pub max_per_day: u32,
    /// Minutes between nudges of the same kind.
    pub cooldown_mins: u32,
    /// Apps in which UBI stays silent (meetings, presentations).
    pub silent_apps: Vec<String>,
    /// Snoozed until this instant (set from the tray).
    pub snoozed_until: Option<DateTime<Utc>>,
}

impl Default for NudgeSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            unproductive: true,
            distracted: true,
            break_suggested: true,
            praise: true,
            idle: false,
            max_per_day: 4,
            cooldown_mins: 60,
            silent_apps: vec![
                "us.zoom.xos".into(),
                "com.microsoft.teams2".into(),
                "com.microsoft.teams".into(),
                "com.apple.iWork.Keynote".into(),
                "com.google.Chrome.app.meet".into(),
            ],
            snoozed_until: None,
        }
    }
}

/// The focus guard: blocked apps/sites, focus sessions and what happens when one starts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct FocusSettings {
    /// Whether the guard loop enforces the list of blocked apps and sites at all.
    pub guard_enabled: bool,
    /// During a focus session, also hold anything the user's rules map to `Distraction`.
    pub block_distraction_in_session: bool,
    /// Hide every other application's windows when a focus session starts.
    pub hide_others_on_start: bool,
    /// Default length of a focus session, in minutes.
    pub session_minutes: u32,
    /// Seconds between two interventions on the same app/site.
    pub intervention_cooldown_secs: u32,
    /// Name of a macOS Shortcut to run when a session starts (e.g. one that turns on a
    /// Focus mode). `None` = nothing.
    pub macos_focus_shortcut_on: Option<String>,
    /// Name of a macOS Shortcut to run when a session ends.
    pub macos_focus_shortcut_off: Option<String>,
}

impl Default for FocusSettings {
    fn default() -> Self {
        Self {
            guard_enabled: true,
            block_distraction_in_session: true,
            hide_others_on_start: true,
            session_minutes: 45,
            intervention_cooldown_secs: 20,
            macos_focus_shortcut_on: None,
            macos_focus_shortcut_off: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Whether tracking is active at all.
    pub tracking_enabled: bool,
    /// Seconds between foreground samples.
    pub sample_interval_secs: u32,
    /// Seconds of no input after which the user is considered away.
    pub idle_threshold_secs: u32,
    /// Blocks shorter than this are merged into their neighbours.
    pub min_block_secs: u32,
    /// Seconds between screenshots (0 disables screenshots entirely).
    pub screenshot_interval_secs: u32,
    /// Longest edge of stored screenshots, in pixels.
    pub screenshot_max_edge: u32,
    /// Hours to keep screenshots on disk after they were taken (they are deleted right after
    /// classification unless `keep_screenshots_for_review` is on).
    pub screenshot_retention_hours: u32,
    pub keep_screenshots_for_review: bool,
    pub vision_policy: VisionPolicy,
    /// Apps the user explicitly excluded from vision even under `AllExceptBlocked`.
    pub vision_denied_apps: Vec<String>,
    /// Application ids / names that are never sampled nor captured.
    pub blocked_apps: Vec<String>,
    /// Web domains that are never recorded (the block keeps app name only).
    pub blocked_domains: Vec<String>,
    /// Pauses sampling and screenshots without stopping the app. Timed: `private_until = None`
    /// with `private_mode = true` means "until I turn it off".
    pub private_mode: bool,
    pub private_until: Option<DateTime<Utc>>,
    /// Which vendor answers remote calls. Its key lives in the secret store under
    /// [`AiProvider::secret_key`]; keys of the other vendors are kept so switching is free.
    pub ai_provider: AiProvider,
    /// Effective model ids for the selected provider (see [`AiModels::reconciled_with`]).
    pub models: AiModels,
    /// Maximum screenshots sent to the vision model per hour.
    pub max_vision_per_hour: u32,
    /// Hard monthly cap on estimated AI spend. At 80% UBI warns; at 100% remote classifiers stop.
    pub ai_monthly_budget_usd: f64,
    /// Flush pending blocks to the remote classifier when at least this many are waiting…
    pub classify_batch_min: u32,
    /// …or when the oldest pending block is at least this old (seconds).
    pub classify_max_wait_secs: u32,
    /// Local mode: never call a remote AI (classification stops after rules/memory).
    pub local_only: bool,
    /// Blocks with confidence below this go to the next classifier in the chain.
    pub min_confidence: f32,
    /// Default local time for daily reports when a category has none.
    pub report_default_time: NaiveTime,
    /// UI language (BCP-47): `pt-BR` or `en`, see [`Settings::ui_language`]. Every text the
    /// engine generates (nudges, notifications, reports, prompts) follows it.
    pub language: String,
    /// Who the user is, in their own words. Given to the report writer for context.
    pub user_profile: Option<String>,
    pub quiet_hours: QuietHours,
    pub nudges: NudgeSettings,
    pub launch_at_login: bool,
    /// Onboarding finished (permissions granted, key stored, categories created).
    pub onboarding_done: bool,
    /// Look for a newer build in the update feed automatically (shortly after start, then
    /// every few hours). A manual check works regardless.
    #[serde(default = "default_true")]
    pub check_updates: bool,
    /// Blocked apps/sites and focus sessions (see [`FocusSettings`]).
    pub focus: FocusSettings,
}

fn default_true() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            tracking_enabled: true,
            sample_interval_secs: 5,
            idle_threshold_secs: 180,
            min_block_secs: 20,
            screenshot_interval_secs: 120,
            screenshot_max_edge: 1280,
            screenshot_retention_hours: 24,
            keep_screenshots_for_review: false,
            vision_policy: VisionPolicy::AllExceptBlocked,
            vision_denied_apps: vec![],
            blocked_apps: vec![
                "com.1password.1password".into(),
                "com.agilebits.onepassword7".into(),
                "com.apple.keychainaccess".into(),
                "com.bitwarden.desktop".into(),
                "com.apple.Passwords".into(),
                "com.apple.Passbook".into(),
            ],
            blocked_domains: vec![],
            private_mode: false,
            private_until: None,
            ai_provider: AiProvider::Anthropic,
            models: AiModels::default(),
            max_vision_per_hour: 20,
            ai_monthly_budget_usd: 5.0,
            classify_batch_min: 8,
            classify_max_wait_secs: 240,
            local_only: false,
            min_confidence: 0.6,
            report_default_time: NaiveTime::from_hms_opt(18, 0, 0).expect("valid"),
            language: "pt-BR".into(),
            user_profile: None,
            quiet_hours: QuietHours::default(),
            nudges: NudgeSettings::default(),
            launch_at_login: true,
            onboarding_done: false,
            check_updates: true,
            focus: FocusSettings::default(),
        }
    }
}

impl Settings {
    /// Makes `models` consistent with `ai_provider` (call after any settings write from the UI).
    pub fn reconcile_models(&mut self) {
        self.models = self.models.clone().reconciled_with(self.ai_provider);
    }

    /// The UI language, parsed leniently from `language` (unknown tags mean pt-BR).
    pub fn ui_language(&self) -> crate::lang::UiLanguage {
        crate::lang::UiLanguage::from_tag(&self.language)
    }

    /// Rewrites `language` as the canonical tag of the language it names (`pt-BR` or `en`), so
    /// every consumer compares against one spelling.
    pub fn normalize_language(&mut self) {
        self.language = self.ui_language().tag().to_string();
    }

    /// Everything a settings write from the UI needs before it is stored: provider-consistent
    /// model ids and a canonical language tag.
    pub fn sanitize(&mut self) {
        self.reconcile_models();
        self.normalize_language();
    }

    /// Whether private mode is active at `now` (a timed private mode expires on its own).
    pub fn is_private(&self, now: DateTime<Utc>) -> bool {
        self.private_mode && self.private_until.map(|t| now < t).unwrap_or(true)
    }
}

/// Events the engine emits for the UI shells (Tauri, CLI). Defined here so the engine never
/// depends on a concrete shell.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EngineEvent {
    BlockOpened {
        block: ActivityBlock,
    },
    BlockClosed {
        block: ActivityBlock,
    },
    BlocksClassified {
        block_ids: Vec<Id>,
    },
    ReportReady {
        report: DailyReport,
    },
    Nudge {
        nudge: Nudge,
    },
    TrackerState {
        state: TrackerState,
    },
    AiHealth {
        health: AiHealth,
    },
    PermissionRequired {
        permission: String,
    },
    ScreenshotTaken {
        screenshot_id: Id,
        block_id: Option<Id>,
    },
    /// A newer build was found in the update feed (once per build, and on every manual
    /// check that finds one).
    UpdateAvailable {
        release: crate::update::ReleaseInfo,
    },
    /// The focus guard held a blocked app or site (already persisted).
    Intervention {
        intervention: Intervention,
    },
    /// A focus session started, ended (`ended_at` set) or counted one more intervention.
    FocusSession {
        session: Option<FocusSession>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrackerState {
    Running,
    Paused,
    Private,
    Idle,
    /// A required permission is missing; sampling produces no useful data.
    Blocked,
}

/// Health of the remote AI path, surfaced through UBI's mood and the settings screen.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum AiHealth {
    Ok,
    NotConfigured,
    /// Temporarily paused after transient failures; retried automatically.
    Degraded {
        reason: String,
        until: DateTime<Utc>,
    },
    /// Stopped until the user acts (invalid key, budget exhausted).
    Paused {
        reason: String,
    },
}

// ---------------------------------------------------------------------------------------------
// Focus guard
// ---------------------------------------------------------------------------------------------

/// What a focus target names: an installed application or a web site.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FocusTargetKind {
    App,
    Site,
}

impl FocusTargetKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            FocusTargetKind::App => "app",
            FocusTargetKind::Site => "site",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "app" => Some(Self::App),
            "site" => Some(Self::Site),
            _ => None,
        }
    }
}

/// An app or site the user asked UBI to hold. `key` is the bundle id for apps
/// (`com.tinyspeck.slackmacgap`) and the lower-case registrable domain for sites
/// (`youtube.com`, which also covers every subdomain).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FocusTarget {
    pub id: Id,
    pub kind: FocusTargetKind,
    pub name: String,
    pub key: String,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub last_blocked_at: Option<DateTime<Utc>>,
    pub blocked_count: u32,
}

/// An application found on this machine, offered when the user types a name to block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledApp {
    pub name: String,
    pub bundle_id: String,
    pub path: String,
}

/// What the guard managed to do about a hit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InterventionAction {
    /// The application was asked to quit and agreed.
    AppQuit,
    /// The browser closed the active tab.
    TabClosed,
    /// Closing failed; the tab was sent to `about:blank` instead.
    TabBlanked,
    /// Nothing could be enforced; the user was only told.
    Notified,
}

impl InterventionAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            InterventionAction::AppQuit => "app_quit",
            InterventionAction::TabClosed => "tab_closed",
            InterventionAction::TabBlanked => "tab_blanked",
            InterventionAction::Notified => "notified",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "app_quit" => Some(Self::AppQuit),
            "tab_closed" => Some(Self::TabClosed),
            "tab_blanked" => Some(Self::TabBlanked),
            "notified" => Some(Self::Notified),
            _ => None,
        }
    }
}

/// One time the guard held a distraction. `target_id` is `None` when the hit came from a
/// classification rule (distraction category during a session) rather than from the list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Intervention {
    pub id: Id,
    pub at: DateTime<Utc>,
    pub target_id: Option<Id>,
    pub kind: FocusTargetKind,
    pub name: String,
    pub key: String,
    pub action: InterventionAction,
    pub session_id: Option<Id>,
    /// What UBI said, in the UI language of the moment.
    pub message: String,
}

/// A timed stretch of work on one task, during which the guard is stricter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FocusSession {
    pub id: Id,
    pub task: String,
    pub started_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    /// Interventions counted while the session ran.
    pub interventions: u32,
    /// Other windows were hidden when it started.
    pub hid_windows: bool,
    /// The configured macOS Shortcut ran when it started.
    pub ran_shortcut: bool,
}

impl FocusSession {
    pub fn is_active(&self) -> bool {
        self.ended_at.is_none()
    }

    /// Seconds left until `ends_at` (never negative).
    pub fn remaining_secs(&self, now: DateTime<Utc>) -> i64 {
        (self.ends_at - now).num_seconds().max(0)
    }

    /// Planned length in minutes.
    pub fn planned_minutes(&self) -> i64 {
        (self.ends_at - self.started_at).num_minutes().max(0)
    }
}

/// The focus page's header numbers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FocusStatus {
    pub session: Option<FocusSession>,
    pub remaining_secs: Option<i64>,
    pub targets_enabled: u32,
    pub interventions_today: u32,
    pub guard_enabled: bool,
}

/// A domain seen in the user's own activity, with the time spent there.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnownDomain {
    pub domain: String,
    pub seconds: i64,
}

// ---------------------------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------------------------

/// Half-open time range `[from, to)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeRange {
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
}

impl TimeRange {
    pub fn new(from: DateTime<Utc>, to: DateTime<Utc>) -> Self {
        Self { from, to }
    }

    pub fn contains(&self, t: DateTime<Utc>) -> bool {
        t >= self.from && t < self.to
    }
}

/// Time spent per category inside a range, ready for charts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CategoryTotal {
    pub category_id: Option<Id>,
    pub secs: i64,
    pub block_count: u32,
}

/// Time spent per application inside a range.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppTotal {
    pub app_id: String,
    pub app_name: String,
    pub secs: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quiet_hours_wrap_midnight() {
        let q = QuietHours::default(); // 20:00 → 08:00
        assert!(q.contains(NaiveTime::from_hms_opt(23, 0, 0).unwrap()));
        assert!(q.contains(NaiveTime::from_hms_opt(3, 0, 0).unwrap()));
        assert!(!q.contains(NaiveTime::from_hms_opt(12, 0, 0).unwrap()));
        let q2 = QuietHours {
            enabled: false,
            ..q
        };
        assert!(!q2.contains(NaiveTime::from_hms_opt(23, 0, 0).unwrap()));
    }

    #[test]
    fn enum_round_trips() {
        for s in [
            ClassificationSource::Rule,
            ClassificationSource::Memory,
            ClassificationSource::Llm,
            ClassificationSource::Vision,
            ClassificationSource::User,
        ] {
            assert_eq!(ClassificationSource::parse(s.as_str()), Some(s));
        }
        for m in [
            RuleMatcher::App,
            RuleMatcher::Domain,
            RuleMatcher::TitleContains,
            RuleMatcher::Regex,
        ] {
            assert_eq!(RuleMatcher::parse(m.as_str()), Some(m));
        }
        for k in [
            NudgeKind::Unproductive,
            NudgeKind::Distracted,
            NudgeKind::BreakSuggested,
            NudgeKind::Praise,
            NudgeKind::Idle,
            NudgeKind::ReportReady,
            NudgeKind::Attention,
            NudgeKind::FocusPrompt,
        ] {
            assert_eq!(NudgeKind::parse(k.as_str()), Some(k));
        }
    }

    #[test]
    fn vision_policy() {
        let blocked = vec!["com.1password".to_string()];
        assert!(!VisionPolicy::Never.allows("a", "A", &blocked, &[]));
        assert!(VisionPolicy::AllExceptBlocked.allows("a", "A", &blocked, &[]));
        assert!(!VisionPolicy::AllExceptBlocked.allows(
            "com.1password",
            "1Password",
            &blocked,
            &[]
        ));
        assert!(!VisionPolicy::AllExceptBlocked.allows("a", "A", &blocked, &["A".into()]));
        let only = VisionPolicy::OnlyApps {
            apps: vec!["a".into()],
        };
        assert!(only.allows("a", "A", &blocked, &[]));
        assert!(!only.allows("b", "B", &blocked, &[]));
    }

    #[test]
    fn timed_private_mode() {
        let now = Utc::now();
        let mut s = Settings {
            private_mode: true,
            ..Default::default()
        };
        assert!(s.is_private(now));
        s.private_until = Some(now - chrono::Duration::minutes(1));
        assert!(!s.is_private(now));
        s.private_until = Some(now + chrono::Duration::minutes(1));
        assert!(s.is_private(now));
    }

    #[test]
    fn activity_kind_round_trip() {
        for k in [
            ActivityKind::Desenvolvimento,
            ActivityKind::Reuniao,
            ActivityKind::Comunicacao,
            ActivityKind::Documentacao,
            ActivityKind::Ensino,
            ActivityKind::Pesquisa,
            ActivityKind::Extensao,
            ActivityKind::Gestao,
            ActivityKind::Outro,
        ] {
            assert_eq!(ActivityKind::parse(k.as_str()), Some(k));
        }
    }

    #[test]
    fn settings_default_serialises_with_missing_fields() {
        let s: Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(s, Settings::default());
    }

    #[test]
    fn ubi_provider_ids_keys_and_models() {
        assert_eq!(serde_json::to_value(AiProvider::Ubi).unwrap(), "ubi");
        assert_eq!(
            serde_json::from_str::<AiProvider>("\"ubi\"").unwrap(),
            AiProvider::Ubi
        );
        assert_eq!(AiProvider::parse("UBI"), Some(AiProvider::Ubi));
        assert_eq!(AiProvider::Ubi.id(), "ubi");
        assert_eq!(AiProvider::Ubi.secret_key(), LICENSE_SECRET_KEY);
        assert_eq!(AiProvider::Ubi.key_prefix(), "UBIQX-");
        assert_eq!(AiProvider::Ubi.console_url(), PLANS_URL);
        assert!(AiProvider::Ubi.is_managed());
        assert!(!AiProvider::Anthropic.is_managed());
        assert_eq!(AiProvider::ALL.last(), Some(&AiProvider::Ubi));
        for p in AiProvider::ALL {
            assert_eq!(AiProvider::parse(p.id()), Some(p));
        }

        assert_eq!(AiProvider::for_model("ubi-fast"), Some(AiProvider::Ubi));
        assert_eq!(AiProvider::for_model("ubi-smart"), Some(AiProvider::Ubi));
        assert_eq!(AiProvider::for_model("ubiquitous"), None);
        assert!(AiProvider::Ubi.owns_model("ubi-smart"));

        let models = AiModels::for_provider(AiProvider::Ubi);
        assert_eq!(models.classify, "ubi-fast");
        assert_eq!(models.vision, "ubi-fast");
        assert_eq!(models.report, "ubi-smart");
    }

    #[test]
    fn ubi_models_are_fixed_aliases_and_replaced_on_switch() {
        // Custom ids do not survive a switch to the managed provider…
        let custom = AiModels {
            classify: "my-custom".into(),
            vision: "claude-haiku-4-5".into(),
            report: "gpt-5".into(),
        };
        assert_eq!(
            custom.reconciled_with(AiProvider::Ubi),
            AiModels::for_provider(AiProvider::Ubi)
        );
        // …and the aliases never survive a switch away from it.
        let back = AiModels::for_provider(AiProvider::Ubi).reconciled_with(AiProvider::OpenAi);
        assert_eq!(back, AiModels::for_provider(AiProvider::OpenAi));
        let mut s = Settings {
            ai_provider: AiProvider::Ubi,
            ..Settings::default()
        };
        s.sanitize();
        assert_eq!(s.models, AiModels::for_provider(AiProvider::Ubi));
    }
}
