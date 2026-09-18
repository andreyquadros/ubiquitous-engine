//! HTTP surface. See README.md for the route table and the error shapes.
//!
//! Every error is Anthropic-shaped so the desktop app's Anthropic client can read it
//! unchanged: `{"type":"error","error":{"type":"<kind>","message":"…"}}`.

use std::sync::Arc;

use axum::body::{Body, Bytes};
use axum::extract::{Query, State};
use axum::http::header::{AUTHORIZATION, CONTENT_TYPE};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::Sha256;
use tracing::{info, warn};
use ubiqx_core::license::{
    self, key_hint, LicenseClaims, Plan, HEADER_UBIQX_COST_USD, HEADER_UBIQX_PLAN,
};

use crate::config::{Config, ANTHROPIC_VERSION};
use crate::db::{is_month, month_of, Db, IssuedLicense, UsageRow};
use crate::issue;
use crate::sse::{usage_from_message, StreamUsage, UsageTap};

/// Header carrying the license key (same name the vendors use, so the app's HTTP client
/// needs no special case).
pub const HEADER_API_KEY: &str = "x-api-key";
/// Signature header of the generic payment webhook.
pub const HEADER_WEBHOOK_SIGNATURE: &str = "x-ubi-signature";
/// Extra response headers on `/v1/messages`: the month's running total and the budget.
pub const HEADER_UBIQX_SPENT_USD: &str = "x-ubiqx-spent-usd";
pub const HEADER_UBIQX_BUDGET_USD: &str = "x-ubiqx-budget-usd";

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db: Db,
    pub http: reqwest::Client,
}

impl AppState {
    pub fn new(config: Config) -> anyhow::Result<Self> {
        license::verifying_key_from_hex(&config.license_pubkey_hex)
            .map_err(|e| anyhow::anyhow!("license public key: {e}"))?;
        let db = Db::open(&config.db_path)?;
        let http = reqwest::Client::builder()
            .user_agent(concat!("ubi-api/", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(AppState {
            config: Arc::new(config),
            db,
            http,
        })
    }
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/admin/models", get(admin_models))
        .route("/v1/license/status", get(license_status))
        .route("/v1/messages", post(messages))
        .route("/admin/licenses/revoke", post(admin_revoke))
        .route("/admin/usage", get(admin_usage))
        .route("/admin/webhooks/generic", post(webhook_generic))
        .with_state(state)
}

// ---------------------------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------------------------

#[derive(Debug)]
pub struct ApiError {
    pub status: StatusCode,
    pub kind: &'static str,
    pub message: String,
    /// Extra top-level fields (the budget figures on a 402).
    pub extra: Option<Value>,
}

impl ApiError {
    pub fn new(status: StatusCode, kind: &'static str, message: impl Into<String>) -> Self {
        ApiError {
            status,
            kind,
            message: message.into(),
            extra: None,
        }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, "invalid_request_error", message)
    }

    pub fn internal(err: impl std::fmt::Display) -> Self {
        warn!(error = %err, "internal error");
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "api_error",
            "internal error",
        )
    }

    pub fn body(&self) -> Value {
        let mut body = json!({
            "type": "error",
            "error": { "type": self.kind, "message": self.message },
        });
        if let (Some(Value::Object(extra)), Value::Object(obj)) = (&self.extra, &mut body) {
            for (k, v) in extra {
                obj.insert(k.clone(), v.clone());
            }
        }
        body
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(self.body())).into_response()
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(e: anyhow::Error) -> Self {
        ApiError::internal(e)
    }
}

// ---------------------------------------------------------------------------------------------
// License checks
// ---------------------------------------------------------------------------------------------

fn header_str<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
}

/// The license key from `x-api-key` or `Authorization: Bearer …`.
fn license_key_from(headers: &HeaderMap) -> Option<String> {
    if let Some(k) = header_str(headers, HEADER_API_KEY).filter(|k| !k.is_empty()) {
        return Some(k.to_string());
    }
    header_str(headers, AUTHORIZATION.as_str())
        .and_then(|v| {
            v.strip_prefix("Bearer ")
                .or_else(|| v.strip_prefix("bearer "))
        })
        .map(str::trim)
        .filter(|k| !k.is_empty())
        .map(str::to_string)
}

