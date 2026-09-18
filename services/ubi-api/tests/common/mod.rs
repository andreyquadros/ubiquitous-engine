//! Shared harness: an in-process proxy on 127.0.0.1 and an in-process mock vendor.

#![allow(dead_code)]

use std::sync::{Arc, Mutex};

use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::Router;
use chrono::{Duration, Utc};
use ed25519_dalek::SigningKey;
use serde_json::{json, Value};
use ubi_api::issue::{self, KeyPairHex};
use ubi_api::{AppState, Config};
use ubiqx_core::license::{LicenseClaims, Plan, CLAIMS_VERSION};

pub const VENDOR_KEY: &str = "sk-ant-test-vendor-key";
pub const ADMIN_TOKEN: &str = "admin-t0ken";
pub const WEBHOOK_SECRET: &str = "hook-s3cret";

/// What the mock vendor saw and how it must answer.
#[derive(Debug, Clone)]
pub struct VendorCall {
    pub headers: Vec<(String, String)>,
    pub body: Value,
}

#[derive(Clone)]
pub struct MockVendor {
    pub calls: Arc<Mutex<Vec<VendorCall>>>,
    pub mode: Arc<Mutex<VendorMode>>,
}

#[derive(Debug, Clone)]
pub enum VendorMode {
    /// A normal message with the given usage.
    Json {
        input_tokens: u64,
        output_tokens: u64,
        cache_read: u64,
    },
    /// An SSE stream with the given usage.
    Sse {
        input_tokens: u64,
        output_tokens: u64,
    },
    /// An error status with an Anthropic-shaped body.
    Error(u16),
}

impl MockVendor {
    pub fn calls(&self) -> Vec<VendorCall> {
        self.calls.lock().unwrap().clone()
    }
    pub fn set_mode(&self, mode: VendorMode) {
        *self.mode.lock().unwrap() = mode;
    }
    pub fn last_model(&self) -> String {
        self.calls()
            .last()
            .and_then(|c| {
                c.body
                    .get("model")
                    .and_then(Value::as_str)
                    .map(String::from)
            })
            .unwrap_or_default()
    }
}

async fn vendor_messages(
    State(vendor): State<MockVendor>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let body: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
    let hs = headers
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();
    vendor.calls.lock().unwrap().push(VendorCall {
        headers: hs,
        body: body.clone(),
    });
    let model = body
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or("?")
        .to_string();
    let mode = vendor.mode.lock().unwrap().clone();
    match mode {
        VendorMode::Json {
            input_tokens,
            output_tokens,
            cache_read,
        } => (
            [
                ("content-type", "application/json"),
                ("request-id", "req_mock_1"),
            ],
            serde_json::to_vec(&json!({
                "id": "msg_mock",
                "type": "message",
                "role": "assistant",
                "model": model,
                "content": [{"type": "text", "text": "{\"category\":\"work\"}"}],
                "stop_reason": "end_turn",
                "usage": {
                    "input_tokens": input_tokens,
                    "output_tokens": output_tokens,
                    "cache_read_input_tokens": cache_read,
                    "cache_creation_input_tokens": 0
                }
            }))
            .unwrap(),
        )
            .into_response(),
        VendorMode::Sse {
            input_tokens,
            output_tokens,
        } => {
            let sse = format!(
                "event: message_start\ndata: {}\n\nevent: content_block_start\ndata: {}\n\nevent: content_block_delta\ndata: {}\n\nevent: content_block_stop\ndata: {}\n\nevent: message_delta\ndata: {}\n\nevent: message_stop\ndata: {}\n\n",
                json!({"type":"message_start","message":{"id":"msg_mock","type":"message","role":"assistant","model":model,"content":[],"usage":{"input_tokens":input_tokens,"output_tokens":1}}}),
                json!({"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}),
                json!({"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Olá"}}),
                json!({"type":"content_block_stop","index":0}),
                json!({"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":output_tokens}}),
                json!({"type":"message_stop"}),
            );
            ([("content-type", "text/event-stream")], sse).into_response()
        }
        VendorMode::Error(status) => (
            StatusCode::from_u16(status).unwrap(),
            [("content-type", "application/json")],
            serde_json::to_vec(&json!({
                "type": "error",
                "error": {"type": "overloaded_error", "message": "mock says no"}
            }))
            .unwrap(),
        )
            .into_response(),
    }
}

