//! Episodes and the two-pass confirmation of the guard (design section 1.2).
//!
//! * A host is acted on only when the previous pass saw the same host in the same app.
//! * Hits less than [`EPISODE_GAP_SECS`] apart are one episode: the card at the first hit
//!   and once more at the third, one counter increment per episode. Enforcement itself has
//!   no cooldown: every confirmed hit is acted on.
//!
//! Seeded by the foundation unit: complete types and constants, signatures only.
#![allow(unused_variables, dead_code)]

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
    /// same pair was seen in [`CONFIRM_PASSES`] consecutive passes.
    pub fn observe(&mut self, app_id: &str, key: u64) -> bool {
        todo!()
    }

    /// The pass saw nothing blocked: forget the previous one.
    pub fn clear(&mut self) {
        todo!()
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
    /// Records a confirmed hit at `now`.
    pub fn observe(&mut self, now: DateTime<Utc>) -> EpisodeStep {
        todo!()
    }

    /// Whether an episode is running at `now` (its last hit is less than
    /// [`EPISODE_GAP_SECS`] old).
    pub fn in_episode(&self, now: DateTime<Utc>) -> bool {
        todo!()
    }
}
