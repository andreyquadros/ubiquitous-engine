//! Composition root of the standalone product (design section 8): one [`NofapService`]
//! built from the store, the platform, the AI clients and the system layer, and the loops
//! that drive it. **No engine**: no `Engine`, `EngineState`, sampler, tracker, classifier,
//! report writer, nudge loop, focus guard or retention loop exists in the process, and
//! [`NofapApp`] holds no `EngineHandle`. The productivity tables exist in the database
//! (one migration list) and stay empty.
//!
//! The standalone shell exposes two commands: `nofap`, answered by [`NofapApp::handle`]
//! (the same handler as ubiqX's), and `nofap_host`, answered by [`NofapApp::handle_host`]
//! (the few settings the standalone product has of its own, the AI key, permissions,
//! updates).

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::watch;
use ubiqx_core::nofap::{
    AiReadiness, ExplicitDetector, FrameSource, NofapEnv, NofapEnvSnapshot, NofapPresenter,
    NofapSink,
};
use ubiqx_core::nofap_ipc::NofapCommand;
use ubiqx_core::ports::{
    EventSink, Notifier, PermissionKind, PermissionStatus, SecretStore, SettingsRepo,
};
use ubiqx_core::product::SHIELD;
use ubiqx_core::{
    AiModels, AiProvider, BuildInfo, Clock, CoreError, CoreResult, EngineEvent, Settings,
    SystemClock, UiLanguage, UpdateStatus,
};
use ubiqx_engine::nofap::Tuning;
use ubiqx_engine::{NofapDeps, NofapHost, NofapService, UpdateChecker};
use ubiqx_platform::mock::{Scenario, ScriptedPlatform};
use ubiqx_platform::PlatformServices;
use ubiqx_storage::{Db, SqliteStore};

use crate::{
    nofap_ports, nofap_shared_dir, nofap_system_layer, AiBackend, NofapWiring, RemoteClients,
};

/// Application identifier of the standalone product (data directory, keychain).
pub const APP_ID: &str = SHIELD.bundle_id;

/// How often the guard looks at the window in front.
pub const GUARD_EVERY: Duration = Duration::from_secs(2);
/// How often the system tiers are read again (never elevates).
pub const VERIFY_EVERY: Duration = Duration::from_secs(120);
/// Wait before the first automatic update check, so start-up stays quiet.
pub const UPDATE_FIRST_AFTER: Duration = Duration::from_secs(45);
pub const UPDATE_EVERY: Duration = Duration::from_secs(6 * 3600);

/// How long an answer of the secret store about a key is reused. The guard asks for the
/// environment every pass while the AI check is on, and the store is the operating
/// system's keychain; a stored or removed key is seen at once all the same, because the
/// host commands that change it clear the answer.
const KEY_ANSWER_FOR: Duration = Duration::from_secs(15);

/// Platform default data directory of the standalone product: the user's data directory
/// joined with the identifier, which is where its Tauri shell's `app_data_dir` points
/// (`~/Library/Application Support/ai.ubiqx.shield` on macOS).
pub fn default_data_dir() -> PathBuf {
    directories::BaseDirs::new()
        .map(|d| d.data_dir().join(APP_ID))
        .unwrap_or_else(|| PathBuf::from(APP_ID))
}

