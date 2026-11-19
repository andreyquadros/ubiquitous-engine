//! The few lines the engine and the shells need in Rust, PT-BR and EN: the fallback
//! notification, the tray labels, the quit dialog and the rotation of support messages.
//! Every string comes from the reviewed copy bank (design section 5.6) and is checked by
//! the forbidden-word and naming tests; `{name}` is `ubiqx_core::nofap::DISPLAY_NAME`. No
//! string here has a placeholder for a host, a title or an app.
//!
//! Seeded by the foundation unit: complete types, signatures only.
#![allow(unused_variables)]

use ubiqx_core::UiLanguage;

/// Title of every notification and of the card window. Says nothing about the subject.
pub const NOTIFICATION_TITLE: &str = "UBI";

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
    todo!()
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
    todo!()
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
    todo!()
}

/// The message key of the `n`-th block card: `block.discreet` while `discreet`; otherwise
/// `block.default` for a hit on the user's own list, and the rotation `block.breathe`,
/// `block.choice`, `block.next` for the rest.
pub fn rotation_key(discreet: bool, user_list: bool, n: usize) -> &'static str {
    todo!()
}

/// Every `(key, PT-BR, EN)` this module holds, with `{name}` not yet replaced: the input
/// of the parity, forbidden-word and naming tests.
pub fn all() -> Vec<(&'static str, &'static str, &'static str)> {
    todo!()
}
