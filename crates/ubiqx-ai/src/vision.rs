//! Single-screenshot classification: [`LlmVisionClassifier`] implements [`VisionClassifier`].
//!
//! The request carries one image plus the redacted block line. The model returns a topic-only
//! description, the kind of on-screen evidence and a coarse confidence; `low` confidence is
//! mapped to "unknown" so the block goes to review instead of getting a guess.

use std::sync::Arc;

use async_trait::async_trait;
use base64::Engine;
use serde::Deserialize;
use serde_json::{json, Value};
use tracing::{debug, warn};
use ubiqx_core::ports::{
    AiUsageKind, Classification, ClassificationContext, EncodedImage, VisionClassifier,
};
use ubiqx_core::{ActivityBlock, AiModels, ClassificationSource, CoreError, CoreResult};

use crate::classifier::{clean_description, model_or_default};
use crate::client::{
    thinking_disabled_for, LlmClient, LlmContent, LlmMessage, LlmRequest, SystemBlock, LONG_TIMEOUT,
};
use crate::prompts;

/// Output budget of one vision answer.
pub const MAX_TOKENS: u32 = 400;
/// Image media types the API accepts.
pub const SUPPORTED_MEDIA_TYPES: [&str; 4] = ["image/jpeg", "image/png", "image/webp", "image/gif"];
/// Confidence values of the answer, mapped to 0.9 / 0.7 / 0.4.
pub const CONFIDENCE_HIGH: f32 = 0.9;
pub const CONFIDENCE_MEDIUM: f32 = 0.7;
pub const CONFIDENCE_LOW: f32 = 0.4;

/// JSON schema of the answer.
pub fn vision_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "category_id": {"anyOf": [{"type": "string"}, {"type": "null"}]},
            "description": {"type": "string"},
            "evidence": {"type": "string"},
            "confidence": {"type": "string", "enum": ["high", "medium", "low"]}
        },
        "required": ["category_id", "description", "evidence", "confidence"],
        "additionalProperties": false
    })
}

/// Vision classifier backed by the Messages API.
pub struct LlmVisionClassifier {
    client: Arc<dyn LlmClient>,
}

impl LlmVisionClassifier {
    pub fn new(client: Arc<dyn LlmClient>) -> Self {
        Self { client }
    }

    /// The request for one block + screenshot (model from `ctx.models.vision`, 120 s timeout).
    pub fn build_request(
        block: &ActivityBlock,
        image: &EncodedImage,
        ctx: &ClassificationContext,
    ) -> CoreResult<LlmRequest> {
        let media_type = image.mime.trim().to_ascii_lowercase();
        if !SUPPORTED_MEDIA_TYPES.contains(&media_type.as_str()) {
            return Err(CoreError::Invalid(format!(
                "unsupported image media type for vision: {}",
                image.mime
            )));
        }
        if image.bytes.is_empty() {
            return Err(CoreError::Invalid("empty image".into()));
        }
        let model = model_or_default(&ctx.models.vision, &AiModels::default().vision);
        Ok(LlmRequest {
            system: vec![SystemBlock::new(prompts::vision_system_prompt(ctx), false)],
            messages: vec![LlmMessage::user(vec![
                LlmContent::Image {
                    media_type,
                    base64: base64::engine::general_purpose::STANDARD.encode(&image.bytes),
                },
                LlmContent::Text(prompts::vision_user_text(block)),
            ])],
            max_tokens: MAX_TOKENS,
            json_schema: Some(vision_schema()),
            disable_thinking: thinking_disabled_for(&model),
            usage_kind: AiUsageKind::Vision,
            timeout: Some(LONG_TIMEOUT),
            model,
        })
    }
}

#[async_trait]
impl VisionClassifier for LlmVisionClassifier {
    async fn classify_with_image(
        &self,
        block: &ActivityBlock,
        image: &EncodedImage,
        ctx: &ClassificationContext,
    ) -> CoreResult<Classification> {
        let mut req = Self::build_request(block, image, ctx)?;
        let mut resp = self.client.complete(&req).await?;
        if resp.is_truncated() {
            req.max_tokens = req.max_tokens.saturating_mul(2);
            warn!(block_id = %block.id, max_tokens = req.max_tokens, "vision: answer truncated, retrying once");
            resp = self.client.complete(&req).await?;
            if resp.is_truncated() {
                return Err(CoreError::Ai("vision output truncated twice".into()));
            }
        }
        parse_vision(&resp.text, &block.id, ctx)
    }
}

#[derive(Debug, Deserialize)]
struct RawVision {
    #[serde(default)]
    category_id: Option<String>,
    #[serde(default)]
    description: String,
    #[serde(default)]
    evidence: String,
    #[serde(default)]
    confidence: String,
}

