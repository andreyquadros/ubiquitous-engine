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
}

impl ActivityBlock {
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
}

// ---------------------------------------------------------------------------------------------
// Reports and insights
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReportItem {
    /// Local time range, e.g. "09:10–10:25".
    pub time_range: String,
    pub activity: String,
    pub duration_secs: i64,
    pub apps: Vec<String>,
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
        Self {
            classify: "claude-haiku-4-5".into(),
            vision: "claude-haiku-4-5".into(),
            report: "claude-sonnet-5".into(),
        }
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
    /// Days to keep screenshots on disk.
    pub screenshot_retention_days: u32,
    /// Application ids / names that are never sampled nor captured.
    pub blocked_apps: Vec<String>,
    /// Web domains that are never recorded (the block keeps app name only).
    pub blocked_domains: Vec<String>,
    /// Pauses sampling and screenshots without stopping the app.
    pub private_mode: bool,
    pub models: AiModels,
    /// Maximum screenshots sent to the vision model per hour.
    pub max_vision_per_hour: u32,
    /// Blocks with confidence below this go to the next classifier in the chain.
    pub min_confidence: f32,
    /// Default local time for daily reports when a category has none.
    pub report_default_time: NaiveTime,
    /// Language for AI-generated text (BCP-47).
    pub language: String,
    pub quiet_hours: QuietHours,
    pub nudges_enabled: bool,
    pub launch_at_login: bool,
    /// Onboarding finished (permissions granted, key stored, categories created).
    pub onboarding_done: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            tracking_enabled: true,
            sample_interval_secs: 5,
            idle_threshold_secs: 180,
            min_block_secs: 20,
            screenshot_interval_secs: 120,
            screenshot_max_edge: 1024,
            screenshot_retention_days: 7,
            blocked_apps: vec![
                "com.1password.1password".into(),
                "com.agilebits.onepassword7".into(),
                "com.apple.keychainaccess".into(),
                "com.bitwarden.desktop".into(),
            ],
            blocked_domains: vec![],
            private_mode: false,
            models: AiModels::default(),
            max_vision_per_hour: 20,
            min_confidence: 0.6,
            report_default_time: NaiveTime::from_hms_opt(18, 0, 0).expect("valid"),
            language: "pt-BR".into(),
            quiet_hours: QuietHours::default(),
            nudges_enabled: true,
            launch_at_login: true,
            onboarding_done: false,
        }
    }
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
        let q2 = QuietHours { enabled: false, ..q };
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
        ] {
            assert_eq!(NudgeKind::parse(k.as_str()), Some(k));
        }
    }

    #[test]
    fn settings_default_serialises_with_missing_fields() {
        let s: Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(s, Settings::default());
    }
}
