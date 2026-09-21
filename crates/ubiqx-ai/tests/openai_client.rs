//! HTTP behaviour of `OpenAiCompatClient` (OpenAI and xAI) against a mock server.

use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use ubiqx_ai::client::{
    ApiKeySource, LlmContent, LlmMessage, LlmRequest, StaticApiKey, StopReason, SystemBlock,
};
use ubiqx_ai::fake::MemoryUsageRepo;
use ubiqx_ai::openai::{OpenAiCompatClient, OpenAiCompatConfig};
use ubiqx_core::ports::AiUsageKind;
use ubiqx_core::{AiProvider, CoreError};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

const KEY: &str = "sk-test-key";

struct NoKey;

impl ApiKeySource for NoKey {
    fn api_key(&self) -> Option<String> {
        None
    }
}

fn config(server: &MockServer, provider: AiProvider) -> OpenAiCompatConfig {
    OpenAiCompatConfig {
        provider,
        base_url: format!("{}/v1", server.uri()),
        timeout: Duration::from_secs(5),
        max_attempts: 4,
        backoff_base: Duration::from_millis(10),
        backoff_cap: Duration::from_secs(30),
    }
}

fn client(
    server: &MockServer,
    provider: AiProvider,
    usage: Arc<MemoryUsageRepo>,
) -> OpenAiCompatClient {
    OpenAiCompatClient::new(
        config(server, provider),
        Arc::new(StaticApiKey(KEY.into())),
        usage,
    )
    .unwrap()
}

fn openai(server: &MockServer, usage: Arc<MemoryUsageRepo>) -> OpenAiCompatClient {
    client(server, AiProvider::OpenAi, usage)
}

fn xai(server: &MockServer, usage: Arc<MemoryUsageRepo>) -> OpenAiCompatClient {
    client(server, AiProvider::Xai, usage)
}

fn request(model: &str, kind: AiUsageKind) -> LlmRequest {
    LlmRequest {
        model: model.into(),
        system: vec![
            SystemBlock::new("You classify things.", false),
            SystemBlock::new("Profile.", true),
        ],
        messages: vec![LlmMessage::user(vec![
            LlmContent::Image {
                media_type: "image/png".into(),
                base64: "iVBORw0KGgo=".into(),
            },
            LlmContent::Text("<blocks>\n- id=b1\n</blocks>".into()),
        ])],
        max_tokens: 320,
        json_schema: Some(
            json!({"type": "object", "properties": {"ok": {"type": "boolean"}}, "required": ["ok"], "additionalProperties": false}),
        ),
        disable_thinking: true,
        usage_kind: kind,
        timeout: None,
    }
}

fn ok_body(model: &str, content: Option<&str>, refusal: Option<&str>, finish: &str) -> Value {
    json!({
        "id": "chatcmpl-01",
        "object": "chat.completion",
        "model": model,
        "choices": [{
            "index": 0,
            "message": {"role": "assistant", "content": content, "refusal": refusal},
            "finish_reason": finish,
            "logprobs": null
        }],
        "usage": {
            "prompt_tokens": 1200,
            "completion_tokens": 80,
            "total_tokens": 1280,
            "prompt_tokens_details": {"cached_tokens": 500, "audio_tokens": 0},
            "completion_tokens_details": {"reasoning_tokens": 30}
        }
    })
}

fn error_body(kind: &str, code: Option<&str>, message: &str) -> Value {
    json!({"error": {"message": message, "type": kind, "param": null, "code": code}})
}

fn body_of(req: &Request) -> Value {
    serde_json::from_slice(&req.body).unwrap()
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-12
}

// ---------------------------------------------------------------------------------------------
// Request shape
// ---------------------------------------------------------------------------------------------

