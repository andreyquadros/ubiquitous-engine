//! Local (no API) productivity analytics: focus score, mood and nudges.
//!
//! The engine calls [`compute_stats`] for dashboards and [`NudgePolicy::evaluate`] once a
//! minute. Everything here is deterministic and cheap.

use std::collections::HashMap;

use chrono::{DateTime, Duration, Utc};

use crate::model::*;

/// Classifies categories into productive / distraction / neutral for scoring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Productive,
    Distraction,
    Uncategorized,
}

fn kind_of(block: &ActivityBlock, categories: &HashMap<&str, &Category>) -> Kind {
    match block.category_id.as_deref() {
        None => Kind::Uncategorized,
        Some(system_categories::UNCATEGORIZED) => Kind::Uncategorized,
        Some(system_categories::DISTRACTION) => Kind::Distraction,
        Some(id) => match categories.get(id) {
            Some(c) if c.is_productive => Kind::Productive,
            Some(_) => Kind::Distraction,
            None => Kind::Uncategorized,
        },
    }
}

/// Computes aggregate statistics for `blocks` (closed, sorted by start) inside `range`.
/// `idle_secs` is the time the sampler reported the user as away inside the same range.
pub fn compute_stats(
    blocks: &[ActivityBlock],
    categories: &[Category],
    range: TimeRange,
    idle_secs: i64,
) -> FocusStats {
    let cats: HashMap<&str, &Category> = categories.iter().map(|c| (c.id.as_str(), c)).collect();

    let mut productive = 0i64;
    let mut distraction = 0i64;
    let mut uncategorized = 0i64;
    let mut switches = 0u32;
    let mut longest = 0i64;
    let mut current_focus = 0i64;
    let mut last_end: Option<DateTime<Utc>> = None;
    let mut last_ctx: Option<(String, String)> = None;

    for b in blocks.iter().filter(|b| !b.is_open) {
        let start = b.started_at.max(range.from);
        let end = b.ended_at.min(range.to);
        if end <= start {
            continue;
        }
        let secs = (end - start).num_seconds();
        let kind = kind_of(b, &cats);
        match kind {
            Kind::Productive => productive += secs,
            Kind::Distraction => distraction += secs,
            Kind::Uncategorized => uncategorized += secs,
        }

        // Context switch: different app or domain than the previous block.
        let ctx = (b.app_id.clone(), b.domain.clone().unwrap_or_default());
        if let Some(prev) = &last_ctx {
            if *prev != ctx {
                switches += 1;
            }
        }
        last_ctx = Some(ctx);

        // Longest focus: consecutive productive blocks with gaps < 2 min.
        let contiguous = last_end
            .map(|e| start - e <= Duration::minutes(2))
            .unwrap_or(true);
        if kind == Kind::Productive && contiguous {
            current_focus += secs;
        } else if kind == Kind::Productive {
            current_focus = secs;
        } else {
            current_focus = 0;
        }
        longest = longest.max(current_focus);
        last_end = Some(end);
    }

    let active = productive + distraction + uncategorized;
    let total = active + idle_secs;
    let hours = active as f32 / 3600.0;
    let switches_per_hour = if hours > 0.05 { switches as f32 / hours } else { 0.0 };
    let focus_score = focus_score(productive, distraction, uncategorized, switches_per_hour);

    FocusStats {
        focus_score,
        productive_secs: productive,
        distraction_secs: distraction,
        uncategorized_secs: uncategorized,
        idle_secs,
        total_secs: total,
        switches_per_hour,
        longest_focus_secs: longest,
        mood: mood_for(focus_score, active),
    }
}

/// 0–100. Share of productive time, penalised by heavy context switching. Uncategorised time
/// counts half (we do not know yet), so a freshly installed app is not punished.
pub fn focus_score(productive: i64, distraction: i64, uncategorized: i64, switches_per_hour: f32) -> u8 {
    let active = (productive + distraction + uncategorized) as f32;
    if active < 60.0 {
        return 0;
    }
    let base = (productive as f32 + 0.5 * uncategorized as f32) / active * 100.0;
    // Up to 12 switches/hour is normal; beyond that each extra switch costs 1 point (max 25).
    let penalty = (switches_per_hour - 12.0).clamp(0.0, 25.0);
    (base - penalty).clamp(0.0, 100.0).round() as u8
}

pub fn mood_for(score: u8, active_secs: i64) -> Mood {
    if active_secs < 60 {
        return Mood::Sleeping;
    }
    match score {
        85..=100 => Mood::Excited,
        65..=84 => Mood::Focused,
        45..=64 => Mood::Calm,
        _ => Mood::Worried,
    }
}