/// Why a key cannot use the managed plan right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Valid,
    Expired,
    Revoked,
    /// A genuine key of another plan (`annual_own_key`).
    NotManaged,
}

impl Verdict {
    pub fn state(self) -> &'static str {
        match self {
            Verdict::Valid => "valid",
            Verdict::Expired => "expired",
            Verdict::Revoked => "revoked",
            Verdict::NotManaged => "valid",
        }
    }
}

fn verdict(
    state: &AppState,
    claims: &LicenseClaims,
    now: DateTime<Utc>,
) -> anyhow::Result<Verdict> {
    if claims.is_expired_at(now) {
        return Ok(Verdict::Expired);
    }
    if state.db.is_revoked(&claims.sub)? {
        return Ok(Verdict::Revoked);
    }
    if claims.plan != Plan::MonthlyManaged {
        return Ok(Verdict::NotManaged);
    }
    Ok(Verdict::Valid)
}

/// Verifies the caller's key for `/v1/messages`: signature, expiry, plan and revocation.
fn authorize_managed(
    state: &AppState,
    headers: &HeaderMap,
    now: DateTime<Utc>,
) -> Result<LicenseClaims, ApiError> {
    let key = license_key_from(headers).ok_or_else(|| {
        ApiError::new(
            StatusCode::UNAUTHORIZED,
            "authentication_error",
            "missing license key (x-api-key)",
        )
    })?;
    let claims = license::verify_key_with(&key, &state.config.license_pubkey_hex).map_err(|e| {
        ApiError::new(
            StatusCode::UNAUTHORIZED,
            "license_invalid",
            format!("license key rejected: {e}"),
        )
    })?;
    match verdict(state, &claims, now)? {
        Verdict::Valid => Ok(claims),
        Verdict::Expired => Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "license_expired",
            format!(
                "license expired on {}",
                claims.expires_at_utc().to_rfc3339()
            ),
        )),
        Verdict::Revoked => Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "license_revoked",
            "license was revoked (subscription cancelled)",
        )),
        Verdict::NotManaged => Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "plan_not_managed",
            "this license is for the annual plan: use your own API key",
        )),
    }
}

// ---------------------------------------------------------------------------------------------
// Public routes
// ---------------------------------------------------------------------------------------------

/// Public liveness probe. It names no vendor and no model: which models sit behind the
/// aliases is the operator's business, logged at start-up and served by `/admin/models`.
async fn healthz() -> Json<Value> {
    Json(json!({
        "ok": true,
        "service": "ubi-api",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

/// `GET /v1/license/status` with the key in `x-api-key`.
///
/// 200 with the claims and the month's figures for any genuine key (`state` says whether it
/// is usable: `valid` | `expired` | `revoked`); 401 with `state: "invalid"` for a key that
/// does not verify. The desktop app merges `month` / `spent_usd` / `budget_usd` into its
/// local verdict.
async fn license_status(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let now = Utc::now();
    let month = month_of(now);
    let budget = state.config.monthly_budget_usd;
    let Some(key) = license_key_from(&headers) else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "state": "invalid", "plan": null, "expires_at": null,
                "month": month, "spent_usd": 0.0, "budget_usd": budget,
                "error": "missing license key (x-api-key)",
            })),
        )
            .into_response();
    };
    let claims = match license::verify_key_with(&key, &state.config.license_pubkey_hex) {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "state": "invalid", "plan": null, "expires_at": null,
                    "month": month, "spent_usd": 0.0, "budget_usd": budget,
                    "error": e.to_string(),
                })),
            )
                .into_response();
        }
    };
    let verdict = match verdict(&state, &claims, now) {
        Ok(v) => v,
        Err(e) => return ApiError::internal(e).into_response(),
    };
    let spent = if claims.plan.is_managed() {
        match state.db.spent_usd(&claims.sub, &month) {
            Ok(v) => v,
            Err(e) => return ApiError::internal(e).into_response(),
        }
    } else {
        0.0
    };
    Json(json!({
        "state": verdict.state(),
        "plan": claims.plan,
        "expires_at": claims.expires_at_utc().to_rfc3339(),
        "days_left": claims.days_left_at(now).max(0),
        "key_hint": key_hint(&key),
        "month": month,
        "spent_usd": spent,
        "budget_usd": if claims.plan.is_managed() { budget } else { 0.0 },
    }))
    .into_response()
}

