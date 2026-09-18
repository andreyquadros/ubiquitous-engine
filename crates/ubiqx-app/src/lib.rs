//! Composition root shared by the desktop shell and the CLI.
//!
//! Everything concrete is chosen here: SQLite for persistence, the native platform adapters
//! (or the scripted mock), and the hosted AI components (or fakes). With
//! [`AiBackend::Remote`] one HTTP client per vendor (Anthropic, OpenAI, xAI) is built and the
//! port implementations talk to a [`RoutingLlmClient`] that forwards each call to the vendor
//! selected in settings, so the user can switch providers without restarting.

use std::path::PathBuf;
use std::sync::Arc;

use parking_lot::Mutex;
use ubiqx_ai::client::{
    AnthropicClient, AnthropicConfig, ApiKeySource, LanguageSource, LlmClient, StaticApiKey,
};
use ubiqx_ai::openai::{OpenAiCompatClient, OpenAiCompatConfig};
use ubiqx_ai::router::{ProviderSource, RoutingLlmClient};
use ubiqx_core::ports::{EventSink, InterventionPresenter, SecretStore, SettingsRepo};
use ubiqx_core::{AiModels, AiProvider, BuildInfo, CoreError, CoreResult, SystemClock, UiLanguage};
use ubiqx_engine::{
    AiPorts, Engine, EngineDeps, EngineHandle, LogInterventionPresenter, PlatformPorts, Repos,
};
use ubiqx_platform::PlatformServices;
use ubiqx_storage::{Db, SqliteStore};

pub use ubiqx_ai;
pub use ubiqx_core;
pub use ubiqx_engine;
pub use ubiqx_platform;
pub use ubiqx_storage;

/// Application identifier used for data directories and the Keychain.
pub const APP_ID: &str = "ai.ubiqx.app";

/// Which AI backend to wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiBackend {
    /// The real vendor APIs (Anthropic, OpenAI, xAI), routed by `settings.ai_provider`; each
    /// key comes from the secret store.
    Remote,
    /// Deterministic fakes (CI, demos, `--offline`).
    Fake,
    /// No remote AI at all.
    None,
}

#[derive(Clone)]
pub struct AppConfig {
    /// Directory for the database, screenshots and logs. `None` = platform default.
    pub data_dir: Option<PathBuf>,
    /// Use an in-memory database (tests / dry runs).
    pub in_memory_db: bool,
    /// Use the scripted platform instead of the native one.
    pub scripted_platform: Option<ubiqx_platform::mock::Scenario>,
    pub ai: AiBackend,
    /// Notifier provided by the shell (desktop notifications). Defaults to logging.
    pub notifier: Option<Arc<dyn ubiqx_core::ports::Notifier>>,
    /// Shows the focus guard's intervention window (the desktop shell). Defaults to logging,
    /// in which case interventions fall back to notifications.
    pub presenter: Option<Arc<dyn InterventionPresenter>>,
    /// Identity of the running build, stamped by the shell at compile time. The default is
    /// a development build, which never checks for updates automatically.
    pub build: BuildInfo,
    /// Where `latest.json` is fetched from; `None` = the product's rolling GitHub release
    /// ([`ubiqx_core::update::DEFAULT_FEED_URL`]).
    pub update_feed_url: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            data_dir: None,
            in_memory_db: false,
            scripted_platform: None,
            ai: AiBackend::Remote,
            notifier: None,
            presenter: None,
            build: BuildInfo::dev(),
            update_feed_url: None,
        }
    }
}

impl std::fmt::Debug for App {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("App")
            .field("data_dir", &self.data_dir)
            .finish()
    }
}

/// Platform default data directory (`~/Library/Application Support/ai.ubiqx.app` on macOS).
pub fn default_data_dir() -> PathBuf {
    directories::ProjectDirs::from("ai", "ubiqx", "ubiqX")
        .map(|d| d.data_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from(".ubiqx"))
}

/// Bridges the engine's secret store to one vendor's client so a key pasted at runtime is
/// used immediately, without rebuilding the client.
struct SecretStoreKey(Arc<dyn SecretStore>, AiProvider);

impl ApiKeySource for SecretStoreKey {
    fn api_key(&self) -> Option<String> {
        self.0
            .get(self.1.secret_key())
            .ok()
            .flatten()
            .filter(|k| !k.trim().is_empty())
    }
}

