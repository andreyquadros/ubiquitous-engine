//! The Ubi proxy's license endpoint.
//!
//! The Messages API side of the proxy is served by [`crate::AnthropicClient`] with
//! [`crate::AnthropicConfig::for_ubi`]; this module holds the one call that is not part of
//! the Messages API: `GET /v1/license/status`, which tells a `monthly_managed` subscriber how
//! much of the month's budget is spent. The engine merges the answer into the
//! [`ubiqx_core::LicenseStatus`] it shows.

use std::fmt;
use std::time::Duration;

use serde::Deserialize;
use tracing::debug;
use ubiqx_core::license::{Plan, UBIQX_API_BASE_DEFAULT};
use ubiqx_core::{CoreError, CoreResult, ManagedUsage};

/// Timeout of the status call: it runs at start-up and after a key is entered, never on the
/// critical path of a classification.
pub const STATUS_TIMEOUT: Duration = Duration::from_secs(15);

/// What `GET /v1/license/status` answers (the proxy's own verdict; the app keeps its local
/// one when they disagree, the local check being offline and signature-based).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct RemoteLicenseStatus {
    #[serde(default)]
    pub plan: Option<Plan>,
    #[serde(default)]
    pub expires_at: Option<String>,
    /// `YYYY-MM` (UTC).
    pub month: String,
    pub spent_usd: f64,
    pub budget_usd: f64,
    /// `valid` | `expired` | `revoked` | `invalid` as the proxy words it.
    #[serde(default)]
    pub state: Option<String>,
}

impl RemoteLicenseStatus {
    pub fn managed_usage(&self) -> ManagedUsage {
        ManagedUsage {
            month: self.month.clone(),
            spent_usd: self.spent_usd,
            budget_usd: self.budget_usd,
        }
    }
}

/// Client of the proxy's license endpoint.
pub struct UbiLicenseClient {
    http: reqwest::Client,
    base_url: String,
}

impl fmt::Debug for UbiLicenseClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UbiLicenseClient")
            .field("base_url", &self.base_url)
            .finish()
    }
}

impl UbiLicenseClient {
    /// A client for the proxy at `api_base` (blank → [`UBIQX_API_BASE_DEFAULT`]). Fails only
    /// when the TLS backend cannot be initialised.
    pub fn new(api_base: impl Into<String>) -> CoreResult<Self> {
        let base_url = {
            let b: String = api_base.into();
            let b = b.trim().trim_end_matches('/').to_string();
            if b.is_empty() {
                UBIQX_API_BASE_DEFAULT.to_string()
            } else {
                b
            }
        };
        let http = reqwest::Client::builder()
            .timeout(STATUS_TIMEOUT)
            .user_agent(concat!("ubiqx/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| CoreError::Ai(format!("cannot build http client: {e}")))?;
        Ok(Self { http, base_url })
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// `GET /v1/license/status` with `key` as `x-api-key`. A 401/403 (unknown, revoked or
    /// expired key as far as the proxy is concerned) yields `CoreError::Ai("license
    /// rejected …")`; any other failure (network, 5xx, unparseable body) a transient
    /// [`CoreError::Ai`]; the caller treats both as "no managed usage known".
    pub async fn status(&self, key: &str) -> CoreResult<RemoteLicenseStatus> {
        let url = format!("{}/v1/license/status", self.base_url);
        let resp = self
            .http
            .get(&url)
            .header("x-api-key", key.trim())
            .send()
            .await
            .map_err(|e| CoreError::Ai(format!("license status: network error: {e}")))?;
        let code = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();
        match code {
            200..=299 => {
                let parsed: RemoteLicenseStatus = serde_json::from_str(&body).map_err(|e| {
                    CoreError::Ai(format!("license status: unparseable answer: {e}"))
                })?;
                debug!(
                    month = %parsed.month,
                    spent_usd = parsed.spent_usd,
                    budget_usd = parsed.budget_usd,
                    state = parsed.state.as_deref().unwrap_or("-"),
                    "ubi: license status"
                );
                Ok(parsed)
            }
            401 | 403 => Err(CoreError::Ai(format!(
                "license rejected by the Ubi proxy (HTTP {code}): {}",
                body.chars().take(200).collect::<String>()
            ))),
            _ => Err(CoreError::Ai(format!(
                "license status: HTTP {code}: {}",
                body.chars().take(200).collect::<String>()
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_url_defaults_and_trims() {
        assert_eq!(
            UbiLicenseClient::new("  ").unwrap().base_url(),
            UBIQX_API_BASE_DEFAULT
        );
        assert_eq!(
            UbiLicenseClient::new("http://127.0.0.1:9/")
                .unwrap()
                .base_url(),
            "http://127.0.0.1:9"
        );
    }

    #[test]
    fn remote_status_parses_the_contract() {
        let s: RemoteLicenseStatus = serde_json::from_str(
            r#"{"plan":"monthly_managed","expires_at":"2026-10-18T00:00:00Z","month":"2026-09","spent_usd":1.25,"budget_usd":6.0,"state":"valid"}"#,
        )
        .unwrap();
        assert_eq!(s.plan, Some(Plan::MonthlyManaged));
        assert_eq!(
            s.managed_usage(),
            ManagedUsage {
                month: "2026-09".into(),
                spent_usd: 1.25,
                budget_usd: 6.0
            }
        );
        // Only the usage fields are required.
        let minimal: RemoteLicenseStatus =
            serde_json::from_str(r#"{"month":"2026-09","spent_usd":0,"budget_usd":6}"#).unwrap();
        assert_eq!(minimal.plan, None);
        assert_eq!(minimal.managed_usage().budget_usd, 6.0);
    }
}