/// `POST /v1/messages`: Anthropic Messages API passthrough for `monthly_managed` licenses.
async fn messages(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, ApiError> {
    let now = Utc::now();
    let claims = authorize_managed(&state, &headers, now)?;
    if let Some(plan) = header_str(&headers, HEADER_UBIQX_PLAN) {
        if Plan::parse(plan) != Some(Plan::MonthlyManaged) {
            return Err(ApiError::bad_request(format!(
                "{HEADER_UBIQX_PLAN} must be monthly_managed, got `{plan}`"
            )));
        }
    }

    let mut request: Value = serde_json::from_slice(&body)
        .map_err(|e| ApiError::bad_request(format!("request body is not JSON: {e}")))?;
    let Some(obj) = request.as_object_mut() else {
        return Err(ApiError::bad_request("request body must be a JSON object"));
    };
    let alias = obj
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let model = state
        .config
        .resolve_model_alias(&alias)
        .ok_or_else(|| {
            ApiError::bad_request(format!(
                "unknown model `{alias}`: the Ubi proxy accepts only `{}` and `{}`",
                license::UBI_MODEL_FAST,
                license::UBI_MODEL_SMART
            ))
        })?
        .to_string();
    obj.insert("model".into(), Value::String(model.clone()));
    let streaming = obj.get("stream").and_then(Value::as_bool).unwrap_or(false);

    let month = month_of(now);
    let budget = state.config.monthly_budget_usd;
    let spent = state.db.spent_usd(&claims.sub, &month)?;
    if spent >= budget {
        let mut err = ApiError::new(
            StatusCode::PAYMENT_REQUIRED,
            "budget_exhausted",
            format!("the plan's monthly AI budget is used up for {month}; it resets next month"),
        );
        err.extra = Some(json!({ "month": month, "spent_usd": spent, "budget_usd": budget }));
        return Err(err);
    }

    let version = header_str(&headers, "anthropic-version").unwrap_or(ANTHROPIC_VERSION);
    let mut upstream = state
        .http
        .post(format!("{}/v1/messages", state.config.vendor_base_url))
        .header(HEADER_API_KEY, &state.config.vendor_api_key)
        .header("anthropic-version", version)
        .header(CONTENT_TYPE, "application/json")
        .body(serde_json::to_vec(&request).map_err(ApiError::internal)?);
    if let Some(beta) = headers.get("anthropic-beta") {
        upstream = upstream.header("anthropic-beta", beta.clone());
    }
    let resp = upstream.send().await.map_err(|e| {
        warn!(error = %e, "vendor unreachable");
        ApiError::new(
            StatusCode::BAD_GATEWAY,
            "upstream_error",
            format!("vendor unreachable: {e}"),
        )
    })?;

    let status = StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
    let content_type = resp
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|v| HeaderValue::from_bytes(v.as_bytes()).ok())
        .unwrap_or_else(|| HeaderValue::from_static("application/json"));
    let request_id = resp
        .headers()
        .get("request-id")
        .and_then(|v| HeaderValue::from_bytes(v.as_bytes()).ok());

    let mut response = Response::builder()
        .status(status)
        .header(CONTENT_TYPE, content_type.clone());
    if let Some(id) = &request_id {
        response = response.header("request-id", id.clone());
    }
    response = response.header(HEADER_UBIQX_BUDGET_USD, fmt_usd(budget));

    if !status.is_success() {
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| ApiError::new(StatusCode::BAD_GATEWAY, "upstream_error", e.to_string()))?;
        warn!(%status, sub = %claims.sub, model = %model, "vendor error passed through");
        return response
            .header(HEADER_UBIQX_SPENT_USD, fmt_usd(spent))
            .body(Body::from(bytes))
            .map_err(ApiError::internal);
    }

    let is_sse = content_type
        .to_str()
        .map(|c| c.starts_with("text/event-stream"))
        .unwrap_or(false);
    if streaming && is_sse {
        let db = state.db.clone();
        let config = state.config.clone();
        let sub = claims.sub.clone();
        let model_for_ledger = model.clone();
        let on_done = Box::new(move |usage: StreamUsage| {
            if !usage.saw_message_start {
                return;
            }
            let cost = config.prices.cost_usd(&model_for_ledger, &usage.usage);
            let row = UsageRow {
                sub,
                model: model_for_ledger,
                usage: usage.usage,
                cost_usd: cost,
                at: Utc::now(),
            };
            if let Err(e) = db.record_usage(&row) {
                warn!(error = %e, "could not record streamed usage");
            } else {
                info!(sub = %row.sub, model = %row.model, cost_usd = row.cost_usd, "usage (stream)");
            }
        });
        let tap = UsageTap::new(Box::pin(resp.bytes_stream()), on_done);
        return response
            .header(HEADER_UBIQX_SPENT_USD, fmt_usd(spent))
            .body(Body::from_stream(tap))
            .map_err(ApiError::internal);
    }

    let bytes = resp
        .bytes()
        .await
        .map_err(|e| ApiError::new(StatusCode::BAD_GATEWAY, "upstream_error", e.to_string()))?;
    let mut cost = 0.0;
    let mut spent_after = spent;
    match serde_json::from_slice::<Value>(&bytes)
        .ok()
        .and_then(|v| usage_from_message(&v))
    {
        Some(usage) => {
            cost = state.config.prices.cost_usd(&model, &usage);
            let row = UsageRow {
                sub: claims.sub.clone(),
                model: model.clone(),
                usage,
                cost_usd: cost,
                at: now,
            };
            state.db.record_usage(&row)?;
            spent_after += cost;
            info!(sub = %claims.sub, model = %model, cost_usd = cost, "usage");
        }
        None => warn!(sub = %claims.sub, "vendor response without a usage block"),
    }
    response
        .header(HEADER_UBIQX_COST_USD, fmt_usd(cost))
        .header(HEADER_UBIQX_SPENT_USD, fmt_usd(spent_after))
        .body(Body::from(bytes))
        .map_err(ApiError::internal)
}

