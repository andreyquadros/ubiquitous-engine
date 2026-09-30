//! Batched text classification: [`LlmTextClassifier`] implements
//! [`RemoteClassifier`] on top of an [`LlmClient`].
//!
//! Blocks are sent in chunks of at most [`MAX_BATCH`] redacted lines with a structured-output
//! schema; the answer is validated strictly (unknown block ids and duplicate ids are dropped,
//! unknown category ids become `None`, confidence is clamped). Blocks the model left out are
//! simply absent from the result — the engine treats them as unknown.

use std::collections::HashSet;
use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};
use tracing::{debug, warn};
use ubiqx_core::ports::{AiUsageKind, Classification, ClassificationContext, RemoteClassifier};
use ubiqx_core::redact::redact_text;
use ubiqx_core::{ActivityBlock, AiModels, ClassificationSource, CoreError, CoreResult};

use crate::client::{thinking_disabled_for, LlmClient, LlmMessage, LlmRequest, SystemBlock};
use crate::prompts;

/// Largest number of blocks in one request.
pub const MAX_BATCH: usize = 25;
/// Output budget per block (`max_tokens = TOKENS_PER_BLOCK * n + TOKENS_BASE`). Block and
/// category ids are UUIDs (≈ 20-25 tokens each), so one result with a pt-BR description is
/// ≈ 100-150 tokens; the ceiling is generous because only generated tokens are billed and a
/// truncated batch is re-sent whole.
pub const TOKENS_PER_BLOCK: u32 = 220;
/// Fixed part of the output budget.
pub const TOKENS_BASE: u32 = 300;

/// JSON schema of the answer (structured outputs: every object closed, all properties required).
pub fn classification_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "results": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "block_id": {"type": "string"},
                        "category_id": {"anyOf": [{"type": "string"}, {"type": "null"}]},
                        "confidence": {"type": "number"},
                        "needs_vision": {"type": "boolean"},
                        "description": {"anyOf": [{"type": "string"}, {"type": "null"}]}
                    },
                    "required": ["block_id", "category_id", "confidence", "needs_vision", "description"],
                    "additionalProperties": false
                }
            }
        },
        "required": ["results"],
        "additionalProperties": false
    })
}

/// `max_tokens` for a batch of `n` blocks.
pub fn max_tokens_for(n: usize) -> u32 {
    TOKENS_PER_BLOCK.saturating_mul(u32::try_from(n).unwrap_or(u32::MAX)) + TOKENS_BASE
}

pub(crate) fn model_or_default(model: &str, default: &str) -> String {
    let m = model.trim();
    if m.is_empty() {
        default.to_string()
    } else {
        m.to_string()
    }
}

/// Text classifier backed by the Messages API.
pub struct LlmTextClassifier {
    client: Arc<dyn LlmClient>,
    batch_size: usize,
}

impl LlmTextClassifier {
    pub fn new(client: Arc<dyn LlmClient>) -> Self {
        Self {
            client,
            batch_size: MAX_BATCH,
        }
    }

    /// Smaller chunks (never larger than [`MAX_BATCH`]).
    pub fn with_batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = batch_size.clamp(1, MAX_BATCH);
        self
    }

    /// The request sent for one chunk (model from `ctx.models.classify`, no cache breakpoint,
    /// no `thinking` field on Haiku).
    pub fn build_request(chunk: &[ActivityBlock], ctx: &ClassificationContext) -> LlmRequest {
        let model = model_or_default(&ctx.models.classify, &AiModels::default().classify);
        LlmRequest {
            system: vec![SystemBlock::new(
                prompts::classification_system_prompt(ctx),
                false,
            )],
            messages: vec![LlmMessage::user_text(prompts::classification_user_message(
                chunk, ctx,
            ))],
            max_tokens: max_tokens_for(chunk.len()),
            json_schema: Some(classification_schema()),
            disable_thinking: thinking_disabled_for(&model),
            usage_kind: AiUsageKind::Classify,
            timeout: None,
            model,
        }
    }

    async fn classify_chunk(
        &self,
        chunk: &[ActivityBlock],
        ctx: &ClassificationContext,
    ) -> CoreResult<Vec<Classification>> {
        let mut req = Self::build_request(chunk, ctx);
        let mut resp = self.client.complete(&req).await?;
        if resp.is_truncated() {
            // One more try with twice the output budget; a second truncation is an error.
            req.max_tokens = req.max_tokens.saturating_mul(2);
            warn!(
                blocks = chunk.len(),
                max_tokens = req.max_tokens,
                "classifier: answer truncated, retrying once"
            );
            resp = self.client.complete(&req).await?;
            if resp.is_truncated() {
                return Err(CoreError::Ai(format!(
                    "classifier output truncated twice for {} blocks",
                    chunk.len()
                )));
            }
        }
        let results = parse_classifications(&resp.text, chunk, ctx)?;
        debug!(
            requested = chunk.len(),
            answered = results.len(),
            classified = results.iter().filter(|c| c.category_id.is_some()).count(),
            "classifier: chunk done"
        );
        Ok(results)
    }
}

