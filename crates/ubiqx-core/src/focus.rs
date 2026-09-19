//! The focus guard's pure rules: which foreground app/page hits a blocked target, what UBI
//! says when it holds one, when the "a lot of windows" prompt fires and how target keys are
//! normalised. No I/O here; the engine's `focus` module drives these with the platform ports.

use chrono::{DateTime, Duration, Utc};

use crate::lang::UiLanguage;
use crate::model::*;
use crate::normalize::domain_matches;

/// A target disabled or removed within this many minutes of its last block goes through the
/// three-warning dialog in the UI instead of a single confirmation.
pub const DISABLE_WARNING_WINDOW_MINS: i64 = 15;

/// Shortest and longest focus session the engine accepts, in minutes.
pub const SESSION_MIN_MINUTES: u32 = 5;
pub const SESSION_MAX_MINUTES: u32 = 240;

/// A session stopped by hand before this many minutes earns no praise.
pub const PRAISE_MIN_MINUTES: i64 = 5;

/// The Terminal is left alone for this long after a session starts (the user may be closing
/// things down or launching the task's tools from it).
pub const TERMINAL_GRACE_SECS: i64 = 60;

/// The "a lot of windows" prompt: at least this many app switches inside the window.
pub const PROMPT_SWITCH_THRESHOLD: usize = 10;
pub const PROMPT_WINDOW_MINS: i64 = 15;
pub const PROMPT_COOLDOWN_MINS: i64 = 45;

/// Bundle id of ubiqX itself: the guard never acts on its own windows.
pub const SELF_BUNDLE_ID: &str = "ai.ubiqx.app";

/// Application ids the guard never enforces, whatever the list says: the shell that hosts
/// UBI, the Finder, System Settings and the login window.
pub const EXEMPT_APP_IDS: &[&str] = &[
    SELF_BUNDLE_ID,
    "com.apple.finder",
    "com.apple.systempreferences",
    "com.apple.SystemPreferences",
    "com.apple.loginwindow",
];

/// Application names that mean the same as [`EXEMPT_APP_IDS`] when the id is missing.
pub const EXEMPT_APP_NAMES: &[&str] = &[
    "ubiqX",
    "Finder",
    "System Settings",
    "System Preferences",
    "Ajustes do Sistema",
    "Preferências do Sistema",
    "loginwindow",
];

pub const TERMINAL_APP_ID: &str = "com.apple.Terminal";

/// What UBI says when it holds a distraction, rotated in order per process. Sites and apps
/// share the list; the target name is a separate field the UI shows as a chip.
pub const MESSAGES_PT: [&str; 5] = [
    "Não! Foque na sua produtividade.",
    "Esse app fica pra depois. Sua meta agradece.",
    "Você bloqueou isso por um motivo. Volta pro que importa.",
    "Não hoje. Que tal mais 20 minutos de foco?",
    "Eu seguro a distração; você segura o foco.",
];

pub const MESSAGES_EN: [&str; 5] = [
    "No! Focus on your productivity.",
    "That one can wait. Your goal says thanks.",
    "You blocked this for a reason. Back to what matters.",
    "Not today. How about 20 more minutes of focus?",
    "I hold the distraction; you hold the focus.",
];

/// The `index`-th message of the rotation (wraps around), in the UI language.
pub fn intervention_message(index: usize, lang: UiLanguage) -> &'static str {
    let i = index % MESSAGES_PT.len();
    match lang {
        UiLanguage::PtBr => MESSAGES_PT[i],
        UiLanguage::En => MESSAGES_EN[i],
    }
}

/// Title and message of the praise nudge stored when a session ends.
pub fn praise_text(
    lang: UiLanguage,
    minutes: i64,
    task: &str,
    interventions: u32,
) -> (String, String) {
    match lang {
        UiLanguage::PtBr => (
            "Sessão de foco concluída".into(),
            format!(
                "Sessão de foco concluída: {minutes} min em \"{task}\", {interventions} distrações seguradas."
            ),
        ),
        UiLanguage::En => (
            "Focus session done".into(),
            format!(
                "Focus session done: {minutes} min on \"{task}\", {interventions} distractions held."
            ),
        ),
    }
}