fn fmt_usd(v: f64) -> String {
    format!("{v:.6}")
}

// ---------------------------------------------------------------------------------------------
// Admin routes
// ---------------------------------------------------------------------------------------------

fn require_admin(state: &AppState, headers: &HeaderMap) -> Result<(), ApiError> {
    let Some(expected) = state.config.admin_token.as_deref() else {
        return Err(ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "admin_disabled",
            "UBI_ADMIN_TOKEN is not configured",
        ));
    };
    let presented = header_str(headers, AUTHORIZATION.as_str())
        .and_then(|v| {
            v.strip_prefix("Bearer ")
                .or_else(|| v.strip_prefix("bearer "))
        })
        .map(str::trim)
        .unwrap_or("");
    if presented.is_empty() || !constant_time_eq(presented.as_bytes(), expected.as_bytes()) {
        return Err(ApiError::new(
            StatusCode::UNAUTHORIZED,
            "authentication_error",
            "admin bearer token missing or wrong",
        ));
    }
    Ok(())
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

#[derive(Debug, Deserialize)]
struct RevokeBody {
    sub: String,
    #[serde(default)]
    reason: Option<String>,
}

async fn admin_revoke(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<RevokeBody>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &headers)?;
    let sub = body.sub.trim().to_string();
    if sub.is_empty() {
        return Err(ApiError::bad_request("sub is required"));
    }
    state.db.revoke(&sub, body.reason.as_deref(), Utc::now())?;
    info!(%sub, "license revoked");
    Ok(Json(json!({ "sub": sub, "revoked": true })))
}

#[derive(Debug, Deserialize)]
struct UsageQuery {
    #[serde(default)]
    month: Option<String>,
}

/// Which vendor and which model ids the aliases resolve to, for the operator.
async fn admin_models(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &headers)?;
    Ok(Json(json!({
        "vendor": state.config.vendor,
        "models": {
            license::UBI_MODEL_FAST: state.config.model_fast,
            license::UBI_MODEL_SMART: state.config.model_smart,
        },
    })))
}

