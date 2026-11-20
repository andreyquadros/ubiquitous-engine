//! Price table for the models ubiqX uses and the cost estimate recorded with every call.
//!
//! Prices are USD per million tokens. Cache reads cost 10 % of the input price and cache
//! writes 125 % (only Anthropic reports cache writes; the other vendors always send zero).
//! Unknown models are billed at the vendor's flagship price so the estimate errs on the
//! conservative side for cheaper models.
//!
//! The vendor of a model id is inferred with [`AiProvider::for_model`] (`claude-*` →
//! Anthropic, `gpt-*`/`o*` → OpenAI, `grok-*` → xAI, `ubi-*` → the Ubi proxy, whose aliases
//! are priced like the Anthropic models behind them and only used as a fallback when the
//! proxy's `x-ubiqx-cost-usd` header is missing); ids that belong to nobody are priced as
//! Anthropic models, which keeps the historical behaviour of [`price_for`]. Within a vendor the
//! price is resolved by family substring, most specific family first (`gpt-5-mini` before
//! `gpt-5`, `grok-4-1-fast` before `grok-4`), so dated snapshots such as
//! `claude-haiku-4-5-20251001` or `gpt-4o-2024-08-06` are priced like their family.

use ubiqx_core::AiProvider;

/// Input/output price of a model family, in USD per million tokens.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModelPrice {
    pub input_per_mtok: f64,
    pub output_per_mtok: f64,
}

const fn price(input_per_mtok: f64, output_per_mtok: f64) -> ModelPrice {
    ModelPrice {
        input_per_mtok,
        output_per_mtok,
    }
}

// --- Anthropic -------------------------------------------------------------------------------

/// `claude-haiku-4-5`.
pub const HAIKU_4_5: ModelPrice = price(1.0, 5.0);
/// `claude-sonnet-5` (also the fallback for unknown Anthropic models and unknown vendors).
pub const SONNET_5: ModelPrice = price(2.0, 10.0);
/// `claude-opus-5`.
pub const OPUS_5: ModelPrice = price(5.0, 25.0);

// --- OpenAI ----------------------------------------------------------------------------------

/// Fallback for OpenAI models not in the table.
pub const OPENAI_UNKNOWN: ModelPrice = price(2.50, 15.0);

/// OpenAI families, most specific first. Entries starting with `o` followed by a digit are
/// matched by prefix (so `o3` cannot match inside `gpt-4o-...`), the rest by substring.
const OPENAI_PRICES: &[(&str, ModelPrice)] = &[
    ("gpt-5.4-mini", price(0.75, 4.50)),
    ("gpt-5.4-nano", price(0.20, 1.25)),
    ("gpt-5.4", price(2.50, 15.0)),
    ("gpt-5.1", price(1.25, 10.0)),
    ("gpt-5-mini", price(0.25, 2.0)),
    ("gpt-5-nano", price(0.05, 0.40)),
    ("gpt-5", price(1.25, 10.0)),
    ("gpt-4.1-mini", price(0.40, 1.60)),
    ("gpt-4.1-nano", price(0.10, 0.40)),
    ("gpt-4.1", price(2.0, 8.0)),
    ("gpt-4o-mini", price(0.15, 0.60)),
    ("gpt-4o", price(2.50, 10.0)),
    ("o4-mini", price(1.10, 4.40)),
    ("o3", price(2.0, 8.0)),
];

// --- xAI -------------------------------------------------------------------------------------

/// Fallback for xAI models not in the table.
pub const XAI_UNKNOWN: ModelPrice = price(3.0, 15.0);

/// xAI families, most specific first (substring match).
const XAI_PRICES: &[(&str, ModelPrice)] = &[
    ("grok-4-1-fast", price(0.20, 0.50)),
    ("grok-4-fast", price(0.20, 0.50)),
    ("grok-4.20", price(1.25, 2.50)),
    ("grok-4.3", price(1.25, 2.50)),
    ("grok-4.6", price(2.0, 6.0)),
    ("grok-4", price(3.0, 15.0)),
    ("grok-3-mini", price(0.30, 0.50)),
    ("grok-3", price(3.0, 15.0)),
    ("grok-2-vision", price(2.0, 10.0)),
];

/// Cache-read tokens cost this fraction of the input price.
pub const CACHE_READ_FACTOR: f64 = 0.10;
/// Cache-write tokens cost this multiple of the input price.
pub const CACHE_WRITE_FACTOR: f64 = 1.25;

fn normalise(model: &str) -> String {
    model.trim().to_ascii_lowercase()
}

fn anthropic_price(m: &str) -> ModelPrice {
    if m.contains("haiku") {
        HAIKU_4_5
    } else if m.contains("opus") {
        OPUS_5
    } else {
        SONNET_5
    }
}

