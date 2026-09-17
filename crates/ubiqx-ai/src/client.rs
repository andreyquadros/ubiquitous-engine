//! Anthropic Messages API adapter.
//!
//! [`AnthropicClient`] is deliberately small: it serialises an [`LlmRequest`] into the exact
//! JSON shape of `POST /v1/messages`, retries transient failures with exponential backoff,
//! maps HTTP errors onto [`CoreError`] and records an [`AiUsage`] (with a cost estimate) after
//! every successful call. Prompt construction and answer validation live in the port
//! implementations ([`crate::classifier`], [`crate::vision`], [`crate::report`],
//! [`crate::advisor`]).
//!
//! The request/response types are private to this crate on purpose: `ubiqx-core` must not know
//! how a vendor API looks.

use std::fmt;
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use chrono::Utc;
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::{debug, info, trace, warn};
use ubiqx_core::ports::{AiUsage, AiUsageKind, UsageRepo};
use ubiqx_core::{CoreError, CoreResult};

use crate::pricing;

/// Production endpoint.
pub const DEFAULT_BASE_URL: &str = "https://api.anthropic.com";
/// Preferred (cheapest) model for the one-token billing probe in
/// [`AnthropicClient::validate_key`], used when the account's models listing includes it.
const KEY_PROBE_MODEL: &str = "claude-haiku-4-5";
/// Value of the `anthropic-version` header.
pub const API_VERSION: &str = "2023-06-01";
/// Total per-request timeout for classification and advice calls.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);
/// Total per-request timeout for vision and report calls.
pub const LONG_TIMEOUT: Duration = Duration::from_secs(120);
/// Attempts per call (first try + 3 retries).
pub const DEFAULT_MAX_ATTEMPTS: u32 = 4;
/// First backoff wait; doubles on every retry.
pub const DEFAULT_BACKOFF_BASE: Duration = Duration::from_secs(2);
/// Longest single wait between attempts.
pub const DEFAULT_BACKOFF_CAP: Duration = Duration::from_secs(30);

// ---------------------------------------------------------------------------------------------
// Configuration and key source
// ---------------------------------------------------------------------------------------------

/// Where the API key comes from. Read on every call so the key can be changed at runtime
/// (the desktop app backs this with the Keychain).
pub trait ApiKeySource: Send + Sync {
    /// `None` when no key is configured (the client then returns [`CoreError::AiNotConfigured`]
    /// without touching the network).
    fn api_key(&self) -> Option<String>;
}

/// A fixed key (CLI, tests). Blank strings count as "not configured".
#[derive(Clone)]
pub struct StaticApiKey(pub String);

impl fmt::Debug for StaticApiKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("StaticApiKey(***)")
    }
}

impl ApiKeySource for StaticApiKey {
    fn api_key(&self) -> Option<String> {
        let key = self.0.trim();
        (!key.is_empty()).then(|| key.to_string())
    }
}