async fn admin_usage(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<UsageQuery>,
) -> Result<Json<Value>, ApiError> {
    require_admin(&state, &headers)?;
    let month = q.month.unwrap_or_else(|| month_of(Utc::now()));
    if !is_month(&month) {
        return Err(ApiError::bad_request("month must be YYYY-MM"));
    }
    let subscribers = state.db.usage_by_sub(&month)?;
    let total: f64 = subscribers.iter().map(|s| s.cost_usd).sum();
    Ok(Json(json!({
        "month": month,
        "budget_usd": state.config.monthly_budget_usd,
        "total_cost_usd": total,
        "subscribers": subscribers,
    })))
}

// ---------------------------------------------------------------------------------------------
// Payment webhook
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebhookEvent {
    #[serde(rename = "subscription.created")]
    SubscriptionCreated,
    #[serde(rename = "subscription.renewed")]
    SubscriptionRenewed,
    #[serde(rename = "subscription.cancelled")]
    SubscriptionCancelled,
}

#[derive(Debug, Deserialize)]
pub struct WebhookBody {
    pub event: WebhookEvent,
    #[serde(default)]
    pub plan: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub months: Option<u32>,
    #[serde(default)]
    pub external_id: Option<String>,
}

/// Checks `x-ubi-signature` (hex HMAC-SHA256 of the raw body, optional `sha256=` prefix).
pub fn verify_webhook_signature(secret: &str, body: &[u8], header: Option<&str>) -> bool {
    let Some(sig) = header.map(|h| h.trim().trim_start_matches("sha256=")) else {
        return false;
    };
    let Ok(sig_bytes) = decode_hex(sig) else {
        return false;
    };
    let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(secret.as_bytes()) else {
        return false;
    };
    mac.update(body);
    mac.verify_slice(&sig_bytes).is_ok()
}

/// Computes the header value a sender must attach (tests, the README example).
pub fn sign_webhook_body(secret: &str, body: &[u8]) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("hmac accepts any key");
    mac.update(body);
    issue::hex_encode(&mac.finalize().into_bytes())
}

fn decode_hex(s: &str) -> Result<Vec<u8>, ()> {
    if s.len() % 2 != 0 || s.is_empty() {
        return Err(());
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| ()))
        .collect()
}

