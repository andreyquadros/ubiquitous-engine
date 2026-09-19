//! What the focus guard does to a distraction on macOS: quits an application through
//! `NSRunningApplication`, closes or blanks a browser tab and hides other windows through
//! AppleScript, and runs a Shortcut through the `shortcuts` command.
//!
//! Permissions: tab actions need the same Automation grant as reading the browser URL;
//! `hide_others` sends Apple events to System Events, which raises its own Automation
//! consent alert the first time (documented in the focus page's options).

use std::process::{Command, Stdio};
use std::time::Duration;

use objc2_app_kit::NSRunningApplication;
use objc2_foundation::NSString;
use ubiqx_core::ports::Enforcer;
use ubiqx_core::{CoreError, CoreResult};

use crate::browser::{run_osascript, script_for};

/// Budget for one tab or window script. Tab scripts talk to a browser we already have a
/// grant for; the System Events script may show a consent alert once, hence the longer wait.
const TAB_TIMEOUT: Duration = Duration::from_secs(5);
const HIDE_TIMEOUT: Duration = Duration::from_secs(60);

/// Bundle id of ubiqX: never hidden with the others.
const SELF_BUNDLE_ID: &str = ubiqx_core::focus::SELF_BUNDLE_ID;

/// How a browser exposes its tabs to AppleScript.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserFamily {
    /// Safari and WebKit browsers: `current tab of front window`.
    WebKit,
    /// Chrome, Chromium, Brave, Edge, Arc, Vivaldi, Opera: `active tab of front window`.
    Chromium,
}

/// The scripting family of a supported browser, from the same list the URL resolver uses.
pub fn browser_family(bundle_id: &str) -> Option<BrowserFamily> {
    let script = script_for(bundle_id)?;
    if script.contains("front document") {
        Some(BrowserFamily::WebKit)
    } else {
        Some(BrowserFamily::Chromium)
    }
}

/// AppleScript that closes the active tab of the browser's front window.
pub fn close_tab_script(bundle_id: &str) -> Option<String> {
    let id = bundle_id.trim();
    Some(match browser_family(id)? {
        BrowserFamily::WebKit => format!(
            "tell application id \"{id}\" to if (count of windows) > 0 then close current tab of front window"
        ),
        BrowserFamily::Chromium => format!(
            "tell application id \"{id}\" to if (count of windows) > 0 then close active tab of front window"
        ),
    })
}

/// AppleScript that sends the active tab of the browser's front window to `about:blank`.
pub fn blank_tab_script(bundle_id: &str) -> Option<String> {
    let id = bundle_id.trim();
    Some(match browser_family(id)? {
        BrowserFamily::WebKit => format!(
            "tell application id \"{id}\" to if (count of windows) > 0 then set URL of current tab of front window to \"about:blank\""
        ),
        BrowserFamily::Chromium => format!(
            "tell application id \"{id}\" to if (count of windows) > 0 then set URL of active tab of front window to \"about:blank\""
        ),
    })
}

/// AppleScript that hides every visible process except `keep_bundle_id`, ubiqX and (when
/// known) the front app's process name, through System Events.
pub fn hide_others_script(keep_bundle_id: &str, keep_name: Option<&str>) -> String {
    let keep = escape(keep_bundle_id.trim());
    let mut conditions = format!(
        "visible is true and bundle identifier is not \"{keep}\" and bundle identifier is not \"{SELF_BUNDLE_ID}\" and name is not \"ubiqX\""
    );
    if let Some(name) = keep_name.map(str::trim).filter(|n| !n.is_empty()) {
        conditions.push_str(&format!(" and name is not \"{}\"", escape(name)));
    }
    format!(
        "tell application \"System Events\" to set visible of every process whose {conditions} to false"
    )
}

/// Escapes a string for use inside an AppleScript string literal.
fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Runs a script and reports whether it succeeded; failures are logged, never returned.
fn run_quietly(what: &str, script: &str, timeout: Duration) -> CoreResult<bool> {
    match run_osascript(script, timeout) {
        Ok((true, _, _)) => Ok(true),
        Ok((false, _, err)) => {
            tracing::debug!(what, stderr = %err.trim(), "osascript failed");
            Ok(false)
        }
        Err(e) => {
            tracing::debug!(what, error = %e, "osascript did not run");
            Ok(false)
        }
    }
}

/// The running application(s) with this bundle id.
fn running_apps(bundle_id: &str) -> Vec<objc2::rc::Retained<NSRunningApplication>> {
    NSRunningApplication::runningApplicationsWithBundleIdentifier(&NSString::from_str(bundle_id))
        .iter()
        .collect()
}

