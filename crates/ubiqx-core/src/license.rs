//! Licensing: plans, signed license keys and the status the UI shows.
//!
//! Everything here is pure (no I/O) and shared by the desktop app (verify only) and the
//! `ubi-api` service / `ubi-license` CLI (verify **and** issue). The key format is:
//!
//! ```text
//! UBIQX-<base32(canonical JSON claims)>-<base32(Ed25519 signature)>
//! ```
//!
//! * base32 is RFC 4648 without padding, upper-case, so a key survives e-mail clients, chat
//!   apps and hand typing;
//! * the claims are canonical JSON (keys sorted, no whitespace) so the same claims always
//!   produce the same bytes and the signature is over exactly those bytes;
//! * the signature is a 64-byte Ed25519 signature made with the ubiqX private key. The app
//!   embeds only the public key ([`UBIQX_LICENSE_PUBKEY_HEX`]); the private key lives in the
//!   operator's secret store and never enters the repository.
//!
//! Verification is fully offline: no call home is needed to know whether a key is genuine
//! and until when it is valid. The `monthly_managed` plan additionally asks the Ubi proxy for
//! the month's usage, which is merged into [`LicenseStatus::managed_usage`] by the caller.
//!
//! [`LICENSE_ENFORCEMENT`] is a compile-time constant. With [`LicenseEnforcement::Soft`] a
//! missing or expired license only shows a reminder; with [`LicenseEnforcement::Hard`] the AI
//! features are blocked until a valid key is entered. Tracking, the timeline and manual
//! categorisation always work regardless of the license.

use chrono::{DateTime, TimeZone, Utc};
use data_encoding::BASE32_NOPAD;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Hex-encoded Ed25519 public key that every ubiqX build trusts. Keys signed by any other
/// private key are rejected as [`LicenseError::BadSignature`].
///
/// This is the **development** key pair's public half; the private half is kept by the
/// product owner (see `docs/LICENSING.md`). Rotate it here (and in the proxy's
/// `UBI_LICENSE_PUBKEY_HEX`) to invalidate every key ever issued.
pub const UBIQX_LICENSE_PUBKEY_HEX: &str =
    "de87f36a111822ff93cdc9c39d06a543e179b2345d593826e04fac4d46a1c210";

/// Prefix of every license key.
pub const KEY_PREFIX: &str = "UBIQX-";

/// Claims format version this build understands.
pub const CLAIMS_VERSION: u8 = 1;

/// Number of trailing key characters shown in the UI ([`LicenseStatus::key_hint`]).
pub const KEY_HINT_LEN: usize = 4;

/// How a missing or invalid license affects the app. See the module docs.
pub const LICENSE_ENFORCEMENT: LicenseEnforcement = LicenseEnforcement::Soft;

/// Default base URL of the Ubi proxy (the `monthly_managed` plan's AI endpoint). The desktop
/// app overrides it with the `UBIQX_API_BASE` environment variable at build time.
pub const UBIQX_API_BASE_DEFAULT: &str = "https://api.ubiqx.ai";

/// Model alias the Ubi proxy maps to its fast model (classification and vision).
pub const UBI_MODEL_FAST: &str = "ubi-fast";
/// Model alias the Ubi proxy maps to its smart model (reports and advice).
pub const UBI_MODEL_SMART: &str = "ubi-smart";

/// Request header carrying the plan when the app talks to the Ubi proxy.
pub const HEADER_UBIQX_PLAN: &str = "x-ubiqx-plan";
/// Response header in which the Ubi proxy reports the cost of a call, in USD.
pub const HEADER_UBIQX_COST_USD: &str = "x-ubiqx-cost-usd";

// ---------------------------------------------------------------------------------------------
// Plans and claims
// ---------------------------------------------------------------------------------------------

/// The two products sold on the site. Prices live on the site and in the UI copy, never here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Plan {
    /// "ubiqX Anual, com a sua IA": the user brings their own vendor API key.
    AnnualOwnKey,
    /// "ubiqX Mensal, com a IA do Ubi": the AI goes through the Ubi proxy, no vendor key.
    MonthlyManaged,
}

impl Plan {
    pub const ALL: [Plan; 2] = [Plan::AnnualOwnKey, Plan::MonthlyManaged];

