//! Conversions between domain value types and their SQLite representations.

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::error::{StorageError, StorageResult};

const DATE_FMT: &str = "%Y-%m-%d";
const TIME_FMT: &str = "%H:%M";

/// Unix milliseconds: the storage form of every timestamp.
pub(crate) fn ms(t: DateTime<Utc>) -> i64 {
    t.timestamp_millis()
}

pub(crate) fn opt_ms(t: Option<DateTime<Utc>>) -> Option<i64> {
    t.map(ms)
}

/// Inverse of [`ms`].
pub(crate) fn dt(millis: i64) -> StorageResult<DateTime<Utc>> {
    DateTime::<Utc>::from_timestamp_millis(millis)
        .ok_or_else(|| StorageError::Decode(format!("timestamp out of range: {millis}")))
}

pub(crate) fn opt_dt(millis: Option<i64>) -> StorageResult<Option<DateTime<Utc>>> {
    millis.map(dt).transpose()
}

/// `YYYY-MM-DD`.
pub(crate) fn date_str(d: NaiveDate) -> String {
    d.format(DATE_FMT).to_string()
}

pub(crate) fn date(s: &str) -> StorageResult<NaiveDate> {
    NaiveDate::parse_from_str(s, DATE_FMT)
        .map_err(|e| StorageError::Decode(format!("bad date `{s}`: {e}")))
}

/// `HH:MM` (seconds are dropped; report times have minute granularity).
pub(crate) fn time_str(t: NaiveTime) -> String {
    t.format(TIME_FMT).to_string()
}

pub(crate) fn time(s: &str) -> StorageResult<NaiveTime> {
    NaiveTime::parse_from_str(s, TIME_FMT)
        .or_else(|_| NaiveTime::parse_from_str(s, "%H:%M:%S"))
        .map_err(|e| StorageError::Decode(format!("bad time `{s}`: {e}")))
}

pub(crate) fn opt_time(s: Option<String>) -> StorageResult<Option<NaiveTime>> {
    s.as_deref().map(time).transpose()
}

/// Decodes an enum stored through its `as_str()` form using the matching `parse()`.
pub(crate) fn parse_enum<T>(
    what: &str,
    raw: &str,
    parse: fn(&str) -> Option<T>,
) -> StorageResult<T> {
    parse(raw).ok_or_else(|| StorageError::Decode(format!("unknown {what} `{raw}`")))
}

pub(crate) fn opt_enum<T>(
    what: &str,
    raw: Option<String>,
    parse: fn(&str) -> Option<T>,
) -> StorageResult<Option<T>> {
    raw.as_deref()
        .map(|s| parse_enum(what, s, parse))
        .transpose()
}

pub(crate) fn json<T: Serialize>(value: &T) -> StorageResult<String> {
    Ok(serde_json::to_string(value)?)
}

pub(crate) fn from_json<T: DeserializeOwned>(raw: &str) -> StorageResult<T> {
    Ok(serde_json::from_str(raw)?)
}

/// `usize` limits become SQL integers; absurdly large limits saturate instead of failing.
pub(crate) fn sql_limit(n: usize) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

/// Reads a `COUNT(*)` / `SUM(...)` column, which SQLite yields as `i64`, into a `u64`.
/// Negative values cannot occur for counts and sums of unsigned columns; they map to `0`.
pub(crate) fn count(row: &rusqlite::Row<'_>, idx: impl rusqlite::RowIndex) -> StorageResult<u64> {
    let raw: i64 = row.get(idx)?;
    Ok(u64::try_from(raw).unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamps_round_trip_at_millisecond_precision() {
        let t = DateTime::<Utc>::from_timestamp_millis(1_758_130_200_123).expect("valid");
        assert_eq!(dt(ms(t)).expect("round trip"), t);
        assert!(dt(i64::MAX).is_err());
    }

    #[test]
    fn dates_and_times_round_trip() {
        let d = NaiveDate::from_ymd_opt(2026, 9, 17).expect("valid");
        assert_eq!(date_str(d), "2026-09-17");
        assert_eq!(date("2026-09-17").expect("parse"), d);
        assert!(date("17/09/2026").is_err());

        let t = NaiveTime::from_hms_opt(18, 30, 0).expect("valid");
        assert_eq!(time_str(t), "18:30");
        assert_eq!(time("18:30").expect("parse"), t);
        assert_eq!(time("18:30:00").expect("parse with seconds"), t);
        assert!(time("6pm").is_err());
    }

    #[test]
    fn enum_decoding_reports_unknown_tags() {
        use ubiqx_core::NudgeKind;
        assert_eq!(
            parse_enum("nudge kind", "praise", NudgeKind::parse).expect("known"),
            NudgeKind::Praise
        );
        assert!(parse_enum("nudge kind", "nope", NudgeKind::parse).is_err());
        assert_eq!(
            opt_enum("nudge kind", None, NudgeKind::parse).expect("none"),
            None
        );
    }
}