#[tokio::test]
async fn openai_classification_body_and_usage() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(header("authorization", format!("Bearer {KEY}").as_str()))
        .and(header("content-type", "application/json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ok_body(
            "gpt-5-mini-2025-08-07",
            Some("{\"ok\":true}"),
            None,
            "stop",
        )))
        .expect(1)
        .mount(&server)
        .await;
    let usage = Arc::new(MemoryUsageRepo::new());
    let c = openai(&server, usage.clone());
    assert_eq!(c.provider(), AiProvider::OpenAi);
    let resp = c
        .complete(&request("gpt-5-mini", AiUsageKind::Classify))
        .await
        .unwrap();

    assert_eq!(resp.text, "{\"ok\":true}");
    assert_eq!(resp.model, "gpt-5-mini-2025-08-07");
    assert_eq!(
        resp.input_tokens, 700,
        "cached tokens are split out of prompt_tokens"
    );
    assert_eq!(resp.output_tokens, 80);
    assert_eq!(resp.cache_read_tokens, 500);
    assert_eq!(resp.cache_write_tokens, 0);
    assert_eq!(resp.stop_reason, StopReason::EndTurn);

    let records = usage.records();
    assert_eq!(records.len(), 1);
    let u = &records[0];
    assert_eq!(u.kind, AiUsageKind::Classify);
    assert_eq!(u.model, "gpt-5-mini");
    assert_eq!(
        (
            u.input_tokens,
            u.output_tokens,
            u.cache_read_tokens,
            u.cache_write_tokens
        ),
        (700, 80, 500, 0)
    );
    let expected = 700.0 / 1e6 * 0.25 + 80.0 / 1e6 * 2.0 + 500.0 / 1e6 * 0.025;
    assert!(close(u.cost_usd, expected), "{} vs {expected}", u.cost_usd);

    let sent = server.received_requests().await.unwrap();
    let body = body_of(&sent[0]);
    assert_eq!(body["model"], "gpt-5-mini");
    assert_eq!(body["max_completion_tokens"], 320);
    assert!(body.get("max_tokens").is_none());
    assert!(body.get("temperature").is_none());
    assert_eq!(body["reasoning_effort"], "minimal");
    assert_eq!(
        body["messages"][0],
        json!({"role": "system", "content": "You classify things.\n\nProfile."})
    );
    assert_eq!(
        body["messages"][1],
        json!({"role": "user", "content": [
            {"type": "image_url", "image_url": {"url": "data:image/png;base64,iVBORw0KGgo="}},
            {"type": "text", "text": "<blocks>\n- id=b1\n</blocks>"}
        ]})
    );
    assert_eq!(
        body["response_format"],
        json!({"type": "json_schema", "json_schema": {
            "name": "ubiqx_output",
            "strict": true,
            "schema": {"type": "object", "properties": {"ok": {"type": "boolean"}}, "required": ["ok"], "additionalProperties": false}
        }})
    );
    assert!(body.get("thinking").is_none());
    assert!(body.get("system").is_none());
    assert_eq!(body["messages"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn openai_report_omits_reasoning_effort_and_honours_timeout_override() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(ok_body("gpt-5", Some("{}"), None, "stop"))
                .set_delay(Duration::from_millis(300)),
        )
        .expect(2)
        .mount(&server)
        .await;
    let usage = Arc::new(MemoryUsageRepo::new());
    let mut req = request("gpt-5", AiUsageKind::Report);
    // Shorter than the server delay: the call must time out (and be retried) instead of
    // waiting for the config's 5 s.
    req.timeout = Some(Duration::from_millis(50));
    let mut cfg = config(&server, AiProvider::OpenAi);
    cfg.max_attempts = 1;
    let c =
        OpenAiCompatClient::new(cfg, Arc::new(StaticApiKey(KEY.into())), usage.clone()).unwrap();
    let started = Instant::now();
    let err = c.complete(&req).await.unwrap_err();
    assert!(
        started.elapsed() < Duration::from_millis(290),
        "{:?}",
        started.elapsed()
    );
    assert!(
        matches!(err, CoreError::Ai(ref m) if m.contains("timed out")),
        "{err:?}"
    );
    assert!(usage.records().is_empty());

    // Same request without the override goes through and carries no reasoning_effort.
    req.timeout = None;
    let resp = openai(&server, usage.clone()).complete(&req).await.unwrap();
    assert_eq!(resp.text, "{}");
    let sent = server.received_requests().await.unwrap();
    let body = body_of(sent.last().unwrap());
    assert!(body.get("reasoning_effort").is_none());
    assert_eq!(body["max_completion_tokens"], 320);
    let u = &usage.records()[0];
    assert_eq!(u.kind, AiUsageKind::Report);
    let expected = 700.0 / 1e6 * 1.25 + 80.0 / 1e6 * 10.0 + 500.0 / 1e6 * 0.125;
    assert!(close(u.cost_usd, expected), "{} vs {expected}", u.cost_usd);
}

