//! Everything the engine needs, as trait objects. Built by the composition root.

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use ubiqx_core::license::{UBIQX_API_BASE_DEFAULT, UBIQX_LICENSE_PUBKEY_HEX};
use ubiqx_core::nofap::{
    ApplyOutcome, DayCount, DayKind, ExplicitDetector, Frame, FrameOpts, FrameSource, NofapCard,
    NofapConfig, NofapEvent, NofapPresenter, NofapRepo, NofapSink, NofapSite, PlannedChange,
    RemoveScope, SiteOrigin, SystemCapabilities, SystemObservation, SystemPlan, SystemShield,
};
use ubiqx_core::ports::*;
use ubiqx_core::{
    BuildInfo, Clock, CoreError, CoreResult, Intervention, ManagedUsage, ProductId, ProductIdentity,
};

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

// ---------------------------------------------------------------------------------------------
// The `nofap` module
// ---------------------------------------------------------------------------------------------

/// The ports of the `nofap` module that a composition root supplies. The module's service
/// (`crate::nofap`) is built from these plus ports the engine already has (activity, URLs,
/// the enforcer, permissions, the notifier, the usage ledger, the clock), so ubiqX's
/// engine and the standalone app host the same service.
#[derive(Clone)]
pub struct NofapPorts {
    /// L2 to L4: the hosts block, the family resolver and the browser policies.
    pub system: Arc<dyn SystemShield>,
    /// One reduced image of one window, for the AI check.
    pub frames: Arc<dyn FrameSource>,
    /// The AI check itself; `None` when no provider is wired (vision is then off with
    /// `no_provider`).
    pub detector: Option<Arc<dyn ExplicitDetector>>,
    /// Shows the card window ([`LogNofapPresenter`] without a shell).
    pub presenter: Arc<dyn NofapPresenter>,
    /// The module's own event channel.
    pub sink: Arc<dyn NofapSink>,
    pub repo: Arc<dyn NofapRepo>,
    /// Directory both products share for the primary lock
    /// (`<data_local_dir>/ubiqx-shield`). `None`: no lock is read or written and this
    /// process always runs the passes (tests, the CLI).
    pub shared_dir: Option<PathBuf>,
}

impl NofapPorts {
    /// Ports that do nothing: no system layer, no frames, no detector, a presenter that
    /// only logs, a sink that drops everything and an in-memory repository. The module
    /// stays off with them unless a test enables it, so the CLI, the scripted demo and
    /// every existing test are unchanged.
    pub fn inert() -> Self {
        Self {
            system: Arc::new(NoSystemShield),
            frames: Arc::new(NoFrameSource),
            detector: None,
            presenter: Arc::new(LogNofapPresenter),
            sink: Arc::new(NullNofapSink),
            repo: Arc::new(MemoryNofapRepo::default()),
            shared_dir: None,
        }
    }
}

/// Reason code the inert system layer reports for every tier.
pub const NO_SYSTEM_LAYER: &str = "no_system_layer";

/// [`SystemShield`] of a host without a system layer: every tier is unsupported, nothing
/// is observed and nothing is ever applied.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoSystemShield;

impl SystemShield for NoSystemShield {
    fn capabilities(&self) -> SystemCapabilities {
        SystemCapabilities::none(NO_SYSTEM_LAYER)
    }

    fn observe(&self) -> CoreResult<SystemObservation> {
        Ok(SystemObservation::default())
    }

    fn preview(&self, _plan: &SystemPlan) -> CoreResult<Vec<PlannedChange>> {
        Ok(Vec::new())
    }

    fn apply(&self, _plan: &SystemPlan) -> CoreResult<ApplyOutcome> {
        Ok(ApplyOutcome::Unavailable(NO_SYSTEM_LAYER.to_string()))
    }

    fn remove(&self, _owner: ProductId, _scope: RemoveScope) -> CoreResult<ApplyOutcome> {
        Ok(ApplyOutcome::Unavailable(NO_SYSTEM_LAYER.to_string()))
    }
}

/// [`FrameSource`] that never has a frame.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoFrameSource;

impl FrameSource for NoFrameSource {
    fn frame(&self, _window_id: u32, _opts: FrameOpts) -> CoreResult<Option<Frame>> {
        Ok(None)
    }
}

/// [`NofapPresenter`] that only logs that a card exists (its kind, never what caused it).
/// It answers `false`, so the service falls back to one neutral OS notification.
#[derive(Debug, Default, Clone, Copy)]
pub struct LogNofapPresenter;

impl NofapPresenter for LogNofapPresenter {
    fn show(&self, card: &NofapCard) -> CoreResult<bool> {
        tracing::debug!(target: "ubiqx::shield", kind = ?card.kind, "card");
        Ok(false)
    }
}