async fn webhook_generic(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<Value>, ApiError> {
    let Some(secret) = state.config.webhook_secret.as_deref() else {
        return Err(ApiError::new(
            StatusCode::SERVICE_UNAVAILABLE,
            "webhook_disabled",
            "UBI_WEBHOOK_SECRET is not configured",
        ));
    };
    // The signature is the webhook's own authentication; a payment platform that can also
    // send the admin bearer may, but it is not required.
    if !verify_webhook_signature(
        secret,
        &body,
        header_str(&headers, HEADER_WEBHOOK_SIGNATURE),
    ) {
        return Err(ApiError::new(
            StatusCode::UNAUTHORIZED,
            "authentication_error",
            format!("{HEADER_WEBHOOK_SIGNATURE} missing or wrong"),
        ));
    }
    let event: WebhookBody = serde_json::from_slice(&body)
        .map_err(|e| ApiError::bad_request(format!("webhook body: {e}")))?;
    let now = Utc::now();

    let email_hash = event
        .email
        .as_deref()
        .map(str::trim)
        .filter(|e| !e.is_empty())
        .map(license::email_hash);

    match event.event {
        WebhookEvent::SubscriptionCancelled => {
            let Some(sub) = subscriber_id(event.external_id.as_deref(), email_hash.as_deref())
            else {
                return Err(ApiError::bad_request("external_id or email is required"));
            };
            state.db.revoke(&sub, Some("subscription.cancelled"), now)?;
            info!(%sub, "subscription cancelled: license revoked");
            Ok(Json(
                json!({ "event": event.event, "sub": sub, "revoked": true }),
            ))
        }
        WebhookEvent::SubscriptionCreated | WebhookEvent::SubscriptionRenewed => {
            let Some(privkey) = state.config.license_privkey_hex.as_deref() else {
                return Err(ApiError::new(
                    StatusCode::SERVICE_UNAVAILABLE,
                    "issuing_disabled",
                    "UBI_LICENSE_PRIVKEY_HEX is not configured",
                ));
            };
            let signing_key = issue::signing_key_from_hex(privkey).map_err(ApiError::internal)?;
            let plan = event.plan.as_deref().and_then(Plan::parse).ok_or_else(|| {
                ApiError::bad_request("plan must be annual_own_key or monthly_managed")
            })?;
            let Some(email_hash) = email_hash else {
                return Err(ApiError::bad_request("email is required"));
            };
            let months = event.months.unwrap_or(match plan {
                Plan::AnnualOwnKey => 12,
                Plan::MonthlyManaged => 1,
            });
            let sub = issue::derive_sub(event.external_id.as_deref(), &email_hash);
            let claims = issue::claims_for(plan, sub.clone(), email_hash.clone(), months, now)
                .map_err(|e| ApiError::bad_request(e.to_string()))?;
            let key = issue::issue(&claims, &signing_key);
            // A renewal (or a re-purchase) reinstates a cancelled subscriber.
            state.db.unrevoke(&sub)?;
            state.db.record_license(&IssuedLicense {
                sub: sub.clone(),
                plan: plan.id().into(),
                email_hash: email_hash.clone(),
                external_id: event.external_id.clone(),
                event: serde_json::to_value(event.event)
                    .ok()
                    .and_then(|v| v.as_str().map(String::from))
                    .unwrap_or_default(),
                key_hint: key_hint(&key),
                issued_at: now,
                expires_at: claims.expires_at_utc(),
            })?;
            info!(%sub, plan = plan.id(), months, "license issued");
            Ok(Json(json!({
                "event": event.event,
                "sub": sub,
                "plan": plan,
                "email_hash": email_hash,
                "months": months,
                "expires_at": claims.expires_at_utc().to_rfc3339(),
                "key_hint": key_hint(&key),
                "key": key,
            })))
        }
    }
}

fn subscriber_id(external_id: Option<&str>, email_hash: Option<&str>) -> Option<String> {
    match (
        external_id.map(str::trim).filter(|s| !s.is_empty()),
        email_hash,
    ) {
        (Some(id), _) => Some(issue::derive_sub(Some(id), "")),
        (None, Some(h)) => Some(issue::derive_sub(None, h)),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn webhook_signature_round_trip() {
        let body = br#"{"event":"subscription.created"}"#;
        let sig = sign_webhook_body("s3cret", body);
        assert!(verify_webhook_signature("s3cret", body, Some(&sig)));
        assert!(verify_webhook_signature(
            "s3cret",
            body,
            Some(&format!("sha256={sig}"))
        ));
        assert!(!verify_webhook_signature("other", body, Some(&sig)));
        assert!(!verify_webhook_signature("s3cret", b"{}", Some(&sig)));
        assert!(!verify_webhook_signature("s3cret", body, None));
        assert!(!verify_webhook_signature("s3cret", body, Some("zz")));
        assert!(!verify_webhook_signature("s3cret", body, Some("")));
    }

    #[test]
    fn error_body_is_anthropic_shaped() {
        let mut e = ApiError::new(StatusCode::PAYMENT_REQUIRED, "budget_exhausted", "nope");
        e.extra = Some(json!({ "month": "2026-09" }));
        let b = e.body();
        assert_eq!(b["type"], "error");
        assert_eq!(b["error"]["type"], "budget_exhausted");
        assert_eq!(b["error"]["message"], "nope");
        assert_eq!(b["month"], "2026-09");
    }

    #[test]
    fn license_key_header_forms() {
        let mut h = HeaderMap::new();
        assert_eq!(license_key_from(&h), None);
        h.insert(AUTHORIZATION, HeaderValue::from_static("Bearer UBIQX-A-B"));
        assert_eq!(license_key_from(&h).as_deref(), Some("UBIQX-A-B"));
        h.insert(HEADER_API_KEY, HeaderValue::from_static(" UBIQX-C-D "));
        assert_eq!(license_key_from(&h).as_deref(), Some("UBIQX-C-D"));
    }

    #[test]
    fn subscriber_id_prefers_external_id() {
        let h = license::email_hash("a@b.c");
        assert_eq!(
            subscriber_id(Some("ext"), Some(&h)),
            Some(issue::derive_sub(Some("ext"), &h))
        );
        assert_eq!(
            subscriber_id(None, Some(&h)),
            Some(issue::derive_sub(None, &h))
        );
        assert_eq!(subscriber_id(Some(" "), None), None);
    }
}
