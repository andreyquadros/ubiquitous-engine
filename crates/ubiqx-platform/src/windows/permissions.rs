//! Permissions on Windows: none of the macOS grants apply. Screen content, window titles
//! and the browser's address bar (UI Automation) are readable by any process of the user's
//! session, so every kind reports `NotApplicable` and requesting one is a no-op. The only
//! consent Windows asks for is notifications, which the desktop shell handles.

use ubiqx_core::ports::{PermissionChecker, PermissionKind, PermissionState, PermissionStatus};
use ubiqx_core::CoreResult;

#[derive(Debug, Default, Clone, Copy)]
pub struct WindowsPermissions;

impl PermissionChecker for WindowsPermissions {
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