/// [`NofapSink`] that drops everything.
#[derive(Debug, Default, Clone, Copy)]
pub struct NullNofapSink;

impl NofapSink for NullNofapSink {
    fn emit(&self, _event: NofapEvent) {}
}

/// [`NofapRepo`] kept in memory: the CLI, the scripted demo and tests that do not care
/// about persistence. Same semantics as the SQLite repository.
#[derive(Debug, Default)]
pub struct MemoryNofapRepo {
    inner: Mutex<MemoryNofap>,
}

#[derive(Debug, Default)]
struct MemoryNofap {
    config: Option<NofapConfig>,
    sites: Vec<NofapSite>,
    days: Vec<DayCount>,
}

impl NofapRepo for MemoryNofapRepo {
    fn load_config(&self) -> CoreResult<NofapConfig> {
        Ok(self.inner.lock().config.clone().unwrap_or_default())
    }

    fn save_config(&self, config: &NofapConfig) -> CoreResult<()> {
        self.inner.lock().config = Some(config.clone());
        Ok(())
    }

    fn list_sites(&self) -> CoreResult<Vec<NofapSite>> {
        Ok(self.inner.lock().sites.clone())
    }

    fn upsert_site(&self, site: &NofapSite) -> CoreResult<()> {
        let mut inner = self.inner.lock();
        inner
            .sites
            .retain(|s| s.id != site.id && !(s.list == site.list && s.host_key == site.host_key));
        inner.sites.push(site.clone());
        Ok(())
    }

    fn delete_site(&self, id: &str) -> CoreResult<()> {
        self.inner.lock().sites.retain(|s| s.id != id);
        Ok(())
    }

    fn delete_sites_by_origin(&self, origin: SiteOrigin) -> CoreResult<u64> {
        let mut inner = self.inner.lock();
        let before = inner.sites.len();
        inner.sites.retain(|s| s.origin != origin);
        Ok((before - inner.sites.len()) as u64)
    }

    fn purge_expired_sites(&self, now: DateTime<Utc>) -> CoreResult<u64> {
        let mut inner = self.inner.lock();
        let before = inner.sites.len();
        inner
            .sites
            .retain(|s| s.expires_at.is_none_or(|at| at > now));
        Ok((before - inner.sites.len()) as u64)
    }

    fn bump_day(&self, day: &str, hour: u8, kind: DayKind, by: u32) -> CoreResult<()> {
        let mut inner = self.inner.lock();
        match inner
            .days
            .iter_mut()
            .find(|c| c.day == day && c.hour == hour && c.kind == kind)
        {
            Some(count) => count.n = count.n.saturating_add(by),
            None => inner.days.push(DayCount {
                day: day.to_string(),
                hour,
                kind,
                n: by,
            }),
        }
        Ok(())
    }

    fn days_between(&self, from: &str, to: &str) -> CoreResult<Vec<DayCount>> {
        let mut out: Vec<DayCount> = self
            .inner
            .lock()
            .days
            .iter()
            .filter(|c| c.day.as_str() >= from && c.day.as_str() <= to)
            .cloned()
            .collect();
        out.sort_by(|a, b| {
            (&a.day, a.hour, a.kind.as_str()).cmp(&(&b.day, b.hour, b.kind.as_str()))
        });
        Ok(out)
    }

    fn delete_days_before(&self, day: &str) -> CoreResult<u64> {
        let mut inner = self.inner.lock();
        let before = inner.days.len();
        inner.days.retain(|c| c.day.as_str() >= day);
        Ok((before - inner.days.len()) as u64)
    }

    fn erase_history(&self) -> CoreResult<()> {
        let mut inner = self.inner.lock();
        inner.days.clear();
        inner.sites.retain(|s| s.origin != SiteOrigin::Snooze);
        Ok(())
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
    /// The ports of the `nofap` module ([`NofapPorts::inert`] where it is not wired).
    pub nofap: NofapPorts,
    /// Which product this process is (`ubiqx_core::product::UBIQX` for ubiqX): its update
    /// feed, and the identity the guards exempt and the system layer signs its wants with.
    pub identity: &'static ProductIdentity,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use ubiqx_core::nofap::{SiteList, TierCapability};

    fn at(h: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 9, 17, h, 0, 0).unwrap()
    }

    fn site(id: &str, origin: SiteOrigin, key: u8, expires_at: Option<DateTime<Utc>>) -> NofapSite {
        NofapSite {
            id: id.into(),
            list: if origin == SiteOrigin::Snooze {
                SiteList::Allow
            } else {
                SiteList::Block
            },
            origin,
            host: (origin == SiteOrigin::User).then(|| "example.com".to_string()),
            host_key: [key; 16],
            created_at: at(9),
            expires_at,
        }
    }