#[async_trait]
impl RemoteClassifier for LlmTextClassifier {
    fn name(&self) -> &'static str {
        "llm-text"
    }

    fn describe_payload(&self, block: &ActivityBlock) -> String {
        crate::prompts::render_block_line(block)
    }

    async fn classify_batch(
        &self,
        blocks: &[ActivityBlock],
        ctx: &ClassificationContext,
    ) -> CoreResult<Vec<Classification>> {
        let mut out = Vec::with_capacity(blocks.len());
        for chunk in blocks.chunks(self.batch_size) {
            out.extend(self.classify_chunk(chunk, ctx).await?);
        }
        Ok(out)
    }
}

#[derive(Debug, Deserialize)]
struct RawResults {
    #[serde(default)]
    results: Vec<RawResult>,
}

#[derive(Debug, Deserialize)]
struct RawResult {
    #[serde(default)]
    block_id: String,
    #[serde(default)]
    category_id: Option<String>,
    #[serde(default)]
    confidence: f32,
    #[serde(default)]
    needs_vision: bool,
    #[serde(default)]
    description: Option<String>,
}

/// Cleans a model-written description: masked, whitespace-collapsed, `None` when empty.
pub(crate) fn clean_description(text: Option<String>) -> Option<String> {
    text.map(|d| {
        redact_text(&d)
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    })
    .filter(|d| !d.is_empty())
}

