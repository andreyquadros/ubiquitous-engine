//! Deterministic, network-free implementations of the AI ports and test doubles for the client.
//!
//! Used by the engine's tests and by the Linux CLI, where no key (and no macOS) is available:
//! [`FakeClassifier`], [`FakeVisionClassifier`], [`FakeReportWriter`], [`FakeAdvisor`], plus
//! [`MemoryUsageRepo`] (an in-memory `UsageRepo`) and [`ScriptedLlmClient`] (a scripted
//! `LlmClient` that records the requests it receives).

use std::collections::{BTreeMap, VecDeque};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ubiqx_core::ports::{
    Advice, AdviceRequest, Advisor, AiUsage, AiUsageTotals, Classification, ClassificationContext,
    EncodedImage, RemoteClassifier, ReportRequest, ReportWriter, UsageRepo, VisionClassifier,
};
use ubiqx_core::report::format_minutes;
use ubiqx_core::{
    ActivityBlock, ActivityKind, ClassificationSource, CoreError, CoreResult, DailyReport,
    ReportItem, TimeRange,
};

use crate::client::{LlmClient, LlmRequest, LlmResponse, StopReason};
use crate::prompts::{format_time_range, PromptBlock};
use crate::report::{build_report, round_minutes_to_5};

/// Model name reported by every fake.
pub const FAKE_MODEL: &str = "fake";

// ---------------------------------------------------------------------------------------------
// Usage repo
// ---------------------------------------------------------------------------------------------

/// In-memory [`UsageRepo`].
#[derive(Debug, Default)]
pub struct MemoryUsageRepo {
    records: Mutex<Vec<AiUsage>>,
}

impl MemoryUsageRepo {
    pub fn new() -> Self {
        Self::default()
    }

    /// Everything recorded so far, in order.
    pub fn records(&self) -> Vec<AiUsage> {
        self.records.lock().map(|r| r.clone()).unwrap_or_default()
    }
}

impl UsageRepo for MemoryUsageRepo {
    fn record(&self, usage: &AiUsage) -> CoreResult<()> {
        self.records
            .lock()
            .map_err(|_| CoreError::Storage("usage repo poisoned".into()))?
            .push(usage.clone());
        Ok(())
    }

    fn totals(&self, range: TimeRange) -> CoreResult<AiUsageTotals> {
        let records = self.records();
        let mut totals = AiUsageTotals::default();
        for u in records.iter().filter(|u| range.contains(u.at)) {
            totals.calls += 1;
            totals.input_tokens += u64::from(u.input_tokens);
            totals.output_tokens += u64::from(u.output_tokens);
            totals.cost_usd += u.cost_usd;
        }
        Ok(totals)
    }
}

// ---------------------------------------------------------------------------------------------
// Scripted LLM client
// ---------------------------------------------------------------------------------------------

/// An [`LlmClient`] that answers with scripted texts (in order) and records every request.
/// Once the script is exhausted it fails with `CoreError::Ai`.
#[derive(Debug, Default)]
pub struct ScriptedLlmClient {
    responses: Mutex<VecDeque<LlmResponse>>,
    requests: Mutex<Vec<LlmRequest>>,
    truncated: Mutex<Vec<usize>>,
}

impl ScriptedLlmClient {
    /// Each text becomes one `end_turn` response with 1000 input / 100 output tokens.
    pub fn new(texts: Vec<String>) -> Self {
        let responses = texts
            .into_iter()
            .map(|text| LlmResponse {
                text,
                model: FAKE_MODEL.into(),
                input_tokens: 1000,
                output_tokens: 100,
                cache_read_tokens: 0,
                cache_write_tokens: 0,
                stop_reason: StopReason::EndTurn,
            })
            .collect();
        Self {
            responses: Mutex::new(responses),
            requests: Mutex::new(Vec::new()),
            truncated: Mutex::new(Vec::new()),
        }
    }

    /// Appends a fully specified response.
    pub fn push_response(&self, response: LlmResponse) {
        if let Ok(mut r) = self.responses.lock() {
            r.push_back(response);
        }
    }

    /// Marks the `index`-th response (0-based, in script order) as cut off by `max_tokens`.
    pub fn truncate_response(&self, index: usize) {
        if let Ok(mut t) = self.truncated.lock() {
            t.push(index);
        }
    }

