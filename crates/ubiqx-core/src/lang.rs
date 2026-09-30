//! The UI language: the one setting every user-facing text produced by the Rust side follows
//! (nudges, notifications, template reports, prompt instructions, validation messages).
//!
//! `Settings.language` stays a BCP-47 string across the IPC boundary; [`UiLanguage::from_tag`]
//! accepts the usual spellings and anything unknown falls back to Brazilian Portuguese, the
//! product's default.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UiLanguage {
    /// Brazilian Portuguese (`pt-BR`), the default.
    #[default]
    PtBr,
    /// English (`en`).
    En,
}

impl UiLanguage {
    /// Every supported language, in menu order.
    pub const ALL: [UiLanguage; 2] = [UiLanguage::PtBr, UiLanguage::En];

    /// Parses a language tag leniently: `pt`, `pt-BR`, `pt_br`, `pt-PT`, `en`, `en-US`,
    /// `en_GB`… Case, separators and regions are ignored; anything else (including an empty
    /// string) is Brazilian Portuguese.
    pub fn from_tag(tag: &str) -> Self {
        let primary = tag
            .trim()
            .split(['-', '_', '.', '@'])
            .next()
            .unwrap_or("")
            .to_ascii_lowercase();
        match primary.as_str() {
            "en" | "eng" => UiLanguage::En,
            _ => UiLanguage::PtBr,
        }
    }

    /// The canonical BCP-47 tag stored in `Settings.language`.
    pub fn tag(self) -> &'static str {
        match self {
            UiLanguage::PtBr => "pt-BR",
            UiLanguage::En => "en",
        }
    }

    /// The language's own name for prompts ("português do Brasil" / "English").
    pub fn name(self) -> &'static str {
        match self {
            UiLanguage::PtBr => "português do Brasil",
            UiLanguage::En => "English",
        }
    }

    /// Picks the text for this language.
    pub fn pick<'a>(self, pt: &'a str, en: &'a str) -> &'a str {
        match self {
            UiLanguage::PtBr => pt,
            UiLanguage::En => en,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tags_leniently() {
        for tag in [
            "pt", "pt-BR", "pt_BR", "PT-br", "pt-PT", "", "  ", "xx", "fr-FR",
        ] {
            assert_eq!(UiLanguage::from_tag(tag), UiLanguage::PtBr, "{tag:?}");
        }
        for tag in ["en", "EN", "en-US", "en_GB", "en-GB.UTF-8", " en "] {
            assert_eq!(UiLanguage::from_tag(tag), UiLanguage::En, "{tag:?}");
        }
    }

    #[test]
    fn tags_round_trip() {
        for lang in UiLanguage::ALL {
            assert_eq!(UiLanguage::from_tag(lang.tag()), lang);
        }
        assert_eq!(UiLanguage::PtBr.pick("a", "b"), "a");
        assert_eq!(UiLanguage::En.pick("a", "b"), "b");
        assert_eq!(UiLanguage::default(), UiLanguage::PtBr);
    }
}