/// Title and message of the "a lot of windows" prompt.
pub fn focus_prompt_text(lang: UiLanguage) -> (&'static str, &'static str) {
    match lang {
        UiLanguage::PtBr => (
            "Muitas janelas",
            "Você tem alternado entre muitas janelas, que tal focar mais? Que tarefa você precisa fazer agora e quer que eu te ajude com um foco maior?",
        ),
        UiLanguage::En => (
            "A lot of windows",
            "You've been switching between a lot of windows. How about focusing more? What do you need to get done right now, and shall I help you focus on it?",
        ),
    }
}

/// Normalises what the user typed into a target key: bundle ids are kept as given (trimmed);
/// domains are lower-cased with the scheme, credentials, port, path, query, fragment and a
/// leading `www.` stripped, so `https://www.YouTube.com/watch?v=x` becomes `youtube.com`.
pub fn normalize_key(kind: FocusTargetKind, raw: &str) -> String {
    let raw = raw.trim();
    match kind {
        FocusTargetKind::App => raw.to_string(),
        FocusTargetKind::Site => {
            let mut s = raw.to_lowercase();
            if let Some(idx) = s.find("://") {
                s = s[idx + 3..].to_string();
            }
            // Everything after the host: path, query, fragment.
            if let Some(idx) = s.find(['/', '?', '#']) {
                s.truncate(idx);
            }
            // Credentials and port.
            if let Some(idx) = s.rfind('@') {
                s = s[idx + 1..].to_string();
            }
            if let Some(idx) = s.find(':') {
                s.truncate(idx);
            }
            let s = s.trim_matches('.');
            s.strip_prefix("www.").unwrap_or(s).to_string()
        }
    }
}

/// Whether `text` looks like a domain the user could block: at least one dot, no spaces,
/// only host characters.
pub fn looks_like_domain(text: &str) -> bool {
    let key = normalize_key(FocusTargetKind::Site, text);
    !key.is_empty()
        && key.contains('.')
        && !key.starts_with('.')
        && !key.ends_with('.')
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
}

/// Whether a site target with `key` covers `page_domain`: the domain itself or any of its
/// subdomains (`youtube.com` covers `m.youtube.com`).
pub fn domain_hits(key: &str, page_domain: &str) -> bool {
    domain_matches(page_domain, key)
}

/// Whether an app target with `key` names the foreground app.
pub fn app_hits(key: &str, app_id: &str) -> bool {
    !key.trim().is_empty() && key.trim().eq_ignore_ascii_case(app_id.trim())
}

/// The first enabled target the foreground app/page hits, apps before sites. Disabled
/// targets never hit.
pub fn find_hit<'a>(
    targets: &'a [FocusTarget],
    app_id: &str,
    page_domain: Option<&str>,
) -> Option<&'a FocusTarget> {
    let enabled = targets.iter().filter(|t| t.enabled);
    enabled
        .clone()
        .find(|t| t.kind == FocusTargetKind::App && app_hits(&t.key, app_id))
        .or_else(|| {
            let domain = page_domain?;
            enabled
                .filter(|t| t.kind == FocusTargetKind::Site)
                .find(|t| domain_hits(&t.key, domain))
        })
}

/// Apps the guard leaves alone: ubiqX, Finder, System Settings, the login window, and the
/// Terminal during the first [`TERMINAL_GRACE_SECS`] of a session (`session_age_secs`,
/// `None` when no session runs).
pub fn is_exempt_app(app_id: &str, app_name: &str, session_age_secs: Option<i64>) -> bool {
    if EXEMPT_APP_IDS
        .iter()
        .any(|id| id.eq_ignore_ascii_case(app_id))
        || EXEMPT_APP_NAMES
            .iter()
            .any(|n| n.eq_ignore_ascii_case(app_name))
    {
        return true;
    }
    if app_id.eq_ignore_ascii_case(TERMINAL_APP_ID) || app_name.eq_ignore_ascii_case("Terminal") {
        return session_age_secs.is_some_and(|age| age < TERMINAL_GRACE_SECS);
    }
    false
}