/// [`ProviderSource`] backed by the settings row of the store: the engine persists every
/// settings change before it applies it, so a read per call is always current (one small
/// row, negligible next to the HTTP call it precedes). A failed read keeps the last value
/// seen instead of silently routing to the default vendor.
struct StoredProvider {
    settings: Arc<dyn SettingsRepo>,
    last: Mutex<AiProvider>,
}

impl StoredProvider {
    fn new(settings: Arc<dyn SettingsRepo>) -> Self {
        let last = settings.load().map(|s| s.ai_provider).unwrap_or_default();
        Self {
            settings,
            last: Mutex::new(last),
        }
    }
}

impl ProviderSource for StoredProvider {
    fn provider(&self) -> AiProvider {
        match self.settings.load() {
            Ok(s) => {
                *self.last.lock() = s.ai_provider;
                s.ai_provider
            }
            Err(e) => {
                let last = *self.last.lock();
                tracing::warn!(error = %e, provider = last.id(), "could not read the selected AI provider; keeping the last one");
                last
            }
        }
    }
}

/// [`LanguageSource`] backed by the settings row of the store, so a rejection message is
/// worded in the language selected at the moment it is produced. Read only on that (rare)
/// path; a failed read falls back to the default language.
struct StoredLanguage(Arc<dyn SettingsRepo>);

impl LanguageSource for StoredLanguage {
    fn language(&self) -> UiLanguage {
        self.0.load().map(|s| s.ui_language()).unwrap_or_default()
    }
}

/// The vendor clients behind [`AiBackend::Remote`], all reading their key from the secret
/// store on every call and the UI language from the settings row when they word a rejection.
#[derive(Clone)]
pub struct RemoteClients {
    pub anthropic: Arc<AnthropicClient>,
    pub openai: Arc<OpenAiCompatClient>,
    pub xai: Arc<OpenAiCompatClient>,
}

impl RemoteClients {
    fn build(secrets: &Arc<dyn SecretStore>, store: Arc<SqliteStore>) -> CoreResult<Self> {
        let key = |p: AiProvider| -> Arc<dyn ApiKeySource> {
            Arc::new(SecretStoreKey(secrets.clone(), p))
        };
        let language: Arc<dyn LanguageSource> = Arc::new(StoredLanguage(store.clone()));
        Ok(Self {
            anthropic: Arc::new(
                AnthropicClient::new(
                    AnthropicConfig::default(),
                    key(AiProvider::Anthropic),
                    store.clone(),
                )?
                .with_language_source(language.clone()),
            ),
            openai: Arc::new(
                OpenAiCompatClient::new(
                    OpenAiCompatConfig::for_provider(AiProvider::OpenAi),
                    key(AiProvider::OpenAi),
                    store.clone(),
                )?
                .with_language_source(language.clone()),
            ),
            xai: Arc::new(
                OpenAiCompatClient::new(
                    OpenAiCompatConfig::for_provider(AiProvider::Xai),
                    key(AiProvider::Xai),
                    store,
                )?
                .with_language_source(language),
            ),
        })
    }

    /// The vendor's client as the trait object the router and the ports use.
    pub fn llm(&self, provider: AiProvider) -> Arc<dyn LlmClient> {
        match provider {
            AiProvider::Anthropic => self.anthropic.clone(),
            AiProvider::OpenAi => self.openai.clone(),
            AiProvider::Xai => self.xai.clone(),
        }
    }

    /// Chat-capable model ids the stored key of `provider` can use.
    pub async fn list_models(&self, provider: AiProvider) -> CoreResult<Vec<String>> {
        match provider {
            AiProvider::Anthropic => self.anthropic.list_models().await,
            AiProvider::OpenAi => self.openai.list_models().await,
            AiProvider::Xai => self.xai.list_models().await,
        }
    }
}

impl std::fmt::Debug for RemoteClients {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RemoteClients").finish_non_exhaustive()
    }
}

/// Everything the shells need to keep around.
#[derive(Clone)]
pub struct App {
    pub engine: EngineHandle,
    pub store: Arc<SqliteStore>,
    pub platform: PlatformServices,
    /// The vendor clients, present with [`AiBackend::Remote`] only.
    pub remote: Option<RemoteClients>,
    pub data_dir: PathBuf,
    pub scripted: Option<Arc<ubiqx_platform::mock::ScriptedPlatform>>,
}

