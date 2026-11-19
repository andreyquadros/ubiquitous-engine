//! ubiqX's privacy lists (`blocked_apps`, `vision_denied_apps`, `blocked_domains`) are
//! honoured by the AI check, but are not an instant off switch (design section 4.2): an
//! entry present when the module was enabled is effective at once; an entry added later is
//! effective for vision after the commitment delay (it is effective for the tracker at
//! once, as today).

use std::collections::HashSet;

use chrono::{DateTime, Utc};
use ubiqx_core::nofap::ExclusionSeen;
use ubiqx_core::normalize::domain_matches;

/// The host's privacy lists, as `NofapEnvSnapshot` carries them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExclusionLists {
    pub apps: Vec<String>,
    pub domains: Vec<String>,
}

/// The key under which an application entry is remembered (`app:<lower-case id>`).
pub fn app_key(app: &str) -> String {
    format!("app:{}", app.trim().to_lowercase())
}

/// The key under which a domain entry is remembered (`domain:<lower-case name>`).
pub fn domain_key(domain: &str) -> String {
    format!("domain:{}", domain.trim().to_lowercase())
}

/// `seen` brought in line with `lists`: an entry not yet remembered gets `now` as its first
/// sighting, an entry already remembered keeps its time, and an entry no longer on the
/// lists is dropped. The result is in list order (apps, then domains); the caller stores
/// it when it differs from `seen`. Called with the enabling instant when the module is
/// enabled, which is what makes the entries present at that moment effective at once.
///
/// An entry that leaves a list and comes back is new again: its wait starts over.
pub fn reconcile(
    lists: &ExclusionLists,
    seen: &[ExclusionSeen],
    now: DateTime<Utc>,
) -> Vec<ExclusionSeen> {
    let keys = lists
        .apps
        .iter()
        .map(|app| app_key(app))
        .chain(lists.domains.iter().map(|domain| domain_key(domain)));
    let mut known: HashSet<String> = HashSet::new();
    let mut reconciled = Vec::new();
    for key in keys {
        if !known.insert(key.clone()) {
            continue;
        }
        let first_seen_at = seen
            .iter()
            .find(|entry| entry.key == key)
            .map_or(now, |entry| entry.first_seen_at);
        reconciled.push(ExclusionSeen { key, first_seen_at });
    }
    reconciled
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
    let is_effective = |key: String| {
        seen.iter()
            .find(|entry| entry.key == key)
            .is_some_and(|entry| {
                enabled_at.is_some_and(|enabled| entry.first_seen_at <= enabled)
                    || (now - entry.first_seen_at).num_seconds() >= i64::from(delay_secs)
            })
    };
    ExclusionLists {
        apps: lists
            .apps
            .iter()
            .filter(|app| is_effective(app_key(app)))
            .cloned()
            .collect(),
        domains: lists
            .domains
            .iter()
            .filter(|domain| is_effective(domain_key(domain)))
            .cloned()
            .collect(),
    }
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
    let app = effective
        .apps
        .iter()
        .any(|entry| entry.eq_ignore_ascii_case(app_id) || entry.eq_ignore_ascii_case(app_name));
    app || domain.is_some_and(|domain| {
        effective
            .domains
            .iter()
            .any(|pattern| domain_matches(domain, pattern))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone};

    const DELAY: u32 = 600;

    fn at(secs: i64) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 9, 17, 15, 0, 0).unwrap() + Duration::seconds(secs)
    }

    fn lists(apps: &[&str], domains: &[&str]) -> ExclusionLists {
        ExclusionLists {
            apps: apps.iter().map(|app| (*app).to_owned()).collect(),
            domains: domains.iter().map(|domain| (*domain).to_owned()).collect(),
        }
    }

    #[test]
    fn keys_ignore_case_and_blanks() {
        assert_eq!(app_key(" Com.Example.App "), "app:com.example.app");
        assert_eq!(
            domain_key("Intranet.Example.COM"),
            "domain:intranet.example.com"
        );
        assert_ne!(app_key("example.com"), domain_key("example.com"));
    }

    #[test]
    fn entries_present_when_the_module_is_enabled_are_effective_at_once() {
        let enabled_at = at(0);
        let present = lists(&["com.example.notes"], &["intranet.example.com"]);
        let seen = reconcile(&present, &[], enabled_at);
        assert_eq!(seen.len(), 2);
        assert_eq!(
            effective(&present, &seen, Some(enabled_at), DELAY, at(0)),
            present
        );
        assert_eq!(
            effective(&present, &seen, Some(enabled_at), DELAY, at(1)),
            present
        );
    }

    #[test]
    fn an_entry_added_later_waits_the_delay() {
        let enabled_at = at(0);
        let before = lists(&["com.example.notes"], &[]);
        let seen = reconcile(&before, &[], enabled_at);
        let after = lists(
            &["com.example.notes", "Video Player"],
            &["videos.example.org"],
        );
        let seen = reconcile(&after, &seen, at(1_000));
        assert_eq!(
            effective(&after, &seen, Some(enabled_at), DELAY, at(1_000)),
            before,
            "only the entry that was there at enable"
        );
        assert_eq!(
            effective(&after, &seen, Some(enabled_at), DELAY, at(1_599)),
            before
        );
        assert_eq!(
            effective(&after, &seen, Some(enabled_at), DELAY, at(1_600)),
            after
        );
        // A longer delay keeps it waiting; no delay makes it effective at once.
        assert_eq!(
            effective(&after, &seen, Some(enabled_at), 3_600, at(1_600)),
            before
        );
        assert_eq!(
            effective(&after, &seen, Some(enabled_at), 0, at(1_000)),
            after
        );
    }

    #[test]
    fn an_entry_nobody_recorded_is_not_effective() {
        let unknown = lists(&["com.example.notes"], &["intranet.example.com"]);
        assert_eq!(
            effective(&unknown, &[], Some(at(0)), DELAY, at(1_000_000)),
            ExclusionLists::default()
        );
    }

    #[test]
    fn a_removed_entry_is_dropped_and_starts_over_when_it_returns() {
        let both = lists(&["com.example.notes"], &["intranet.example.com"]);
        let seen = reconcile(&both, &[], at(0));
        let only_app = lists(&["com.example.notes"], &[]);
        let seen = reconcile(&only_app, &seen, at(50));
        assert_eq!(
            seen,
            vec![ExclusionSeen {
                key: "app:com.example.notes".into(),
                first_seen_at: at(0)
            }]
        );
        let seen = reconcile(&both, &seen, at(100));
        assert_eq!(
            seen[1],
            ExclusionSeen {
                key: "domain:intranet.example.com".into(),
                first_seen_at: at(100)
            }
        );
        assert_eq!(
            effective(&both, &seen, Some(at(0)), DELAY, at(699)),
            only_app
        );
        assert_eq!(effective(&both, &seen, Some(at(0)), DELAY, at(700)), both);
    }

    #[test]
    fn reconcile_keeps_list_order_and_first_sightings() {
        let seen = vec![
            ExclusionSeen {
                key: "domain:b.example".into(),
                first_seen_at: at(-500),
            },
            ExclusionSeen {
                key: "app:gone".into(),
                first_seen_at: at(-900),
            },
        ];
        let current = lists(&["New App", "new app"], &["B.example", "a.example"]);
        assert_eq!(
            reconcile(&current, &seen, at(0)),
            vec![
                ExclusionSeen {
                    key: "app:new app".into(),
                    first_seen_at: at(0)
                },
                ExclusionSeen {
                    key: "domain:b.example".into(),
                    first_seen_at: at(-500)
                },
                ExclusionSeen {
                    key: "domain:a.example".into(),
                    first_seen_at: at(0)
                },
            ]
        );
        // Nothing changed: the result equals the input, so the caller stores nothing.
        let again = reconcile(&current, &reconcile(&current, &seen, at(0)), at(77));
        assert_eq!(again, reconcile(&current, &seen, at(0)));
    }

    #[test]
    fn a_clock_set_back_does_not_make_a_new_entry_effective() {
        let added = lists(&["com.example.notes"], &[]);
        let seen = reconcile(&added, &[], at(1_000));
        assert_eq!(
            effective(&added, &seen, Some(at(0)), DELAY, at(-86_400)),
            ExclusionLists::default()
        );
        // Without an enabling time only the delay counts.
        assert_eq!(
            effective(&added, &seen, None, DELAY, at(1_599)),
            ExclusionLists::default()
        );
        assert_eq!(effective(&added, &seen, None, DELAY, at(1_600)), added);
    }

    #[test]
    fn excludes_matches_like_the_tracker() {
        let effective = lists(&["com.example.Notes", "Video Player"], &["example.com"]);
        assert!(excludes(&effective, "COM.EXAMPLE.NOTES", "Notes", None));
        assert!(excludes(&effective, "player", "video player", None));
        assert!(excludes(
            &effective,
            "chrome",
            "Chrome",
            Some("example.com")
        ));
        assert!(excludes(
            &effective,
            "chrome",
            "Chrome",
            Some("docs.Example.com")
        ));
        assert!(!excludes(
            &effective,
            "chrome",
            "Chrome",
            Some("notexample.com")
        ));
        assert!(!excludes(
            &effective,
            "chrome",
            "Chrome",
            Some("example.com.evil.org")
        ));
        assert!(!excludes(&effective, "chrome", "Chrome", None));
        assert!(!excludes(
            &ExclusionLists::default(),
            "chrome",
            "Chrome",
            Some("example.com")
        ));
    }
}
