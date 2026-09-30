//! The managed provider against a mock Ubi proxy: the license key travels as `x-api-key`
//! with the plan header, the recorded cost comes from `x-ubiqx-cost-usd`, the proxy's
//! budget cut-off is a rejection and the license status endpoint is parsed.

use std::sync::Arc;
use std::time::Duration;

use serde_json::json;
use ubiqx_ai::client::{
    AnthropicClient, AnthropicConfig, LlmMessage, LlmRequest, StaticApiKey, UBI_MODEL_ALIASES,
};
use ubiqx_ai::fake::MemoryUsageRepo;
use ubiqx_ai::ubi::UbiLicenseClient;
use ubiqx_core::ports::AiUsageKind;
use ubiqx_core::{AiProvider, CoreError, Plan};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const LICENSE: &str = "UBIQX-FAKECLAIMS-FAKESIG";

fn config(server: &MockServer) -> AnthropicConfig {
    AnthropicConfig {
        timeout: Duration::from_secs(5),
        max_attempts: 2,
        backoff_base: Duration::from_millis(10),
        ..AnthropicConfig::for_ubi(server.uri())
    }
}

fn client(server: &MockServer, usage: Arc<MemoryUsageRepo>) -> AnthropicClient {
    AnthropicClient::new(
        config(server),
        Arc::new(StaticApiKey(LICENSE.into())),
        usage,
    )
    .unwrap()
}

fn request(model: &str) -> LlmRequest {
    LlmRequest {
        model: model.into(),
        system: vec![],
        messages: vec![LlmMessage::user_text("classify this")],
        max_tokens: 64,
        json_schema: None,
        disable_thinking: false,
        usage_kind: AiUsageKind::Classify,
        timeout: None,
    }
}

fn ok_body() -> serde_json::Value {
    json!({
        "id": "msg_01",
        "type": "message",
        "role": "assistant",
        "model": "ubi-fast",
        "stop_reason": "end_turn",
        "content": [{"type": "text", "text": "{\"ok\":true}"}],
        "usage": {"input_tokens": 1000, "output_tokens": 100}
    })
}

#[tokio::test]
async fn sends_license_and_plan_headers_and_records_reported_cost() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(header("x-api-key", LICENSE))
        .and(header("x-ubiqx-plan", Plan::MonthlyManaged.id()))
        .and(header("anthropic-version", "2023-06-01"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(ok_body())
                .insert_header("x-ubiqx-cost-usd", "0.0042"),
        )
        .expect(1)
        .mount(&server)
        .await;
    let usage = Arc::new(MemoryUsageRepo::default());
    let c = client(&server, usage.clone());
    assert_eq!(c.provider(), AiProvider::Ubi);
    let resp = c.complete(&request("ubi-fast")).await.unwrap();
    assert_eq!(resp.text, "{\"ok\":true}");
    assert_eq!(resp.input_tokens, 1000);
    let recorded = usage.records();
    assert_eq!(recorded.len(), 1);
    assert_eq!(recorded[0].model, "ubi-fast");
    assert!(
        (recorded[0].cost_usd - 0.0042).abs() < 1e-12,
        "{}",
        recorded[0].cost_usd
    );
}

#[tokio::test]
async fn falls_back_to_the_tier_price_without_the_cost_header() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ok_body()))
        .mount(&server)
        .await;
    let usage = Arc::new(MemoryUsageRepo::default());
    client(&server, usage.clone())
        .complete(&request("ubi-fast"))
        .await
        .unwrap();
    // 1000 in × $1/M + 100 out × $5/M on the fast tier.
    let cost = usage.records()[0].cost_usd;
    assert!((cost - (0.001 + 0.0005)).abs() < 1e-12, "{cost}");
}

#[tokio::test]
async fn budget_exhausted_is_a_rejection_not_a_retry() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(402).set_body_json(json!({
            "type": "error",
            "error": {"type": "budget_exhausted", "message": "monthly budget reached"}
        })))
        .expect(1)
        .mount(&server)
        .await;
    let usage = Arc::new(MemoryUsageRepo::default());
    let err = client(&server, usage.clone())
        .complete(&request("ubi-smart"))
        .await
        .unwrap_err();
    assert!(
        matches!(err, CoreError::AiRejected(ref m) if m.contains("Ubi")),
        "{err}"
    );
    assert!(usage.records().is_empty());
}

#[tokio::test]
async fn unknown_model_is_a_rejection() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "type": "error",
            "error": {"type": "invalid_request_error", "message": "unknown model: claude-opus-5"}
        })))
        .mount(&server)
        .await;
    let usage = Arc::new(MemoryUsageRepo::default());
    let err = client(&server, usage)
        .complete(&request("claude-opus-5"))
        .await
        .unwrap_err();
    assert!(matches!(err, CoreError::AiRejected(_)), "{err}");
}

#[tokio::test]
async fn models_are_the_fixed_aliases_and_keys_are_not_probed() {
    let server = MockServer::start().await;
    // No mock mounted: any request would fail, and there must be none.
    let usage = Arc::new(MemoryUsageRepo::default());
    let c = client(&server, usage);
    assert_eq!(c.list_models().await.unwrap(), UBI_MODEL_ALIASES);
    assert!(matches!(c.validate_key().await, Err(CoreError::Invalid(_))));
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn license_status_endpoint() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/license/status"))
        .and(header("x-api-key", LICENSE))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "plan": "monthly_managed",
            "expires_at": "2026-10-18T00:00:00Z",
            "month": "2026-09",
            "spent_usd": 2.5,
            "budget_usd": 6.0,
            "state": "valid"
        })))
        .mount(&server)
        .await;
    let c = UbiLicenseClient::new(server.uri()).unwrap();
    let status = c.status(LICENSE).await.unwrap();
    assert_eq!(status.plan, Some(Plan::MonthlyManaged));
    assert_eq!(status.managed_usage().month, "2026-09");
    assert_eq!(status.managed_usage().spent_usd, 2.5);
    assert_eq!(status.state.as_deref(), Some("valid"));

    // A revoked key is a rejection; an outage is transient. Both are `Ai` errors.
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/license/status"))
        .respond_with(ResponseTemplate::new(401).set_body_string("revoked"))
        .mount(&server)
        .await;
    let err = UbiLicenseClient::new(server.uri())
        .unwrap()
        .status(LICENSE)
        .await
        .unwrap_err();
    assert!(
        matches!(err, CoreError::Ai(ref m) if m.contains("rejected")),
        "{err}"
    );
    let err = UbiLicenseClient::new("http://127.0.0.1:9")
        .unwrap()
        .status(LICENSE)
        .await
        .unwrap_err();
    assert!(matches!(err, CoreError::Ai(_)), "{err}");
}