    /// Requests received so far, in order.
    pub fn requests(&self) -> Vec<LlmRequest> {
        self.requests.lock().map(|r| r.clone()).unwrap_or_default()
    }
}

#[async_trait]
impl LlmClient for ScriptedLlmClient {
    async fn complete(&self, req: &LlmRequest) -> CoreResult<LlmResponse> {
        let index = {
            let mut requests = self
                .requests
                .lock()
                .map_err(|_| CoreError::Ai("scripted client poisoned".into()))?;
            requests.push(req.clone());
            requests.len() - 1
        };
        let mut response = self
            .responses
            .lock()
            .map_err(|_| CoreError::Ai("scripted client poisoned".into()))?
            .pop_front()
            .ok_or_else(|| CoreError::Ai("scripted client: no more responses".into()))?;
        let truncated = self
            .truncated
            .lock()
            .map(|t| t.contains(&index))
            .unwrap_or(false);
        if truncated {
            response.stop_reason = StopReason::MaxTokens;
        }
        Ok(response)
    }
}

// ---------------------------------------------------------------------------------------------
// Classifiers
// ---------------------------------------------------------------------------------------------

/// Keyword → category classifier. The first keyword (case-insensitive substring of the app
/// name, domain or title) wins; unmatched blocks come back with `category_id = None`.
#[derive(Debug, Default)]
pub struct FakeClassifier {
    rules: Vec<(String, String)>,
    calls: AtomicUsize,
}

impl FakeClassifier {
    pub fn new<I, K, C>(rules: I) -> Self
    where
        I: IntoIterator<Item = (K, C)>,
        K: Into<String>,
        C: Into<String>,
    {
        Self {
            rules: rules
                .into_iter()
                .map(|(k, c)| (k.into().to_lowercase(), c.into()))
                .collect(),
            calls: AtomicUsize::new(0),
        }
    }

    /// Number of `classify_batch` calls so far.
    pub fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }

    fn category_for(&self, block: &ActivityBlock) -> Option<String> {
        let haystack = format!(
            "{} {} {}",
            block.app_name,
            block.domain.as_deref().unwrap_or(""),
            block.title
        )
        .to_lowercase();
        self.rules
            .iter()
            .find(|(k, _)| !k.is_empty() && haystack.contains(k.as_str()))
            .map(|(_, c)| c.clone())
    }
}

#[async_trait]
impl RemoteClassifier for FakeClassifier {
    fn name(&self) -> &'static str {
        "fake"
    }

    async fn classify_batch(
        &self,
        blocks: &[ActivityBlock],
        _ctx: &ClassificationContext,
    ) -> CoreResult<Vec<Classification>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(blocks
            .iter()
            .map(|b| match self.category_for(b) {
                Some(category_id) => Classification {
                    block_id: b.id.clone(),
                    category_id: Some(category_id),
                    confidence: 0.9,
                    source: ClassificationSource::Llm,
                    description: None,
                    needs_vision: false,
                    rule_id: None,
                },
                None => Classification::unknown(&b.id, ClassificationSource::Llm),
            })
            .collect())
    }
}

/// Always answers the same category and description.
#[derive(Debug, Clone, Default)]
pub struct FakeVisionClassifier {
    category_id: Option<String>,
    description: String,
    calls: std::sync::Arc<AtomicUsize>,
}

impl FakeVisionClassifier {
    pub fn new(category_id: Option<&str>, description: &str) -> Self {
        Self {
            category_id: category_id.map(str::to_string),
            description: description.to_string(),
            calls: Default::default(),
        }
    }

    pub fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl VisionClassifier for FakeVisionClassifier {
    async fn classify_with_image(
        &self,
        block: &ActivityBlock,
        _image: &EncodedImage,
        _ctx: &ClassificationContext,
    ) -> CoreResult<Classification> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(Classification {
            block_id: block.id.clone(),
            category_id: self.category_id.clone(),
            confidence: if self.category_id.is_some() { 0.9 } else { 0.0 },
            source: ClassificationSource::Vision,
            description: (!self.description.is_empty()).then(|| self.description.clone()),
            needs_vision: false,
            rule_id: None,
        })
    }
}

