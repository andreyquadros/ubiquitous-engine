//! ubiqX's privacy lists (`blocked_apps`, `vision_denied_apps`, `blocked_domains`) are
//! honoured by the AI check, but are not an instant off switch (design section 4.2): an
//! entry present when the module was enabled is effective at once; an entry added later is
//! effective for vision after the commitment delay (it is effective for the tracker at
//! once, as today).
//!
//! Seeded by the foundation unit: complete types, signatures only.
#![allow(unused_variables)]

use chrono::{DateTime, Utc};
use ubiqx_core::nofap::ExclusionSeen;

/// The host's privacy lists, as `NofapEnvSnapshot` carries them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExclusionLists {
    pub apps: Vec<String>,
    pub domains: Vec<String>,
}

/// The key under which an application entry is remembered (`app:<lower-case id>`).
pub fn app_key(app: &str) -> String {
    todo!()
}

/// The key under which a domain entry is remembered (`domain:<lower-case name>`).
pub fn domain_key(domain: &str) -> String {
    todo!()
}

/// `seen` brought in line with `lists`: an entry not yet remembered gets `now` as its first
/// sighting, an entry already remembered keeps its time, and an entry no longer on the
/// lists is dropped. The result is in list order (apps, then domains); the caller stores
/// it when it differs from `seen`. Called with the enabling instant when the module is
/// enabled, which is what makes the entries present at that moment effective at once.
pub fn reconcile(
    lists: &ExclusionLists,
    seen: &[ExclusionSeen],
    now: DateTime<Utc>,
) -> Vec<ExclusionSeen> {
    todo!()
}

/// The entries of `lists` that are effective for vision at `now`: first seen at or before
/// `enabled_at`, or at least `delay_secs` ago. An entry `seen` does not know is not
/// effective yet.
pub fn effective(
    lists: &ExclusionLists,
    seen: &[ExclusionSeen],
    enabled_at: Option<DateTime<Utc>>,
    delay_secs: u32,
    now: DateTime<Utc>,
) -> ExclusionLists {
    todo!()
}

/// Whether a window of `app_id` / `app_name`, or a page of `domain`, falls under the
/// effective lists (same matching as the tracker's privacy lists: case-insensitive app id
/// or name, domain or subdomain).
pub fn excludes(
    effective: &ExclusionLists,
    app_id: &str,
    app_name: &str,
    domain: Option<&str>,
) -> bool {
    todo!()
}
