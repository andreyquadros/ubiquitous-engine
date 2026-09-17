//! Weekly recommendations: [`LlmAdvisor`] implements [`Advisor`] from aggregated numbers only.

use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};
use tracing::{info, warn};
use ubiqx_core::ports::{Advice, AdviceRequest, Advisor, AiUsageKind};
use ubiqx_core::redact::redact_text;
use ubiqx_core::{AiModels, CoreError, CoreResult};

use crate::classifier::model_or_default;
use crate::client::{
    prompt_cache_for, thinking_disabled_for, LlmClient, LlmMessage, LlmRequest, SystemBlock,
};
use crate::prompts;

/// Output budget of one advice answer.
pub const MAX_TOKENS: u32 = 1000;
/// Recommendations kept from the answer.
pub const MAX_RECOMMENDATIONS: usize = 5;

/// JSON schema of the answer.
pub fn advice_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "headline": {"type": "string"},
            "recommendations": {"type": "array", "items": {"type": "string"}}
        },
        "required": ["headline", "recommendations"],
        "additionalProperties": false
    })
}

/// Advisor backed by the Messages API.
pub struct LlmAdvisor {
    client: Arc<dyn LlmClient>,
}

impl LlmAdvisor {
    pub fn new(client: Arc<dyn LlmClient>) -> Self {
        Self { client }
    }

    /// The request for `req` (model from `req.model`, cached system prompt on Sonnet/Opus,
    /// thinking disabled).
    pub fn build_request(req: &AdviceRequest) -> LlmRequest {
        let model = model_or_default(&req.model, &AiModels::default().report);
        LlmRequest {
            system: vec![SystemBlock::new(
                prompts::advisor_system_prompt(req),
                prompt_cache_for(&model),
            )],
            messages: vec![LlmMessage::user_text(prompts::advisor_user_message(req))],
            max_tokens: MAX_TOKENS,
            json_schema: Some(advice_schema()),
            disable_thinking: thinking_disabled_for(&model),
            usage_kind: AiUsageKind::Advice,
            timeout: None,
            model,
        }
    }
}

#[async_trait]
impl Advisor for LlmAdvisor {
    async fn advise(&self, req: &AdviceRequest) -> CoreResult<Advice> {
        let mut llm_req = Self::build_request(req);
        let mut resp = self.client.complete(&llm_req).await?;
        if resp.is_truncated() {
            llm_req.max_tokens = llm_req.max_tokens.saturating_mul(2);
            warn!(
                max_tokens = llm_req.max_tokens,
                "advisor: answer truncated, retrying once"
            );
            resp = self.client.complete(&llm_req).await?;
            if resp.is_truncated() {
                return Err(CoreError::Ai("advice output truncated twice".into()));
            }
        }
        let advice = parse_advice(&resp.text, &llm_req.model)?;
        info!(
            recommendations = advice.recommendations.len(),
            "advisor: done"
        );
        Ok(advice)
    }
}

#[derive(Debug, Deserialize)]
struct RawAdvice {
    #[serde(default)]
    headline: String,
    #[serde(default)]
    recommendations: Vec<String>,
}

fn tidy(text: &str) -> String {
    redact_text(text)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Validates the answer: at most [`MAX_RECOMMENDATIONS`] non-empty recommendations, masked.
pub fn parse_advice(text: &str, model: &str) -> CoreResult<Advice> {
    let raw: RawAdvice = serde_json::from_str(text.trim())
        .map_err(|e| CoreError::Ai(format!("advisor returned invalid JSON: {e}")))?;
    let headline = tidy(&raw.headline);
    if headline.is_empty() {
        return Err(CoreError::Ai("advisor returned an empty headline".into()));
    }
    Ok(Advice {
        headline,
        recommendations: raw
            .recommendations
            .iter()
            .map(|r| tidy(r))
            .filter(|r| !r.is_empty())
            .take(MAX_RECOMMENDATIONS)
            .collect(),
        model: model.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::ScriptedLlmClient;
    use ubiqx_core::{FocusStats, Mood};

    fn request(model: &str) -> AdviceRequest {
        AdviceRequest {
            language: "pt-BR".into(),
            stats: FocusStats {
                focus_score: 60,
                productive_secs: 3600,
                distraction_secs: 600,
                uncategorized_secs: 0,
                idle_secs: 0,
                total_secs: 4200,
                switches_per_hour: 8.0,
                longest_focus_secs: 1800,
                mood: Mood::Calm,
            },
            category_totals: vec![],
            top_apps: vec![],
            recent_nudges: vec![],
            user_profile: None,
            model: model.into(),
        }
    }

    #[test]
    fn request_shape() {
        let req = LlmAdvisor::build_request(&request("claude-sonnet-5"));
        assert_eq!(req.model, "claude-sonnet-5");
        assert!(req.disable_thinking);
        assert!(req.system[0].cache);
        assert_eq!(req.usage_kind, AiUsageKind::Advice);
        assert_eq!(req.max_tokens, MAX_TOKENS);
        let req = LlmAdvisor::build_request(&request(""));
        assert_eq!(req.model, "claude-sonnet-5");
    }

    #[test]
    fn parsing_caps_recommendations() {
        let text = json!({"headline": " Semana  focada ", "recommendations": ["a", "", "b", "c", "d", "e", "f"]}).to_string();
        let advice = parse_advice(&text, "m").unwrap();
        assert_eq!(advice.headline, "Semana focada");
        assert_eq!(advice.recommendations, vec!["a", "b", "c", "d", "e"]);
        assert_eq!(advice.model, "m");
        assert!(matches!(
            parse_advice(r#"{"headline":"","recommendations":[]}"#, "m"),
            Err(CoreError::Ai(_))
        ));
        assert!(matches!(parse_advice("x", "m"), Err(CoreError::Ai(_))));
    }

    #[tokio::test]
    async fn end_to_end() {
        let client = Arc::new(ScriptedLlmClient::new(vec![
            r#"{"headline":"Boa semana","recommendations":["Reserve blocos de foco de 45 min."]}"#
                .into(),
        ]));
        let advice = LlmAdvisor::new(client)
            .advise(&request("claude-sonnet-5"))
            .await
            .unwrap();
        assert_eq!(advice.headline, "Boa semana");
        assert_eq!(advice.recommendations.len(), 1);
        assert_eq!(advice.model, "claude-sonnet-5");
    }
}
