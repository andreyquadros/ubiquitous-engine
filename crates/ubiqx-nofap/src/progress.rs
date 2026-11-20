//! Progress as the user defined it: "nos últimos 30 dias, {days} foram do jeito que você
//! queria", and the optional streak (design section 5.5). Both are derived from the hourly
//! counters; nothing else is stored.

use chrono::{DateTime, Duration, NaiveDate, Timelike, Utc};
use ubiqx_core::nofap::{DayCount, DayKind};

/// Days the progress line looks back.
pub const PROGRESS_WINDOW_DAYS: u32 = 30;
/// Counters older than this are pruned by the verify pass.
pub const RETENTION_DAYS: i64 = 90;
/// Format of `DayCount::day`.
pub const DAY_FORMAT: &str = "%Y-%m-%d";

/// The local day (`YYYY-MM-DD`) and hour (`0..=23`) of `now` for a UTC offset in seconds:
/// the key of a counter.
pub fn local_day_hour(now: DateTime<Utc>, utc_offset_secs: i32) -> (String, u8) {
    let local = now.naive_utc() + Duration::seconds(i64::from(utc_offset_secs));
    // An hour is 0..=23, so the conversion cannot fail.
    let hour = u8::try_from(local.hour()).unwrap_or(0);
    (local.format(DAY_FORMAT).to_string(), hour)
}

/// `day` as the text a counter is keyed by.
pub fn day_key(day: NaiveDate) -> String {
    day.format(DAY_FORMAT).to_string()
}

/// The day of a counter; `None` for a row whose day is not a date.
fn day_of(count: &DayCount) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(&count.day, DAY_FORMAT).ok()
}

fn lapse_on(counts: &[DayCount], day: NaiveDate) -> bool {
    counts
        .iter()
        .any(|count| count.kind == DayKind::Lapse && count.n > 0 && day_of(count) == Some(day))
}

/// The first of the `window` days ending at `today`, moved up to `since`.
fn first_day(since: NaiveDate, today: NaiveDate, window: u32) -> NaiveDate {
    let back = Duration::days(i64::from(window.saturating_sub(1)));
    today
        .checked_sub_signed(back)
        .map_or(since, |start| start.max(since))
}

/// Days without a lapse among the `window` days ending at `today`, counting only days on
/// or after `since` (the day the module was enabled). Without `since` nothing is known
/// about any day, so the answer is 0.
pub fn days_your_way(
    counts: &[DayCount],
    since: Option<NaiveDate>,
    today: NaiveDate,
    window: u32,
) -> u32 {
    let Some(since) = since else { return 0 };
    if window == 0 {
        return 0;
    }
    let mut days = 0;
    let mut day = first_day(since, today, window);
    while day <= today {
        if !lapse_on(counts, day) {
            days += 1;
        }
        match day.succ_opt() {
            Some(next) => day = next,
            None => break,
        }
    }
    days
}

/// Consecutive days without a lapse ending at `today`, not reaching before `since`.
pub fn streak(counts: &[DayCount], since: Option<NaiveDate>, today: NaiveDate) -> u32 {
    let Some(since) = since else { return 0 };
    let mut days = 0;
    let mut day = today;
    while day >= since && !lapse_on(counts, day) {
        days += 1;
        match day.pred_opt() {
            Some(previous) => day = previous,
            None => break,
        }
    }
    days
}

/// Whether a lapse was recorded in the `window` days ending at `today`.
pub fn lapse_in_window(counts: &[DayCount], today: NaiveDate, window: u32) -> bool {
    if window == 0 {
        return false;
    }
    let start = first_day(NaiveDate::MIN, today, window);
    counts.iter().any(|count| {
        count.kind == DayKind::Lapse
            && count.n > 0
            && day_of(count).is_some_and(|day| day >= start && day <= today)
    })
}

