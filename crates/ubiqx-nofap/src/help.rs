//! When the professional-help card is due, and the help resources (design sections 5.5 and
//! 5.7). The thresholds are product heuristics, not clinical cut-offs, and the card says
//! "isso não é um diagnóstico".

use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use ubiqx_core::nofap::{DayCount, DayKind};
use ubiqx_core::nofap_ipc::HelpResource;

/// In the last [`HELP_WINDOW_DAYS`] days: this many lapses, or this many override requests
/// (quits while on included), or this many "muito difícil" check-ins.
pub const HELP_LAPSES_7D: u32 = 5;
pub const HELP_OVERRIDES_7D: u32 = 10;
pub const HELP_VERY_BAD_7D: u32 = 2;
pub const HELP_WINDOW_DAYS: i64 = 7;
/// The card is shown at most once in this many days.
pub const HELP_EVERY_DAYS: i64 = 7;
/// The reminder that the module is on, while it is on.
pub const REMINDER_EVERY_DAYS: i64 = 30;

/// The help resources shipped with the app. The one file besides the blocklist data that
/// is embedded; `list` holds the blocklist, this module the counselling data.
const RESOURCES_JSON: &str = include_str!("../data/resources.json");

/// Whether the help card is due: a threshold is reached inside the window ending at
/// `today` (local day), and the card was not shown in the last [`HELP_EVERY_DAYS`] days.
///
/// `last_shown` is compared by its UTC date, which can be a day ahead of the local one; a
/// date further in the future than that (the clock was set back) does not hold the card.
pub fn help_due(counts: &[DayCount], last_shown: Option<DateTime<Utc>>, today: NaiveDate) -> bool {
    if let Some(shown) = last_shown {
        let days_since = (today - shown.date_naive()).num_days();
        if (-1..HELP_EVERY_DAYS).contains(&days_since) {
            return false;
        }
    }
    let Some(first) = today.checked_sub_signed(Duration::days(HELP_WINDOW_DAYS - 1)) else {
        return false;
    };
    let (mut lapses, mut overrides, mut very_bad) = (0u32, 0u32, 0u32);
    for count in counts {
        let in_window = NaiveDate::parse_from_str(&count.day, crate::progress::DAY_FORMAT)
            .is_ok_and(|day| day >= first && day <= today);
        if !in_window {
            continue;
        }
        let total = match count.kind {
            DayKind::Lapse => &mut lapses,
            // Quitting while the module is on is the cheapest way around it, and counts.
            DayKind::OverrideRequest | DayKind::QuitOn => &mut overrides,
            DayKind::CheckinVeryBad => &mut very_bad,
            _ => continue,
        };
        *total = total.saturating_add(count.n);
    }
    lapses >= HELP_LAPSES_7D || overrides >= HELP_OVERRIDES_7D || very_bad >= HELP_VERY_BAD_7D
}

/// One entry of `data/resources.json`. `verified_on` and `verified_by` ship empty; the
/// release-mode test `resources_verified` fails while any is, so a person has to open each
/// site and check each number before a release.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceEntry {
    pub id: String,
    pub country: String,
    pub name: String,
    pub phone: Option<String>,
    pub url: Option<String>,
    pub note_pt: String,
    pub note_en: String,
    pub verified_on: String,
    pub verified_by: String,
}

/// Parses the text of `resources.json`.
pub fn parse_resources(json: &str) -> Result<Vec<ResourceEntry>, serde_json::Error> {
    serde_json::from_str(json)
}

/// The embedded resource list (`data/resources.json`). A test keeps the file readable; a
/// file that could not be read would give an empty list, and the window then says that
/// the contacts did not come with this version rather than showing a made-up one.
pub fn resources() -> Vec<ResourceEntry> {
    parse_resources(RESOURCES_JSON).unwrap_or_default()
}

/// The embedded resources as the wire type (without the verification fields).
pub fn resource_views() -> Vec<HelpResource> {
    resources()
        .into_iter()
        .map(|entry| HelpResource {
            id: entry.id,
            country: entry.country,
            name: entry.name,
            phone: entry.phone,
            url: entry.url,
            note_pt: entry.note_pt,
            note_en: entry.note_en,
        })
        .collect()
}

/// The parsed text of `data/forbidden-words.json`: words no user-facing string may hold.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ForbiddenWords {
    /// PT-BR entries; one ending in `*` is a stem.
    pub pt: Vec<String>,
    /// English entries.
    pub en: Vec<String>,
}

