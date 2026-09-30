//! Seconds since the last input event through the XScreenSaver extension.

use ubiqx_core::ports::IdleDetector;
use ubiqx_core::CoreResult;

use super::x11::X11;

#[derive(Debug, Default, Clone, Copy)]
pub struct X11IdleDetector;

impl IdleDetector for X11IdleDetector {
    /// `Ok(None)` without a display or when the extension is missing: the tracker then
    /// treats the user as active.
    fn idle_secs(&self) -> CoreResult<Option<f64>> {
        Ok(X11::connect()
            .and_then(|x| x.idle_ms())
            .map(|ms| f64::from(ms) / 1000.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn never_errors_without_a_display() {
        if !super::super::x11::display_available() {
            assert_eq!(X11IdleDetector.idle_secs().unwrap(), None);
        }
    }
}
