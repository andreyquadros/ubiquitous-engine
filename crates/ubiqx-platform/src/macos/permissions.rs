//! TCC permission checks and requests.

use std::ffi::c_void;
use std::ptr;

use objc2_app_kit::NSWorkspace;
use objc2_core_graphics::{CGPreflightScreenCaptureAccess, CGRequestScreenCaptureAccess};
use ubiqx_core::ports::{PermissionChecker, PermissionKind, PermissionState, PermissionStatus};
use ubiqx_core::{CoreError, CoreResult};

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> bool;
}

/// Apple Events descriptor (`AEDesc`): a four-char type code plus an opaque data handle.
#[repr(C)]
struct AEDesc {
    descriptor_type: u32,
    data_handle: *mut c_void,
}

#[link(name = "CoreServices", kind = "framework")]
extern "C" {
    fn AECreateDesc(type_code: u32, data: *const c_void, size: isize, result: *mut AEDesc) -> i16;
    fn AEDisposeDesc(desc: *mut AEDesc) -> i16;
    /// macOS 10.14+: asks TCC whether this process may send Apple events to `target`. With
    /// `ask_user_if_needed` set it shows the consent alert and blocks until it is answered.
    fn AEDeterminePermissionToAutomateTarget(
        target: *const AEDesc,
        event_class: u32,
        event_id: u32,
        ask_user_if_needed: u8,
    ) -> i32;
}

/// `typeApplicationBundleID`.
const TYPE_APPLICATION_BUNDLE_ID: u32 = u32::from_be_bytes(*b"bund");
/// `typeWildCard`: any event class / id.
const TYPE_WILD_CARD: u32 = u32::from_be_bytes(*b"****");
/// `errAEEventNotPermitted`: the user (or a profile) refused.
const ERR_AE_EVENT_NOT_PERMITTED: i32 = -1743;
/// `errAEEventWouldRequireUserConsent`: never asked; only returned when not asking.
const ERR_AE_EVENT_WOULD_REQUIRE_USER_CONSENT: i32 = -1744;
/// `procNotFound`: the target application is not running.
const PROC_NOT_FOUND: i32 = -600;

pub fn screen_recording_granted() -> bool {
    CGPreflightScreenCaptureAccess()
}

pub fn accessibility_granted() -> bool {
    // SAFETY: plain ApplicationServices query without side effects.
    unsafe { AXIsProcessTrusted() }
}

/// Result of one Automation (Apple events) permission check against a browser.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutomationCheck {
    Granted,
    Denied,
    /// The consent alert was never shown (or was dismissed without an answer).
    NotAsked,
    NotRunning,
    /// Unexpected `OSStatus`.
    Error(i32),
}

/// Asks TCC about sending Apple events to `bundle_id`. With `ask_user` the system alert is
/// shown when needed and the call blocks until the user answers it: call off the main thread.
fn automation_permission(bundle_id: &str, ask_user: bool) -> AutomationCheck {
    let mut desc = AEDesc {
        descriptor_type: 0,
        data_handle: ptr::null_mut(),
    };
    let bytes = bundle_id.as_bytes();
    // SAFETY: `desc` is a valid out-pointer; `AECreateDesc` copies `bytes`, which outlive the call.
    let err = unsafe {
        AECreateDesc(
            TYPE_APPLICATION_BUNDLE_ID,
            bytes.as_ptr().cast(),
            bytes.len() as isize,
            &mut desc,
        )
    };
    if err != 0 {
        return AutomationCheck::Error(i32::from(err));
    }
    // SAFETY: `desc` was initialised by `AECreateDesc` above and is disposed exactly once.
    let status = unsafe {
        let status = AEDeterminePermissionToAutomateTarget(
            &desc,
            TYPE_WILD_CARD,
            TYPE_WILD_CARD,
            u8::from(ask_user),
        );
        AEDisposeDesc(&mut desc);
        status
    };
    match status {
        0 => AutomationCheck::Granted,
        ERR_AE_EVENT_NOT_PERMITTED => AutomationCheck::Denied,
        ERR_AE_EVENT_WOULD_REQUIRE_USER_CONSENT => AutomationCheck::NotAsked,
        PROC_NOT_FOUND => AutomationCheck::NotRunning,
        other => AutomationCheck::Error(other),
    }
}

