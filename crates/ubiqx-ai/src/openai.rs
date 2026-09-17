//! OpenAI-compatible Chat Completions adapter, used for both OpenAI and xAI (Grok).
//!
//! [`OpenAiCompatClient`] serialises an [`LlmRequest`] into `POST {base}/chat/completions`,
//! retries transient failures with the same policy as [`crate::client::AnthropicClient`]
//! (see [`crate::retry`]), maps the vendor's error envelope onto [`CoreError`] and records an
//! [`AiUsage`] with a cost estimate after every successful call. The two vendors share the
//! wire format; the differences are captured in one place, the request policy below.
//!
//! # Per-vendor request policy
//!
//! | field                         | OpenAI                                   | xAI                              |
//! |-------------------------------|------------------------------------------|----------------------------------|
//! | base URL                      | `https://api.openai.com/v1`              | `https://api.x.ai/v1`            |
//! | auth                          | `Authorization: Bearer <key>`            | same                             |
//! | token cap                     | `max_completion_tokens`                  | `max_tokens`                     |
//! | `temperature`                 | never sent (gpt-5 rejects it)            | never sent                       |
//! | `reasoning_effort`            | `gpt-5*`: `"minimal"`; `o*`: `"low"`     | `grok-3-mini*`: `"low"`          |
//! |                               | `gpt-4*` and others: omitted             | `grok-4*` and others: omitted    |
//! | structured output             | `response_format.json_schema` (strict)   | same                             |
//! | images                        | `image_url` with a `data:` URL           | same                             |
//!
//! `reasoning_effort` is only ever sent for the high-volume, latency-sensitive calls
//! ([`AiUsageKind::Classify`] and [`AiUsageKind::Vision`]); reports and advice
//! ([`AiUsageKind::Report`], [`AiUsageKind::Advice`]) run at the model's default effort.
//! [`crate::client::SystemBlock::cache`] and [`LlmRequest::disable_thinking`] are Anthropic-only and ignored
//! here; system blocks are joined with blank lines into one `system` message.
//!
//! # Usage accounting
//!
//! These vendors count cached prompt tokens inside `usage.prompt_tokens`; the recorded
//! [`AiUsage`] keeps the Anthropic semantics instead (`input_tokens` = uncached input,
//! `cache_read_tokens` = `prompt_tokens_details.cached_tokens`, `cache_write_tokens` = 0), so
//! the cost ledger prices cached tokens once, at 10 % of the input price.
//! `output_tokens` is `usage.completion_tokens`, which includes reasoning tokens.
//!
//! # Error mapping
//!
//! * missing key → [`CoreError::AiNotConfigured`] without any request;
//! * 401/403 → `CoreError::Ai("invalid api key …")`, never retried;
//! * 404, or error code `model_not_found` → [`CoreError::AiRejected`] (model unavailable);
//! * 402, or 429 with type/code `insufficient_quota` or code `billing_hard_limit_reached` →
//!   [`CoreError::AiRejected`] (billing), never retried;
//! * other 429 → retried, honouring `retry-after` or a "try again in Ns" hint in the message,
//!   then [`CoreError::RateLimited`];
//! * 400/413/422 → [`CoreError::Invalid`];
//! * 408/409/5xx/timeouts/network errors → retried with backoff, then [`CoreError::Ai`].
//!
//! Error messages meant for the user are written in pt-BR and keep the vendor's own text.

use std::fmt;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use chrono::Utc;
use serde::Deserialize;
use serde_json::{json, Value};
use tracing::{debug, info, trace, warn};
use ubiqx_core::ports::{AiUsage, AiUsageKind, UsageRepo};
use ubiqx_core::{AiProvider, CoreError, CoreResult};

use crate::client::{
    validate_request, ApiKeySource, LlmClient, LlmContent, LlmRequest, LlmResponse, LlmRole,
    StopReason, DEFAULT_BACKOFF_BASE, DEFAULT_BACKOFF_CAP, DEFAULT_MAX_ATTEMPTS, DEFAULT_TIMEOUT,
};
use crate::pricing;
use crate::retry::{self, Outcome, RetryPolicy, RetryableError};

/// Production endpoint of OpenAI.
pub const OPENAI_BASE_URL: &str = "https://api.openai.com/v1";
/// Production endpoint of xAI.
pub const XAI_BASE_URL: &str = "https://api.x.ai/v1";
/// Name of the JSON schema sent in `response_format` (the vendors require one).
pub const SCHEMA_NAME: &str = "ubiqx_output";

/// Model ids containing any of these fragments are not chat models and are dropped from
/// [`OpenAiCompatClient::list_models`].
const NON_CHAT_FRAGMENTS: &[&str] = &[
    "embedding",
    "tts",
    "whisper",
    "dall-e",
    "audio",
    "realtime",
    "image",
    "moderation",
    "transcribe",
    "sora",
    "search",
    "codex",
];

// ---------------------------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------------------------

/// Client configuration for one OpenAI-compatible vendor. Build it with
/// [`OpenAiCompatConfig::for_provider`] (production endpoint, same retry policy as
/// [`crate::client::AnthropicConfig::default`]) or [`OpenAiCompatConfig::with_base_url`].
#[derive(Debug, Clone)]
pub struct OpenAiCompatConfig {
    /// [`AiProvider::OpenAi`] or [`AiProvider::Xai`]; decides the request policy.
    pub provider: AiProvider,
    /// Base URL including the API version segment (`https://api.openai.com/v1`).
    pub base_url: String,
    /// Default total timeout of one HTTP request (overridable per [`LlmRequest`]).
    pub timeout: Duration,
    /// Attempts per call, including the first one. `0` behaves like `1`.
    pub max_attempts: u32,
    /// Wait before the first retry; doubled on every subsequent retry.
    pub backoff_base: Duration,
    /// Upper bound of any single wait. A `retry-after` above it aborts the call instead.
    pub backoff_cap: Duration,
}

