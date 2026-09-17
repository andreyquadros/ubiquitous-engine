//! Composition root shared by the desktop shell and the CLI.
//!
//! Everything concrete is chosen here: SQLite for persistence, the native platform adapters
//! (or the scripted mock), and the Anthropic-backed AI components (or fakes).

use std::path::PathBuf;
use std::sync::Arc;

use ubiqx_ai::client::{AnthropicClient, AnthropicConfig, ApiKeySource, LlmClient};
use ubiqx_core::ports::{secret_keys, EventSink, SecretStore};
use ubiqx_core::{CoreError, CoreResult, SystemClock};
use ubiqx_engine::{AiPorts, Engine, EngineDeps, EngineHandle, PlatformPorts, Repos};
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
    /// Real Anthropic API (key from the secret store).
    Anthropic,
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
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            data_dir: None,
            in_memory_db: false,
            scripted_platform: None,
            ai: AiBackend::Anthropic,
            notifier: None,
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

/// Bridges the engine's secret store to the AI client so a key pasted at runtime is used
/// immediately, without rebuilding the client.
struct SecretStoreKey(Arc<dyn SecretStore>);

impl ApiKeySource for SecretStoreKey {
    fn api_key(&self) -> Option<String> {
        self.0
            .get(secret_keys::ANTHROPIC_API_KEY)
            .ok()
            .flatten()
            .filter(|k| !k.trim().is_empty())
    }
}

/// Everything the shells need to keep around.
#[derive(Clone)]
pub struct App {
    pub engine: EngineHandle,
    pub store: Arc<SqliteStore>,
    pub platform: PlatformServices,
    pub anthropic: Option<Arc<AnthropicClient>>,
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
            None => (PlatformServices::native(), None),
        };
        if let Some(n) = config.notifier.clone() {
            platform.notifier = n;
        }

        let (ai, anthropic) = build_ai(config.ai, &platform, store.clone());

        let deps = EngineDeps {
            platform: PlatformPorts {
                activity: platform.activity.clone(),
                urls: platform.urls.clone(),
                idle: platform.idle.clone(),
                capturer: platform.capturer.clone(),
                permissions: platform.permissions.clone(),
                secrets: platform.secrets.clone(),
                notifier: platform.notifier.clone(),
            },
            repos: Repos::from_store(store.clone()),
            ai,
            sink,
            clock: Arc::new(SystemClock),
            data_dir: data_dir.clone(),
        };
        let engine = Engine::start(deps)?;
        Ok(App {
            engine,
            store,
            platform,
            anthropic,
            data_dir,
            scripted,
        })
    }

    /// Validates a candidate API key against the provider without storing it.
    pub async fn validate_api_key(&self, key: &str) -> CoreResult<()> {
        let source: Arc<dyn ApiKeySource> =
            Arc::new(ubiqx_ai::client::StaticApiKey(key.trim().to_string()));
        let client = AnthropicClient::new(AnthropicConfig::default(), source, self.store.clone())?;
        client.validate_key().await
    }
}

fn build_ai(
    backend: AiBackend,
    platform: &PlatformServices,
    store: Arc<SqliteStore>,
) -> (AiPorts, Option<Arc<AnthropicClient>>) {
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
        AiBackend::Anthropic => {
            let key: Arc<dyn ApiKeySource> = Arc::new(SecretStoreKey(platform.secrets.clone()));
            let client = match AnthropicClient::new(AnthropicConfig::default(), key, store) {
                Ok(c) => Arc::new(c),
                Err(e) => {
                    tracing::error!(error = %e, "could not build the Anthropic client; running without remote AI");
                    return (AiPorts::default(), None);
                }
            };
            let llm: Arc<dyn LlmClient> = client.clone();
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
                Some(client),
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
