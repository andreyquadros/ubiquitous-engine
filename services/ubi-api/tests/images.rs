//! Requests that carry an image (a classification screenshot, or the reduced frame of the
//! app's screen check): the image reaches the vendor unchanged, a body above the stated
//! limit is refused, and no part of a request body reaches the log or the ledger.

mod common;

use std::io::Write;
use std::sync::{Arc, Mutex};

use common::{Harness, VendorMode};
use serde_json::{json, Value};
use tracing::subscriber::DefaultGuard;
use tracing_subscriber::fmt::MakeWriter;
use ubi_api::routes::MESSAGES_BODY_LIMIT;

const SYSTEM_TEXT: &str = "SYSTEM-TEXT-MARKER: you are a content classifier.";
const USER_TEXT: &str = "USER-TEXT-MARKER: classify this screenshot.";

/// `chars` characters of the base64 alphabet standing in for an encoded image: a pattern
/// that is easy to find again, so any 48 characters of it identify it.
fn image_base64(chars: usize) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    (0..chars)
        .map(|i| ALPHABET[(i * 7 + i / 64) % 64] as char)
        .collect()
}

/// A screen-check request whose serialised body is exactly `len` bytes.
fn check_body_of_len(len: usize) -> Value {
    let overhead = serde_json::to_vec(&check_body("ubi-fast", ""))
        .unwrap()
        .len();
    let body = check_body("ubi-fast", &image_base64(len - overhead));
    assert_eq!(serde_json::to_vec(&body).unwrap().len(), len);
    body
}

/// A request shaped like the app's screen check: a system prompt, one image, one short
/// text and a closed schema.
fn check_body(model: &str, image: &str) -> Value {
    json!({
        "model": model,
        "max_tokens": 64,
        "system": [{"type": "text", "text": SYSTEM_TEXT}],
        "messages": [{"role": "user", "content": [
            {"type": "image", "source": {
                "type": "base64", "media_type": "image/jpeg", "data": image}},
            {"type": "text", "text": USER_TEXT}
        ]}],
        "output_config": {"format": {"type": "json_schema", "schema": {
            "type": "object",
            "properties": {
                "verdict": {"type": "string", "enum": ["explicit", "suggestive", "none", "unsure"]},
                "confidence": {"type": "string", "enum": ["high", "medium", "low"]}
            },
            "required": ["verdict", "confidence"],
            "additionalProperties": false
        }}}
    })
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
    /// Captures everything logged on this thread at every level until the guard drops. The
    /// test runs on a current-thread runtime, so the proxy, the mock vendor and the HTTP
    /// client all log here.
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

    fn text(&self) -> String {
        String::from_utf8_lossy(&self.0.lock().unwrap()).into_owned()
    }
}

// ---------------------------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------------------------

#[tokio::test]
async fn an_image_block_is_forwarded_unchanged() {
    let h = Harness::start().await;
    let key = h.managed_key("sub_img");
    let image = image_base64(64_000);
    let sent = check_body("ubi-fast", &image);

    let resp = h.post_messages(&key, &sent).await;
    assert_eq!(resp.status(), 200);
    assert!(resp.headers().contains_key("x-ubiqx-cost-usd"));

    // The vendor got the request as it was sent, with the alias replaced and nothing else
    // touched: the image, the texts, the schema.
    let calls = h.vendor.calls();
    assert_eq!(calls.len(), 1);
    let mut expected = sent.clone();
    expected["model"] = json!("mock-fast-model");
    assert_eq!(calls[0].body, expected);
    assert_eq!(
        calls[0].body["messages"][0]["content"][0]["source"]["data"]
            .as_str()
            .map(str::len),
        Some(image.len())
    );

    // The ledger has one row for it, and that row is who, when, which model and how much.
    let usage: Value = h
        .http
        .get(h.url("/admin/usage"))
        .bearer_auth(common::ADMIN_TOKEN)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let listed = usage.to_string();
    assert!(listed.contains("sub_img"), "{listed}");
    assert!(!listed.contains(&image[..48]), "{listed}");
    assert!(!listed.contains("MARKER"), "{listed}");
}

#[tokio::test]
async fn the_ledger_file_holds_nothing_of_a_request_body() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("ledger.sqlite");
    let h = Harness::start_with(|config| {
        config.db_path = db_path.to_string_lossy().into_owned();
    })
    .await;
    let key = h.managed_key("sub_ledger");
    let image = image_base64(64_000);
    let resp = h.post_messages(&key, &check_body("ubi-fast", &image)).await;
    assert_eq!(resp.status(), 200);

    let mut bytes = std::fs::read(&db_path).unwrap();
    bytes.extend(std::fs::read(dir.path().join("ledger.sqlite-wal")).unwrap_or_default());
    let holds = |needle: &str| bytes.windows(needle.len()).any(|w| w == needle.as_bytes());
    assert!(
        holds("sub_ledger") && holds("mock-fast-model"),
        "the request was recorded: the search finds what the ledger stores"
    );
    assert!(!holds(&image[..48]), "no image");
    assert!(
        !holds("SYSTEM-TEXT-MARKER") && !holds("USER-TEXT-MARKER"),
        "no text"
    );
}

