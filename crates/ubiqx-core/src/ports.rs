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

/// Captures the primary display.
pub trait ScreenCapturer: Send + Sync {
    /// Captures, downsizes so that the longest edge is `max_edge` and encodes the result.
    fn capture(&self, max_edge: u32) -> CoreResult<EncodedImage>;
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

/// Secret storage (macOS Keychain, etc.). Used for API keys.
pub trait SecretStore: Send + Sync {
    fn get(&self, key: &str) -> CoreResult<Option<String>>;
    fn set(&self, key: &str, value: &str) -> CoreResult<()>;
    fn delete(&self, key: &str) -> CoreResult<()>;
}

pub mod secret_keys {
    pub const ANTHROPIC_API_KEY: &str = "anthropic_api_key";
}

// ---------------------------------------------------------------------------------------------
// Persistence
// ---------------------------------------------------------------------------------------------

pub trait BlockRepo: Send + Sync {
    fn insert(&self, block: &ActivityBlock) -> CoreResult<()>;
    fn update(&self, block: &ActivityBlock) -> CoreResult<()>;
    fn get(&self, id: &str) -> CoreResult<Option<ActivityBlock>>;
    /// The block currently being extended by the sampler, if any.
    fn open_block(&self) -> CoreResult<Option<ActivityBlock>>;
    /// Closed blocks overlapping `range`, ordered by `started_at`.
    fn list_in_range(&self, range: TimeRange) -> CoreResult<Vec<ActivityBlock>>;
    /// Closed blocks without a category, oldest first.
    fn list_unclassified(&self, limit: usize) -> CoreResult<Vec<ActivityBlock>>;
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
    fn delete(&self, id: &str) -> CoreResult<()>;
}

pub trait NudgeRepo: Send + Sync {
    fn insert(&self, nudge: &Nudge) -> CoreResult<()>;
    fn list_recent(&self, limit: usize) -> CoreResult<Vec<Nudge>>;
    fn mark_seen(&self, id: &str) -> CoreResult<()>;
    fn mark_all_seen(&self) -> CoreResult<()>;
    fn last_of_kind(&self, kind: NudgeKind) -> CoreResult<Option<DateTime<Utc>>>;
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
        }
    }

    pub fn is_confident(&self, min_confidence: f32) -> bool {
        self.category_id.is_some() && self.confidence >= min_confidence
    }
}

/// A classifier in the chain. Implementations must be cheap to call with an empty slice.
#[async_trait]
pub trait Classifier: Send + Sync {
    fn name(&self) -> &'static str;

    /// Returns one classification per input block, in the same order. Blocks the classifier
    /// cannot decide are returned with `category_id = None`.
    async fn classify(
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
    /// Descriptions of what the user did, gathered from vision/LLM, keyed by block id.
    pub language: String,
    /// Local UTC offset in seconds for rendering times.
    pub utc_offset_secs: i32,
    /// Optional free text the user wants the writer to know (role, institution, expectations).
    pub user_profile: Option<String>,
}

#[async_trait]
pub trait ReportWriter: Send + Sync {
    async fn write_daily(&self, req: &ReportRequest) -> CoreResult<DailyReport>;
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdviceRequest {
    pub language: String,
    pub stats: FocusStats,
    pub category_totals: Vec<(String, i64)>,
    pub top_apps: Vec<AppTotal>,
    pub recent_nudges: Vec<NudgeKind>,
    pub user_profile: Option<String>,
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

// ---------------------------------------------------------------------------------------------
// Generic LLM client (vendor-agnostic request/response so the AI crate can be swapped)
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LlmContent {
    Text { text: String },
    Image { mime: String, base64: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LlmRole {
    User,
    Assistant,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LlmMessage {
    pub role: LlmRole,
    pub content: Vec<LlmContent>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LlmRequest {
    pub model: String,
    /// Stable instructions; providers that support prompt caching cache this part.
    pub system: String,
    pub messages: Vec<LlmMessage>,
    pub max_tokens: u32,
    /// When set, the provider is asked to constrain the answer to this JSON schema.
    pub json_schema: Option<serde_json::Value>,
    pub usage_kind: AiUsageKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LlmResponse {
    pub text: String,
    pub model: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_read_tokens: u32,
    pub cache_write_tokens: u32,
    pub stop_reason: String,
}

#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn complete(&self, req: &LlmRequest) -> CoreResult<LlmResponse>;
    /// Cheap connectivity/credential check used by the settings screen.
    async fn ping(&self) -> CoreResult<()>;
}