impl OpenAiCompatConfig {
    /// Production base URL of an OpenAI-compatible vendor; `None` for Anthropic, which has
    /// its own client.
    pub fn default_base_url(provider: AiProvider) -> Option<&'static str> {
        match provider {
            AiProvider::OpenAi => Some(OPENAI_BASE_URL),
            AiProvider::Xai => Some(XAI_BASE_URL),
            AiProvider::Anthropic => None,
        }
    }

    /// Production configuration for `provider`.
    ///
    /// # Panics
    ///
    /// When `provider` is [`AiProvider::Anthropic`]: that vendor speaks the Messages API and
    /// is served by [`crate::client::AnthropicClient`]. Use [`Self::default_base_url`] to
    /// check first when the provider comes from user input.
    pub fn for_provider(provider: AiProvider) -> Self {
        let base_url = Self::default_base_url(provider)
            .unwrap_or_else(|| panic!("{} is not an OpenAI-compatible provider", provider.id()));
        Self::with_base_url(provider, base_url)
    }

    /// Configuration pointing at another base URL (tests, proxies). The URL must include the
    /// version segment when the vendor has one (`…/v1`).
    pub fn with_base_url(provider: AiProvider, base_url: impl Into<String>) -> Self {
        Self {
            provider,
            base_url: base_url.into(),
            timeout: DEFAULT_TIMEOUT,
            max_attempts: DEFAULT_MAX_ATTEMPTS,
            backoff_base: DEFAULT_BACKOFF_BASE,
            backoff_cap: DEFAULT_BACKOFF_CAP,
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Model-dependent request policy
// ---------------------------------------------------------------------------------------------

/// Name of the output-token cap field: OpenAI's gpt-5 and o-series reject `max_tokens`,
/// xAI only knows `max_tokens`.
pub fn max_tokens_field(provider: AiProvider) -> &'static str {
    match provider {
        AiProvider::OpenAi => "max_completion_tokens",
        _ => "max_tokens",
    }
}

fn is_o_series(model: &str) -> bool {
    model.starts_with('o') && model[1..].starts_with(|c: char| c.is_ascii_digit())
}

/// The `reasoning_effort` value to send for `model` on a call of kind `kind`, or `None` to
/// omit the field. Only classification and vision calls get one (the cheapest setting the
/// model accepts); other kinds and models that reject the field get nothing.
pub fn reasoning_effort_for(
    provider: AiProvider,
    model: &str,
    kind: AiUsageKind,
) -> Option<&'static str> {
    if !matches!(kind, AiUsageKind::Classify | AiUsageKind::Vision) {
        return None;
    }
    let m = model.trim().to_ascii_lowercase();
    match provider {
        AiProvider::OpenAi if m.starts_with("gpt-5") => Some("minimal"),
        AiProvider::OpenAi if is_o_series(&m) => Some("low"),
        AiProvider::Xai if m.starts_with("grok-3-mini") => Some("low"),
        _ => None,
    }
}

// ---------------------------------------------------------------------------------------------
// JSON body
// ---------------------------------------------------------------------------------------------

fn content_part(c: &LlmContent) -> Value {
    match c {
        LlmContent::Text(text) => json!({"type": "text", "text": text}),
        LlmContent::Image { media_type, base64 } => json!({
            "type": "image_url",
            "image_url": {"url": format!("data:{media_type};base64,{base64}")}
        }),
    }
}

