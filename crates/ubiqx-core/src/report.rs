//! Markdown rendering of daily and monthly reports.
//!
//! Pure functions shared by the AI adapter (which fills `summary_md` right after generation),
//! the CLI and the UI, so every surface produces exactly the same text for the same items.

use std::collections::{BTreeMap, HashMap};

use chrono::{Datelike, NaiveDate};

use crate::model::{ActivityKind, Category, DailyReport, ReportItem};

/// Presentation order of activity kinds in reports.
pub const KIND_ORDER: [ActivityKind; 9] = [
    ActivityKind::Desenvolvimento,
    ActivityKind::Reuniao,
    ActivityKind::Comunicacao,
    ActivityKind::Documentacao,
    ActivityKind::Ensino,
    ActivityKind::Pesquisa,
    ActivityKind::Extensao,
    ActivityKind::Gestao,
    ActivityKind::Outro,
];

/// Formats a duration in minutes as `"45 min"`, `"2 h"` or `"1 h 20 min"`.
pub fn format_minutes(minutes: u32) -> String {
    let h = minutes / 60;
    let m = minutes % 60;
    match (h, m) {
        (0, m) => format!("{m} min"),
        (h, 0) => format!("{h} h"),
        (h, m) => format!("{h} h {m:02} min"),
    }
}

/// `dd/mm/yyyy`.
pub fn format_date_br(date: NaiveDate) -> String {
    date.format("%d/%m/%Y").to_string()
}

fn secs_to_minutes(secs: i64) -> u32 {
    u32::try_from(secs.max(0) / 60).unwrap_or(u32::MAX)
}

fn clean(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Renders the Markdown body of a daily report:
///
/// ```text
/// # Relatório — <categoria> — dd/mm/yyyy
/// **Tempo total:** 3 h 25 min
/// ## Destaques
/// - …
/// ## Atividades
/// - **Documentação** — Elaborou o parecer… (1 h 20 min, 09:10–10:30)
///   Evidências: SEI, parecer.docx
/// ```
///
/// A report without items renders "Sem atividade registrada.".
pub fn render_summary_md(report: &DailyReport, category: &Category) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "# Relatório — {} — {}\n\n",
        clean(&category.name),
        format_date_br(report.date)
    ));
    out.push_str(&format!(
        "**Tempo total:** {}\n\n",
        format_minutes(secs_to_minutes(report.total_secs))
    ));

    let highlights: Vec<String> = report
        .highlights
        .iter()
        .map(|h| clean(h))
        .filter(|h| !h.is_empty())
        .collect();
    if !highlights.is_empty() {
        out.push_str("## Destaques\n\n");
        for h in &highlights {
            out.push_str(&format!("- {h}\n"));
        }
        out.push('\n');
    }

    out.push_str("## Atividades\n\n");
    if report.items.is_empty() {
        out.push_str("Sem atividade registrada.\n");
    } else {
        for item in &report.items {
            out.push_str(&render_item(item));
        }
    }
    out
}

fn render_item(item: &ReportItem) -> String {
    let mut line = format!("- **{}** — {}", item.kind.label_pt(), clean(&item.activity));
    let mut meta = vec![format_minutes(item.minutes)];
    let range = clean(&item.time_range);
    if !range.is_empty() {
        meta.push(range);
    }
    line.push_str(&format!(" ({})", meta.join(", ")));
    if let Some(prev) = item
        .continuation_of
        .as_deref()
        .map(clean)
        .filter(|p| !p.is_empty())
    {
        line.push_str(&format!(" — continuação de \"{prev}\""));
    }
    let evidence: Vec<String> = item
        .evidence
        .iter()
        .map(|e| clean(e))
        .filter(|e| !e.is_empty())
        .collect();
    if !evidence.is_empty() {
        line.push_str(&format!("\n  Evidências: {}", evidence.join(", ")));
    }
    line.push('\n');
    line
}

struct MonthlyGroup {
    activity: String,
    kind: ActivityKind,
    minutes: u32,
    dates: Vec<NaiveDate>,
}

fn activity_key(text: &str) -> String {
    clean(text).to_lowercase().trim_end_matches('.').to_string()
}