/// Fallback when the AE query itself fails: sending any Apple event to the browser also raises
/// the consent alert (the send blocks inside `osascript` until the user answers).
fn prompt_via_osascript(bundle_id: &str) -> AutomationCheck {
    let script = format!("tell application id \"{bundle_id}\" to get name");
    match crate::browser::run_osascript(&script, crate::browser::FIRST_CONTACT_TIMEOUT) {
        Ok((true, _, _)) => AutomationCheck::Granted,
        Ok((false, _, err)) if crate::browser::is_automation_denied(&err) => {
            AutomationCheck::Denied
        }
        Ok((false, _, err)) => {
            tracing::debug!(app = %bundle_id, stderr = %err.trim(), "osascript failed");
            AutomationCheck::NotAsked
        }
        Err(e) => {
            tracing::warn!(app = %bundle_id, error = %e, "automation prompt failed");
            AutomationCheck::NotAsked
        }
    }
}

/// Supported browsers that are running right now, from AppKit (no TCC involved) instead of
/// System Events, which would raise its own, unrelated Automation prompt.
fn running_browsers() -> Vec<&'static str> {
    let running: Vec<String> = NSWorkspace::sharedWorkspace()
        .runningApplications()
        .iter()
        .filter_map(|a| a.bundleIdentifier().map(|s| s.to_string()))
        .collect();
    crate::browser::supported_browsers()
        .iter()
        .copied()
        .filter(|id| running.iter().any(|r| r == id))
        .collect()
}

/// Folds per-browser checks into the one state the UI shows: granted as soon as a running
/// browser allows us, denied when every answered browser refused, unknown otherwise (no
/// supported browser running, or the alert never answered).
pub fn automation_state(checks: &[AutomationCheck]) -> PermissionState {
    let answered = checks
        .iter()
        .filter(|c| matches!(c, AutomationCheck::Granted | AutomationCheck::Denied));
    let mut any_answered = false;
    for check in answered {
        if *check == AutomationCheck::Granted {
            return PermissionState::Granted;
        }
        any_answered = true;
    }
    if any_answered {
        PermissionState::Denied
    } else {
        PermissionState::Unknown
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct MacPermissions;

impl PermissionChecker for MacPermissions {
    fn status(&self) -> PermissionStatus {
        let checks: Vec<AutomationCheck> = running_browsers()
            .into_iter()
            .map(|id| automation_permission(id, false))
            .collect();
        PermissionStatus {
            screen_recording: if screen_recording_granted() {
                PermissionState::Granted
            } else {
                PermissionState::Denied
            },
            // Automation is per target application: only the browsers running now can be asked.
            automation: automation_state(&checks),
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
                let running = running_browsers();
                if running.is_empty() {
                    return Err(CoreError::Permission(
                        "Nenhum navegador compatível está aberto. Abra o Safari, Chrome, Arc, \
                         Brave, Edge, Vivaldi ou Opera e clique em Solicitar de novo."
                            .into(),
                    ));
                }
                let mut denied = Vec::new();
                for id in running {
                    // The system shows its consent alert; this blocks until it is answered.
                    let check = match automation_permission(id, true) {
                        AutomationCheck::Error(code) => {
                            tracing::warn!(app = %id, code, "AE permission query failed; using osascript");
                            prompt_via_osascript(id)
                        }
                        other => other,
                    };
                    tracing::info!(app = %id, ?check, "automation permission");
                    if check == AutomationCheck::Denied {
                        denied.push(id);
                    }
                }
                if denied.is_empty() {
                    Ok(())
                } else {
                    Err(CoreError::Permission(format!(
                        "Automação negada para {}. Em Ajustes → Privacidade e Segurança → \
                         Automação, marque o navegador embaixo de ubiqX.",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn automation_state_folds_per_browser_checks() {
        use AutomationCheck::*;
        assert_eq!(automation_state(&[]), PermissionState::Unknown);
        assert_eq!(automation_state(&[NotAsked]), PermissionState::Unknown);
        assert_eq!(
            automation_state(&[NotRunning, Error(-1)]),
            PermissionState::Unknown
        );
        assert_eq!(automation_state(&[Denied]), PermissionState::Denied);
        assert_eq!(
            automation_state(&[Denied, NotAsked]),
            PermissionState::Denied
        );
        assert_eq!(
            automation_state(&[Denied, Granted]),
            PermissionState::Granted
        );
        assert_eq!(
            automation_state(&[NotAsked, Granted]),
            PermissionState::Granted
        );
    }
}
