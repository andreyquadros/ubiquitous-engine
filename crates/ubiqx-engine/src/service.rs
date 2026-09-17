//! Read models for the UI. Every function here is synchronous (repository calls) and is
//! meant to be run through `spawn_blocking` by the shell.

use std::collections::HashMap;

use chrono::{Datelike, Duration, Local, NaiveDate, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use ubiqx_core::insights::compute_stats;
use ubiqx_core::ports::*;
use ubiqx_core::scheduler::day_range;
use ubiqx_core::*;

use crate::state::EngineState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardData {
    pub date: NaiveDate,
    pub stats: FocusStats,
    pub totals: Vec<CategoryTotal>,
    pub top_apps: Vec<AppTotal>,
    pub timeline: Vec<ActivityBlock>,
    pub open_block: Option<ActivityBlock>,
    pub categories: Vec<Category>,
    pub unseen_nudges: Vec<Nudge>,
    pub tracker_state: TrackerState,
    pub ai_health: AiHealth,
    pub usage_month: AiUsageTotals,
    pub budget_usd: f64,
    pub needs_review: usize,
    /// Focus score per local hour of the day (0..24), `None` when no activity.
    pub hourly_focus: Vec<Option<u8>>,
}

/// A group of blocks sharing app + domain/title, the unit of the review screen.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockGroup {
    pub key: String,
    pub app_id: String,
    pub app_name: String,
    pub domain: Option<String>,
    pub title: String,
    pub total_secs: i64,
    pub block_ids: Vec<Id>,
    /// Majority category of the group, if any.
    pub category_id: Option<Id>,
    pub min_confidence: f32,
    pub source: Option<ClassificationSource>,
    pub needs_review: bool,
    pub description: Option<String>,
    pub first_started_at: chrono::DateTime<Utc>,
}

pub fn local_day_range(date: NaiveDate) -> TimeRange {
    day_range(date)
}

pub fn today() -> NaiveDate {
    Local::now().date_naive()
}

pub fn dashboard(state: &EngineState, date: NaiveDate) -> CoreResult<DashboardData> {
    let repos = &state.deps.repos;
    let range = day_range(date);
    let now = state.now();
    let effective = TimeRange::new(range.from, range.to.min(now.max(range.from)));
    let mut timeline = repos.blocks.list_in_range(range)?;
    let open_block = repos
        .blocks
        .open_block()?
        .filter(|b| range.contains(b.started_at));
    if let Some(o) = &open_block {
        let mut o = o.clone();
        o.ended_at = now;
        timeline.push(o);
    }
    timeline.sort_by_key(|b| b.started_at);
    let categories = repos.categories.list(false)?;
    let idle_secs = idle_seconds(&timeline, effective);
    let closed_for_stats: Vec<ActivityBlock> = timeline
        .iter()
        .cloned()
        .map(|mut b| {
            b.is_open = false;
            b
        })
        .collect();
    let stats = compute_stats(&closed_for_stats, &categories, effective, idle_secs);
    let mut totals = repos.blocks.totals_by_category(range)?;
    if let Some(o) = &open_block {
        let secs = (now - o.started_at).num_seconds().max(0);
        match totals.iter_mut().find(|t| t.category_id == o.category_id) {
            Some(t) => {
                t.secs += secs;
                t.block_count += 1;
            }
            None => totals.push(CategoryTotal {
                category_id: o.category_id.clone(),
                secs,
                block_count: 1,
            }),
        }
    }
    totals.sort_by(|a, b| b.secs.cmp(&a.secs));
    let top_apps = repos.blocks.totals_by_app(range, 8)?;
    let unseen_nudges = repos
        .nudges
        .list_recent(20)?
        .into_iter()
        .filter(|n| !n.seen)
        .collect();
    let usage_month = repos.usage.totals(month_range(now))?;
    let settings = state.settings();
    let needs_review = repos.blocks.list_needs_review(500)?.len();
    let hourly_focus = hourly_focus(&closed_for_stats, &categories, date);
    Ok(DashboardData {
        date,
        stats,
        totals,
        top_apps,
        timeline,
        open_block,
        categories,
        unseen_nudges,
        tracker_state: *state.tracker.read(),
        ai_health: state.ai_health(),
        usage_month,
        budget_usd: settings.ai_monthly_budget_usd,
        needs_review,
        hourly_focus,
    })
}

fn month_range(now: chrono::DateTime<Utc>) -> TimeRange {
    let start = Utc
        .with_ymd_and_hms(now.year(), now.month(), 1, 0, 0, 0)
        .single()
        .unwrap_or(now);
    TimeRange::new(start, now + Duration::seconds(1))
}

/// Gaps between blocks inside the range count as idle (the sampler drops idle samples).
fn idle_seconds(blocks: &[ActivityBlock], range: TimeRange) -> i64 {
    let mut cursor = range.from;
    let mut idle = 0i64;
    let mut sorted: Vec<&ActivityBlock> = blocks.iter().collect();
    sorted.sort_by_key(|b| b.started_at);
    let mut first = true;
    for b in sorted {
        let s = b.started_at.max(range.from);
        if first {
            cursor = s;
            first = false;
        }
        if s > cursor {
            idle += (s - cursor).num_seconds();
        }
        cursor = cursor.max(b.ended_at.min(range.to));
    }
    idle.max(0)
}

