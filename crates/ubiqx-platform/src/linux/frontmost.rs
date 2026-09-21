//! The active window through EWMH (`_NET_ACTIVE_WINDOW`) and its process through `/proc`.

use std::sync::Arc;

use ubiqx_core::ports::ActivitySource;
use ubiqx_core::{CoreResult, ForegroundWindow};

use super::apps::LinuxAppCatalog;
use super::proc::exe_stem_of;
use super::x11::X11;

/// Everything the tracker needs about the active window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveWindow {
    pub window: u32,
    pub pid: Option<u32>,
    pub instance: String,
    pub class: String,
    pub title: String,
    pub bounds: Option<(i32, i32, u32, u32)>,
}

/// Reads the active window; `None` without a display or when nothing is active.
pub fn active_window() -> Option<ActiveWindow> {
    let x = X11::connect()?;
    let window = x.active_window()?;
    let pid = x.window_pid(window);
    let (instance, class) = x.window_class(window).unwrap_or_default();
    Some(ActiveWindow {
        window,
        pid,
        instance,
        class,
        title: x.window_name(window),
        bounds: x.window_rect(window),
    })
}

/// The app id of a window: its process's executable stem, else the `WM_CLASS` instance
/// lower-cased (Flatpak sandboxes hide their pids), else the class.
pub fn app_id_of(pid: Option<u32>, instance: &str, class: &str) -> String {
    pid.and_then(exe_stem_of)
        .filter(|s| !s.is_empty())
        .or_else(|| {
            let i = instance.trim().to_lowercase();
            (!i.is_empty()).then_some(i)
        })
        .unwrap_or_else(|| class.trim().to_lowercase())
}

/// [`ActivitySource`] over X11; names come from the `.desktop` catalogue.
pub struct X11ActivitySource {
    catalog: Arc<LinuxAppCatalog>,
}

impl X11ActivitySource {
    pub fn new(catalog: Arc<LinuxAppCatalog>) -> Self {
        Self { catalog }
    }
}

impl ActivitySource for X11ActivitySource {
    fn foreground(&self) -> CoreResult<Option<ForegroundWindow>> {
        let Some(win) = active_window() else {
            return Ok(None);
        };
        let app_id = app_id_of(win.pid, &win.instance, &win.class);
        if app_id.is_empty() {
            return Ok(None);
        }
        let app_name = self
            .catalog
            .name_of(&app_id)
            .or_else(|| {
                let c = win.class.trim();
                (!c.is_empty()).then(|| c.to_string())
            })
            .unwrap_or_else(|| app_id.clone());
        Ok(Some(ForegroundWindow {
            app_name,
            app_id,
            window_title: win.title,
            pid: win.pid,
            window_id: Some(win.window),
            bounds: win.bounds,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_id_prefers_the_process_then_the_class() {
        let me = std::process::id();
        let mine = exe_stem_of(me).unwrap();
        assert_eq!(app_id_of(Some(me), "Navigator", "Firefox"), mine);
        assert_eq!(app_id_of(None, "Navigator", "Firefox"), "navigator");
        assert_eq!(app_id_of(None, "", "Firefox"), "firefox");
        // A pid nobody has (the kernel never hands out this one) falls back too.
        assert_eq!(app_id_of(Some(u32::MAX), "code", "Code"), "code");
    }

    #[test]
    fn foreground_without_display_is_none() {
        if !super::super::x11::display_available() {
            let source = X11ActivitySource::new(Arc::new(LinuxAppCatalog::default()));
            assert!(source.foreground().unwrap().is_none());
        }
    }
}
