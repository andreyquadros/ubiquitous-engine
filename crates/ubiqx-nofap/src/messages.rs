//! The few lines the engine and the shells need in Rust, PT-BR and EN: the fallback
//! notification, the tray labels, the quit dialog and the rotation of support messages.
//! Every string comes from the reviewed copy bank (design section 5.6) and is checked by
//! the forbidden-word and naming tests; `{name}` is `ubiqx_core::nofap::DISPLAY_NAME`. No
//! string here has a placeholder for a host, a title or an app.

use ubiqx_core::nofap::DISPLAY_NAME;
use ubiqx_core::UiLanguage;

/// Title of every notification and of the card window. Says nothing about the subject.
pub const NOTIFICATION_TITLE: &str = "UBI";

/// The placeholder for [`DISPLAY_NAME`], the only one a string of this module may hold.
pub const NAME_PLACEHOLDER: &str = "{name}";

const BLOCK_DISCREET: &str = "block.discreet";
const BLOCK_DEFAULT: &str = "block.default";
const BLOCK_ROTATION: [&str; 3] = ["block.breathe", "block.choice", "block.next"];
const DRIFT_NOTICE: &str = "drift.notice";
const QUIT_CONFIRM_SYSTEM: &str = "quit.confirm_system";
const QUIT_CONFIRM_NONE: &str = "quit.confirm_none";
const QUIT_KEEP_OPEN: &str = "quit.keep_open";
const QUIT_QUIT: &str = "quit.quit";
const TRAY_OPEN: &str = "tray.open";
const TRAY_STATUS_ON: &str = "tray.status_on";
const TRAY_BREATHER: &str = "tray.breather";
const TRAY_CHECK_UPDATES: &str = "tray.check_updates";
const TRAY_QUIT: &str = "tray.quit";
const TRAY_NOT_PAUSED: &str = "tray.not_paused";

/// `(key, PT-BR, EN)`. The first four are strings of the copy bank (design section 5.6)
/// and are kept equal to it by a test; the rest are button and menu labels.
const BANK: &[(&str, &str, &str)] = &[
    (
        BLOCK_DISCREET,
        "Página bloqueada pelo UBI.",
        "Page blocked by UBI.",
    ),
    (
        DRIFT_NOTICE,
        "O {name} precisa da sua atenção.",
        "{name} needs your attention.",
    ),
    (
        QUIT_CONFIRM_SYSTEM,
        "Com o app fechado, o que está aplicado no sistema continua valendo; o bloqueio no app e a verificação por IA param.",
        "With the app closed, what is applied to the system still holds; the in-app block and the AI check stop.",
    ),
    (
        QUIT_CONFIRM_NONE,
        "Com o app fechado, nada fica barrando sites neste computador até você abrir o app de novo.",
        "With the app closed, nothing stops sites on this computer until you open the app again.",
    ),
    (QUIT_KEEP_OPEN, "Manter aberto", "Keep open"),
    (QUIT_QUIT, "Sair", "Quit"),
    (TRAY_OPEN, "Abrir", "Open"),
    (TRAY_STATUS_ON, "{name}: ligado", "{name}: on"),
    (TRAY_BREATHER, "Preciso de um respiro", "I need a breather"),
    (TRAY_CHECK_UPDATES, "Verificar atualizações", "Check for updates"),
    (TRAY_QUIT, "Sair", "Quit"),
    (TRAY_NOT_PAUSED, "Não pausa o {name}", "Does not pause {name}"),
];

/// The string of `key` in `lang`, with the display name filled in. A key this module does
/// not hold gives the neutral blocked line, never an empty window.
fn text(key: &str, lang: UiLanguage) -> String {
    let (_, pt, en) = BANK
        .iter()
        .find(|(entry, _, _)| *entry == key)
        .unwrap_or(&BANK[0]);
    let raw = match lang {
        UiLanguage::PtBr => pt,
        UiLanguage::En => en,
    };
    raw.replace(NAME_PLACEHOLDER, DISPLAY_NAME)
}

