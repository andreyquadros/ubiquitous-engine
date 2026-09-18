//! # ubiqx-platform
//!
//! Operating-system adapters for the ports defined in `ubiqx_core::ports`.
//!
//! * [`macos`] (only compiled on macOS): foreground application and window via AppKit and
//!   CoreGraphics, idle time, per-window screenshots, browser URL through AppleScript,
//!   permission checks and the Keychain-backed secret store.
//! * [`mock`]: a scripted platform that replays a scenario. It is what the CLI and the engine
//!   tests use on Linux/CI, and what the desktop app falls back to on unsupported systems.
//! * Portable helpers: [`image_util`] (downscale + JPEG encode), [`browser`] (AppleScript
//!   snippets and URL parsing), [`secrets`] (in-memory / environment stores),
//!   [`notify`] (logging notifier), [`update_feed`] (the update feed over HTTP).
//!
//! [`PlatformServices`] bundles one implementation of every port so composition roots have a
//! single thing to build.

use std::sync::Arc;

use ubiqx_core::ports::*;

pub mod browser;
pub mod image_util;
pub mod mock;
pub mod notify;
pub mod secrets;
pub mod update_feed;

#[cfg(target_os = "macos")]
pub mod macos;

/// One implementation of every platform port.
#[derive(Clone)]
pub struct PlatformServices {
    pub activity: Arc<dyn ActivitySource>,
    pub urls: Arc<dyn BrowserUrlResolver>,
    pub idle: Arc<dyn IdleDetector>,
    pub capturer: Arc<dyn ScreenCapturer>,
    pub permissions: Arc<dyn PermissionChecker>,
    pub secrets: Arc<dyn SecretStore>,
    pub notifier: Arc<dyn Notifier>,
    pub update_feed: Arc<dyn UpdateFeedSource>,
}

impl PlatformServices {
    /// The real platform on macOS; a scripted mock elsewhere (so the app still starts).
    pub fn native() -> Self {
        #[cfg(target_os = "macos")]
        {
            macos::services()
        }
        #[cfg(not(target_os = "macos"))]
        {
            tracing::warn!("no native platform adapter for this OS; using the scripted mock");
            let (mut services, _) = Self::scripted(mock::Scenario::demo_day());
            // The feed is plain HTTP: it works on any OS, so a developer can still test a
            // manual check here.
            services.update_feed = Arc::new(update_feed::HttpUpdateFeed::new());
            services
        }
    }

    /// A fully scripted platform (tests, CLI simulations). The returned handle lets the
    /// caller inspect or advance the scenario.
    pub fn scripted(scenario: mock::Scenario) -> (Self, Arc<mock::ScriptedPlatform>) {
        let scripted = Arc::new(mock::ScriptedPlatform::new(scenario));
        let services = Self {
            activity: scripted.clone(),
            urls: scripted.clone(),
            idle: scripted.clone(),
            capturer: Arc::new(mock::SyntheticCapturer),
            permissions: scripted.clone(),
            secrets: Arc::new(secrets::EnvOrMemorySecretStore::default()),
            notifier: Arc::new(notify::LogNotifier),
            update_feed: Arc::new(mock::StaticUpdateFeed::new(None)),
        };
        (services, scripted)
    }
}