fn hourly_focus(
    blocks: &[ActivityBlock],
    categories: &[Category],
    date: NaiveDate,
) -> Vec<Option<u8>> {
    let day = day_range(date);
    (0..24)
        .map(|h| {
            let from = day.from + Duration::hours(h);
            let to = from + Duration::hours(1);
            let r = TimeRange::new(from, to);
            let any = blocks
                .iter()
                .any(|b| b.started_at < to && b.ended_at > from);
            if !any {
                return None;
            }
            let s = compute_stats(blocks, categories, r, 0);
            Some(s.focus_score)
        })
        .collect()
}

pub fn timeline(state: &EngineState, date: NaiveDate) -> CoreResult<Vec<ActivityBlock>> {
    let range = day_range(date);
    let mut blocks = state.deps.repos.blocks.list_in_range(range)?;
    if let Some(o) = state.deps.repos.blocks.open_block()? {
        if range.contains(o.started_at) {
            blocks.push(o);
        }
    }
    blocks.sort_by_key(|b| b.started_at);
    Ok(blocks)
}

/// Groups the day's blocks for the review screen, worst-classified first.
pub fn review_groups(state: &EngineState, date: NaiveDate) -> CoreResult<Vec<BlockGroup>> {
    let blocks = timeline(state, date)?;
    let mut groups: HashMap<String, BlockGroup> = HashMap::new();
    for b in blocks.into_iter().filter(|b| !b.is_open) {
        let key = match &b.domain {
            Some(d) => format!("{}|{}", b.app_id.to_lowercase(), d),
            None => format!("{}|{}", b.app_id.to_lowercase(), b.title_key),
        };
        let g = groups.entry(key.clone()).or_insert_with(|| BlockGroup {
            key: key.clone(),
            app_id: b.app_id.clone(),
            app_name: b.app_name.clone(),
            domain: b.domain.clone(),
            title: b.title.clone(),
            total_secs: 0,
            block_ids: vec![],
            category_id: None,
            min_confidence: 1.0,
            source: None,
            needs_review: false,
            description: None,
            first_started_at: b.started_at,
        });
        g.total_secs += b.duration_secs();
        g.block_ids.push(b.id.clone());
        g.min_confidence = g.min_confidence.min(if b.category_id.is_some() {
            b.confidence
        } else {
            0.0
        });
        g.needs_review |= b.needs_review;
        g.first_started_at = g.first_started_at.min(b.started_at);
        if g.description.is_none() {
            g.description = b.description.clone();
        }
        if b.source == Some(ClassificationSource::User)
            || g.source != Some(ClassificationSource::User)
        {
            g.category_id = b.category_id.clone().or(g.category_id.clone());
            g.source = b.source.or(g.source);
        }
    }
    let mut out: Vec<BlockGroup> = groups.into_values().collect();
    out.sort_by(|a, b| {
        let wa = a.total_secs as f32 * (1.0 - a.min_confidence);
        let wb = b.total_secs as f32 * (1.0 - b.min_confidence);
        wb.partial_cmp(&wa)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(b.total_secs.cmp(&a.total_secs))
    });
    Ok(out)
}

/// Reclassifies every block of a group (same app + domain/title) for the day.
pub fn group_block_ids(state: &EngineState, date: NaiveDate, key: &str) -> CoreResult<Vec<Id>> {
    Ok(review_groups(state, date)?
        .into_iter()
        .find(|g| g.key == key)
        .map(|g| g.block_ids)
        .unwrap_or_default())
}

pub fn monthly_report_md(
    state: &EngineState,
    category_id: &str,
    year: i32,
    month: u32,
) -> CoreResult<String> {
    let category = state
        .deps
        .repos
        .categories
        .get(category_id)?
        .ok_or_else(|| CoreError::NotFound(format!("category {category_id}")))?;
    let from = NaiveDate::from_ymd_opt(year, month, 1)
        .ok_or_else(|| CoreError::Invalid("invalid month".into()))?;
    let to = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    }
    .and_then(|d| d.pred_opt())
    .ok_or_else(|| CoreError::Invalid("invalid month".into()))?;
    let reports: Vec<DailyReport> = state
        .deps
        .repos
        .reports
        .list_between(from, to)?
        .into_iter()
        .filter(|r| r.category_id == category_id)
        .collect();
    Ok(report::render_monthly_md(&reports, &category, year, month))
}

/// Blocks with the exact text that was sent to the AI (transparency view).
pub fn ai_sent_blocks(state: &EngineState, date: NaiveDate) -> CoreResult<Vec<ActivityBlock>> {
    Ok(timeline(state, date)?
        .into_iter()
        .filter(|b| b.ai_sent_at.is_some())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blk(s: i64, e: i64) -> ActivityBlock {
        let base = Utc.with_ymd_and_hms(2026, 9, 17, 12, 0, 0).unwrap();
        let mut b = ActivityBlock::new_manual(
            base + Duration::minutes(s),
            base + Duration::minutes(e),
            "c".into(),
            None,
        );
        b.is_manual = false;
        b
    }

    #[test]
    fn idle_counts_gaps_between_blocks_only() {
        let base = Utc.with_ymd_and_hms(2026, 9, 17, 12, 0, 0).unwrap();
        let blocks = vec![blk(0, 10), blk(20, 30), blk(30, 40)];
        let idle = idle_seconds(
            &blocks,
            TimeRange::new(base - Duration::hours(1), base + Duration::hours(2)),
        );
        assert_eq!(idle, 10 * 60);
    }
}
