//! HTTP behaviour of `AnthropicClient` against a mock server.

use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use ubiqx_ai::client::{
    AnthropicClient, AnthropicConfig, ApiKeySource, LlmMessage, LlmRequest, StaticApiKey,
    StopReason, SystemBlock,
};
use ubiqx_ai::fake::MemoryUsageRepo;
use ubiqx_core::ports::AiUsageKind;
use ubiqx_core::CoreError;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

const KEY: &str = "sk-ant-test-key";

struct NoKey;

impl ApiKeySource for NoKey {
    fn api_key(&self) -> Option<String> {
        None
    }
}

fn config(server: &MockServer) -> AnthropicConfig {
    AnthropicConfig {
        base_url: server.uri(),
        timeout: Duration::from_secs(5),
        max_attempts: 4,
        backoff_base: Duration::from_millis(10),
        backoff_cap: Duration::from_secs(30),
        ..AnthropicConfig::default()
    }
}

fn client(server: &MockServer, usage: Arc<MemoryUsageRepo>) -> AnthropicClient {
    AnthropicClient::new(config(server), Arc::new(StaticApiKey(KEY.into())), usage).unwrap()
}

fn request(model: &str) -> LlmRequest {
    LlmRequest {
        model: model.into(),
        system: vec![
            SystemBlock::new("You classify things.", false),
            SystemBlock::new("Profile.", !model.contains("haiku")),
        ],
        messages: vec![LlmMessage::user_text("<blocks>\n- id=b1\n</blocks>")],
        max_tokens: 320,
        json_schema: Some(
            json!({"type": "object", "properties": {"ok": {"type": "boolean"}}, "required": ["ok"], "additionalProperties": false}),
        ),
        disable_thinking: !model.contains("haiku"),
        usage_kind: AiUsageKind::Classify,
        timeout: None,
    }
}

fn ok_body(text: &str, stop_reason: &str) -> Value {
    json!({
        "id": "msg_01",
        "type": "message",
        "role": "assistant",
        "model": "claude-haiku-4-5-20251001",
        "stop_reason": stop_reason,
        "content": [{"type": "text", "text": text}],
        "usage": {
            "input_tokens": 1200,
            "output_tokens": 80,
            "cache_creation_input_tokens": 0,
            "cache_read_input_tokens": 500,
            "service_tier": "standard"
        }
    })
}

fn error_body(kind: &str, message: &str) -> Value {
    json!({"type": "error", "error": {"type": kind, "message": message}})
}

fn body_of(req: &Request) -> Value {
    serde_json::from_slice(&req.body).unwrap()
}

#[tokio::test]
async fn success_path_parses_usage_and_records_cost() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(header("x-api-key", KEY))
        .and(header("anthropic-version", "2023-06-01"))
        .and(header("content-type", "application/json"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(ok_body("{\"ok\":true}", "end_turn")),
        )
        .expect(1)
        .mount(&server)
        .await;
    let usage = Arc::new(MemoryUsageRepo::new());
    let resp = client(&server, usage.clone())
        .complete(&request("claude-haiku-4-5"))
        .await
        .unwrap();

    assert_eq!(resp.text, "{\"ok\":true}");
    assert_eq!(resp.model, "claude-haiku-4-5-20251001");
    assert_eq!(resp.input_tokens, 1200);
    assert_eq!(resp.output_tokens, 80);
    assert_eq!(resp.cache_read_tokens, 500);
    assert_eq!(resp.cache_write_tokens, 0);
    assert_eq!(resp.stop_reason, StopReason::EndTurn);
    assert!(!resp.is_truncated());

    let records = usage.records();
    assert_eq!(records.len(), 1);
    let u = &records[0];
    assert_eq!(u.kind, AiUsageKind::Classify);
    assert_eq!(
        u.model, "claude-haiku-4-5",
        "usage is recorded under the requested model id"
    );
    assert_eq!(
        (
            u.input_tokens,
            u.output_tokens,
            u.cache_read_tokens,
            u.cache_write_tokens
        ),
        (1200, 80, 500, 0)
    );
    let expected = 1200.0 / 1e6 * 1.0 + 80.0 / 1e6 * 5.0 + 500.0 / 1e6 * 0.1;
    assert!(
        (u.cost_usd - expected).abs() < 1e-12,
        "{} vs {expected}",
        u.cost_usd
    );

    // Exact request shape for a Haiku classification call.
    let sent = server.received_requests().await.unwrap();
    let body = body_of(&sent[0]);
    assert_eq!(body["model"], "claude-haiku-4-5");
    assert_eq!(body["max_tokens"], 320);
    assert_eq!(
        body["system"],
        json!([{"type": "text", "text": "You classify things."}, {"type": "text", "text": "Profile."}])
    );
    assert_eq!(
        body["messages"],
        json!([{"role": "user", "content": [{"type": "text", "text": "<blocks>\n- id=b1\n</blocks>"}]}])
    );
    assert_eq!(body["output_config"]["format"]["type"], "json_schema");
    assert_eq!(
        body["output_config"]["format"]["schema"]["additionalProperties"],
        false
    );
    assert!(body.get("thinking").is_none(), "no thinking field on Haiku");
    assert!(body.get("temperature").is_none());
}