/// What the standalone shell chooses. The fields after `update_feed_url` replace one
/// piece of the composition, for tests and demos; production leaves them `None`.
#[derive(Clone)]
pub struct NofapAppConfig {
    /// Directory of the database, exports and the secrets file Windows and Linux fall back
    /// to. `None` = [`default_data_dir`].
    pub data_dir: Option<PathBuf>,
    /// Use an in-memory database (tests).
    pub in_memory_db: bool,
    /// Use the scripted platform instead of the native one.
    pub scripted_platform: Option<Scenario>,
    /// The vendor APIs (Anthropic, OpenAI, xAI), the fake check (it never sees anything) or
    /// no check at all.
    pub ai: AiBackend,
    /// Desktop notifications of the shell (the card's fallback). Defaults to the platform's.
    pub notifier: Option<Arc<dyn Notifier>>,
    /// Shows the card window. Defaults to logging, in which case a card falls back to the
    /// neutral notification.
    pub presenter: Option<Arc<dyn NofapPresenter>>,
    /// Receives `EngineEvent::UpdateAvailable` from the update checker. Defaults to dropping it.
    pub update_sink: Option<Arc<dyn EventSink>>,
    /// Whether the machine's system layer (hosts, resolver, policies) and the primary lock
    /// shared with ubiqX are wired. The system layer elevates the running executable with
    /// `--shield-helper`, so only a shell whose `main` hands those runs to the helper may
    /// set it. Ignored with a scripted platform.
    pub system_layer: bool,
    /// Spawn the guard, verify and update loops. Tests drive the service by hand.
    pub loops: bool,
    pub build: BuildInfo,
    /// `None` = the standalone product's own feed (`SHIELD.feed_url`).
    pub update_feed_url: Option<String>,
    pub detector: Option<Arc<dyn ExplicitDetector>>,
    pub frames: Option<Arc<dyn FrameSource>>,
    pub clock: Option<Arc<dyn Clock>>,
    pub tuning: Option<Tuning>,
}

impl Default for NofapAppConfig {
    fn default() -> Self {
        Self {
            data_dir: None,
            in_memory_db: false,
            scripted_platform: None,
            ai: AiBackend::Remote,
            notifier: None,
            presenter: None,
            update_sink: None,
            system_layer: false,
            loops: true,
            build: BuildInfo::dev(),
            update_feed_url: None,
            detector: None,
            frames: None,
            clock: None,
            tuning: None,
        }
    }
}

/// [`NofapEnv`] of the standalone product: its settings row and its secret store. There is
/// no `local_only`, no privacy list and no managed provider in this product.
pub struct StandaloneEnv {
    settings: Arc<dyn SettingsRepo>,
    secrets: Arc<dyn SecretStore>,
    /// `false` with the fake backend, which needs no key.
    requires_key: bool,
    key: Mutex<Option<(AiProvider, bool, Instant)>>,
}

impl StandaloneEnv {
    pub fn new(
        settings: Arc<dyn SettingsRepo>,
        secrets: Arc<dyn SecretStore>,
        requires_key: bool,
    ) -> Self {
        Self {
            settings,
            secrets,
            requires_key,
            key: Mutex::new(None),
        }
    }

    /// A key was stored or removed, or another provider chosen: ask the store again.
    pub fn forget_key(&self) {
        *self.key.lock() = None;
    }

    fn has_key(&self, provider: AiProvider) -> bool {
        if let Some((of, present, at)) = *self.key.lock() {
            if of == provider && at.elapsed() < KEY_ANSWER_FOR {
                return present;
            }
        }
        // The keychain can take its time: asked without holding the lock.
        let present = matches!(
            self.secrets.get(provider.secret_key()),
            Ok(Some(key)) if !key.trim().is_empty()
        );
        *self.key.lock() = Some((provider, present, Instant::now()));
        present
    }

    fn readiness(&self, settings: &Settings) -> AiReadiness {
        if !settings.onboarding_done {
            AiReadiness::NotOnboarded
        } else if settings.ai_provider.is_managed() {
            // "IA do Ubi" and licence keys are not in this product (design 8.5).
            AiReadiness::LicenceRequired
        } else if !self.requires_key || self.has_key(settings.ai_provider) {
            AiReadiness::Ready
        } else {
            AiReadiness::NoKey
        }
    }
}

