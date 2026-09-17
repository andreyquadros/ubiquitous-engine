//! Price table for the models ubiqX uses and the cost estimate recorded with every call.
//!
//! Prices are USD per million tokens. Cache reads cost 10 % of the input price and cache
//! writes 125 %. Unknown models are billed at Sonnet prices so the estimate errs on the
//! conservative side for cheaper models.

/// Input/output price of a model family, in USD per million tokens.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModelPrice {
    pub input_per_mtok: f64,
    pub output_per_mtok: f64,
}

/// `claude-haiku-4-5`.
pub const HAIKU_4_5: ModelPrice = ModelPrice {
    input_per_mtok: 1.0,
    output_per_mtok: 5.0,
};
/// `claude-sonnet-5` (also the fallback for unknown models).
pub const SONNET_5: ModelPrice = ModelPrice {
    input_per_mtok: 2.0,
    output_per_mtok: 10.0,
};
/// `claude-opus-5`.
pub const OPUS_5: ModelPrice = ModelPrice {
    input_per_mtok: 5.0,
    output_per_mtok: 25.0,
};
/// Cache-read tokens cost this fraction of the input price.
pub const CACHE_READ_FACTOR: f64 = 0.10;
/// Cache-write tokens cost this multiple of the input price.
pub const CACHE_WRITE_FACTOR: f64 = 1.25;

/// Resolves the price of a model id by family (`haiku`, `opus`, otherwise Sonnet), so dated
/// snapshots such as `claude-haiku-4-5-20251001` are priced like their family.
pub fn price_for(model: &str) -> ModelPrice {
    let m = model.trim().to_ascii_lowercase();
    if m.contains("haiku") {
        HAIKU_4_5
    } else if m.contains("opus") {
        OPUS_5
    } else {
        SONNET_5
    }
}

/// Estimated cost in USD of one call.
pub fn estimate_cost_usd(
    model: &str,
    input_tokens: u32,
    output_tokens: u32,
    cache_read_tokens: u32,
    cache_write_tokens: u32,
) -> f64 {
    let price = price_for(model);
    let per = |tokens: u32, usd_per_mtok: f64| f64::from(tokens) / 1_000_000.0 * usd_per_mtok;
    per(input_tokens, price.input_per_mtok)
        + per(output_tokens, price.output_per_mtok)
        + per(cache_read_tokens, price.input_per_mtok * CACHE_READ_FACTOR)
        + per(
            cache_write_tokens,
            price.input_per_mtok * CACHE_WRITE_FACTOR,
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
}
