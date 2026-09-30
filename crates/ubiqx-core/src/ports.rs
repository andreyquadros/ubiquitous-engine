//! Ports: the interfaces adapters implement. Keeping them here means the application layer
//! (`ubiqx-engine`) and the UI shells never depend on a concrete OS API, database or AI vendor.
//!
//! Persistence ports are synchronous on purpose: SQLite is synchronous and the engine wraps
//! calls in `spawn_blocking` where latency matters. AI ports are async because they do network I/O.

use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::error::CoreResult;
use crate::model::*;
use crate::update::UpdateFeed;

// ---------------------------------------------------------------------------------------------
// Platform
// ---------------------------------------------------------------------------------------------

/// Reads the foreground application/window.
pub trait ActivitySource: Send + Sync {
    /// `Ok(None)` when nothing is in the foreground (login window, screensaver, no session).
    fn foreground(&self) -> CoreResult<Option<ForegroundWindow>>;
}

/// Resolves the URL of the active tab for supported browsers.
pub trait BrowserUrlResolver: Send + Sync {
    fn supports(&self, app_id: &str) -> bool;
    fn resolve(&self, window: &ForegroundWindow) -> CoreResult<Option<String>>;
}

/// Seconds since the last user input event.
pub trait IdleDetector: Send + Sync {
    fn idle_secs(&self) -> CoreResult<Option<f64>>;
}

/// An image already downscaled and encoded by the platform layer, ready for disk or the API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodedImage {
    pub bytes: Vec<u8>,
    /// `image/webp`, `image/jpeg` or `image/png`.
    pub mime: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureTarget {
    /// Only the window with this OS id (preferred: never includes other apps' windows).
    Window(u32),
    /// The display that contains the given point, or the primary display.
    DisplayAt {
        x: i32,
        y: i32,
    },
    PrimaryDisplay,
}

/// Captures screen content.
pub trait ScreenCapturer: Send + Sync {
    /// Captures `target`, downsizes so that the longest edge is at most `max_edge` and encodes
    /// the result.
    fn capture(&self, target: CaptureTarget, max_edge: u32) -> CoreResult<EncodedImage>;

