//! Daily report generation and scheduling.

use std::sync::Arc;

use chrono::{Duration, Local, NaiveDate};
use ubiqx_core::ports::*;
use ubiqx_core::scheduler::{day_range, due_reports};
use ubiqx_core::segmenter::merge_short_blocks;
use ubiqx_core::*;

use crate::state::EngineState;

const KV_LAST_REPORT_CHECK: &str = "last_report_check";

/// Blocks of `date` that belong to `category_id`, merged and ordered.
pub fn blocks_for(
    state: &EngineState,
    date: NaiveDate,
    category_id: &str,
) -> CoreResult<Vec<ActivityBlock>> {
    let blocks = state.deps.repos.blocks.list_in_range(day_range(date))?;
    let mine: Vec<ActivityBlock> = blocks
        .into_iter()
        .filter(|b| b.category_id.as_deref() == Some(category_id))
        .collect();
    Ok(merge_short_blocks(mine, 60))
}

/// Deterministic report used when no AI writer is available (local mode, no key, budget).
pub fn template_report(
    date: NaiveDate,
    category: &Category,
    blocks: &[ActivityBlock],
    now: chrono::DateTime<chrono::Utc>,
) -> DailyReport {
    use std::collections::BTreeMap;
    let mut groups: BTreeMap<String, (i64, Vec<String>, String)> = BTreeMap::new();
    for b in blocks {
        let key = match &b.domain {
            Some(d) => format!("{} · {}", b.app_name, d),
            None => format!("{} · {}", b.app_name, b.title_key),
        };
        let e = groups.entry(key.clone()).or_insert((
            0,
            vec![],
            b.description.clone().unwrap_or_default(),
        ));
        e.0 += b.duration_secs();
        if !e.1.contains(&b.app_name) {
            e.1.push(b.app_name.clone());
        }
        if e.2.is_empty() {
            if let Some(d) = &b.description {
                e.2 = d.clone();
            }
        }
    }
    let mut items: Vec<ReportItem> = groups
        .into_iter()
        .map(|(key, (secs, apps, desc))| ReportItem {
            activity: if desc.is_empty() {
                format!("Trabalhou em {key}")
            } else {
                desc
            },
            kind: ActivityKind::Outro,
            minutes: ((secs + 150) / 300 * 5).max(5) as u32,
            evidence: apps,
            time_range: String::new(),
            continuation_of: None,
        })
        .collect();
    items.sort_by_key(|a| std::cmp::Reverse(a.minutes));
    let total_secs: i64 = blocks.iter().map(|b| b.duration_secs()).sum();
    let mut report = DailyReport {
        id: new_id(),
        date,
        category_id: category.id.clone(),
        generated_at: now,
        summary_md: String::new(),
        items,
        highlights: vec![],
        total_secs,
        model: "template".into(),
        input_tokens: 0,
        output_tokens: 0,
        stale: false,
        edited: false,
    };
    report.summary_md = report::render_summary_md(&report, category);
    report
}

/// Generates (or regenerates) the report for one day and category.
pub async fn generate(
    state: &Arc<EngineState>,
    date: NaiveDate,
    category_id: &str,
    force: bool,
) -> CoreResult<DailyReport> {
    let repos = state.deps.repos.clone();
    let category = repos
        .categories
        .get(category_id)?
        .ok_or_else(|| CoreError::NotFound(format!("category {category_id}")))?;
    if let Some(existing) = repos.reports.get(date, category_id)? {
        if existing.edited && !force {
            return Ok(existing);
        }
        if !existing.stale && !force {
            return Ok(existing);
        }
    }
    let blocks = blocks_for(state, date, category_id)?;
    let settings = state.settings();
    let now = state.now();

    let previous_items: Vec<ReportItem> = repos
        .reports
        .list_between(date - Duration::days(5), date - Duration::days(1))?
        .into_iter()
        .filter(|r| r.category_id == category_id)
        .flat_map(|r| r.items)
        .collect();

    let mut report = None;
    if let Some(writer) = state.deps.ai.report_writer.clone() {
        if !blocks.is_empty() && state.remote_allowed() && crate::classify::budget_allows(state)? {
            let req = ReportRequest {
                date,
                category: category.clone(),
                blocks: blocks.clone(),
                previous_items,
                language: settings.language.clone(),
                utc_offset_secs: Local::now().offset().local_minus_utc(),
                user_profile: settings.user_profile.clone(),
                model: settings.models.report.clone(),
            };
            match writer.write_daily(&req).await {
                Ok(mut r) => {
                    state.set_ai_health(AiHealth::Ok);
                    if r.summary_md.trim().is_empty() {
                        r.summary_md = report::render_summary_md(&r, &category);
                    }
                    report = Some(r);
                }
                Err(e) => {
                    tracing::warn!(error = %e, "report writer failed; using template");
                    if matches!(e, CoreError::AiNotConfigured) {
                        state.set_ai_health(AiHealth::NotConfigured);
                    }
                }
            }
        }
    }
    let mut report = report.unwrap_or_else(|| template_report(date, &category, &blocks, now));
    if let Some(existing) = repos.reports.get(date, category_id)? {
        report.id = existing.id;
    }
    report.stale = false;
    repos.reports.upsert(&report)?;
    state.deps.sink.emit(EngineEvent::ReportReady {
        report: report.clone(),
    });
    Ok(report)
}

/// Runs the due reports since the last persisted check. Called every 30 s and at start-up.
pub async fn run_due(state: &Arc<EngineState>) -> CoreResult<usize> {
    let repos = state.deps.repos.clone();
    let now = state.now();
    let last = repos
        .kv
        .get(KV_LAST_REPORT_CHECK)?
        .and_then(|s| s.parse::<chrono::DateTime<chrono::Utc>>().ok())
        .unwrap_or(now - Duration::minutes(1));
    let categories = repos.categories.list(false)?;
    let settings = state.settings();
    let due = due_reports(&categories, &settings, last, now);
    let mut generated = 0;
    for d in due {
        match generate(state, d.date, &d.category_id, true).await {
            Ok(r) => {
                generated += 1;
                let name = categories
                    .iter()
                    .find(|c| c.id == d.category_id)
                    .map(|c| c.name.clone())
                    .unwrap_or_default();
                let late = now - d.scheduled_at > Duration::minutes(10);
                let title = if late {
                    format!("Relatório atrasado de {name} pronto")
                } else {
                    format!("Relatório de {name} pronto")
                };
                let body = if r.items.is_empty() {
                    "Sem atividade registrada hoje nesta categoria.".to_string()
                } else {
                    format!("{} itens · revise em 2 min no ubiqX.", r.items.len())
                };
                crate::nudges::emit(state, NudgeKind::ReportReady, &title, &body, true);
            }
            Err(e) => {
                tracing::warn!(error = %e, category = %d.category_id, "scheduled report failed")
            }
        }
    }
    repos.kv.set(KV_LAST_REPORT_CHECK, &now.to_rfc3339())?;
    Ok(generated)
}

/// Marks the reports of a block's day as stale (after corrections/splits/manual entries).
pub fn mark_day_stale(state: &EngineState, at: chrono::DateTime<chrono::Utc>) {
    let date = at.with_timezone(&Local).date_naive();
    if let Err(e) = state.deps.repos.reports.mark_stale(date) {
        tracing::debug!(error = %e, "could not mark reports stale");
    }
}
