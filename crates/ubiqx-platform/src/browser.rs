//! Browser URL resolution through AppleScript (`osascript`).
//!
//! Only Safari and Chromium-based browsers expose the active tab URL to AppleScript; Firefox
//! does not, so it reports `supports() == false` and the tracker keeps the window title only.
//! Everything except the actual `osascript` spawn is pure and unit-tested here.

use std::collections::HashMap;
use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use ubiqx_core::ports::BrowserUrlResolver;
use ubiqx_core::{CoreError, CoreResult, ForegroundWindow};

/// AppleScript that returns the URL of the active tab for a browser bundle id.
pub fn script_for(app_id: &str) -> Option<String> {
    let id = app_id.trim();
    let safari = matches!(id, "com.apple.Safari" | "com.apple.SafariTechnologyPreview");
    let chromium = matches!(
        id,
        "com.google.Chrome"
            | "com.google.Chrome.canary"
            | "com.google.Chrome.beta"
            | "org.chromium.Chromium"
            | "com.brave.Browser"
            | "com.brave.Browser.beta"
            | "com.microsoft.edgemac"
            | "com.microsoft.edgemac.Beta"
            | "company.thebrowser.Browser"
            | "com.vivaldi.Vivaldi"
            | "com.operasoftware.Opera"
            | "com.operasoftware.OperaGX"
    );
    if safari {
        Some(format!(
            "tell application id \"{id}\" to if (count of windows) > 0 then return URL of front document"
        ))
    } else if chromium {
        Some(format!(
            "tell application id \"{id}\" to if (count of windows) > 0 then return URL of active tab of front window"
        ))
    } else {
        None
    }
}

/// Bundle ids of every browser we know how to query.
pub fn supported_browsers() -> &'static [&'static str] {
    &[
        "com.apple.Safari",
        "com.google.Chrome",
        "org.chromium.Chromium",
        "com.brave.Browser",
        "com.microsoft.edgemac",
        "company.thebrowser.Browser",
        "com.vivaldi.Vivaldi",
        "com.operasoftware.Opera",
    ]
}

/// Normalises `osascript` output into a URL, or `None` for empty / non-http output.
pub fn parse_url_output(stdout: &str) -> Option<String> {
    let s = stdout.trim().trim_matches('"').trim();
    if s.starts_with("http://") || s.starts_with("https://") || s.starts_with("file://") {
        Some(s.to_string())
    } else {
        None
    }
}

/// Whether an `osascript` failure means the Automation permission was denied.
pub fn is_automation_denied(stderr: &str) -> bool {
    stderr.contains("-1743") || stderr.to_lowercase().contains("not authorized")
}

/// Runs a short AppleScript with a timeout. Blocking; call from the sampler thread only.
pub fn run_osascript(script: &str, timeout: Duration) -> CoreResult<(bool, String, String)> {
    let mut child = Command::new("/usr/bin/osascript")
        .arg("-e")
        .arg(script)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| CoreError::Platform(format!("osascript spawn: {e}")))?;
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let mut out = String::new();
                let mut err = String::new();
                if let Some(mut o) = child.stdout.take() {
                    let _ = o.read_to_string(&mut out);
                }
                if let Some(mut e) = child.stderr.take() {
                    let _ = e.read_to_string(&mut err);
                }
                return Ok((status.success(), out, err));
            }
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(CoreError::Platform("osascript timed out".into()));
                }
                std::thread::sleep(Duration::from_millis(25));
            }
            Err(e) => return Err(CoreError::Platform(format!("osascript wait: {e}"))),
        }
    }
}

/// How long a denied browser is left alone before we try again.
const DENIED_BACKOFF: Duration = Duration::from_secs(30 * 60);
/// How long a resolved (app, title) pair is reused without re-querying.
const CACHE_TTL: Duration = Duration::from_secs(20);

