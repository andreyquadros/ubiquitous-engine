//! Pure helpers for the Windows and Linux browser URL resolvers: which app ids are browsers,
//! how a value read from the address bar becomes a URL, and the title-based best effort
//! Linux uses while no accessibility route exists there.

use ubiqx_core::normalize::domain_of;

/// Browser app ids (executable stems) on Windows. Chromium-based ones first, then Firefox.
pub const WINDOWS_BROWSERS: &[&str] = &["chrome", "msedge", "brave", "opera", "vivaldi", "firefox"];

/// Browser app ids (executable stems) on Linux: distributions and packaging formats name
/// the same browsers differently.
pub const LINUX_BROWSERS: &[&str] = &[
    "chrome",
    "google-chrome",
    "chromium",
    "chromium-browser",
    "brave",
    "brave-browser",
    "microsoft-edge",
    "msedge",
    "opera",
    "vivaldi",
    "firefox",
    "firefox-esr",
];

/// The Firefox family: their address bar is a different UI Automation element.
pub fn is_firefox(app_id: &str) -> bool {
    matches!(
        app_id.trim().to_lowercase().as_str(),
        "firefox" | "firefox-esr"
    )
}

/// Title suffixes browsers append (` - Google Chrome`), with the display name they belong
/// to. Matching is case-insensitive on the suffix.
pub const TITLE_SUFFIXES: &[&str] = &[
    " - Google Chrome",
    " - Chromium",
    " - Brave",
    " - Microsoft Edge",
    " - Opera",
    " - Vivaldi",
    " — Mozilla Firefox",
    " - Mozilla Firefox",
    " — Mozilla Firefox Private Browsing",
    " - Mozilla Firefox Private Browsing",
];

/// Turns the text of a browser's address bar into a URL. `None` when the field holds
/// something that is not a URL (a search in progress, a placeholder, an empty field).
/// A bare `host/path` gets `https://` prepended; internal pages (`chrome://`, `about:`,
/// `edge://`) are returned as they are.
pub fn normalize_typed_url(value: &str) -> Option<String> {
    let v = value.trim();
    if v.is_empty() || v.contains(char::is_whitespace) {
        return None;
    }
    let lower = v.to_lowercase();
    if lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("file://")
        || lower.starts_with("about:")
        || lower.starts_with("chrome://")
        || lower.starts_with("edge://")
        || lower.starts_with("brave://")
        || lower.starts_with("vivaldi://")
        || lower.starts_with("opera://")
    {
        return Some(v.to_string());
    }
    if lower.contains("://") {
        // Another scheme (ftp, ws, custom): not something we track.
        return None;
    }
    // A bare address must have a host with a dot (or be localhost) before any path.
    let host = v.split(['/', '?', '#']).next().unwrap_or("");
    let host = host.rsplit('@').next().unwrap_or(host);
    let host = host.split(':').next().unwrap_or(host);
    if host.is_empty() || host.starts_with('.') || host.ends_with('.') {
        return None;
    }
    if host != "localhost" && !host.contains('.') {
        return None;
    }
    if !host
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
    {
        return None;
    }
    Some(format!("https://{v}"))
}

/// Best-effort URL from a window title, for platforms without an accessibility route: the
/// title must end with a recognised browser suffix and contain a bare domain (`docs.rs`,
/// `github.com/x/y`). Returns `https://<domain or domain/path>`.
pub fn url_from_title(title: &str) -> Option<String> {
    let t = title.trim();
    let lower = t.to_lowercase();
    let suffix = TITLE_SUFFIXES
        .iter()
        .find(|s| lower.ends_with(&s.to_lowercase()))?;
    let body = t[..t.len() - suffix.len()].trim();
    body.split_whitespace()
        .map(|w| {
            w.trim_matches(|c: char| matches!(c, '(' | ')' | '[' | ']' | '"' | '\'' | ',' | ';'))
        })
        .find_map(|w| {
            let candidate = if w.contains("://") {
                w.to_string()
            } else {
                format!("https://{w}")
            };
            let bare = w
                .trim_start_matches("https://")
                .trim_start_matches("http://");
            let bare_host = bare.split(['/', '?', '#']).next().unwrap_or("");
            // The domain must be the whole word before any path: `docs.rs/foo` yes,
            // `v1.2.3` no (a version, not a host), `e.g.` no.
            if !looks_like_domain(bare_host) {
                return None;
            }
            domain_of(&candidate).map(|_| candidate)
        })
}

/// A hostname with at least one dot, letters in the last label and nothing exotic.
fn looks_like_domain(host: &str) -> bool {
    let labels: Vec<&str> = host.split('.').collect();
    if labels.len() < 2 || labels.iter().any(|l| l.is_empty()) {
        return false;
    }
    let tld = labels[labels.len() - 1];
    if tld.len() < 2 || !tld.chars().all(|c| c.is_ascii_alphabetic()) {
        return false;
    }
    host.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_values_become_urls() {
        assert_eq!(
            normalize_typed_url("https://github.com/x").as_deref(),
            Some("https://github.com/x")
        );
        assert_eq!(
            normalize_typed_url("github.com/x/y").as_deref(),
            Some("https://github.com/x/y")
        );
        assert_eq!(
            normalize_typed_url("localhost:1420/app").as_deref(),
            Some("https://localhost:1420/app")
        );
        assert_eq!(
            normalize_typed_url("chrome://settings").as_deref(),
            Some("chrome://settings")
        );
        assert_eq!(
            normalize_typed_url("about:blank").as_deref(),
            Some("about:blank")
        );
        assert_eq!(normalize_typed_url("how to bake bread"), None);
        assert_eq!(normalize_typed_url("bread"), None);
        assert_eq!(normalize_typed_url(""), None);
        assert_eq!(normalize_typed_url("ftp://x.y"), None);
        assert_eq!(normalize_typed_url("Search or type URL"), None);
        assert_eq!(normalize_typed_url(".com"), None);
    }

    #[test]
    fn titles_yield_urls_only_with_a_suffix_and_a_domain() {
        assert_eq!(
            url_from_title("docs.rs - Rust docs - Google Chrome").as_deref(),
            Some("https://docs.rs")
        );
        assert_eq!(
            url_from_title("github.com/andreyquadros/ubiquitous-engine — Mozilla Firefox")
                .as_deref(),
            Some("https://github.com/andreyquadros/ubiquitous-engine")
        );
        assert_eq!(
            url_from_title("(1) Inbox - mail.google.com - Brave").as_deref(),
            Some("https://mail.google.com")
        );
        assert_eq!(url_from_title("Release v1.2.3 - Microsoft Edge"), None);
        assert_eq!(
            url_from_title("docs.rs - Rust docs"),
            None,
            "no browser suffix"
        );
        assert_eq!(url_from_title("New Tab - Google Chrome"), None);
        assert_eq!(url_from_title("main.rs - project - Code"), None);
        assert_eq!(url_from_title(""), None);
    }

    #[test]
    fn browser_lists() {
        assert!(WINDOWS_BROWSERS.contains(&"msedge"));
        assert!(LINUX_BROWSERS.contains(&"firefox-esr"));
        assert!(is_firefox("firefox"));
        assert!(is_firefox("Firefox-ESR"));
        assert!(!is_firefox("chrome"));
        assert!(looks_like_domain("a.bc"));
        assert!(!looks_like_domain("v1.2"));
        assert!(!looks_like_domain("nodot"));
    }
}