// ---------------------------------------------------------------------------------------------
// Report writer
// ---------------------------------------------------------------------------------------------

/// Guesses the kind of an activity from app name and domain.
fn guess_kind(app: &str, domain: Option<&str>) -> ActivityKind {
    let hay = format!("{} {}", app, domain.unwrap_or("")).to_lowercase();
    let any = |needles: &[&str]| needles.iter().any(|n| hay.contains(n));
    if any(&["meet", "zoom", "teams", "webex"]) {
        ActivityKind::Reuniao
    } else if any(&[
        "mail", "whatsapp", "slack", "outlook", "telegram", "discord", "messages",
    ]) {
        ActivityKind::Comunicacao
    } else if any(&[
        "code", "xcode", "terminal", "iterm", "github", "gitlab", "intellij", "cursor",
    ]) {
        ActivityKind::Desenvolvimento
    } else if any(&["moodle", "classroom", "suap", "ava"]) {
        ActivityKind::Ensino
    } else if any(&["sei.", "sei ", "sipac", "sigaa", "planejamento"]) {
        ActivityKind::Gestao
    } else if any(&[
        "docs",
        "word",
        "excel",
        "sheets",
        "pages",
        "numbers",
        "keynote",
        "powerpoint",
        "pdf",
        "preview",
        "notion",
    ]) {
        ActivityKind::Documentacao
    } else if any(&["scholar", "periodicos", "arxiv", "researchgate"]) {
        ActivityKind::Pesquisa
    } else {
        ActivityKind::Outro
    }
}

struct Group {
    app: String,
    topic: String,
    domain: Option<String>,
    kind: ActivityKind,
    first: DateTime<Utc>,
    last: DateTime<Utc>,
    secs: i64,
}

/// One item per `(app, domain-or-title)` group with summed minutes; no network.
#[derive(Debug, Default, Clone, Copy)]
pub struct FakeReportWriter;

impl FakeReportWriter {
    pub fn new() -> Self {
        Self
    }

    /// The deterministic items for `req` (public so the CLI can preview them).
    pub fn items_for(req: &ReportRequest) -> Vec<ReportItem> {
        let mut blocks: Vec<&ActivityBlock> = req.blocks.iter().collect();
        blocks.sort_by_key(|b| b.started_at);
        let mut groups: BTreeMap<(String, String), Group> = BTreeMap::new();
        let mut order: Vec<(String, String)> = Vec::new();
        for b in blocks {
            let view = PromptBlock::from_block(b, req.utc_offset_secs);
            let topic = view
                .domain()
                .map(str::to_string)
                .unwrap_or_else(|| view.title().to_string());
            let key = (view.app_name().to_string(), topic.clone());
            let g = groups.entry(key.clone()).or_insert_with(|| {
                order.push(key);
                Group {
                    app: view.app_name().to_string(),
                    topic,
                    domain: view.domain().map(str::to_string),
                    kind: guess_kind(view.app_name(), view.domain()),
                    first: b.started_at,
                    last: b.ended_at,
                    secs: 0,
                }
            });
            g.first = g.first.min(b.started_at);
            g.last = g.last.max(b.ended_at);
            g.secs += b.duration_secs();
        }
        order
            .iter()
            .filter_map(|k| groups.get(k))
            .map(|g| {
                let mut evidence = vec![g.app.clone()];
                if let Some(d) = &g.domain {
                    evidence.push(d.clone());
                }
                ReportItem {
                    activity: format!("Utilizou {} em atividades de {}", g.app, g.topic),
                    kind: g.kind,
                    minutes: round_minutes_to_5(
                        u32::try_from((g.secs + 30) / 60).unwrap_or(u32::MAX),
                    ),
                    evidence,
                    time_range: format_time_range(g.first, g.last, req.utc_offset_secs),
                    continuation_of: None,
                }
            })
            .collect()
    }
}

#[async_trait]
impl ReportWriter for FakeReportWriter {
    fn describe_payload(&self, block: &ActivityBlock, utc_offset_secs: i32) -> String {
        PromptBlock::from_block(block, utc_offset_secs).render_report_line()
    }

