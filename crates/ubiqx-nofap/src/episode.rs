//! Episodes and the two-pass confirmation of the guard (design section 1.2).
//!
//! * A host is acted on only when the previous pass saw the same host in the same app.
//! * Hits less than [`EPISODE_GAP_SECS`] apart are one episode: the card at the first hit
//!   and once more at the third, one counter increment per episode. Enforcement itself has
//!   no cooldown: every confirmed hit is acted on.

use chrono::{DateTime, Utc};

/// Passes that must agree before the guard acts.
pub const CONFIRM_PASSES: u32 = 2;
/// Hits closer than this are one episode.
pub const EPISODE_GAP_SECS: i64 = 120;
/// The hit of an episode at which the "repeat" card is shown.
pub const REPEAT_CARD_AT_HIT: u32 = 3;
/// At most one card in this many seconds, whatever the episode says.
pub const CARD_MIN_GAP_SECS: i64 = 60;

/// Remembers what the previous guard pass saw, so that host-shaped text being typed in an
/// address bar is not acted on.
#[derive(Debug, Clone, Default)]
pub struct PassConfirmer {
    last: Option<(String, u64)>,
    seen: u32,
}

impl PassConfirmer {
    /// Records that this pass saw `key` (a hash of the host) in `app_id`; `true` once the
    /// same pair was seen in [`CONFIRM_PASSES`] consecutive passes, and in every pass after
    /// that for as long as it stays in front (enforcement has no cooldown).
    pub fn observe(&mut self, app_id: &str, key: u64) -> bool {
        let same = self
            .last
            .as_ref()
            .is_some_and(|(app, seen_key)| app == app_id && *seen_key == key);
        if same {
            self.seen = self.seen.saturating_add(1);
        } else {
            self.last = Some((app_id.to_owned(), key));
            self.seen = 1;
        }
        self.seen >= CONFIRM_PASSES
    }

    /// The pass saw nothing blocked: forget the previous one.
    pub fn clear(&mut self) {
        self.last = None;
        self.seen = 0;
    }
}

/// Which card a hit shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EpisodeCard {
    /// The first hit of an episode.
    First,
    /// The third hit of an episode.
    Repeat,
}

/// What one confirmed hit means for the card and the counter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EpisodeStep {
    /// `Some` at the first and at the third hit of an episode.
    pub card: Option<EpisodeCard>,
    /// `true` at the first hit only: one counter increment per episode.
    pub count: bool,
    /// 1-based position of this hit in its episode.
    pub hit: u32,
}

/// Groups hits into episodes.
#[derive(Debug, Clone, Default)]
pub struct EpisodeTracker {
    last_hit_at: Option<DateTime<Utc>>,
    hits: u32,
}

impl EpisodeTracker {
    /// Records a confirmed hit at `now`. A hit less than [`EPISODE_GAP_SECS`] after the
    /// previous one continues its episode; so does a hit that seems to come before the
    /// previous one (the clock was set back), because a clock change must not hand out a
    /// second card and a second count.
    pub fn observe(&mut self, now: DateTime<Utc>) -> EpisodeStep {
        self.hits = if self.in_episode(now) {
            self.hits.saturating_add(1)
        } else {
            1
        };
        self.last_hit_at = Some(now);
        let card = match self.hits {
            1 => Some(EpisodeCard::First),
            REPEAT_CARD_AT_HIT => Some(EpisodeCard::Repeat),
            _ => None,
        };
        EpisodeStep {
            card,
            count: self.hits == 1,
            hit: self.hits,
        }
    }

    /// Whether an episode is running at `now` (its last hit is less than
    /// [`EPISODE_GAP_SECS`] old).
    pub fn in_episode(&self, now: DateTime<Utc>) -> bool {
        self.last_hit_at
            .is_some_and(|last| (now - last).num_seconds() < EPISODE_GAP_SECS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone};

    fn at(secs: i64) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 9, 17, 15, 0, 0).unwrap() + Duration::seconds(secs)
    }

    #[test]
    fn a_host_is_confirmed_by_the_second_consecutive_pass() {
        let mut confirmer = PassConfirmer::default();
        assert!(!confirmer.observe("chrome", 7));
        assert!(confirmer.observe("chrome", 7));
        // No cooldown: it stays confirmed for as long as it is in front.
        assert!(confirmer.observe("chrome", 7));
        assert!(confirmer.observe("chrome", 7));
    }

    #[test]
    fn another_host_or_another_app_starts_over() {
        let mut confirmer = PassConfirmer::default();
        assert!(!confirmer.observe("chrome", 7));
        assert!(!confirmer.observe("chrome", 8), "text still being typed");
        assert!(
            !confirmer.observe("firefox", 8),
            "the same host in another app"
        );
        assert!(confirmer.observe("firefox", 8));
        assert!(!confirmer.observe("chrome", 8));
    }

    #[test]
    fn a_pass_without_a_hit_forgets_the_previous_one() {
        let mut confirmer = PassConfirmer::default();
        assert!(!confirmer.observe("chrome", 7));
        confirmer.clear();
        assert!(!confirmer.observe("chrome", 7));
        assert!(confirmer.observe("chrome", 7));
        confirmer.clear();
        assert!(!confirmer.observe("chrome", 7));
    }

    #[test]
    fn an_episode_shows_a_card_at_the_first_and_third_hit_and_counts_once() {
        let mut tracker = EpisodeTracker::default();
        assert!(!tracker.in_episode(at(0)));
        assert_eq!(
            tracker.observe(at(0)),
            EpisodeStep {
                card: Some(EpisodeCard::First),
                count: true,
                hit: 1
            }
        );
        assert_eq!(
            tracker.observe(at(2)),
            EpisodeStep {
                card: None,
                count: false,
                hit: 2
            }
        );
        assert_eq!(
            tracker.observe(at(40)),
            EpisodeStep {
                card: Some(EpisodeCard::Repeat),
                count: false,
                hit: 3
            }
        );
        for (n, secs) in [(4, 60), (5, 150), (6, 269)] {
            assert_eq!(
                tracker.observe(at(secs)),
                EpisodeStep {
                    card: None,
                    count: false,
                    hit: n
                }
            );
        }
        assert!(tracker.in_episode(at(388)));
        assert!(!tracker.in_episode(at(389)));
    }

    #[test]
    fn a_hit_after_the_gap_starts_a_new_episode() {
        let mut tracker = EpisodeTracker::default();
        tracker.observe(at(0));
        tracker.observe(at(119));
        // 119 s after the last hit continues; exactly 120 s starts over.
        assert_eq!(tracker.observe(at(238)).hit, 3);
        assert_eq!(
            tracker.observe(at(358)),
            EpisodeStep {
                card: Some(EpisodeCard::First),
                count: true,
                hit: 1
            }
        );
    }

    #[test]
    fn a_clock_set_back_does_not_start_a_new_episode() {
        let mut tracker = EpisodeTracker::default();
        tracker.observe(at(0));
        assert_eq!(
            tracker.observe(at(-3_600)),
            EpisodeStep {
                card: None,
                count: false,
                hit: 2
            }
        );
        // From the new time on the gap counts again.
        assert_eq!(tracker.observe(at(-3_600 + 120)).hit, 1);
    }
}
