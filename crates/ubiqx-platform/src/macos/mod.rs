//! macOS adapters. Every public item here maps one-to-one onto a port in `ubiqx_core::ports`.
//!
//! Permission model (macOS 13+):
//! * Frontmost application (bundle id, name, pid): **no permission** (`NSWorkspace`).
//! * Window titles and window ids: **Screen Recording** (`CGWindowListCopyWindowInfo` only
//!   returns `kCGWindowName` when granted).
//! * Screenshots: **Screen Recording**.
//! * Browser URLs: **Automation** (Apple Events) for each browser, prompted on first use.
//! * Idle time: no permission.

use std::sync::Arc;

use crate::PlatformServices;

pub mod capture;
pub mod frontmost;
pub mod idle;
pub mod keychain;
pub mod permissions;

/// Builds the real macOS services.
pub fn services() -> PlatformServices {
    PlatformServices {
        activity: Arc::new(frontmost::AppKitActivitySource),
        urls: Arc::new(crate::browser::OsascriptUrlResolver::default()),
        idle: Arc::new(idle::CgIdleDetector),
        capturer: Arc::new(capture::CgScreenCapturer),
        permissions: Arc::new(permissions::MacPermissions),
        secrets: Arc::new(keychain::KeychainSecretStore::default()),
        notifier: Arc::new(crate::notify::LogNotifier),
        update_feed: Arc::new(crate::update_feed::HttpUpdateFeed::new()),
    }
}
