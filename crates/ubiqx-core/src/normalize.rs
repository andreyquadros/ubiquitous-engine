//! Text normalisation helpers shared by the segmenter, rules, learning and classifiers.

use std::collections::HashSet;

/// Titles that carry no information about *what* the user is doing, only *where*.
/// Blocks whose normalised title is one of these are candidates for vision classification.
const GENERIC_TITLES: &[&str] = &[
    "", "whatsapp", "mail", "messages", "finder", "terminal", "iterm2", "slack", "discord",
    "telegram", "new tab", "nova guia", "untitled", "sem título", "home", "inbox", "calendar",
    "notes", "preview", "safari", "google chrome", "arc", "firefox", "microsoft teams", "zoom",
];

/// Lower-cases, strips app-name suffixes such as " - Google Chrome", collapses whitespace,
/// drops notification counters like "(3)" and trims unread markers.
pub fn normalize_title(raw: &str, app_name: &str) -> String {
    let mut t = raw.trim().to_lowercase();
    let app = app_name.trim().to_lowercase();
    if !app.is_empty() {
        for sep in [" - ", " — ", " – ", " | ", " • "] {
            let suffix = format!("{sep}{app}");
            if let Some(stripped) = t.strip_suffix(&suffix) {
                t = stripped.to_string();
                break;
            }
        }
    }
    // Remove leading counters "(3) " and trailing " (3)".
    let t = strip_counters(&t);
    // Collapse whitespace.
    let t = t.split_whitespace().collect::<Vec<_>>().join(" ");
    t.trim_matches(|c: char| c == '•' || c == '·' || c == '-' || c.is_whitespace())
        .to_string()
}

fn strip_counters(t: &str) -> String {
    let mut s = t.to_string();
    loop {
        let before = s.clone();
        if let Some(rest) = s.strip_prefix('(') {
            if let Some(end) = rest.find(')') {
                if rest[..end].chars().all(|c| c.is_ascii_digit()) && end > 0 {
                    s = rest[end + 1..].trim_start().to_string();
                }
            }
        }
        if let Some(open) = s.rfind('(') {
            if s.ends_with(')') && s[open + 1..s.len() - 1].chars().all(|c| c.is_ascii_digit()) {
                s = s[..open].trim_end().to_string();
            }
        }
        if s == before {
            return s;
        }
    }
}

/// Whether a normalised title tells us nothing about the task at hand.
pub fn is_generic_title(title_key: &str) -> bool {
    let t = title_key.trim();
    GENERIC_TITLES.contains(&t) || t.chars().count() < 3
}

/// Extracts the registrable-ish domain of a URL: `https://sei.ifro.edu.br/x` → `sei.ifro.edu.br`.
/// Strips a leading `www.`.
pub fn domain_of(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url).ok()?;
    let host = parsed.host_str()?.to_lowercase();
    Some(host.strip_prefix("www.").unwrap_or(&host).to_string())
}

/// Suffix-aware domain match: pattern `ifro.edu.br` matches `ifro.edu.br` and `sei.ifro.edu.br`.
pub fn domain_matches(domain: &str, pattern: &str) -> bool {
    let d = domain.trim().to_lowercase();
    let p = pattern.trim().trim_start_matches("*.").to_lowercase();
    if p.is_empty() {
        return false;
    }
    d == p || d.ends_with(&format!(".{p}"))
}

/// Tokenises text for similarity: lower-case alphanumeric words with 3+ characters.
pub fn tokens(text: &str) -> HashSet<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.chars().count() >= 3)
        .map(|w| w.to_string())
        .collect()
}

/// Jaccard similarity of two token sets (0.0–1.0).
pub fn jaccard(a: &HashSet<String>, b: &HashSet<String>) -> f32 {
    if a.is_empty() && b.is_empty() {
        return 0.0;
    }
    let inter = a.intersection(b).count() as f32;
    let union = a.union(b).count() as f32;
    if union == 0.0 {
        0.0
    } else {
        inter / union
    }
}

/// Similarity between two activity descriptors (app + title + domain), weighting app identity.
pub fn activity_similarity(
    app_a: &str,
    title_a: &str,
    domain_a: Option<&str>,
    app_b: &str,
    title_b: &str,
    domain_b: Option<&str>,
) -> f32 {
    let same_app = app_a.eq_ignore_ascii_case(app_b);
    let same_domain = match (domain_a, domain_b) {
        (Some(a), Some(b)) => a.eq_ignore_ascii_case(b),
        _ => false,
    };
    let title_sim = jaccard(&tokens(title_a), &tokens(title_b));
    let mut score = 0.5 * title_sim;
    if same_app {
        score += 0.25;
    }
    if same_domain {
        score += 0.35;
    }
    score.min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_app_suffix_and_counters() {
        assert_eq!(
            normalize_title("(3) Inbox - Gmail - Google Chrome", "Google Chrome"),
            "inbox - gmail"
        );
        assert_eq!(normalize_title("  Edital  IFRO (2)", "Preview"), "edital ifro");
        assert_eq!(normalize_title("WhatsApp", "WhatsApp"), "whatsapp");
    }

    #[test]
    fn generic_titles() {
        assert!(is_generic_title("whatsapp"));
        assert!(is_generic_title(""));
        assert!(is_generic_title("ab"));
        assert!(!is_generic_title("edital 12/2026 - incubadora"));
    }

    #[test]
    fn domains() {
        assert_eq!(domain_of("https://www.sei.ifro.edu.br/sei/x?y=1"), Some("sei.ifro.edu.br".into()));
        assert_eq!(domain_of("not a url"), None);
        assert!(domain_matches("sei.ifro.edu.br", "ifro.edu.br"));
        assert!(domain_matches("ifro.edu.br", "*.ifro.edu.br"));
        assert!(!domain_matches("notifro.edu.br", "ifro.edu.br"));
        assert!(!domain_matches("x", ""));
    }

    #[test]
    fn similarity() {
        let s = activity_similarity(
            "Google Chrome",
            "SEI - Processo 123 IFRO",
            Some("sei.ifro.edu.br"),
            "Google Chrome",
            "SEI - Processo 456 IFRO",
            Some("sei.ifro.edu.br"),
        );
        assert!(s > 0.8, "{s}");
        let d = activity_similarity("Slack", "general", None, "Xcode", "main.swift", None);
        assert!(d < 0.1, "{d}");
    }
}
