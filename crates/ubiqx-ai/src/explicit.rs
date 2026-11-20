//! The screen check of the `nofap` module: [`LlmExplicitDetector`] implements
//! [`ExplicitDetector`].
//!
//! One request carries one reduced image and two constant strings
//! ([`prompts::EXPLICIT_SYSTEM_PROMPT`], [`prompts::EXPLICIT_USER_TEXT`]): no URL, title, app
//! name, language, profile or identifier. The answer is a class and a confidence from a
//! closed schema without a free-text field, so the model has nowhere to describe what it
//! saw.
//!
//! The answer is read strictly ([`parse_explicit`]): anything that is not exactly the
//! schema's object is "unsure, low", which the sampler treats as "keep watching". A refusal
//! is reported as such and is never a positive. Nothing here writes a log line, and the
//! clients write none for [`AiUsageKind::NofapVision`] either (see [`crate::client`]).
//!
//! There is no provider-specific code: the detector is given the routing client, which
//! turns the image into the Messages API's image block or into an `image_url` data URL.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use base64::Engine;
use serde::Deserialize;
use serde_json::{json, Value};
use ubiqx_core::nofap::{Confidence, ExplicitCheck, ExplicitDetector, ExplicitVerdict};
use ubiqx_core::ports::{AiUsageKind, EncodedImage};
use ubiqx_core::{CoreError, CoreResult};

use crate::client::{
    thinking_disabled_for, LlmClient, LlmContent, LlmMessage, LlmRequest, LlmResponse, SystemBlock,
};
use crate::prompts;
use crate::vision::SUPPORTED_MEDIA_TYPES;

/// Output budget of one answer: the object is about fifteen tokens.
pub const MAX_TOKENS: u32 = 64;
/// Output budget of the single retry after a truncated answer (a model that spends output
/// tokens on reasoning before it answers).
pub const RETRY_MAX_TOKENS: u32 = 256;
/// Total timeout of one HTTP request. The caller bounds the whole check on top of it,
/// because the retry policy belongs to the client, not to the request.
pub const TIMEOUT: Duration = Duration::from_secs(10);

/// What an answer that cannot be trusted is read as: no action, keep watching.
pub const UNSURE: ExplicitCheck = ExplicitCheck {
    verdict: ExplicitVerdict::Unsure,
    confidence: Confidence::Low,
    refused: false,
};

/// What a provider's refusal to look is read as. A refusal alone is never a positive.
pub const REFUSED: ExplicitCheck = ExplicitCheck {
    verdict: ExplicitVerdict::Unsure,
    confidence: Confidence::Low,
    refused: true,
};

/// JSON schema of the answer: closed, both fields required, no free text.
pub fn explicit_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "verdict": {"type": "string", "enum": ["explicit", "suggestive", "none", "unsure"]},
            "confidence": {"type": "string", "enum": ["high", "medium", "low"]}
        },
        "required": ["verdict", "confidence"],
        "additionalProperties": false
    })
}

/// The request for one image with `model` (the vision model of the provider in settings).
///
/// [`CoreError::Invalid`] for an empty image, a media type the vendors do not accept or a
/// blank model id; nothing is sent in those cases.
pub fn build_request(image: &EncodedImage, model: &str) -> CoreResult<LlmRequest> {
    let model = model.trim();
    if model.is_empty() {
        return Err(CoreError::Invalid("no vision model is set".into()));
    }
    let media_type = image.mime.trim().to_ascii_lowercase();
    if !SUPPORTED_MEDIA_TYPES.contains(&media_type.as_str()) {
        return Err(CoreError::Invalid(format!(
            "unsupported image media type: {}",
            image.mime
        )));
    }
    if image.bytes.is_empty() {
        return Err(CoreError::Invalid("empty image".into()));
    }
    Ok(LlmRequest {
        model: model.to_string(),
        system: vec![SystemBlock::new(prompts::EXPLICIT_SYSTEM_PROMPT, false)],
        messages: vec![LlmMessage::user(vec![
            LlmContent::Image {
                media_type,
                base64: base64::engine::general_purpose::STANDARD.encode(&image.bytes),
            },
            LlmContent::Text(prompts::EXPLICIT_USER_TEXT.to_string()),
        ])],
        max_tokens: MAX_TOKENS,
        json_schema: Some(explicit_schema()),
        disable_thinking: thinking_disabled_for(model),
        usage_kind: AiUsageKind::NofapVision,
        timeout: Some(TIMEOUT),
    })
}