#[tokio::test]
async fn xai_body_uses_max_tokens_and_reasoning_effort_only_for_grok_3_mini() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(header("authorization", format!("Bearer {KEY}").as_str()))
        .respond_with(ResponseTemplate::new(200).set_body_json(ok_body(
            "grok-4-1-fast-non-reasoning",
            Some("{\"ok\":false}"),
            None,
            "stop",
        )))
        .expect(2)
        .mount(&server)
        .await;
    let usage = Arc::new(MemoryUsageRepo::new());
    let c = xai(&server, usage.clone());
    assert_eq!(c.provider(), AiProvider::Xai);
    c.complete(&request("grok-4-1-fast-non-reasoning", AiUsageKind::Vision))
        .await
        .unwrap();
    c.complete(&request("grok-3-mini", AiUsageKind::Classify))
        .await
        .unwrap();

    let sent = server.received_requests().await.unwrap();
    let grok4 = body_of(&sent[0]);
    assert_eq!(grok4["max_tokens"], 320);
    assert!(grok4.get("max_completion_tokens").is_none());
    assert!(grok4.get("reasoning_effort").is_none());
    assert!(grok4.get("temperature").is_none());
    assert_eq!(grok4["response_format"]["json_schema"]["strict"], true);
    assert_eq!(grok4["messages"][1]["content"][0]["type"], "image_url");
    let grok3 = body_of(&sent[1]);
    assert_eq!(grok3["reasoning_effort"], "low");
    assert_eq!(grok3["max_tokens"], 320);

    let records = usage.records();
    assert_eq!(records[0].kind, AiUsageKind::Vision);
    let expected = 700.0 / 1e6 * 0.20 + 80.0 / 1e6 * 0.50 + 500.0 / 1e6 * 0.02;
    assert!(
        close(records[0].cost_usd, expected),
        "{}",
        records[0].cost_usd
    );
    let expected = 700.0 / 1e6 * 0.30 + 80.0 / 1e6 * 0.50 + 500.0 / 1e6 * 0.03;
    assert!(
        close(records[1].cost_usd, expected),
        "{}",
        records[1].cost_usd
    );
}

// ---------------------------------------------------------------------------------------------
// Response parsing
// ---------------------------------------------------------------------------------------------

#[tokio::test]
async fn length_finish_reason_is_reported_not_hidden() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ok_body(
            "gpt-5-mini",
            Some("{\"results\": ["),
            None,
            "length",
        )))
        .expect(1)
        .mount(&server)
        .await;
    let usage = Arc::new(MemoryUsageRepo::new());
    let resp = openai(&server, usage.clone())
        .complete(&request("gpt-5-mini", AiUsageKind::Classify))
        .await
        .unwrap();
    assert_eq!(resp.stop_reason, StopReason::MaxTokens);
    assert!(resp.is_truncated());
    assert_eq!(resp.text, "{\"results\": [");
    assert_eq!(usage.records().len(), 1, "a truncated call is still billed");
}

