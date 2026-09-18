//! Configuration from environment variables (see README.md for the full table).

use std::collections::BTreeMap;
use std::net::SocketAddr;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

/// Production endpoint of the only vendor supported today.
pub const ANTHROPIC_BASE_URL: &str = "https://api.anthropic.com";
/// Value of the `anthropic-version` header when the client does not send one.
pub const ANTHROPIC_VERSION: &str = "2023-06-01";

pub const DEFAULT_LISTEN: &str = "0.0.0.0:8080";
pub const DEFAULT_MODEL_FAST: &str = "claude-haiku-4-5-20251001";
pub const DEFAULT_MODEL_SMART: &str = "claude-sonnet-5";
pub const DEFAULT_MONTHLY_BUDGET_USD: f64 = 6.0;
pub const DEFAULT_DB: &str = "ubi-api.sqlite";

/// Upstream vendors. Only Anthropic for now; the enum exists so the config surface does not
/// change when a second one is added.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Vendor {
    Anthropic,
}

impl Vendor {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "anthropic" => Some(Vendor::Anthropic),
            _ => None,
        }
    }
}

/// USD per million tokens.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ModelPrice {
    pub input_per_mtok: f64,
    pub output_per_mtok: f64,
}

/// Token usage of one vendor call, as reported by the vendor.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
}

impl TokenUsage {
    /// Everything the vendor read, cached or not: what the ledger stores as `input_tokens`.
    pub fn total_input(&self) -> u64 {
        self.input_tokens + self.cache_read_tokens + self.cache_write_tokens
    }
}

/// Cache reads cost 10 % of the input price, cache writes 125 % (Anthropic's rule).
const CACHE_READ_FACTOR: f64 = 0.1;
const CACHE_WRITE_FACTOR: f64 = 1.25;

/// Model id → price. Exact id first, then the longest family substring (so
/// `claude-haiku-4-5-20251001` is priced like `claude-haiku-4-5`); unknown ids are billed at
/// the most expensive configured price so the estimate errs on the safe side.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PriceTable(pub BTreeMap<String, ModelPrice>);

impl Default for PriceTable {
    fn default() -> Self {
        let mut m = BTreeMap::new();
        m.insert(
            "claude-haiku-4-5".to_string(),
            ModelPrice {
                input_per_mtok: 1.0,
                output_per_mtok: 5.0,
            },
        );
        m.insert(
            "claude-sonnet-5".to_string(),
            ModelPrice {
                input_per_mtok: 2.0,
                output_per_mtok: 10.0,
            },
        );
        PriceTable(m)
    }
}

impl PriceTable {
    /// Parses `UBI_PRICES_JSON` (`{"model-id": {"input_per_mtok": 1.0, "output_per_mtok": 5.0}}`)
    /// on top of the defaults: entries override or extend.
    pub fn from_json(json: &str) -> Result<Self> {
        let extra: BTreeMap<String, ModelPrice> =
            serde_json::from_str(json).context("UBI_PRICES_JSON is not a price table")?;
        let mut table = PriceTable::default();
        table.0.extend(extra);
        Ok(table)
    }

    pub fn price_for(&self, model: &str) -> ModelPrice {
        if let Some(p) = self.0.get(model) {
            return *p;
        }
        let family = self
            .0
            .iter()
            .filter(|(family, _)| model.starts_with(family.as_str()))
            .max_by_key(|(family, _)| family.len());
        if let Some((_, p)) = family {
            return *p;
        }
        self.0
            .values()
            .copied()
            .max_by(|a, b| {
                (a.input_per_mtok + a.output_per_mtok)
                    .total_cmp(&(b.input_per_mtok + b.output_per_mtok))
            })
            .unwrap_or(ModelPrice {
                input_per_mtok: 5.0,
                output_per_mtok: 25.0,
            })
    }