    async fn write_daily(&self, req: &ReportRequest) -> CoreResult<DailyReport> {
        let items = Self::items_for(req);
        let highlights = items
            .iter()
            .max_by_key(|i| i.minutes)
            .map(|i| vec![format!("{} ({})", i.activity, format_minutes(i.minutes))])
            .unwrap_or_else(|| vec!["Sem atividade registrada".to_string()]);
        Ok(build_report(req, items, highlights, FAKE_MODEL, 0, 0))
    }
}

// ---------------------------------------------------------------------------------------------
// Advisor
// ---------------------------------------------------------------------------------------------

/// Rule-based recommendations from the aggregated numbers; no network.
#[derive(Debug, Default, Clone, Copy)]
pub struct FakeAdvisor;

impl FakeAdvisor {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Advisor for FakeAdvisor {
    async fn advise(&self, req: &AdviceRequest) -> CoreResult<Advice> {
        let st = &req.stats;
        let headline = if st.focus_score >= 75 {
            "Semana de foco excelente"
        } else if st.focus_score >= 50 {
            "Semana de foco razoável"
        } else {
            "Semana dispersa"
        }
        .to_string();
        let mut recommendations = Vec::new();
        if st.switches_per_hour > 12.0 {
            recommendations.push(format!(
                "Reduza as trocas de contexto ({:.0} por hora): reserve blocos de 45 minutos em um único aplicativo.",
                st.switches_per_hour
            ));
        }
        if st.distraction_secs > 0 && st.distraction_secs * 4 > st.productive_secs.max(1) {
            recommendations.push(format!(
                "Limite o tempo em distrações ({} na semana).",
                format_minutes(u32::try_from(st.distraction_secs.max(0) / 60).unwrap_or(u32::MAX))
            ));
        }
        if st.longest_focus_secs > 90 * 60 {
            recommendations
                .push("Faça pausas curtas a cada 90 minutos de foco contínuo.".to_string());
        }
        if st.uncategorized_secs > 0 {
            recommendations.push(
                "Classifique os blocos pendentes para relatórios mais completos.".to_string(),
            );
        }
        if let Some((name, _)) = req.category_totals.iter().max_by_key(|(_, s)| *s) {
            recommendations.push(format!(
                "Mantenha o ritmo em {name}, sua categoria com mais horas."
            ));
        }
        recommendations.truncate(crate::advisor::MAX_RECOMMENDATIONS);
        Ok(Advice {
            headline,
            recommendations,
            model: FAKE_MODEL.into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prompts::test_support::{block, category, ctx};
    use ubiqx_core::{FocusStats, Mood};

    #[tokio::test]
    async fn fake_classifier_matches_keywords() {
        let c = FakeClassifier::new([("sei.ifro", "cat-ifro"), ("youtube", "sys-distraction")]);
        let blocks = vec![
            block(
                "b1",
                "com.google.Chrome",
                "Google Chrome",
                "SEI",
                Some("https://SEI.ifro.edu.br/x"),
            ),
            block(
                "b2",
                "com.google.Chrome",
                "Google Chrome",
                "Video",
                Some("https://www.youtube.com/watch?v=1"),
            ),
            block("b3", "com.apple.finder", "Finder", "Downloads", None),
        ];
        let out = c.classify_batch(&blocks, &ctx(vec![])).await.unwrap();
        assert_eq!(out[0].category_id.as_deref(), Some("cat-ifro"));
        assert_eq!(out[1].category_id.as_deref(), Some("sys-distraction"));
        assert_eq!(out[2].category_id, None);
        assert!(out.iter().all(|c| c.source == ClassificationSource::Llm));
        assert_eq!(c.calls(), 1);
    }

    #[tokio::test]
    async fn fake_vision() {
        let v = FakeVisionClassifier::new(Some("cat-ifro"), "Editou um documento");
        let img = EncodedImage {
            bytes: vec![0],
            mime: "image/png".into(),
            width: 1,
            height: 1,
        };
        let out = v
            .classify_with_image(&block("b1", "a", "A", "t", None), &img, &ctx(vec![]))
            .await
            .unwrap();
        assert_eq!(out.category_id.as_deref(), Some("cat-ifro"));
        assert_eq!(out.source, ClassificationSource::Vision);
        assert_eq!(out.description.as_deref(), Some("Editou um documento"));
        assert_eq!(v.calls(), 1);
    }

    #[tokio::test]
    async fn fake_report_groups_blocks() {
        let mut b1 = block(
            "b1",
            "com.google.Chrome",
            "Google Chrome",
            "SEI - x",
            Some("https://sei.ifro.edu.br/a"),
        );
        let mut b2 = block(
            "b2",
            "com.google.Chrome",
            "Google Chrome",
            "SEI - y",
            Some("https://sei.ifro.edu.br/b"),
        );
        b2.started_at = b1.ended_at + chrono::Duration::minutes(10);
        b2.ended_at = b2.started_at + chrono::Duration::minutes(7);
        let b3 = block(
            "b3",
            "us.zoom.xos",
            "zoom.us",
            "Reunião com ana@ifro.edu.br",
            None,
        );
        b1.description = None;
        b2.description = None;
        let req = ReportRequest {
            date: chrono::NaiveDate::from_ymd_opt(2026, 9, 17).unwrap(),
            category: category("cat-ifro", "IFRO"),
            blocks: vec![b3.clone(), b1.clone(), b2.clone()],
            previous_items: vec![],
            language: "pt-BR".into(),
            utc_offset_secs: -4 * 3600,
            user_profile: None,
            model: String::new(),
        };
        let report = FakeReportWriter::new().write_daily(&req).await.unwrap();
        assert_eq!(report.items.len(), 2);
        let sei = report
            .items
            .iter()
            .find(|i| i.activity.contains("sei.ifro.edu.br"))
            .unwrap();
        assert_eq!(sei.minutes, 30, "25 + 7 = 32 → 30");
        assert_eq!(sei.kind, ActivityKind::Gestao);
        assert_eq!(sei.time_range, "09:10–09:52");
        assert_eq!(sei.evidence, vec!["Google Chrome", "sei.ifro.edu.br"]);
        let zoom = report
            .items
            .iter()
            .find(|i| i.activity.contains("zoom.us"))
            .unwrap();
        assert_eq!(zoom.kind, ActivityKind::Reuniao);
        assert!(!zoom.activity.contains("ana@"), "{}", zoom.activity);
        assert_eq!(report.model, FAKE_MODEL);
        assert_eq!(report.total_secs, 25 * 60 + 7 * 60 + 25 * 60);
        assert!(report.summary_md.contains("## Destaques"));
        assert!(report
            .summary_md
            .contains("# Relatório — IFRO — 17/09/2026"));
    }

    #[tokio::test]
    async fn fake_advisor_and_usage_repo() {
        let req = AdviceRequest {
            language: "pt-BR".into(),
            stats: FocusStats {
                focus_score: 40,
                productive_secs: 3600,
                distraction_secs: 3000,
                uncategorized_secs: 60,
                idle_secs: 0,
                total_secs: 6660,
                switches_per_hour: 20.0,
                longest_focus_secs: 100 * 60,
                mood: Mood::Worried,
            },
            category_totals: vec![("IFRO".into(), 3600)],
            top_apps: vec![],
            recent_nudges: vec![],
            user_profile: None,
            model: String::new(),
        };
        let advice = FakeAdvisor::new().advise(&req).await.unwrap();
        assert_eq!(advice.headline, "Semana dispersa");
        assert_eq!(advice.recommendations.len(), 5);

        let repo = MemoryUsageRepo::new();
        let now = Utc::now();
        repo.record(&AiUsage {
            at: now,
            kind: ubiqx_core::ports::AiUsageKind::Classify,
            model: "m".into(),
            input_tokens: 10,
            output_tokens: 5,
            cache_read_tokens: 0,
            cache_write_tokens: 0,
            cost_usd: 0.5,
        })
        .unwrap();
        let totals = repo
            .totals(TimeRange::new(
                now - chrono::Duration::hours(1),
                now + chrono::Duration::hours(1),
            ))
            .unwrap();
        assert_eq!(totals.calls, 1);
        assert_eq!(totals.input_tokens, 10);
        assert_eq!(totals.cost_usd, 0.5);
        let none = repo
            .totals(TimeRange::new(
                now + chrono::Duration::hours(1),
                now + chrono::Duration::hours(2),
            ))
            .unwrap();
        assert_eq!(none.calls, 0);
    }
}
