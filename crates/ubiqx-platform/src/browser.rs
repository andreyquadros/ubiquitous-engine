//! Browser URL resolution through AppleScript (`osascript`).
//!
//! Only Safari and Chromium-based browsers expose the active tab URL to AppleScript; Firefox
//! does not, so it reports `supports() == false` and the tracker keeps the window title only.
//! Everything except the actual `osascript` spawn is pure and unit-tested here.

use std::collections::{HashMap, HashSet};
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
/// Budget for the first `osascript` call per browser: it blocks inside the Apple-events send
/// while the system shows the Automation consent alert, which the user needs time to read
/// and answer. Killing it early tears the alert down before it can be answered and records
/// neither a URL nor a denial.
pub const FIRST_CONTACT_TIMEOUT: Duration = Duration::from_secs(120);

/// `BrowserUrlResolver` backed by `osascript`. Throttles calls per (app, title), remembers
/// Automation denials and never blocks longer than `timeout`, except for the first completed
/// call per browser (see [`FIRST_CONTACT_TIMEOUT`]).
pub struct OsascriptUrlResolver {
    timeout: Duration,
    cache: Mutex<HashMap<CacheKey, CacheEntry>>,
    denied: Mutex<HashMap<String, Instant>>,
    /// Browsers for which at least one `osascript` run has *completed* (any exit status). A
    /// timed-out first contact is not recorded, so the next call is again a long attempt.
    contacted: Mutex<HashSet<String>>,
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
            contacted: Mutex::new(HashSet::new()),
        }
    }

    /// Timeout of one `osascript` run for `app_id`: the long first-contact budget until a
    /// run has completed for that browser, the configured short budget afterwards.
    pub fn budget_for(&self, app_id: &str) -> Duration {
        if self.contacted.lock().contains(app_id) {
            self.timeout
        } else {
            FIRST_CONTACT_TIMEOUT
        }
    }

    /// Records that an `osascript` run for `app_id` completed (with any exit status).
    fn mark_contacted(&self, app_id: &str) {
        self.contacted.lock().insert(app_id.to_string());
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
        self.resolve_with(window, true)
    }

    /// Asks the browser now, whatever the cache holds: the answer decides whether a tab is
    /// closed, and a 20-second-old URL may belong to a page the user already left. The
    /// denial back-off still applies, and the fresh answer replaces the cached one.
    fn resolve_fresh(&self, window: &ForegroundWindow) -> CoreResult<Option<String>> {
        self.resolve_with(window, false)
    }
}

impl OsascriptUrlResolver {
    /// The cached answer for `key` when `use_cache` allows one and it is young enough.
    fn cached(&self, key: &CacheKey, use_cache: bool) -> Option<Option<String>> {
        if !use_cache {
            return None;
        }
        let cache = self.cache.lock();
        let (at, url) = cache.get(key)?;
        (at.elapsed() < CACHE_TTL).then(|| url.clone())
    }

    fn resolve_with(
        &self,
        window: &ForegroundWindow,
        use_cache: bool,
    ) -> CoreResult<Option<String>> {
        let Some(script) = script_for(&window.app_id) else {
            return Ok(None);
        };
        if let Some(at) = self.denied.lock().get(&window.app_id) {
            if at.elapsed() < DENIED_BACKOFF {
                return Ok(None);
            }
        }
        let key = (window.app_id.clone(), window.window_title.clone());
        if let Some(url) = self.cached(&key, use_cache) {
            return Ok(url);
        }
        // On `Err` (timeout) the browser stays uncontacted, so the next call is again a
        // long first-contact attempt rather than a 1.5 s one that kills the consent alert.
        let (ok, out, err) = run_osascript(&script, self.budget_for(&window.app_id))?;
        self.mark_contacted(&window.app_id);
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
    fn first_contact_gets_the_long_budget_then_the_short_one() {
        let short = Duration::from_millis(1500);
        let r = OsascriptUrlResolver::new(short);
        assert_eq!(r.budget_for("com.apple.Safari"), FIRST_CONTACT_TIMEOUT);
        r.mark_contacted("com.apple.Safari");
        assert_eq!(r.budget_for("com.apple.Safari"), short);
        // Per browser: Chrome has not been contacted yet.
        assert_eq!(r.budget_for("com.google.Chrome"), FIRST_CONTACT_TIMEOUT);
    }

    fn chrome_window(title: &str) -> ForegroundWindow {
        ForegroundWindow {
            app_name: "Google Chrome".into(),
            app_id: "com.google.Chrome".into(),
            window_title: title.into(),
            pid: Some(1),
            window_id: Some(2),
            bounds: None,
        }
    }

    #[test]
    fn resolve_fresh_bypasses_the_cache() {
        let r = OsascriptUrlResolver::default();
        let key = ("com.google.Chrome".to_string(), "Page".to_string());
        r.cache.lock().insert(
            key.clone(),
            (Instant::now(), Some("https://stale.example/".into())),
        );
        // The cache answers `resolve`: no process is started for it.
        assert_eq!(
            r.resolve(&chrome_window("Page")).unwrap().as_deref(),
            Some("https://stale.example/")
        );
        assert_eq!(
            r.cached(&key, true),
            Some(Some("https://stale.example/".to_string()))
        );
        // The fresh read is never answered from it, however young the entry is.
        assert_eq!(r.cached(&key, false), None);
        // An entry past its time is not served to either.
        r.cache.lock().insert(
            key.clone(),
            (
                Instant::now() - CACHE_TTL - Duration::from_secs(1),
                Some("https://stale.example/".into()),
            ),
        );
        assert_eq!(r.cached(&key, true), None);
    }

    #[test]
    fn resolve_fresh_still_honours_a_denial_and_unsupported_apps() {
        let r = OsascriptUrlResolver::default();
        r.denied
            .lock()
            .insert("com.google.Chrome".into(), Instant::now());
        // Backing off: no script is run, so this returns at once on any OS.
        assert_eq!(r.resolve_fresh(&chrome_window("Page")).unwrap(), None);
        let mut firefox = chrome_window("Page");
        firefox.app_id = "org.mozilla.firefox".into();
        assert_eq!(r.resolve_fresh(&firefox).unwrap(), None);
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