    pub fn cost_usd(&self, model: &str, usage: &TokenUsage) -> f64 {
        let p = self.price_for(model);
        let per = |tokens: u64, price: f64| tokens as f64 * price / 1_000_000.0;
        per(usage.input_tokens, p.input_per_mtok)
            + per(
                usage.cache_read_tokens,
                p.input_per_mtok * CACHE_READ_FACTOR,
            )
            + per(
                usage.cache_write_tokens,
                p.input_per_mtok * CACHE_WRITE_FACTOR,
            )
            + per(usage.output_tokens, p.output_per_mtok)
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub listen: SocketAddr,
    /// Hex Ed25519 public key that license keys must verify against.
    pub license_pubkey_hex: String,
    /// Hex Ed25519 private key used by the payment webhook to issue keys. Optional: without it
    /// the webhook answers 503 for `subscription.created` / `renewed`.
    pub license_privkey_hex: Option<String>,
    /// Bearer token for `/admin/*`. Optional: without it every admin route answers 503.
    pub admin_token: Option<String>,
    /// HMAC secret for `/admin/webhooks/generic`. Optional: without it the webhook answers 503.
    pub webhook_secret: Option<String>,
    pub vendor: Vendor,
    pub vendor_api_key: String,
    /// Where `/v1/messages` is forwarded (tests point it at a mock server).
    pub vendor_base_url: String,
    pub monthly_budget_usd: f64,
    pub db_path: String,
    pub model_fast: String,
    pub model_smart: String,
    pub prices: PriceTable,
}

impl Config {
    /// A config with defaults for everything but the pubkey and vendor key (tests, examples).
    pub fn minimal(
        license_pubkey_hex: impl Into<String>,
        vendor_api_key: impl Into<String>,
    ) -> Self {
        Config {
            listen: DEFAULT_LISTEN.parse().expect("default listen address"),
            license_pubkey_hex: license_pubkey_hex.into(),
            license_privkey_hex: None,
            admin_token: None,
            webhook_secret: None,
            vendor: Vendor::Anthropic,
            vendor_api_key: vendor_api_key.into(),
            vendor_base_url: ANTHROPIC_BASE_URL.to_string(),
            monthly_budget_usd: DEFAULT_MONTHLY_BUDGET_USD,
            db_path: ":memory:".to_string(),
            model_fast: DEFAULT_MODEL_FAST.to_string(),
            model_smart: DEFAULT_MODEL_SMART.to_string(),
            prices: PriceTable::default(),
        }
    }

    pub fn from_env() -> Result<Self> {
        let env = |k: &str| std::env::var(k).ok().filter(|v| !v.trim().is_empty());
        let vendor_name = env("UBI_VENDOR").unwrap_or_else(|| "anthropic".into());
        let vendor = Vendor::parse(&vendor_name)
            .with_context(|| format!("UBI_VENDOR={vendor_name}: only `anthropic` is supported"))?;
        let vendor_api_key = match vendor {
            Vendor::Anthropic => {
                env("ANTHROPIC_API_KEY").context("ANTHROPIC_API_KEY is required")?
            }
        };
        let license_pubkey_hex = env("UBI_LICENSE_PUBKEY_HEX")
            .unwrap_or_else(|| ubiqx_core::license::UBIQX_LICENSE_PUBKEY_HEX.to_string());
        ubiqx_core::license::verifying_key_from_hex(&license_pubkey_hex)
            .map_err(|e| anyhow::anyhow!("UBI_LICENSE_PUBKEY_HEX: {e}"))?;
        let monthly_budget_usd = match env("UBI_MONTHLY_BUDGET_USD") {
            Some(v) => v
                .parse::<f64>()
                .with_context(|| format!("UBI_MONTHLY_BUDGET_USD={v} is not a number"))?,
            None => DEFAULT_MONTHLY_BUDGET_USD,
        };
        if !(monthly_budget_usd.is_finite() && monthly_budget_usd >= 0.0) {
            bail!("UBI_MONTHLY_BUDGET_USD must be a non-negative number");
        }
        let prices = match env("UBI_PRICES_JSON") {
            Some(json) => PriceTable::from_json(&json)?,
            None => PriceTable::default(),
        };
        let listen_raw = env("UBI_API_LISTEN").unwrap_or_else(|| DEFAULT_LISTEN.into());
        let listen: SocketAddr = listen_raw
            .parse()
            .with_context(|| format!("UBI_API_LISTEN={listen_raw} is not host:port"))?;
        Ok(Config {
            listen,
            license_pubkey_hex,
            license_privkey_hex: env("UBI_LICENSE_PRIVKEY_HEX"),
            admin_token: env("UBI_ADMIN_TOKEN"),
            webhook_secret: env("UBI_WEBHOOK_SECRET"),
            vendor,
            vendor_api_key,
            vendor_base_url: env("UBI_VENDOR_BASE_URL")
                .unwrap_or_else(|| ANTHROPIC_BASE_URL.into())
                .trim_end_matches('/')
                .to_string(),
            monthly_budget_usd,
            db_path: env("UBI_DB").unwrap_or_else(|| DEFAULT_DB.into()),
            model_fast: env("UBI_MODEL_FAST").unwrap_or_else(|| DEFAULT_MODEL_FAST.into()),
            model_smart: env("UBI_MODEL_SMART").unwrap_or_else(|| DEFAULT_MODEL_SMART.into()),
            prices,
        })
    }

