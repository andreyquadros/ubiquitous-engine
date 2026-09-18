//! UBI's voice: evaluates the nudge policy and delivers nudges (event + OS notification).

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{Duration, Local};
use ubiqx_core::insights::{NudgeInput, NudgePolicy};
use ubiqx_core::*;

use crate::state::EngineState;

/// Stores, emits and (optionally) notifies a nudge. Attention/report nudges bypass the cap.
pub fn emit(
    state: &EngineState,
    kind: NudgeKind,
    title: &str,
    message: &str,
    os_notification: bool,
) {
    let nudge = Nudge {
        id: new_id(),
        at: state.now(),
        kind,
        title: title.into(),
        message: message.into(),
        seen: false,
    };
    if let Err(e) = state.deps.repos.nudges.insert(&nudge) {
        tracing::warn!(error = %e, "could not store nudge");
    }
    state.deps.sink.emit(EngineEvent::Nudge { nudge });
    if os_notification {
        if let Err(e) = state.deps.platform.notifier.notify(title, message) {
            tracing::debug!(error = %e, "notification failed");
        }
    }
}

pub fn emit_attention(state: &EngineState, title: &str, message: &str) {
    emit(state, NudgeKind::Attention, title, message, true);
}

/// One evaluation pass; called every minute by the engine.
pub fn run_once(state: &Arc<EngineState>) -> CoreResult<Vec<Nudge>> {
    let settings = state.settings();
    let now = state.now();
    if !settings.nudges.enabled {
        return Ok(vec![]);
    }
    if *state.tracker.read() != TrackerState::Running {
        return Ok(vec![]);
    }
    if settings
        .nudges
        .snoozed_until
        .map(|t| now < t)
        .unwrap_or(false)
    {
        return Ok(vec![]);
    }
    let quiet = settings
        .quiet_hours
        .contains(now.with_timezone(&Local).time());
    let repos = &state.deps.repos;

    // Daily cap.
    let day_start = ubiqx_core::scheduler::day_range(now.with_timezone(&Local).date_naive()).from;
    if repos.nudges.count_since(day_start)? >= settings.nudges.max_per_day as u64 {
        return Ok(vec![]);
    }

    let mut recent = repos
        .blocks
        .list_in_range(TimeRange::new(now - Duration::hours(2), now))?;
    let open = repos.blocks.open_block()?;
    if let Some(o) = &open {
        // Silence in meetings / presentations.
        if settings
            .nudges
            .silent_apps
            .iter()
            .any(|a| a.eq_ignore_ascii_case(&o.app_id) || a.eq_ignore_ascii_case(&o.app_name))
        {
            return Ok(vec![]);
        }
        let mut o = o.clone();
        o.ended_at = now;
        recent.push(o);
    }
    recent.sort_by_key(|b| b.started_at);
    let categories = repos.categories.list(false)?;
    let mut last_emitted: HashMap<NudgeKind, chrono::DateTime<chrono::Utc>> = HashMap::new();
    for kind in [
        NudgeKind::Unproductive,
        NudgeKind::Distracted,
        NudgeKind::BreakSuggested,
        NudgeKind::Praise,
        NudgeKind::Idle,
    ] {
        if let Some(t) = repos.nudges.last_of_kind(kind)? {
            last_emitted.insert(kind, t);
        }
    }
    let policy = NudgePolicy {
        cooldown_mins: settings.nudges.cooldown_mins.max(10) as i64,
        ..NudgePolicy::default()
    };
    let nudges = policy.evaluate(&NudgeInput {
        now,
        recent: &recent,
        categories: &categories,
        last_emitted: &last_emitted,
        quiet,
        language: settings.ui_language(),
    });
    let mut out = Vec::new();
    for n in nudges {
        let enabled = match n.kind {
            NudgeKind::Unproductive => settings.nudges.unproductive,
            NudgeKind::Distracted => settings.nudges.distracted,
            NudgeKind::BreakSuggested => settings.nudges.break_suggested,
            NudgeKind::Praise => settings.nudges.praise,
            NudgeKind::Idle => settings.nudges.idle,
            _ => true,
        };
        if !enabled {
            continue;
        }
        emit(
            state,
            n.kind,
            &n.title,
            &n.message,
            n.kind != NudgeKind::Praise,
        );
        out.push(n);
    }
    Ok(out)
}
