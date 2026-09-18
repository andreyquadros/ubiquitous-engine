//! Everything the engine needs, as trait objects. Built by the composition root.

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use ubiqx_core::license::{UBIQX_API_BASE_DEFAULT, UBIQX_LICENSE_PUBKEY_HEX};
use ubiqx_core::ports::*;
use ubiqx_core::{BuildInfo, Clock, CoreError, CoreResult, Intervention, ManagedUsage};

/// [`InterventionPresenter`] that only logs: the CLI, tests and any shell without a window.
/// It answers `false`, so the engine falls back to an OS notification.
#[derive(Debug, Default, Clone, Copy)]
pub struct LogInterventionPresenter;

impl InterventionPresenter for LogInterventionPresenter {
    fn show(&self, intervention: &Intervention) -> CoreResult<bool> {
        tracing::info!(
            target: "ubiqx::focus",
            name = %intervention.name,
            action = intervention.action.as_str(),
            message = %intervention.message,
            "intervention"
        );
        Ok(false)
    }
}

#[derive(Clone)]
pub struct PlatformPorts {
    pub activity: Arc<dyn ActivitySource>,
    pub urls: Arc<dyn BrowserUrlResolver>,
    pub idle: Arc<dyn IdleDetector>,
    pub capturer: Arc<dyn ScreenCapturer>,
    pub permissions: Arc<dyn PermissionChecker>,
    pub secrets: Arc<dyn SecretStore>,
    pub notifier: Arc<dyn Notifier>,
    /// Where `latest.json` comes from (HTTP in production, a static feed in tests).
    pub update_feed: Arc<dyn UpdateFeedSource>,
    /// Installed applications, for the focus page's search.
    pub apps: Arc<dyn AppCatalog>,
    /// What the focus guard may do to a distraction.
    pub enforcer: Arc<dyn Enforcer>,
    /// Shows the intervention window ([`LogInterventionPresenter`] without a shell).
    pub presenter: Arc<dyn InterventionPresenter>,
}

#[derive(Clone)]
pub struct Repos {
    pub blocks: Arc<dyn BlockRepo>,
    pub categories: Arc<dyn CategoryRepo>,
    pub rules: Arc<dyn RuleRepo>,
    pub corrections: Arc<dyn CorrectionRepo>,
    pub screenshots: Arc<dyn ScreenshotRepo>,
    pub reports: Arc<dyn ReportRepo>,
    pub nudges: Arc<dyn NudgeRepo>,
    pub settings: Arc<dyn SettingsRepo>,
    pub usage: Arc<dyn UsageRepo>,
    pub kv: Arc<dyn KvRepo>,
    pub maintenance: Arc<dyn MaintenanceRepo>,
    pub focus: Arc<dyn FocusRepo>,
}

impl Repos {
    /// Builds every repository from one object implementing all persistence ports.
    pub fn from_store<S>(store: Arc<S>) -> Self
    where
        S: BlockRepo
            + CategoryRepo
            + RuleRepo
            + CorrectionRepo
            + ScreenshotRepo
            + ReportRepo
            + NudgeRepo
            + SettingsRepo
            + UsageRepo
            + KvRepo
            + MaintenanceRepo
            + FocusRepo
            + 'static,
    {
        Self {
            blocks: store.clone(),
            categories: store.clone(),
            rules: store.clone(),
            corrections: store.clone(),
            screenshots: store.clone(),
            reports: store.clone(),
            nudges: store.clone(),
            settings: store.clone(),
            usage: store.clone(),
            kv: store.clone(),
            maintenance: store.clone(),
            focus: store,
        }
    }
}

/// AI-backed ports. All optional: without them the engine runs in local mode
/// (rules + memory + template reports).
#[derive(Clone)]
pub struct AiPorts {
    pub remote: Option<Arc<dyn RemoteClassifier>>,
    pub vision: Option<Arc<dyn VisionClassifier>>,
    pub report_writer: Option<Arc<dyn ReportWriter>>,
    pub advisor: Option<Arc<dyn Advisor>>,
    /// Whether the remote components need an API key in the secret store (false for fakes).
    pub requires_api_key: bool,
}

impl Default for AiPorts {
    fn default() -> Self {
        Self {
            remote: None,
            vision: None,
            report_writer: None,
            advisor: None,
            requires_api_key: true,
        }
    }
}

/// The Ubi proxy's license endpoint, as far as the engine needs it: the month's usage of a
/// `monthly_managed` subscription. Implemented over HTTP by the composition root; tests and
/// the CLI use [`NoLicenseServer`].
#[async_trait]
pub trait LicenseServer: Send + Sync {
    /// `GET /v1/license/status` with `key`. Any failure (offline, rejected key, outage) is an
    /// error: the engine then shows the local verdict without usage numbers.
    async fn managed_usage(&self, key: &str) -> CoreResult<ManagedUsage>;
}

/// A [`LicenseServer`] that is never reachable.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoLicenseServer;

#[async_trait]
impl LicenseServer for NoLicenseServer {
    async fn managed_usage(&self, _key: &str) -> CoreResult<ManagedUsage> {
        Err(CoreError::Ai("no license server configured".into()))
    }
}

/// Licensing inputs: the public key every stored license key is verified against, the
/// proxy's base URL (shown in Settings, used by the managed provider's client) and the
/// license endpoint. `Default` is the production key and proxy without a server.
#[derive(Clone)]
pub struct LicenseDeps {
    /// Hex Ed25519 public key ([`UBIQX_LICENSE_PUBKEY_HEX`] in production; tests sign with
    /// their own pair).
    pub pubkey_hex: String,
    /// The Ubi proxy's base URL (`UBIQX_API_BASE`).
    pub api_base: String,
    pub server: Arc<dyn LicenseServer>,
}

impl Default for LicenseDeps {
    fn default() -> Self {
        Self {
            pubkey_hex: UBIQX_LICENSE_PUBKEY_HEX.to_string(),
            api_base: UBIQX_API_BASE_DEFAULT.to_string(),
            server: Arc::new(NoLicenseServer),
        }
    }
}

impl std::fmt::Debug for LicenseDeps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LicenseDeps")
            .field("pubkey_hex", &self.pubkey_hex)
            .field("api_base", &self.api_base)
            .finish_non_exhaustive()
    }
}

#[derive(Clone)]
pub struct EngineDeps {
    pub platform: PlatformPorts,
    pub repos: Repos,
    pub ai: AiPorts,
    pub sink: Arc<dyn EventSink>,
    pub clock: Arc<dyn Clock>,
    /// Directory for screenshots and other engine-owned files.
    pub data_dir: PathBuf,
    /// Identity of the running build (stamped by the shell; [`BuildInfo::dev`] otherwise).
    pub build: BuildInfo,
    /// URL of the update feed (`latest.json`) checked against `build`.
    pub update_feed_url: String,
    /// License verification key, proxy URL and endpoint (see [`LicenseDeps`]).
    pub license: LicenseDeps,
}