#[tokio::test]
async fn refusal_and_content_filter_are_errors_but_usage_is_recorded() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ok_body(
            "gpt-5-mini",
            None,
            Some("I'm sorry, I can't help with that."),
            "stop",
        )))
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ok_body(
            "gpt-5-mini",
            Some(""),
            None,
            "content_filter",
        )))
        .expect(1)
        .mount(&server)
        .await;
    let usage = Arc::new(MemoryUsageRepo::new());
    let c = openai(&server, usage.clone());
    let req = request("gpt-5-mini", AiUsageKind::Vision);
    assert!(matches!(c.complete(&req).await, Err(CoreError::AiRefused)));
    assert!(matches!(c.complete(&req).await, Err(CoreError::AiRefused)));
    assert_eq!(usage.records().len(), 2);
}

#[tokio::test]
async fn null_content_and_missing_usage_are_tolerated() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "choices": [{"message": {"role": "assistant", "content": null}, "finish_reason": "stop"}]
        })))
        .expect(1)
        .mount(&server)
        .await;
    let usage = Arc::new(MemoryUsageRepo::new());
    let resp = openai(&server, usage.clone())
        .complete(&request("gpt-4.1-mini", AiUsageKind::Classify))
        .await
        .unwrap();
    assert_eq!(resp.text, "");
    assert_eq!(resp.model, "gpt-4.1-mini", "falls back to the requested id");
    assert_eq!(
        (
            resp.input_tokens,
            resp.output_tokens,
            resp.cache_read_tokens
        ),
        (0, 0, 0)
    );
    assert!(close(usage.records()[0].cost_usd, 0.0));
    let body = body_of(&server.received_requests().await.unwrap()[0]);
    assert!(
        body.get("reasoning_effort").is_none(),
        "gpt-4* gets no effort"
    );
}

#[tokio::test]
async fn unparseable_success_body_is_an_ai_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_string("<html>oops</html>"))
        .expect(1)
        .mount(&server)
        .await;
    let usage = Arc::new(MemoryUsageRepo::new());
    let err = openai(&server, usage.clone())
        .complete(&request("gpt-5-mini", AiUsageKind::Classify))
        .await
        .unwrap_err();
    assert!(
        matches!(err, CoreError::Ai(ref m) if m.contains("unparseable")),
        "{err:?}"
    );
    assert!(usage.records().is_empty());
}

// ---------------------------------------------------------------------------------------------
// Error mapping
// ---------------------------------------------------------------------------------------------

#[tokio::test]
async fn unauthorized_is_not_retried() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(401).set_body_json(error_body(
            "invalid_request_error",
            Some("invalid_api_key"),
            "Incorrect API key provided: sk-test***.",
        )))
        .expect(1)
        .mount(&server)
        .await;
    let usage = Arc::new(MemoryUsageRepo::new());
    let err = openai(&server, usage.clone())
        .complete(&request("gpt-5-mini", AiUsageKind::Classify))
        .await
        .unwrap_err();
    match err {
        CoreError::Ai(msg) => {
            assert!(msg.contains("invalid api key (HTTP 401)"), "{msg}");
            assert!(msg.contains("Incorrect API key provided"), "{msg}");
        }
        other => panic!("unexpected {other:?}"),
    }
    assert!(usage.records().is_empty());
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn insufficient_quota_is_rejected_and_not_retried() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(429).set_body_json(error_body(
            "insufficient_quota",
            Some("insufficient_quota"),
            "You exceeded your current quota, please check your plan and billing details.",
        )))
        .expect(1)
        .mount(&server)
        .await;
    let usage = Arc::new(MemoryUsageRepo::new());
    let err = openai(&server, usage.clone())
        .complete(&request("gpt-5-mini", AiUsageKind::Classify))
        .await
        .unwrap_err();
    match err {
        CoreError::AiRejected(ref msg) => {
            assert!(msg.starts_with("cobrança:"), "{msg}");
            assert!(msg.contains("OpenAI"), "{msg}");
            assert!(msg.contains("exceeded your current quota"), "{msg}");
        }
        other => panic!("unexpected {other:?}"),
    }
    assert!(!err.is_transient());
    assert!(usage.records().is_empty());
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn xai_payment_required_is_rejected() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(402).set_body_json(json!({
            "error": "Your team has insufficient credits.", "code": "Payment required"
        })))
        .expect(1)
        .mount(&server)
        .await;
    let err = xai(&server, Arc::new(MemoryUsageRepo::new()))
        .complete(&request("grok-4", AiUsageKind::Classify))
        .await
        .unwrap_err();
    assert!(
        matches!(err, CoreError::AiRejected(ref m) if m.starts_with("cobrança:") && m.contains("xAI Grok") && m.contains("insufficient credits") && m.contains("console.x.ai")),
        "{err:?}"
    );
}

