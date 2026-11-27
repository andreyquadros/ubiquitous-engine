//! The screen check of the `nofap` module over HTTP: what each provider is sent, how a
//! refusal and a truncated answer are handled, what is recorded and what is logged.

use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use base64::Engine;
use chrono::{DateTime, Timelike, Utc};
use serde_json::{json, Value};
use tracing::subscriber::DefaultGuard;
use tracing_subscriber::fmt::MakeWriter;
use ubiqx_ai::client::{
    AnthropicClient, AnthropicConfig, LlmClient, LlmContent, LlmMessage, LlmRequest, StaticApiKey,
    SystemBlock,
};
use ubiqx_ai::explicit::{explicit_schema, LlmExplicitDetector, REFUSED, UNSURE};
use ubiqx_ai::fake::MemoryUsageRepo;
use ubiqx_ai::openai::{OpenAiCompatClient, OpenAiCompatConfig, SCHEMA_NAME};
use ubiqx_ai::prompts::{EXPLICIT_SYSTEM_PROMPT, EXPLICIT_USER_TEXT};
use ubiqx_ai::router::{ProviderSource, RoutingLlmClient};
use ubiqx_core::nofap::{Confidence, ExplicitCheck, ExplicitDetector, ExplicitVerdict};
use ubiqx_core::ports::{AiUsage, AiUsageKind, EncodedImage};
use ubiqx_core::{AiProvider, CoreError};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

const ANTHROPIC_KEY: &str = "sk-ant-test-key";
const OPENAI_KEY: &str = "sk-openai-test-key";
const XAI_KEY: &str = "xai-test-key";
const LICENSE: &str = "UBIQX-FAKECLAIMS-FAKESIG";

const ANTHROPIC_PATH: &str = "/anthropic/v1/messages";
const UBI_PATH: &str = "/ubi/v1/messages";
const OPENAI_PATH: &str = "/openai/v1/chat/completions";
const XAI_PATH: &str = "/xai/v1/chat/completions";

// ---------------------------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------------------------

/// Stands in for a reduced frame: bytes that are easy to recognise once encoded.
fn frame() -> EncodedImage {
    EncodedImage {
        bytes: (0u8..=255).cycle().take(3000).collect(),
        mime: "image/jpeg".into(),
        width: 640,
        height: 400,
    }
}

fn frame_base64() -> String {
    base64::engine::general_purpose::STANDARD.encode(frame().bytes)
}

fn verdict(verdict: ExplicitVerdict, confidence: Confidence) -> ExplicitCheck {
    ExplicitCheck {
        verdict,
        confidence,
        refused: false,
    }
}

struct Selected(Mutex<AiProvider>);

impl ProviderSource for Selected {
    fn provider(&self) -> AiProvider {
        *self.0.lock().unwrap()
    }
}

/// The four providers behind one routing client, as the apps wire them, each talking to its
/// own path of the mock server.
struct Rig {
    server: MockServer,
    usage: Arc<MemoryUsageRepo>,
    selected: Arc<Selected>,
    router: Arc<RoutingLlmClient>,
    detector: LlmExplicitDetector,
}