#[tokio::test]
async fn sonnet_request_disables_thinking_and_caches_last_system_block() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ok_body("{}", "end_turn")))
        .expect(1)
        .mount(&server)
        .await;
    let usage = Arc::new(MemoryUsageRepo::new());
    let mut req = request("claude-sonnet-5");
    req.usage_kind = AiUsageKind::Report;
    client(&server, usage.clone()).complete(&req).await.unwrap();
    let body = body_of(&server.received_requests().await.unwrap()[0]);
    assert_eq!(body["thinking"], json!({"type": "disabled"}));
    assert!(body["system"][0].get("cache_control").is_none());
    assert_eq!(
        body["system"][1]["cache_control"],
        json!({"type": "ephemeral"})
    );
    let u = &usage.records()[0];
    assert_eq!(u.kind, AiUsageKind::Report);
    let expected = 1200.0 / 1e6 * 2.0 + 80.0 / 1e6 * 10.0 + 500.0 / 1e6 * 0.2;
    assert!((u.cost_usd - expected).abs() < 1e-12);
}

#[tokio::test]
async fn unauthorized_is_not_retried() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(401)
                .set_body_json(error_body("authentication_error", "invalid x-api-key")),
        )
        .expect(1)
        .mount(&server)
        .await;
    let usage = Arc::new(MemoryUsageRepo::new());
    let err = client(&server, usage.clone())
        .complete(&request("claude-haiku-4-5"))
        .await
        .unwrap_err();
    match err {
        CoreError::Ai(msg) => {
            assert!(msg.contains("invalid api key"), "{msg}");
            assert!(msg.contains("invalid x-api-key"), "{msg}");
        }
        other => panic!("unexpected {other:?}"),
    }
    assert!(usage.records().is_empty());
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn bad_request_is_invalid_and_not_retried() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(400)
                .set_body_json(error_body("invalid_request_error", "max_tokens: too large")),
        )
        .expect(1)
        .mount(&server)
        .await;
    let err = client(&server, Arc::new(MemoryUsageRepo::new()))
        .complete(&request("claude-haiku-4-5"))
        .await
        .unwrap_err();
    assert!(
        matches!(err, CoreError::Invalid(ref m) if m.contains("max_tokens: too large")),
        "{err:?}"
    );
}

#[tokio::test]
async fn rate_limit_honours_retry_after_then_succeeds() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", "1")
                .set_body_json(error_body("rate_limit_error", "slow down")),
        )
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ok_body("{}", "end_turn")))
        .expect(1)
        .mount(&server)
        .await;
    let usage = Arc::new(MemoryUsageRepo::new());
    let started = Instant::now();
    let resp = client(&server, usage.clone())
        .complete(&request("claude-haiku-4-5"))
        .await
        .unwrap();
    assert!(
        started.elapsed() >= Duration::from_millis(900),
        "waited {:?}",
        started.elapsed()
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
async fn rate_limit_without_recovery_returns_rate_limited() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(429).insert_header("retry-after", "0"))
        .expect(4)
        .mount(&server)
        .await;
    let err = client(&server, Arc::new(MemoryUsageRepo::new()))
        .complete(&request("claude-haiku-4-5"))
        .await
        .unwrap_err();
    assert!(
        matches!(
            err,
            CoreError::RateLimited {
                retry_after_secs: 0
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
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(429).insert_header("retry-after", "120"))
        .expect(1)
        .mount(&server)
        .await;
    let err = client(&server, Arc::new(MemoryUsageRepo::new()))
        .complete(&request("claude-haiku-4-5"))
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
async fn overloaded_then_success() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(529).set_body_json(error_body("overloaded_error", "Overloaded")),
        )
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ok_body("{}", "end_turn")))
        .expect(1)
        .mount(&server)
        .await;
    let usage = Arc::new(MemoryUsageRepo::new());
    client(&server, usage.clone())
        .complete(&request("claude-haiku-4-5"))
        .await
        .unwrap();
    assert_eq!(usage.records().len(), 1);
    assert_eq!(server.received_requests().await.unwrap().len(), 2);
}

