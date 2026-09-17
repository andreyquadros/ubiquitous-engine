//! Redaction of personal data before anything is sent to a remote model.
//!
//! The rules are deliberately conservative and pure-Rust: strip URL query strings and
//! fragments, mask e-mail addresses, phone numbers, CPF/CNPJ and long digit runs (cards,
//! account numbers). Titles of messaging/mail apps are reduced to the app name because the
//! title is usually a contact or a subject line.

use std::sync::OnceLock;

use regex::Regex;

/// Application ids/names whose window titles are personal by nature.
const MESSAGING_APPS: &[&str] = &[
    "net.whatsapp.WhatsApp",
    "whatsapp",
    "com.apple.MobileSMS",
    "messages",
    "ru.keepcoder.Telegram",
    "telegram",
    "com.apple.mail",
    "mail",
    "com.microsoft.Outlook",
    "microsoft outlook",
    "com.tinyspeck.slackmacgap",
    "slack",
    "com.hnc.Discord",
    "discord",
    "com.facebook.archon",
    "messenger",
    "com.microsoft.teams2",
    "microsoft teams",
];

fn re(cell: &'static OnceLock<Regex>, pattern: &str) -> &'static Regex {
    cell.get_or_init(|| Regex::new(pattern).expect("valid regex"))
}

static EMAIL: OnceLock<Regex> = OnceLock::new();
static PHONE: OnceLock<Regex> = OnceLock::new();
static CPF: OnceLock<Regex> = OnceLock::new();
static CNPJ: OnceLock<Regex> = OnceLock::new();
static DIGITS: OnceLock<Regex> = OnceLock::new();

/// Masks personal identifiers inside free text.
pub fn redact_text(text: &str) -> String {
    let mut t = re(&EMAIL, r"(?i)[a-z0-9._%+-]+@[a-z0-9.-]+\.[a-z]{2,}")
        .replace_all(text, "[email]")
        .into_owned();
    t = re(&CNPJ, r"\b\d{2}\.?\d{3}\.?\d{3}/?\d{4}-?\d{2}\b")
        .replace_all(&t, "[cnpj]")
        .into_owned();
    t = re(&CPF, r"\b\d{3}\.?\d{3}\.?\d{3}-?\d{2}\b")
        .replace_all(&t, "[cpf]")
        .into_owned();
    // Long digit runs (cards, accounts, bare CPF/CNPJ) before phones, which are shorter.
    t = re(&DIGITS, r"\b\d{9,}\b").replace_all(&t, "[número]").into_owned();
    t = re(&PHONE, r"(?:\+?\d{1,3}[\s-]?)?\(?\d{2,3}\)?[\s-]?\d{4,5}[\s-]?\d{4}\b")
        .replace_all(&t, "[telefone]")
        .into_owned();
    t
}

/// Keeps only scheme, host and path of a URL (no query string, no fragment, no credentials).
pub fn redact_url(url: &str) -> Option<String> {
    let mut u = url::Url::parse(url).ok()?;
    u.set_query(None);
    u.set_fragment(None);
    let _ = u.set_username("");
    let _ = u.set_password(None);
    Some(u.to_string())
}

pub fn is_messaging_app(app_id: &str, app_name: &str) -> bool {
    MESSAGING_APPS
        .iter()
        .any(|m| m.eq_ignore_ascii_case(app_id) || m.eq_ignore_ascii_case(app_name))
}

/// Text describing a block that is safe to send to a remote model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RedactedBlock {
    pub app_name: String,
    pub title: String,
    pub url: Option<String>,
    pub domain: Option<String>,
}

pub fn redact_block(app_id: &str, app_name: &str, title: &str, url: Option<&str>, domain: Option<&str>) -> RedactedBlock {
    let title = if is_messaging_app(app_id, app_name) {
        app_name.to_string()
    } else {
        redact_text(title)
    };
    RedactedBlock {
        app_name: app_name.to_string(),
        title,
        url: url.and_then(redact_url).map(|u| redact_text(&u)),
        domain: domain.map(|d| d.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_identifiers() {
        let t = redact_text("Fale com joao.silva@ifro.edu.br ou (69) 99999-1234, CPF 123.456.789-09, CNPJ 12.345.678/0001-95, conta 1234567890123");
        assert!(!t.contains("joao.silva"), "{t}");
        assert!(t.contains("[email]"));
        assert!(t.contains("[telefone]"), "{t}");
        assert!(t.contains("[cpf]"), "{t}");
        assert!(t.contains("[cnpj]"), "{t}");
        assert!(t.contains("[número]"), "{t}");
        assert_eq!(redact_text("Edital 12/2026 - Incubadora"), "Edital 12/2026 - Incubadora");
    }

    #[test]
    fn strips_url_query() {
        assert_eq!(
            redact_url("https://user:pw@docs.google.com/d/abc/edit?token=SECRET#x").as_deref(),
            Some("https://docs.google.com/d/abc/edit")
        );
        assert_eq!(redact_url("nope"), None);
    }

    #[test]
    fn messaging_titles_become_app_name() {
        let r = redact_block("net.whatsapp.WhatsApp", "WhatsApp", "Maria Souza", None, None);
        assert_eq!(r.title, "WhatsApp");
        let r = redact_block("com.apple.dt.Xcode", "Xcode", "main.swift", None, None);
        assert_eq!(r.title, "main.swift");
    }
}