    /// Stable id used in claims, the IPC contract and the proxy.
    pub fn id(self) -> &'static str {
        match self {
            Plan::AnnualOwnKey => "annual_own_key",
            Plan::MonthlyManaged => "monthly_managed",
        }
    }

    pub fn parse(id: &str) -> Option<Self> {
        match id.trim().to_ascii_lowercase().as_str() {
            "annual_own_key" | "annual" => Some(Plan::AnnualOwnKey),
            "monthly_managed" | "managed" | "monthly" => Some(Plan::MonthlyManaged),
            _ => None,
        }
    }

    /// Whether the plan's AI calls go through the Ubi proxy.
    pub fn is_managed(self) -> bool {
        matches!(self, Plan::MonthlyManaged)
    }
}

/// What a license key asserts. Serialised canonically (sorted keys, no whitespace) and
/// signed; see the module docs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LicenseClaims {
    /// Claims format version ([`CLAIMS_VERSION`]).
    pub v: u8,
    pub plan: Plan,
    /// Opaque subscriber id (stable across renewals of the same subscription).
    pub sub: String,
    /// SHA-256 hex of the lower-cased e-mail; the e-mail itself never travels.
    pub email_hash: String,
    /// Unix seconds.
    pub issued_at: i64,
    /// Unix seconds; the key is valid while `now < expires_at`.
    pub expires_at: i64,
    pub seats: u32,
}

impl LicenseClaims {
    /// Canonical JSON bytes: keys in alphabetical order, no whitespace. This is what gets
    /// signed, so it must not depend on `serde_json`'s map ordering (which other crates in a
    /// build can switch to insertion order).
    pub fn canonical_json(&self) -> Vec<u8> {
        let s = |v: &str| serde_json::to_string(v).expect("string serialises");
        format!(
            r#"{{"email_hash":{},"expires_at":{},"issued_at":{},"plan":{},"seats":{},"sub":{},"v":{}}}"#,
            s(&self.email_hash),
            self.expires_at,
            self.issued_at,
            s(self.plan.id()),
            self.seats,
            s(&self.sub),
            self.v
        )
        .into_bytes()
    }

    pub fn expires_at_utc(&self) -> DateTime<Utc> {
        Utc.timestamp_opt(self.expires_at, 0)
            .single()
            .unwrap_or(DateTime::<Utc>::MAX_UTC)
    }

    pub fn issued_at_utc(&self) -> DateTime<Utc> {
        Utc.timestamp_opt(self.issued_at, 0)
            .single()
            .unwrap_or(DateTime::<Utc>::MIN_UTC)
    }

    /// `true` once `now` reaches `expires_at`.
    pub fn is_expired_at(&self, now: DateTime<Utc>) -> bool {
        now.timestamp() >= self.expires_at
    }

    /// Whole days until expiry (0 on the last day, negative once expired).
    pub fn days_left_at(&self, now: DateTime<Utc>) -> i64 {
        let secs = self.expires_at - now.timestamp();
        secs.div_euclid(86_400)
    }
}

/// SHA-256 hex of the trimmed, lower-cased e-mail: the only form of the e-mail that appears
/// in a key.
pub fn email_hash(email: &str) -> String {
    let normalised = email.trim().to_lowercase();
    hex_lower(&Sha256::digest(normalised.as_bytes()))
}

fn hex_lower(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut acc, b| {
            let _ = write!(acc, "{b:02x}");
            acc
        })
}