#[tokio::test]
async fn persistent_server_errors_exhaust_attempts() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(503))
        .expect(4)
        .mount(&server)
        .await;
    let err = client(&server, Arc::new(MemoryUsageRepo::new()))
        .complete(&request("claude-haiku-4-5"))
        .await
        .unwrap_err();
    assert!(
        matches!(err, CoreError::Ai(ref m) if m.contains("HTTP 503")),
        "{err:?}"
    );
    assert!(err.is_transient());
}

#[tokio::test]
async fn max_tokens_stop_reason_is_reported_not_hidden() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(ok_body("{\"results\": [", "max_tokens")),
        )
        .expect(1)
        .mount(&server)
        .await;
    let usage = Arc::new(MemoryUsageRepo::new());
    let resp = client(&server, usage.clone())
        .complete(&request("claude-haiku-4-5"))
        .await
        .unwrap();
    assert_eq!(resp.stop_reason, StopReason::MaxTokens);
    assert!(resp.is_truncated());
    assert_eq!(resp.text, "{\"results\": [");
    assert_eq!(
        usage.records().len(),
        1,
        "a truncated call is still billed and recorded"
    );
}

#[tokio::test]
async fn refusal_is_an_error_but_usage_is_recorded() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ok_body("", "refusal")))
        .expect(1)
        .mount(&server)
        .await;
    let usage = Arc::new(MemoryUsageRepo::new());
    let err = client(&server, usage.clone())
        .complete(&request("claude-haiku-4-5"))
        .await
        .unwrap_err();
    assert!(matches!(err, CoreError::AiRefused), "{err:?}");
    assert_eq!(usage.records().len(), 1);
}

#[tokio::test]
async fn zero_credit_account_is_rejected_and_not_retried() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(400).set_body_json(error_body(
            "invalid_request_error",
            "Your credit balance is too low to access the Anthropic API. Please go to Plans & Billing to upgrade or purchase credits.",
        )))
        .expect(1)
        .mount(&server)
        .await;
    let usage = Arc::new(MemoryUsageRepo::new());
    let err = client(&server, usage.clone())
        .complete(&request("claude-haiku-4-5"))
        .await
        .unwrap_err();
    assert!(
        matches!(err, CoreError::AiRejected(ref m) if m.starts_with("cobrança:")),
        "{err:?}"
    );
    assert!(!err.is_transient());
    assert!(usage.records().is_empty());
}

#[tokio::test]
async fn unknown_model_is_rejected() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(404)
                .set_body_json(error_body("not_found_error", "model: claude-haiku-9")),
        )
        .expect(1)
        .mount(&server)
        .await;
    let err = client(&server, Arc::new(MemoryUsageRepo::new()))
        .complete(&request("claude-haiku-9"))
        .await
        .unwrap_err();
    assert!(
        matches!(err, CoreError::AiRejected(ref m) if m.contains("claude-haiku-9")),
        "{err:?}"
    );
}

fn models_listing() -> Value {
    json!({
        "data": [{"id": "claude-sonnet-5", "type": "model"}, {"id": "claude-haiku-4-5", "type": "model"}],
        "has_more": false
    })
}