impl NofapEnv for StandaloneEnv {
    fn snapshot(&self) -> NofapEnvSnapshot {
        let settings = match self.settings.load() {
            Ok(settings) => settings,
            Err(error) => {
                // Nothing may be sent anywhere while the settings cannot be read.
                tracing::debug!(error = %error, "nofap: the settings row could not be read");
                return NofapEnvSnapshot {
                    language: UiLanguage::default(),
                    provider: AiProvider::default(),
                    vision_model: String::new(),
                    ai: AiReadiness::NotOnboarded,
                    excluded_apps: Vec::new(),
                    excluded_domains: Vec::new(),
                };
            }
        };
        NofapEnvSnapshot {
            language: settings.ui_language(),
            provider: settings.ai_provider,
            vision_model: settings.models.vision.clone(),
            ai: self.readiness(&settings),
            excluded_apps: Vec::new(),
            excluded_domains: Vec::new(),
        }
    }
}

/// The settings the standalone product has of its own (`nofap_host` `settings_get`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HostSettings {
    pub ui_language: UiLanguage,
    pub ai_provider: AiProvider,
    pub vision_model: String,
    pub check_updates: bool,
    pub onboarding_done: bool,
}

impl HostSettings {
    fn of(settings: &Settings) -> Self {
        Self {
            ui_language: settings.ui_language(),
            ai_provider: settings.ai_provider,
            vision_model: settings.models.vision.clone(),
            check_updates: settings.check_updates,
            onboarding_done: settings.onboarding_done,
        }
    }
}

/// `settings_set`: the same fields, each optional; only the ones given change.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct HostSettingsPatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ui_language: Option<UiLanguage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ai_provider: Option<AiProvider>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vision_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub check_updates: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub onboarding_done: Option<bool>,
}

/// `about`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HostAbout {
    pub display_name: String,
    pub identifier: String,
    pub build: BuildInfo,
}

/// The second command of the standalone shell, `nofap_host(command)`. Wire form
/// `{ "cmd": "settings_set", "args": { … } }`, like `NofapCommand`. The reply of each
/// command is documented on its variant. No `Debug`: `api_key_set` carries a key.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "cmd", content = "args", rename_all = "snake_case")]
pub enum NofapHostCommand {
    /// → [`HostSettings`]
    SettingsGet,
    /// → [`HostSettings`], after the write
    SettingsSet(HostSettingsPatch),
    /// Validates the key with the vendor, then stores it in the keychain. → `null`
    ApiKeySet { provider: AiProvider, key: String },
    /// → `null`
    ApiKeyClear { provider: AiProvider },
    /// Model ids the stored key of `provider` can use. → `string[]`
    Models { provider: AiProvider },
    /// → `PermissionStatus`
    Permissions,
    /// Raises the system prompt or opens the settings pane. → `null`
    PermissionRequest { kind: PermissionKind },
    /// A manual update check. → `UpdateStatus`
    UpdateCheck,
    /// → [`HostAbout`]
    About,
}

impl NofapHostCommand {
    /// The command's name on the wire.
    pub fn name(&self) -> &'static str {
        match self {
            NofapHostCommand::SettingsGet => "settings_get",
            NofapHostCommand::SettingsSet(_) => "settings_set",
            NofapHostCommand::ApiKeySet { .. } => "api_key_set",
            NofapHostCommand::ApiKeyClear { .. } => "api_key_clear",
            NofapHostCommand::Models { .. } => "models",
            NofapHostCommand::Permissions => "permissions",
            NofapHostCommand::PermissionRequest { .. } => "permission_request",
            NofapHostCommand::UpdateCheck => "update_check",
            NofapHostCommand::About => "about",
        }
    }
}

/// An update sink that drops everything (no shell listening).
struct NoUpdateSink;

impl EventSink for NoUpdateSink {
    fn emit(&self, _event: EngineEvent) {}
}

/// Everything the standalone shell keeps around.
pub struct NofapApp {
    pub service: Arc<NofapService>,
    pub store: Arc<SqliteStore>,
    pub platform: PlatformServices,
    /// The vendor clients, present with [`AiBackend::Remote`] only.
    pub remote: Option<RemoteClients>,
    pub updates: Arc<UpdateChecker>,
    pub data_dir: PathBuf,
    pub scripted: Option<Arc<ScriptedPlatform>>,
    pub build: BuildInfo,
    env: Arc<StandaloneEnv>,
    stop: watch::Sender<bool>,
}

