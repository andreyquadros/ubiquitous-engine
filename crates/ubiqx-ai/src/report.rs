//! Daily report writer: [`LlmReportWriter`] implements [`ReportWriter`].
//!
//! System prompt = stable instructions + per-category context (profile, category, template),
//! the latter carrying the prompt-cache breakpoint on Sonnet. The user message is the day:
//! date, previous items and redacted block lines with local times. The answer is validated and
//! rendered to Markdown with [`ubiqx_core::report::render_summary_md`].

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use serde::Deserialize;
use serde_json::{json, Value};
use tracing::{info, warn};
use ubiqx_core::ports::{AiUsageKind, ReportRequest, ReportWriter};
use ubiqx_core::redact::redact_text;
use ubiqx_core::report::render_summary_md;
use ubiqx_core::{
    new_id, ActivityBlock, ActivityKind, AiModels, CoreError, CoreResult, DailyReport, ReportItem,
};

use crate::classifier::model_or_default;
use crate::client::{
    prompt_cache_for, thinking_disabled_for, LlmClient, LlmMessage, LlmRequest, SystemBlock,
    LONG_TIMEOUT,
};
use crate::prompts;

/// Output budget of a daily report.
pub const MAX_TOKENS: u32 = 4000;
/// Highlights kept from the answer.
pub const MAX_HIGHLIGHTS: usize = 5;
/// Evidence entries kept per item.
pub const MAX_EVIDENCE: usize = 6;
/// Upper bound of minutes for one item (a day).
pub const MAX_ITEM_MINUTES: u32 = 24 * 60;

/// JSON schema of the answer.
pub fn report_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "items": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "activity": {"type": "string"},
                        "kind": {
                            "type": "string",
                            "enum": ["desenvolvimento", "reuniao", "comunicacao", "documentacao", "ensino", "pesquisa", "extensao", "gestao", "outro"]
                        },
                        "minutes": {"type": "integer"},
                        "evidence": {"type": "array", "items": {"type": "string"}},
                        "time_range": {"type": "string"},
                        "continuation_of": {"anyOf": [{"type": "string"}, {"type": "null"}]}
                    },
                    "required": ["activity", "kind", "minutes", "evidence", "time_range", "continuation_of"],
                    "additionalProperties": false
                }
            },
            "highlights": {"type": "array", "items": {"type": "string"}}
        },
        "required": ["items", "highlights"],
        "additionalProperties": false
    })
}

/// Rounds to the nearest multiple of 5 (anything positive is at least 5).
pub fn round_minutes_to_5(minutes: u32) -> u32 {
    if minutes == 0 {
        return 0;
    }
    (((minutes + 2) / 5) * 5).max(5)
}