/// Sum of the counters of `kind` on `day`.
pub fn total_on(counts: &[DayCount], day: &str, kind: DayKind) -> u32 {
    counts
        .iter()
        .filter(|count| count.kind == kind && count.day == day)
        .fold(0u32, |sum, count| sum.saturating_add(count.n))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn day(text: &str) -> NaiveDate {
        NaiveDate::parse_from_str(text, DAY_FORMAT).unwrap()
    }

    fn count(day: &str, hour: u8, kind: DayKind, n: u32) -> DayCount {
        DayCount {
            day: day.to_owned(),
            hour,
            kind,
            n,
        }
    }

    #[test]
    fn local_day_and_hour_follow_the_injected_offset() {
        let now = Utc.with_ymd_and_hms(2026, 9, 17, 1, 30, 0).unwrap();
        assert_eq!(local_day_hour(now, 0), ("2026-09-17".to_owned(), 1));
        // Brasília (UTC-3): still the evening before.
        assert_eq!(
            local_day_hour(now, -3 * 3_600),
            ("2026-09-16".to_owned(), 22)
        );
        // Tokyo (UTC+9), and an offset with minutes (UTC+5:30).
        assert_eq!(
            local_day_hour(now, 9 * 3_600),
            ("2026-09-17".to_owned(), 10)
        );
        assert_eq!(local_day_hour(now, 19_800), ("2026-09-17".to_owned(), 7));
        let late = Utc.with_ymd_and_hms(2026, 12, 31, 23, 59, 59).unwrap();
        assert_eq!(local_day_hour(late, 1), ("2027-01-01".to_owned(), 0));
        assert_eq!(local_day_hour(late, -86_399), ("2026-12-31".to_owned(), 0));
    }

    #[test]
    fn days_your_way_counts_lapse_free_days_since_the_module_was_enabled() {
        let today = day("2026-09-17");
        let counts = vec![
            count("2026-09-10", 23, DayKind::Lapse, 1),
            count("2026-09-10", 3, DayKind::Lapse, 2),
            count("2026-09-15", 9, DayKind::Lapse, 1),
            count("2026-09-16", 9, DayKind::Panic, 4),
            count("2026-09-17", 9, DayKind::Lapse, 0),
            count("2026-08-01", 9, DayKind::Lapse, 1),
            count("not a day", 9, DayKind::Lapse, 1),
        ];
        // Enabled long ago: 30 days in the window, two of them with a lapse.
        assert_eq!(
            days_your_way(&counts, Some(day("2026-01-01")), today, 30),
            28
        );
        // Enabled on the 12th: six days, one with a lapse.
        assert_eq!(
            days_your_way(&counts, Some(day("2026-09-12")), today, 30),
            5
        );
        // Enabled today.
        assert_eq!(days_your_way(&counts, Some(today), today, 30), 1);
        // A shorter window: the 11th to the 17th.
        assert_eq!(days_your_way(&counts, Some(day("2026-01-01")), today, 7), 6);
        assert_eq!(days_your_way(&[], Some(day("2026-01-01")), today, 30), 30);
    }

    #[test]
    fn nothing_is_claimed_about_days_the_module_did_not_see() {
        let today = day("2026-09-17");
        assert_eq!(days_your_way(&[], None, today, 30), 0);
        assert_eq!(days_your_way(&[], Some(day("2026-09-18")), today, 30), 0);
        assert_eq!(days_your_way(&[], Some(day("2026-01-01")), today, 0), 0);
        assert_eq!(streak(&[], None, today), 0);
        assert_eq!(streak(&[], Some(day("2026-09-18")), today), 0);
    }

    #[test]
    fn the_streak_ends_at_the_last_lapse_or_at_the_enabling_day() {
        let today = day("2026-09-17");
        let counts = vec![count("2026-09-10", 23, DayKind::Lapse, 1)];
        assert_eq!(streak(&counts, Some(day("2026-01-01")), today), 7);
        assert_eq!(streak(&counts, Some(day("2026-09-14")), today), 4);
        assert_eq!(streak(&[], Some(day("2026-06-10")), today), 100);
        let lapse_today = vec![count("2026-09-17", 8, DayKind::Lapse, 1)];
        assert_eq!(streak(&lapse_today, Some(day("2026-01-01")), today), 0);
        let lapse_yesterday = vec![count("2026-09-16", 8, DayKind::Lapse, 1)];
        assert_eq!(streak(&lapse_yesterday, Some(day("2026-01-01")), today), 1);
    }

    #[test]
    fn lapse_in_window_looks_at_the_window_only() {
        let today = day("2026-09-17");
        let counts = vec![
            count("2026-08-18", 10, DayKind::Lapse, 1),
            count("2026-09-17", 10, DayKind::CheckinHard, 1),
        ];
        assert!(
            !lapse_in_window(&counts, today, 30),
            "the 18th of August is day 31"
        );
        assert!(lapse_in_window(&counts, today, 31));
        assert!(!lapse_in_window(&counts, today, 0));
        let future = vec![count("2026-09-18", 10, DayKind::Lapse, 1)];
        assert!(!lapse_in_window(&future, today, 30));
        let today_lapse = vec![count("2026-09-17", 0, DayKind::Lapse, 1)];
        assert!(lapse_in_window(&today_lapse, today, 1));
    }

    #[test]
    fn total_on_sums_the_hours_of_one_day_and_kind() {
        let counts = vec![
            count("2026-09-17", 9, DayKind::VisionCheck, 5),
            count("2026-09-17", 10, DayKind::VisionCheck, 7),
            count("2026-09-17", 10, DayKind::VisionHit, 1),
            count("2026-09-16", 10, DayKind::VisionCheck, 100),
            count("2026-09-17", 11, DayKind::VisionCheck, u32::MAX),
        ];
        assert_eq!(
            total_on(&counts[..4], "2026-09-17", DayKind::VisionCheck),
            12
        );
        assert_eq!(total_on(&counts[..4], "2026-09-17", DayKind::VisionHit), 1);
        assert_eq!(
            total_on(&counts[..4], "2026-09-18", DayKind::VisionCheck),
            0
        );
        assert_eq!(
            total_on(&counts, "2026-09-17", DayKind::VisionCheck),
            u32::MAX
        );
        assert_eq!(day_key(day("2026-09-17")), "2026-09-17");
    }
}
