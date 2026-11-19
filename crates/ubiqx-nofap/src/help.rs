//! When the professional-help card is due, and the help resources (design sections 5.5 and
//! 5.7). The thresholds are product heuristics, not clinical cut-offs, and the card says
//! "isso não é um diagnóstico".
//!
//! Seeded by the foundation unit: complete types and constants, signatures only.
#![allow(unused_variables)]

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use ubiqx_core::nofap::DayCount;
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

/// Whether the help card is due: a threshold is reached inside the window ending at
/// `today` (local day), and the card was not shown in the last [`HELP_EVERY_DAYS`] days.
pub fn help_due(counts: &[DayCount], last_shown: Option<DateTime<Utc>>, today: NaiveDate) -> bool {
    todo!()
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
    todo!()
}

/// The embedded resource list (`data/resources.json`).
pub fn resources() -> Vec<ResourceEntry> {
    todo!()
}

/// The embedded resources as the wire type (without the verification fields).
pub fn resource_views() -> Vec<HelpResource> {
    todo!()
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

/// The entries of `words` found in `text` as whole words, case- and accent-insensitively.
pub fn forbidden_in(text: &str, words: &ForbiddenWords) -> Vec<String> {
    todo!()
}