/// Which fallback notification to send when the card window cannot be shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Notice {
    /// `block.discreet`.
    Blocked,
    /// `drift.notice`.
    Drift,
}

/// Title and body of a fallback notification.
pub fn notification_text(kind: Notice, lang: UiLanguage) -> (String, String) {
    let key = match kind {
        Notice::Blocked => BLOCK_DISCREET,
        Notice::Drift => DRIFT_NOTICE,
    };
    (NOTIFICATION_TITLE.to_owned(), text(key, lang))
}

/// Labels of the tray items the module adds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrayLabels {
    /// "Abrir" (standalone).
    pub open: String,
    /// The neutral status line while the module is on.
    pub status_on: String,
    /// "Preciso de um respiro".
    pub breather: String,
    /// "Verificar atualizações" (standalone).
    pub check_updates: String,
    /// "Sair" (standalone).
    pub quit: String,
    /// Tooltip of ubiqX's pause and private-mode items: "Não pausa o {name}".
    pub not_paused: String,
}

pub fn tray_labels(lang: UiLanguage) -> TrayLabels {
    TrayLabels {
        open: text(TRAY_OPEN, lang),
        status_on: text(TRAY_STATUS_ON, lang),
        breather: text(TRAY_BREATHER, lang),
        check_updates: text(TRAY_CHECK_UPDATES, lang),
        quit: text(TRAY_QUIT, lang),
        not_paused: text(TRAY_NOT_PAUSED, lang),
    }
}

/// The quit dialog (`quit.confirm_system` or `quit.confirm_none`, whichever is true). The
/// primary button is "Manter aberto".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuitDialog {
    pub message: String,
    pub keep_open: String,
    pub quit: String,
}

pub fn quit_dialog(system_applied: bool, lang: UiLanguage) -> QuitDialog {
    let message = if system_applied {
        QUIT_CONFIRM_SYSTEM
    } else {
        QUIT_CONFIRM_NONE
    };
    QuitDialog {
        message: text(message, lang),
        keep_open: text(QUIT_KEEP_OPEN, lang),
        quit: text(QUIT_QUIT, lang),
    }
}

/// The message key of the `n`-th block card: `block.discreet` while `discreet`; otherwise
/// `block.default` for a hit on the user's own list, and the rotation `block.breathe`,
/// `block.choice`, `block.next` for the rest.
pub fn rotation_key(discreet: bool, user_list: bool, n: usize) -> &'static str {
    if discreet {
        BLOCK_DISCREET
    } else if user_list {
        BLOCK_DEFAULT
    } else {
        BLOCK_ROTATION[n % BLOCK_ROTATION.len()]
    }
}