fn openai_price(m: &str) -> ModelPrice {
    // Fine-tuned ids look like `ft:gpt-4.1-mini:org::id`; the family is still a substring.
    OPENAI_PRICES
        .iter()
        .find(|(family, _)| {
            let o_series =
                family.starts_with('o') && family[1..].starts_with(|c: char| c.is_ascii_digit());
            if o_series {
                m.starts_with(family)
            } else {
                m.contains(family)
            }
        })
        .map(|(_, p)| *p)
        .unwrap_or(OPENAI_UNKNOWN)
}

/// The proxy's aliases: `ubi-fast` runs on the Haiku tier, `ubi-smart` on the Sonnet tier.
fn ubi_price(m: &str) -> ModelPrice {
    if m == ubiqx_core::license::UBI_MODEL_FAST || m.contains("fast") {
        HAIKU_4_5
    } else {
        SONNET_5
    }
}

fn xai_price(m: &str) -> ModelPrice {
    XAI_PRICES
        .iter()
        .find(|(family, _)| m.contains(family))
        .map(|(_, p)| *p)
        .unwrap_or(XAI_UNKNOWN)
}

/// Price of `model` at `provider`, resolved by family (see the module docs). Unknown families
/// get the vendor's flagship price.
pub fn price_for_provider(provider: AiProvider, model: &str) -> ModelPrice {
    let m = normalise(model);
    match provider {
        AiProvider::Anthropic => anthropic_price(&m),
        AiProvider::OpenAi => openai_price(&m),
        AiProvider::Xai => xai_price(&m),
        AiProvider::Ubi => ubi_price(&m),
    }
}

/// Price of a model id whose vendor is inferred from the id ([`AiProvider::for_model`]);
/// ids that belong to no vendor are priced as Anthropic models (Sonnet unless the family says
/// otherwise).
pub fn price_for(model: &str) -> ModelPrice {
    let provider = AiProvider::for_model(model).unwrap_or(AiProvider::Anthropic);
    price_for_provider(provider, model)
}

fn cost(
    price: ModelPrice,
    input_tokens: u32,
    output_tokens: u32,
    cache_read_tokens: u32,
    cache_write_tokens: u32,
) -> f64 {
    let per = |tokens: u32, usd_per_mtok: f64| f64::from(tokens) / 1_000_000.0 * usd_per_mtok;
    per(input_tokens, price.input_per_mtok)
        + per(output_tokens, price.output_per_mtok)
        + per(cache_read_tokens, price.input_per_mtok * CACHE_READ_FACTOR)
        + per(
            cache_write_tokens,
            price.input_per_mtok * CACHE_WRITE_FACTOR,
        )
}

/// Estimated cost in USD of one call, with the vendor inferred from the model id.
pub fn estimate_cost_usd(
    model: &str,
    input_tokens: u32,
    output_tokens: u32,
    cache_read_tokens: u32,
    cache_write_tokens: u32,
) -> f64 {
    cost(
        price_for(model),
        input_tokens,
        output_tokens,
        cache_read_tokens,
        cache_write_tokens,
    )
}