#[tokio::test]
async fn rate_limit_honours_retry_after_then_succeeds() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", "1")
                .set_body_json(error_body(
                    "requests",
                    Some("rate_limit_exceeded"),
                    "Rate limit reached. Please try again in 20s.",
                )),
        )
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ok_body(
            "gpt-5-mini",
            Some("{}"),
            None,
            "stop",
        )))
        .expect(1)
        .mount(&server)
        .await;
    let usage = Arc::new(MemoryUsageRepo::new());
    let started = Instant::now();
    let resp = openai(&server, usage.clone())
        .complete(&request("gpt-5-mini", AiUsageKind::Classify))
        .await
        .unwrap();
    let elapsed = started.elapsed();
    assert!(
        elapsed >= Duration::from_millis(900) && elapsed < Duration::from_secs(10),
        "header wins over the message hint: waited {elapsed:?}"
    );
    assert_eq!(resp.text, "{}");
    assert_eq!(
        usage.records().len(),
        1,
        "only the successful call is recorded"
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 2);
}

#[tokio::test]
async fn rate_limit_without_recovery_returns_rate_limited_with_message_hint() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(429).set_body_json(error_body(
            "tokens",
            Some("rate_limit_exceeded"),
            "Rate limit reached for gpt-5-mini. Please try again in 0.5s.",
        )))
        .expect(4)
        .mount(&server)
        .await;
    let err = openai(&server, Arc::new(MemoryUsageRepo::new()))
        .complete(&request("gpt-5-mini", AiUsageKind::Classify))
        .await
        .unwrap_err();
    assert!(
        matches!(
            err,
            CoreError::RateLimited {
                retry_after_secs: 1
            }
        ),
        "{err:?}"
    );
    assert!(err.is_transient());
}

#[tokio::test]
async fn retry_after_above_cap_gives_up_immediately() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(429).insert_header("retry-after", "120"))
        .expect(1)
        .mount(&server)
        .await;
    let err = xai(&server, Arc::new(MemoryUsageRepo::new()))
        .complete(&request("grok-4", AiUsageKind::Classify))
        .await
        .unwrap_err();
    assert!(
        matches!(
            err,
            CoreError::RateLimited {
                retry_after_secs: 120
            }
        ),
        "{err:?}"
    );
}

#[tokio::test]
async fn unknown_model_is_rejected() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(404).set_body_json(error_body(
            "invalid_request_error",
            Some("model_not_found"),
            "The model `gpt-9` does not exist or you do not have access to it.",
        )))
        .expect(1)
        .mount(&server)
        .await;
    let err = openai(&server, Arc::new(MemoryUsageRepo::new()))
        .complete(&request("gpt-9", AiUsageKind::Classify))
        .await
        .unwrap_err();
    assert!(
        matches!(err, CoreError::AiRejected(ref m) if m.starts_with("modelo não encontrado") && m.contains("gpt-9")),
        "{err:?}"
    );
}

#[tokio::test]
async fn bad_request_is_invalid_and_not_retried() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(400).set_body_json(error_body(
            "invalid_request_error",
            Some("unsupported_parameter"),
            "Unsupported parameter: 'temperature' is not supported with this model.",
        )))
        .expect(1)
        .mount(&server)
        .await;
    let err = openai(&server, Arc::new(MemoryUsageRepo::new()))
        .complete(&request("gpt-5", AiUsageKind::Classify))
        .await
        .unwrap_err();
    assert!(
        matches!(err, CoreError::Invalid(ref m) if m.contains("unsupported_parameter")),
        "{err:?}"
    );
}