#[tokio::test]
async fn a_body_over_the_limit_is_413() {
    let h = Harness::start().await;
    let key = h.managed_key("sub_big");

    // The limit is the stated 4 MiB, which is above what the framework allows by default
    // (2 MiB). A body of exactly that size passes and reaches the vendor whole.
    assert_eq!(MESSAGES_BODY_LIMIT, 4 * 1024 * 1024);
    let at_the_limit = check_body_of_len(MESSAGES_BODY_LIMIT);
    let resp = h.post_messages(&key, &at_the_limit).await;
    assert_eq!(resp.status(), 200);
    assert_eq!(h.vendor.calls().len(), 1);
    assert_eq!(
        h.vendor.calls()[0].body["messages"],
        at_the_limit["messages"]
    );

    // One byte more: refused in the shape the app's client reads, and never forwarded.
    let over = check_body_of_len(MESSAGES_BODY_LIMIT + 1);
    let resp = h.post_messages(&key, &over).await;
    assert_eq!(resp.status(), 413);
    let error: Value = resp.json().await.unwrap();
    assert_eq!(error["type"], "error");
    assert_eq!(error["error"]["type"], "request_too_large");
    assert!(error["error"]["message"]
        .as_str()
        .is_some_and(|m| m.contains("4 MiB")));
    assert_eq!(h.vendor.calls().len(), 1, "the vendor never saw it");

    // The licence is still checked first: an oversized body without a key is a 401.
    let resp = h
        .http
        .post(h.url("/v1/messages"))
        .json(&over)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);

    // Nothing was billed for the refused requests.
    let status: Value = h
        .http
        .get(h.url("/v1/license/status"))
        .header("x-api-key", &key)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let one_call = 1_000.0 * 2.0 / 1e6 + 100.0 * 10.0 / 1e6;
    assert!(
        (status["spent_usd"].as_f64().unwrap() - one_call).abs() < 1e-9,
        "{status}"
    );
}

#[tokio::test]
async fn no_request_body_reaches_the_log() {
    let h = Harness::start().await;
    let key = h.managed_key("sub_quiet");
    let image = image_base64(64_000);
    let (capture, _guard) = Capture::start();

    // A normal call, a streamed one, a vendor error that is passed through, a request the
    // proxy refuses for its model, one that is not JSON, and one that is too large.
    let resp = h.post_messages(&key, &check_body("ubi-fast", &image)).await;
    assert_eq!(resp.status(), 200);

    h.vendor.set_mode(VendorMode::Sse {
        input_tokens: 700,
        output_tokens: 20,
    });
    let mut streamed = check_body("ubi-fast", &image);
    streamed["stream"] = json!(true);
    let resp = h.post_messages(&key, &streamed).await;
    assert_eq!(resp.status(), 200);
    resp.bytes().await.unwrap();

    h.vendor.set_mode(VendorMode::Error(529));
    let resp = h
        .post_messages(&key, &check_body("ubi-smart", &image))
        .await;
    assert_eq!(resp.status(), 529);

    let resp = h
        .post_messages(&key, &check_body("claude-opus-5", &image))
        .await;
    assert_eq!(resp.status(), 400);

    let resp = h
        .http
        .post(h.url("/v1/messages"))
        .header("x-api-key", &key)
        .header("content-type", "application/json")
        .body(format!("NOT-JSON-MARKER {image}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);

    let resp = h
        .post_messages(&key, &check_body_of_len(MESSAGES_BODY_LIMIT + 1))
        .await;
    assert_eq!(resp.status(), 413);

    let logged = capture.text();
    // The capture is live: the proxy's own lines about these requests are in it.
    assert!(logged.contains("usage"), "{logged}");
    assert!(logged.contains("sub_quiet"), "{logged}");
    assert!(logged.contains("vendor error passed through"), "{logged}");
    // And nothing of what the requests carried is.
    assert!(!logged.contains(&image[..48]), "an image in the log");
    for marker in ["SYSTEM-TEXT-MARKER", "USER-TEXT-MARKER", "NOT-JSON-MARKER"] {
        assert!(!logged.contains(marker), "{marker} in the log: {logged}");
    }
    for word in ["verdict", "explicit", "json_schema", "image/jpeg"] {
        assert!(!logged.contains(word), "`{word}` in the log: {logged}");
    }
    // Nor the licence key that came with them.
    assert!(!logged.contains(&key), "the licence key in the log");
}