fn tidy(text: &str) -> String {
    redact_text(text)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Report writer backed by the Messages API.
pub struct LlmReportWriter {
    client: Arc<dyn LlmClient>,
}

impl LlmReportWriter {
    pub fn new(client: Arc<dyn LlmClient>) -> Self {
        Self { client }
    }

    /// The request for `req` (model from `req.model`, cache breakpoint on the last system
    /// block for Sonnet/Opus, thinking disabled, 120 s timeout).
    pub fn build_request(req: &ReportRequest) -> LlmRequest {
        let model = model_or_default(&req.model, &AiModels::default().report);
        LlmRequest {
            system: vec![
                SystemBlock::new(prompts::report_instructions(&req.language), false),
                SystemBlock::new(prompts::report_context(req), prompt_cache_for(&model)),
            ],
            messages: vec![LlmMessage::user_text(prompts::report_user_message(req))],
            max_tokens: MAX_TOKENS,
            json_schema: Some(report_schema()),
            disable_thinking: thinking_disabled_for(&model),
            usage_kind: AiUsageKind::Report,
            timeout: Some(LONG_TIMEOUT),
            model,
        }
    }
}

#[async_trait]
impl ReportWriter for LlmReportWriter {
    fn describe_payload(&self, block: &ActivityBlock, utc_offset_secs: i32) -> String {
        prompts::PromptBlock::from_block(block, utc_offset_secs).render_report_line()
    }

    async fn write_daily(&self, req: &ReportRequest) -> CoreResult<DailyReport> {
        if req.blocks.is_empty() {
            // Nothing to write about: no call, no cost. The renderer prints
            // "Sem atividade registrada.".
            info!(date = %req.date, category = %req.category.id, "report: no blocks, writing empty report");
            let model = model_or_default(&req.model, &AiModels::default().report);
            return Ok(build_report(req, vec![], vec![], &model, 0, 0));
        }

        let mut llm_req = Self::build_request(req);
        let mut resp = self.client.complete(&llm_req).await?;
        if resp.is_truncated() {
            llm_req.max_tokens = llm_req.max_tokens.saturating_mul(2);
            warn!(date = %req.date, max_tokens = llm_req.max_tokens, "report: answer truncated, retrying once");
            resp = self.client.complete(&llm_req).await?;
            if resp.is_truncated() {
                return Err(CoreError::Ai("report output truncated twice".into()));
            }
        }
        let (items, highlights) = parse_report(&resp.text)?;
        info!(
            date = %req.date,
            category = %req.category.id,
            items = items.len(),
            blocks = req.blocks.len(),
            "report: written"
        );
        Ok(build_report(
            req,
            items,
            highlights,
            &llm_req.model,
            resp.input_tokens,
            resp.output_tokens,
        ))
    }
}

#[derive(Debug, Deserialize)]
struct RawReport {
    #[serde(default)]
    items: Vec<RawItem>,
    #[serde(default)]
    highlights: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RawItem {
    #[serde(default)]
    activity: String,
    #[serde(default)]
    kind: String,
    #[serde(default)]
    minutes: f64,
    #[serde(default)]
    evidence: Vec<String>,
    #[serde(default)]
    time_range: String,
    #[serde(default)]
    continuation_of: Option<String>,
}

/// Validates the answer: items without activity text are dropped, unknown kinds become
/// `Outro`, minutes are rounded to 5 and capped at a day, evidence/highlights are masked,
/// trimmed and bounded.
pub fn parse_report(text: &str) -> CoreResult<(Vec<ReportItem>, Vec<String>)> {
    let raw: RawReport = serde_json::from_str(text.trim())
        .map_err(|e| CoreError::Ai(format!("report writer returned invalid JSON: {e}")))?;
    let items = raw
        .items
        .into_iter()
        .filter_map(|r| {
            let activity = tidy(&r.activity);
            if activity.is_empty() {
                return None;
            }
            let minutes = if r.minutes.is_finite() && r.minutes > 0.0 {
                round_minutes_to_5(r.minutes.round().min(f64::from(MAX_ITEM_MINUTES)) as u32)
            } else {
                0
            };
            Some(ReportItem {
                activity,
                kind: ActivityKind::parse(r.kind.trim()).unwrap_or(ActivityKind::Outro),
                minutes,
                evidence: r
                    .evidence
                    .iter()
                    .map(|e| tidy(e))
                    .filter(|e| !e.is_empty())
                    .take(MAX_EVIDENCE)
                    .collect(),
                time_range: r.time_range.trim().to_string(),
                continuation_of: r
                    .continuation_of
                    .map(|c| tidy(&c))
                    .filter(|c| !c.is_empty()),
            })
        })
        .collect();
    let highlights = raw
        .highlights
        .iter()
        .map(|h| tidy(h))
        .filter(|h| !h.is_empty())
        .take(MAX_HIGHLIGHTS)
        .collect();
    Ok((items, highlights))
}

/// Assembles the [`DailyReport`] (fresh id, `generated_at = now`, `total_secs` = sum of block
/// durations, `summary_md` rendered by the core renderer, not stale, not edited).
pub fn build_report(
    req: &ReportRequest,
    items: Vec<ReportItem>,
    highlights: Vec<String>,
    model: &str,
    input_tokens: u32,
    output_tokens: u32,
) -> DailyReport {
    let mut report = DailyReport {
        id: new_id(),
        date: req.date,
        category_id: req.category.id.clone(),
        generated_at: Utc::now(),
        summary_md: String::new(),
        items,
        highlights,
        total_secs: req.blocks.iter().map(ActivityBlock::duration_secs).sum(),
        model: model.to_string(),
        input_tokens,
        output_tokens,
        stale: false,
        edited: false,
    };
    report.summary_md =
        render_summary_md(&report, &req.category, prompts::ui_language(&req.language));
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::ScriptedLlmClient;
    use crate::prompts::test_support::{block, category};

    fn request(blocks: Vec<ActivityBlock>) -> ReportRequest {
        ReportRequest {
            date: chrono::NaiveDate::from_ymd_opt(2026, 9, 17).unwrap(),
            category: category("cat-ifro", "IFRO"),
            blocks,
            previous_items: vec![],
            language: "pt-BR".into(),
            utc_offset_secs: -4 * 3600,
            user_profile: None,
            model: "claude-sonnet-5".into(),
        }
    }

    #[test]
    fn rounding() {
        assert_eq!(round_minutes_to_5(0), 0);
        assert_eq!(round_minutes_to_5(1), 5);
        assert_eq!(round_minutes_to_5(7), 5);
        assert_eq!(round_minutes_to_5(8), 10);
        assert_eq!(round_minutes_to_5(80), 80);
        assert_eq!(round_minutes_to_5(83), 85);
    }

    #[test]
    fn schema_is_structured_output_compatible() {
        let s = report_schema().to_string();
        assert!(s.contains("\"additionalProperties\":false"));
        for forbidden in ["minimum", "maximum", "minLength", "maxLength"] {
            assert!(!s.contains(forbidden));
        }
    }

    #[test]
    fn request_shape() {
        let req = LlmReportWriter::build_request(&request(vec![block("b1", "a", "A", "t", None)]));
        assert_eq!(req.model, "claude-sonnet-5");
        assert_eq!(req.max_tokens, MAX_TOKENS);
        assert!(req.disable_thinking);
        assert_eq!(req.system.len(), 2);
        assert!(!req.system[0].cache);
        assert!(
            req.system[1].cache,
            "cache breakpoint on the last system block"
        );
        assert_eq!(req.timeout, Some(LONG_TIMEOUT));
        assert_eq!(req.usage_kind, AiUsageKind::Report);
        let mut r = request(vec![]);
        r.model = "claude-haiku-4-5".into();
        let req = LlmReportWriter::build_request(&r);
        assert!(!req.disable_thinking);
        assert!(!req.system[1].cache);
    }

    #[test]
    fn parsing_validates_items() {
        let text = json!({
            "items": [
                {"activity": " Elaborou o parecer  do edital ", "kind": "documentacao", "minutes": 83, "evidence": ["SEI", " ", "parecer.docx"], "time_range": "09:10–10:33", "continuation_of": ""},
                {"activity": "", "kind": "gestao", "minutes": 10, "evidence": [], "time_range": "", "continuation_of": null},
                {"activity": "Respondeu a maria@ifro.edu.br", "kind": "weird", "minutes": -4, "evidence": [], "time_range": "", "continuation_of": "Atendimento por e-mail"}
            ],
            "highlights": ["Fechou o parecer", "", "b", "c", "d", "e", "f"]
        })
        .to_string();
        let (items, highlights) = parse_report(&text).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].activity, "Elaborou o parecer do edital");
        assert_eq!(items[0].kind, ActivityKind::Documentacao);
        assert_eq!(items[0].minutes, 85);
        assert_eq!(items[0].evidence, vec!["SEI", "parecer.docx"]);
        assert_eq!(items[0].continuation_of, None);
        assert_eq!(items[1].activity, "Respondeu a [email]");
        assert_eq!(items[1].kind, ActivityKind::Outro);
        assert_eq!(items[1].minutes, 0);
        assert_eq!(
            items[1].continuation_of.as_deref(),
            Some("Atendimento por e-mail")
        );
        assert_eq!(highlights.len(), MAX_HIGHLIGHTS);
        assert!(matches!(parse_report("["), Err(CoreError::Ai(_))));
    }

    #[tokio::test]
    async fn writes_report_and_renders_markdown() {
        let client = Arc::new(ScriptedLlmClient::new(vec![json!({
            "items": [{"activity": "Elaborou o parecer do edital 12/2026", "kind": "documentacao", "minutes": 25, "evidence": ["SEI"], "time_range": "09:10–09:35", "continuation_of": null}],
            "highlights": ["Parecer concluído"]
        })
        .to_string()]));
        let req = request(vec![block(
            "b1",
            "com.google.Chrome",
            "Google Chrome",
            "SEI - parecer",
            Some("https://sei.ifro.edu.br/x?y=1"),
        )]);
        let report = LlmReportWriter::new(client.clone())
            .write_daily(&req)
            .await
            .unwrap();
        assert_eq!(report.date, req.date);
        assert_eq!(report.category_id, "cat-ifro");
        assert_eq!(report.total_secs, 25 * 60);
        assert_eq!(report.model, "claude-sonnet-5");
        assert_eq!(report.input_tokens, 1000);
        assert_eq!(report.output_tokens, 100);
        assert!(!report.stale && !report.edited);
        assert_eq!(report.items.len(), 1);
        assert!(
            report
                .summary_md
                .starts_with("# Relatório — IFRO — 17/09/2026\n"),
            "{}",
            report.summary_md
        );
        assert!(report.summary_md.contains("- **Documentação** — Elaborou o parecer do edital 12/2026 (25 min, 09:10–09:35)\n  Evidências: SEI"));
        assert!(report
            .summary_md
            .contains("## Destaques\n\n- Parecer concluído"));
        assert_eq!(client.requests().len(), 1);
    }

    #[tokio::test]
    async fn empty_day_makes_no_call() {
        let client = Arc::new(ScriptedLlmClient::new(vec![]));
        let report = LlmReportWriter::new(client.clone())
            .write_daily(&request(vec![]))
            .await
            .unwrap();
        assert!(report.items.is_empty());
        assert_eq!(report.total_secs, 0);
        assert!(report.summary_md.contains("Sem atividade registrada."));
        assert!(client.requests().is_empty());
    }
}
