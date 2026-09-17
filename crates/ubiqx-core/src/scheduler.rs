//! Decides when daily reports are due. Pure function of (categories, settings, now, last run).

use chrono::{DateTime, Local, NaiveDate, NaiveTime, TimeZone, Utc};

use crate::model::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DueReport {
    pub category_id: Id,
    /// The local day the report covers (today, since reports run in the evening).
    pub date: NaiveDate,
    pub scheduled_at: DateTime<Utc>,
}

/// Returns the reports whose scheduled time has passed since `last_check` (exclusive) up to
/// `now` (inclusive). Categories that are archived or system-owned never get reports.
pub fn due_reports(
    categories: &[Category],
    settings: &Settings,
    last_check: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Vec<DueReport> {
    let mut out = Vec::new();
    for c in categories.iter().filter(|c| !c.archived && !c.is_system) {
        let time = c.report_time.unwrap_or(settings.report_default_time);
        // Check the local days touched by (last_check, now]; normally just today.
        let mut day = last_check.with_timezone(&Local).date_naive();
        let today = now.with_timezone(&Local).date_naive();
        while day <= today {
            if let Some(at) = local_datetime(day, time) {
                if at > last_check && at <= now {
                    out.push(DueReport { category_id: c.id.clone(), date: day, scheduled_at: at });
                }
            }
            day = day.succ_opt().unwrap_or(day);
            if day == today.succ_opt().unwrap_or(today) {
                break;
            }
        }
    }
    out.sort_by(|a, b| a.scheduled_at.cmp(&b.scheduled_at).then(a.category_id.cmp(&b.category_id)));
    out
}

fn local_datetime(day: NaiveDate, time: NaiveTime) -> Option<DateTime<Utc>> {
    let naive = day.and_time(time);
    match Local.from_local_datetime(&naive) {
        chrono::LocalResult::Single(dt) => Some(dt.with_timezone(&Utc)),
        chrono::LocalResult::Ambiguous(a, _) => Some(a.with_timezone(&Utc)),
        chrono::LocalResult::None => None,
    }
}

/// Local day boundaries `[start, end)` for a date, in UTC.
pub fn day_range(date: NaiveDate) -> TimeRange {
    let start = local_datetime(date, NaiveTime::MIN).unwrap_or_else(|| Utc.from_utc_datetime(&date.and_time(NaiveTime::MIN)));
    let next = date.succ_opt().unwrap_or(date);
    let end = local_datetime(next, NaiveTime::MIN).unwrap_or(start + chrono::Duration::days(1));
    TimeRange::new(start, end)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cat(id: &str, time: Option<(u32, u32)>) -> Category {
        Category {
            id: id.into(),
            name: id.into(),
            color: "#000".into(),
            icon: "x".into(),
            description: String::new(),
            keywords: vec![],
            report_time: time.map(|(h, m)| NaiveTime::from_hms_opt(h, m, 0).unwrap()),
            is_productive: true,
            is_system: false,
            archived: false,
            sort_order: 0,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn reports_become_due_once() {
        let cats = vec![cat("a", Some((18, 0))), cat("b", None), cat("sys", None)];
        let mut cats = cats;
        cats[2].is_system = true;
        let settings = Settings { report_default_time: NaiveTime::from_hms_opt(17, 30, 0).unwrap(), ..Default::default() };
        let today = Local::now().date_naive();
        let at = |h: u32, m: u32| local_datetime(today, NaiveTime::from_hms_opt(h, m, 0).unwrap()).unwrap();

        let due = due_reports(&cats, &settings, at(17, 0), at(17, 45));
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].category_id, "b");

        let due = due_reports(&cats, &settings, at(17, 45), at(18, 30));
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].category_id, "a");

        let due = due_reports(&cats, &settings, at(18, 30), at(19, 0));
        assert!(due.is_empty());

        // A long sleep spanning both times yields both, ordered by time.
        let due = due_reports(&cats, &settings, at(9, 0), at(23, 0));
        assert_eq!(due.iter().map(|d| d.category_id.as_str()).collect::<Vec<_>>(), vec!["b", "a"]);
    }

    #[test]
    fn day_range_is_24h() {
        let r = day_range(NaiveDate::from_ymd_opt(2026, 9, 17).unwrap());
        assert_eq!((r.to - r.from).num_hours(), 24);
    }
}