impl Rig {
    async fn start() -> Self {
        let server = MockServer::start().await;
        let usage = Arc::new(MemoryUsageRepo::new());
        let anthropic = AnthropicClient::new(
            AnthropicConfig {
                base_url: format!("{}/anthropic", server.uri()),
                timeout: Duration::from_secs(5),
                max_attempts: 2,
                backoff_base: Duration::from_millis(5),
                ..AnthropicConfig::default()
            },
            Arc::new(StaticApiKey(ANTHROPIC_KEY.into())),
            usage.clone(),
        )
        .unwrap();
        let ubi = AnthropicClient::new(
            AnthropicConfig {
                timeout: Duration::from_secs(5),
                max_attempts: 2,
                backoff_base: Duration::from_millis(5),
                ..AnthropicConfig::for_ubi(format!("{}/ubi", server.uri()))
            },
            Arc::new(StaticApiKey(LICENSE.into())),
            usage.clone(),
        )
        .unwrap();
        let compat = |provider: AiProvider, prefix: &str, key: &str| {
            OpenAiCompatClient::new(
                OpenAiCompatConfig {
                    provider,
                    base_url: format!("{}/{prefix}/v1", server.uri()),
                    timeout: Duration::from_secs(5),
                    max_attempts: 2,
                    backoff_base: Duration::from_millis(5),
                    backoff_cap: Duration::from_secs(30),
                },
                Arc::new(StaticApiKey(key.into())),
                usage.clone(),
            )
            .unwrap()
        };
        let clients: Vec<(AiProvider, Arc<dyn LlmClient>)> = vec![
            (AiProvider::Anthropic, Arc::new(anthropic)),
            (AiProvider::Ubi, Arc::new(ubi)),
            (
                AiProvider::OpenAi,
                Arc::new(compat(AiProvider::OpenAi, "openai", OPENAI_KEY)),
            ),
            (
                AiProvider::Xai,
                Arc::new(compat(AiProvider::Xai, "xai", XAI_KEY)),
            ),
        ];
        let selected = Arc::new(Selected(Mutex::new(AiProvider::Anthropic)));
        let router = Arc::new(RoutingLlmClient::new(clients, selected.clone()));
        let detector = LlmExplicitDetector::new(router.clone());
        Rig {
            server,
            usage,
            selected,
            router,
            detector,
        }
    }

    fn select(&self, provider: AiProvider) {
        *self.selected.0.lock().unwrap() = provider;
    }

    async fn check(&self, model: &str) -> Result<ExplicitCheck, CoreError> {
        self.detector.check(&frame(), model).await
    }

    async fn requests_to(&self, route: &str) -> Vec<Value> {
        self.server
            .received_requests()
            .await
            .unwrap_or_default()
            .iter()
            .filter(|r| r.url.path() == route)
            .map(body_of)
            .collect()
    }
}

fn body_of(req: &Request) -> Value {
    serde_json::from_slice(&req.body).unwrap()
}

fn anthropic_answer(text: &str, stop_reason: &str) -> Value {
    json!({
        "id": "msg_01",
        "type": "message",
        "role": "assistant",
        "model": "claude-haiku-4-5-20251001",
        "stop_reason": stop_reason,
        "content": [{"type": "text", "text": text}],
        "usage": {"input_tokens": 600, "output_tokens": 15}
    })
}

fn chat_answer(content: Option<&str>, refusal: Option<&str>, finish: &str) -> Value {
    json!({
        "id": "chatcmpl-01",
        "object": "chat.completion",
        "model": "gpt-5-mini-2026-01-01",
        "choices": [{
            "index": 0,
            "message": {"role": "assistant", "content": content, "refusal": refusal},
            "finish_reason": finish
        }],
        "usage": {"prompt_tokens": 610, "completion_tokens": 14, "total_tokens": 624}
    })
}

async fn answer_with(server: &MockServer, route: &str, body: Value) {
    Mock::given(method("POST"))
        .and(path(route))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(server)
        .await;
}

async fn answer_once_with(server: &MockServer, route: &str, body: Value) {
    Mock::given(method("POST"))
        .and(path(route))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .up_to_n_times(1)
        .mount(server)
        .await;
}

/// Every string in `value` that is not the image, a media type or a word of the wire format
/// itself (object keys are not values and are not listed).
fn strings_in(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::String(s) => out.push(s.clone()),
        Value::Array(items) => items.iter().for_each(|v| strings_in(v, out)),
        Value::Object(map) => map.values().for_each(|v| strings_in(v, out)),
        _ => {}
    }
}