/// Whether the target was blocked inside the last [`DISABLE_WARNING_WINDOW_MINS`] minutes.
pub fn recently_blocked(target: &FocusTarget, now: DateTime<Utc>) -> bool {
    target
        .last_blocked_at
        .is_some_and(|at| now - at <= Duration::minutes(DISABLE_WARNING_WINDOW_MINS))
}

/// Whether a session of `minutes` on `task` may start.
pub fn validate_session(task: &str, minutes: u32) -> Result<(), String> {
    if task.trim().is_empty() {
        return Err("task must not be empty".into());
    }
    if !(SESSION_MIN_MINUTES..=SESSION_MAX_MINUTES).contains(&minutes) {
        return Err(format!(
            "minutes must be between {SESSION_MIN_MINUTES} and {SESSION_MAX_MINUTES}"
        ));
    }
    Ok(())
}

/// Number of app changes between consecutive blocks (sorted by start) that ended after
/// `since`. The open block counts like any other. Two blocks of the same app in a row
/// (a title change) are not a switch.
pub fn count_app_switches(blocks: &[ActivityBlock], since: DateTime<Utc>) -> usize {
    let mut recent: Vec<&ActivityBlock> = blocks.iter().filter(|b| b.ended_at > since).collect();
    recent.sort_by_key(|b| b.started_at);
    recent
        .windows(2)
        .filter(|w| !w[0].app_id.eq_ignore_ascii_case(&w[1].app_id))
        .count()
}

