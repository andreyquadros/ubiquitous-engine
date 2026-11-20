//! Usage extraction from vendor responses: the `usage` block of a JSON message and, for
//! streaming calls, the `message_start` / `message_delta` events of the SSE stream.
//!
//! [`UsageTap`] wraps the upstream byte stream: bytes are forwarded untouched to the client
//! while the SSE events are parsed on the side. When the stream ends (or the client goes
//! away) the collected usage is handed to a callback that records it.

use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::Bytes;
use futures_util::Stream;
use serde_json::Value;

use crate::config::TokenUsage;

/// Reads `input_tokens`, `output_tokens`, `cache_read_input_tokens` and
/// `cache_creation_input_tokens` from a `usage` object.
pub fn usage_from_value(usage: &Value) -> TokenUsage {
    let get = |k: &str| usage.get(k).and_then(Value::as_u64).unwrap_or(0);
    TokenUsage {
        input_tokens: get("input_tokens"),
        output_tokens: get("output_tokens"),
        cache_read_tokens: get("cache_read_input_tokens"),
        cache_write_tokens: get("cache_creation_input_tokens"),
    }
}

/// Usage of a non-streaming message (`response.usage`).
pub fn usage_from_message(body: &Value) -> Option<TokenUsage> {
    body.get("usage").map(usage_from_value)
}

/// Accumulates usage across the events of one streamed message.
#[derive(Debug, Default, Clone)]
pub struct StreamUsage {
    pub usage: TokenUsage,
    pub saw_message_start: bool,
}

impl StreamUsage {
    /// Feeds one decoded SSE event (its `data:` JSON).
    pub fn apply_event(&mut self, event: &Value) {
        match event.get("type").and_then(Value::as_str) {
            Some("message_start") => {
                if let Some(u) = event.pointer("/message/usage") {
                    let start = usage_from_value(u);
                    self.usage.input_tokens = start.input_tokens;
                    self.usage.cache_read_tokens = start.cache_read_tokens;
                    self.usage.cache_write_tokens = start.cache_write_tokens;
                    self.usage.output_tokens = self.usage.output_tokens.max(start.output_tokens);
                    self.saw_message_start = true;
                }
            }
            Some("message_delta") => {
                if let Some(u) = event.get("usage") {
                    // `message_delta.usage` is cumulative. Newer API versions also repeat the
                    // input side there; older ones only send `output_tokens`.
                    let delta = usage_from_value(u);
                    self.usage.output_tokens = self.usage.output_tokens.max(delta.output_tokens);
                    if u.get("input_tokens").is_some() {
                        self.usage.input_tokens = delta.input_tokens;
                        self.usage.cache_read_tokens = delta.cache_read_tokens;
                        self.usage.cache_write_tokens = delta.cache_write_tokens;
                    }
                }
            }
            _ => {}
        }
    }

    /// Feeds raw SSE bytes, splitting complete events out of `buf` (the incomplete tail
    /// stays in `buf` for the next chunk).
    pub fn feed(&mut self, buf: &mut Vec<u8>, chunk: &[u8]) {
        buf.extend_from_slice(chunk);
        while let Some((event_bytes, sep_len)) = find_event_end(buf) {
            let event = buf[..event_bytes].to_vec();
            buf.drain(..event_bytes + sep_len);
            self.apply_raw_event(&event);
        }
    }

    fn apply_raw_event(&mut self, event: &[u8]) {
        let Ok(text) = std::str::from_utf8(event) else {
            return;
        };
        let data: String = text
            .lines()
            .filter_map(|l| l.strip_prefix("data:"))
            .map(str::trim)
            .collect::<Vec<_>>()
            .join("\n");
        if data.is_empty() {
            return;
        }
        if let Ok(v) = serde_json::from_str::<Value>(&data) {
            self.apply_event(&v);
        }
    }
}

/// Position and length of the first blank line (`\n\n` or `\r\n\r\n`) ending an event.
fn find_event_end(buf: &[u8]) -> Option<(usize, usize)> {
    let lf = buf.windows(2).position(|w| w == b"\n\n").map(|p| (p, 2));
    let crlf = buf
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .map(|p| (p, 4));
    match (lf, crlf) {
        (Some(a), Some(b)) => Some(if a.0 <= b.0 { a } else { b }),
        (a, b) => a.or(b),
    }
}