impl std::fmt::Debug for NofapApp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NofapApp")
            .field("data_dir", &self.data_dir)
            .finish_non_exhaustive()
    }
}

impl NofapApp {
    /// Builds the service and, with `config.loops`, spawns its loops. Must be called inside
    /// a tokio runtime. `sink` receives the module's events.
    pub fn start(config: NofapAppConfig, sink: Arc<dyn NofapSink>) -> CoreResult<NofapApp> {
        let data_dir = config.data_dir.clone().unwrap_or_else(default_data_dir);
        std::fs::create_dir_all(&data_dir)
            .map_err(|e| CoreError::Platform(format!("data dir {}: {e}", data_dir.display())))?;
        let db = if config.in_memory_db {
            Db::open_in_memory()?
        } else {
            Db::open(&data_dir.join(SHIELD.db_file))?
        };
        let store = Arc::new(SqliteStore::new(db));

        let (mut platform, scripted) = match config.scripted_platform.clone() {
            Some(scenario) => {
                let (p, s) = PlatformServices::scripted(scenario);
                (p, Some(s))
            }
            // Its own keyring service and its own data directory: the secrets file must
            // not fall into ubiqX's directory (integrator decision 25).
            None => (PlatformServices::native_for(&SHIELD, Some(&data_dir)), None),
        };
        if let Some(n) = config.notifier.clone() {
            platform.notifier = n;
        }
        if let Some(frames) = config.frames.clone() {
            platform.frames = frames;
        }

        let (remote, detector, requires_key) = match config.ai {
            AiBackend::None => (None, None, false),
            AiBackend::Fake => (None, Some(crate::demo_fake_detector()), false),
            AiBackend::Remote => {
                // The managed provider is not offered here, but its client costs nothing to
                // build and keeps the router total.
                let remote = RemoteClients::build(
                    &platform.secrets,
                    store.clone(),
                    ubiqx_core::license::UBIQX_API_BASE_DEFAULT,
                )?;
                let detector: Arc<dyn ExplicitDetector> = Arc::new(
                    ubiqx_ai::explicit::LlmExplicitDetector::new(remote.routing(store.clone())),
                );
                (Some(remote), Some(detector), true)
            }
        };
        let detector = config.detector.clone().or(detector);

        let system = config.system_layer && config.scripted_platform.is_none();
        let mut ports = nofap_ports(
            &platform,
            store.clone(),
            NofapWiring {
                system: nofap_system_layer(&SHIELD, system),
                detector,
                presenter: config.presenter.clone(),
                sink: Some(sink),
                shared_dir: if system { nofap_shared_dir() } else { None },
            },
        );
        if let Some(tuning) = config.tuning.clone() {
            ports.tuning = tuning;
        }

        let clock: Arc<dyn Clock> = config.clock.clone().unwrap_or(Arc::new(SystemClock));
        let env = Arc::new(StandaloneEnv::new(
            store.clone(),
            platform.secrets.clone(),
            requires_key,
        ));
        let service = NofapService::new(NofapDeps::assemble(
            ports,
            NofapHost {
                env: env.clone(),
                activity: platform.activity.clone(),
                urls: platform.urls.clone(),
                enforcer: platform.enforcer.clone(),
                permissions: platform.permissions.clone(),
                notifier: platform.notifier.clone(),
                usage: store.clone(),
                clock: clock.clone(),
                identity: &SHIELD,
                export_dir: data_dir.join("exports"),
            },
        ));

        let feed_url = config
            .update_feed_url
            .clone()
            .filter(|u| !u.trim().is_empty())
            .unwrap_or_else(|| SHIELD.feed_url.to_string());
        let updates = Arc::new(
            UpdateChecker::new(
                config.build.clone(),
                feed_url,
                platform.update_feed.clone(),
                store.clone(),
                store.clone(),
                platform.notifier.clone(),
                config.update_sink.clone().unwrap_or(Arc::new(NoUpdateSink)),
                clock,
            )
            .for_product(SHIELD.feed_product),
        );

        let (stop, _) = watch::channel(false);
        if config.loops {
            tokio::spawn(guard_loop(service.clone(), GUARD_EVERY, stop.subscribe()));
            tokio::spawn(verify_loop(service.clone(), VERIFY_EVERY, stop.subscribe()));
            tokio::spawn(update_loop(
                updates.clone(),
                store.clone(),
                UPDATE_FIRST_AFTER,
                UPDATE_EVERY,
                stop.subscribe(),
            ));
        }

        Ok(NofapApp {
            service,
            store,
            platform,
            remote,
            updates,
            data_dir,
            scripted,
            build: config.build,
            env,
            stop,
        })
    }