/// The answer's object, exactly as the schema defines it: an unknown field, a missing one,
/// a value outside either list or a second document after the first fails the parse.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawExplicit {
    verdict: ExplicitVerdict,
    confidence: Confidence,
}

/// Reads an answer. Anything but the schema's object (invalid JSON, an unknown verdict or
/// confidence, an extra field, text around the object) is [`UNSURE`]: an answer that
/// cannot be read exactly is never acted on.
pub fn parse_explicit(text: &str) -> ExplicitCheck {
    match serde_json::from_str::<RawExplicit>(text.trim()) {
        Ok(raw) => ExplicitCheck {
            verdict: raw.verdict,
            confidence: raw.confidence,
            refused: false,
        },
        Err(_) => UNSURE,
    }
}

/// Screen check backed by an [`LlmClient`] (in the apps, the routing client, so the check
/// follows the provider selected in settings).
pub struct LlmExplicitDetector {
    client: Arc<dyn LlmClient>,
}

impl LlmExplicitDetector {
    pub fn new(client: Arc<dyn LlmClient>) -> Self {
        Self { client }
    }

    /// One call, with [`CoreError::AiRefused`] read as "the provider refused to look"
    /// (`None`).
    async fn ask(&self, req: &LlmRequest) -> CoreResult<Option<LlmResponse>> {
        match self.client.complete(req).await {
            Ok(resp) => Ok(Some(resp)),
            Err(CoreError::AiRefused) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

#[async_trait]
impl ExplicitDetector for LlmExplicitDetector {
    /// A truncated answer is asked for once more with [`RETRY_MAX_TOKENS`]; a second
    /// truncation is an error (the model cannot answer within the budget, which the
    /// caller's health tracking has to see), not a verdict. Every other failure of the
    /// provider is returned as it is.
    async fn check(&self, image: &EncodedImage, model: &str) -> CoreResult<ExplicitCheck> {
        let mut req = build_request(image, model)?;
        let Some(mut resp) = self.ask(&req).await? else {
            return Ok(REFUSED);
        };
        if resp.is_truncated() {
            req.max_tokens = RETRY_MAX_TOKENS;
            resp = match self.ask(&req).await? {
                Some(resp) => resp,
                None => return Ok(REFUSED),
            };
            if resp.is_truncated() {
                return Err(CoreError::Ai("screen check answer truncated twice".into()));
            }
        }
        Ok(parse_explicit(&resp.text))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::StopReason;
    use crate::fake::ScriptedLlmClient;

    fn image() -> EncodedImage {
        EncodedImage {
            bytes: vec![1, 2, 3, 4],
            mime: "image/jpeg".into(),
            width: 640,
            height: 400,
        }
    }

    fn check(verdict: ExplicitVerdict, confidence: Confidence) -> ExplicitCheck {
        ExplicitCheck {
            verdict,
            confidence,
            refused: false,
        }
    }

    /// Every string anywhere in a request, with the place it was found.
    fn texts_of(req: &LlmRequest) -> Vec<(&'static str, String)> {
        let mut out: Vec<(&'static str, String)> = req
            .system
            .iter()
            .map(|b| ("system", b.text.clone()))
            .collect();
        for message in &req.messages {
            for content in &message.content {
                if let LlmContent::Text(text) = content {
                    out.push(("user", text.clone()));
                }
            }
        }
        out
    }

    #[test]
    fn request_is_one_image_then_the_constant_text() {
        let req = build_request(&image(), " claude-haiku-4-5 ").unwrap();
        assert_eq!(req.model, "claude-haiku-4-5");
        assert_eq!(req.usage_kind, AiUsageKind::NofapVision);
        assert_eq!(req.max_tokens, 64);
        assert_eq!(req.timeout, Some(Duration::from_secs(10)));
        assert_eq!(req.json_schema, Some(explicit_schema()));
        assert!(!req.disable_thinking, "Haiku must not get the field");
        assert!(
            build_request(&image(), "claude-sonnet-5")
                .unwrap()
                .disable_thinking
        );

        assert_eq!(req.messages.len(), 1);
        assert_eq!(req.messages[0].role, crate::client::LlmRole::User);
        assert_eq!(
            req.messages[0].content,
            vec![
                LlmContent::Image {
                    media_type: "image/jpeg".into(),
                    base64: "AQIDBA==".into(),
                },
                LlmContent::Text("Classify this screenshot.".into()),
            ]
        );
        assert_eq!(
            req.system,
            vec![SystemBlock::new(prompts::EXPLICIT_SYSTEM_PROMPT, false)]
        );
    }

    #[test]
    fn the_only_text_parts_are_the_two_constants() {
        // Whatever the image and the model, the text of the request is the same two
        // constants: nothing about the person, the page or the app can be in it.
        let other = EncodedImage {
            bytes: vec![9; 2048],
            mime: "IMAGE/PNG".into(),
            width: 320,
            height: 200,
        };
        for req in [
            build_request(&image(), "claude-haiku-4-5").unwrap(),
            build_request(&other, "gpt-5-mini").unwrap(),
            build_request(&other, "ubi-fast").unwrap(),
        ] {
            assert_eq!(
                texts_of(&req),
                vec![
                    ("system", prompts::EXPLICIT_SYSTEM_PROMPT.to_string()),
                    ("user", prompts::EXPLICIT_USER_TEXT.to_string()),
                ]
            );
        }
        // The constants themselves hold no placeholder to fill in.
        for text in [prompts::EXPLICIT_SYSTEM_PROMPT, prompts::EXPLICIT_USER_TEXT] {
            assert!(!text.contains('{') && !text.contains('}'), "{text}");
            assert!(!text.contains("http") && !text.contains('<'), "{text}");
        }
    }

    #[test]
    fn a_request_that_cannot_be_sent_is_refused_before_any_call() {
        let bmp = EncodedImage {
            mime: "image/bmp".into(),
            ..image()
        };
        assert!(matches!(
            build_request(&bmp, "claude-haiku-4-5"),
            Err(CoreError::Invalid(_))
        ));
        let empty = EncodedImage {
            bytes: Vec::new(),
            ..image()
        };
        assert!(matches!(
            build_request(&empty, "claude-haiku-4-5"),
            Err(CoreError::Invalid(_))
        ));
        assert!(matches!(
            build_request(&image(), "  "),
            Err(CoreError::Invalid(_))
        ));
    }

    #[test]
    fn schema_is_closed_and_has_no_free_text() {
        let schema = explicit_schema();
        assert_eq!(schema["type"], "object");
        assert_eq!(schema["additionalProperties"], false);
        assert_eq!(schema["required"], json!(["verdict", "confidence"]));
        let properties = schema["properties"].as_object().unwrap();
        assert_eq!(properties.len(), 2);
        for (name, property) in properties {
            assert_eq!(property["type"], "string", "{name}");
            assert!(
                property["enum"].as_array().is_some_and(|e| !e.is_empty()),
                "{name} is a closed list, not free text"
            );
        }
        assert_eq!(
            schema["properties"]["verdict"]["enum"],
            json!(["explicit", "suggestive", "none", "unsure"])
        );
        assert_eq!(
            schema["properties"]["confidence"]["enum"],
            json!(["high", "medium", "low"])
        );
    }

    #[test]
    fn the_prompt_and_the_schema_are_the_design_documents() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/nofap/DESIGN.md");
        let design = std::fs::read_to_string(path)
            .expect("docs/nofap/DESIGN.md")
            .replace("\r\n", "\n");
        let fenced_after = |marker: &str| -> String {
            let from = design.find(marker).expect(marker);
            let rest = &design[from..];
            let open = rest.find("```").expect("opening fence");
            let body = &rest[open..];
            let start = body.find('\n').expect("fence line") + 1;
            let end = body[start..].find("\n```").expect("closing fence");
            body[start..start + end].to_string()
        };
        assert_eq!(
            prompts::EXPLICIT_SYSTEM_PROMPT,
            fenced_after("System prompt (`ubiqx_ai::prompts::EXPLICIT_SYSTEM_PROMPT`")
        );
        let schema: Value =
            serde_json::from_str(&fenced_after("Schema (closed, all required")).unwrap();
        assert_eq!(explicit_schema(), schema);
        assert!(design.contains(&format!(
            "`LlmContent::Text(\"{}\")`",
            prompts::EXPLICIT_USER_TEXT
        )));
    }

    #[test]
    fn every_verdict_and_confidence_parses() {
        use Confidence::{High, Low, Medium};
        use ExplicitVerdict::{Explicit, None, Suggestive, Unsure};
        for (verdict_text, verdict) in [
            ("explicit", Explicit),
            ("suggestive", Suggestive),
            ("none", None),
            ("unsure", Unsure),
        ] {
            for (confidence_text, confidence) in [("high", High), ("medium", Medium), ("low", Low)]
            {
                let text =
                    format!(r#"{{"verdict":"{verdict_text}","confidence":"{confidence_text}"}}"#);
                assert_eq!(parse_explicit(&text), check(verdict, confidence), "{text}");
            }
        }
        // Key order and surrounding whitespace do not matter.
        assert_eq!(
            parse_explicit("\n {\"confidence\": \"medium\", \"verdict\": \"explicit\"}\n"),
            check(Explicit, Medium)
        );
    }

    #[test]
    fn anything_else_is_unsure_and_never_a_positive() {
        for text in [
            "",
            "   ",
            "not json",
            "explicit",
            "null",
            "[]",
            r#""explicit""#,
            r#"{}"#,
            r#"{"verdict":"explicit"}"#,
            r#"{"confidence":"high"}"#,
            r#"{"verdict":"porn","confidence":"high"}"#,
            r#"{"verdict":"Explicit","confidence":"high"}"#,
            r#"{"verdict":"explicit ","confidence":"high"}"#,
            r#"{"verdict":"explicit","confidence":"very high"}"#,
            r#"{"verdict":"explicit","confidence":"HIGH"}"#,
            r#"{"verdict":"explicit","confidence":0.99}"#,
            r#"{"verdict":["explicit"],"confidence":"high"}"#,
            r#"{"verdict":"explicit","confidence":"high","description":"a page"}"#,
            r#"{"verdict":"explicit","confidence":"high"} trailing"#,
            r#"{"verdict":"explicit","confidence":"high"}{"verdict":"none","confidence":"high"}"#,
            r#"Here you go: {"verdict":"explicit","confidence":"high"}"#,
            "```json\n{\"verdict\":\"explicit\",\"confidence\":\"high\"}\n```",
            r#"{"verdict":"explicit","confidence":"high""#,
            r#"{"verdict":"explicit","verdict":"none","confidence":"high"}"#,
        ] {
            assert_eq!(parse_explicit(text), UNSURE, "{text:?}");
        }
        assert_eq!(
            UNSURE,
            ExplicitCheck {
                verdict: ExplicitVerdict::Unsure,
                confidence: Confidence::Low,
                refused: false,
            }
        );
    }

    fn detector(client: &Arc<ScriptedLlmClient>) -> LlmExplicitDetector {
        LlmExplicitDetector::new(client.clone())
    }

    fn answer(text: &str, stop_reason: StopReason) -> LlmResponse {
        LlmResponse {
            text: text.into(),
            model: "claude-haiku-4-5".into(),
            input_tokens: 600,
            output_tokens: 15,
            cache_read_tokens: 0,
            cache_write_tokens: 0,
            stop_reason,
        }
    }

    const HIGH: &str = r#"{"verdict":"explicit","confidence":"high"}"#;

    #[tokio::test]
    async fn one_call_one_answer() {
        let client = Arc::new(ScriptedLlmClient::new(vec![HIGH.into()]));
        let out = detector(&client)
            .check(&image(), "claude-haiku-4-5")
            .await
            .unwrap();
        assert_eq!(
            out,
            check(ExplicitVerdict::Explicit, Confidence::High),
            "a readable answer is passed on as it is"
        );
        let requests = client.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].usage_kind, AiUsageKind::NofapVision);
        assert_eq!(requests[0].max_tokens, MAX_TOKENS);
    }

    #[tokio::test]
    async fn a_refusal_is_reported_as_one_and_is_never_a_positive() {
        let client = Arc::new(ScriptedLlmClient::new(vec![]));
        client.push_error(CoreError::AiRefused);
        let out = detector(&client)
            .check(&image(), "claude-haiku-4-5")
            .await
            .unwrap();
        assert_eq!(out, REFUSED);
        assert!(out.refused);
        assert_eq!(out.verdict, ExplicitVerdict::Unsure);
        assert_eq!(out.confidence, Confidence::Low);
        assert_eq!(client.requests().len(), 1, "a refusal is not retried");

        // Even when the refused answer carries a verdict, the stop reason wins: a client
        // turns `stop_reason = refusal` into `AiRefused` whatever the text says.
        let client = Arc::new(ScriptedLlmClient::new(vec![]));
        client.push_response(answer(HIGH, StopReason::MaxTokens));
        client.push_error(CoreError::AiRefused);
        let out = detector(&client)
            .check(&image(), "claude-haiku-4-5")
            .await
            .unwrap();
        assert_eq!(out, REFUSED, "a refusal on the retry is a refusal too");
    }

    #[tokio::test]
    async fn a_truncated_answer_is_retried_once_with_a_larger_budget() {
        let client = Arc::new(ScriptedLlmClient::new(vec![
            r#"{"verdict":"expl"#.into(),
            r#"{"verdict":"suggestive","confidence":"medium"}"#.into(),
        ]));
        client.truncate_response(0);
        let out = detector(&client)
            .check(&image(), "gpt-5-mini")
            .await
            .unwrap();
        assert_eq!(out, check(ExplicitVerdict::Suggestive, Confidence::Medium));
        let requests = client.requests();
        assert_eq!(
            requests.iter().map(|r| r.max_tokens).collect::<Vec<_>>(),
            [64, 256]
        );
        assert_eq!(
            requests[0].messages, requests[1].messages,
            "the retry sends the same content"
        );
        assert_eq!(requests[1].usage_kind, AiUsageKind::NofapVision);

        // A complete-looking answer that was cut off is not trusted either.
        let client = Arc::new(ScriptedLlmClient::new(vec![HIGH.into(), "{}".into()]));
        client.truncate_response(0);
        let out = detector(&client)
            .check(&image(), "gpt-5-mini")
            .await
            .unwrap();
        assert_eq!(
            out, UNSURE,
            "the first answer is dropped, the retry is read"
        );
    }

    #[tokio::test]
    async fn two_truncations_are_an_error_not_a_verdict() {
        let client = Arc::new(ScriptedLlmClient::new(vec![HIGH.into(), HIGH.into()]));
        client.truncate_response(0);
        client.truncate_response(1);
        let err = detector(&client)
            .check(&image(), "o4-mini")
            .await
            .unwrap_err();
        assert!(matches!(err, CoreError::Ai(_)), "{err}");
        assert!(err.is_transient());
        assert!(!err.to_string().contains("nofap"), "{err}");
        assert_eq!(client.requests().len(), 2, "one retry, not more");
    }

    #[tokio::test]
    async fn an_unreadable_answer_is_unsure_without_a_second_call() {
        let client = Arc::new(ScriptedLlmClient::new(vec![
            "I cannot classify this image.".into(),
        ]));
        let out = detector(&client)
            .check(&image(), "claude-haiku-4-5")
            .await
            .unwrap();
        assert_eq!(out, UNSURE);
        assert_eq!(client.requests().len(), 1);
    }

    #[tokio::test]
    async fn provider_failures_are_returned_as_they_are() {
        for error in [
            CoreError::AiNotConfigured,
            CoreError::RateLimited {
                retry_after_secs: 30,
            },
            CoreError::AiRejected("no credits".into()),
            CoreError::Ai("HTTP 503".into()),
        ] {
            let expected = error.to_string();
            let client = Arc::new(ScriptedLlmClient::new(vec![]));
            client.push_error(error);
            let err = detector(&client)
                .check(&image(), "claude-haiku-4-5")
                .await
                .unwrap_err();
            assert_eq!(err.to_string(), expected);
            assert_eq!(client.requests().len(), 1);
        }
        // A request that cannot be built reaches no client.
        let client = Arc::new(ScriptedLlmClient::new(vec![HIGH.into()]));
        assert!(matches!(
            detector(&client).check(&image(), "").await,
            Err(CoreError::Invalid(_))
        ));
        assert!(client.requests().is_empty());
    }
}