#[tokio::test]
async fn server_error_is_retried_then_fails_as_ai_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(500).set_body_json(error_body(
            "server_error",
            None,
            "The server had an error while processing your request.",
        )))
        .expect(4)
        .mount(&server)
        .await;
    let err = openai(&server, Arc::new(MemoryUsageRepo::new()))
        .complete(&request("gpt-5-mini", AiUsageKind::Classify))
        .await
        .unwrap_err();
    assert!(
        matches!(err, CoreError::Ai(ref m) if m.contains("HTTP 500")),
        "{err:?}"
    );
    assert!(err.is_transient());
    assert_eq!(server.received_requests().await.unwrap().len(), 4);
}

#[tokio::test]
async fn server_error_then_success() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(503))
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ok_body(
            "grok-4",
            Some("{}"),
            None,
            "stop",
        )))
        .expect(1)
        .mount(&server)
        .await;
    let usage = Arc::new(MemoryUsageRepo::new());
    xai(&server, usage.clone())
        .complete(&request("grok-4", AiUsageKind::Classify))
        .await
        .unwrap();
    assert_eq!(usage.records().len(), 1);
    assert_eq!(server.received_requests().await.unwrap().len(), 2);
}

#[tokio::test]
async fn missing_key_means_not_configured_without_any_request() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;
    let usage = Arc::new(MemoryUsageRepo::new());
    let c = OpenAiCompatClient::new(
        config(&server, AiProvider::OpenAi),
        Arc::new(NoKey),
        usage.clone(),
    )
    .unwrap();
    assert!(matches!(
        c.complete(&request("gpt-5-mini", AiUsageKind::Classify))
            .await,
        Err(CoreError::AiNotConfigured)
    ));
    assert!(matches!(
        c.validate_key("gpt-5-mini").await,
        Err(CoreError::AiNotConfigured)
    ));
    assert!(matches!(
        c.list_models().await,
        Err(CoreError::AiNotConfigured)
    ));
    assert!(server.received_requests().await.unwrap().is_empty());
    assert!(usage.records().is_empty());
}

// ---------------------------------------------------------------------------------------------
// Key validation and model listing
// ---------------------------------------------------------------------------------------------

fn models_listing() -> Value {
    json!({
        "object": "list",
        "data": [
            {"id": "gpt-5", "object": "model", "owned_by": "system"},
            {"id": "text-embedding-3-small", "object": "model"},
            {"id": "gpt-5-mini", "object": "model"},
            {"id": "whisper-1", "object": "model"},
            {"id": "gpt-4o-audio-preview", "object": "model"},
            {"id": "gpt-4.1", "object": "model"},
            {"id": "gpt-5-mini", "object": "model"}
        ]
    })
}

#[tokio::test]
async fn validate_key_ok_probes_the_given_model_and_records_nothing() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .and(header("authorization", format!("Bearer {KEY}").as_str()))
        .respond_with(ResponseTemplate::new(200).set_body_json(models_listing()))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .and(header("authorization", format!("Bearer {KEY}").as_str()))
        .respond_with(ResponseTemplate::new(200).set_body_json(ok_body(
            "gpt-5-mini",
            Some(""),
            None,
            "length",
        )))
        .expect(1)
        .mount(&server)
        .await;
    let usage = Arc::new(MemoryUsageRepo::new());
    openai(&server, usage.clone())
        .validate_key("gpt-5-mini")
        .await
        .unwrap();
    let probe = body_of(&server.received_requests().await.unwrap()[1]);
    assert_eq!(probe["model"], "gpt-5-mini");
    assert_eq!(probe["max_completion_tokens"], 1);
    assert!(probe.get("max_tokens").is_none());
    assert_eq!(probe["messages"][0]["role"], "user");
    assert!(probe.get("response_format").is_none());
    assert!(
        usage.records().is_empty(),
        "key validation is not billed usage"
    );

    // xAI probes with max_tokens.
    let server2 = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": [{"id": "grok-4"}]})))
        .expect(1)
        .mount(&server2)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ok_body(
            "grok-4",
            Some("pong"),
            None,
            "stop",
        )))
        .expect(1)
        .mount(&server2)
        .await;
    xai(&server2, usage.clone())
        .validate_key("grok-4")
        .await
        .unwrap();
    let probe = body_of(&server2.received_requests().await.unwrap()[1]);
    assert_eq!(probe["max_tokens"], 1);
    assert!(probe.get("max_completion_tokens").is_none());
    assert!(usage.records().is_empty());
}

