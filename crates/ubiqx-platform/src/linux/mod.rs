//! Linux adapters (X11). Every public item maps one-to-one onto a port in
//! `ubiqx_core::ports`.
//!
//! Identity: an application is its executable's file stem (`/proc/<pid>/exe`, lower-case:
//! `firefox`, `code`, `gnome-terminal-server`), see [`crate::app_id`]; when the pid is not
//! readable (a Flatpak sandbox, a remote client) the `WM_CLASS` instance stands in. The
//! human name is the `.desktop` entry's `Name` when the catalogue knows the executable,
//! else the `WM_CLASS` class.
//!
//! Display server: everything window-related goes through X11 (`x11rb`, pure Rust), and an
//! X11 client only sees X11 windows. In a Wayland session the applications running through
//! XWayland are tracked; while a Wayland-native window has the focus, `_NET_ACTIVE_WINDOW`
//! on the XWayland root points nowhere and [`frontmost::X11ActivitySource`] reports nothing
//! in the foreground. Without any `DISPLAY` (Wayland without XWayland, headless) the same
//! happens, logged once, and screenshots fail with a platform error the engine tolerates.
//! A portal route (`org.freedesktop.portal`) is the future path there.
//!
//! Permission model: X11 has none of the macOS grants, so every kind is `NotApplicable`.

use std::path::Path;
use std::sync::Arc;

use crate::PlatformServices;

pub mod apps;
pub mod browser;
pub mod capture;
pub mod enforce;
pub mod frontmost;
pub mod idle;
pub mod permissions;
pub mod proc;
pub mod secrets;
pub mod x11;

/// Builds the real Linux services. `data_dir` hosts the secrets file used when the Secret
/// Service is unavailable (see [`crate::secrets_file`]).
pub fn services(data_dir: Option<&Path>) -> PlatformServices {
    let apps = Arc::new(apps::LinuxAppCatalog::default());
    PlatformServices {
        activity: Arc::new(frontmost::X11ActivitySource::new(apps.clone())),
        urls: Arc::new(browser::TitleUrlResolver),
        idle: Arc::new(idle::X11IdleDetector),
        capturer: Arc::new(capture::X11ScreenCapturer),
        permissions: Arc::new(permissions::LinuxPermissions),
        secrets: Arc::new(secrets::secret_store(data_dir)),
        notifier: Arc::new(crate::notify::LogNotifier),
        update_feed: Arc::new(crate::update_feed::HttpUpdateFeed::new()),
        apps,
        enforcer: Arc::new(enforce::LinuxEnforcer),
    }
}