/// Whether the "a lot of windows" prompt is due: no session running, enough switches inside
/// the window and the cooldown since the last prompt elapsed.
pub fn focus_prompt_due(
    switches: usize,
    session_active: bool,
    last_prompt: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> bool {
    if session_active || switches < PROMPT_SWITCH_THRESHOLD {
        return false;
    }
    last_prompt
        .map(|t| now - t >= Duration::minutes(PROMPT_COOLDOWN_MINS))
        .unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn t(mins: i64) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 9, 17, 9, 0, 0).unwrap() + Duration::minutes(mins)
    }

    fn target(kind: FocusTargetKind, key: &str, enabled: bool) -> FocusTarget {
        FocusTarget {
            id: format!("t-{key}"),
            kind,
            name: key.into(),
            key: key.into(),
            enabled,
            created_at: t(0),
            last_blocked_at: None,
            blocked_count: 0,
        }
    }

    fn block(s: i64, e: i64, app: &str) -> ActivityBlock {
        let mut b = ActivityBlock::new_manual(t(s), t(e), "c".into(), None);
        b.app_id = app.into();
        b.app_name = app.into();
        b
    }

    #[test]
    fn keys_are_normalised() {
        for (raw, key) in [
            ("YouTube.com", "youtube.com"),
            ("https://www.YouTube.com/watch?v=x#t", "youtube.com"),
            ("http://user:pw@m.reddit.com:8080/r/x", "m.reddit.com"),
            ("  www.instagram.com/  ", "instagram.com"),
            ("twitter.com.", "twitter.com"),
        ] {
            assert_eq!(normalize_key(FocusTargetKind::Site, raw), key, "{raw}");
        }
        assert_eq!(
            normalize_key(FocusTargetKind::App, " com.apple.Safari "),
            "com.apple.Safari"
        );
        assert!(looks_like_domain("youtube.com"));
        assert!(looks_like_domain("https://news.ycombinator.com/x"));
        assert!(!looks_like_domain("youtube"));
        assert!(!looks_like_domain("you tube.com"));
        assert!(!looks_like_domain(""));
    }

    #[test]
    fn hits_apps_and_domains_with_parents() {
        let targets = vec![
            target(FocusTargetKind::App, "com.tinyspeck.slackmacgap", true),
            target(FocusTargetKind::Site, "youtube.com", true),
            target(FocusTargetKind::Site, "reddit.com", false),
        ];
        assert_eq!(
            find_hit(&targets, "com.tinyspeck.slackmacgap", None)
                .unwrap()
                .key,
            "com.tinyspeck.slackmacgap"
        );
        assert_eq!(
            find_hit(&targets, "com.google.Chrome", Some("m.youtube.com"))
                .unwrap()
                .key,
            "youtube.com"
        );
        assert_eq!(
            find_hit(&targets, "com.google.Chrome", Some("youtube.com"))
                .unwrap()
                .key,
            "youtube.com"
        );
        assert!(find_hit(&targets, "com.google.Chrome", Some("notyoutube.com")).is_none());
        assert!(
            find_hit(&targets, "com.google.Chrome", Some("reddit.com")).is_none(),
            "disabled targets never hit"
        );
        assert!(find_hit(&targets, "com.apple.Safari", None).is_none());
        assert!(app_hits(
            "COM.TINYSPECK.SLACKMACGAP",
            "com.tinyspeck.slackmacgap"
        ));
        assert!(!app_hits("", ""));
    }

    #[test]
    fn exempt_apps() {
        assert!(is_exempt_app("ai.ubiqx.app", "ubiqX", None));
        assert!(is_exempt_app("com.apple.finder", "Finder", Some(1000)));
        assert!(is_exempt_app("x", "System Settings", None));
        assert!(is_exempt_app("com.apple.loginwindow", "", None));
        assert!(is_exempt_app("com.apple.Terminal", "Terminal", Some(30)));
        assert!(!is_exempt_app("com.apple.Terminal", "Terminal", Some(61)));
        assert!(!is_exempt_app("com.apple.Terminal", "Terminal", None));
        assert!(!is_exempt_app(
            "com.tinyspeck.slackmacgap",
            "Slack",
            Some(1)
        ));
    }

    #[test]
    fn messages_rotate_per_language() {
        assert_eq!(intervention_message(0, UiLanguage::PtBr), MESSAGES_PT[0]);
        assert_eq!(intervention_message(5, UiLanguage::PtBr), MESSAGES_PT[0]);
        assert_eq!(intervention_message(6, UiLanguage::En), MESSAGES_EN[1]);
        assert_eq!(MESSAGES_PT.len(), MESSAGES_EN.len());
        let (title, msg) = praise_text(UiLanguage::PtBr, 45, "relatório", 2);
        assert_eq!(title, "Sessão de foco concluída");
        assert_eq!(
            msg,
            "Sessão de foco concluída: 45 min em \"relatório\", 2 distrações seguradas."
        );
        let (_, msg) = praise_text(UiLanguage::En, 25, "report", 0);
        assert_eq!(
            msg,
            "Focus session done: 25 min on \"report\", 0 distractions held."
        );
        assert_eq!(focus_prompt_text(UiLanguage::PtBr).0, "Muitas janelas");
        assert_eq!(focus_prompt_text(UiLanguage::En).0, "A lot of windows");
    }

    #[test]
    fn recently_blocked_window() {
        let mut tg = target(FocusTargetKind::App, "a", true);
        assert!(!recently_blocked(&tg, t(100)));
        tg.last_blocked_at = Some(t(90));
        assert!(recently_blocked(&tg, t(100)));
        tg.last_blocked_at = Some(t(84));
        assert!(!recently_blocked(&tg, t(100)));
    }

    #[test]
    fn session_validation() {
        assert!(validate_session("  ", 45).is_err());
        assert!(validate_session("x", 4).is_err());
        assert!(validate_session("x", 241).is_err());
        assert!(validate_session("x", 5).is_ok());
        assert!(validate_session("x", 240).is_ok());
    }

    #[test]
    fn switch_counting_and_prompt_rule() {
        let mut blocks = Vec::new();
        for i in 0..12 {
            let app = if i % 2 == 0 { "a" } else { "b" };
            blocks.push(block(i, i + 1, app));
        }
        // 12 blocks alternating → 11 switches; only those ending after `since` count.
        assert_eq!(count_app_switches(&blocks, t(-1)), 11);
        assert_eq!(count_app_switches(&blocks, t(6)), 5);
        // Same app twice in a row is not a switch.
        let same = vec![block(0, 1, "a"), block(1, 2, "A"), block(2, 3, "b")];
        assert_eq!(count_app_switches(&same, t(-1)), 1);

        assert!(focus_prompt_due(10, false, None, t(0)));
        assert!(!focus_prompt_due(9, false, None, t(0)));
        assert!(!focus_prompt_due(20, true, None, t(0)));
        assert!(!focus_prompt_due(20, false, Some(t(-30)), t(0)));
        assert!(focus_prompt_due(20, false, Some(t(-45)), t(0)));
    }
}
