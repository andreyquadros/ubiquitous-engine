//! ubiqX's privacy lists (`blocked_apps`, `vision_denied_apps`, `blocked_domains`) are
//! honoured by the AI check, but are not an instant off switch (design section 4.2): an
//! entry present when the module was enabled is effective at once; an entry added later is
//! effective for vision after the commitment delay (it is effective for the tracker at
//! once, as today).
//!
//! # Clock changes
//!
//! The wait of a new entry is over when the wall clock is `delay` past its first sighting
//! **and** the app has been observed running for `min(delay, 15 minutes)` since then, the
//! same rule as a waiting request (`commitment`). The observed time grows by at most 150
//! seconds per verify pass whatever the clock says ([`observe`]), so setting the clock
//! forward still costs up to 15 minutes of real running time. A first sighting that lies
//! in the future (the clock was set back after the fact) never counts as passed, and
//! [`reconcile_since`] remembers nothing new while the clock reads earlier than the
//! enabling instant, so an entry added then cannot pass for one that was there when the
//! module was enabled.

use std::collections::HashSet;

use chrono::{DateTime, Utc};
use ubiqx_core::nofap::ExclusionSeen;
use ubiqx_core::normalize::domain_matches;

use crate::commitment::{OBSERVED_CAP_SECS, OBSERVE_STEP_MAX_SECS};

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

/// The keys of `lists` in list order (apps, then domains), each once.
fn list_keys(lists: &ExclusionLists) -> Vec<String> {
    let mut known: HashSet<String> = HashSet::new();
    lists
        .apps
        .iter()
        .map(|app| app_key(app))
        .chain(lists.domains.iter().map(|domain| domain_key(domain)))
        .filter(|key| known.insert(key.clone()))
        .collect()
}

/// `seen` brought in line with `lists`: an entry not yet remembered gets `now` as its first
/// sighting and no observed running time, an entry already remembered keeps both, and an
/// entry no longer on the lists is dropped. The result is in list order (apps, then
/// domains); the caller stores it when it differs from `seen`. Called with the enabling
/// instant when the module is enabled, which is what makes the entries present at that
/// moment effective at once.
///
/// An entry that leaves a list and comes back is new again: its wait starts over.
pub fn reconcile(
    lists: &ExclusionLists,
    seen: &[ExclusionSeen],
    now: DateTime<Utc>,
) -> Vec<ExclusionSeen> {
    reconcile_since(lists, seen, None, now)
}