/// Estimated cost in USD of one call answered by `provider` (use when the vendor is known,
/// e.g. for custom model ids that [`AiProvider::for_model`] cannot place).
pub fn estimate_cost_usd_for_provider(
    provider: AiProvider,
    model: &str,
    input_tokens: u32,
    output_tokens: u32,
    cache_read_tokens: u32,
    cache_write_tokens: u32,
) -> f64 {
    cost(
        price_for_provider(provider, model),
        input_tokens,
        output_tokens,
        cache_read_tokens,
        cache_write_tokens,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-12
    }

    #[test]
    fn families_resolve() {
        assert_eq!(price_for("claude-haiku-4-5"), HAIKU_4_5);
        assert_eq!(price_for("claude-haiku-4-5-20251001"), HAIKU_4_5);
        assert_eq!(price_for("claude-sonnet-5"), SONNET_5);
        assert_eq!(price_for("claude-opus-5"), OPUS_5);
        assert_eq!(price_for("something-else"), SONNET_5);
        assert_eq!(price_for(""), SONNET_5);
    }

    #[test]
    fn openai_families_resolve_most_specific_first() {
        let cases = [
            ("gpt-5", price(1.25, 10.0)),
            ("gpt-5-2025-08-07", price(1.25, 10.0)),
            ("gpt-5-mini", price(0.25, 2.0)),
            ("gpt-5-nano", price(0.05, 0.40)),
            ("gpt-5.1", price(1.25, 10.0)),
            ("gpt-5.4", price(2.50, 15.0)),
            ("gpt-5.4-mini", price(0.75, 4.50)),
            ("gpt-5.4-nano", price(0.20, 1.25)),
            ("gpt-4.1", price(2.0, 8.0)),
            ("gpt-4.1-mini", price(0.40, 1.60)),
            ("gpt-4.1-nano", price(0.10, 0.40)),
            ("ft:gpt-4.1-mini:acme::abc", price(0.40, 1.60)),
            ("gpt-4o", price(2.50, 10.0)),
            ("gpt-4o-2024-08-06", price(2.50, 10.0)),
            ("gpt-4o-mini", price(0.15, 0.60)),
            ("o4-mini", price(1.10, 4.40)),
            ("o3", price(2.0, 8.0)),
            ("o3-2025-04-16", price(2.0, 8.0)),
            ("o1", OPENAI_UNKNOWN),
            ("GPT-5-MINI", price(0.25, 2.0)),
        ];
        for (model, expected) in cases {
            assert_eq!(price_for(model), expected, "{model}");
            assert_eq!(
                price_for_provider(AiProvider::OpenAi, model),
                expected,
                "{model}"
            );
        }
        // A custom id that the inference cannot place, priced explicitly as OpenAI.
        assert_eq!(
            price_for_provider(AiProvider::OpenAi, "my-deployment"),
            OPENAI_UNKNOWN
        );
    }

    #[test]
    fn xai_families_resolve_most_specific_first() {
        let cases = [
            ("grok-4", price(3.0, 15.0)),
            ("grok-4-0709", price(3.0, 15.0)),
            ("grok-4-fast-reasoning", price(0.20, 0.50)),
            ("grok-4-fast-non-reasoning", price(0.20, 0.50)),
            ("grok-4-1-fast-reasoning", price(0.20, 0.50)),
            ("grok-4-1-fast-non-reasoning", price(0.20, 0.50)),
            ("grok-4.6", price(2.0, 6.0)),
            ("grok-4.3", price(1.25, 2.50)),
            ("grok-4.20-beta", price(1.25, 2.50)),
            ("grok-3", price(3.0, 15.0)),
            ("grok-3-mini", price(0.30, 0.50)),
            ("grok-3-mini-fast", price(0.30, 0.50)),
            ("grok-2-vision-1212", price(2.0, 10.0)),
            ("grok-9", XAI_UNKNOWN),
        ];
        for (model, expected) in cases {
            assert_eq!(price_for(model), expected, "{model}");
            assert_eq!(
                price_for_provider(AiProvider::Xai, model),
                expected,
                "{model}"
            );
        }
        assert_eq!(price_for_provider(AiProvider::Xai, "custom"), XAI_UNKNOWN);
    }

    #[test]
    fn ubi_aliases_are_priced_like_their_tiers() {
        assert_eq!(price_for("ubi-fast"), HAIKU_4_5);
        assert_eq!(price_for("ubi-smart"), SONNET_5);
        assert_eq!(price_for_provider(AiProvider::Ubi, "anything"), SONNET_5);
        assert_eq!(price_for_provider(AiProvider::Ubi, "UBI-FAST"), HAIKU_4_5);
    }

    #[test]
    fn haiku_cost_math() {
        // 1 M in, 1 M out, 1 M cache read, 1 M cache write.
        let c = estimate_cost_usd(
            "claude-haiku-4-5",
            1_000_000,
            1_000_000,
            1_000_000,
            1_000_000,
        );
        assert!(close(c, 1.0 + 5.0 + 0.10 + 1.25), "{c}");
        // Typical classification batch: 1200 in, 80 out.
        let c = estimate_cost_usd("claude-haiku-4-5", 1200, 80, 0, 0);
        assert!(close(c, 0.0012 + 0.0004), "{c}");
    }

    #[test]
    fn sonnet_and_opus_cost_math() {
        let c = estimate_cost_usd("claude-sonnet-5", 6000, 1500, 4000, 2000);
        let expected =
            6000.0 / 1e6 * 2.0 + 1500.0 / 1e6 * 10.0 + 4000.0 / 1e6 * 0.2 + 2000.0 / 1e6 * 2.5;
        assert!(close(c, expected), "{c} vs {expected}");
        let c = estimate_cost_usd("claude-opus-5", 1_000_000, 0, 0, 0);
        assert!(close(c, 5.0));
        assert!(close(estimate_cost_usd("unknown", 0, 0, 0, 0), 0.0));
    }

    #[test]
    fn openai_and_xai_cost_math() {
        // gpt-5-mini: 1200 uncached in, 500 cached in, 80 out.
        let c = estimate_cost_usd("gpt-5-mini", 1200, 80, 500, 0);
        let expected = 1200.0 / 1e6 * 0.25 + 80.0 / 1e6 * 2.0 + 500.0 / 1e6 * 0.025;
        assert!(close(c, expected), "{c} vs {expected}");
        let c = estimate_cost_usd("grok-4-1-fast-non-reasoning", 1_000_000, 1_000_000, 0, 0);
        assert!(close(c, 0.70), "{c}");
        let c = estimate_cost_usd_for_provider(AiProvider::Xai, "custom", 1_000_000, 0, 0, 0);
        assert!(close(c, 3.0), "{c}");
    }
}
