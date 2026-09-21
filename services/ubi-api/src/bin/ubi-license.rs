//! `ubi-license`: the operator's key tooling.
//!
//! ```text
//! ubi-license keygen
//! ubi-license issue --plan monthly_managed --email x@y.z --months 1 --privkey HEX
//! ubi-license verify --key UBIQX-... [--pubkey HEX]
//! ```

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand};
use ubi_api::issue;
use ubiqx_core::license::{self, LicenseStatus, Plan};

#[derive(Parser)]
#[command(
    name = "ubi-license",
    about = "ubiqX license keys: generate key pairs, issue and verify keys"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Prints a new Ed25519 key pair (hex). Keep the private key secret; commit only the
    /// public key (ubiqx-core `UBIQX_LICENSE_PUBKEY_HEX`, proxy `UBI_LICENSE_PUBKEY_HEX`).
    Keygen,
    /// Issues a signed license key.
    Issue {
        /// annual_own_key | monthly_managed
        #[arg(long)]
        plan: String,
        /// Subscriber e-mail (only its SHA-256 goes into the key).
        #[arg(long)]
        email: String,
        /// Validity in calendar months from now (or from --issued-at).
        #[arg(long, default_value_t = 12)]
        months: u32,
        /// Hex Ed25519 private key (or set UBI_LICENSE_PRIVKEY_HEX).
        #[arg(long, env = "UBI_LICENSE_PRIVKEY_HEX", hide_env_values = true)]
        privkey: String,
        /// Subscriber id; derived from --external-id / the e-mail when omitted.
        #[arg(long)]
        sub: Option<String>,
        /// Payment platform id used to derive a stable subscriber id.
        #[arg(long)]
        external_id: Option<String>,
        /// Issue date (RFC 3339) instead of now; lets you mint an already expired key for tests.
        #[arg(long)]
        issued_at: Option<String>,
    },
    /// Verifies a key and prints its claims and state.
    Verify {
        #[arg(long)]
        key: String,
        /// Hex public key; defaults to the one built into ubiqx-core.
        #[arg(long, env = "UBI_LICENSE_PUBKEY_HEX")]
        pubkey: Option<String>,
    },
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Keygen => {
            let pair = issue::keygen();
            println!("private key (hex, keep secret): {}", pair.private_hex);
            println!("public key  (hex, commit):      {}", pair.public_hex);
        }
        Command::Issue {
            plan,
            email,
            months,
            privkey,
            sub,
            external_id,
            issued_at,
        } => {
            let plan = Plan::parse(&plan).with_context(|| {
                format!("unknown plan `{plan}` (annual_own_key | monthly_managed)")
            })?;
            let signing_key = issue::signing_key_from_hex(&privkey)?;
            let now = match issued_at {
                Some(s) => DateTime::parse_from_rfc3339(&s)
                    .with_context(|| format!("--issued-at `{s}` is not RFC 3339"))?
                    .with_timezone(&Utc),
                None => Utc::now(),
            };
            let email_hash = license::email_hash(&email);
            let sub = sub.unwrap_or_else(|| issue::derive_sub(external_id.as_deref(), &email_hash));
            let claims = issue::claims_for(plan, sub, email_hash, months, now)?;
            let key = issue::issue(&claims, &signing_key);
            eprintln!(
                "plan={} sub={} expires_at={}",
                claims.plan.id(),
                claims.sub,
                claims.expires_at_utc().to_rfc3339()
            );
            println!("{key}");
        }
        Command::Verify { key, pubkey } => {
            let pubkey = pubkey.unwrap_or_else(|| license::UBIQX_LICENSE_PUBKEY_HEX.to_string());
            let now = Utc::now();
            match license::verify_key_with(&key, &pubkey) {
                Ok(claims) => {
                    let status = LicenseStatus::evaluate_with(Some(&key), &pubkey, now);
                    println!(
                        "state:      {}",
                        serde_json::to_value(status.state)?.as_str().unwrap_or("?")
                    );
                    println!("plan:       {}", claims.plan.id());
                    println!("sub:        {}", claims.sub);
                    println!("email_hash: {}", claims.email_hash);
                    println!("issued_at:  {}", claims.issued_at_utc().to_rfc3339());
                    println!("expires_at: {}", claims.expires_at_utc().to_rfc3339());
                    println!("days_left:  {}", status.days_left.unwrap_or(0));
                    println!("seats:      {}", claims.seats);
                    if claims.is_expired_at(now) {
                        eprintln!("key is expired");
                        std::process::exit(1);
                    }
                }
                Err(e) => {
                    eprintln!("invalid key: {e}");
                    std::process::exit(1);
                }
            }
        }
    }
    Ok(())
}