fn start_of_hour(t: DateTime<Utc>) -> DateTime<Utc> {
    t.with_minute(0)
        .and_then(|t| t.with_second(0))
        .and_then(|t| t.with_nanosecond(0))
        .unwrap()
}

/// The usage row of a screen check made between `before` and `after`: its own kind, and a
/// time that says the hour and nothing finer.
fn assert_on_the_hour(row: &AiUsage, before: DateTime<Utc>, after: DateTime<Utc>) {
    assert_eq!(row.kind, AiUsageKind::NofapVision);
    assert_eq!(
        (row.at.minute(), row.at.second(), row.at.nanosecond()),
        (0, 0, 0),
        "{}",
        row.at
    );
    assert!(
        row.at == start_of_hour(before) || row.at == start_of_hour(after),
        "{} is the hour of the call",
        row.at
    );
}

// ---------------------------------------------------------------------------------------------
// Log capture
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Default)]
struct Capture(Arc<Mutex<Vec<u8>>>);

impl Write for Capture {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for Capture {
    type Writer = Capture;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

impl Capture {
    /// Captures everything logged on this thread, at every level, until the guard drops.
    /// The tests run on a current-thread runtime, so the client's work happens here.
    fn start() -> (Capture, DefaultGuard) {
        let capture = Capture::default();
        let subscriber = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::TRACE)
            .with_ansi(false)
            .with_writer(capture.clone())
            .finish();
        let guard = tracing::subscriber::set_default(subscriber);
        (capture, guard)
    }

    /// Everything captured, the HTTP library's own lines included.
    fn everything(&self) -> String {
        String::from_utf8_lossy(&self.0.lock().unwrap()).into_owned()
    }

    /// What this crate logged: the capture without the lines of the HTTP library under
    /// it, which names the address it connects to at `debug` and `trace` for every request
    /// of every kind and is not this crate's to silence (see [`transport_lines`]).
    fn text(&self) -> String {
        self.everything()
            .lines()
            .filter(|line| !is_transport(line))
            .map(|line| format!("{line}\n"))
            .collect()
    }

    /// The lines of the HTTP library under the clients.
    fn transport_lines(&self) -> Vec<String> {
        self.everything()
            .lines()
            .filter(|line| is_transport(line))
            .map(str::to_string)
            .collect()
    }

