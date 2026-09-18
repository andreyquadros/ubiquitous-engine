//! Windows adapters. Every public item maps one-to-one onto a port in `ubiqx_core::ports`.
//!
//! Identity: an application is its executable's file stem, lower-case (`chrome`, `msedge`,
//! `code`), see [`crate::app_id`]. The human name is the executable's `FileDescription`
//! version resource when it has one.
//!
//! Permission model: none of the macOS grants exist here. The foreground window, its
//! title, the idle time and screenshots are readable by any process of the same session;
//! the browser URL comes from UI Automation, which needs no consent either. Notifications
//! are the only thing the OS may ask about, and Tauri handles that prompt.

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
pub mod secrets;
pub mod util;

/// Builds the real Windows services. `data_dir` hosts the secrets file used when the
/// Credential Manager refuses (see [`crate::secrets_file`]).
pub fn services(data_dir: Option<&Path>) -> PlatformServices {
    let apps = Arc::new(apps::WindowsAppCatalog::default());
    PlatformServices {
        activity: Arc::new(frontmost::WindowsActivitySource),
        urls: Arc::new(browser::UiaUrlResolver::default()),
        idle: Arc::new(idle::WindowsIdleDetector),
        capturer: Arc::new(capture::WindowsScreenCapturer),
        permissions: Arc::new(permissions::WindowsPermissions),
        secrets: Arc::new(secrets::secret_store(data_dir)),
        notifier: Arc::new(crate::notify::LogNotifier),
        update_feed: Arc::new(crate::update_feed::HttpUpdateFeed::new()),
        apps,
        enforcer: Arc::new(enforce::WindowsEnforcer),
    }
}
