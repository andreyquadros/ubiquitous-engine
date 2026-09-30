//! Browser URL on Linux: title-based, best effort.
//!
//! There is no accessibility route yet (AT-SPI would need the browser's accessibility
//! support switched on, which Chromium only does when a screen reader is detected), so the
//! resolver reads the window title: when it ends with a recognised ` - <Browser>` suffix
//! and contains a bare domain (`docs.rs`, `github.com/x/y`), that domain is the URL.
//! Everything else is `Ok(None)` and the tracker keeps the title. Rules and focus targets
//! by site therefore only fire on pages whose title shows the domain.

use ubiqx_core::ports::BrowserUrlResolver;
use ubiqx_core::{CoreResult, ForegroundWindow};

use crate::browser_url::{url_from_title, LINUX_BROWSERS};

#[derive(Debug, Default, Clone, Copy)]
pub struct TitleUrlResolver;

impl BrowserUrlResolver for TitleUrlResolver {
    fn supports(&self, app_id: &str) -> bool {
        let id = app_id.trim().to_lowercase();
        LINUX_BROWSERS.contains(&id.as_str())
    }

    fn resolve(&self, window: &ForegroundWindow) -> CoreResult<Option<String>> {
        if !self.supports(&window.app_id) {
            return Ok(None);
        }
        Ok(url_from_title(&window.window_title))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn win(app_id: &str, title: &str) -> ForegroundWindow {
        ForegroundWindow {
            app_name: app_id.into(),
            app_id: app_id.into(),
            window_title: title.into(),
            pid: None,
            window_id: None,
            bounds: None,
        }
    }

    #[test]
    fn resolves_from_titles_only() {
        let r = TitleUrlResolver;
        assert!(r.supports("firefox"));
        assert!(r.supports("google-chrome"));
        assert!(!r.supports("code"));
        assert_eq!(
            r.resolve(&win("firefox", "docs.rs — Mozilla Firefox"))
                .unwrap()
                .as_deref(),
            Some("https://docs.rs")
        );
        assert_eq!(
            r.resolve(&win("firefox", "Reading list — Mozilla Firefox"))
                .unwrap(),
            None
        );
        assert_eq!(
            r.resolve(&win("code", "docs.rs — Mozilla Firefox"))
                .unwrap(),
            None
        );
    }
}