    /// The `nofap` command, the same handler as ubiqX's. Blocks (the database, and for the
    /// system commands the password prompt): a shell calls it on the blocking pool.
    pub fn handle(&self, command: NofapCommand) -> CoreResult<Value> {
        ubiqx_engine::nofap::ipc::handle(&self.service, command)
    }

    /// The `nofap_host` command. Settings and keychain work is blocking but short; the
    /// key validation, the model list and the update check are network calls.
    pub async fn handle_host(&self, command: NofapHostCommand) -> CoreResult<Value> {
        match command {
            NofapHostCommand::SettingsGet => reply(HostSettings::of(&self.store.load()?)),
            NofapHostCommand::SettingsSet(patch) => reply(self.settings_set(patch)?),
            NofapHostCommand::ApiKeySet { provider, key } => {
                own_provider(provider)?;
                let key = key.trim().to_string();
                if key.is_empty() {
                    return Err(CoreError::Invalid("api key: empty".into()));
                }
                let settings = self.store.load()?;
                if self.remote.is_some() {
                    crate::validate_api_key(
                        self.store.clone(),
                        provider,
                        &key,
                        settings.ui_language(),
                        &settings.models.vision,
                    )
                    .await?;
                }
                self.platform.secrets.set(provider.secret_key(), &key)?;
                self.ai_changed();
                Ok(Value::Null)
            }
            NofapHostCommand::ApiKeyClear { provider } => {
                own_provider(provider)?;
                self.platform.secrets.delete(provider.secret_key())?;
                self.ai_changed();
                Ok(Value::Null)
            }
            NofapHostCommand::Models { provider } => {
                own_provider(provider)?;
                let models = match &self.remote {
                    Some(remote) => remote.list_models(provider).await?,
                    None => {
                        RemoteClients::build(
                            &self.platform.secrets,
                            self.store.clone(),
                            ubiqx_core::license::UBIQX_API_BASE_DEFAULT,
                        )?
                        .list_models(provider)
                        .await?
                    }
                };
                reply(models)
            }
            NofapHostCommand::Permissions => reply(self.permissions()),
            NofapHostCommand::PermissionRequest { kind } => {
                self.platform.permissions.request(kind)?;
                Ok(Value::Null)
            }
            NofapHostCommand::UpdateCheck => reply(self.updates.check_now().await?),
            NofapHostCommand::About => reply(HostAbout {
                display_name: SHIELD.display_name.to_string(),
                identifier: SHIELD.bundle_id.to_string(),
                build: self.build.clone(),
            }),
        }
    }

    /// The update checker's status (the tray's update item).
    pub fn update_status(&self) -> UpdateStatus {
        self.updates.status()
    }

    /// Stops the loops. The guard loop gives up the primary lock as it ends.
    pub fn shutdown(&self) {
        let _ = self.stop.send(true);
    }

    fn permissions(&self) -> PermissionStatus {
        self.platform.permissions.status()
    }