/// A letter without its accent, in lower case.
fn fold(c: char) -> char {
    match c.to_lowercase().next().unwrap_or(c) {
        'á' | 'à' | 'â' | 'ã' | 'ä' | 'å' => 'a',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'í' | 'ì' | 'î' | 'ï' => 'i',
        'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
        'ú' | 'ù' | 'û' | 'ü' => 'u',
        'ç' => 'c',
        'ñ' => 'n',
        lower => lower,
    }
}

/// The words of `text`: runs of letters and digits, folded.
fn words_of(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(|word| word.chars().map(fold).collect())
        .collect()
}

/// The entries of `words` found in `text` as whole words, case- and accent-insensitively.
/// Both lists are applied to every text, whatever its language. An entry of several words
/// matches those words in a row; an entry ending in `*` matches any word that starts with
/// it. Each entry is reported once, as it is written in the list.
pub fn forbidden_in(text: &str, words: &ForbiddenWords) -> Vec<String> {
    let said = words_of(text);
    let mut found: Vec<String> = Vec::new();
    for entry in words.pt.iter().chain(&words.en) {
        let (pattern, stem) = match entry.strip_suffix('*') {
            Some(pattern) => (pattern, true),
            None => (entry.as_str(), false),
        };
        let pattern = words_of(pattern);
        let Some((last, first)) = pattern.split_last() else {
            continue;
        };
        let hit = said.windows(pattern.len()).any(|window| {
            let (tail, head) = window.split_last().unwrap_or((&window[0], &[]));
            head == first
                && if stem {
                    tail.starts_with(last.as_str())
                } else {
                    tail == last
                }
        });
        if hit && !found.contains(entry) {
            found.push(entry.clone());
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn day(text: &str) -> NaiveDate {
        NaiveDate::parse_from_str(text, "%Y-%m-%d").unwrap()
    }

    fn count(day: &str, kind: DayKind, n: u32) -> DayCount {
        DayCount {
            day: day.to_owned(),
            hour: 12,
            kind,
            n,
        }
    }

    const TODAY: &str = "2026-09-17";

    fn due(counts: &[DayCount]) -> bool {
        help_due(counts, None, day(TODAY))
    }

    #[test]
    fn five_lapses_in_seven_days() {
        assert!(!due(&[]));
        assert!(!due(&[count("2026-09-17", DayKind::Lapse, 4)]));
        assert!(due(&[count("2026-09-17", DayKind::Lapse, 5)]));
        // Spread over the window, first and last day included.
        let spread = [
            count("2026-09-11", DayKind::Lapse, 1),
            count("2026-09-13", DayKind::Lapse, 2),
            count("2026-09-17", DayKind::Lapse, 2),
        ];
        assert!(due(&spread));
        // The eighth day back is outside the window, and so is tomorrow.
        let outside = [
            count("2026-09-10", DayKind::Lapse, 3),
            count("2026-09-18", DayKind::Lapse, 3),
            count("2026-09-17", DayKind::Lapse, 4),
            count("not a day", DayKind::Lapse, 9),
        ];
        assert!(!due(&outside));
    }

    #[test]
    fn ten_override_requests_quits_included() {
        assert!(!due(&[count(TODAY, DayKind::OverrideRequest, 9)]));
        assert!(due(&[count(TODAY, DayKind::OverrideRequest, 10)]));
        assert!(due(&[count(TODAY, DayKind::QuitOn, 10)]));
        assert!(due(&[
            count("2026-09-12", DayKind::OverrideRequest, 6),
            count("2026-09-16", DayKind::QuitOn, 4),
        ]));
        // A cancelled request, a restored network and "não era isso" are not overrides.
        assert!(!due(&[
            count(TODAY, DayKind::OverrideRequest, 9),
            count(TODAY, DayKind::OverrideCancel, 50),
            count(TODAY, DayKind::NetworkRestore, 50),
            count(TODAY, DayKind::NotIt, 50),
        ]));
    }

    #[test]
    fn two_very_hard_check_ins() {
        assert!(!due(&[count(TODAY, DayKind::CheckinVeryBad, 1)]));
        assert!(due(&[count(TODAY, DayKind::CheckinVeryBad, 2)]));
        assert!(due(&[
            count("2026-09-11", DayKind::CheckinVeryBad, 1),
            count("2026-09-17", DayKind::CheckinVeryBad, 1),
        ]));
        assert!(!due(&[count(TODAY, DayKind::CheckinHard, 30)]));
    }

    #[test]
    fn the_thresholds_do_not_add_up_across_kinds_and_nothing_else_counts() {
        let below_each = [
            count(TODAY, DayKind::Lapse, 4),
            count(TODAY, DayKind::OverrideRequest, 9),
            count(TODAY, DayKind::CheckinVeryBad, 1),
        ];
        assert!(!due(&below_each));
        // Blocks, vision hits, panic presses and surf sessions never raise the card.
        for kind in [
            DayKind::BlockList,
            DayKind::BlockUser,
            DayKind::BlockKeyword,
            DayKind::VisionHit,
            DayKind::VisionCheck,
            DayKind::Panic,
            DayKind::SurfStarted,
            DayKind::Drift,
            DayKind::HelpShown,
        ] {
            assert!(!due(&[count(TODAY, kind, 1_000)]), "{kind:?}");
        }
    }

    #[test]
    fn the_card_is_shown_at_most_once_in_seven_days() {
        let counts = [count(TODAY, DayKind::Lapse, 9)];
        let shown = |text: &str| {
            let date = day(text);
            Some(Utc.from_utc_datetime(&date.and_hms_opt(12, 0, 0).unwrap()))
        };
        assert!(help_due(&counts, None, day(TODAY)));
        assert!(!help_due(&counts, shown("2026-09-17"), day(TODAY)));
        assert!(
            !help_due(&counts, shown("2026-09-11"), day(TODAY)),
            "six days ago"
        );
        assert!(
            help_due(&counts, shown("2026-09-10"), day(TODAY)),
            "seven days ago"
        );
        assert!(help_due(&counts, shown("2026-01-01"), day(TODAY)));
        // The UTC date of "just now" can be tomorrow's: still shown recently.
        assert!(!help_due(&counts, shown("2026-09-18"), day(TODAY)));
        // A time far in the future (the clock was set back) does not silence the card.
        assert!(help_due(&counts, shown("2026-12-25"), day(TODAY)));
        // Without a threshold reached, nothing is due however long ago it was shown.
        assert!(!help_due(&[], shown("2026-01-01"), day(TODAY)));
    }

    #[test]
    fn the_embedded_resources_are_the_five_of_the_design() {
        let embedded = resources();
        let ids: Vec<&str> = embedded.iter().map(|entry| entry.id.as_str()).collect();
        assert_eq!(ids, vec!["cvv", "samu", "caps", "crp", "findahelpline"]);
        assert_eq!(parse_resources(RESOURCES_JSON).unwrap(), embedded);
        let by_id = |id: &str| embedded.iter().find(|entry| entry.id == id).unwrap();
        assert_eq!(by_id("cvv").phone.as_deref(), Some("188"));
        assert_eq!(by_id("cvv").url.as_deref(), Some("https://cvv.org.br/chat"));
        assert_eq!(by_id("samu").phone.as_deref(), Some("192"));
        assert_eq!(
            by_id("caps").url.as_deref(),
            Some("https://www.gov.br/saude/pt-br/caps")
        );
        assert_eq!(
            (by_id("crp").phone.as_deref(), by_id("crp").url.as_deref()),
            (None, None)
        );
        assert_eq!(
            by_id("findahelpline").url.as_deref(),
            Some("https://findahelpline.com")
        );
        assert_eq!(by_id("findahelpline").country, "");
        for entry in &embedded {
            assert!(!entry.name.trim().is_empty(), "{}", entry.id);
            assert!(
                !entry.note_pt.trim().is_empty() && !entry.note_en.trim().is_empty(),
                "{}",
                entry.id
            );
            assert!(
                entry.country.is_empty() || entry.country == "BR",
                "{}",
                entry.id
            );
            if let Some(url) = &entry.url {
                assert!(url.starts_with("https://") && !url.contains(' '), "{url}");
            }
            if let Some(phone) = &entry.phone {
                assert!(phone.bytes().all(|b| b.is_ascii_digit()), "{phone}");
            }
            // A verification is a date and a name, or nothing at all.
            assert_eq!(
                entry.verified_on.is_empty(),
                entry.verified_by.trim().is_empty(),
                "{}",
                entry.id
            );
            if !entry.verified_on.is_empty() {
                assert!(
                    NaiveDate::parse_from_str(&entry.verified_on, "%Y-%m-%d").is_ok(),
                    "{}",
                    entry.id
                );
            }
        }
    }

    #[test]
    fn the_wire_form_of_a_resource_has_no_verification_field() {
        let views = resource_views();
        assert_eq!(views.len(), resources().len());
        let json = serde_json::to_value(&views).unwrap();
        for (view, entry) in json.as_array().unwrap().iter().zip(resources()) {
            let fields: Vec<&str> = view
                .as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect();
            assert_eq!(
                fields,
                vec!["country", "id", "name", "note_en", "note_pt", "phone", "url"]
            );
            assert_eq!(view["id"], entry.id);
            assert_eq!(view["note_pt"], entry.note_pt);
        }
    }

    #[test]
    fn a_resource_file_that_is_not_a_list_of_entries_is_an_error() {
        assert_eq!(parse_resources("[]").unwrap(), Vec::new());
        assert!(parse_resources("").is_err());
        assert!(parse_resources("{}").is_err());
        assert!(parse_resources(r#"[{ "id": "x" }]"#).is_err());
    }

    /// Blocks a release until a person has opened each site and checked each number, and
    /// has written the date and their name into `resources.json`. Runs in release mode
    /// only: `cargo test -p ubiqx-nofap --release resources_verified`.
    #[cfg(not(debug_assertions))]
    #[test]
    fn resources_verified() {
        let unverified: Vec<String> = resources()
            .into_iter()
            .filter(|entry| {
                entry.verified_on.trim().is_empty() || entry.verified_by.trim().is_empty()
            })
            .map(|entry| entry.id)
            .collect();
        assert!(
            unverified.is_empty(),
            "help resources nobody has verified yet: {unverified:?}. Open each site, call or \
             check each number, then fill `verified_on` (YYYY-MM-DD) and `verified_by` in \
             crates/ubiqx-nofap/data/resources.json."
        );
        assert!(!resources().is_empty(), "the resource list is empty");
    }

    fn list(pt: &[&str], en: &[&str]) -> ForbiddenWords {
        ForbiddenWords {
            pt: pt.iter().map(|word| (*word).to_owned()).collect(),
            en: en.iter().map(|word| (*word).to_owned()).collect(),
        }
    }

    #[test]
    fn forbidden_words_match_whole_words_whatever_the_case_or_accent() {
        let words = list(
            &["vício", "limpo", "força de vontade", "fracass*"],
            &["sin", "addict*", "never again"],
        );
        let found = |text: &str| forbidden_in(text, &words);
        assert_eq!(found("Isso é um VÍCIO."), vec!["vício"]);
        assert_eq!(found("isso e um vicio"), vec!["vício"]);
        assert_eq!(found("Fique limpo!"), vec!["limpo"]);
        // Whole words only.
        assert!(found("Olímpico, limpou e vicioso.").is_empty());
        assert!(found("Since then: a single basin, using it.").is_empty());
        assert_eq!(found("It is a sin."), vec!["sin"]);
        // Several words in a row, whatever separates them.
        assert_eq!(found("Falta força de vontade."), vec!["força de vontade"]);
        assert_eq!(found("FORCA  DE\nVONTADE"), vec!["força de vontade"]);
        assert!(found("força da vontade, vontade de força").is_empty());
        assert_eq!(found("Never, again!"), vec!["never again"]);
        // Stems.
        assert_eq!(found("Você fracassou."), vec!["fracass*"]);
        assert_eq!(found("um fracasso"), vec!["fracass*"]);
        assert_eq!(found("Addiction, addicted, addicts"), vec!["addict*"]);
        assert!(found("fracas e raddict").is_empty());
        // Both lists apply to every text, and each entry is named once.
        assert_eq!(found("vício, vicio, sin, SIN"), vec!["vício", "sin"]);
        assert!(found("").is_empty());
        assert!(forbidden_in("vício", &ForbiddenWords::default()).is_empty());
        assert!(forbidden_in("anything", &list(&["", "*"], &[" "])).is_empty());
    }

    #[test]
    fn the_word_list_is_the_one_of_the_design_and_the_resources_use_none_of_it() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/data/forbidden-words.json");
        let words: ForbiddenWords =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        for entry in [
            "viciado",
            "vício",
            "recaída",
            "fracass*",
            "vergonha",
            "para sempre",
            "fapstronaut",
        ] {
            assert!(words.pt.iter().any(|word| word == entry), "{entry}");
        }
        for entry in [
            "addict*",
            "relapse*",
            "shame",
            "willpower",
            "never again",
            "forever",
        ] {
            assert!(words.en.iter().any(|word| word == entry), "{entry}");
        }
        assert_eq!((words.pt.len(), words.en.len()), (25, 16));
        // The list catches what it is for.
        assert_eq!(
            forbidden_in(
                "Você é um viciado, que vergonha. You relapsed; shame on you.",
                &words
            ),
            vec!["viciado", "vergonha", "relapse*", "shame"]
        );
        for entry in resources() {
            for text in [&entry.name, &entry.note_pt, &entry.note_en] {
                assert_eq!(
                    forbidden_in(text, &words),
                    Vec::<String>::new(),
                    "{}: {text}",
                    entry.id
                );
            }
        }
    }
}