/// [`Enforcer`] for macOS.
#[derive(Debug, Default, Clone, Copy)]
pub struct MacEnforcer;

impl Enforcer for MacEnforcer {
    fn quit_app(&self, bundle_id: &str) -> CoreResult<bool> {
        let id = bundle_id.trim();
        if id.is_empty() || id.eq_ignore_ascii_case(SELF_BUNDLE_ID) {
            return Ok(false);
        }
        let apps = running_apps(id);
        if apps.is_empty() {
            return Ok(false);
        }
        // `terminate` is the graceful quit (the app may show "save changes?" and refuse).
        let mut any = false;
        for app in apps {
            any |= app.terminate();
        }
        tracing::info!(app = %id, quit = any, "focus guard asked an app to quit");
        Ok(any)
    }

    fn close_active_tab(&self, browser_bundle_id: &str) -> CoreResult<bool> {
        let Some(script) = close_tab_script(browser_bundle_id) else {
            return Ok(false);
        };
        run_quietly("close tab", &script, TAB_TIMEOUT)
    }

    fn blank_active_tab(&self, browser_bundle_id: &str) -> CoreResult<bool> {
        let Some(script) = blank_tab_script(browser_bundle_id) else {
            return Ok(false);
        };
        run_quietly("blank tab", &script, TAB_TIMEOUT)
    }

    fn hide_others(&self, keep_bundle_id: &str) -> CoreResult<bool> {
        let keep_name = running_apps(keep_bundle_id)
            .first()
            .and_then(|a| a.localizedName())
            .map(|n| n.to_string());
        let script = hide_others_script(keep_bundle_id, keep_name.as_deref());
        run_quietly("hide others", &script, HIDE_TIMEOUT)
    }

    fn run_shortcut(&self, name: &str) -> CoreResult<bool> {
        let name = name.trim();
        if name.is_empty() {
            return Ok(false);
        }
        // Spawned, never awaited here: a Shortcut may take a while (or ask for its own
        // permission), and the session must start right away. Its exit status is logged.
        let child = Command::new("/usr/bin/shortcuts")
            .args(["run", name])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| CoreError::Platform(format!("shortcuts run: {e}")))?;
        let name = name.to_string();
        std::thread::Builder::new()
            .name("ubiqx-shortcut".into())
            .spawn(move || match child.wait_with_output() {
                Ok(out) if out.status.success() => {
                    tracing::info!(shortcut = %name, "shortcut ran");
                }
                Ok(out) => tracing::warn!(
                    shortcut = %name,
                    status = %out.status,
                    stderr = %String::from_utf8_lossy(&out.stderr).trim(),
                    "shortcut failed"
                ),
                Err(e) => tracing::warn!(shortcut = %name, error = %e, "shortcut did not finish"),
            })
            .map_err(|e| CoreError::Platform(format!("shortcut watcher: {e}")))?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scripts_per_family() {
        assert_eq!(
            browser_family("com.apple.Safari"),
            Some(BrowserFamily::WebKit)
        );
        assert_eq!(
            browser_family("com.google.Chrome"),
            Some(BrowserFamily::Chromium)
        );
        assert_eq!(browser_family("org.mozilla.firefox"), None);
        assert!(close_tab_script("com.apple.Safari")
            .unwrap()
            .contains("close current tab of front window"));
        assert!(close_tab_script("com.brave.Browser")
            .unwrap()
            .contains("close active tab of front window"));
        assert!(blank_tab_script("com.google.Chrome")
            .unwrap()
            .contains("set URL of active tab of front window to \"about:blank\""));
        assert!(close_tab_script("org.mozilla.firefox").is_none());
    }

    #[test]
    fn hide_script_keeps_front_app_and_self() {
        let s = hide_others_script("com.microsoft.VSCode", Some("Code \"x\""));
        assert!(s.starts_with("tell application \"System Events\" to set visible of every process whose visible is true"));
        assert!(s.contains("bundle identifier is not \"com.microsoft.VSCode\""));
        assert!(s.contains("bundle identifier is not \"ai.ubiqx.app\""));
        assert!(s.contains("name is not \"ubiqX\""));
        assert!(s.contains("name is not \"Code \\\"x\\\"\""));
        assert!(s.ends_with("to false"));
        assert!(!hide_others_script("x", Some("  ")).contains("name is not \"  \""));
    }
}
