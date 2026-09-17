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
                use objc2_app_kit::NSWorkspace;
                // Running browsers, checked from AppKit (no TCC involved) instead of via
                // System Events, which would raise its own, unrelated Automation prompt.
                let running: Vec<String> = NSWorkspace::sharedWorkspace()
                    .runningApplications()
                    .iter()
                    .filter_map(|a| a.bundleIdentifier().map(|s| s.to_string()))
                    .collect();
                let mut denied = Vec::new();
                for id in crate::browser::supported_browsers() {
                    if !running.iter().any(|r| r == id) {
                        continue;
                    }
                    // Trigger the Apple-events consent alert by asking the browser for its
                    // name. The send blocks inside osascript until the user answers, so give
                    // them time to read it.
                    let script = format!("tell application id \"{id}\" to get name");
                    match crate::browser::run_osascript(
                        &script,
                        crate::browser::FIRST_CONTACT_TIMEOUT,
                    ) {
                        Ok((false, _, err)) if crate::browser::is_automation_denied(&err) => {
                            tracing::warn!(app = %id, "automation denied");
                            denied.push(id.to_string());
                        }
                        Ok((false, _, err)) => {
                            tracing::debug!(app = %id, stderr = %err.trim(), "osascript failed")
                        }
                        Err(e) => tracing::warn!(app = %id, error = %e, "automation prompt failed"),
                        Ok(_) => {}
                    }
                }
                if denied.is_empty() {
                    Ok(())
                } else {
                    Err(ubiqx_core::CoreError::Permission(format!(
                        "automation denied for {}",
                        denied.join(", ")
                    )))
                }
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