fn hex_decode(s: &str) -> Option<Vec<u8>> {
    let s = s.trim();
    if s.len() % 2 != 0 {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

// ---------------------------------------------------------------------------------------------
// Key encoding / verification
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum LicenseError {
    /// Not a key: wrong prefix, bad base32, JSON that is not canonical claims…
    #[error("malformed license key: {0}")]
    Malformed(String),
    /// A well-formed key whose signature was not made by the ubiqX private key (or whose
    /// claims were altered).
    #[error("license signature does not verify")]
    BadSignature,
    /// Claims issued for a newer format than this build understands.
    #[error("unsupported license claims version {0}")]
    UnsupportedVersion(u8),
}

/// Removes whitespace and upper-cases a pasted key so lower-cased or line-wrapped copies of
/// a key still verify.
pub fn normalize_key(raw: &str) -> String {
    raw.chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

/// Builds a key from canonical claims bytes and their signature.
pub fn encode_key(payload: &[u8], signature: &[u8; 64]) -> String {
    format!(
        "{KEY_PREFIX}{}-{}",
        BASE32_NOPAD.encode(payload),
        BASE32_NOPAD.encode(signature)
    )
}

/// A key split into its parts, **not yet verified**.
#[derive(Debug, Clone)]
pub struct ParsedKey {
    pub claims: LicenseClaims,
    /// The canonical claims bytes exactly as carried by the key (what the signature covers).
    pub payload: Vec<u8>,
    pub signature: [u8; 64],
}

/// Decodes a key without checking the signature. Use [`verify_key`] unless you only need
/// the claims for display.
pub fn parse_key(raw: &str) -> Result<ParsedKey, LicenseError> {
    let key = normalize_key(raw);
    let rest = key
        .strip_prefix(KEY_PREFIX)
        .ok_or_else(|| LicenseError::Malformed("missing UBIQX- prefix".into()))?;
    let (payload_b32, sig_b32) = rest
        .split_once('-')
        .ok_or_else(|| LicenseError::Malformed("missing signature part".into()))?;
    if payload_b32.is_empty() || sig_b32.is_empty() || sig_b32.contains('-') {
        return Err(LicenseError::Malformed("unexpected key layout".into()));
    }
    let payload = BASE32_NOPAD
        .decode(payload_b32.as_bytes())
        .map_err(|e| LicenseError::Malformed(format!("claims are not base32: {e}")))?;
    let sig_bytes = BASE32_NOPAD
        .decode(sig_b32.as_bytes())
        .map_err(|e| LicenseError::Malformed(format!("signature is not base32: {e}")))?;
    let signature: [u8; 64] = sig_bytes
        .try_into()
        .map_err(|_| LicenseError::Malformed("signature is not 64 bytes".into()))?;
    let claims: LicenseClaims = serde_json::from_slice(&payload)
        .map_err(|e| LicenseError::Malformed(format!("claims are not valid JSON: {e}")))?;
    if claims.v != CLAIMS_VERSION {
        return Err(LicenseError::UnsupportedVersion(claims.v));
    }
    if claims.canonical_json() != payload {
        return Err(LicenseError::Malformed("claims are not canonical".into()));
    }
    Ok(ParsedKey {
        claims,
        payload,
        signature,
    })
}

/// Parses a hex public key into a verifying key.
pub fn verifying_key_from_hex(pubkey_hex: &str) -> Result<VerifyingKey, LicenseError> {
    let bytes = hex_decode(pubkey_hex)
        .ok_or_else(|| LicenseError::Malformed("public key is not hex".into()))?;
    let arr: [u8; 32] = bytes
        .try_into()
        .map_err(|_| LicenseError::Malformed("public key is not 32 bytes".into()))?;
    VerifyingKey::from_bytes(&arr)
        .map_err(|_| LicenseError::Malformed("public key is not a valid Ed25519 point".into()))
}

/// Decodes a key and checks its signature against `pubkey_hex`. Expiry is **not** checked
/// here: the claims are returned so the caller can decide (see [`LicenseStatus::evaluate`]).
pub fn verify_key_with(raw: &str, pubkey_hex: &str) -> Result<LicenseClaims, LicenseError> {
    let verifying_key = verifying_key_from_hex(pubkey_hex)?;
    let parsed = parse_key(raw)?;
    let signature = Signature::from_bytes(&parsed.signature);
    verifying_key
        .verify(&parsed.payload, &signature)
        .map_err(|_| LicenseError::BadSignature)?;
    Ok(parsed.claims)
}

/// [`verify_key_with`] against the embedded ubiqX public key.
pub fn verify_key(raw: &str) -> Result<LicenseClaims, LicenseError> {
    verify_key_with(raw, UBIQX_LICENSE_PUBKEY_HEX)
}

/// Signs canonical claims and returns the key. Only the issuing tooling (`ubi-license`, the
/// proxy's payment webhook) needs this, hence the `sign` feature.
#[cfg(feature = "sign")]
pub fn sign_claims(claims: &LicenseClaims, signing_key: &ed25519_dalek::SigningKey) -> String {
    use ed25519_dalek::Signer;
    let payload = claims.canonical_json();
    let signature = signing_key.sign(&payload);
    encode_key(&payload, &signature.to_bytes())
}

/// Last [`KEY_HINT_LEN`] characters of a (normalised) key, for the UI.
pub fn key_hint(raw: &str) -> String {
    let key = normalize_key(raw);
    let start = key.len().saturating_sub(KEY_HINT_LEN);
    key[start..].to_string()
}

// ---------------------------------------------------------------------------------------------
// Status
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LicenseEnforcement {
    /// Reminders only; every feature keeps working without a license.
    Soft,
    /// AI features are blocked until a valid license is entered. Tracking, the timeline and
    /// manual categorisation still work.
    Hard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LicenseState {
    Unlicensed,
    Valid,
    Expired,
    Invalid,
}

/// The current month's spend of a `monthly_managed` subscriber, as reported by the proxy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ManagedUsage {
    /// `YYYY-MM` (UTC).
    pub month: String,
    pub spent_usd: f64,
    pub budget_usd: f64,
}

/// What the UI shows in Settings › Licença and in the onboarding step. Serialised
/// `snake_case`; this is the IPC contract.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LicenseStatus {
    pub state: LicenseState,
    pub plan: Option<Plan>,
    /// RFC 3339 (UTC).
    pub expires_at: Option<String>,
    pub days_left: Option<i64>,
    /// Last characters of the stored key, so the user can tell which key is loaded.
    pub key_hint: Option<String>,
    pub enforcement: LicenseEnforcement,
    /// Only for a valid `monthly_managed` license, and only when the proxy answered.
    pub managed_usage: Option<ManagedUsage>,
}

impl LicenseStatus {
    /// No key stored.
    pub fn unlicensed() -> Self {
        LicenseStatus {
            state: LicenseState::Unlicensed,
            plan: None,
            expires_at: None,
            days_left: None,
            key_hint: None,
            enforcement: LICENSE_ENFORCEMENT,
            managed_usage: None,
        }
    }

    /// Evaluates a stored key (or `None`) against the embedded public key at `now`.
    pub fn evaluate(key: Option<&str>, now: DateTime<Utc>) -> Self {
        Self::evaluate_with(key, UBIQX_LICENSE_PUBKEY_HEX, now)
    }

    /// [`LicenseStatus::evaluate`] with an explicit public key (tests, the proxy).
    pub fn evaluate_with(key: Option<&str>, pubkey_hex: &str, now: DateTime<Utc>) -> Self {
        let Some(key) = key.map(str::trim).filter(|k| !k.is_empty()) else {
            return Self::unlicensed();
        };
        let hint = Some(key_hint(key));
        match verify_key_with(key, pubkey_hex) {
            Ok(claims) => {
                let expired = claims.is_expired_at(now);
                LicenseStatus {
                    state: if expired {
                        LicenseState::Expired
                    } else {
                        LicenseState::Valid
                    },
                    plan: Some(claims.plan),
                    expires_at: Some(claims.expires_at_utc().to_rfc3339()),
                    days_left: Some(claims.days_left_at(now).max(0)),
                    key_hint: hint,
                    enforcement: LICENSE_ENFORCEMENT,
                    managed_usage: None,
                }
            }
            Err(_) => LicenseStatus {
                state: LicenseState::Invalid,
                plan: None,
                expires_at: None,
                days_left: None,
                key_hint: hint,
                enforcement: LICENSE_ENFORCEMENT,
                managed_usage: None,
            },
        }
    }

    pub fn is_valid(&self) -> bool {
        self.state == LicenseState::Valid
    }

    /// Whether the Ubi provider may be selected: a valid `monthly_managed` license.
    pub fn allows_managed_ai(&self) -> bool {
        self.is_valid() && self.plan == Some(Plan::MonthlyManaged)
    }

    /// Whether AI features are blocked by the license policy (only ever `true` under
    /// [`LicenseEnforcement::Hard`]).
    pub fn blocks_ai(&self) -> bool {
        self.enforcement == LicenseEnforcement::Hard && !self.is_valid()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn test_signing_key() -> SigningKey {
        SigningKey::from_bytes(&[7u8; 32])
    }

    fn test_pubkey_hex() -> String {
        hex_lower(test_signing_key().verifying_key().as_bytes())
    }

    fn claims(plan: Plan, issued_at: i64, expires_at: i64) -> LicenseClaims {
        LicenseClaims {
            v: CLAIMS_VERSION,
            plan,
            sub: "sub_test_0001".into(),
            email_hash: email_hash("Someone@Example.com "),
            issued_at,
            expires_at,
            seats: 1,
        }
    }

    fn sign(claims: &LicenseClaims, key: &SigningKey) -> String {
        let payload = claims.canonical_json();
        encode_key(&payload, &key.sign(&payload).to_bytes())
    }

    #[test]
    fn canonical_json_is_sorted_and_compact() {
        let c = claims(Plan::MonthlyManaged, 1_700_000_000, 1_702_592_000);
        let json = String::from_utf8(c.canonical_json()).unwrap();
        assert_eq!(
            json,
            format!(
                r#"{{"email_hash":"{}","expires_at":1702592000,"issued_at":1700000000,"plan":"monthly_managed","seats":1,"sub":"sub_test_0001","v":1}}"#,
                c.email_hash
            )
        );
        let back: LicenseClaims = serde_json::from_str(&json).unwrap();
        assert_eq!(back, c);
    }

    #[test]
    fn email_hash_normalises_case_and_whitespace() {
        assert_eq!(email_hash("A@B.com"), email_hash("  a@b.COM\n"));
        assert_eq!(email_hash("a@b.com").len(), 64);
        assert_ne!(email_hash("a@b.com"), email_hash("c@b.com"));
    }

    #[test]
    fn round_trip_signs_and_verifies() {
        let c = claims(Plan::AnnualOwnKey, 1_700_000_000, 1_731_536_000);
        let key = sign(&c, &test_signing_key());
        assert!(key.starts_with("UBIQX-"));
        assert!(key
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '-'));
        let verified = verify_key_with(&key, &test_pubkey_hex()).unwrap();
        assert_eq!(verified, c);
        // Lower-cased and line-wrapped copies still verify.
        let sloppy = format!("  {}\n", key.to_lowercase().replace('-', "-\n"));
        assert_eq!(verify_key_with(&sloppy, &test_pubkey_hex()).unwrap(), c);
    }

    #[test]
    fn tampered_claims_fail_verification() {
        let c = claims(Plan::AnnualOwnKey, 1_700_000_000, 1_731_536_000);
        let key = sign(&c, &test_signing_key());
        let parsed = parse_key(&key).unwrap();
        let mut forged = parsed.claims.clone();
        forged.expires_at += 86_400 * 365;
        let forged_key = encode_key(&forged.canonical_json(), &parsed.signature);
        assert_eq!(
            verify_key_with(&forged_key, &test_pubkey_hex()),
            Err(LicenseError::BadSignature)
        );
        // Signed by another private key.
        let other = sign(&c, &SigningKey::from_bytes(&[9u8; 32]));
        assert_eq!(
            verify_key_with(&other, &test_pubkey_hex()),
            Err(LicenseError::BadSignature)
        );
    }

    #[test]
    fn malformed_keys_are_rejected() {
        let pk = test_pubkey_hex();
        assert!(matches!(
            verify_key_with("", &pk),
            Err(LicenseError::Malformed(_))
        ));
        assert!(matches!(
            verify_key_with("UBIQX-", &pk),
            Err(LicenseError::Malformed(_))
        ));
        assert!(matches!(
            verify_key_with("UBIQX-ABC", &pk),
            Err(LicenseError::Malformed(_))
        ));
        assert!(matches!(
            verify_key_with("UBIQX-ABC-DEF", &pk),
            Err(LicenseError::Malformed(_))
        ));
        assert!(matches!(
            verify_key_with("sk-ant-not-a-license", &pk),
            Err(LicenseError::Malformed(_))
        ));
        // Non-canonical claims (extra whitespace) are refused even before the signature.
        let c = claims(Plan::AnnualOwnKey, 1, 2);
        let mut loose = c.canonical_json();
        loose.push(b' ');
        let sig = test_signing_key().sign(&loose).to_bytes();
        assert!(matches!(
            verify_key_with(&encode_key(&loose, &sig), &pk),
            Err(LicenseError::Malformed(_))
        ));
        // Unknown claims version.
        let mut future = c.clone();
        future.v = 2;
        let payload = future.canonical_json();
        let sig = test_signing_key().sign(&payload).to_bytes();
        assert_eq!(
            verify_key_with(&encode_key(&payload, &sig), &pk),
            Err(LicenseError::UnsupportedVersion(2))
        );
        assert!(matches!(
            verify_key_with("UBIQX-ABC-DEF", "zz"),
            Err(LicenseError::Malformed(_))
        ));
    }

    #[test]
    fn status_reflects_validity_expiry_and_plan() {
        let now = Utc.with_ymd_and_hms(2026, 9, 18, 12, 0, 0).unwrap();
        let pk = test_pubkey_hex();

        let none = LicenseStatus::evaluate_with(None, &pk, now);
        assert_eq!(none.state, LicenseState::Unlicensed);
        assert_eq!(none.enforcement, LICENSE_ENFORCEMENT);
        assert!(none.key_hint.is_none());
        assert_eq!(LicenseStatus::evaluate_with(Some("  "), &pk, now), none);

        let valid_claims = claims(
            Plan::MonthlyManaged,
            now.timestamp() - 10,
            now.timestamp() + 86_400 * 30 + 5,
        );
        let key = sign(&valid_claims, &test_signing_key());
        let valid = LicenseStatus::evaluate_with(Some(&key), &pk, now);
        assert_eq!(valid.state, LicenseState::Valid);
        assert_eq!(valid.plan, Some(Plan::MonthlyManaged));
        assert_eq!(valid.days_left, Some(30));
        assert_eq!(valid.key_hint.as_deref(), Some(&key[key.len() - 4..]));
        assert_eq!(
            valid.expires_at.as_deref(),
            Some(valid_claims.expires_at_utc().to_rfc3339().as_str())
        );
        assert!(valid.allows_managed_ai());
        assert!(!valid.blocks_ai());
        assert!(valid.managed_usage.is_none());

        let expired_claims = claims(
            Plan::AnnualOwnKey,
            now.timestamp() - 400 * 86_400,
            now.timestamp() - 1,
        );
        let expired = LicenseStatus::evaluate_with(
            Some(&sign(&expired_claims, &test_signing_key())),
            &pk,
            now,
        );
        assert_eq!(expired.state, LicenseState::Expired);
        assert_eq!(expired.plan, Some(Plan::AnnualOwnKey));
        assert_eq!(expired.days_left, Some(0));
        assert!(!expired.allows_managed_ai());

        let invalid = LicenseStatus::evaluate_with(Some("UBIQX-NOPE-NOPE"), &pk, now);
        assert_eq!(invalid.state, LicenseState::Invalid);
        assert!(invalid.plan.is_none());
        assert_eq!(invalid.key_hint.as_deref(), Some("NOPE"));

        let annual = LicenseStatus::evaluate_with(
            Some(&sign(
                &claims(Plan::AnnualOwnKey, 0, now.timestamp() + 1),
                &test_signing_key(),
            )),
            &pk,
            now,
        );
        assert!(annual.is_valid());
        assert!(!annual.allows_managed_ai());
    }

    #[test]
    fn status_serialises_snake_case() {
        let status = LicenseStatus {
            state: LicenseState::Valid,
            plan: Some(Plan::MonthlyManaged),
            expires_at: Some("2027-09-18T12:00:00+00:00".into()),
            days_left: Some(365),
            key_hint: Some("ABCD".into()),
            enforcement: LicenseEnforcement::Soft,
            managed_usage: Some(ManagedUsage {
                month: "2026-09".into(),
                spent_usd: 1.5,
                budget_usd: 6.0,
            }),
        };
        let json = serde_json::to_value(&status).unwrap();
        assert_eq!(json["state"], "valid");
        assert_eq!(json["plan"], "monthly_managed");
        assert_eq!(json["enforcement"], "soft");
        assert_eq!(json["managed_usage"]["budget_usd"], 6.0);
        assert_eq!(json["key_hint"], "ABCD");
        let unl = serde_json::to_value(LicenseStatus::unlicensed()).unwrap();
        assert_eq!(unl["state"], "unlicensed");
        assert!(unl["plan"].is_null());
        assert!(unl["managed_usage"].is_null());
        let back: LicenseStatus = serde_json::from_value(json).unwrap();
        assert_eq!(back, status);
    }

    #[test]
    fn plan_ids_round_trip() {
        for plan in Plan::ALL {
            assert_eq!(Plan::parse(plan.id()), Some(plan));
            assert_eq!(serde_json::to_value(plan).unwrap(), plan.id());
        }
        assert_eq!(Plan::parse("annual"), Some(Plan::AnnualOwnKey));
        assert_eq!(Plan::parse("managed"), Some(Plan::MonthlyManaged));
        assert_eq!(Plan::parse("free"), None);
        assert!(Plan::MonthlyManaged.is_managed());
        assert!(!Plan::AnnualOwnKey.is_managed());
    }

    #[test]
    fn embedded_public_key_is_well_formed() {
        assert!(verifying_key_from_hex(UBIQX_LICENSE_PUBKEY_HEX).is_ok());
    }
}