/// The exact JSON body sent to `POST /chat/completions` for `req` at `provider`. Public so
/// the shape can be inspected and tested without a server. Never adds `temperature`.
pub fn request_body(provider: AiProvider, req: &LlmRequest) -> Value {
    let system = req
        .system
        .iter()
        .map(|b| b.text.trim())
        .filter(|t| !t.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");

    let mut messages = Vec::with_capacity(req.messages.len() + 1);
    if !system.is_empty() {
        messages.push(json!({"role": "system", "content": system}));
    }
    for m in &req.messages {
        let content = match m.role {
            LlmRole::User => Value::Array(m.content.iter().map(content_part).collect()),
            // Assistant turns carry text only; the vendors take it as a plain string.
            LlmRole::Assistant => Value::String(
                m.content
                    .iter()
                    .filter_map(|c| match c {
                        LlmContent::Text(t) => Some(t.as_str()),
                        LlmContent::Image { .. } => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n\n"),
            ),
        };
        messages.push(json!({"role": m.role, "content": content}));
    }

    let mut body = json!({
        "model": req.model,
        "messages": messages,
    });
    body[max_tokens_field(provider)] = json!(req.max_tokens);
    if let Some(effort) = reasoning_effort_for(provider, &req.model, req.usage_kind) {
        body["reasoning_effort"] = json!(effort);
    }
    if let Some(schema) = &req.json_schema {
        body["response_format"] = json!({
            "type": "json_schema",
            "json_schema": {"name": SCHEMA_NAME, "strict": true, "schema": schema}
        });
    }
    body
}

// ---------------------------------------------------------------------------------------------
// Response parsing
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct ChatResponse {
    #[serde(default)]
    model: String,
    #[serde(default)]
    choices: Vec<Choice>,
    #[serde(default)]
    usage: ChatUsage,
}

#[derive(Debug, Deserialize)]
struct Choice {
    #[serde(default)]
    message: ChoiceMessage,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct ChoiceMessage {
    #[serde(default)]
    content: Option<MessageContent>,
    #[serde(default)]
    refusal: Option<String>,
}

/// `content` is a string in practice; some compatible servers send an array of text parts.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum MessageContent {
    Text(String),
    Parts(Vec<ContentPart>),
}

#[derive(Debug, Deserialize)]
struct ContentPart {
    #[serde(default)]
    text: Option<String>,
}

impl MessageContent {
    fn into_text(self) -> String {
        match self {
            MessageContent::Text(t) => t,
            MessageContent::Parts(parts) => parts.into_iter().filter_map(|p| p.text).collect(),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
struct ChatUsage {
    #[serde(default)]
    prompt_tokens: u32,
    #[serde(default)]
    completion_tokens: u32,
    #[serde(default)]
    prompt_tokens_details: Option<PromptTokensDetails>,
}

#[derive(Debug, Default, Deserialize)]
struct PromptTokensDetails {
    #[serde(default)]
    cached_tokens: u32,
}

impl ChatUsage {
    fn cached(&self) -> u32 {
        self.prompt_tokens_details
            .as_ref()
            .map_or(0, |d| d.cached_tokens)
            .min(self.prompt_tokens)
    }
}

/// Maps `finish_reason` (and a non-empty `refusal`) onto the vendor-neutral [`StopReason`].
fn stop_reason(finish_reason: Option<&str>, refusal: Option<&str>) -> StopReason {
    if refusal.is_some_and(|r| !r.trim().is_empty()) {
        return StopReason::Refusal;
    }
    match finish_reason {
        None | Some("stop") => StopReason::EndTurn,
        Some("length") => StopReason::MaxTokens,
        Some("content_filter") => StopReason::Refusal,
        Some(other) => StopReason::Other(other.to_string()),
    }
}

#[derive(Debug, Deserialize)]
struct ErrorEnvelope {
    /// `{message, type, code}` at OpenAI; xAI sometimes sends a bare string here with the
    /// code as a sibling field.
    error: Option<Value>,
    #[serde(default)]
    code: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct ErrorBody {
    #[serde(default)]
    message: Option<String>,
    #[serde(rename = "type", default)]
    kind: Option<String>,
    #[serde(default)]
    code: Option<Value>,
}

/// A parsed `{error: {message, type, code}}` (or `{error: "…", code: "…"}`) envelope.
#[derive(Debug, Default, PartialEq, Eq)]
struct ApiError {
    message: String,
    kind: String,
    code: String,
}

fn code_text(code: Option<Value>) -> String {
    match code {
        Some(Value::String(s)) => s,
        Some(Value::Number(n)) => n.to_string(),
        _ => String::new(),
    }
}

fn parse_error(body: &str) -> Option<ApiError> {
    let env = serde_json::from_str::<ErrorEnvelope>(body).ok()?;
    let parsed = match env.error? {
        Value::String(message) => ApiError {
            message,
            kind: String::new(),
            code: code_text(env.code),
        },
        object @ Value::Object(_) => {
            let err: ErrorBody = serde_json::from_value(object).ok()?;
            ApiError {
                message: err.message.unwrap_or_default(),
                kind: err.kind.unwrap_or_default(),
                code: code_text(err.code.or(env.code)),
            }
        }
        _ => return None,
    };
    (parsed != ApiError::default()).then_some(parsed)
}

/// Extracts a short, loggable message from an API error body.
fn error_message(body: &str) -> String {
    if let Some(err) = parse_error(body) {
        let label = if err.code.is_empty() {
            err.kind
        } else {
            err.code
        };
        return format!("{label}: {}", err.message)
            .trim_matches([':', ' '])
            .to_string();
    }
    let trimmed: String = body.chars().take(200).collect();
    trimmed.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Seconds suggested by a rate-limit message such as "Please try again in 1.2s" or
/// "… in 350ms" (rounded up; `None` when there is no such hint).
fn retry_hint_secs(message: &str) -> Option<u64> {
    let lower = message.to_ascii_lowercase();
    let idx = lower.find("try again in ")?;
    let rest = lower[idx + "try again in ".len()..].trim_start();
    let num_len = rest
        .char_indices()
        .take_while(|(_, c)| c.is_ascii_digit() || *c == '.')
        .map(|(i, c)| i + c.len_utf8())
        .last()?;
    let number: f64 = rest[..num_len].parse().ok()?;
    let unit = rest[num_len..].trim_start();
    let secs = if unit.starts_with("ms") {
        number / 1000.0
    } else if unit.starts_with('m') && !unit.starts_with("ms") {
        number * 60.0
    } else if unit.starts_with('h') {
        number * 3600.0
    } else {
        number
    };
    (secs.is_finite() && secs >= 0.0).then(|| secs.ceil() as u64)
}

/// Provider-side rejections that are neither transient nor the request's fault: a billing
/// problem or a model the account cannot use. The user has to act, so the message is written
/// for them (pt-BR), keeping the vendor's own text.
fn rejection(provider: AiProvider, code: u16, body: &str) -> Option<CoreError> {
    let err = parse_error(body).unwrap_or_default();
    let detail = if err.message.is_empty() {
        format!("HTTP {code}")
    } else {
        err.message.clone()
    };
    let billing = code == 402
        || err.kind == "insufficient_quota"
        || err.code == "insufficient_quota"
        || err.code == "billing_hard_limit_reached";
    if billing {
        return Some(CoreError::AiRejected(format!(
            "cobrança: sua conta {} não tem créditos ou cota disponível ({detail}). Verifique o faturamento em {}.",
            provider.label(),
            provider.console_url()
        )));
    }
    if code == 404 || err.code == "model_not_found" {
        return Some(CoreError::AiRejected(format!(
            "modelo não encontrado ou indisponível para a sua conta {} ({detail}). Verifique o modelo em Configurações → IA.",
            provider.label()
        )));
    }
    None
}

fn classify_response(
    provider: AiProvider,
    status: reqwest::StatusCode,
    retry_after: Option<u64>,
    body: String,
) -> Outcome {
    let code = status.as_u16();
    match code {
        200..=299 => Outcome::Ok(body),
        401 | 403 => Outcome::Fatal(CoreError::Ai(format!(
            "invalid api key (HTTP {code}): {}",
            error_message(&body)
        ))),
        400..=499 if rejection(provider, code, &body).is_some() => {
            Outcome::Fatal(rejection(provider, code, &body).expect("checked above"))
        }
        429 => {
            let hint = retry_after
                .or_else(|| parse_error(&body).and_then(|e| retry_hint_secs(&e.message)));
            Outcome::Retry {
                error: RetryableError::RateLimited {
                    retry_after_secs: hint,
                },
                wait: hint.map(Duration::from_secs),
            }
        }
        400 | 413 | 422 => Outcome::Fatal(CoreError::Invalid(format!(
            "HTTP {code}: {}",
            error_message(&body)
        ))),
        408 | 409 | 500..=599 => Outcome::Retry {
            error: RetryableError::Transient(format!("HTTP {code}: {}", error_message(&body))),
            wait: None,
        },
        _ => Outcome::Fatal(CoreError::Ai(format!(
            "unexpected HTTP {code}: {}",
            error_message(&body)
        ))),
    }
}

/// Chat-capable model ids of a `GET /models` body, sorted and de-duplicated.
fn chat_model_ids(models_body: &str) -> Vec<String> {
    let Ok(listing) = serde_json::from_str::<Value>(models_body) else {
        return Vec::new();
    };
    let mut ids: Vec<String> = listing
        .get("data")
        .and_then(Value::as_array)
        .map(|models| {
            models
                .iter()
                .filter_map(|m| m.get("id").and_then(Value::as_str))
                .filter(|id| is_chat_model(id))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    ids.sort();
    ids.dedup();
    ids
}

/// Whether a model id looks like a chat model: ids of embedding, speech, image, moderation,
/// realtime, search and codex models are not.
pub fn is_chat_model(id: &str) -> bool {
    let lower = id.to_ascii_lowercase();
    !lower.trim().is_empty() && !NON_CHAT_FRAGMENTS.iter().any(|f| lower.contains(f))
}

// ---------------------------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------------------------

/// HTTP client for an OpenAI-compatible Chat Completions API. Cheap to clone behind an `Arc`.
pub struct OpenAiCompatClient {
    http: reqwest::Client,
    config: OpenAiCompatConfig,
    keys: Arc<dyn ApiKeySource>,
    usage: Arc<dyn UsageRepo>,
}

impl fmt::Debug for OpenAiCompatClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OpenAiCompatClient")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl OpenAiCompatClient {
    /// Builds the client. Fails with [`CoreError::Invalid`] when `config.provider` is
    /// [`AiProvider::Anthropic`] and with [`CoreError::Ai`] when the TLS backend cannot be
    /// initialised.
    pub fn new(
        config: OpenAiCompatConfig,
        keys: Arc<dyn ApiKeySource>,
        usage: Arc<dyn UsageRepo>,
    ) -> CoreResult<Self> {
        if OpenAiCompatConfig::default_base_url(config.provider).is_none() {
            return Err(CoreError::Invalid(format!(
                "{} is not an OpenAI-compatible provider",
                config.provider.id()
            )));
        }
        let http = reqwest::Client::builder()
            .timeout(config.timeout)
            .user_agent(concat!("ubiqx/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| CoreError::Ai(format!("cannot build http client: {e}")))?;
        Ok(Self {
            http,
            config,
            keys,
            usage,
        })
    }

    pub fn config(&self) -> &OpenAiCompatConfig {
        &self.config
    }

    /// The vendor this client talks to.
    pub fn provider(&self) -> AiProvider {
        self.config.provider
    }

    fn vendor(&self) -> &'static str {
        self.config.provider.id()
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.config.base_url.trim_end_matches('/'), path)
    }

    fn key(&self) -> CoreResult<String> {
        self.keys.api_key().ok_or(CoreError::AiNotConfigured)
    }

    fn retry_policy(&self) -> RetryPolicy {
        RetryPolicy {
            max_attempts: self.config.max_attempts,
            backoff_base: self.config.backoff_base,
            backoff_cap: self.config.backoff_cap,
        }
    }

    async fn send_with_retry<F>(&self, what: &'static str, build: F) -> CoreResult<String>
    where
        F: Fn() -> reqwest::RequestBuilder,
    {
        let provider = self.config.provider;
        retry::send_with_retry(
            self.retry_policy(),
            self.vendor(),
            what,
            build,
            move |status, retry_after, body| classify_response(provider, status, retry_after, body),
        )
        .await
    }

    /// Sends one chat completion, retrying transient failures, and records its usage.
    ///
    /// Returns [`CoreError::AiNotConfigured`] without any HTTP call when no key is available;
    /// otherwise errors map as described in the module docs. A refusal (`message.refusal`
    /// or `finish_reason = content_filter`) yields [`CoreError::AiRefused`] after the usage
    /// is recorded; `finish_reason = length` is returned as [`StopReason::MaxTokens`].
    pub async fn complete(&self, req: &LlmRequest) -> CoreResult<LlmResponse> {
        validate_request(req)?;
        let key = self.key()?;
        let provider = self.config.provider;
        let vendor = self.vendor();
        let body = request_body(provider, req);
        let url = self.url("/chat/completions");
        let timeout = req.timeout.unwrap_or(self.config.timeout);
        let kind = req.usage_kind.as_str();

        debug!(
            vendor,
            model = %req.model,
            kind,
            max_tokens = req.max_tokens,
            messages = req.messages.len(),
            structured = req.json_schema.is_some(),
            reasoning_effort = body.get("reasoning_effort").and_then(serde_json::Value::as_str),
            "sending chat completion"
        );
        trace!(vendor, body = %body, "request body");

        let started = Instant::now();
        let text = self
            .send_with_retry("chat completion", || {
                self.http
                    .post(&url)
                    .bearer_auth(&key)
                    .header("content-type", "application/json")
                    .timeout(timeout)
                    .json(&body)
            })
            .await?;

        let parsed: ChatResponse = serde_json::from_str(&text)
            .map_err(|e| CoreError::Ai(format!("unparseable chat completion response: {e}")))?;
        let (answer, stop_reason) = match parsed.choices.into_iter().next() {
            Some(choice) => {
                let stop = stop_reason(
                    choice.finish_reason.as_deref(),
                    choice.message.refusal.as_deref(),
                );
                let answer = choice
                    .message
                    .content
                    .map(MessageContent::into_text)
                    .unwrap_or_default();
                (answer, stop)
            }
            None => (String::new(), StopReason::EndTurn),
        };
        let cache_read = parsed.usage.cached();
        let input_tokens = parsed.usage.prompt_tokens - cache_read;
        let output_tokens = parsed.usage.completion_tokens;
        let cost_usd = pricing::estimate_cost_usd_for_provider(
            provider,
            &req.model,
            input_tokens,
            output_tokens,
            cache_read,
            0,
        );

        let usage = AiUsage {
            at: Utc::now(),
            kind: req.usage_kind,
            model: req.model.clone(),
            input_tokens,
            output_tokens,
            cache_read_tokens: cache_read,
            cache_write_tokens: 0,
            cost_usd,
        };
        if let Err(e) = self.usage.record(&usage) {
            warn!(vendor, error = %e, "could not record usage");
        }
        info!(
            vendor,
            model = %req.model,
            kind,
            input_tokens,
            output_tokens,
            cache_read_tokens = cache_read,
            cost_usd,
            stop_reason = stop_reason.as_str(),
            elapsed_ms = started.elapsed().as_millis() as u64,
            "call completed"
        );
        trace!(vendor, answer = %answer, "response text");

        if stop_reason == StopReason::Refusal {
            // A per-content decision by the model, not an outage: the caller must not retry
            // the same payload.
            return Err(CoreError::AiRefused);
        }
        if stop_reason == StopReason::MaxTokens {
            warn!(vendor, model = %req.model, kind, max_tokens = req.max_tokens, "output truncated");
        }

        Ok(LlmResponse {
            text: answer,
            model: if parsed.model.is_empty() {
                req.model.clone()
            } else {
                parsed.model
            },
            input_tokens,
            output_tokens,
            cache_read_tokens: cache_read,
            cache_write_tokens: 0,
            stop_reason,
        })
    }

    /// Checks the configured key with `GET /models`, then probes billing and model access
    /// with a one-token chat completion on `probe_model` (the models listing succeeds for an
    /// account without credits, so a key would otherwise look fine at setup and fail on every
    /// classification). Nothing is recorded as usage.
    ///
    /// `Ok(())` means the key is accepted and `probe_model` can be billed; 401/403 yield
    /// `CoreError::Ai("invalid api key …")`; a billing rejection or an unavailable
    /// `probe_model` yields [`CoreError::AiRejected`] with a message written for the user; a
    /// missing key yields [`CoreError::AiNotConfigured`] without any request. Any other
    /// outcome of the probe (rate limits, outages) is not held against the key. A blank
    /// `probe_model` skips the probe.
    pub async fn validate_key(&self, probe_model: &str) -> CoreResult<()> {
        let key = self.key()?;
        let vendor = self.vendor();
        let url = self.url("/models");
        let timeout = self.config.timeout;
        self.send_with_retry("models", || {
            self.http.get(&url).bearer_auth(&key).timeout(timeout)
        })
        .await?;

        let model = probe_model.trim();
        if model.is_empty() {
            debug!(vendor, "no probe model given; skipping the billing probe");
            return Ok(());
        }
        let url = self.url("/chat/completions");
        let mut body = json!({
            "model": model,
            "messages": [{"role": "user", "content": "ping"}],
        });
        body[max_tokens_field(self.config.provider)] = json!(1);
        match self
            .send_with_retry("key probe", || {
                self.http
                    .post(&url)
                    .bearer_auth(&key)
                    .header("content-type", "application/json")
                    .timeout(timeout)
                    .json(&body)
            })
            .await
        {
            Ok(_) => Ok(()),
            Err(err @ CoreError::AiRejected(_)) => Err(err),
            Err(e) => {
                debug!(vendor, error = %e, "billing probe inconclusive; accepting the key");
                Ok(())
            }
        }
    }

    /// Chat-capable model ids the key can use, from `GET /models`, sorted. Ids of embedding,
    /// speech, image, moderation, realtime, search and codex models are dropped (see
    /// [`is_chat_model`]). Errors map like [`Self::validate_key`]'s listing step.
    pub async fn list_models(&self) -> CoreResult<Vec<String>> {
        let key = self.key()?;
        let url = self.url("/models");
        let timeout = self.config.timeout;
        let listing = self
            .send_with_retry("models", || {
                self.http.get(&url).bearer_auth(&key).timeout(timeout)
            })
            .await?;
        let ids = chat_model_ids(&listing);
        debug!(
            vendor = self.vendor(),
            count = ids.len(),
            "listed chat models"
        );
        Ok(ids)
    }
}

#[async_trait]
impl LlmClient for OpenAiCompatClient {
    async fn complete(&self, req: &LlmRequest) -> CoreResult<LlmResponse> {
        OpenAiCompatClient::complete(self, req).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::{LlmMessage, SystemBlock};
    use reqwest::StatusCode;

    fn request(model: &str, kind: AiUsageKind) -> LlmRequest {
        LlmRequest {
            model: model.into(),
            system: vec![
                SystemBlock::new("stable instructions", false),
                SystemBlock::new("  ", false),
                SystemBlock::new("profile", true),
            ],
            messages: vec![LlmMessage::user(vec![
                LlmContent::Text("hello".into()),
                LlmContent::Image {
                    media_type: "image/jpeg".into(),
                    base64: "AAAA".into(),
                },
            ])],
            max_tokens: 100,
            json_schema: Some(
                json!({"type": "object", "properties": {}, "required": [], "additionalProperties": false}),
            ),
            disable_thinking: true,
            usage_kind: kind,
            timeout: None,
        }
    }

    #[test]
    fn config_for_provider() {
        let c = OpenAiCompatConfig::for_provider(AiProvider::OpenAi);
        assert_eq!(c.base_url, OPENAI_BASE_URL);
        assert_eq!(c.provider, AiProvider::OpenAi);
        assert_eq!(c.max_attempts, DEFAULT_MAX_ATTEMPTS);
        let c = OpenAiCompatConfig::for_provider(AiProvider::Xai);
        assert_eq!(c.base_url, XAI_BASE_URL);
        assert_eq!(
            OpenAiCompatConfig::default_base_url(AiProvider::Anthropic),
            None
        );
    }

    #[test]
    #[should_panic(expected = "not an OpenAI-compatible provider")]
    fn config_for_anthropic_panics() {
        let _ = OpenAiCompatConfig::for_provider(AiProvider::Anthropic);
    }

    #[test]
    fn openai_body_shape() {
        let body = request_body(
            AiProvider::OpenAi,
            &request("gpt-5-mini", AiUsageKind::Classify),
        );
        assert_eq!(body["model"], "gpt-5-mini");
        assert_eq!(body["max_completion_tokens"], 100);
        assert!(body.get("max_tokens").is_none());
        assert!(body.get("temperature").is_none());
        assert_eq!(body["reasoning_effort"], "minimal");
        assert_eq!(
            body["messages"][0],
            json!({"role": "system", "content": "stable instructions\n\nprofile"})
        );
        assert_eq!(body["messages"][1]["role"], "user");
        assert_eq!(
            body["messages"][1]["content"][0],
            json!({"type": "text", "text": "hello"})
        );
        assert_eq!(
            body["messages"][1]["content"][1],
            json!({"type": "image_url", "image_url": {"url": "data:image/jpeg;base64,AAAA"}})
        );
        assert_eq!(body["response_format"]["type"], "json_schema");
        assert_eq!(body["response_format"]["json_schema"]["name"], SCHEMA_NAME);
        assert_eq!(body["response_format"]["json_schema"]["strict"], true);
        assert_eq!(
            body["response_format"]["json_schema"]["schema"]["additionalProperties"],
            false
        );
        // Anthropic-only knobs leave no trace.
        assert!(body.get("thinking").is_none());
        assert!(body.get("system").is_none());
        assert!(!body.to_string().contains("cache_control"));
    }

    #[test]
    fn reasoning_effort_policy() {
        use AiProvider::{OpenAi, Xai};
        use AiUsageKind::{Advice, Classify, Report, Vision};
        assert_eq!(
            reasoning_effort_for(OpenAi, "gpt-5-mini", Classify),
            Some("minimal")
        );
        assert_eq!(
            reasoning_effort_for(OpenAi, "gpt-5.4", Vision),
            Some("minimal")
        );
        assert_eq!(reasoning_effort_for(OpenAi, "gpt-5", Report), None);
        assert_eq!(reasoning_effort_for(OpenAi, "gpt-5", Advice), None);
        assert_eq!(
            reasoning_effort_for(OpenAi, "o4-mini", Classify),
            Some("low")
        );
        assert_eq!(reasoning_effort_for(OpenAi, "o3", Report), None);
        assert_eq!(reasoning_effort_for(OpenAi, "gpt-4.1-mini", Classify), None);
        assert_eq!(reasoning_effort_for(OpenAi, "gpt-4o", Vision), None);
        assert_eq!(
            reasoning_effort_for(Xai, "grok-3-mini", Classify),
            Some("low")
        );
        assert_eq!(
            reasoning_effort_for(Xai, "grok-3-mini-fast", Vision),
            Some("low")
        );
        assert_eq!(reasoning_effort_for(Xai, "grok-3-mini", Report), None);
        assert_eq!(
            reasoning_effort_for(Xai, "grok-4-1-fast-reasoning", Classify),
            None
        );
        assert_eq!(reasoning_effort_for(Xai, "grok-4", Classify), None);
        // xAI never sees OpenAI's policy even for an OpenAI-looking id.
        assert_eq!(reasoning_effort_for(Xai, "gpt-5", Classify), None);
    }

    #[test]
    fn xai_body_shape() {
        let body = request_body(
            AiProvider::Xai,
            &request("grok-4-1-fast-non-reasoning", AiUsageKind::Vision),
        );
        assert_eq!(body["max_tokens"], 100);
        assert!(body.get("max_completion_tokens").is_none());
        assert!(body.get("reasoning_effort").is_none());
        assert!(body.get("temperature").is_none());
        let body = request_body(
            AiProvider::Xai,
            &request("grok-3-mini", AiUsageKind::Classify),
        );
        assert_eq!(body["reasoning_effort"], "low");
    }

    #[test]
    fn report_without_system_or_schema() {
        let mut req = request("gpt-5", AiUsageKind::Report);
        req.system.clear();
        req.json_schema = None;
        req.messages = vec![LlmMessage::user_text("write")];
        let body = request_body(AiProvider::OpenAi, &req);
        assert_eq!(body["messages"].as_array().unwrap().len(), 1);
        assert_eq!(body["messages"][0]["role"], "user");
        assert!(body.get("response_format").is_none());
        assert!(body.get("reasoning_effort").is_none());
    }

    #[test]
    fn assistant_turns_are_plain_strings() {
        let mut req = request("gpt-5", AiUsageKind::Report);
        req.messages = vec![
            LlmMessage::user_text("a"),
            LlmMessage {
                role: LlmRole::Assistant,
                content: vec![LlmContent::Text("b".into())],
            },
            LlmMessage::user_text("c"),
        ];
        let body = request_body(AiProvider::OpenAi, &req);
        assert_eq!(
            body["messages"][2],
            json!({"role": "assistant", "content": "b"})
        );
    }

    #[test]
    fn response_parsing() {
        let raw = r#"{"id":"chatcmpl-1","model":"gpt-5-mini-2025-08-07","choices":[{"index":0,"message":{"role":"assistant","content":"{\"a\":1}","refusal":null,"annotations":[]},"finish_reason":"stop","logprobs":null}],"usage":{"prompt_tokens":1200,"completion_tokens":80,"total_tokens":1280,"prompt_tokens_details":{"cached_tokens":500,"audio_tokens":0},"completion_tokens_details":{"reasoning_tokens":40}}}"#;
        let parsed: ChatResponse = serde_json::from_str(raw).unwrap();
        assert_eq!(parsed.model, "gpt-5-mini-2025-08-07");
        assert_eq!(parsed.usage.prompt_tokens, 1200);
        assert_eq!(parsed.usage.cached(), 500);
        let choice = parsed.choices.into_iter().next().unwrap();
        assert_eq!(choice.message.content.unwrap().into_text(), "{\"a\":1}");
        assert_eq!(
            stop_reason(
                choice.finish_reason.as_deref(),
                choice.message.refusal.as_deref()
            ),
            StopReason::EndTurn
        );

        let parts = r#"{"choices":[{"message":{"content":[{"type":"text","text":"x"},{"type":"text","text":"y"}]}}]}"#;
        let parsed: ChatResponse = serde_json::from_str(parts).unwrap();
        let choice = parsed.choices.into_iter().next().unwrap();
        assert_eq!(choice.message.content.unwrap().into_text(), "xy");
        assert_eq!(parsed.usage.cached(), 0);

        let no_usage: ChatResponse = serde_json::from_str(r#"{"choices":[]}"#).unwrap();
        assert!(no_usage.choices.is_empty());
        // A cached count above prompt_tokens (never expected) cannot underflow the input.
        let odd = ChatUsage {
            prompt_tokens: 10,
            completion_tokens: 0,
            prompt_tokens_details: Some(PromptTokensDetails { cached_tokens: 50 }),
        };
        assert_eq!(odd.cached(), 10);
    }

    #[test]
    fn stop_reason_mapping() {
        assert_eq!(stop_reason(Some("stop"), None), StopReason::EndTurn);
        assert_eq!(stop_reason(None, Some("")), StopReason::EndTurn);
        assert_eq!(stop_reason(Some("length"), None), StopReason::MaxTokens);
        assert_eq!(
            stop_reason(Some("content_filter"), None),
            StopReason::Refusal
        );
        assert_eq!(
            stop_reason(Some("stop"), Some("I can't help")),
            StopReason::Refusal
        );
        assert_eq!(
            stop_reason(Some("tool_calls"), None),
            StopReason::Other("tool_calls".into())
        );
    }

    #[test]
    fn error_envelope_parsing() {
        let body = r#"{"error":{"message":"Incorrect API key provided","type":"invalid_request_error","param":null,"code":"invalid_api_key"}}"#;
        let err = parse_error(body).unwrap();
        assert_eq!(err.code, "invalid_api_key");
        assert_eq!(err.kind, "invalid_request_error");
        assert_eq!(
            error_message(body),
            "invalid_api_key: Incorrect API key provided"
        );
        let numeric = r#"{"error":{"message":"nope","code":404}}"#;
        assert_eq!(parse_error(numeric).unwrap().code, "404");
        assert_eq!(error_message(numeric), "404: nope");
        let bare = r#"{"error":"Your team has insufficient credits.","code":"Payment required"}"#;
        let err = parse_error(bare).unwrap();
        assert_eq!(err.message, "Your team has insufficient credits.");
        assert_eq!(err.code, "Payment required");
        assert_eq!(parse_error("<html>"), None);
        assert_eq!(parse_error(r#"{"error":{}}"#), None);
        assert_eq!(parse_error(r#"{"error":null}"#), None);
        assert_eq!(parse_error(r#"{"error":7}"#), None);
        assert_eq!(
            error_message("  <html>  oops </html>"),
            "<html> oops </html>"
        );
    }

    #[test]
    fn retry_hints() {
        assert_eq!(
            retry_hint_secs("Rate limit reached for gpt-4o. Please try again in 20s."),
            Some(20)
        );
        assert_eq!(retry_hint_secs("please try again in 1.2s"), Some(2));
        assert_eq!(retry_hint_secs("Please try again in 350ms."), Some(1));
        assert_eq!(retry_hint_secs("try again in 2m"), Some(120));
        assert_eq!(retry_hint_secs("try again in 2 seconds"), Some(2));
        assert_eq!(retry_hint_secs("slow down"), None);
        assert_eq!(retry_hint_secs("try again in a while"), None);
    }

    #[test]
    fn status_classification() {
        let p = AiProvider::OpenAi;
        assert!(matches!(
            classify_response(p, StatusCode::OK, None, "{}".into()),
            Outcome::Ok(_)
        ));
        let unauthorized = r#"{"error":{"message":"Incorrect API key provided","type":"invalid_request_error","code":"invalid_api_key"}}"#;
        match classify_response(p, StatusCode::UNAUTHORIZED, None, unauthorized.into()) {
            Outcome::Fatal(CoreError::Ai(msg)) => {
                assert!(msg.contains("invalid api key (HTTP 401)"), "{msg}");
                assert!(msg.contains("Incorrect API key provided"), "{msg}");
            }
            other => panic!("unexpected {other:?}"),
        }
        assert!(matches!(
            classify_response(p, StatusCode::FORBIDDEN, None, "".into()),
            Outcome::Fatal(CoreError::Ai(_))
        ));
        assert!(matches!(
            classify_response(p, StatusCode::BAD_REQUEST, None, "bad".into()),
            Outcome::Fatal(CoreError::Invalid(_))
        ));
        assert!(matches!(
            classify_response(p, StatusCode::PAYLOAD_TOO_LARGE, None, "".into()),
            Outcome::Fatal(CoreError::Invalid(_))
        ));
        assert!(matches!(
            classify_response(p, StatusCode::UNPROCESSABLE_ENTITY, None, "".into()),
            Outcome::Fatal(CoreError::Invalid(_))
        ));
        match classify_response(p, StatusCode::TOO_MANY_REQUESTS, Some(7), "".into()) {
            Outcome::Retry {
                error: RetryableError::RateLimited { retry_after_secs },
                wait,
            } => {
                assert_eq!(retry_after_secs, Some(7));
                assert_eq!(wait, Some(Duration::from_secs(7)));
            }
            other => panic!("unexpected {other:?}"),
        }
        // Without the header the message hint is used.
        let hinted = r#"{"error":{"message":"Rate limit reached. Please try again in 3s.","type":"requests","code":"rate_limit_exceeded"}}"#;
        match classify_response(p, StatusCode::TOO_MANY_REQUESTS, None, hinted.into()) {
            Outcome::Retry { wait, .. } => assert_eq!(wait, Some(Duration::from_secs(3))),
            other => panic!("unexpected {other:?}"),
        }
        for code in [408u16, 409, 500, 502, 503, 504] {
            let status = StatusCode::from_u16(code).unwrap();
            assert!(
                matches!(
                    classify_response(p, status, None, "".into()),
                    Outcome::Retry {
                        error: RetryableError::Transient(_),
                        wait: None
                    }
                ),
                "{code}"
            );
        }
        assert!(matches!(
            classify_response(p, StatusCode::IM_A_TEAPOT, None, "".into()),
            Outcome::Fatal(CoreError::Ai(_))
        ));
    }

    #[test]
    fn billing_and_model_rejections() {
        let p = AiProvider::OpenAi;
        let quota = r#"{"error":{"message":"You exceeded your current quota, please check your plan and billing details.","type":"insufficient_quota","param":null,"code":"insufficient_quota"}}"#;
        match classify_response(p, StatusCode::TOO_MANY_REQUESTS, None, quota.into()) {
            Outcome::Fatal(CoreError::AiRejected(msg)) => {
                assert!(msg.starts_with("cobrança:"), "{msg}");
                assert!(msg.contains("exceeded your current quota"), "{msg}");
                assert!(msg.contains("platform.openai.com"), "{msg}");
            }
            other => panic!("unexpected {other:?}"),
        }
        let hard_limit = r#"{"error":{"message":"Billing hard limit has been reached","type":"invalid_request_error","code":"billing_hard_limit_reached"}}"#;
        assert!(matches!(
            classify_response(p, StatusCode::TOO_MANY_REQUESTS, None, hard_limit.into()),
            Outcome::Fatal(CoreError::AiRejected(m)) if m.starts_with("cobrança:")
        ));
        assert!(matches!(
            classify_response(AiProvider::Xai, StatusCode::PAYMENT_REQUIRED, None, "".into()),
            Outcome::Fatal(CoreError::AiRejected(m)) if m.contains("xAI Grok") && m.contains("HTTP 402")
        ));
        let model = r#"{"error":{"message":"The model `gpt-9` does not exist or you do not have access to it.","type":"invalid_request_error","param":null,"code":"model_not_found"}}"#;
        match classify_response(p, StatusCode::NOT_FOUND, None, model.into()) {
            Outcome::Fatal(CoreError::AiRejected(msg)) => {
                assert!(msg.starts_with("modelo não encontrado"), "{msg}");
                assert!(msg.contains("gpt-9"), "{msg}");
            }
            other => panic!("unexpected {other:?}"),
        }
        // The code alone is enough, whatever the status.
        assert!(matches!(
            classify_response(p, StatusCode::BAD_REQUEST, None, model.into()),
            Outcome::Fatal(CoreError::AiRejected(_))
        ));
        assert!(matches!(
            classify_response(p, StatusCode::NOT_FOUND, None, "".into()),
            Outcome::Fatal(CoreError::AiRejected(_))
        ));
        // Other 400 bodies stay request errors.
        let other400 = r#"{"error":{"message":"Unsupported parameter: 'max_tokens'","type":"invalid_request_error","param":"max_tokens","code":"unsupported_parameter"}}"#;
        assert!(matches!(
            classify_response(p, StatusCode::BAD_REQUEST, None, other400.into()),
            Outcome::Fatal(CoreError::Invalid(m)) if m.contains("max_tokens")
        ));
    }

    #[test]
    fn chat_model_filtering() {
        let body = r#"{"object":"list","data":[
            {"id":"gpt-5-mini"},{"id":"gpt-5"},{"id":"text-embedding-3-small"},{"id":"tts-1"},
            {"id":"whisper-1"},{"id":"dall-e-3"},{"id":"gpt-4o-audio-preview"},{"id":"gpt-4o-realtime-preview"},
            {"id":"gpt-image-1"},{"id":"omni-moderation-latest"},{"id":"gpt-4o-transcribe"},{"id":"sora-2"},
            {"id":"gpt-4o-search-preview"},{"id":"gpt-5-codex"},{"id":"gpt-5"},{"id":"grok-2-vision-1212"}
        ]}"#;
        assert_eq!(
            chat_model_ids(body),
            vec!["gpt-5", "gpt-5-mini", "grok-2-vision-1212"]
        );
        assert!(chat_model_ids("nope").is_empty());
        assert!(is_chat_model("o4-mini"));
        assert!(!is_chat_model("Text-Embedding-Ada-002"));
        assert!(!is_chat_model(" "));
    }

    #[test]
    fn anthropic_config_is_rejected_by_new() {
        let config = OpenAiCompatConfig::with_base_url(AiProvider::Anthropic, "http://x");
        let err = OpenAiCompatClient::new(
            config,
            Arc::new(crate::client::StaticApiKey("k".into())),
            Arc::new(crate::fake::MemoryUsageRepo::new()),
        )
        .err()
        .unwrap();
        assert!(matches!(err, CoreError::Invalid(_)), "{err:?}");
    }
}