/// Validates a vision answer: unknown categories → `None`, `low` confidence → `None`,
/// confidence mapped to [`CONFIDENCE_HIGH`] / [`CONFIDENCE_MEDIUM`] / [`CONFIDENCE_LOW`].
pub fn parse_vision(
    text: &str,
    block_id: &str,
    ctx: &ClassificationContext,
) -> CoreResult<Classification> {
    let raw: RawVision = serde_json::from_str(text.trim())
        .map_err(|e| CoreError::Ai(format!("vision returned invalid JSON: {e}")))?;
    let allowed = prompts::allowed_category_ids(&ctx.categories);
    let confidence = match raw.confidence.trim().to_ascii_lowercase().as_str() {
        "high" => CONFIDENCE_HIGH,
        "medium" => CONFIDENCE_MEDIUM,
        _ => CONFIDENCE_LOW,
    };
    let category_id = raw
        .category_id
        .map(|c| c.trim().to_string())
        .filter(|c| !c.is_empty() && allowed.contains(c))
        .filter(|_| confidence > CONFIDENCE_LOW);
    debug!(block_id, evidence = %raw.evidence.trim(), confidence, "vision: answer");
    Ok(Classification {
        block_id: block_id.to_string(),
        category_id,
        confidence,
        source: ClassificationSource::Vision,
        description: clean_description(Some(raw.description)),
        needs_vision: false,
        rule_id: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::ScriptedLlmClient;
    use crate::prompts::test_support::{block, category, ctx};

    fn image(mime: &str) -> EncodedImage {
        EncodedImage {
            bytes: vec![1, 2, 3, 4],
            mime: mime.into(),
            width: 4,
            height: 1,
        }
    }

    #[test]
    fn request_has_image_then_text() {
        let c = ctx(vec![category("cat-ifro", "IFRO")]);
        let b = block("b1", "net.whatsapp.WhatsApp", "WhatsApp", "Maria", None);
        let req = LlmVisionClassifier::build_request(&b, &image("image/JPEG"), &c).unwrap();
        assert_eq!(req.model, "claude-haiku-4-5");
        assert_eq!(req.timeout, Some(LONG_TIMEOUT));
        assert_eq!(req.usage_kind, AiUsageKind::Vision);
        match &req.messages[0].content[0] {
            LlmContent::Image { media_type, base64 } => {
                assert_eq!(media_type, "image/jpeg");
                assert_eq!(base64, "AQIDBA==");
            }
            other => panic!("{other:?}"),
        }
        match &req.messages[0].content[1] {
            LlmContent::Text(t) => {
                assert!(t.contains("title=WhatsApp"));
                assert!(!t.contains("Maria"));
            }
            other => panic!("{other:?}"),
        }
        assert!(matches!(
            LlmVisionClassifier::build_request(&b, &image("image/bmp"), &c),
            Err(CoreError::Invalid(_))
        ));
    }

    #[test]
    fn confidence_mapping_and_validation() {
        let c = ctx(vec![category("cat-ifro", "IFRO")]);
        let high = parse_vision(r#"{"category_id":"cat-ifro","description":"Editou o edital","evidence":"document title bar","confidence":"high"}"#, "b1", &c).unwrap();
        assert_eq!(high.category_id.as_deref(), Some("cat-ifro"));
        assert_eq!(high.confidence, 0.9);
        assert_eq!(high.source, ClassificationSource::Vision);
        assert_eq!(high.description.as_deref(), Some("Editou o edital"));
        let medium = parse_vision(
            r#"{"category_id":"cat-ifro","description":"","evidence":"","confidence":"medium"}"#,
            "b1",
            &c,
        )
        .unwrap();
        assert_eq!(medium.confidence, 0.7);
        assert_eq!(medium.description, None);
        let low = parse_vision(
            r#"{"category_id":"cat-ifro","description":"x","evidence":"","confidence":"low"}"#,
            "b1",
            &c,
        )
        .unwrap();
        assert_eq!(
            low.category_id, None,
            "low confidence never assigns a category"
        );
        assert_eq!(low.confidence, 0.4);
        let unknown = parse_vision(
            r#"{"category_id":"nope","description":"x","evidence":"","confidence":"high"}"#,
            "b1",
            &c,
        )
        .unwrap();
        assert_eq!(unknown.category_id, None);
        assert!(matches!(
            parse_vision("nope", "b1", &c),
            Err(CoreError::Ai(_))
        ));
    }

    #[tokio::test]
    async fn end_to_end_with_scripted_client() {
        let c = ctx(vec![category("cat-ifro", "IFRO")]);
        let client = Arc::new(ScriptedLlmClient::new(vec![
            r#"{"category_id":"cat-ifro","description":"Revisou um edital","evidence":"pdf viewer","confidence":"high"}"#.into(),
        ]));
        let b = block("b1", "com.apple.Preview", "Preview", "edital.pdf", None);
        let out = LlmVisionClassifier::new(client.clone())
            .classify_with_image(&b, &image("image/png"), &c)
            .await
            .unwrap();
        assert_eq!(out.block_id, "b1");
        assert_eq!(out.category_id.as_deref(), Some("cat-ifro"));
        assert_eq!(client.requests().len(), 1);
    }
}
