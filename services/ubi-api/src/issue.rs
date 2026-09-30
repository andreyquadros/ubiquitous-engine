//! Issuing keys: key pairs, subscriber ids and signed claims. Verification itself lives in
//! `ubiqx_core::license` so the app and the proxy can never disagree on what a key means.

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Months, Utc};
use ed25519_dalek::SigningKey;
use sha2::{Digest, Sha256};
use ubiqx_core::license::{self, LicenseClaims, Plan, CLAIMS_VERSION};

/// A freshly generated Ed25519 key pair, hex-encoded.
#[derive(Debug, Clone)]
pub struct KeyPairHex {
    pub private_hex: String,
    pub public_hex: String,
}

pub fn hex_encode(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut acc, b| {
            let _ = write!(acc, "{b:02x}");
            acc
        })
}

fn hex_decode(s: &str) -> Result<Vec<u8>> {
    let s = s.trim();
    if s.len() % 2 != 0 {
        bail!("hex string has odd length");
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(Into::into))
        .collect()
}

pub fn keygen() -> KeyPairHex {
    let signing = SigningKey::generate(&mut rand::rngs::OsRng);
    KeyPairHex {
        private_hex: hex_encode(&signing.to_bytes()),
        public_hex: hex_encode(signing.verifying_key().as_bytes()),
    }
}

pub fn signing_key_from_hex(privkey_hex: &str) -> Result<SigningKey> {
    let bytes = hex_decode(privkey_hex).context("private key is not hex")?;
    let arr: [u8; 32] = bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("private key must be 32 bytes (64 hex chars)"))?;
    Ok(SigningKey::from_bytes(&arr))
}

/// Deterministic, opaque subscriber id: the payment platform's id (or, failing that, the
/// e-mail hash) hashed so that neither appears in the key, yet a renewal of the same
/// subscription lands on the same `sub` (and a revocation of it sticks).
pub fn derive_sub(external_id: Option<&str>, email_hash: &str) -> String {
    let seed = match external_id.map(str::trim).filter(|s| !s.is_empty()) {
        Some(id) => format!("ubiqx-sub:ext:{id}"),
        None => format!("ubiqx-sub:email:{email_hash}"),
    };
    let digest = Sha256::digest(seed.as_bytes());
    format!("sub_{}", &hex_encode(&digest)[..24])
}

/// Builds the claims of a key that starts now and lasts `months` calendar months.
pub fn claims_for(
    plan: Plan,
    sub: impl Into<String>,
    email_hash: impl Into<String>,
    months: u32,
    now: DateTime<Utc>,
) -> Result<LicenseClaims> {
    if months == 0 {
        bail!("months must be at least 1");
    }
    let expires = now
        .checked_add_months(Months::new(months))
        .context("expiry date out of range")?;
    Ok(LicenseClaims {
        v: CLAIMS_VERSION,
        plan,
        sub: sub.into(),
        email_hash: email_hash.into(),
        issued_at: now.timestamp(),
        expires_at: expires.timestamp(),
        seats: 1,
    })
}

/// Signs `claims` into a key string.
pub fn issue(claims: &LicenseClaims, signing_key: &SigningKey) -> String {
    license::sign_claims(claims, signing_key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keygen_produces_a_matching_pair() {
        let pair = keygen();
        assert_eq!(pair.private_hex.len(), 64);
        assert_eq!(pair.public_hex.len(), 64);
        let signing = signing_key_from_hex(&pair.private_hex).unwrap();
        assert_eq!(
            hex_encode(signing.verifying_key().as_bytes()),
            pair.public_hex
        );
        assert!(signing_key_from_hex("abc").is_err());
        assert!(signing_key_from_hex("zz".repeat(32).as_str()).is_err());
    }

    #[test]
    fn sub_is_stable_and_opaque() {
        let h = license::email_hash("x@y.z");
        let a = derive_sub(Some("mp_123"), &h);
        assert_eq!(a, derive_sub(Some(" mp_123 "), "other"));
        assert_ne!(a, derive_sub(Some("mp_124"), &h));
        assert_eq!(derive_sub(None, &h), derive_sub(Some(""), &h));
        assert!(a.starts_with("sub_"));
        assert!(!a.contains("mp_123"));
    }

    #[test]
    fn claims_last_calendar_months() {
        let now = DateTime::parse_from_rfc3339("2026-01-31T10:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let c = claims_for(Plan::AnnualOwnKey, "s", "h", 12, now).unwrap();
        assert_eq!(c.expires_at_utc().to_rfc3339(), "2027-01-31T10:00:00+00:00");
        let c = claims_for(Plan::MonthlyManaged, "s", "h", 1, now).unwrap();
        assert_eq!(c.expires_at_utc().to_rfc3339(), "2026-02-28T10:00:00+00:00");
        assert!(claims_for(Plan::MonthlyManaged, "s", "h", 0, now).is_err());
    }
}