    fn settings_set(&self, patch: HostSettingsPatch) -> CoreResult<HostSettings> {
        let mut settings = self.store.load()?;
        if let Some(lang) = patch.ui_language {
            settings.language = lang.tag().to_string();
        }
        if let Some(provider) = patch.ai_provider {
            own_provider(provider)?;
            if provider != settings.ai_provider {
                settings.ai_provider = provider;
                // The models of the vendor that was chosen before belong to nobody now.
                settings.models = AiModels::for_provider(provider);
            }
        }
        if let Some(model) = patch.vision_model {
            let model = model.trim();
            if model.is_empty() {
                return Err(CoreError::Invalid("vision model: empty".into()));
            }
            if AiProvider::for_model(model).is_some_and(|of| of != settings.ai_provider) {
                return Err(CoreError::Invalid(format!(
                    "vision model {model} is not a model of {}",
                    settings.ai_provider.id()
                )));
            }
            settings.models.vision = model.to_string();
        }
        if let Some(check) = patch.check_updates {
            settings.check_updates = check;
        }
        if let Some(done) = patch.onboarding_done {
            settings.onboarding_done = done;
        }
        self.store.save(&settings)?;
        self.ai_changed();
        Ok(HostSettings::of(&settings))
    }

    fn ai_changed(&self) {
        self.env.forget_key();
        self.service.ai_settings_changed();
    }
}