    #[test]
    fn nofap_inert_ports_apply_nothing() {
        let ports = NofapPorts::inert();
        let caps = ports.system.capabilities();
        for tier in ubiqx_core::nofap::Tier::ALL {
            assert_eq!(
                caps.of(tier),
                &TierCapability::Unsupported {
                    reason: NO_SYSTEM_LAYER.into()
                }
            );
        }
        let plan = SystemPlan {
            v: 1,
            owner: ProductId::Ubiqx,
            hosts: true,
            extra: vec![],
            resolver: None,
            policies: false,
        };
        assert_eq!(
            ports.system.apply(&plan).unwrap(),
            ApplyOutcome::Unavailable(NO_SYSTEM_LAYER.into())
        );
        assert_eq!(
            ports
                .system
                .remove(ProductId::Ubiqx, RemoveScope::All)
                .unwrap(),
            ApplyOutcome::Unavailable(NO_SYSTEM_LAYER.into())
        );
        assert!(ports.system.preview(&plan).unwrap().is_empty());
        assert_eq!(
            ports.system.observe().unwrap(),
            SystemObservation::default()
        );
        let opts = FrameOpts {
            max_edge: 640,
            crop_top_frac: 0.0,
        };
        assert!(ports.frames.frame(1, opts).unwrap().is_none());
        assert!(ports.detector.is_none());
        assert!(ports.shared_dir.is_none());
        // The module is off in a fresh repository.
        assert_eq!(ports.repo.load_config().unwrap(), NofapConfig::default());
        assert!(!ports.repo.load_config().unwrap().enabled);
    }

    #[test]
    fn nofap_memory_repo_keeps_config_sites_and_counters() {
        let repo = MemoryNofapRepo::default();
        let config = NofapConfig {
            enabled: true,
            delay_secs: 1_800,
            ..NofapConfig::default()
        };
        repo.save_config(&config).unwrap();
        assert_eq!(repo.load_config().unwrap(), config);

        // Upsert replaces by id and by (list, host_key).
        repo.upsert_site(&site("a", SiteOrigin::User, 1, None))
            .unwrap();
        repo.upsert_site(&site("b", SiteOrigin::Learned, 2, None))
            .unwrap();
        repo.upsert_site(&site("c", SiteOrigin::Learned, 2, None))
            .unwrap();
        repo.upsert_site(&site("s1", SiteOrigin::Snooze, 3, Some(at(12))))
            .unwrap();
        repo.upsert_site(&site("s2", SiteOrigin::Snooze, 4, Some(at(20))))
            .unwrap();
        let ids = |repo: &MemoryNofapRepo| -> Vec<String> {
            repo.list_sites()
                .unwrap()
                .into_iter()
                .map(|s| s.id)
                .collect()
        };
        assert_eq!(ids(&repo), ["a", "c", "s1", "s2"]);

        // A snooze that expires exactly now is gone; a later one stays.
        assert_eq!(repo.purge_expired_sites(at(12)).unwrap(), 1);
        assert_eq!(ids(&repo), ["a", "c", "s2"]);
        assert_eq!(repo.delete_sites_by_origin(SiteOrigin::Learned).unwrap(), 1);
        repo.delete_site("missing").unwrap();
        assert_eq!(ids(&repo), ["a", "s2"]);

        repo.bump_day("2026-09-17", 15, DayKind::BlockList, 1)
            .unwrap();
        repo.bump_day("2026-09-17", 15, DayKind::BlockList, 2)
            .unwrap();
        repo.bump_day("2026-09-17", 9, DayKind::Lapse, 1).unwrap();
        repo.bump_day("2026-09-10", 23, DayKind::VisionCheck, 5)
            .unwrap();
        let days = repo.days_between("2026-09-17", "2026-09-17").unwrap();
        assert_eq!(days.len(), 2);
        assert_eq!(
            (days[0].hour, days[0].kind, days[0].n),
            (9, DayKind::Lapse, 1)
        );
        assert_eq!(
            (days[1].hour, days[1].kind, days[1].n),
            (15, DayKind::BlockList, 3)
        );
        assert_eq!(
            repo.days_between("2026-09-01", "2026-09-30").unwrap().len(),
            3
        );
        assert_eq!(repo.delete_days_before("2026-09-17").unwrap(), 1);
        assert_eq!(
            repo.days_between("2026-01-01", "2026-12-31").unwrap().len(),
            2
        );

        // Erasing the history drops counters and snoozes; the user's list and the
        // configuration stay.
        repo.erase_history().unwrap();
        assert!(repo
            .days_between("2026-01-01", "2026-12-31")
            .unwrap()
            .is_empty());
        assert_eq!(ids(&repo), ["a"]);
        assert_eq!(repo.load_config().unwrap(), config);
    }
}