/// Decides which nudges UBI should emit right now.
#[derive(Debug, Clone)]
pub struct NudgePolicy {
    /// Minutes of continuous non-productive time before an "unproductive" nudge.
    pub unproductive_after_mins: i64,
    /// Switches per hour (over the last 30 min) considered "distracted".
    pub distracted_switches_per_hour: f32,
    /// Minutes of continuous productive work before suggesting a break.
    pub break_after_mins: i64,
    /// Minimum minutes between two nudges of the same kind.
    pub cooldown_mins: i64,
}

impl Default for NudgePolicy {
    fn default() -> Self {
        Self {
            unproductive_after_mins: 25,
            distracted_switches_per_hour: 40.0,
            break_after_mins: 90,
            cooldown_mins: 45,
        }
    }
}

/// Inputs the engine gathers before asking the policy.
#[derive(Debug, Clone)]
pub struct NudgeInput<'a> {
    pub now: DateTime<Utc>,
    /// Closed blocks from the last ~2 hours, plus the open block if any (sorted by start).
    pub recent: &'a [ActivityBlock],
    pub categories: &'a [Category],
    /// Last time each nudge kind was emitted.
    pub last_emitted: &'a HashMap<NudgeKind, DateTime<Utc>>,
    pub quiet: bool,
}

