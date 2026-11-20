//! Seconds since the last keyboard/mouse event. No permission required.

use objc2_core_graphics::{CGEventSource, CGEventSourceStateID, CGEventType};
use ubiqx_core::ports::IdleDetector;
use ubiqx_core::CoreResult;

/// `kCGAnyInputEventType` is defined as `((CGEventType)(~0))`.
const ANY_INPUT_EVENT: CGEventType = CGEventType(u32::MAX);

pub fn idle_seconds() -> f64 {
    CGEventSource::seconds_since_last_event_type(
        CGEventSourceStateID::CombinedSessionState,
        ANY_INPUT_EVENT,
    )
}

#[derive(Debug, Default, Clone, Copy)]
pub struct CgIdleDetector;

impl IdleDetector for CgIdleDetector {
    fn idle_secs(&self) -> CoreResult<Option<f64>> {
        let s = idle_seconds();
        Ok(if s.is_finite() && s >= 0.0 {
            Some(s)
        } else {
            None
        })
    }
}