/// [`reconcile`] for a module enabled at `enabled_at`. While the clock reads earlier than
/// the enabling instant (it was set back), an entry not yet remembered stays unremembered,
/// and so not effective: recorded with that time it would pass for an entry that was there
/// when the module was enabled. It is remembered, and its wait starts, at the first pass
/// after the clock is at or past the enabling instant again. With `None` (the module was
/// never enabled) this is [`reconcile`].
pub fn reconcile_since(
    lists: &ExclusionLists,
    seen: &[ExclusionSeen],
    enabled_at: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> Vec<ExclusionSeen> {
    let clock_behind_enabling = enabled_at.is_some_and(|enabled| now < enabled);
    list_keys(lists)
        .into_iter()
        .filter_map(|key| match seen.iter().find(|entry| entry.key == key) {
            Some(entry) => Some(entry.clone()),
            None if clock_behind_enabling => None,
            None => Some(ExclusionSeen {
                key,
                first_seen_at: now,
                observed_secs: 0,
            }),
        })
        .collect()
}

/// One verify pass: adds `min(elapsed_secs, OBSERVE_STEP_MAX_SECS)` to the observed
/// running time of every remembered entry, up to `OBSERVED_CAP_SECS` (more is never
/// asked for, so an entry stops changing there). Returns whether anything changed, which
/// is when the caller stores `seen`.
///
/// `elapsed_secs` is the running time since the previous pass, the same value that feeds
/// `commitment::tick`.
pub fn observe(seen: &mut [ExclusionSeen], elapsed_secs: u32) -> bool {
    let step = elapsed_secs.min(OBSERVE_STEP_MAX_SECS);
    let mut changed = false;
    for entry in seen {
        let observed = entry
            .observed_secs
            .saturating_add(step)
            .min(OBSERVED_CAP_SECS)
            .max(entry.observed_secs);
        changed |= observed != entry.observed_secs;
        entry.observed_secs = observed;
    }
    changed
}

/// Whether the wait of a new entry is over: the clock is `delay_secs` past the first
/// sighting **and** `observed_secs >= min(delay_secs, OBSERVED_CAP_SECS)`.
fn waited(entry: &ExclusionSeen, delay_secs: u32, now: DateTime<Utc>) -> bool {
    (now - entry.first_seen_at).num_seconds() >= i64::from(delay_secs)
        && entry.observed_secs >= delay_secs.min(OBSERVED_CAP_SECS)
}

/// The entries of `lists` that are effective for vision at `now`: first seen at or before
/// `enabled_at`, or at least `delay_secs` ago by the clock with
/// `min(delay_secs, OBSERVED_CAP_SECS)` of observed running time since. An entry `seen`
/// does not know is not effective yet.
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
                    || waited(entry, delay_secs, now)
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

    /// `seen` after the app was observed running for `secs`, in passes of 45 s.
    fn ran(seen: &[ExclusionSeen], secs: u32) -> Vec<ExclusionSeen> {
        let mut seen = seen.to_vec();
        let mut left = secs;
        while left > 0 {
            let step = left.min(45);
            observe(&mut seen, step);
            left -= step;
        }
        seen
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
    fn an_entry_added_later_waits_the_delay_by_the_clock_and_by_running_time() {
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
        // One second short on both counts, then on each count alone.
        assert_eq!(
            effective(&after, &ran(&seen, 599), Some(enabled_at), DELAY, at(1_599)),
            before
        );
        assert_eq!(
            effective(&after, &ran(&seen, 600), Some(enabled_at), DELAY, at(1_599)),
            before,
            "the running time alone is not enough"
        );
        assert_eq!(
            effective(&after, &ran(&seen, 599), Some(enabled_at), DELAY, at(1_600)),
            before,
            "the clock alone is not enough"
        );
        assert_eq!(
            effective(&after, &ran(&seen, 600), Some(enabled_at), DELAY, at(1_600)),
            after
        );
        // A longer delay keeps it waiting, and asks for 15 minutes of running time, not
        // for the whole hour.
        assert_eq!(
            effective(&after, &ran(&seen, 900), Some(enabled_at), 3_600, at(1_600)),
            before
        );
        assert_eq!(
            effective(&after, &ran(&seen, 899), Some(enabled_at), 3_600, at(4_600)),
            before
        );
        assert_eq!(
            effective(&after, &ran(&seen, 900), Some(enabled_at), 3_600, at(4_600)),
            after
        );
        // No delay makes it effective at once.
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
        let seen = ran(&reconcile(&both, &[], at(0)), 300);
        let only_app = lists(&["com.example.notes"], &[]);
        let seen = reconcile(&only_app, &seen, at(50));
        assert_eq!(
            seen,
            vec![ExclusionSeen {
                key: "app:com.example.notes".into(),
                first_seen_at: at(0),
                observed_secs: 300,
            }]
        );
        let seen = reconcile(&both, &seen, at(100));
        assert_eq!(
            seen[1],
            ExclusionSeen {
                key: "domain:intranet.example.com".into(),
                first_seen_at: at(100),
                observed_secs: 0,
            },
            "the running time observed before it left does not come back with it"
        );
        let seen = ran(&seen, 600);
        assert_eq!(
            effective(&both, &seen, Some(at(0)), DELAY, at(699)),
            only_app
        );
        assert_eq!(effective(&both, &seen, Some(at(0)), DELAY, at(700)), both);
    }

    #[test]
    fn reconcile_keeps_list_order_first_sightings_and_running_time() {
        let seen = vec![
            ExclusionSeen {
                key: "domain:b.example".into(),
                first_seen_at: at(-500),
                observed_secs: 240,
            },
            ExclusionSeen {
                key: "app:gone".into(),
                first_seen_at: at(-900),
                observed_secs: 900,
            },
        ];
        let current = lists(&["New App", "new app"], &["B.example", "a.example"]);
        assert_eq!(
            reconcile(&current, &seen, at(0)),
            vec![
                ExclusionSeen {
                    key: "app:new app".into(),
                    first_seen_at: at(0),
                    observed_secs: 0,
                },
                ExclusionSeen {
                    key: "domain:b.example".into(),
                    first_seen_at: at(-500),
                    observed_secs: 240,
                },
                ExclusionSeen {
                    key: "domain:a.example".into(),
                    first_seen_at: at(0),
                    observed_secs: 0,
                },
            ]
        );
        // Nothing changed: the result equals the input, so the caller stores nothing.
        let again = reconcile(&current, &reconcile(&current, &seen, at(0)), at(77));
        assert_eq!(again, reconcile(&current, &seen, at(0)));
    }

    #[test]
    fn observe_adds_at_most_one_step_per_pass_and_stops_at_the_cap() {
        let mut seen = reconcile(&lists(&["com.example.notes"], &["a.example"]), &[], at(0));
        assert!(!observe(&mut seen, 0), "no time passed, nothing to store");
        assert!(observe(&mut seen, 45));
        assert_eq!(seen[0].observed_secs, 45);
        assert_eq!(seen[1].observed_secs, 45);
        // A pass that claims a day (the machine slept, or the clock moved) adds one step.
        assert!(observe(&mut seen, 86_400));
        assert_eq!(seen[0].observed_secs, 45 + OBSERVE_STEP_MAX_SECS);
        assert!(observe(&mut seen, u32::MAX));
        assert_eq!(seen[0].observed_secs, 45 + 2 * OBSERVE_STEP_MAX_SECS);
        // It stops at the cap, and a pass that changes nothing says so.
        for _ in 0..3 {
            assert!(observe(&mut seen, OBSERVE_STEP_MAX_SECS));
        }
        assert_eq!(seen[0].observed_secs, 795);
        assert!(observe(&mut seen, OBSERVE_STEP_MAX_SECS));
        assert_eq!(seen[0].observed_secs, OBSERVED_CAP_SECS);
        assert_eq!(seen[1].observed_secs, OBSERVED_CAP_SECS);
        assert!(!observe(&mut seen, OBSERVE_STEP_MAX_SECS));
        assert_eq!(seen[0].observed_secs, OBSERVED_CAP_SECS);
        // Only the entries below the cap change.
        seen[1].observed_secs = 880;
        assert!(observe(&mut seen, 45));
        assert_eq!(seen[0].observed_secs, OBSERVED_CAP_SECS);
        assert_eq!(seen[1].observed_secs, OBSERVED_CAP_SECS);
        assert!(!observe(&mut [], 45));
    }

    #[test]
    fn a_clock_set_forward_does_not_make_a_new_entry_effective() {
        let added = lists(&["com.example.notes"], &[]);
        let enabled_at = Some(at(0));
        let mut seen = reconcile(&added, &[], at(1_000));
        // The clock jumps a day ahead right after the entry was added.
        let jumped = at(1_000 + 86_400);
        assert_eq!(
            effective(&added, &seen, enabled_at, DELAY, jumped),
            ExclusionLists::default()
        );
        // Each pass reports the jump as elapsed time and is credited one step: 600 s of
        // running time take four passes.
        for pass in 1..=3 {
            assert!(observe(&mut seen, 86_400));
            assert_eq!(
                effective(&added, &seen, enabled_at, DELAY, jumped),
                ExclusionLists::default(),
                "pass {pass}"
            );
        }
        assert!(observe(&mut seen, 86_400));
        assert_eq!(effective(&added, &seen, enabled_at, DELAY, jumped), added);

        // With a delay of an hour or more the jump still costs 15 minutes of running.
        let mut seen = reconcile(&added, &[], at(1_000));
        for _ in 0..5 {
            observe(&mut seen, 86_400);
        }
        assert_eq!(seen[0].observed_secs, 750);
        assert_eq!(
            effective(&added, &seen, enabled_at, 86_400, jumped),
            ExclusionLists::default()
        );
        observe(&mut seen, 86_400);
        assert_eq!(effective(&added, &seen, enabled_at, 86_400, jumped), added);
    }

    #[test]
    fn a_clock_set_back_does_not_make_a_new_entry_effective() {
        let added = lists(&["com.example.notes"], &[]);
        let seen = reconcile(&added, &[], at(1_000));
        assert_eq!(
            effective(&added, &seen, Some(at(0)), DELAY, at(-86_400)),
            ExclusionLists::default()
        );
        // Not with all the running time either: the first sighting lies in the future.
        let seen = ran(&seen, 900);
        assert_eq!(
            effective(&added, &seen, Some(at(0)), DELAY, at(-86_400)),
            ExclusionLists::default()
        );
        assert_eq!(
            effective(&added, &seen, Some(at(0)), DELAY, at(999)),
            ExclusionLists::default()
        );
        // The running time is kept: when the clock is right again the wait is the clock's.
        assert_eq!(
            effective(&added, &seen, Some(at(0)), DELAY, at(1_599)),
            ExclusionLists::default()
        );
        assert_eq!(
            effective(&added, &seen, Some(at(0)), DELAY, at(1_600)),
            added
        );
        // Without an enabling time only the delay counts.
        assert_eq!(
            effective(&added, &seen, None, DELAY, at(1_599)),
            ExclusionLists::default()
        );
        assert_eq!(effective(&added, &seen, None, DELAY, at(1_600)), added);
    }

    #[test]
    fn an_entry_added_while_the_clock_is_behind_the_enabling_instant_is_not_remembered() {
        let enabled_at = at(0);
        let before = lists(&["com.example.notes"], &[]);
        let seen = reconcile_since(&before, &[], Some(enabled_at), enabled_at);
        assert_eq!(seen.len(), 1, "the enabling instant itself is not behind");
        // The clock is set back a day and a second entry is added.
        let after = lists(&["com.example.notes"], &["videos.example.org"]);
        let back = at(-86_400);
        let while_back = reconcile_since(&after, &seen, Some(enabled_at), back);
        assert_eq!(
            while_back, seen,
            "the entry known before stays, nothing is added"
        );
        // The clock is set right again: the new entry has not passed for an old one.
        assert_eq!(
            effective(&after, &while_back, Some(enabled_at), DELAY, at(2_000)),
            before
        );
        // It is remembered from the first pass with a clock at or past the enabling
        // instant, and waits like any new entry.
        let seen = reconcile_since(&after, &while_back, Some(enabled_at), at(2_000));
        assert_eq!(
            seen[1],
            ExclusionSeen {
                key: "domain:videos.example.org".into(),
                first_seen_at: at(2_000),
                observed_secs: 0,
            }
        );
        assert_eq!(
            effective(&after, &ran(&seen, 600), Some(enabled_at), DELAY, at(2_599)),
            before
        );
        assert_eq!(
            effective(&after, &ran(&seen, 600), Some(enabled_at), DELAY, at(2_600)),
            after
        );
        // An entry that left the lists is dropped while the clock is behind too.
        assert_eq!(
            reconcile_since(&lists(&[], &[]), &seen, Some(enabled_at), back),
            vec![]
        );
        // A module that was never enabled has no enabling instant to be behind of.
        assert_eq!(
            reconcile_since(&after, &[], None, back),
            reconcile(&after, &[], back)
        );
        assert_eq!(reconcile(&after, &[], back).len(), 2);
    }

    /// One verify pass as the service runs it: reconcile, then credit the running time.
    fn pass(
        lists: &ExclusionLists,
        seen: &[ExclusionSeen],
        enabled_at: DateTime<Utc>,
        elapsed_secs: u32,
        now: DateTime<Utc>,
    ) -> Vec<ExclusionSeen> {
        let mut seen = reconcile_since(lists, seen, Some(enabled_at), now);
        observe(&mut seen, elapsed_secs);
        seen
    }

    /// `seen` as it is after the app was closed and opened again: written and read back.
    fn restarted(seen: &[ExclusionSeen]) -> Vec<ExclusionSeen> {
        serde_json::from_str(&serde_json::to_string(seen).expect("written")).expect("read back")
    }

    #[test]
    fn the_running_time_survives_a_restart_and_a_restart_adds_none() {
        let enabled_at = at(0);
        let before = lists(&["com.example.notes"], &[]);
        let after = lists(&["com.example.notes"], &["videos.example.org"]);
        let mut seen = reconcile(&before, &[], enabled_at);
        // The entry is added, and the clock is set a week ahead before the next pass.
        let week = 7 * 86_400;
        seen = pass(&after, &seen, enabled_at, 45, at(1_000));
        assert_eq!(seen[1].observed_secs, 45);
        // The app is closed and opened three times; every first pass after a start claims
        // the whole week as elapsed time and is credited one step.
        for start in 1..=3 {
            seen = restarted(&seen);
            seen = pass(&after, &seen, enabled_at, 604_800, at(1_000 + week + start));
            assert_eq!(seen[1].first_seen_at, at(1_000), "start {start}");
            assert_eq!(
                effective(&after, &seen, Some(enabled_at), DELAY, at(2_000 + week)),
                before,
                "start {start}: {} s of running time",
                seen[1].observed_secs
            );
        }
        assert_eq!(seen[1].observed_secs, 45 + 3 * OBSERVE_STEP_MAX_SECS);
        // A restart alone, with no pass in between, changes nothing.
        assert_eq!(restarted(&restarted(&seen)), seen);
        // The fourth pass reaches the 600 s the delay asks for.
        seen = pass(&after, &restarted(&seen), enabled_at, 45, at(2_000 + week));
        assert_eq!(seen[1].observed_secs, 540);
        assert_eq!(
            effective(&after, &seen, Some(enabled_at), DELAY, at(2_000 + week)),
            before
        );
        seen = pass(&after, &restarted(&seen), enabled_at, 60, at(2_060 + week));
        assert_eq!(
            effective(&after, &seen, Some(enabled_at), DELAY, at(2_060 + week)),
            after
        );
        // The entry that was there at enabling was effective all along, whatever the clock.
        for now in [at(-week), at(0), at(week)] {
            assert_eq!(
                effective(&after, &seen, Some(enabled_at), DELAY, now).apps,
                before.apps
            );
        }
    }

    #[test]
    fn entries_stored_by_a_build_without_the_running_time_wait_it_out() {
        // What an earlier build stored: no `observed_secs`. One entry from the enabling
        // instant, one added later that had already waited its delay by the clock.
        let enabled_at = at(0);
        let stored = format!(
            r#"[{{"key":"app:com.example.notes","first_seen_at":"{}"}},
                {{"key":"domain:videos.example.org","first_seen_at":"{}"}}]"#,
            at(0).to_rfc3339(),
            at(500).to_rfc3339()
        );
        let mut seen: Vec<ExclusionSeen> = serde_json::from_str(&stored).expect("old data reads");
        assert_eq!(seen[0].observed_secs, 0);
        assert_eq!(seen[1].observed_secs, 0);
        let both = lists(&["com.example.notes"], &["videos.example.org"]);
        let only_app = lists(&["com.example.notes"], &[]);
        let now = at(86_400);
        // The entry of the enabling instant needs no running time.
        assert_eq!(
            effective(&both, &seen, Some(enabled_at), DELAY, now),
            only_app
        );
        // The later one gets it pass by pass, and keeps its first sighting.
        for n in 1..=13 {
            seen = pass(
                &both,
                &seen,
                enabled_at,
                45,
                now + Duration::seconds(45 * n),
            );
            assert_eq!(
                effective(&both, &seen, Some(enabled_at), DELAY, now),
                only_app,
                "pass {n}"
            );
        }
        seen = pass(&both, &seen, enabled_at, 45, now + Duration::seconds(630));
        assert_eq!(seen[1].first_seen_at, at(500));
        assert_eq!(seen[1].observed_secs, 630);
        assert_eq!(effective(&both, &seen, Some(enabled_at), DELAY, now), both);
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