impl App {
    /// Builds and starts the engine. Must be called inside a tokio runtime.
    pub fn start(config: AppConfig, sink: Arc<dyn EventSink>) -> CoreResult<App> {
        let data_dir = config.data_dir.clone().unwrap_or_else(default_data_dir);
        std::fs::create_dir_all(&data_dir)
            .map_err(|e| CoreError::Platform(format!("data dir {}: {e}", data_dir.display())))?;

        let db = if config.in_memory_db {
            Db::open_in_memory()?
        } else {
            Db::open(&data_dir.join("ubiqx.db"))?
        };
        let store = Arc::new(SqliteStore::new(db));

        let (mut platform, scripted) = match config.scripted_platform {
            Some(scenario) => {
                let (p, s) = PlatformServices::scripted(scenario);
                (p, Some(s))
            }
            // The data directory also hosts the secrets file Windows and Linux fall back
            // to when the OS keyring is unavailable.
            None => (PlatformServices::native_in(Some(&data_dir)), None),
        };
        if let Some(n) = config.notifier.clone() {
            platform.notifier = n;
        }

        let (ai, remote) = build_ai(config.ai, &platform, store.clone());
        let presenter: Arc<dyn InterventionPresenter> = config
            .presenter
            .unwrap_or_else(|| Arc::new(LogInterventionPresenter));

        let deps = EngineDeps {
            platform: PlatformPorts {
                activity: platform.activity.clone(),
                urls: platform.urls.clone(),
                idle: platform.idle.clone(),
                capturer: platform.capturer.clone(),
                permissions: platform.permissions.clone(),
                secrets: platform.secrets.clone(),
                notifier: platform.notifier.clone(),
                update_feed: platform.update_feed.clone(),
                apps: platform.apps.clone(),
                enforcer: platform.enforcer.clone(),
                presenter,
            },
            repos: Repos::from_store(store.clone()),
            ai,
            sink,
            clock: Arc::new(SystemClock),
            data_dir: data_dir.clone(),
            build: config.build,
            update_feed_url: config
                .update_feed_url
                .filter(|u| !u.trim().is_empty())
                .unwrap_or_else(|| ubiqx_core::update::DEFAULT_FEED_URL.to_string()),
        };
        let engine = Engine::start(deps)?;
        Ok(App {
            engine,
            store,
            platform,
            remote,
            data_dir,
            scripted,
        })
    }

    /// Validates a candidate API key of `provider` against that vendor without storing it.
    /// The billing probe uses the engine's current classification model when it belongs to
    /// `provider`, otherwise the vendor's recommended one, so a key is judged with the model
    /// that will actually be billed.
    pub async fn validate_api_key(&self, provider: AiProvider, key: &str) -> CoreResult<()> {
        let source: Arc<dyn ApiKeySource> = Arc::new(StaticApiKey(key.trim().to_string()));
        let language: Arc<dyn LanguageSource> = Arc::new(self.engine.settings().ui_language());
        match provider {
            AiProvider::Anthropic => {
                AnthropicClient::new(AnthropicConfig::default(), source, self.store.clone())?
                    .with_language_source(language)
                    .validate_key()
                    .await
            }
            AiProvider::OpenAi | AiProvider::Xai => {
                let current = self.engine.settings().models.classify;
                let probe = if provider.owns_model(&current) {
                    current
                } else {
                    AiModels::for_provider(provider).classify
                };
                OpenAiCompatClient::new(
                    OpenAiCompatConfig::for_provider(provider),
                    source,
                    self.store.clone(),
                )?
                .with_language_source(language)
                .validate_key(&probe)
                .await
            }
        }
    }

    /// Chat-capable model ids the stored key of `provider` can use, sorted. Works with any
    /// backend (the listing needs no port), returning [`CoreError::AiNotConfigured`] when
    /// that provider has no key.
    pub async fn list_models(&self, provider: AiProvider) -> CoreResult<Vec<String>> {
        match &self.remote {
            Some(remote) => remote.list_models(provider).await,
            None => {
                RemoteClients::build(&self.platform.secrets, self.store.clone())?
                    .list_models(provider)
                    .await
            }
        }
    }
}