/// Client configuration. `Default` targets the production API with the documented policy
/// (60 s timeout, 4 attempts, 2 s base backoff capped at 30 s).
#[derive(Debug, Clone)]
pub struct AnthropicConfig {
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

impl Default for AnthropicConfig {
    fn default() -> Self {
        Self {
            base_url: DEFAULT_BASE_URL.to_string(),
            timeout: DEFAULT_TIMEOUT,
            max_attempts: DEFAULT_MAX_ATTEMPTS,
            backoff_base: DEFAULT_BACKOFF_BASE,
            backoff_cap: DEFAULT_BACKOFF_CAP,
        }
    }
}

impl AnthropicConfig {
    /// Configuration pointing at another base URL (tests, proxies).
    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            ..Self::default()
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Request / response model
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LlmRole {
    User,
    Assistant,
}

/// One content block of a message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LlmContent {
    Text(String),
    /// A base64-encoded image (`image/jpeg`, `image/png`, `image/webp` or `image/gif`).
    Image {
        media_type: String,
        base64: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmMessage {
    pub role: LlmRole,
    pub content: Vec<LlmContent>,
}

impl LlmMessage {
    pub fn user(content: Vec<LlmContent>) -> Self {
        Self {
            role: LlmRole::User,
            content,
        }
    }

    pub fn user_text(text: impl Into<String>) -> Self {
        Self::user(vec![LlmContent::Text(text.into())])
    }
}

/// One block of the system prompt. `cache` marks it as a prompt-caching breakpoint
/// (`cache_control: {"type": "ephemeral"}`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemBlock {
    pub text: String,
    pub cache: bool,
}

impl SystemBlock {
    pub fn new(text: impl Into<String>, cache: bool) -> Self {
        Self {
            text: text.into(),
            cache,
        }
    }
}

/// A complete Messages API call, vendor details already decided by the caller.
#[derive(Debug, Clone)]
pub struct LlmRequest {
    pub model: String,
    pub system: Vec<SystemBlock>,
    pub messages: Vec<LlmMessage>,
    pub max_tokens: u32,
    /// When set, structured output is requested (`output_config.format = json_schema`).
    pub json_schema: Option<Value>,
    /// Sends `thinking: {"type": "disabled"}` (Sonnet/Opus only; Haiku must not get the field).
    pub disable_thinking: bool,
    /// What the call is for; stored with the usage record.
    pub usage_kind: AiUsageKind,
    /// Overrides [`AnthropicConfig::timeout`] for this call (vision and reports use
    /// [`LONG_TIMEOUT`]).
    pub timeout: Option<Duration>,
}

/// Why the model stopped generating.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StopReason {
    EndTurn,
    /// Output was cut off: the text is most likely unparseable and the caller should retry.
    MaxTokens,
    Refusal,
    Other(String),
}

impl StopReason {
    fn parse(raw: Option<&str>) -> Self {
        match raw {
            None | Some("end_turn") | Some("stop_sequence") => StopReason::EndTurn,
            Some("max_tokens") => StopReason::MaxTokens,
            Some("refusal") => StopReason::Refusal,
            Some(other) => StopReason::Other(other.to_string()),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            StopReason::EndTurn => "end_turn",
            StopReason::MaxTokens => "max_tokens",
            StopReason::Refusal => "refusal",
            StopReason::Other(s) => s.as_str(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LlmResponse {
    /// Text of the first `text` content block (the JSON document for structured outputs).
    pub text: String,
    pub model: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cache_read_tokens: u32,
    pub cache_write_tokens: u32,
    pub stop_reason: StopReason,
}

impl LlmResponse {
    /// True when the answer hit `max_tokens` and must not be trusted as complete.
    pub fn is_truncated(&self) -> bool {
        self.stop_reason == StopReason::MaxTokens
    }
}

/// Anything that can answer an [`LlmRequest`]. Implemented by [`AnthropicClient`] and by
/// scripted doubles in [`crate::fake`], so the port implementations are testable offline.
#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn complete(&self, req: &LlmRequest) -> CoreResult<LlmResponse>;
}

// ---------------------------------------------------------------------------------------------
// Model-dependent request policy
// ---------------------------------------------------------------------------------------------

fn is_haiku(model: &str) -> bool {
    model.to_ascii_lowercase().contains("haiku")
}

/// Whether `thinking: {"type": "disabled"}` should be sent for this model. Haiku 4.5 has no
/// adaptive thinking to switch off, so it must not receive the field; Sonnet/Opus 5 would
/// otherwise think (and bill) on every call.
pub fn thinking_disabled_for(model: &str) -> bool {
    !is_haiku(model)
}

/// Whether a prompt-cache breakpoint on the system prompt is worthwhile. Haiku's 4096-token
/// minimum makes it a no-op for the prompts this crate sends.
pub fn prompt_cache_for(model: &str) -> bool {
    !is_haiku(model)
}

// ---------------------------------------------------------------------------------------------
// JSON body
// ---------------------------------------------------------------------------------------------

fn content_json(c: &LlmContent) -> Value {
    match c {
        LlmContent::Text(text) => json!({"type": "text", "text": text}),
        LlmContent::Image { media_type, base64 } => json!({
            "type": "image",
            "source": {"type": "base64", "media_type": media_type, "data": base64}
        }),
    }
}

/// The exact JSON body sent to `POST /v1/messages` for `req`. Public so the shape can be
/// inspected and tested without a server. Never adds `temperature` or an assistant prefill.
pub fn request_body(req: &LlmRequest) -> Value {
    let system: Vec<Value> = req
        .system
        .iter()
        .filter(|b| !b.text.trim().is_empty())
        .map(|b| {
            let mut block = json!({"type": "text", "text": b.text});
            if b.cache {
                block["cache_control"] = json!({"type": "ephemeral"});
            }
            block
        })
        .collect();
    let messages: Vec<Value> = req
        .messages
        .iter()
        .map(|m| {
            json!({
                "role": m.role,
                "content": m.content.iter().map(content_json).collect::<Vec<_>>()
            })
        })
        .collect();

    let mut body = json!({
        "model": req.model,
        "max_tokens": req.max_tokens,
        "messages": messages,
    });
    if !system.is_empty() {
        body["system"] = Value::Array(system);
    }
    if let Some(schema) = &req.json_schema {
        body["output_config"] = json!({"format": {"type": "json_schema", "schema": schema}});
    }
    if req.disable_thinking {
        body["thinking"] = json!({"type": "disabled"});
    }
    body
}

fn validate_request(req: &LlmRequest) -> CoreResult<()> {
    if req.model.trim().is_empty() {
        return Err(CoreError::Invalid("model id is empty".into()));
    }
    if req.max_tokens == 0 {
        return Err(CoreError::Invalid("max_tokens must be positive".into()));
    }
    match req.messages.last() {
        None => return Err(CoreError::Invalid("request has no messages".into())),
        Some(last) if last.role != LlmRole::User => {
            return Err(CoreError::Invalid(
                "last message must be from the user (assistant prefill is not supported)".into(),
            ))
        }
        _ => {}
    }
    if req.messages.iter().any(|m| m.content.is_empty()) {
        return Err(CoreError::Invalid("message without content".into()));
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Response parsing
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct MessagesResponse {
    #[serde(default)]
    model: String,
    #[serde(default)]
    stop_reason: Option<String>,
    #[serde(default)]
    content: Vec<ContentBlock>,
    #[serde(default)]
    usage: UsageBlock,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(other)]
    Other,
}

#[derive(Debug, Default, Deserialize)]
struct UsageBlock {
    #[serde(default)]
    input_tokens: u32,
    #[serde(default)]
    output_tokens: u32,
    #[serde(default)]
    cache_creation_input_tokens: Option<u32>,
    #[serde(default)]
    cache_read_input_tokens: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct ErrorEnvelope {
    error: Option<ErrorBody>,
}

#[derive(Debug, Deserialize)]
struct ErrorBody {
    #[serde(rename = "type")]
    kind: Option<String>,
    message: Option<String>,
}

/// `(type, message)` of an API error envelope, when the body is one.
fn parse_error(body: &str) -> Option<(String, String)> {
    let env = serde_json::from_str::<ErrorEnvelope>(body).ok()?;
    let err = env.error?;
    let kind = err.kind.unwrap_or_default();
    let msg = err.message.unwrap_or_default();
    (!kind.is_empty() || !msg.is_empty()).then_some((kind, msg))
}

/// Extracts a short, loggable message from an API error body.
fn error_message(body: &str) -> String {
    if let Some((kind, msg)) = parse_error(body) {
        return format!("{kind}: {msg}")
            .trim_matches([':', ' '])
            .to_string();
    }
    let trimmed: String = body.chars().take(200).collect();
    trimmed.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Provider-side rejections that are neither transient nor the request's fault: a billing
/// problem (no credits) or a model the account cannot use. The user has to act, so the
/// message is written for them (pt-BR), keeping the provider's own text.
fn rejection(code: u16, body: &str) -> Option<CoreError> {
    let (kind, msg) = parse_error(body)?;
    let lower = msg.to_ascii_lowercase();
    let billing = (code == 400
        && kind == "invalid_request_error"
        && (lower.contains("credit balance") || lower.contains("plans & billing")))
        || (code == 402 && kind == "billing_error");
    if billing {
        return Some(CoreError::AiRejected(format!(
            "cobrança: sua conta Anthropic não tem créditos disponíveis ({msg}). Adicione créditos em console.anthropic.com → Plans & Billing."
        )));
    }
    if code == 404 && kind == "not_found_error" && lower.starts_with("model:") {
        return Some(CoreError::AiRejected(format!(
            "modelo não encontrado ou indisponível para a sua conta ({msg}). Verifique o modelo em Configurações → IA."
        )));
    }
    None
}

/// Parses a `retry-after` header given in seconds (integer or decimal). HTTP dates are ignored.
fn parse_retry_after(value: Option<&reqwest::header::HeaderValue>) -> Option<u64> {
    let text = value?.to_str().ok()?.trim();
    if let Ok(secs) = text.parse::<u64>() {
        return Some(secs);
    }
    let secs: f64 = text.parse().ok()?;
    (secs.is_finite() && secs >= 0.0).then(|| secs.ceil() as u64)
}

/// Model id for the billing probe from a `GET /v1/models` body: [`KEY_PROBE_MODEL`] when listed,
/// otherwise the first listed model; `None` when the listing is empty or unparseable.
fn probe_model(models_body: &str) -> Option<String> {
    let listing: Value = serde_json::from_str(models_body).ok()?;
    let ids: Vec<&str> = listing
        .get("data")?
        .as_array()?
        .iter()
        .filter_map(|m| m.get("id").and_then(Value::as_str))
        .collect();
    ids.iter()
        .find(|id| **id == KEY_PROBE_MODEL)
        .or_else(|| ids.first())
        .map(|id| id.to_string())
}

// ---------------------------------------------------------------------------------------------
// Retry policy
// ---------------------------------------------------------------------------------------------

/// Wait before retrying after the `attempt`-th failure (1-based): `base * 2^(attempt-1)`, capped
/// at `cap`, then scaled by `0.75 + 0.5 * jitter` (`jitter` in `[0, 1)`) and capped again.
pub fn backoff_delay(attempt: u32, base: Duration, cap: Duration, jitter: f64) -> Duration {
    let exp = attempt.saturating_sub(1).min(16);
    let raw = base.saturating_mul(1u32 << exp).min(cap);
    let factor = 0.75 + 0.5 * jitter.clamp(0.0, 1.0);
    raw.mul_f64(factor).min(cap)
}

/// What to do with an HTTP outcome.
#[derive(Debug)]
enum Outcome {
    Ok(String),
    Fatal(CoreError),
    /// Retry after an optional server-mandated wait (from `retry-after`).
    Retry {
        error: RetryableError,
        wait: Option<Duration>,
    },
}

#[derive(Debug, thiserror::Error)]
enum RetryableError {
    #[error("rate limited")]
    RateLimited { retry_after_secs: Option<u64> },
    #[error("transient failure: {0}")]
    Transient(String),
}

impl RetryableError {
    fn into_core(self, fallback_wait: Duration) -> CoreError {
        match self {
            RetryableError::RateLimited { retry_after_secs } => CoreError::RateLimited {
                retry_after_secs: retry_after_secs.unwrap_or(fallback_wait.as_secs().max(1)),
            },
            RetryableError::Transient(msg) => CoreError::Ai(msg),
        }
    }
}

fn classify_response(
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
        400 | 402 | 404 if rejection(code, &body).is_some() => {
            Outcome::Fatal(rejection(code, &body).expect("checked above"))
        }
        400 | 404 | 413 | 422 => Outcome::Fatal(CoreError::Invalid(format!(
            "HTTP {code}: {}",
            error_message(&body)
        ))),
        429 => Outcome::Retry {
            error: RetryableError::RateLimited {
                retry_after_secs: retry_after,
            },
            wait: retry_after.map(Duration::from_secs),
        },
        408 | 500 | 502 | 503 | 504 | 529 => Outcome::Retry {
            error: RetryableError::Transient(format!("HTTP {code}: {}", error_message(&body))),
            wait: None,
        },
        _ => Outcome::Fatal(CoreError::Ai(format!(
            "unexpected HTTP {code}: {}",
            error_message(&body)
        ))),
    }
}

// ---------------------------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------------------------

/// HTTP client for the Anthropic Messages API. Cheap to clone behind an `Arc`.
pub struct AnthropicClient {
    http: reqwest::Client,
    config: AnthropicConfig,
    keys: Arc<dyn ApiKeySource>,
    usage: Arc<dyn UsageRepo>,
}

impl fmt::Debug for AnthropicClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AnthropicClient")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl AnthropicClient {
    /// Builds the client. Fails only when the TLS backend cannot be initialised.
    pub fn new(
        config: AnthropicConfig,
        keys: Arc<dyn ApiKeySource>,
        usage: Arc<dyn UsageRepo>,
    ) -> CoreResult<Self> {
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

    pub fn config(&self) -> &AnthropicConfig {
        &self.config
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.config.base_url.trim_end_matches('/'), path)
    }

    fn key(&self) -> CoreResult<String> {
        self.keys.api_key().ok_or(CoreError::AiNotConfigured)
    }

    /// Sends one Messages API call, retrying transient failures, and records its usage.
    ///
    /// Returns [`CoreError::AiNotConfigured`] without any HTTP call when no key is available,
    /// `CoreError::Ai("invalid api key …")` on 401/403, [`CoreError::AiRejected`] when the
    /// provider reports a billing problem or an unusable model, [`CoreError::Invalid`] on other
    /// 4xx request errors, [`CoreError::RateLimited`] when 429 persists and a transient
    /// [`CoreError::Ai`] when 5xx/529/network errors persist. A `refusal` stop reason yields
    /// [`CoreError::AiRefused`]; a `max_tokens` stop reason is returned to the caller as
    /// [`StopReason::MaxTokens`].
    pub async fn complete(&self, req: &LlmRequest) -> CoreResult<LlmResponse> {
        validate_request(req)?;
        let key = self.key()?;
        let body = request_body(req);
        let url = self.url("/v1/messages");
        let timeout = req.timeout.unwrap_or(self.config.timeout);
        let kind = req.usage_kind.as_str();

        debug!(
            model = %req.model,
            kind,
            max_tokens = req.max_tokens,
            messages = req.messages.len(),
            structured = req.json_schema.is_some(),
            "anthropic: sending messages request"
        );
        trace!(body = %body, "anthropic: request body");

        let started = Instant::now();
        let text = self
            .send_with_retry("messages", || {
                self.http
                    .post(&url)
                    .header("x-api-key", &key)
                    .header("anthropic-version", API_VERSION)
                    .header("content-type", "application/json")
                    .timeout(timeout)
                    .json(&body)
            })
            .await?;

        let parsed: MessagesResponse = serde_json::from_str(&text)
            .map_err(|e| CoreError::Ai(format!("unparseable messages response: {e}")))?;
        let stop_reason = StopReason::parse(parsed.stop_reason.as_deref());
        let answer = parsed
            .content
            .into_iter()
            .find_map(|c| match c {
                ContentBlock::Text { text } => Some(text),
                ContentBlock::Other => None,
            })
            .unwrap_or_default();
        let cache_read = parsed.usage.cache_read_input_tokens.unwrap_or(0);
        let cache_write = parsed.usage.cache_creation_input_tokens.unwrap_or(0);
        let cost_usd = pricing::estimate_cost_usd(
            &req.model,
            parsed.usage.input_tokens,
            parsed.usage.output_tokens,
            cache_read,
            cache_write,
        );

        let usage = AiUsage {
            at: Utc::now(),
            kind: req.usage_kind,
            model: req.model.clone(),
            input_tokens: parsed.usage.input_tokens,
            output_tokens: parsed.usage.output_tokens,
            cache_read_tokens: cache_read,
            cache_write_tokens: cache_write,
            cost_usd,
        };
        if let Err(e) = self.usage.record(&usage) {
            warn!(error = %e, "anthropic: could not record usage");
        }
        info!(
            model = %req.model,
            kind,
            input_tokens = usage.input_tokens,
            output_tokens = usage.output_tokens,
            cache_read_tokens = cache_read,
            cache_write_tokens = cache_write,
            cost_usd,
            stop_reason = stop_reason.as_str(),
            elapsed_ms = started.elapsed().as_millis() as u64,
            "anthropic: call completed"
        );
        trace!(answer = %answer, "anthropic: response text");

        if stop_reason == StopReason::Refusal {
            // A per-content decision by the model, not an outage: the caller must not retry
            // the same payload.
            return Err(CoreError::AiRefused);
        }
        if stop_reason == StopReason::MaxTokens {
            warn!(model = %req.model, kind, max_tokens = req.max_tokens, "anthropic: output truncated");
        }

        Ok(LlmResponse {
            text: answer,
            model: if parsed.model.is_empty() {
                req.model.clone()
            } else {
                parsed.model
            },
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
            cache_read_tokens: cache_read,
            cache_write_tokens: cache_write,
            stop_reason,
        })
    }

    /// Checks the configured key with `GET /v1/models`, then probes billing with a one-token
    /// `POST /v1/messages` (the models listing succeeds for an account without credits, so a
    /// key would otherwise look fine at setup and fail silently on every classification).
    ///
    /// `Ok(())` means the key is accepted and the account can be billed; 401/403 yield
    /// `CoreError::Ai("invalid api key …")`; a billing rejection yields
    /// [`CoreError::AiRejected`] with a message written for the user; a missing key yields
    /// [`CoreError::AiNotConfigured`] without any request. Any other outcome of the probe
    /// (the probe model not being available to the account, rate limits, outages) is not
    /// held against the key.
    pub async fn validate_key(&self) -> CoreResult<()> {
        let key = self.key()?;
        let url = self.url("/v1/models");
        let timeout = self.config.timeout;
        let listing = self
            .send_with_retry("models", || {
                self.http
                    .get(&url)
                    .header("x-api-key", &key)
                    .header("anthropic-version", API_VERSION)
                    .timeout(timeout)
            })
            .await?;

        // Probe with a model the listing says this account can use, so a "model not found"
        // answer cannot be mistaken for a bad key; the preferred one is the cheapest.
        let Some(model) = probe_model(&listing) else {
            debug!("anthropic: models listing is empty; skipping the billing probe");
            return Ok(());
        };
        let url = self.url("/v1/messages");
        let body = json!({
            "model": model,
            "max_tokens": 1,
            "messages": [{"role": "user", "content": "ping"}],
        });
        match self
            .send_with_retry("key probe", || {
                self.http
                    .post(&url)
                    .header("x-api-key", &key)
                    .header("anthropic-version", API_VERSION)
                    .header("content-type", "application/json")
                    .timeout(timeout)
                    .json(&body)
            })
            .await
        {
            Ok(_) => Ok(()),
            Err(err @ CoreError::AiRejected(_)) => Err(err),
            Err(e) => {
                debug!(error = %e, "anthropic: billing probe inconclusive; accepting the key");
                Ok(())
            }
        }
    }

    async fn send_with_retry<F>(&self, what: &'static str, build: F) -> CoreResult<String>
    where
        F: Fn() -> reqwest::RequestBuilder,
    {
        let max_attempts = self.config.max_attempts.max(1);
        let mut attempt: u32 = 1;
        loop {
            let outcome = match build().send().await {
                Ok(resp) => {
                    let status = resp.status();
                    let retry_after = parse_retry_after(resp.headers().get("retry-after"));
                    let body = resp.text().await.unwrap_or_default();
                    classify_response(status, retry_after, body)
                }
                Err(e) => {
                    let msg = if e.is_timeout() {
                        format!("request timed out: {e}")
                    } else {
                        format!("network error: {e}")
                    };
                    Outcome::Retry {
                        error: RetryableError::Transient(msg),
                        wait: None,
                    }
                }
            };

            match outcome {
                Outcome::Ok(body) => return Ok(body),
                Outcome::Fatal(err) => {
                    warn!(what, attempt, error = %err, "anthropic: request failed");
                    return Err(err);
                }
                Outcome::Retry { error, wait } => {
                    let backoff = backoff_delay(
                        attempt,
                        self.config.backoff_base,
                        self.config.backoff_cap,
                        rand::thread_rng().gen::<f64>(),
                    );
                    let delay = match wait {
                        Some(w) if w > self.config.backoff_cap => {
                            warn!(
                                what,
                                attempt,
                                wait_secs = w.as_secs(),
                                "anthropic: retry-after exceeds cap, giving up"
                            );
                            return Err(error.into_core(w));
                        }
                        Some(w) => w,
                        None => backoff,
                    };
                    if attempt >= max_attempts {
                        warn!(what, attempt, error = %error, "anthropic: giving up after retries");
                        return Err(error.into_core(delay));
                    }
                    warn!(
                        what,
                        attempt,
                        delay_ms = delay.as_millis() as u64,
                        error = %error,
                        "anthropic: retrying"
                    );
                    tokio::time::sleep(delay).await;
                    attempt += 1;
                }
            }
        }
    }
}

#[async_trait]
impl LlmClient for AnthropicClient {
    async fn complete(&self, req: &LlmRequest) -> CoreResult<LlmResponse> {
        AnthropicClient::complete(self, req).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(model: &str) -> LlmRequest {
        LlmRequest {
            model: model.into(),
            system: vec![
                SystemBlock::new("stable instructions", false),
                SystemBlock::new("profile", prompt_cache_for(model)),
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
            disable_thinking: thinking_disabled_for(model),
            usage_kind: AiUsageKind::Classify,
            timeout: None,
        }
    }

    #[test]
    fn body_shape_for_sonnet() {
        let body = request_body(&request("claude-sonnet-5"));
        assert_eq!(body["model"], "claude-sonnet-5");
        assert_eq!(body["max_tokens"], 100);
        assert_eq!(body["system"][0]["type"], "text");
        assert!(body["system"][0].get("cache_control").is_none());
        assert_eq!(body["system"][1]["cache_control"]["type"], "ephemeral");
        assert_eq!(body["messages"][0]["role"], "user");
        assert_eq!(body["messages"][0]["content"][0]["type"], "text");
        assert_eq!(
            body["messages"][0]["content"][1]["source"]["media_type"],
            "image/jpeg"
        );
        assert_eq!(body["output_config"]["format"]["type"], "json_schema");
        assert_eq!(body["thinking"]["type"], "disabled");
        assert!(body.get("temperature").is_none());
    }

    #[test]
    fn body_shape_for_haiku_has_no_thinking_nor_cache() {
        let body = request_body(&request("claude-haiku-4-5"));
        assert!(body.get("thinking").is_none());
        assert!(body["system"][1].get("cache_control").is_none());
    }

    #[test]
    fn empty_system_blocks_are_dropped() {
        let mut req = request("claude-haiku-4-5");
        req.system = vec![SystemBlock::new("   ", false)];
        req.json_schema = None;
        let body = request_body(&req);
        assert!(body.get("system").is_none());
        assert!(body.get("output_config").is_none());
    }

    #[test]
    fn prefill_is_rejected() {
        let mut req = request("claude-haiku-4-5");
        req.messages.push(LlmMessage {
            role: LlmRole::Assistant,
            content: vec![LlmContent::Text("{".into())],
        });
        assert!(matches!(validate_request(&req), Err(CoreError::Invalid(_))));
        req.messages.clear();
        assert!(matches!(validate_request(&req), Err(CoreError::Invalid(_))));
    }

    #[test]
    fn backoff_schedule() {
        let base = Duration::from_secs(2);
        let cap = Duration::from_secs(30);
        // jitter 0.5 → factor 1.0
        assert_eq!(backoff_delay(1, base, cap, 0.5), Duration::from_secs(2));
        assert_eq!(backoff_delay(2, base, cap, 0.5), Duration::from_secs(4));
        assert_eq!(backoff_delay(3, base, cap, 0.5), Duration::from_secs(8));
        assert_eq!(backoff_delay(4, base, cap, 0.5), Duration::from_secs(16));
        assert_eq!(backoff_delay(5, base, cap, 0.5), Duration::from_secs(30));
        assert_eq!(backoff_delay(40, base, cap, 0.5), Duration::from_secs(30));
        // Jitter stays within ±25 % and never exceeds the cap.
        assert_eq!(
            backoff_delay(1, base, cap, 0.0),
            Duration::from_millis(1500)
        );
        assert_eq!(
            backoff_delay(1, base, cap, 1.0),
            Duration::from_millis(2500)
        );
        assert_eq!(backoff_delay(5, base, cap, 1.0), cap);
        assert_eq!(backoff_delay(0, base, cap, 0.5), Duration::from_secs(2));
    }

    #[test]
    fn status_classification() {
        use reqwest::StatusCode;
        assert!(matches!(
            classify_response(StatusCode::OK, None, "{}".into()),
            Outcome::Ok(_)
        ));
        match classify_response(
            StatusCode::UNAUTHORIZED,
            None,
            r#"{"type":"error","error":{"type":"authentication_error","message":"invalid x-api-key"}}"#.into(),
        ) {
            Outcome::Fatal(CoreError::Ai(msg)) => {
                assert!(msg.contains("invalid api key"), "{msg}");
                assert!(msg.contains("authentication_error: invalid x-api-key"), "{msg}");
            }
            other => panic!("unexpected {other:?}"),
        }
        assert!(matches!(
            classify_response(StatusCode::BAD_REQUEST, None, "bad".into()),
            Outcome::Fatal(CoreError::Invalid(_))
        ));
        assert!(matches!(
            classify_response(StatusCode::PAYLOAD_TOO_LARGE, None, "".into()),
            Outcome::Fatal(CoreError::Invalid(_))
        ));
        match classify_response(StatusCode::TOO_MANY_REQUESTS, Some(7), "".into()) {
            Outcome::Retry {
                error: RetryableError::RateLimited { retry_after_secs },
                wait,
            } => {
                assert_eq!(retry_after_secs, Some(7));
                assert_eq!(wait, Some(Duration::from_secs(7)));
            }
            other => panic!("unexpected {other:?}"),
        }
        for code in [500u16, 502, 503, 529] {
            let status = StatusCode::from_u16(code).unwrap();
            assert!(
                matches!(
                    classify_response(status, None, "".into()),
                    Outcome::Retry {
                        error: RetryableError::Transient(_),
                        wait: None
                    }
                ),
                "{code}"
            );
        }
        assert!(matches!(
            classify_response(StatusCode::PAYMENT_REQUIRED, None, "".into()),
            Outcome::Fatal(CoreError::Ai(_))
        ));
    }

    #[test]
    fn billing_and_model_rejections_are_not_invalid_requests() {
        use reqwest::StatusCode;
        let credit = r#"{"type":"error","error":{"type":"invalid_request_error","message":"Your credit balance is too low to access the Anthropic API. Please go to Plans & Billing to upgrade or purchase credits."}}"#;
        match classify_response(StatusCode::BAD_REQUEST, None, credit.into()) {
            Outcome::Fatal(CoreError::AiRejected(msg)) => {
                assert!(msg.starts_with("cobrança:"), "{msg}");
                assert!(msg.contains("credit balance is too low"), "{msg}");
            }
            other => panic!("unexpected {other:?}"),
        }
        let billing =
            r#"{"type":"error","error":{"type":"billing_error","message":"Billing problem."}}"#;
        assert!(matches!(
            classify_response(StatusCode::PAYMENT_REQUIRED, None, billing.into()),
            Outcome::Fatal(CoreError::AiRejected(m)) if m.starts_with("cobrança:")
        ));
        let model = r#"{"type":"error","error":{"type":"not_found_error","message":"model: claude-haiku-9"}}"#;
        match classify_response(StatusCode::NOT_FOUND, None, model.into()) {
            Outcome::Fatal(CoreError::AiRejected(msg)) => {
                assert!(msg.starts_with("modelo não encontrado"), "{msg}");
                assert!(msg.contains("claude-haiku-9"), "{msg}");
            }
            other => panic!("unexpected {other:?}"),
        }
        // Other 400/404 bodies stay request errors.
        let other400 = r#"{"type":"error","error":{"type":"invalid_request_error","message":"max_tokens: too large"}}"#;
        assert!(matches!(
            classify_response(StatusCode::BAD_REQUEST, None, other400.into()),
            Outcome::Fatal(CoreError::Invalid(_))
        ));
        let other404 =
            r#"{"type":"error","error":{"type":"not_found_error","message":"Not Found"}}"#;
        assert!(matches!(
            classify_response(StatusCode::NOT_FOUND, None, other404.into()),
            Outcome::Fatal(CoreError::Invalid(_))
        ));
    }

    #[test]
    fn probe_model_prefers_cheapest_listed() {
        let both =
            r#"{"data":[{"id":"claude-sonnet-5"},{"id":"claude-haiku-4-5"}],"has_more":false}"#;
        assert_eq!(probe_model(both).as_deref(), Some("claude-haiku-4-5"));
        let one = r#"{"data":[{"id":"claude-sonnet-5"}],"has_more":false}"#;
        assert_eq!(probe_model(one).as_deref(), Some("claude-sonnet-5"));
        assert_eq!(probe_model(r#"{"data":[],"has_more":false}"#), None);
        assert_eq!(probe_model("nope"), None);
    }

    #[test]
    fn retry_after_parsing() {
        use reqwest::header::HeaderValue;
        assert_eq!(parse_retry_after(None), None);
        assert_eq!(
            parse_retry_after(Some(&HeaderValue::from_static("12"))),
            Some(12)
        );
        assert_eq!(
            parse_retry_after(Some(&HeaderValue::from_static("1.2"))),
            Some(2)
        );
        assert_eq!(
            parse_retry_after(Some(&HeaderValue::from_static(
                "Wed, 21 Oct 2015 07:28:00 GMT"
            ))),
            None
        );
    }

    #[test]
    fn response_parsing_tolerates_unknown_blocks() {
        let raw = r#"{"id":"msg_1","model":"claude-haiku-4-5","stop_reason":"end_turn","content":[{"type":"thinking","thinking":"..."},{"type":"text","text":"{\"a\":1}"}],"usage":{"input_tokens":10,"output_tokens":2,"service_tier":"standard"}}"#;
        let parsed: MessagesResponse = serde_json::from_str(raw).unwrap();
        assert_eq!(parsed.content.len(), 2);
        assert_eq!(parsed.usage.input_tokens, 10);
        assert_eq!(parsed.usage.cache_read_input_tokens, None);
        assert_eq!(
            StopReason::parse(parsed.stop_reason.as_deref()),
            StopReason::EndTurn
        );
        assert_eq!(StopReason::parse(Some("max_tokens")), StopReason::MaxTokens);
        assert_eq!(StopReason::parse(Some("refusal")), StopReason::Refusal);
        assert_eq!(
            StopReason::parse(Some("tool_use")),
            StopReason::Other("tool_use".into())
        );
    }

    #[test]
    fn static_key_blank_means_unconfigured() {
        assert_eq!(StaticApiKey("  ".into()).api_key(), None);
        assert_eq!(
            StaticApiKey(" sk-1 ".into()).api_key().as_deref(),
            Some("sk-1")
        );
        assert_eq!(
            format!("{:?}", StaticApiKey("secret".into())),
            "StaticApiKey(***)"
        );
    }
}