/// Merges the daily reports of `category` inside `year`/`month` into one Markdown document.
///
/// Items are collapsed across days when they carry the same activity text or when a later item
/// declares `continuation_of` an earlier one (chains are followed), producing lines such as
/// `- Elaborou o edital 12/2026 — 3 dias, 4 h 15 min (02/09, 03/09, 05/09)` grouped by kind.
/// Reports of other categories or months are ignored; when the same day has several reports the
/// most recently generated one wins.
pub fn render_monthly_md(
    reports: &[DailyReport],
    category: &Category,
    year: i32,
    month: u32,
) -> String {
    let mut by_date: BTreeMap<NaiveDate, &DailyReport> = BTreeMap::new();
    for r in reports.iter().filter(|r| {
        r.category_id == category.id && r.date.year() == year && r.date.month() == month
    }) {
        by_date
            .entry(r.date)
            .and_modify(|current| {
                if r.generated_at > current.generated_at {
                    *current = r;
                }
            })
            .or_insert(r);
    }

    let mut groups: Vec<MonthlyGroup> = Vec::new();
    // Every activity text (and continuation alias) seen so far → index of its group.
    let mut aliases: HashMap<String, usize> = HashMap::new();
    let mut total_secs: i64 = 0;

    for (date, report) in &by_date {
        total_secs += report.total_secs.max(0);
        for item in &report.items {
            let key = activity_key(&item.activity);
            if key.is_empty() {
                continue;
            }
            let idx = item
                .continuation_of
                .as_deref()
                .map(activity_key)
                .and_then(|k| aliases.get(&k).copied())
                .or_else(|| aliases.get(&key).copied())
                .unwrap_or_else(|| {
                    groups.push(MonthlyGroup {
                        activity: clean(&item.activity),
                        kind: item.kind,
                        minutes: 0,
                        dates: Vec::new(),
                    });
                    groups.len() - 1
                });
            let group = &mut groups[idx];
            group.minutes = group.minutes.saturating_add(item.minutes);
            if group.dates.last() != Some(date) {
                group.dates.push(*date);
            }
            aliases.entry(key).or_insert(idx);
        }
    }

    let mut out = String::new();
    out.push_str(&format!(
        "# Relatório mensal — {} — {:02}/{}\n\n",
        clean(&category.name),
        month,
        year
    ));
    // Reports exist for every scheduled day, including days without any block in the
    // category: only the ones carrying activity count as active days.
    let active_days = by_date
        .values()
        .filter(|r| !r.items.is_empty() || r.total_secs > 0)
        .count();
    out.push_str(&format!(
        "**Dias com atividade:** {} · **Tempo total:** {}\n\n",
        active_days,
        format_minutes(secs_to_minutes(total_secs))
    ));

    if groups.is_empty() {
        out.push_str("Sem atividade registrada.\n");
        return out;
    }

    for kind in KIND_ORDER {
        let mut of_kind: Vec<&MonthlyGroup> = groups.iter().filter(|g| g.kind == kind).collect();
        if of_kind.is_empty() {
            continue;
        }
        of_kind.sort_by(|a, b| {
            b.minutes
                .cmp(&a.minutes)
                .then_with(|| a.activity.cmp(&b.activity))
        });
        out.push_str(&format!("## {}\n\n", kind.label_pt()));
        for g in of_kind {
            let days = g.dates.len();
            let day_word = if days == 1 { "dia" } else { "dias" };
            let dates: Vec<String> = g
                .dates
                .iter()
                .map(|d| d.format("%d/%m").to_string())
                .collect();
            out.push_str(&format!(
                "- {} — {} {}, {} ({})\n",
                g.activity,
                days,
                day_word,
                format_minutes(g.minutes),
                dates.join(", ")
            ));
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::new_id;
    use chrono::{TimeZone, Utc};

    fn category(id: &str, name: &str) -> Category {
        Category {
            id: id.into(),
            name: name.into(),
            color: "#000".into(),
            icon: "x".into(),
            description: String::new(),
            keywords: vec![],
            report_time: None,
            report_template: None,
            is_productive: true,
            is_system: false,
            archived: false,
            sort_order: 0,
            created_at: Utc::now(),
        }
    }

    fn item(activity: &str, kind: ActivityKind, minutes: u32, cont: Option<&str>) -> ReportItem {
        ReportItem {
            activity: activity.into(),
            kind,
            minutes,
            evidence: vec!["SEI".into(), "".into()],
            time_range: "09:10–10:30".into(),
            continuation_of: cont.map(str::to_string),
        }
    }

    fn report(date: (i32, u32, u32), cat: &str, items: Vec<ReportItem>) -> DailyReport {
        DailyReport {
            id: new_id(),
            date: NaiveDate::from_ymd_opt(date.0, date.1, date.2).unwrap(),
            category_id: cat.into(),
            generated_at: Utc.with_ymd_and_hms(2026, 9, 17, 18, 0, 0).unwrap(),
            summary_md: String::new(),
            items,
            highlights: vec!["Fechou o edital".into()],
            total_secs: 3 * 3600 + 25 * 60,
            model: "test".into(),
            input_tokens: 0,
            output_tokens: 0,
            stale: false,
            edited: false,
        }
    }

    #[test]
    fn minutes_formatting() {
        assert_eq!(format_minutes(0), "0 min");
        assert_eq!(format_minutes(45), "45 min");
        assert_eq!(format_minutes(120), "2 h");
        assert_eq!(format_minutes(80), "1 h 20 min");
        assert_eq!(format_minutes(65), "1 h 05 min");
    }

    #[test]
    fn daily_summary_renders_everything() {
        let cat = category("c1", "IFRO");
        let r = report(
            (2026, 9, 17),
            "c1",
            vec![item(
                "Elaborou o parecer do edital 12/2026",
                ActivityKind::Documentacao,
                80,
                Some("Iniciou o parecer"),
            )],
        );
        let md = render_summary_md(&r, &cat);
        assert!(md.starts_with("# Relatório — IFRO — 17/09/2026\n"), "{md}");
        assert!(md.contains("**Tempo total:** 3 h 25 min"), "{md}");
        assert!(md.contains("## Destaques\n\n- Fechou o edital\n"), "{md}");
        assert!(
            md.contains("- **Documentação** — Elaborou o parecer do edital 12/2026 (1 h 20 min, 09:10–10:30) — continuação de \"Iniciou o parecer\"\n  Evidências: SEI\n"),
            "{md}"
        );
    }

    #[test]
    fn daily_summary_without_items() {
        let cat = category("c1", "IFRO");
        let mut r = report((2026, 9, 17), "c1", vec![]);
        r.highlights.clear();
        r.total_secs = 0;
        let md = render_summary_md(&r, &cat);
        assert!(md.contains("**Tempo total:** 0 min"));
        assert!(!md.contains("## Destaques"));
        assert!(
            md.ends_with("## Atividades\n\nSem atividade registrada.\n"),
            "{md}"
        );
    }

    #[test]
    fn monthly_merges_and_collapses_continuations() {
        let cat = category("c1", "IFRO");
        let reports = vec![
            report(
                (2026, 9, 2),
                "c1",
                vec![
                    item("Elaborou o edital 12/2026", ActivityKind::Gestao, 60, None),
                    item(
                        "Participou de reunião de planejamento",
                        ActivityKind::Reuniao,
                        30,
                        None,
                    ),
                ],
            ),
            report(
                (2026, 9, 3),
                "c1",
                vec![item(
                    "Elaborou o edital 12/2026",
                    ActivityKind::Gestao,
                    75,
                    None,
                )],
            ),
            report(
                (2026, 9, 5),
                "c1",
                vec![item(
                    "Finalizou o edital 12/2026",
                    ActivityKind::Gestao,
                    120,
                    Some("Elaborou o edital 12/2026"),
                )],
            ),
            report(
                (2026, 9, 7),
                "c1",
                vec![item(
                    "Publicou o edital",
                    ActivityKind::Gestao,
                    15,
                    Some("Finalizou o edital 12/2026"),
                )],
            ),
            // A scheduled report for a day without activity: not an active day.
            {
                let mut empty = report((2026, 9, 6), "c1", vec![]);
                empty.total_secs = 0;
                empty
            },
            // Other month and other category: ignored.
            report(
                (2026, 8, 30),
                "c1",
                vec![item("Outra coisa", ActivityKind::Outro, 10, None)],
            ),
            report(
                (2026, 9, 4),
                "c2",
                vec![item("Aula", ActivityKind::Ensino, 10, None)],
            ),
        ];
        let md = render_monthly_md(&reports, &cat, 2026, 9);
        assert!(
            md.starts_with("# Relatório mensal — IFRO — 09/2026\n"),
            "{md}"
        );
        assert!(md.contains("**Dias com atividade:** 4"), "{md}");
        assert!(
            md.contains(
                "- Elaborou o edital 12/2026 — 4 dias, 4 h 30 min (02/09, 03/09, 05/09, 07/09)\n"
            ),
            "{md}"
        );
        assert!(
            md.contains(
                "## Reunião\n\n- Participou de reunião de planejamento — 1 dia, 30 min (02/09)\n"
            ),
            "{md}"
        );
        assert!(!md.contains("Outra coisa"));
        assert!(!md.contains("Aula"));
        // Kind sections follow KIND_ORDER: Reunião before Gestão.
        assert!(md.find("## Reunião").unwrap() < md.find("## Gestão").unwrap());
    }

    #[test]
    fn monthly_without_reports() {
        let cat = category("c1", "IFRO");
        let md = render_monthly_md(&[], &cat, 2026, 9);
        assert!(md.contains("**Dias com atividade:** 0"));
        assert!(md.ends_with("Sem atividade registrada.\n"));
    }
}
