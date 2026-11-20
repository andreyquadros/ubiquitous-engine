//! # ubiqx-platform
//!
//! Operating-system adapters for the ports defined in `ubiqx_core::ports`.
//!
//! * [`macos`] (only compiled on macOS): foreground application and window via AppKit and
//!   CoreGraphics, idle time, per-window screenshots, browser URL through AppleScript,
//!   permission checks, the Keychain-backed secret store, the installed-app catalogue and
//!   the focus guard's enforcer (quit apps, close tabs, hide windows, run Shortcuts).
//! * [`windows`] (only compiled on Windows): foreground window and process through Win32,
//!   idle time, screenshots through `xcap`, browser URL through UI Automation, the
//!   Credential Manager (with a file fallback), Start Menu + `App Paths` catalogue and an
//!   enforcer (close windows, `Ctrl+W`, minimise others, `cmd /C`).
//! * [`linux`] (only compiled on Linux): the same ports over X11 (`x11rb`), `/proc`, the
//!   Secret Service (with a file fallback), `.desktop` entries and XTest.
//! * [`mock`]: a scripted platform that replays a scenario, plus a fixed app catalogue and a
//!   recording enforcer. It is what the CLI and the engine tests use on Linux/CI, and what
//!   the desktop app falls back to on unsupported systems. [`mock_nofap`] holds the
//!   scripted system layer and frame source of the `nofap` module.
//! * [`sysblock`] (compiled on every OS): the system layer of the `nofap` module, i.e. the
//!   hosts block, the family resolver and the browser policies behind the port
//!   `SystemShield`, and the privileged helper that applies them. [`frame`] is the module's
//!   frame source: one reduced image of one window, in memory only.
//! * Portable helpers: [`image_util`] (downscale + JPEG encode), [`browser`] (AppleScript
//!   snippets and URL parsing), [`browser_url`] (address-bar and title parsing for the
//!   other OSes), [`app_id`] (executable stems), [`catalog`] (catalogue cache and order),
//!   [`secrets`] (in-memory / environment stores), [`secrets_file`] (the keyring + file
//!   fallback), [`notify`] (logging notifier), [`update_feed`] (the update feed over HTTP).
//!
//! [`PlatformServices`] bundles one implementation of every port so composition roots have a
//! single thing to build.

use std::path::Path;
use std::sync::Arc;

use ubiqx_core::ports::*;

pub mod app_id;
pub mod browser;
pub mod browser_url;
pub mod catalog;
pub mod frame;
pub mod image_util;
pub mod mock;
pub mod mock_nofap;
pub mod notify;
pub mod secrets;
pub mod secrets_file;
pub mod sysblock;
pub mod update_feed;

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "linux")]
pub mod linux;

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
    /// Installed applications, for the focus page's search.
    pub apps: Arc<dyn AppCatalog>,
    /// What the focus guard may do to a distraction.
    pub enforcer: Arc<dyn Enforcer>,
    /// The frame source of the `nofap` module: one reduced picture of one window, taken
    /// by a capturer that never answers with the screen in place of the window
    /// ([`frame`]).
    pub frames: Arc<dyn ubiqx_core::nofap::FrameSource>,
}

impl PlatformServices {
    /// The real platform of this OS (macOS, Windows, Linux); a scripted mock elsewhere (so
    /// the app still starts). Same as [`native_in`](Self::native_in) with no data directory.
    pub fn native() -> Self {
        Self::native_in(None)
    }

    /// The real platform of this OS for ubiqX. `data_dir` is where Windows and Linux keep
    /// the secrets file they fall back to when the OS keyring is unavailable (see
    /// [`secrets_file`]); `None` picks a per-user default.
    pub fn native_in(data_dir: Option<&Path>) -> Self {
        Self::native_for(&ubiqx_core::product::UBIQX, data_dir)
    }

    /// The real platform of this OS for one of the two products: the same adapters, with
    /// the product's own keyring service, so the standalone app and ubiqX never read each
    /// other's keys.
    #[allow(unused_variables)]
    pub fn native_for(identity: &ubiqx_core::ProductIdentity, data_dir: Option<&Path>) -> Self {
        #[cfg(target_os = "macos")]
        {
            macos::services(identity.keyring_service)
        }
        #[cfg(target_os = "windows")]
        {
            windows::services(data_dir, identity.keyring_service)
        }
        #[cfg(target_os = "linux")]
        {
            linux::services(data_dir, identity.keyring_service)
        }
        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
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
            apps: Arc::new(mock::MockAppCatalog),
            enforcer: Arc::new(mock::MockEnforcer::default()),
            frames: Arc::new(frame::WindowFrameSource::new(Arc::new(
                mock::SyntheticCapturer,
            ))),
        };
        (services, scripted)
    }
}
