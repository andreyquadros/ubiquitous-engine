//! [`UpdateFeedSource`] over HTTP: downloads `latest.json` from the rolling release.
//!
//! One small GET every few hours, so the client is built per call (no TLS state to keep
//! around) with a short timeout, the product's User-Agent and a cache-busting query
//! parameter, because GitHub's release CDN may otherwise serve a stale copy of a file the
//! publisher just replaced.

use std::time::Duration;

use async_trait::async_trait;
use ubiqx_core::ports::UpdateFeedSource;
use ubiqx_core::{CoreError, CoreResult, UpdateFeed};

/// Total time allowed for one feed download.
pub const FEED_TIMEOUT: Duration = Duration::from_secs(15);

/// Largest feed body accepted, so a wrong URL answering with a page never fills memory.
const MAX_BODY_BYTES: usize = 512 * 1024;

#[derive(Debug, Default, Clone, Copy)]
pub struct HttpUpdateFeed;

impl HttpUpdateFeed {
    pub fn new() -> Self {
        Self
    }

    /// `url` with a `_=<unix ms>` query parameter appended, keeping any existing query.
    pub fn cache_busted(url: &str) -> String {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let sep = if url.contains('?') { '&' } else { '?' };
        format!("{url}{sep}_={stamp}")
    }
}

#[async_trait]
impl UpdateFeedSource for HttpUpdateFeed {
    async fn fetch(&self, url: &str) -> CoreResult<UpdateFeed> {
        let client = reqwest::Client::builder()
            .timeout(FEED_TIMEOUT)
            .user_agent(concat!("ubiqX/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| CoreError::Other(format!("update feed client: {e}")))?;
        let response = client
            .get(Self::cache_busted(url))
            .header(reqwest::header::ACCEPT, "application/json")
            .header(reqwest::header::CACHE_CONTROL, "no-cache")
            .send()
            .await
            .map_err(|e| CoreError::Other(format!("update feed: {e}")))?;
        let status = response.status();
        if !status.is_success() {
            return Err(if status == reqwest::StatusCode::NOT_FOUND {
                CoreError::NotFound(format!("update feed: {url} answered 404"))
            } else {
                CoreError::Other(format!("update feed: {url} answered HTTP {status}"))
            });
        }
        let body = response
            .bytes()
            .await
            .map_err(|e| CoreError::Other(format!("update feed body: {e}")))?;
        if body.len() > MAX_BODY_BYTES {
            return Err(CoreError::Other(format!(
                "update feed: body too large ({} bytes)",
                body.len()
            )));
        }
        serde_json::from_slice(&body)
            .map_err(|e| CoreError::Other(format!("update feed is not valid JSON: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_busting_keeps_existing_query() {
        let a = HttpUpdateFeed::cache_busted("https://x.y/latest.json");
        assert!(a.starts_with("https://x.y/latest.json?_="));
        let b = HttpUpdateFeed::cache_busted("https://x.y/latest.json?token=1");
        assert!(b.starts_with("https://x.y/latest.json?token=1&_="));
    }
}