impl NudgePolicy {
    pub fn evaluate(&self, input: &NudgeInput<'_>) -> Vec<Nudge> {
        if input.quiet {
            return vec![];
        }
        let cats: HashMap<&str, &Category> =
            input.categories.iter().map(|c| (c.id.as_str(), c)).collect();
        let mut out = Vec::new();

        // Walk backwards from now: continuous run of the same kind.
        let mut run_kind: Option<Kind> = None;
        let mut run_secs = 0i64;
        let mut cursor = input.now;
        for b in input.recent.iter().rev() {
            if b.ended_at < cursor - Duration::minutes(2) {
                break; // gap → streak ends
            }
            let kind = kind_of(b, &cats);
            match run_kind {
                None => run_kind = Some(kind),
                Some(k) if k != kind => break,
                _ => {}
            }
            run_secs += b.duration_secs();
            cursor = b.started_at;
        }

        let cooled = |kind: NudgeKind| {
            input
                .last_emitted
                .get(&kind)
                .map(|t| input.now - *t >= Duration::minutes(self.cooldown_mins))
                .unwrap_or(true)
        };

        if run_kind == Some(Kind::Distraction)
            && run_secs >= self.unproductive_after_mins * 60
            && cooled(NudgeKind::Unproductive)
        {
            out.push(Nudge {
                id: new_id(),
                at: input.now,
                kind: NudgeKind::Unproductive,
                title: "Hora de voltar ao foco?".into(),
                message: format!(
                    "Já são {} min em atividades que você marcou como distração. Que tal retomar uma tarefa importante?",
                    run_secs / 60
                ),
                seen: false,
            });
        }

        if run_kind == Some(Kind::Productive)
            && run_secs >= self.break_after_mins * 60
            && cooled(NudgeKind::BreakSuggested)
        {
            out.push(Nudge {
                id: new_id(),
                at: input.now,
                kind: NudgeKind::BreakSuggested,
                title: "Ótimo ritmo! Que tal uma pausa?".into(),
                message: format!(
                    "Você está focado há {} min seguidos. Uma pausa de 5 min ajuda a manter a energia.",
                    run_secs / 60
                ),
                seen: false,
            });
        }

        // Distraction by context switching in the last 30 minutes.
        let window_start = input.now - Duration::minutes(30);
        let mut switches = 0u32;
        let mut prev: Option<(&str, &str)> = None;
        let mut active_secs = 0i64;
        for b in input.recent.iter().filter(|b| b.ended_at > window_start) {
            let ctx = (b.app_id.as_str(), b.domain.as_deref().unwrap_or(""));
            if let Some(p) = prev {
                if p != ctx {
                    switches += 1;
                }
            }
            prev = Some(ctx);
            active_secs += (b.ended_at.min(input.now) - b.started_at.max(window_start))
                .num_seconds()
                .max(0);
        }
        if active_secs >= 15 * 60 {
            let per_hour = switches as f32 / (active_secs as f32 / 3600.0);
            if per_hour >= self.distracted_switches_per_hour && cooled(NudgeKind::Distracted) {
                out.push(Nudge {
                    id: new_id(),
                    at: input.now,
                    kind: NudgeKind::Distracted,
                    title: "Muitas trocas de contexto".into(),
                    message: format!(
                        "Foram {switches} trocas de janela nos últimos 30 min. Tente fechar o que não é urgente e ficar em uma tarefa por vez."
                    ),
                    seen: false,
                });
            }
        }

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn t(mins: i64) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 9, 17, 9, 0, 0).unwrap() + Duration::minutes(mins)
    }

    fn cat(id: &str, productive: bool) -> Category {
        Category {
            id: id.into(),
            name: id.into(),
            color: "#000".into(),
            icon: "x".into(),
            description: String::new(),
            keywords: vec![],
            report_time: None,
            is_productive: productive,
            is_system: false,
            archived: false,
            sort_order: 0,
            created_at: t(0),
        }
    }

    fn block(s: i64, e: i64, app: &str, cat: Option<&str>) -> ActivityBlock {
        ActivityBlock {
            id: new_id(),
            started_at: t(s),
            ended_at: t(e),
            app_name: app.into(),
            app_id: app.into(),
            title: String::new(),
            title_key: String::new(),
            url: None,
            domain: None,
            category_id: cat.map(String::from),
            confidence: 1.0,
            source: Some(ClassificationSource::Rule),
            description: None,
            screenshot_id: None,
            sample_count: 1,
            is_open: false,
        }
    }

    #[test]
    fn stats_and_score() {
        let cats = vec![cat("work", true), cat("fun", false)];
        let blocks = vec![
            block(0, 50, "a", Some("work")),
            block(50, 60, "b", Some("fun")),
            block(60, 90, "a", Some("work")),
            block(90, 100, "c", None),
        ];
        let s = compute_stats(&blocks, &cats, TimeRange::new(t(0), t(120)), 600);
        assert_eq!(s.productive_secs, 80 * 60);
        assert_eq!(s.distraction_secs, 10 * 60);
        assert_eq!(s.uncategorized_secs, 10 * 60);
        assert_eq!(s.idle_secs, 600);
        assert_eq!(s.longest_focus_secs, 50 * 60);
        assert!(s.focus_score >= 80, "{}", s.focus_score);
        assert!(matches!(s.mood, Mood::Excited | Mood::Focused));
        let empty = compute_stats(&[], &cats, TimeRange::new(t(0), t(1)), 0);
        assert_eq!(empty.focus_score, 0);
        assert_eq!(empty.mood, Mood::Sleeping);
    }

    #[test]
    fn range_clipping() {
        let cats = vec![cat("work", true)];
        let blocks = vec![block(-30, 30, "a", Some("work"))];
        let s = compute_stats(&blocks, &cats, TimeRange::new(t(0), t(60)), 0);
        assert_eq!(s.productive_secs, 30 * 60);
    }

    #[test]
    fn unproductive_nudge_and_cooldown() {
        let cats = vec![cat("work", true), cat("fun", false)];
        let recent = vec![block(0, 30, "yt", Some("fun"))];
        let mut last = HashMap::new();
        let policy = NudgePolicy::default();
        let n = policy.evaluate(&NudgeInput { now: t(30), recent: &recent, categories: &cats, last_emitted: &last, quiet: false });
        assert_eq!(n.len(), 1);
        assert_eq!(n[0].kind, NudgeKind::Unproductive);
        last.insert(NudgeKind::Unproductive, t(30));
        let n = policy.evaluate(&NudgeInput { now: t(40), recent: &recent, categories: &cats, last_emitted: &last, quiet: false });
        assert!(n.is_empty(), "cooldown should suppress");
        let n = policy.evaluate(&NudgeInput { now: t(30), recent: &recent, categories: &cats, last_emitted: &HashMap::new(), quiet: true });
        assert!(n.is_empty(), "quiet hours suppress");
    }

    #[test]
    fn break_nudge_after_long_focus() {
        let cats = vec![cat("work", true)];
        let recent = vec![block(0, 50, "a", Some("work")), block(50, 95, "a", Some("work"))];
        let n = NudgePolicy::default().evaluate(&NudgeInput { now: t(95), recent: &recent, categories: &cats, last_emitted: &HashMap::new(), quiet: false });
        assert!(n.iter().any(|n| n.kind == NudgeKind::BreakSuggested));
    }

    #[test]
    fn distracted_nudge_on_many_switches() {
        let cats = vec![cat("work", true)];
        let mut recent = Vec::new();
        for i in 0..30 {
            recent.push(block(i, i + 1, if i % 2 == 0 { "a" } else { "b" }, Some("work")));
        }
        let n = NudgePolicy::default().evaluate(&NudgeInput { now: t(30), recent: &recent, categories: &cats, last_emitted: &HashMap::new(), quiet: false });
        assert!(n.iter().any(|n| n.kind == NudgeKind::Distracted));
    }
}
