//! Permissions on Linux: X11 has none of the macOS grants (any client of the display
//! reads windows, titles and pixels), so every kind is `NotApplicable` and requesting one
//! is a no-op. Notifications go through the desktop's notification daemon without consent.

use ubiqx_core::ports::{PermissionChecker, PermissionKind, PermissionState, PermissionStatus};
use ubiqx_core::CoreResult;

#[derive(Debug, Default, Clone, Copy)]
pub struct LinuxPermissions;

impl PermissionChecker for LinuxPermissions {
    fn status(&self) -> PermissionStatus {
        PermissionStatus {
            screen_recording: PermissionState::NotApplicable,
            automation: PermissionState::NotApplicable,
            accessibility: PermissionState::NotApplicable,
        }
    }

    fn request(&self, _kind: PermissionKind) -> CoreResult<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_applies() {
        let s = LinuxPermissions.status();
        assert_eq!(s.screen_recording, PermissionState::NotApplicable);
        assert_eq!(s.automation, PermissionState::NotApplicable);
        assert_eq!(s.accessibility, PermissionState::NotApplicable);
        assert!(LinuxPermissions.request(PermissionKind::Automation).is_ok());
    }
}