fn build_ai(
    backend: AiBackend,
    platform: &PlatformServices,
    store: Arc<SqliteStore>,
) -> (AiPorts, Option<RemoteClients>) {
    match backend {
        AiBackend::None => (AiPorts::default(), None),
        AiBackend::Fake => (
            AiPorts {
                remote: Some(Arc::new(demo_fake_classifier())),
                vision: Some(Arc::new(ubiqx_ai::fake::FakeVisionClassifier::new(
                    Some(demo::CAT_IFRO),
                    "Respondeu mensagens sobre o edital de extensão",
                ))),
                report_writer: Some(Arc::new(ubiqx_ai::fake::FakeReportWriter::new())),
                advisor: Some(Arc::new(ubiqx_ai::fake::FakeAdvisor::new())),
                requires_api_key: false,
            },
            None,
        ),
        AiBackend::Remote => {
            let remote = match RemoteClients::build(&platform.secrets, store.clone()) {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!(error = %e, "could not build the AI clients; running without remote AI");
                    return (AiPorts::default(), None);
                }
            };
            let source: Arc<dyn ProviderSource> = Arc::new(StoredProvider::new(store));
            let llm: Arc<dyn LlmClient> = Arc::new(RoutingLlmClient::new(
                AiProvider::ALL
                    .into_iter()
                    .map(|p| (p, remote.llm(p)))
                    .collect(),
                source,
            ));
            (
                AiPorts {
                    remote: Some(Arc::new(ubiqx_ai::classifier::LlmTextClassifier::new(
                        llm.clone(),
                    ))),
                    vision: Some(Arc::new(ubiqx_ai::vision::LlmVisionClassifier::new(
                        llm.clone(),
                    ))),
                    report_writer: Some(Arc::new(ubiqx_ai::report::LlmReportWriter::new(
                        llm.clone(),
                    ))),
                    advisor: Some(Arc::new(ubiqx_ai::advisor::LlmAdvisor::new(llm))),
                    requires_api_key: true,
                },
                Some(remote),
            )
        }
    }
}

/// Fake classifier keyed to the demo categories and the demo scenario's apps/domains.
fn demo_fake_classifier() -> ubiqx_ai::fake::FakeClassifier {
    ubiqx_ai::fake::FakeClassifier::new([
        ("sei.ifro.edu.br", demo::CAT_IFRO),
        ("mail.google.com", demo::CAT_IFRO),
        ("visual studio code", demo::CAT_INCUBADORA),
        ("microsoft teams", demo::CAT_INCUBADORA),
        ("terminal", demo::CAT_INCUBADORA),
        ("docs.google.com", demo::CAT_CIDADES),
        ("youtube.com", ubiqx_core::system_categories::DISTRACTION),
    ])
}

/// Demo categories used by simulations and the scripted desktop mode.
pub mod demo {
    use chrono::Utc;
    use ubiqx_core::ports::CategoryRepo;
    use ubiqx_core::{Category, CoreResult};

    pub const CAT_IFRO: &str = "cat-ifro";
    pub const CAT_INCUBADORA: &str = "cat-incubadora";
    pub const CAT_CIDADES: &str = "cat-cidades";

    fn cat(
        id: &str,
        name: &str,
        color: &str,
        icon: &str,
        description: &str,
        keywords: &[&str],
        order: i32,
    ) -> Category {
        Category {
            id: id.into(),
            name: name.into(),
            color: color.into(),
            icon: icon.into(),
            description: description.into(),
            keywords: keywords.iter().map(|k| k.to_string()).collect(),
            report_time: None,
            report_template: None,
            is_productive: true,
            is_system: false,
            archived: false,
            sort_order: order,
            created_at: Utc::now(),
        }
    }

    /// Creates the three demo categories when no user category exists yet.
    pub fn seed(categories: &dyn CategoryRepo) -> CoreResult<bool> {
        if categories.list(true)?.iter().any(|c| !c.is_system) {
            return Ok(false);
        }
        for c in [
            cat(CAT_IFRO, "IFRO", "#2563EB", "graduation-cap", "Atividades como servidor do IFRO: SEI, editais, ensino, e-mail institucional, reuniões do campus.", &["ifro", "sei", "edital", "campus"], 0),
            cat(CAT_INCUBADORA, "Incubadora", "#F97316", "rocket", "Trabalho na incubadora de software: desenvolvimento, reuniões com startups, mentorias.", &["incubadora", "startup", "api", "código"], 1),
            cat(CAT_CIDADES, "Cidades Inteligentes", "#10B981", "building-2", "Projeto Cidades Inteligentes: plano de trabalho, documentos, reuniões com a prefeitura, sensores.", &["cidades", "prefeitura", "iot", "plano de trabalho"], 2),
        ] {
            categories.upsert(&c)?;
        }
        Ok(true)
    }
}

/// Initialises `tracing` with an env filter (`UBIQX_LOG`, default `info`) writing to stderr.
pub fn init_tracing(default: &str) {
    use tracing_subscriber::{fmt, EnvFilter};
    let filter = EnvFilter::try_from_env("UBIQX_LOG").unwrap_or_else(|_| EnvFilter::new(default));
    let _ = fmt().with_env_filter(filter).with_target(false).try_init();
}