type Inner = Pin<Box<dyn Stream<Item = Result<Bytes, reqwest::Error>> + Send>>;
type OnDone = Box<dyn FnOnce(StreamUsage) + Send>;

/// Passes bytes through while collecting usage; see the module docs.
pub struct UsageTap {
    inner: Inner,
    buf: Vec<u8>,
    usage: StreamUsage,
    on_done: Option<OnDone>,
}

impl UsageTap {
    pub fn new(inner: Inner, on_done: OnDone) -> Self {
        UsageTap {
            inner,
            buf: Vec::new(),
            usage: StreamUsage::default(),
            on_done: Some(on_done),
        }
    }

    fn finish(&mut self) {
        if let Some(f) = self.on_done.take() {
            f(std::mem::take(&mut self.usage));
        }
    }
}

impl Stream for UsageTap {
    type Item = Result<Bytes, std::io::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.inner.as_mut().poll_next(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Some(Ok(chunk))) => {
                let mut buf = std::mem::take(&mut self.buf);
                self.usage.feed(&mut buf, &chunk);
                self.buf = buf;
                Poll::Ready(Some(Ok(chunk)))
            }
            Poll::Ready(Some(Err(e))) => {
                self.finish();
                Poll::Ready(Some(Err(std::io::Error::other(e))))
            }
            Poll::Ready(None) => {
                self.finish();
                Poll::Ready(None)
            }
        }
    }
}

impl Drop for UsageTap {
    fn drop(&mut self) {
        // Client disconnected mid-stream: record what the vendor already billed us for.
        self.finish();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const START: &str = r#"event: message_start
data: {"type":"message_start","message":{"id":"msg_1","type":"message","role":"assistant","content":[],"model":"m","usage":{"input_tokens":25,"output_tokens":1,"cache_read_input_tokens":4,"cache_creation_input_tokens":0}}}

"#;
    const DELTA: &str = r#"event: message_delta
data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":15}}

event: message_stop
data: {"type":"message_stop"}

"#;

    #[test]
    fn stream_usage_is_collected_across_chunks() {
        let mut s = StreamUsage::default();
        let mut buf = Vec::new();
        let all = format!("{START}{DELTA}");
        // Feed in awkward 7-byte chunks so events straddle chunk boundaries.
        for chunk in all.as_bytes().chunks(7) {
            s.feed(&mut buf, chunk);
        }
        assert!(s.saw_message_start);
        assert_eq!(
            s.usage,
            TokenUsage {
                input_tokens: 25,
                output_tokens: 15,
                cache_read_tokens: 4,
                cache_write_tokens: 0,
            }
        );
        assert!(buf.is_empty());
    }

    #[test]
    fn crlf_events_and_cumulative_input_in_delta() {
        let mut s = StreamUsage::default();
        let mut buf = Vec::new();
        let ev = "data: {\"type\":\"message_start\",\"message\":{\"usage\":{\"input_tokens\":3,\"output_tokens\":0}}}\r\n\r\ndata: {\"type\":\"message_delta\",\"usage\":{\"input_tokens\":30,\"output_tokens\":7}}\r\n\r\n";
        s.feed(&mut buf, ev.as_bytes());
        assert_eq!(s.usage.input_tokens, 30);
        assert_eq!(s.usage.output_tokens, 7);
    }

    #[test]
    fn non_streaming_usage() {
        let body: Value = serde_json::from_str(
            r#"{"id":"m","usage":{"input_tokens":100,"output_tokens":20,"cache_creation_input_tokens":50}}"#,
        )
        .unwrap();
        let u = usage_from_message(&body).unwrap();
        assert_eq!(u.input_tokens, 100);
        assert_eq!(u.output_tokens, 20);
        assert_eq!(u.cache_write_tokens, 50);
        assert_eq!(u.cache_read_tokens, 0);
        assert!(usage_from_message(&serde_json::json!({})).is_none());
    }
}