    /// Application names/ids that currently have a window on screen. The engine refuses to
    /// capture when a blocked application is visible anywhere.
    fn visible_apps(&self) -> CoreResult<Vec<String>>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionState {
    Granted,
    Denied,
    /// The platform cannot tell without prompting, or the permission does not apply here.
    Unknown,
    NotApplicable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionStatus {
    /// macOS "Screen Recording": needed for window titles and screenshots.
    pub screen_recording: PermissionState,
    /// macOS "Automation" (Apple Events): needed to read browser URLs.
    pub automation: PermissionState,
    /// macOS "Accessibility": optional, for richer window information.
    pub accessibility: PermissionState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionKind {
    ScreenRecording,
    Automation,
    Accessibility,
}

pub trait PermissionChecker: Send + Sync {
    fn status(&self) -> PermissionStatus;
    /// Triggers the OS prompt (or opens the settings pane) for the given permission.
    fn request(&self, kind: PermissionKind) -> CoreResult<()>;
}

/// Desktop notifications.
pub trait Notifier: Send + Sync {
    fn notify(&self, title: &str, body: &str) -> CoreResult<()>;
}

/// Applications installed on this machine (for the focus page's search).
pub trait AppCatalog: Send + Sync {
    /// Every installed application with a bundle identifier, sorted by name. Implementations
    /// cache the scan, so the call is cheap to repeat.
    fn installed_apps(&self) -> CoreResult<Vec<InstalledApp>>;
}

/// What the focus guard may do to a distraction. Every method answers `Ok(true)` when the
/// action took effect and `Ok(false)` when the platform could not do it (the engine then
/// falls back to the next action, or to a notification); `Err` only for unexpected failures.
pub trait Enforcer: Send + Sync {
    /// Asks the application with this bundle id to quit (graceful, may be refused).
    fn quit_app(&self, bundle_id: &str) -> CoreResult<bool>;
    /// Closes the active tab of the front window of the browser with this bundle id.
    fn close_active_tab(&self, browser_bundle_id: &str) -> CoreResult<bool>;
    /// Sends the active tab of that browser to `about:blank`.
    fn blank_active_tab(&self, browser_bundle_id: &str) -> CoreResult<bool>;
    /// Hides every other application's windows, keeping the one with this bundle id (and
    /// ubiqX itself) visible.
    fn hide_others(&self, keep_bundle_id: &str) -> CoreResult<bool>;
    /// Runs the macOS Shortcut with this name (`shortcuts run "<name>"`).
    fn run_shortcut(&self, name: &str) -> CoreResult<bool>;
}

/// Shows UBI's intervention window. Implemented by the desktop shell; `Ok(false)` means the
/// window could not be shown and the engine falls back to an OS notification.
pub trait InterventionPresenter: Send + Sync {
    fn show(&self, intervention: &Intervention) -> CoreResult<bool>;
}

/// Fetches the update feed (`latest.json`) the CI publishes next to every build.
#[async_trait]
pub trait UpdateFeedSource: Send + Sync {
    async fn fetch(&self, url: &str) -> CoreResult<UpdateFeed>;
}

/// Secret storage (macOS Keychain, etc.). Used for API keys.
pub trait SecretStore: Send + Sync {
    fn get(&self, key: &str) -> CoreResult<Option<String>>;
    fn set(&self, key: &str, value: &str) -> CoreResult<()>;
    fn delete(&self, key: &str) -> CoreResult<()>;
}

pub mod secret_keys {
    pub const ANTHROPIC_API_KEY: &str = "anthropic_api_key";
    pub const OPENAI_API_KEY: &str = "openai_api_key";
    pub const XAI_API_KEY: &str = "xai_api_key";
}

// ---------------------------------------------------------------------------------------------
// Persistence
// ---------------------------------------------------------------------------------------------

pub trait BlockRepo: Send + Sync {
    fn insert(&self, block: &ActivityBlock) -> CoreResult<()>;
    /// Full-row update. Only for callers holding a freshly read row: it overwrites every
    /// column, including classification and screenshot link. The tracker uses [`touch`].
    ///
    /// [`touch`]: BlockRepo::touch
    fn update(&self, block: &ActivityBlock) -> CoreResult<()>;
    /// Persists only the columns the segmenter owns (`ended_at`, `sample_count`, `title`,
    /// `url`, `is_open`), so a concurrent user reclassification or screenshot link is never
    /// reverted by the in-memory copy of the open block.
    fn touch(&self, block: &ActivityBlock) -> CoreResult<()>;
    /// Links a screenshot to a block without touching any other column.
    fn set_screenshot(&self, id: &str, screenshot_id: &str) -> CoreResult<()>;
    fn get(&self, id: &str) -> CoreResult<Option<ActivityBlock>>;
    /// The block currently being extended by the sampler, if any.
    fn open_block(&self) -> CoreResult<Option<ActivityBlock>>;
    /// Closed blocks overlapping `range`, ordered by `started_at`.
    fn list_in_range(&self, range: TimeRange) -> CoreResult<Vec<ActivityBlock>>;
    /// Closed blocks without a category that a remote classifier may try now
    /// (`needs_review = false`, `next_attempt_at` unset or in the past), oldest first.
    fn list_pending_remote(
        &self,
        now: DateTime<Utc>,
        limit: usize,
    ) -> CoreResult<Vec<ActivityBlock>>;
    /// Closed blocks without a category, regardless of retry state, oldest first.
    fn list_unclassified(&self, limit: usize) -> CoreResult<Vec<ActivityBlock>>;
    /// Blocks flagged for manual review, newest first.
    fn list_needs_review(&self, limit: usize) -> CoreResult<Vec<ActivityBlock>>;
    /// Records a failed remote attempt and when the block may be retried.
    fn record_attempt(
        &self,
        id: &str,
        attempts: u32,
        next_attempt_at: Option<DateTime<Utc>>,
        needs_review: bool,
    ) -> CoreResult<()>;
    /// Stores what was sent to the AI for this block (redacted text) and when.
    fn set_ai_payload(&self, id: &str, payload: &str, at: DateTime<Utc>) -> CoreResult<()>;
    /// Splits a block at `at` into two blocks; returns the id of the new (second) block.
    fn split(&self, id: &str, at: DateTime<Utc>) -> CoreResult<Id>;
    /// Reassigns every non-user-classified block matching the key (app id and, when given,
    /// domain) inside `range` to `category_id`. Returns the number of blocks changed.
    fn backfill_category(
        &self,
        app_id: &str,
        domain: Option<&str>,
        range: TimeRange,
        category_id: &str,
        source: ClassificationSource,
    ) -> CoreResult<u64>;
    fn delete(&self, id: &str) -> CoreResult<()>;
    fn set_classification(
        &self,
        id: &str,
        category_id: Option<&str>,
        confidence: f32,
        source: ClassificationSource,
        description: Option<&str>,
    ) -> CoreResult<()>;
    fn totals_by_category(&self, range: TimeRange) -> CoreResult<Vec<CategoryTotal>>;
    fn totals_by_app(&self, range: TimeRange, limit: usize) -> CoreResult<Vec<AppTotal>>;
    /// Blocks classified by the user (for the memory classifier), newest first.
    fn list_user_classified(&self, limit: usize) -> CoreResult<Vec<ActivityBlock>>;
    fn delete_before(&self, before: DateTime<Utc>) -> CoreResult<u64>;
}

pub trait CategoryRepo: Send + Sync {
    fn list(&self, include_archived: bool) -> CoreResult<Vec<Category>>;
    fn get(&self, id: &str) -> CoreResult<Option<Category>>;
    fn upsert(&self, category: &Category) -> CoreResult<()>;
    fn delete(&self, id: &str) -> CoreResult<()>;
}

pub trait RuleRepo: Send + Sync {
    fn list(&self) -> CoreResult<Vec<Rule>>;
    fn upsert(&self, rule: &Rule) -> CoreResult<()>;
    fn delete(&self, id: &str) -> CoreResult<()>;
    fn increment_hits(&self, id: &str) -> CoreResult<()>;
    fn record_miss(&self, id: &str, at: DateTime<Utc>) -> CoreResult<()>;
}

pub trait CorrectionRepo: Send + Sync {
    fn insert(&self, correction: &Correction) -> CoreResult<()>;
    fn list_recent(&self, limit: usize) -> CoreResult<Vec<Correction>>;
    fn count(&self) -> CoreResult<u64>;
}

pub trait ScreenshotRepo: Send + Sync {
    fn insert(&self, shot: &Screenshot) -> CoreResult<()>;
    fn get(&self, id: &str) -> CoreResult<Option<Screenshot>>;
    /// Newest screenshot taken while `block_id` was active.
    fn latest_for_block(&self, block_id: &str) -> CoreResult<Option<Screenshot>>;
    fn attach_to_block(&self, id: &str, block_id: &str) -> CoreResult<()>;
    fn mark_sent(&self, id: &str) -> CoreResult<()>;
    /// Count of screenshots sent to the AI since `since` (for the per-hour budget).
    fn count_sent_since(&self, since: DateTime<Utc>) -> CoreResult<u64>;
    /// Returns the deleted rows so the caller can remove files from disk.
    fn delete_before(&self, before: DateTime<Utc>) -> CoreResult<Vec<Screenshot>>;
}

pub trait ReportRepo: Send + Sync {
    fn upsert(&self, report: &DailyReport) -> CoreResult<()>;
    fn get(&self, date: NaiveDate, category_id: &str) -> CoreResult<Option<DailyReport>>;
    fn list_for_date(&self, date: NaiveDate) -> CoreResult<Vec<DailyReport>>;
    fn list_between(&self, from: NaiveDate, to: NaiveDate) -> CoreResult<Vec<DailyReport>>;
    /// Flags reports of that day as stale (blocks changed after generation).
    fn mark_stale(&self, date: NaiveDate) -> CoreResult<()>;
    fn delete(&self, id: &str) -> CoreResult<()>;
}

pub trait NudgeRepo: Send + Sync {
    fn insert(&self, nudge: &Nudge) -> CoreResult<()>;
    fn list_recent(&self, limit: usize) -> CoreResult<Vec<Nudge>>;
    fn mark_seen(&self, id: &str) -> CoreResult<()>;
    fn mark_all_seen(&self) -> CoreResult<()>;
    fn last_of_kind(&self, kind: NudgeKind) -> CoreResult<Option<DateTime<Utc>>>;
    /// Number of policy nudges emitted since `since` (for the daily cap). `ReportReady` and
    /// `Attention` nudges are exempt from the cap and are not counted.
    fn count_since(&self, since: DateTime<Utc>) -> CoreResult<u64>;
}

/// Focus guard persistence: blocked targets, interventions and focus sessions.
pub trait FocusRepo: Send + Sync {
    /// Every target, enabled first, then by name.
    fn list_targets(&self) -> CoreResult<Vec<FocusTarget>>;
    fn get_target(&self, id: &str) -> CoreResult<Option<FocusTarget>>;
    /// Inserts the target, or, when one with the same `(kind, key)` exists, re-enables that
    /// one and returns it (its id, name and counters are kept).
    fn upsert_target(&self, target: &FocusTarget) -> CoreResult<FocusTarget>;
    fn set_target_enabled(&self, id: &str, enabled: bool) -> CoreResult<FocusTarget>;
    /// Removes the target; its interventions stay as history.
    fn remove_target(&self, id: &str) -> CoreResult<()>;
    /// Bumps `blocked_count` and sets `last_blocked_at`.
    fn touch_blocked(&self, id: &str, at: DateTime<Utc>) -> CoreResult<()>;

    fn insert_intervention(&self, intervention: &Intervention) -> CoreResult<()>;
    fn get_intervention(&self, id: &str) -> CoreResult<Option<Intervention>>;
    /// Newest first.
    fn list_interventions(&self, limit: usize) -> CoreResult<Vec<Intervention>>;
    fn count_interventions_since(&self, since: DateTime<Utc>) -> CoreResult<u64>;

    fn insert_session(&self, session: &FocusSession) -> CoreResult<()>;
    /// Full-row update of an existing session.
    fn update_session(&self, session: &FocusSession) -> CoreResult<()>;
    /// The session without `ended_at`, newest when several were left behind.
    fn active_session(&self) -> CoreResult<Option<FocusSession>>;
    /// Newest first.
    fn list_sessions(&self, limit: usize) -> CoreResult<Vec<FocusSession>>;
}

/// Small key/value store for engine state that must survive restarts
/// (e.g. `last_report_check`).
pub trait KvRepo: Send + Sync {
    fn get(&self, key: &str) -> CoreResult<Option<String>>;
    fn set(&self, key: &str, value: &str) -> CoreResult<()>;
}

/// Bulk maintenance of the store ("Apagar todos os dados").
pub trait MaintenanceRepo: Send + Sync {
    /// Removes every activity-derived row in one transaction: blocks (open ones included),
    /// screenshots, corrections, reports, nudges, AI usage, the key/value cache, learned
    /// rules, interventions and focus sessions. Categories, user-authored rules, settings
    /// and the focus targets are kept.
    fn wipe_user_data(&self) -> CoreResult<()>;
}

/// Outbound event channel from the engine to whichever shell hosts it.
pub trait EventSink: Send + Sync {
    fn emit(&self, event: EngineEvent);
}

/// Event sink that drops everything (tests, CLI in quiet mode).
#[derive(Debug, Default, Clone, Copy)]
pub struct NullEventSink;

impl EventSink for NullEventSink {
    fn emit(&self, _event: EngineEvent) {}
}

pub trait SettingsRepo: Send + Sync {
    fn load(&self) -> CoreResult<Settings>;
    fn save(&self, settings: &Settings) -> CoreResult<()>;
}

/// What an AI call was for. Used for cost reporting in the UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AiUsageKind {
    Classify,
    Vision,
    Report,
    Advice,
}

impl AiUsageKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            AiUsageKind::Classify => "classify",
            AiUsageKind::Vision => "vision",
            AiUsageKind::Report => "report",
            AiUsageKind::Advice => "advice",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "classify" => Some(Self::Classify),
            "vision" => Some(Self::Vision),
            "report" => Some(Self::Report),
            "advice" => Some(Self::Advice),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AiUsage {
    pub at: DateTime<Utc>,
    pub kind: AiUsageKind,
    pub model: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_read_tokens: u32,
    pub cache_write_tokens: u32,
    /// Estimated cost in USD computed from the model price table at call time.
    pub cost_usd: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AiUsageTotals {
    pub calls: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_usd: f64,
}

pub trait UsageRepo: Send + Sync {
    fn record(&self, usage: &AiUsage) -> CoreResult<()>;
    fn totals(&self, range: TimeRange) -> CoreResult<AiUsageTotals>;
}

// ---------------------------------------------------------------------------------------------
// AI
// ---------------------------------------------------------------------------------------------

/// A previously classified block used as a few-shot example for the LLM.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClassificationExample {
    /// Bundle id (macOS) / executable id: drives redaction exactly like a block does.
    pub app_id: String,
    pub app_name: String,
    pub title: String,
    pub domain: Option<String>,
    pub category_id: Id,
}

/// Everything a classifier needs besides the blocks themselves.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClassificationContext {
    pub categories: Vec<Category>,
    pub rules: Vec<Rule>,
    /// Few-shot examples derived from corrections, most relevant first.
    pub examples: Vec<ClassificationExample>,
    /// Blocks the user classified by hand (for the memory classifier).
    pub user_classified: Vec<ActivityBlock>,
    pub language: String,
    pub min_confidence: f32,
    /// Model ids to use for this request (from settings).
    pub models: AiModels,
    /// Free text about the user, so the model understands ambiguous titles.
    pub user_profile: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Classification {
    pub block_id: Id,
    /// `None` means "I don't know" — the next classifier in the chain gets a go.
    pub category_id: Option<Id>,
    pub confidence: f32,
    pub source: ClassificationSource,
    pub description: Option<String>,
    /// The classifier is sure the block needs a screenshot to be understood.
    pub needs_vision: bool,
    /// The rule that produced this classification (so contradictions can be attributed).
    pub rule_id: Option<Id>,
}

impl Classification {
    pub fn unknown(block_id: &str, source: ClassificationSource) -> Self {
        Self {
            block_id: block_id.to_string(),
            category_id: None,
            confidence: 0.0,
            source,
            description: None,
            needs_vision: false,
            rule_id: None,
        }
    }

    pub fn is_confident(&self, min_confidence: f32) -> bool {
        self.category_id.is_some() && self.confidence >= min_confidence
    }
}

/// A cheap, synchronous, infallible classifier (rules, memory). Runs on every block before
/// anything is sent to a remote model.
pub trait LocalClassifier: Send + Sync {
    fn name(&self) -> &'static str;
    /// `None` means "I don't know".
    fn classify(
        &self,
        block: &ActivityBlock,
        ctx: &ClassificationContext,
    ) -> Option<Classification>;
}

/// A remote (paid, fallible, batched) classifier.
#[async_trait]
pub trait RemoteClassifier: Send + Sync {
    fn name(&self) -> &'static str;

    /// The exact (redacted) text this classifier will send for `block`. The engine stores it
    /// so the user can audit what left the machine. The default mirrors `redact_block`.
    fn describe_payload(&self, block: &ActivityBlock) -> String {
        let r = crate::redact::redact_block(
            &block.app_id,
            &block.app_name,
            &block.title,
            block.url.as_deref(),
            block.domain.as_deref(),
        );
        format!(
            "app={} | title={}{}",
            r.app_name,
            r.title,
            r.domain
                .map(|d| format!(" | domain={d}"))
                .unwrap_or_default()
        )
    }

    /// Returns classifications keyed by block id. Blocks missing from the result are treated
    /// as unclassified by the caller; ids not in `blocks` are ignored.
    async fn classify_batch(
        &self,
        blocks: &[ActivityBlock],
        ctx: &ClassificationContext,
    ) -> CoreResult<Vec<Classification>>;
}

/// Classifies a single block using a screenshot in addition to its metadata.
#[async_trait]
pub trait VisionClassifier: Send + Sync {
    async fn classify_with_image(
        &self,
        block: &ActivityBlock,
        image: &EncodedImage,
        ctx: &ClassificationContext,
    ) -> CoreResult<Classification>;
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReportRequest {
    pub date: NaiveDate,
    pub category: Category,
    /// Closed blocks of the day already filtered to this category, ordered by time.
    pub blocks: Vec<ActivityBlock>,
    /// Items from the previous few days of the same category, so the writer can say
    /// "continuou X" instead of inventing a new start.
    pub previous_items: Vec<ReportItem>,
    pub language: String,
    /// Local UTC offset in seconds for rendering times.
    pub utc_offset_secs: i32,
    /// Optional free text the user wants the writer to know (role, institution, expectations).
    pub user_profile: Option<String>,
    pub model: String,
}

#[async_trait]
pub trait ReportWriter: Send + Sync {
    async fn write_daily(&self, req: &ReportRequest) -> CoreResult<DailyReport>;

    /// The exact (redacted) line this writer will send for `block` in a daily report, so the
    /// engine can record it in the "data sent to AI" audit trail. `utc_offset_secs` is the
    /// user's local offset used to print times. The default mirrors `redact_block`.
    fn describe_payload(&self, block: &ActivityBlock, utc_offset_secs: i32) -> String {
        let r = crate::redact::redact_block(
            &block.app_id,
            &block.app_name,
            &block.title,
            block.url.as_deref(),
            block.domain.as_deref(),
        );
        let start = block.started_at + chrono::Duration::seconds(utc_offset_secs as i64);
        let mut s = format!("{} app={}", start.format("%H:%M"), r.app_name);
        if !r.title.is_empty() {
            s.push_str(&format!(" title={}", r.title));
        }
        if let Some(d) = r.domain.as_deref() {
            s.push_str(&format!(" domain={d}"));
        }
        s
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdviceRequest {
    pub language: String,
    pub stats: FocusStats,
    pub category_totals: Vec<(String, i64)>,
    pub top_apps: Vec<AppTotal>,
    pub recent_nudges: Vec<NudgeKind>,
    pub user_profile: Option<String>,
    pub model: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Advice {
    pub headline: String,
    pub recommendations: Vec<String>,
    pub model: String,
}

/// Produces natural-language productivity recommendations from local statistics.
#[async_trait]
pub trait Advisor: Send + Sync {
    async fn advise(&self, req: &AdviceRequest) -> CoreResult<Advice>;
}