/// `BrowserUrlResolver` backed by `osascript`. Throttles calls per (app, title), remembers
/// Automation denials and never blocks longer than `timeout`.
pub struct OsascriptUrlResolver {
    timeout: Duration,
    cache: Mutex<HashMap<CacheKey, CacheEntry>>,
    denied: Mutex<HashMap<String, Instant>>,
}

/// `(app_id, window_title)`.
type CacheKey = (String, String);
/// When the URL was resolved and what it was.
type CacheEntry = (Instant, Option<String>);

impl Default for OsascriptUrlResolver {
    fn default() -> Self {
        Self::new(Duration::from_millis(1500))
    }
}

impl OsascriptUrlResolver {
    pub fn new(timeout: Duration) -> Self {
        Self {
            timeout,
            cache: Mutex::new(HashMap::new()),
            denied: Mutex::new(HashMap::new()),
        }
    }

    /// Browsers whose Automation permission was denied recently.
    pub fn denied_apps(&self) -> Vec<String> {
        let now = Instant::now();
        self.denied
            .lock()
            .iter()
            .filter(|(_, at)| now.duration_since(**at) < DENIED_BACKOFF)
            .map(|(k, _)| k.clone())
            .collect()
    }

    /// Forgets a denial (the user re-enabled Automation in System Settings).
    pub fn clear_denial(&self, app_id: &str) {
        self.denied.lock().remove(app_id);
    }
}

impl BrowserUrlResolver for OsascriptUrlResolver {
    fn supports(&self, app_id: &str) -> bool {
        script_for(app_id).is_some()
    }

    fn resolve(&self, window: &ForegroundWindow) -> CoreResult<Option<String>> {
        let Some(script) = script_for(&window.app_id) else {
            return Ok(None);
        };
        if let Some(at) = self.denied.lock().get(&window.app_id) {
            if at.elapsed() < DENIED_BACKOFF {
                return Ok(None);
            }
        }
        let key = (window.app_id.clone(), window.window_title.clone());
        if let Some((at, url)) = self.cache.lock().get(&key) {
            if at.elapsed() < CACHE_TTL {
                return Ok(url.clone());
            }
        }
        let (ok, out, err) = run_osascript(&script, self.timeout)?;
        if !ok {
            if is_automation_denied(&err) {
                tracing::warn!(app = %window.app_id, "browser automation denied; backing off");
                self.denied
                    .lock()
                    .insert(window.app_id.clone(), Instant::now());
                return Err(CoreError::Permission(format!(
                    "automation denied for {}",
                    window.app_id
                )));
            }
            tracing::debug!(app = %window.app_id, stderr = %err.trim(), "osascript failed");
            return Ok(None);
        }
        let url = parse_url_output(&out);
        let mut cache = self.cache.lock();
        if cache.len() > 256 {
            cache.clear();
        }
        cache.insert(key, (Instant::now(), url.clone()));
        Ok(url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scripts_per_browser() {
        assert!(script_for("com.apple.Safari")
            .unwrap()
            .contains("front document"));
        assert!(script_for("com.google.Chrome")
            .unwrap()
            .contains("active tab"));
        assert!(script_for("company.thebrowser.Browser").is_some());
        assert!(script_for("org.mozilla.firefox").is_none());
        assert!(script_for("").is_none());
    }

    #[test]
    fn parses_output() {
        assert_eq!(
            parse_url_output("https://a.b/c\n").as_deref(),
            Some("https://a.b/c")
        );
        assert_eq!(
            parse_url_output("\"https://a.b\""),
            Some("https://a.b".into())
        );
        assert_eq!(parse_url_output("missing value"), None);
        assert_eq!(parse_url_output(""), None);
    }

    #[test]
    fn denial_detection() {
        assert!(is_automation_denied(
            "execution error: Not authorized to send Apple events to Safari. (-1743)"
        ));
        assert!(!is_automation_denied(
            "execution error: Safari got an error: Can’t get window 1. (-1728)"
        ));
    }
}