#[tokio::test]
async fn validate_key_rejects_bad_key_before_probing() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(ResponseTemplate::new(401).set_body_json(error_body(
            "invalid_request_error",
            Some("invalid_api_key"),
            "Incorrect API key provided",
        )))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;
    let err = openai(&server, Arc::new(MemoryUsageRepo::new()))
        .validate_key("gpt-5-mini")
        .await
        .unwrap_err();
    assert!(
        matches!(err, CoreError::Ai(ref m) if m.contains("invalid api key")),
        "{err:?}"
    );
}

#[tokio::test]
async fn validate_key_reports_billing_and_model_problems() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(ResponseTemplate::new(200).set_body_json(models_listing()))
        .expect(2)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(429).set_body_json(error_body(
            "insufficient_quota",
            Some("insufficient_quota"),
            "You exceeded your current quota, please check your plan and billing details.",
        )))
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(404).set_body_json(error_body(
            "invalid_request_error",
            Some("model_not_found"),
            "The model `gpt-9` does not exist or you do not have access to it.",
        )))
        .expect(1)
        .mount(&server)
        .await;
    let usage = Arc::new(MemoryUsageRepo::new());
    let c = openai(&server, usage.clone());
    let err = c.validate_key("gpt-5-mini").await.unwrap_err();
    assert!(
        matches!(err, CoreError::AiRejected(ref m) if m.starts_with("cobrança:")),
        "{err:?}"
    );
    let err = c.validate_key("gpt-9").await.unwrap_err();
    assert!(
        matches!(err, CoreError::AiRejected(ref m) if m.starts_with("modelo não encontrado")),
        "{err:?}"
    );
    assert!(usage.records().is_empty());
}

#[tokio::test]
async fn validate_key_ignores_inconclusive_probe_and_skips_blank_model() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(ResponseTemplate::new(200).set_body_json(models_listing()))
        .expect(2)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(503))
        .expect(4)
        .mount(&server)
        .await;
    let c = openai(&server, Arc::new(MemoryUsageRepo::new()));
    c.validate_key("gpt-5-mini").await.unwrap();
    c.validate_key("  ").await.unwrap();
}

#[tokio::test]
async fn list_models_filters_sorts_and_deduplicates() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .and(header("authorization", format!("Bearer {KEY}").as_str()))
        .respond_with(ResponseTemplate::new(200).set_body_json(models_listing()))
        .expect(1)
        .mount(&server)
        .await;
    let ids = openai(&server, Arc::new(MemoryUsageRepo::new()))
        .list_models()
        .await
        .unwrap();
    assert_eq!(ids, vec!["gpt-4.1", "gpt-5", "gpt-5-mini"]);

    let server2 = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(ResponseTemplate::new(403).set_body_json(error_body(
            "permission_error",
            None,
            "forbidden",
        )))
        .expect(1)
        .mount(&server2)
        .await;
    let err = xai(&server2, Arc::new(MemoryUsageRepo::new()))
        .list_models()
        .await
        .unwrap_err();
    assert!(
        matches!(err, CoreError::Ai(ref m) if m.contains("invalid api key (HTTP 403)")),
        "{err:?}"
    );
}