/// Validates the model's answer against the request.
///
/// * results whose `block_id` is not in `blocks` are dropped;
/// * duplicate ids keep the first result;
/// * a `category_id` outside [`prompts::allowed_category_ids`] becomes `None`;
/// * `confidence` is clamped to `0..=1` (NaN → 0).
pub fn parse_classifications(
    text: &str,
    blocks: &[ActivityBlock],
    ctx: &ClassificationContext,
) -> CoreResult<Vec<Classification>> {
    let raw: RawResults = serde_json::from_str(text.trim())
        .map_err(|e| CoreError::Ai(format!("classifier returned invalid JSON: {e}")))?;
    let requested: HashSet<&str> = blocks.iter().map(|b| b.id.as_str()).collect();
    let allowed = prompts::allowed_category_ids(&ctx.categories);
    let mut seen: HashSet<String> = HashSet::with_capacity(raw.results.len());
    let mut out = Vec::with_capacity(raw.results.len());

    for r in raw.results {
        let block_id = r.block_id.trim().to_string();
        if !requested.contains(block_id.as_str()) {
            debug!(block_id, "classifier: dropping result for unknown block id");
            continue;
        }
        if !seen.insert(block_id.clone()) {
            debug!(block_id, "classifier: dropping duplicate result");
            continue;
        }
        let category_id = r
            .category_id
            .map(|c| c.trim().to_string())
            .filter(|c| !c.is_empty())
            .and_then(|c| {
                if allowed.contains(&c) {
                    Some(c)
                } else {
                    debug!(block_id, category_id = %c, "classifier: unknown category id → None");
                    None
                }
            });
        let confidence = if r.confidence.is_nan() {
            0.0
        } else {
            r.confidence.clamp(0.0, 1.0)
        };
        out.push(Classification {
            block_id,
            category_id,
            confidence,
            source: ClassificationSource::Llm,
            description: clean_description(r.description),
            needs_vision: r.needs_vision,
            rule_id: None,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::ScriptedLlmClient;
    use crate::prompts::test_support::{block, category, ctx};
    use ubiqx_core::system_categories;

    fn blocks(n: usize) -> Vec<ActivityBlock> {
        (0..n)
            .map(|i| {
                block(
                    &format!("b{i}"),
                    "com.google.Chrome",
                    "Google Chrome",
                    &format!("Doc {i}"),
                    None,
                )
            })
            .collect()
    }

    #[test]
    fn schema_is_structured_output_compatible() {
        let s = classification_schema();
        assert_eq!(s["additionalProperties"], false);
        assert_eq!(s["required"], json!(["results"]));
        let item = &s["properties"]["results"]["items"];
        assert_eq!(item["additionalProperties"], false);
        assert_eq!(item["required"].as_array().unwrap().len(), 5);
        let text = s.to_string();
        for forbidden in ["minimum", "maximum", "minLength", "maxLength"] {
            assert!(!text.contains(forbidden));
        }
    }

    #[test]
    fn validation_drops_unknown_ids_duplicates_and_bad_categories() {
        let c = ctx(vec![category("cat-ifro", "IFRO")]);
        let bs = blocks(3);
        let answer = json!({"results": [
            {"block_id": "b0", "category_id": "cat-ifro", "confidence": 1.7, "needs_vision": false, "description": "Editou o edital com joao@ifro.edu.br"},
            {"block_id": "b0", "category_id": "cat-ifro", "confidence": 0.2, "needs_vision": true, "description": null},
            {"block_id": "b1", "category_id": "cat-nope", "confidence": 0.8, "needs_vision": false, "description": "  "},
            {"block_id": "ghost", "category_id": "cat-ifro", "confidence": 0.9, "needs_vision": false, "description": null},
            {"block_id": "b2", "category_id": system_categories::DISTRACTION, "confidence": -3.0, "needs_vision": false, "description": null}
        ]});
        let out = parse_classifications(&answer.to_string(), &bs, &c).unwrap();
        assert_eq!(out.len(), 3);
        assert_eq!(out[0].block_id, "b0");
        assert_eq!(out[0].category_id.as_deref(), Some("cat-ifro"));
        assert_eq!(out[0].confidence, 1.0);
        assert!(!out[0].needs_vision, "first result wins");
        assert_eq!(
            out[0].description.as_deref(),
            Some("Editou o edital com [email]")
        );
        assert_eq!(out[0].source, ClassificationSource::Llm);
        assert_eq!(out[0].rule_id, None);
        assert_eq!(out[1].block_id, "b1");
        assert_eq!(out[1].category_id, None);
        assert_eq!(out[1].confidence, 0.8);
        assert_eq!(out[1].description, None);
        assert_eq!(out[2].block_id, "b2");
        assert_eq!(
            out[2].category_id.as_deref(),
            Some(system_categories::DISTRACTION)
        );
        assert_eq!(out[2].confidence, 0.0);
    }

    #[test]
    fn invalid_json_is_an_ai_error() {
        let c = ctx(vec![]);
        assert!(matches!(
            parse_classifications("{\"results\": [", &blocks(1), &c),
            Err(CoreError::Ai(_))
        ));
        assert!(parse_classifications("{}", &blocks(1), &c)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn request_shape() {
        let c = ctx(vec![category("cat-ifro", "IFRO")]);
        let req = LlmTextClassifier::build_request(&blocks(4), &c);
        assert_eq!(req.model, "claude-haiku-4-5");
        assert_eq!(req.max_tokens, max_tokens_for(4));
        assert!(!req.disable_thinking);
        assert!(!req.system[0].cache);
        assert_eq!(req.usage_kind, AiUsageKind::Classify);
        assert!(req.json_schema.is_some());
        let mut c2 = c.clone();
        c2.models.classify = "claude-sonnet-5".into();
        assert!(LlmTextClassifier::build_request(&blocks(1), &c2).disable_thinking);
        c2.models.classify = String::new();
        assert_eq!(
            LlmTextClassifier::build_request(&blocks(1), &c2).model,
            "claude-haiku-4-5"
        );
    }

    #[tokio::test]
    async fn chunks_batches_and_retries_truncation_once() {
        let c = ctx(vec![category("cat-ifro", "IFRO")]);
        let bs = blocks(27);
        let first: Vec<Value> = (0..25).map(|i| json!({"block_id": format!("b{i}"), "category_id": "cat-ifro", "confidence": 0.9, "needs_vision": false, "description": null})).collect();
        let second = json!({"results": [{"block_id": "b25", "category_id": null, "confidence": 0.1, "needs_vision": true, "description": null}]});
        let client = Arc::new(ScriptedLlmClient::new(vec![
            json!({"results": first}).to_string(),
            "{\"results\": [{\"block_id\": \"b25\"".to_string(),
            second.to_string(),
        ]));
        client.truncate_response(1);
        let classifier = LlmTextClassifier::new(client.clone());
        let out = classifier.classify_batch(&bs, &c).await.unwrap();
        assert_eq!(out.len(), 26);
        assert!(out[25].needs_vision);
        let reqs = client.requests();
        assert_eq!(reqs.len(), 3);
        assert_eq!(reqs[0].max_tokens, max_tokens_for(25));
        assert_eq!(reqs[1].max_tokens, max_tokens_for(2));
        assert_eq!(
            reqs[2].max_tokens,
            max_tokens_for(2) * 2,
            "retry doubles the budget"
        );
        assert!(
            reqs[1].messages[0] == reqs[2].messages[0],
            "same prompt on retry"
        );
    }

    #[tokio::test]
    async fn double_truncation_fails() {
        let c = ctx(vec![]);
        let client = Arc::new(ScriptedLlmClient::new(vec!["{".into(), "{".into()]));
        client.truncate_response(0);
        client.truncate_response(1);
        let err = LlmTextClassifier::new(client)
            .classify_batch(&blocks(1), &c)
            .await
            .unwrap_err();
        assert!(matches!(err, CoreError::Ai(msg) if msg.contains("truncated twice")));
    }

    #[tokio::test]
    async fn empty_batch_makes_no_call() {
        let client = Arc::new(ScriptedLlmClient::new(vec![]));
        let out = LlmTextClassifier::new(client.clone())
            .classify_batch(&[], &ctx(vec![]))
            .await
            .unwrap();
        assert!(out.is_empty());
        assert!(client.requests().is_empty());
    }
}