#[tokio::test]
async fn validate_key_ok_and_unauthorized() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .and(header("x-api-key", KEY))
        .and(header("anthropic-version", "2023-06-01"))
        .respond_with(ResponseTemplate::new(200).set_body_json(models_listing()))
        .expect(1)
        .mount(&server)
        .await;
    // The billing probe: one token with the cheapest listed model.
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(header("x-api-key", KEY))
        .respond_with(ResponseTemplate::new(200).set_body_json(ok_body("x", "max_tokens")))
        .expect(1)
        .mount(&server)
        .await;
    let usage = Arc::new(MemoryUsageRepo::new());
    client(&server, usage.clone()).validate_key().await.unwrap();
    let probe = &server.received_requests().await.unwrap()[1];
    let body = body_of(probe);
    assert_eq!(body["model"], "claude-haiku-4-5");
    assert_eq!(body["max_tokens"], 1);
    assert!(
        usage.records().is_empty(),
        "key validation is not billed usage"
    );

    let server2 = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(
            ResponseTemplate::new(401)
                .set_body_json(error_body("authentication_error", "invalid x-api-key")),
        )
        .expect(1)
        .mount(&server2)
        .await;
    let err = client(&server2, usage).validate_key().await.unwrap_err();
    assert!(
        matches!(err, CoreError::Ai(ref m) if m.contains("invalid api key")),
        "{err:?}"
    );
}

#[tokio::test]
async fn validate_key_reports_missing_credits() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(ResponseTemplate::new(200).set_body_json(models_listing()))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(400).set_body_json(error_body(
            "invalid_request_error",
            "Your credit balance is too low to access the Anthropic API.",
        )))
        .expect(1)
        .mount(&server)
        .await;
    let err = client(&server, Arc::new(MemoryUsageRepo::new()))
        .validate_key()
        .await
        .unwrap_err();
    assert!(
        matches!(err, CoreError::AiRejected(ref m) if m.contains("créditos")),
        "{err:?}"
    );
}

#[tokio::test]
async fn validate_key_ignores_inconclusive_probe() {
    // A probe that fails for any reason other than billing (here: an outage that exhausts
    // the retries) must not turn a valid key into a rejection.
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(ResponseTemplate::new(200).set_body_json(models_listing()))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(529).set_body_json(error_body("overloaded_error", "Overloaded")),
        )
        .mount(&server)
        .await;
    client(&server, Arc::new(MemoryUsageRepo::new()))
        .validate_key()
        .await
        .unwrap();
}

#[tokio::test]
async fn missing_key_means_not_configured_without_any_request() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ok_body("{}", "end_turn")))
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
    let c = AnthropicClient::new(config(&server), Arc::new(NoKey), usage.clone()).unwrap();
    assert!(matches!(
        c.complete(&request("claude-haiku-4-5")).await,
        Err(CoreError::AiNotConfigured)
    ));
    assert!(matches!(
        c.validate_key().await,
        Err(CoreError::AiNotConfigured)
    ));
    assert!(server.received_requests().await.unwrap().is_empty());
    assert!(usage.records().is_empty());
}

#[tokio::test]
async fn unparseable_success_body_is_an_ai_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_string("<html>oops</html>"))
        .expect(1)
        .mount(&server)
        .await;
    let usage = Arc::new(MemoryUsageRepo::new());
    let err = client(&server, usage.clone())
        .complete(&request("claude-haiku-4-5"))
        .await
        .unwrap_err();
    assert!(
        matches!(err, CoreError::Ai(ref m) if m.contains("unparseable")),
        "{err:?}"
    );
    assert!(usage.records().is_empty());
}

#[tokio::test]
async fn list_models_returns_sorted_ids() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .and(header("x-api-key", KEY))
        .and(header("anthropic-version", "2023-06-01"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": [
                {"id": "claude-sonnet-5", "type": "model"},
                {"id": "claude-haiku-4-5", "type": "model"},
                {"id": "claude-opus-5", "type": "model"},
                {"id": "claude-haiku-4-5", "type": "model"}
            ],
            "has_more": false
        })))
        .expect(1)
        .mount(&server)
        .await;
    let usage = Arc::new(MemoryUsageRepo::new());
    let ids = client(&server, usage.clone()).list_models().await.unwrap();
    assert_eq!(
        ids,
        vec!["claude-haiku-4-5", "claude-opus-5", "claude-sonnet-5"]
    );
    assert!(usage.records().is_empty());
    let sent = server.received_requests().await.unwrap();
    assert_eq!(sent[0].url.query(), Some("limit=1000"));

    let server2 = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/models"))
        .respond_with(
            ResponseTemplate::new(401)
                .set_body_json(error_body("authentication_error", "invalid x-api-key")),
        )
        .expect(1)
        .mount(&server2)
        .await;
    let err = client(&server2, usage).list_models().await.unwrap_err();
    assert!(
        matches!(err, CoreError::Ai(ref m) if m.contains("invalid api key")),
        "{err:?}"
    );
}
