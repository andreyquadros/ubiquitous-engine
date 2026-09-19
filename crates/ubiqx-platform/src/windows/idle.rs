//! Milliseconds since the last input event, from `GetLastInputInfo`.

use std::mem::size_of;

use ubiqx_core::ports::IdleDetector;
use ubiqx_core::CoreResult;
use windows::Win32::System::SystemInformation::GetTickCount64;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO};

/// Seconds since the last keyboard or mouse event of this session, or `None` when the
/// query fails (a service session without input).
pub fn idle_seconds() -> Option<f64> {
    let mut info = LASTINPUTINFO {
        cbSize: size_of::<LASTINPUTINFO>() as u32,
        dwTime: 0,
    };
    // SAFETY: `info` is initialised with its size, as the API requires.
    if !unsafe { GetLastInputInfo(&mut info) }.as_bool() {
        return None;
    }
    // `dwTime` is a 32-bit tick count: compare in the same width so wrap-around cancels.
    // SAFETY: no arguments.
    let now = unsafe { GetTickCount64() } as u32;
    let ms = now.wrapping_sub(info.dwTime);
    Some(f64::from(ms) / 1000.0)
}

#[derive(Debug, Default, Clone, Copy)]
pub struct WindowsIdleDetector;

impl IdleDetector for WindowsIdleDetector {
    fn idle_secs(&self) -> CoreResult<Option<f64>> {
        Ok(idle_seconds())
    }
}