    /// Maps a public model alias to the configured vendor model; `None` for anything else
    /// (the proxy never forwards arbitrary model ids).
    pub fn resolve_model_alias(&self, alias: &str) -> Option<&str> {
        match alias {
            ubiqx_core::license::UBI_MODEL_FAST => Some(self.model_fast.as_str()),
            ubiqx_core::license::UBI_MODEL_SMART => Some(self.model_smart.as_str()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prices_resolve_by_exact_id_then_family_then_max() {
        let t = PriceTable::default();
        assert_eq!(t.price_for("claude-haiku-4-5").input_per_mtok, 1.0);
        assert_eq!(
            t.price_for("claude-haiku-4-5-20251001").output_per_mtok,
            5.0
        );
        assert_eq!(t.price_for("claude-sonnet-5").input_per_mtok, 2.0);
        // Unknown → most expensive configured.
        assert_eq!(t.price_for("claude-mystery").input_per_mtok, 2.0);
        let custom = PriceTable::from_json(
            r#"{"claude-sonnet-5": {"input_per_mtok": 3.0, "output_per_mtok": 15.0}, "x": {"input_per_mtok": 0.1, "output_per_mtok": 0.2}}"#,
        )
        .unwrap();
        assert_eq!(custom.price_for("claude-sonnet-5").input_per_mtok, 3.0);
        assert_eq!(custom.price_for("x-1").output_per_mtok, 0.2);
        assert_eq!(custom.price_for("claude-haiku-4-5").input_per_mtok, 1.0);
        assert!(PriceTable::from_json("nope").is_err());
    }

    #[test]
    fn cost_accounts_for_cache_tokens() {
        let t = PriceTable::default();
        let usage = TokenUsage {
            input_tokens: 1_000_000,
            output_tokens: 100_000,
            cache_read_tokens: 1_000_000,
            cache_write_tokens: 1_000_000,
        };
        let cost = t.cost_usd("claude-haiku-4-5-20251001", &usage);
        // 1.0 + 0.5 + 0.1 + 1.25
        assert!((cost - 2.85).abs() < 1e-9, "{cost}");
        assert_eq!(usage.total_input(), 3_000_000);
    }

    #[test]
    fn aliases_map_to_configured_models() {
        let mut c = Config::minimal("00", "sk");
        c.model_fast = "fast-model".into();
        c.model_smart = "smart-model".into();
        assert_eq!(c.resolve_model_alias("ubi-fast"), Some("fast-model"));
        assert_eq!(c.resolve_model_alias("ubi-smart"), Some("smart-model"));
        assert_eq!(c.resolve_model_alias("claude-sonnet-5"), None);
        assert_eq!(c.resolve_model_alias(""), None);
    }
}
