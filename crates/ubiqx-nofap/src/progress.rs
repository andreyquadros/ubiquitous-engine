//! Progress as the user defined it: "nos últimos 30 dias, {days} foram do jeito que você
//! queria", and the optional streak (design section 5.5). Both are derived from the hourly
//! counters; nothing else is stored.
//!
//! Seeded by the foundation unit: complete constants, signatures only.
#![allow(unused_variables)]

use chrono::{DateTime, NaiveDate, Utc};
use ubiqx_core::nofap::DayCount;

/// Days the progress line looks back.
pub const PROGRESS_WINDOW_DAYS: u32 = 30;
/// Counters older than this are pruned by the verify pass.
pub const RETENTION_DAYS: i64 = 90;

/// The local day (`YYYY-MM-DD`) and hour (`0..=23`) of `now` for a UTC offset in seconds:
/// the key of a counter.
pub fn local_day_hour(now: DateTime<Utc>, utc_offset_secs: i32) -> (String, u8) {
    todo!()
}

/// Days without a lapse among the `window` days ending at `today`, counting only days on
/// or after `since` (the day the module was enabled).
pub fn days_your_way(
    counts: &[DayCount],
    since: Option<NaiveDate>,
    today: NaiveDate,
    window: u32,
) -> u32 {
    todo!()
}

/// Consecutive days without a lapse ending at `today`, not reaching before `since`.
pub fn streak(counts: &[DayCount], since: Option<NaiveDate>, today: NaiveDate) -> u32 {
    todo!()
}

/// Whether a lapse was recorded in the `window` days ending at `today`.
pub fn lapse_in_window(counts: &[DayCount], today: NaiveDate, window: u32) -> bool {
    todo!()
}

/// Sum of the counters of `kind` on `day`.
pub fn total_on(counts: &[DayCount], day: &str, kind: ubiqx_core::nofap::DayKind) -> u32 {
    todo!()
}
