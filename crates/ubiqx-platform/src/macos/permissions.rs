//! TCC permission checks and requests.

use objc2_core_graphics::{CGPreflightScreenCaptureAccess, CGRequestScreenCaptureAccess};
use ubiqx_core::ports::{PermissionChecker, PermissionKind, PermissionState, PermissionStatus};
use ubiqx_core::CoreResult;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> bool;
}

pub fn screen_recording_granted() -> bool {
    CGPreflightScreenCaptureAccess()
}

pub fn accessibility_granted() -> bool {
    // SAFETY: plain ApplicationServices query without side effects.
    unsafe { AXIsProcessTrusted() }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct MacPermissions;

impl PermissionChecker for MacPermissions {
    fn status(&self) -> PermissionStatus {
        PermissionStatus {
            screen_recording: if screen_recording_granted() {
                PermissionState::Granted
            } else {
                PermissionState::Denied
            },
            // Automation is per target application and only knowable by trying.
            automation: PermissionState::Unknown,
            accessibility: if accessibility_granted() {
                PermissionState::Granted
            } else {
                PermissionState::Denied
            },
        }
    }

    fn request(&self, kind: PermissionKind) -> CoreResult<()> {
        match kind {
            PermissionKind::ScreenRecording => {
                // Shows the system prompt once; afterwards it opens nothing, so also point the
                // user at the settings pane.
                let granted = CGRequestScreenCaptureAccess();
                if !granted {
                    open_settings_pane("Privacy_ScreenCapture");
                }
                Ok(())
            }
            PermissionKind::Accessibility => {
                open_settings_pane("Privacy_Accessibility");
                Ok(())
            }
            PermissionKind::Automation => {
                // Trigger the Apple Events prompt for every supported browser that is running
                // by asking each for its name.
                for id in crate::browser::supported_browsers() {
                    let script = format!(
                        "tell application \"System Events\" to if exists (application process 1 whose bundle identifier is \"{id}\") then tell application id \"{id}\" to get name"
                    );
                    let _ =
                        crate::browser::run_osascript(&script, std::time::Duration::from_secs(5));
                }
                Ok(())
            }
        }
    }
}

fn open_settings_pane(pane: &str) {
    let url = format!("x-apple.systempreferences:com.apple.preference.security?{pane}");
    if let Err(e) = std::process::Command::new("/usr/bin/open")
        .arg(&url)
        .spawn()
    {
        tracing::warn!(error = %e, "could not open System Settings");
    }
}