/// Every `(key, PT-BR, EN)` this module holds, with `{name}` not yet replaced: the input
/// of the parity, forbidden-word and naming tests.
pub fn all() -> Vec<(&'static str, &'static str, &'static str)> {
    BANK.to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::help::{forbidden_in, ForbiddenWords};
    use std::collections::HashSet;

    const REPO: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

    fn read(path: &str) -> String {
        std::fs::read_to_string(format!("{REPO}/{path}"))
            .unwrap_or_else(|error| panic!("{path}: {error}"))
    }

    /// The `{placeholders}` of a string.
    fn placeholders(text: &str) -> Vec<&str> {
        let mut found = Vec::new();
        let mut rest = text;
        while let Some(start) = rest.find('{') {
            let end = rest[start..].find('}').expect("an opening brace is closed") + start;
            found.push(&rest[start..=end]);
            rest = &rest[end + 1..];
        }
        found
    }

    #[test]
    fn both_languages_have_every_string_with_the_same_placeholders() {
        let mut keys = HashSet::new();
        for (key, pt, en) in all() {
            assert!(keys.insert(key), "{key} is listed twice");
            assert!(!pt.trim().is_empty() && !en.trim().is_empty(), "{key}");
            assert_eq!(pt.trim(), pt, "{key}");
            assert_eq!(en.trim(), en, "{key}");
            assert_eq!(placeholders(pt), placeholders(en), "{key}");
        }
        assert_eq!(keys.len(), 12);
    }

    #[test]
    fn no_string_has_a_placeholder_for_a_host_a_title_or_an_app() {
        for (key, pt, en) in all() {
            for text in [pt, en] {
                for placeholder in placeholders(text) {
                    assert_eq!(placeholder, NAME_PLACEHOLDER, "{key}: {text}");
                }
            }
        }
    }

    /// The row of `key` in the copy bank of the design: `| `key` … | PT-BR | English |`.
    fn bank_row(design: &str, key: &str) -> (String, String) {
        let start = format!("| `{key}`");
        let row = design
            .lines()
            .find(|line| line.starts_with(&start))
            .unwrap_or_else(|| panic!("{key} is not in the copy bank of the design"));
        let cells: Vec<&str> = row.trim().trim_matches('|').split(" | ").collect();
        assert_eq!(cells.len(), 3, "{row}");
        (cells[1].trim().to_owned(), cells[2].trim().to_owned())
    }

    #[test]
    fn copy_bank_strings_are_the_reviewed_ones() {
        let design = read("docs/nofap/DESIGN.md");
        for key in [
            BLOCK_DISCREET,
            DRIFT_NOTICE,
            QUIT_CONFIRM_SYSTEM,
            QUIT_CONFIRM_NONE,
        ] {
            let (_, pt, en) = all()
                .into_iter()
                .find(|(entry, _, _)| *entry == key)
                .unwrap();
            assert_eq!(
                (pt.to_owned(), en.to_owned()),
                bank_row(&design, key),
                "{key}"
            );
        }
        // The keys handed to the card window exist in the bank too.
        for key in BLOCK_ROTATION.into_iter().chain([BLOCK_DEFAULT]) {
            bank_row(&design, key);
        }
    }

    /// The two values of `key` in the frontend's bank (PT-BR first, then English).
    fn frontend_values(source: &str, key: &str) -> Vec<String> {
        let needle = format!("'{key}': '");
        let mut values = Vec::new();
        let mut rest = source;
        while let Some(at) = rest.find(&needle) {
            let tail = &rest[at + needle.len()..];
            let mut value = String::new();
            let mut chars = tail.chars();
            while let Some(c) = chars.next() {
                match c {
                    '\\' => value.extend(chars.next()),
                    '\'' => break,
                    other => value.push(other),
                }
            }
            values.push(value);
            rest = tail;
        }
        values
    }

    #[test]
    fn the_strings_shared_with_the_frontend_are_identical() {
        let frontend = read("apps/desktop/src/i18n/messages/nofap.ts");
        let shared = [
            (BLOCK_DISCREET, BLOCK_DISCREET),
            (DRIFT_NOTICE, DRIFT_NOTICE),
            (QUIT_CONFIRM_SYSTEM, QUIT_CONFIRM_SYSTEM),
            (QUIT_CONFIRM_NONE, QUIT_CONFIRM_NONE),
            // The tray item and the page's button are the same sentence.
            (TRAY_BREATHER, "btn.panic"),
        ];
        for (key, frontend_key) in shared {
            let (_, pt, en) = all()
                .into_iter()
                .find(|(entry, _, _)| *entry == key)
                .unwrap();
            assert_eq!(
                frontend_values(&frontend, frontend_key),
                vec![pt.to_owned(), en.to_owned()],
                "{key}"
            );
        }
        // Every key a card can carry has a sentence in the frontend.
        for key in BLOCK_ROTATION
            .into_iter()
            .chain([BLOCK_DEFAULT, BLOCK_DISCREET])
        {
            assert_eq!(frontend_values(&frontend, key).len(), 2, "{key}");
        }
    }

    #[test]
    fn no_string_uses_a_forbidden_word() {
        let words: ForbiddenWords =
            serde_json::from_str(&read("crates/ubiqx-nofap/data/forbidden-words.json")).unwrap();
        assert!(
            words.pt.len() >= 25 && words.en.len() >= 16,
            "the list of design 5.1"
        );
        for (key, pt, en) in all() {
            for text in [pt, en] {
                assert_eq!(
                    forbidden_in(text, &words),
                    Vec::<String>::new(),
                    "{key}: {text}"
                );
            }
        }
    }

    #[test]
    fn the_fallback_notification_says_nothing_about_the_subject() {
        for lang in UiLanguage::ALL {
            for kind in [Notice::Blocked, Notice::Drift] {
                let (title, body) = notification_text(kind, lang);
                assert_eq!(title, "UBI");
                assert!(!body.contains('{'), "{body}");
                let said = body.replace(DISPLAY_NAME, "").to_lowercase();
                for word in ["porn", "adult", "sex", "nud", "site", "http", "www"] {
                    assert!(!said.contains(word), "{body}");
                }
            }
        }
        assert_eq!(
            notification_text(Notice::Blocked, UiLanguage::PtBr).1,
            "Página bloqueada pelo UBI."
        );
        assert_eq!(
            notification_text(Notice::Blocked, UiLanguage::En).1,
            "Page blocked by UBI."
        );
        assert_eq!(
            notification_text(Notice::Drift, UiLanguage::PtBr).1,
            format!("O {DISPLAY_NAME} precisa da sua atenção.")
        );
        assert_eq!(
            notification_text(Notice::Drift, UiLanguage::En).1,
            format!("{DISPLAY_NAME} needs your attention.")
        );
    }

    #[test]
    fn tray_labels_in_both_languages() {
        assert_eq!(
            tray_labels(UiLanguage::PtBr),
            TrayLabels {
                open: "Abrir".into(),
                status_on: format!("{DISPLAY_NAME}: ligado"),
                breather: "Preciso de um respiro".into(),
                check_updates: "Verificar atualizações".into(),
                quit: "Sair".into(),
                not_paused: format!("Não pausa o {DISPLAY_NAME}"),
            }
        );
        assert_eq!(
            tray_labels(UiLanguage::En),
            TrayLabels {
                open: "Open".into(),
                status_on: format!("{DISPLAY_NAME}: on"),
                breather: "I need a breather".into(),
                check_updates: "Check for updates".into(),
                quit: "Quit".into(),
                not_paused: format!("Does not pause {DISPLAY_NAME}"),
            }
        );
    }

    #[test]
    fn the_quit_dialog_says_what_is_actually_left() {
        for lang in UiLanguage::ALL {
            let with_system = quit_dialog(true, lang);
            let without = quit_dialog(false, lang);
            assert_ne!(with_system.message, without.message);
            assert_eq!(with_system.keep_open, without.keep_open);
            assert_eq!(with_system.quit, without.quit);
            assert_ne!(with_system.keep_open, with_system.quit);
        }
        let pt = quit_dialog(false, UiLanguage::PtBr);
        assert!(pt.message.contains("nada fica barrando"));
        assert_eq!(
            (pt.keep_open.as_str(), pt.quit.as_str()),
            ("Manter aberto", "Sair")
        );
        assert!(quit_dialog(true, UiLanguage::PtBr)
            .message
            .contains("continua valendo"));
        let en = quit_dialog(true, UiLanguage::En);
        assert!(en.message.contains("still holds"));
        assert_eq!(
            (en.keep_open.as_str(), en.quit.as_str()),
            ("Keep open", "Quit")
        );
    }

    #[test]
    fn the_rotation_of_block_messages() {
        // Discreet wins over everything.
        for n in 0..5 {
            assert_eq!(rotation_key(true, false, n), "block.discreet");
            assert_eq!(rotation_key(true, true, n), "block.discreet");
            assert_eq!(rotation_key(false, true, n), "block.default");
        }
        let rotation: Vec<&str> = (0..7).map(|n| rotation_key(false, false, n)).collect();
        assert_eq!(
            rotation,
            vec![
                "block.breathe",
                "block.choice",
                "block.next",
                "block.breathe",
                "block.choice",
                "block.next",
                "block.breathe",
            ]
        );
        assert_eq!(rotation_key(false, false, usize::MAX), "block.breathe");
    }
}