    fn clear(&self) {
        self.0.lock().unwrap().clear();
    }
}

fn is_transport(line: &str) -> bool {
    line.contains(" hyper_util::")
}

/// Nothing the HTTP library logs says what a request was for or what came back.
fn assert_transport_says_nothing(capture: &Capture) {
    let image = frame_base64();
    for line in capture.transport_lines() {
        let lower = line.to_lowercase();
        for word in [
            "nofap",
            "explicit",
            "suggestive",
            "verdict",
            "confidence",
            "refus",
            "classif",
            "screenshot",
            "messages",
            "completions",
        ] {
            assert!(
                !lower.contains(word),
                "transport line with `{word}`: {line}"
            );
        }
        assert!(!line.contains(&image[..64]), "{line}");
    }
}

/// A productivity call with an image, as the vision classifier makes one.
fn vision_request(model: &str, kind: AiUsageKind) -> LlmRequest {
    LlmRequest {
        model: model.into(),
        system: vec![SystemBlock::new("You classify things.", false)],
        messages: vec![LlmMessage::user(vec![
            LlmContent::Image {
                media_type: "image/jpeg".into(),
                base64: frame_base64(),
            },
            LlmContent::Text("- id=b1".into()),
        ])],
        max_tokens: 400,
        json_schema: None,
        disable_thinking: false,
        usage_kind: kind,
        timeout: None,
    }
}

// ---------------------------------------------------------------------------------------------
// What each provider is sent
// ---------------------------------------------------------------------------------------------

#[tokio::test]
async fn anthropic_round_trip_sends_the_image_and_the_two_constants() {
    let rig = Rig::start().await;
    Mock::given(method("POST"))
        .and(path(ANTHROPIC_PATH))
        .and(header("x-api-key", ANTHROPIC_KEY))
        .respond_with(ResponseTemplate::new(200).set_body_json(anthropic_answer(
            r#"{"verdict":"explicit","confidence":"high"}"#,
            "end_turn",
        )))
        .expect(1)
        .mount(&rig.server)
        .await;

    let before = Utc::now();
    let out = rig.check("claude-haiku-4-5").await.unwrap();
    let after = Utc::now();
    assert_eq!(out, verdict(ExplicitVerdict::Explicit, Confidence::High));

    let bodies = rig.requests_to(ANTHROPIC_PATH).await;
    assert_eq!(bodies.len(), 1);
    assert_eq!(
        bodies[0],
        json!({
            "model": "claude-haiku-4-5",
            "max_tokens": 64,
            "system": [{"type": "text", "text": EXPLICIT_SYSTEM_PROMPT}],
            "messages": [{"role": "user", "content": [
                {"type": "image", "source": {
                    "type": "base64", "media_type": "image/jpeg", "data": frame_base64()}},
                {"type": "text", "text": EXPLICIT_USER_TEXT}
            ]}],
            "output_config": {"format": {"type": "json_schema", "schema": explicit_schema()}}
        }),
        "the whole body: nothing about the person, the page or the app"
    );

    let rows = rig.usage.records();
    assert_eq!(rows.len(), 1);
    assert_on_the_hour(&rows[0], before, after);
    assert_eq!(rows[0].model, "claude-haiku-4-5");
    assert_eq!((rows[0].input_tokens, rows[0].output_tokens), (600, 15));
    // 600 in and 15 out at Haiku's 1 / 5 dollars per million tokens.
    assert!(
        (rows[0].cost_usd - 0.000675).abs() < 1e-9,
        "{}",
        rows[0].cost_usd
    );
}

#[tokio::test]
async fn openai_round_trip_sends_the_image_and_the_two_constants() {
    let rig = Rig::start().await;
    rig.select(AiProvider::OpenAi);
    Mock::given(method("POST"))
        .and(path(OPENAI_PATH))
        .and(header(
            "authorization",
            format!("Bearer {OPENAI_KEY}").as_str(),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(chat_answer(
            Some(r#"{"verdict":"none","confidence":"high"}"#),
            None,
            "stop",
        )))
        .expect(1)
        .mount(&rig.server)
        .await;

    let before = Utc::now();
    let out = rig.check("gpt-5-mini").await.unwrap();
    let after = Utc::now();
    assert_eq!(out, verdict(ExplicitVerdict::None, Confidence::High));

    let bodies = rig.requests_to(OPENAI_PATH).await;
    assert_eq!(bodies.len(), 1);
    assert_eq!(
        bodies[0],
        json!({
            "model": "gpt-5-mini",
            "max_completion_tokens": 64,
            "reasoning_effort": "minimal",
            "messages": [
                {"role": "system", "content": EXPLICIT_SYSTEM_PROMPT},
                {"role": "user", "content": [
                    {"type": "image_url", "image_url": {
                        "url": format!("data:image/jpeg;base64,{}", frame_base64())}},
                    {"type": "text", "text": EXPLICIT_USER_TEXT}
                ]}
            ],
            "response_format": {"type": "json_schema", "json_schema": {
                "name": SCHEMA_NAME, "strict": true, "schema": explicit_schema()}}
        })
    );

    let rows = rig.usage.records();
    assert_eq!(rows.len(), 1);
    assert_on_the_hour(&rows[0], before, after);
    assert_eq!(rows[0].model, "gpt-5-mini");
    assert_eq!((rows[0].input_tokens, rows[0].output_tokens), (610, 14));
}

#[tokio::test]
async fn the_check_follows_the_selected_provider_with_the_same_content() {
    let rig = Rig::start().await;
    let none = r#"{"verdict":"none","confidence":"medium"}"#;
    answer_with(
        &rig.server,
        ANTHROPIC_PATH,
        anthropic_answer(none, "end_turn"),
    )
    .await;
    answer_with(
        &rig.server,
        OPENAI_PATH,
        chat_answer(Some(none), None, "stop"),
    )
    .await;
    answer_with(&rig.server, XAI_PATH, chat_answer(Some(none), None, "stop")).await;
    Mock::given(method("POST"))
        .and(path(UBI_PATH))
        .and(header("x-api-key", LICENSE))
        .and(header("x-ubiqx-plan", "monthly_managed"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("x-ubiqx-cost-usd", "0.000900")
                .set_body_json(anthropic_answer(none, "end_turn")),
        )
        .mount(&rig.server)
        .await;

    let image = frame_base64();
    let constants = [EXPLICIT_SYSTEM_PROMPT, EXPLICIT_USER_TEXT];
    for (provider, model, route) in [
        (AiProvider::Anthropic, "claude-sonnet-5", ANTHROPIC_PATH),
        (AiProvider::Ubi, "ubi-fast", UBI_PATH),
        (AiProvider::OpenAi, "gpt-4.1-mini", OPENAI_PATH),
        (AiProvider::Xai, "grok-4-1-fast-non-reasoning", XAI_PATH),
    ] {
        rig.select(provider);
        assert_eq!(rig.router.selected(), provider);
        let out = rig.check(model).await.unwrap();
        assert_eq!(
            out,
            verdict(ExplicitVerdict::None, Confidence::Medium),
            "{provider:?}"
        );

        let bodies = rig.requests_to(route).await;
        assert_eq!(
            bodies.len(),
            1,
            "{provider:?}: one request, to its own endpoint"
        );
        let body = &bodies[0];
        assert_eq!(body["model"], model);

        // Every string of the body is the image, one of the two constants, or a word of
        // the wire format and the schema. Nothing else travels.
        let mut strings = Vec::new();
        strings_in(body, &mut strings);
        let mut schema_words = Vec::new();
        strings_in(&explicit_schema(), &mut schema_words);
        let mut texts = Vec::new();
        let mut images = 0;
        for s in strings {
            if s.contains(&image) {
                images += 1;
            } else if constants.contains(&s.as_str()) {
                texts.push(s);
            } else {
                assert!(
                    [
                        model,
                        "user",
                        "system",
                        "text",
                        "image",
                        "image_url",
                        "base64",
                        "image/jpeg",
                        "json_schema",
                        "disabled",
                        SCHEMA_NAME
                    ]
                    .contains(&s.as_str())
                        || schema_words.contains(&s),
                    "{provider:?} sent an unexpected string: {s:?}"
                );
            }
        }
        assert_eq!(images, 1, "{provider:?}: one image");
        texts.sort();
        let mut expected = constants.to_vec();
        expected.sort_unstable();
        assert_eq!(
            texts, expected,
            "{provider:?}: the two constants, once each"
        );

        let tokens = match provider {
            AiProvider::OpenAi => &body["max_completion_tokens"],
            _ => &body["max_tokens"],
        };
        assert_eq!(*tokens, 64, "{provider:?}");
    }

    let rows = rig.usage.records();
    assert_eq!(rows.len(), 4);
    assert!(rows.iter().all(|r| r.kind == AiUsageKind::NofapVision));
    assert!(rows
        .iter()
        .all(|r| (r.at.minute(), r.at.second(), r.at.nanosecond()) == (0, 0, 0)));
    assert_eq!(
        rows[1].cost_usd, 0.0009,
        "the managed provider's cost is the one the proxy reports"
    );

    // A provider without a client is "not configured", and nothing is sent anywhere.
    let lonely = RoutingLlmClient::new(vec![], rig.selected.clone());
    let err = LlmExplicitDetector::new(Arc::new(lonely))
        .check(&frame(), "claude-haiku-4-5")
        .await
        .unwrap_err();
    assert!(matches!(err, CoreError::AiNotConfigured));
    assert_eq!(rig.usage.records().len(), 4);
}

// ---------------------------------------------------------------------------------------------
// Refusal and truncation
// ---------------------------------------------------------------------------------------------

#[tokio::test]
async fn a_refusal_from_any_vendor_is_reported_and_never_a_positive() {
    let rig = Rig::start().await;
    // The text of a refused answer is ignored, even when it reads like a verdict.
    answer_with(
        &rig.server,
        ANTHROPIC_PATH,
        anthropic_answer(r#"{"verdict":"explicit","confidence":"high"}"#, "refusal"),
    )
    .await;
    answer_with(
        &rig.server,
        OPENAI_PATH,
        chat_answer(None, Some("I can't help with that."), "stop"),
    )
    .await;
    answer_with(
        &rig.server,
        XAI_PATH,
        chat_answer(
            Some(r#"{"verdict":"explicit","confidence":"high"}"#),
            None,
            "content_filter",
        ),
    )
    .await;

    let before = Utc::now();
    for (provider, model, route) in [
        (AiProvider::Anthropic, "claude-haiku-4-5", ANTHROPIC_PATH),
        (AiProvider::OpenAi, "gpt-5-mini", OPENAI_PATH),
        (AiProvider::Xai, "grok-4", XAI_PATH),
    ] {
        rig.select(provider);
        let out = rig.check(model).await.unwrap();
        assert_eq!(out, REFUSED, "{provider:?}");
        assert!(out.refused);
        assert_ne!(out.verdict, ExplicitVerdict::Explicit);
        assert_eq!(
            rig.requests_to(route).await.len(),
            1,
            "{provider:?}: a refusal is not retried"
        );
    }
    let after = Utc::now();

    // A refused call was still billed, so it is still recorded: by the hour, like any check.
    let rows = rig.usage.records();
    assert_eq!(rows.len(), 3);
    for row in &rows {
        assert_on_the_hour(row, before, after);
    }
}

#[tokio::test]
async fn a_truncated_answer_is_asked_for_once_more_with_a_larger_budget() {
    let rig = Rig::start().await;
    answer_once_with(
        &rig.server,
        ANTHROPIC_PATH,
        anthropic_answer(r#"{"verdict":"expl"#, "max_tokens"),
    )
    .await;
    answer_with(
        &rig.server,
        ANTHROPIC_PATH,
        anthropic_answer(
            r#"{"verdict":"explicit","confidence":"medium"}"#,
            "end_turn",
        ),
    )
    .await;

    let out = rig.check("claude-haiku-4-5").await.unwrap();
    assert_eq!(out, verdict(ExplicitVerdict::Explicit, Confidence::Medium));
    let bodies = rig.requests_to(ANTHROPIC_PATH).await;
    assert_eq!(
        bodies
            .iter()
            .map(|b| b["max_tokens"].clone())
            .collect::<Vec<_>>(),
        [json!(64), json!(256)]
    );
    assert_eq!(bodies[0]["messages"], bodies[1]["messages"]);
    assert_eq!(rig.usage.records().len(), 2, "both calls were billed");

    // The same through an OpenAI-compatible vendor, where truncation is `length`; a second
    // truncation is an error, not a verdict.
    rig.select(AiProvider::OpenAi);
    answer_with(
        &rig.server,
        OPENAI_PATH,
        chat_answer(
            Some(r#"{"verdict":"explicit","confidence":"high"}"#),
            None,
            "length",
        ),
    )
    .await;
    let err = rig.check("o4-mini").await.unwrap_err();
    assert!(matches!(err, CoreError::Ai(_)), "{err}");
    let bodies = rig.requests_to(OPENAI_PATH).await;
    assert_eq!(
        bodies
            .iter()
            .map(|b| b["max_completion_tokens"].clone())
            .collect::<Vec<_>>(),
        [json!(64), json!(256)]
    );
}

#[tokio::test]
async fn an_answer_outside_the_schema_is_unsure() {
    let rig = Rig::start().await;
    answer_with(
        &rig.server,
        ANTHROPIC_PATH,
        anthropic_answer(
            r#"{"verdict":"explicit","confidence":"high","description":"a page of thumbnails"}"#,
            "end_turn",
        ),
    )
    .await;
    assert_eq!(rig.check("claude-haiku-4-5").await.unwrap(), UNSURE);
    assert_eq!(rig.requests_to(ANTHROPIC_PATH).await.len(), 1);
}

// ---------------------------------------------------------------------------------------------
// What is logged
// ---------------------------------------------------------------------------------------------

#[tokio::test]
async fn a_screen_check_leaves_no_log_line_through_either_client() {
    let rig = Rig::start().await;
    answer_with(
        &rig.server,
        ANTHROPIC_PATH,
        anthropic_answer(r#"{"verdict":"explicit","confidence":"high"}"#, "end_turn"),
    )
    .await;
    answer_with(
        &rig.server,
        OPENAI_PATH,
        chat_answer(
            Some(r#"{"verdict":"suggestive","confidence":"low"}"#),
            None,
            "stop",
        ),
    )
    .await;
    let (capture, _guard) = Capture::start();

    for (provider, model, kind_of_client) in [
        (AiProvider::Anthropic, "claude-haiku-4-5", "anthropic"),
        (AiProvider::OpenAi, "gpt-5-mini", "openai"),
    ] {
        rig.select(provider);

        // The capture is live: a productivity call through the same client is logged as
        // before, with its kind, and its request body shows the image as a size.
        capture.clear();
        let before = Utc::now();
        rig.router
            .complete(&vision_request(model, AiUsageKind::Vision))
            .await
            .unwrap();
        let after = Utc::now();
        let logged = capture.text();
        assert!(
            logged.contains("call completed"),
            "{kind_of_client}: {logged}"
        );
        assert!(
            logged.contains("kind=\"vision\""),
            "{kind_of_client}: {logged}"
        );
        assert!(
            logged.contains("request body"),
            "{kind_of_client}: {logged}"
        );
        assert!(
            logged.contains("response text"),
            "{kind_of_client}: {logged}"
        );
        assert!(
            logged.contains("<image: 3000 bytes>"),
            "{kind_of_client}: {logged}"
        );
        assert!(
            !logged.contains(&frame_base64()[..64]),
            "{kind_of_client}: the image itself is not in the trace"
        );
        let row = rig.usage.records().pop().unwrap();
        assert_eq!(row.kind, AiUsageKind::Vision);
        assert!(
            row.at >= before && row.at <= after,
            "{kind_of_client}: every other kind keeps its exact time"
        );

        // One check: nothing, at any level.
        capture.clear();
        rig.check(model).await.unwrap();
        assert_eq!(capture.text(), "", "{kind_of_client}: one check");

        // Twenty more: still nothing, so the log does not grow with the number of checks.
        for _ in 0..20 {
            rig.check(model).await.unwrap();
        }
        assert_eq!(capture.text(), "", "{kind_of_client}: twenty checks");
        assert_transport_says_nothing(&capture);
    }
}

#[tokio::test]
async fn refused_and_truncated_checks_leave_no_log_line_either() {
    let rig = Rig::start().await;
    answer_with(&rig.server, ANTHROPIC_PATH, anthropic_answer("", "refusal")).await;
    answer_with(
        &rig.server,
        OPENAI_PATH,
        chat_answer(Some("{"), None, "length"),
    )
    .await;
    answer_with(&rig.server, XAI_PATH, chat_answer(None, Some("no"), "stop")).await;
    let (capture, _guard) = Capture::start();

    assert_eq!(rig.check("claude-haiku-4-5").await.unwrap(), REFUSED);
    rig.select(AiProvider::Xai);
    assert_eq!(rig.check("grok-4").await.unwrap(), REFUSED);
    rig.select(AiProvider::OpenAi);
    assert!(rig.check("gpt-5-mini").await.is_err(), "truncated twice");
    assert_eq!(
        capture.text(),
        "",
        "a refusal marked in the log would be a line per declined screen"
    );
    assert_transport_says_nothing(&capture);

    // The same outcomes of a productivity call are logged as before.
    let truncated = rig
        .router
        .complete(&vision_request("gpt-5-mini", AiUsageKind::Classify))
        .await
        .unwrap();
    assert!(truncated.is_truncated());
    let logged = capture.text();
    assert!(logged.contains("output truncated"), "{logged}");
    assert!(logged.contains("stop_reason=\"max_tokens\""), "{logged}");
}

#[tokio::test]
async fn a_check_declined_with_an_http_400_is_a_quiet_refusal() {
    let rig = Rig::start().await;
    rig.select(AiProvider::OpenAi);
    Mock::given(method("POST"))
        .and(path(OPENAI_PATH))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "error": {
                "message": "Your request was rejected as a result of our safety system.",
                "type": "invalid_request_error",
                "param": null,
                "code": "content_policy_violation"
            }
        })))
        .mount(&rig.server)
        .await;
    let (capture, _guard) = Capture::start();

    let out = rig.check("gpt-5-mini").await.unwrap();
    assert_eq!(
        out, REFUSED,
        "declined by the vendor: a refusal, not an outage"
    );
    assert_eq!(rig.requests_to(OPENAI_PATH).await.len(), 1, "not retried");
    assert!(rig.usage.records().is_empty(), "nothing was billed");
    assert_eq!(
        capture.text(),
        "",
        "a line here would mark the screens a provider declined to look at"
    );
    assert_transport_says_nothing(&capture);

    // A productivity call declined the same way is an `AiRefused` too, and is logged.
    let err = rig
        .router
        .complete(&vision_request("gpt-5-mini", AiUsageKind::Vision))
        .await
        .unwrap_err();
    assert!(matches!(err, CoreError::AiRefused), "{err}");
    let logged = capture.text();
    assert!(
        logged.contains("request declined for its content"),
        "{logged}"
    );
    assert!(logged.contains("kind=\"vision\""), "{logged}");
}

#[tokio::test]
async fn a_failed_check_is_logged_without_its_kind() {
    let rig = Rig::start().await;
    Mock::given(method("POST"))
        .and(path(ANTHROPIC_PATH))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "type": "error",
            "error": {"type": "invalid_request_error", "message": "image too large"}
        })))
        .mount(&rig.server)
        .await;
    Mock::given(method("POST"))
        .and(path(OPENAI_PATH))
        .respond_with(ResponseTemplate::new(503).set_body_string("overloaded"))
        .mount(&rig.server)
        .await;
    let (capture, _guard) = Capture::start();

    let err = rig.check("claude-haiku-4-5").await.unwrap_err();
    assert!(matches!(err, CoreError::Invalid(_)), "{err}");
    rig.select(AiProvider::OpenAi);
    let err = rig.check("gpt-5-mini").await.unwrap_err();
    assert!(matches!(err, CoreError::Ai(_)), "{err}");

    let logged = capture.text();
    assert!(
        logged.contains("request failed") && logged.contains("giving up after retries"),
        "a failure is still visible: {logged}"
    );
    for word in ["nofap", "kind", "explicit", "verdict", "screen"] {
        assert!(
            !logged.to_lowercase().contains(word),
            "a failure line says nothing about what the call was for ({word}): {logged}"
        );
    }
    assert!(!logged.contains(&frame_base64()[..64]));
    assert!(rig.usage.records().is_empty(), "nothing was billed");
}