pub async fn spawn_router(router: Router) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    format!("http://{addr}")
}

pub async fn spawn_vendor() -> (String, MockVendor) {
    let vendor = MockVendor {
        calls: Arc::new(Mutex::new(Vec::new())),
        mode: Arc::new(Mutex::new(VendorMode::Json {
            input_tokens: 1_000,
            output_tokens: 100,
            cache_read: 0,
        })),
    };
    let router = Router::new()
        .route("/v1/messages", post(vendor_messages))
        .with_state(vendor.clone());
    (spawn_router(router).await, vendor)
}

pub struct Harness {
    pub base: String,
    pub vendor_base: String,
    pub vendor: MockVendor,
    pub state: AppState,
    pub pair: KeyPairHex,
    pub signing_key: SigningKey,
    pub http: reqwest::Client,
}

impl Harness {
    pub async fn start() -> Self {
        Self::start_with(|_| {}).await
    }

    pub async fn start_with(tweak: impl FnOnce(&mut Config)) -> Self {
        let (vendor_base, vendor) = spawn_vendor().await;
        let pair = issue::keygen();
        let mut config = Config::minimal(pair.public_hex.clone(), VENDOR_KEY);
        config.vendor_base_url = vendor_base.clone();
        config.admin_token = Some(ADMIN_TOKEN.into());
        config.webhook_secret = Some(WEBHOOK_SECRET.into());
        config.license_privkey_hex = Some(pair.private_hex.clone());
        config.model_fast = "mock-fast-model".into();
        config.model_smart = "mock-smart-model".into();
        tweak(&mut config);
        let state = AppState::new(config).unwrap();
        let base = spawn_router(ubi_api::router(state.clone())).await;
        let signing_key = issue::signing_key_from_hex(&pair.private_hex).unwrap();
        Harness {
            base,
            vendor_base,
            vendor,
            state,
            pair,
            signing_key,
            http: reqwest::Client::new(),
        }
    }

    pub fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base)
    }

    /// A key for `plan` valid for `days` from now (negative = already expired).
    pub fn key(&self, plan: Plan, sub: &str, days: i64) -> String {
        let now = Utc::now();
        let claims = LicenseClaims {
            v: CLAIMS_VERSION,
            plan,
            sub: sub.into(),
            email_hash: ubiqx_core::license::email_hash(&format!("{sub}@example.com")),
            issued_at: (now - Duration::days(1)).timestamp(),
            expires_at: (now + Duration::days(days)).timestamp(),
            seats: 1,
        };
        issue::issue(&claims, &self.signing_key)
    }

    pub fn managed_key(&self, sub: &str) -> String {
        self.key(Plan::MonthlyManaged, sub, 30)
    }

    pub fn messages_body(model: &str, stream: bool) -> Value {
        json!({
            "model": model,
            "max_tokens": 256,
            "stream": stream,
            "messages": [{"role": "user", "content": "classify this"}]
        })
    }

    pub async fn post_messages(&self, key: &str, body: &Value) -> reqwest::Response {
        self.http
            .post(self.url("/v1/messages"))
            .header("x-api-key", key)
            .header("x-ubiqx-plan", "monthly_managed")
            .header("anthropic-version", "2023-06-01")
            .json(body)
            .send()
            .await
            .unwrap()
    }

    pub async fn webhook(&self, body: &Value, secret: &str) -> reqwest::Response {
        let raw = serde_json::to_vec(body).unwrap();
        let sig = ubi_api::routes::sign_webhook_body(secret, &raw);
        self.http
            .post(self.url("/admin/webhooks/generic"))
            .header("content-type", "application/json")
            .header("x-ubi-signature", sig)
            .body(raw)
            .send()
            .await
            .unwrap()
    }
}
