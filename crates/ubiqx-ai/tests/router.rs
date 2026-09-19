//! `RoutingLlmClient` picks the vendor client the `ProviderSource` currently selects.

use std::sync::Arc;

use parking_lot::Mutex;
use serde_json::json;
use ubiqx_ai::client::{LlmClient, LlmMessage, LlmRequest, StaticApiKey, SystemBlock};
use ubiqx_ai::fake::{MemoryUsageRepo, ScriptedLlmClient};
use ubiqx_ai::openai::{OpenAiCompatClient, OpenAiCompatConfig};
use ubiqx_ai::router::{FixedProvider, ProviderSource, RoutingLlmClient};
use ubiqx_core::ports::AiUsageKind;
use ubiqx_core::{AiProvider, CoreError};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// A settings-like source the test flips at runtime.
struct SwitchableProvider(Mutex<AiProvider>);

impl ProviderSource for SwitchableProvider {
    fn provider(&self) -> AiProvider {
        *self.0.lock()
    }
}

fn request(model: &str) -> LlmRequest {
    LlmRequest {
        model: model.into(),
        system: vec![SystemBlock::new("sys", false)],
        messages: vec![LlmMessage::user_text("hi")],
        max_tokens: 64,
        json_schema: None,
        disable_thinking: false,
        usage_kind: AiUsageKind::Classify,
        timeout: None,
    }
}

#[tokio::test]
async fn routes_to_the_selected_provider_and_follows_changes() {
    let anthropic = Arc::new(ScriptedLlmClient::new(vec!["from-anthropic".into()]));
    let openai = Arc::new(ScriptedLlmClient::new(vec!["from-openai".into()]));
    let source = Arc::new(SwitchableProvider(Mutex::new(AiProvider::Anthropic)));
    let router = RoutingLlmClient::new(
        vec![
            (AiProvider::Anthropic, anthropic.clone()),
            (AiProvider::OpenAi, openai.clone()),
        ],
        source.clone(),
    );

    let resp = router.complete(&request("claude-haiku-4-5")).await.unwrap();
    assert_eq!(resp.text, "from-anthropic");
    assert_eq!(router.selected(), AiProvider::Anthropic);

    *source.0.lock() = AiProvider::OpenAi;
    let resp = router.complete(&request("gpt-5-mini")).await.unwrap();
    assert_eq!(resp.text, "from-openai");
    assert_eq!(router.selected(), AiProvider::OpenAi);

    // No client for xAI: not configured, and nobody else is called.
    *source.0.lock() = AiProvider::Xai;
    assert!(matches!(
        router.complete(&request("grok-4")).await,
        Err(CoreError::AiNotConfigured)
    ));
    assert_eq!(anthropic.requests().len(), 1);
    assert_eq!(openai.requests().len(), 1);
    assert_eq!(
        router.providers(),
        vec![AiProvider::Anthropic, AiProvider::OpenAi]
    );
}

#[tokio::test]
async fn fixed_provider_with_a_real_http_client() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "model": "grok-4-1-fast-non-reasoning",
            "choices": [{"message": {"role": "assistant", "content": "{\"ok\":true}"}, "finish_reason": "stop"}],
            "usage": {"prompt_tokens": 10, "completion_tokens": 5}
        })))
        .expect(1)
        .mount(&server)
        .await;
    let usage = Arc::new(MemoryUsageRepo::new());
    let xai: Arc<dyn LlmClient> = Arc::new(
        OpenAiCompatClient::new(
            OpenAiCompatConfig::with_base_url(AiProvider::Xai, format!("{}/v1", server.uri())),
            Arc::new(StaticApiKey("xai-test".into())),
            usage.clone(),
        )
        .unwrap(),
    );
    let router = RoutingLlmClient::new(
        vec![(AiProvider::Xai, xai)],
        Arc::new(FixedProvider(AiProvider::Xai)),
    );
    let resp = router
        .complete(&request("grok-4-1-fast-non-reasoning"))
        .await
        .unwrap();
    assert_eq!(resp.text, "{\"ok\":true}");
    assert_eq!(usage.records().len(), 1);
    assert_eq!(usage.records()[0].model, "grok-4-1-fast-non-reasoning");

    let empty = RoutingLlmClient::new(vec![], Arc::new(FixedProvider(AiProvider::Anthropic)));
    assert!(matches!(
        empty.complete(&request("claude-haiku-4-5")).await,
        Err(CoreError::AiNotConfigured)
    ));
}