impl Drop for NofapApp {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn reply<T: Serialize>(value: T) -> CoreResult<Value> {
    serde_json::to_value(value).map_err(|error| CoreError::Other(error.to_string()))
}

/// The standalone product offers Anthropic, OpenAI and xAI only (design 8.5).
fn own_provider(provider: AiProvider) -> CoreResult<()> {
    if provider.is_managed() {
        return Err(CoreError::Invalid(
            "the managed provider is not offered in this product".into(),
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Loops
// ---------------------------------------------------------------------------------------------

/// Every `every` one guard pass on the blocking pool (the first tick is skipped); a check
/// the pass asks for is spawned, not awaited. Gives up the primary lock when it ends.
async fn guard_loop(service: Arc<NofapService>, every: Duration, mut stop: watch::Receiver<bool>) {
    let mut tick = tokio::time::interval(every);
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    tick.tick().await;
    loop {
        tokio::select! {
            _ = stop.changed() => break,
            _ = tick.tick() => {}
        }
        let pass = {
            let service = service.clone();
            tokio::task::spawn_blocking(move || service.guard_pass()).await
        };
        match pass {
            Ok(Ok(outcome)) => {
                if let Some(job) = outcome.job {
                    let service = service.clone();
                    tokio::spawn(async move {
                        let _ = service.vision_check(job).await;
                    });
                }
            }
            Ok(Err(_)) => tracing::debug!("nofap: a guard pass failed"),
            Err(_) => tracing::warn!("nofap: a guard pass panicked"),
        }
    }
    let _ = tokio::task::spawn_blocking(move || service.release_primary()).await;
}

/// Every `every` one verify pass on the blocking pool (the first tick is skipped).
async fn verify_loop(service: Arc<NofapService>, every: Duration, mut stop: watch::Receiver<bool>) {
    let mut tick = tokio::time::interval(every);
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    tick.tick().await;
    loop {
        tokio::select! {
            _ = stop.changed() => break,
            _ = tick.tick() => {}
        }
        let service = service.clone();
        match tokio::task::spawn_blocking(move || service.verify_pass()).await {
            Ok(Ok(())) => {}
            Ok(Err(_)) => tracing::debug!("nofap: a verify pass failed"),
            Err(_) => tracing::warn!("nofap: a verify pass panicked"),
        }
    }
}

/// Automatic update checks: one after `first`, then every `every`, each only while the
/// user wants them and the build is a published one.
async fn update_loop(
    updates: Arc<UpdateChecker>,
    settings: Arc<dyn SettingsRepo>,
    first: Duration,
    every: Duration,
    mut stop: watch::Receiver<bool>,
) {
    let mut wait = first;
    loop {
        tokio::select! {
            _ = stop.changed() => break,
            _ = tokio::time::sleep(wait) => {}
        }
        wait = every;
        let wanted = settings.load().map(|s| s.check_updates).unwrap_or(false);
        if updates.enabled() && wanted {
            if let Err(e) = updates.check_scheduled().await {
                tracing::warn!(error = %e, "update check failed");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    /// A keychain that counts how often it is asked.
    #[derive(Default)]
    struct CountingSecrets {
        values: Mutex<HashMap<String, String>>,
        gets: AtomicUsize,
    }

    impl SecretStore for CountingSecrets {
        fn get(&self, key: &str) -> CoreResult<Option<String>> {
            self.gets.fetch_add(1, Ordering::SeqCst);
            Ok(self.values.lock().get(key).cloned())
        }

        fn set(&self, key: &str, value: &str) -> CoreResult<()> {
            self.values.lock().insert(key.into(), value.into());
            Ok(())
        }

        fn delete(&self, key: &str) -> CoreResult<()> {
            self.values.lock().remove(key);
            Ok(())
        }
    }

    fn env(settings: Settings, requires_key: bool) -> (StandaloneEnv, Arc<CountingSecrets>) {
        let store = Arc::new(SqliteStore::new(Db::open_in_memory().unwrap()));
        store.save(&settings).unwrap();
        let secrets = Arc::new(CountingSecrets::default());
        (
            StandaloneEnv::new(store, secrets.clone(), requires_key),
            secrets,
        )
    }

    fn onboarded(provider: AiProvider) -> Settings {
        Settings {
            onboarding_done: true,
            ai_provider: provider,
            language: "en".into(),
            ..Settings::default()
        }
    }

    #[test]
    fn readiness_follows_the_settings_row_and_the_keychain() {
        let (not_yet, _) = env(Settings::default(), true);
        assert_eq!(not_yet.snapshot().ai, AiReadiness::NotOnboarded);

        // The managed provider is not part of this product.
        let (managed, _) = env(onboarded(AiProvider::Ubi), false);
        assert_eq!(managed.snapshot().ai, AiReadiness::LicenceRequired);

        let (env, secrets) = env(onboarded(AiProvider::OpenAi), true);
        let snapshot = env.snapshot();
        assert_eq!(snapshot.ai, AiReadiness::NoKey);
        assert_eq!(snapshot.language, UiLanguage::En);
        assert_eq!(snapshot.provider, AiProvider::OpenAi);
        assert_eq!(
            snapshot.vision_model,
            onboarded(AiProvider::OpenAi).models.vision
        );
        assert!(snapshot.excluded_apps.is_empty() && snapshot.excluded_domains.is_empty());

        // The guard asks every pass: the keychain is not asked every time.
        secrets
            .set(AiProvider::OpenAi.secret_key(), "sk-ana")
            .unwrap();
        assert_eq!(env.snapshot().ai, AiReadiness::NoKey);
        assert_eq!(secrets.gets.load(Ordering::SeqCst), 1);
        // A host command that stored the key clears the answer.
        env.forget_key();
        assert_eq!(env.snapshot().ai, AiReadiness::Ready);
        assert_eq!(secrets.gets.load(Ordering::SeqCst), 2);
        // A blank key is no key.
        secrets.set(AiProvider::OpenAi.secret_key(), "  ").unwrap();
        env.forget_key();
        assert_eq!(env.snapshot().ai, AiReadiness::NoKey);
    }

    #[test]
    fn the_fake_backend_needs_no_key() {
        let (env, secrets) = env(onboarded(AiProvider::Anthropic), false);
        assert_eq!(env.snapshot().ai, AiReadiness::Ready);
        assert_eq!(secrets.gets.load(Ordering::SeqCst), 0);
    }
}
