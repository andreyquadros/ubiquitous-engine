//! License round trip through the `ubi-license` binary and the shared verifier.

use std::process::Command;

use chrono::Utc;
use ubiqx_core::license::{self, LicenseState, LicenseStatus, Plan};

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_ubi-license"))
}

fn run(args: &[&str]) -> (bool, String, String) {
    let out = cli().args(args).output().expect("ubi-license runs");
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).trim().to_string(),
        String::from_utf8_lossy(&out.stderr).trim().to_string(),
    )
}

fn parse_keygen(stdout: &str) -> (String, String) {
    let mut private = None;
    let mut public = None;
    for line in stdout.lines() {
        if let Some((label, value)) = line.split_once(':') {
            if label.starts_with("private key") {
                private = Some(value.trim().to_string());
            } else if label.starts_with("public key") {
                public = Some(value.trim().to_string());
            }
        }
    }
    (private.unwrap(), public.unwrap())
}

#[test]
fn keygen_issue_verify_round_trip() {
    let (ok, stdout, _) = run(&["keygen"]);
    assert!(ok);
    let (privkey, pubkey) = parse_keygen(&stdout);
    assert_eq!(privkey.len(), 64);
    assert_eq!(pubkey.len(), 64);

    for (plan, id) in [
        (Plan::AnnualOwnKey, "annual_own_key"),
        (Plan::MonthlyManaged, "monthly_managed"),
    ] {
        let (ok, key, stderr) = run(&[
            "issue",
            "--plan",
            id,
            "--email",
            "Pessoa@Exemplo.com",
            "--months",
            "12",
            "--privkey",
            &privkey,
        ]);
        assert!(ok, "{stderr}");
        assert!(key.starts_with("UBIQX-"));
        assert!(stderr.contains(&format!("plan={id}")));

        let claims = license::verify_key_with(&key, &pubkey).unwrap();
        assert_eq!(claims.plan, plan);
        assert_eq!(claims.email_hash, license::email_hash("pessoa@exemplo.com"));
        assert_eq!(claims.seats, 1);
        let days = claims.days_left_at(Utc::now());
        assert!((360..=366).contains(&days), "{days}");

        let status = LicenseStatus::evaluate_with(Some(&key), &pubkey, Utc::now());
        assert_eq!(status.state, LicenseState::Valid);
        assert_eq!(status.plan, Some(plan));

        // The CLI verifier agrees.
        let (ok, out, _) = run(&["verify", "--key", &key, "--pubkey", &pubkey]);
        assert!(ok);
        assert!(out.contains("state:      valid"));
        assert!(out.contains(&format!("plan:       {id}")));

        // …and rejects it under another public key.
        let (_, other_keys, _) = run(&["keygen"]);
        let (_, other_pub) = parse_keygen(&other_keys);
        let (ok, _, err) = run(&["verify", "--key", &key, "--pubkey", &other_pub]);
        assert!(!ok);
        assert!(err.contains("invalid key"));
        assert_eq!(
            license::verify_key_with(&key, &other_pub),
            Err(license::LicenseError::BadSignature)
        );
    }
}

#[test]
fn issue_can_backdate_and_derives_a_stable_sub() {
    let (_, stdout, _) = run(&["keygen"]);
    let (privkey, pubkey) = parse_keygen(&stdout);
    let (ok, key, _) = run(&[
        "issue",
        "--plan",
        "monthly_managed",
        "--email",
        "x@y.z",
        "--months",
        "1",
        "--privkey",
        &privkey,
        "--issued-at",
        "2025-01-01T00:00:00Z",
        "--external-id",
        "ext-42",
    ]);
    assert!(ok);
    let claims = license::verify_key_with(&key, &pubkey).unwrap();
    assert_eq!(
        claims.issued_at_utc().to_rfc3339(),
        "2025-01-01T00:00:00+00:00"
    );
    assert_eq!(
        claims.expires_at_utc().to_rfc3339(),
        "2025-02-01T00:00:00+00:00"
    );
    assert!(claims.is_expired_at(Utc::now()));
    assert_eq!(
        LicenseStatus::evaluate_with(Some(&key), &pubkey, Utc::now()).state,
        LicenseState::Expired
    );
    let (ok, out, err) = run(&["verify", "--key", &key, "--pubkey", &pubkey]);
    assert!(!ok);
    assert!(out.contains("state:      expired"));
    assert!(err.contains("expired"));

    // Same external id → same sub; explicit --sub wins.
    let (_, key2, _) = run(&[
        "issue",
        "--plan",
        "monthly_managed",
        "--email",
        "other@y.z",
        "--privkey",
        &privkey,
        "--external-id",
        "ext-42",
    ]);
    assert_eq!(
        license::verify_key_with(&key2, &pubkey).unwrap().sub,
        claims.sub
    );
    let (_, key3, _) = run(&[
        "issue",
        "--plan",
        "annual_own_key",
        "--email",
        "x@y.z",
        "--privkey",
        &privkey,
        "--sub",
        "sub_custom",
    ]);
    assert_eq!(
        license::verify_key_with(&key3, &pubkey).unwrap().sub,
        "sub_custom"
    );

    // Bad inputs.
    assert!(
        !run(&[
            "issue",
            "--plan",
            "gold",
            "--email",
            "x@y.z",
            "--privkey",
            &privkey
        ])
        .0
    );
    assert!(
        !run(&[
            "issue",
            "--plan",
            "annual",
            "--email",
            "x@y.z",
            "--privkey",
            "zz"
        ])
        .0
    );
    assert!(
        !run(&[
            "issue",
            "--plan",
            "annual",
            "--email",
            "x@y.z",
            "--months",
            "0",
            "--privkey",
            &privkey
        ])
        .0
    );
    assert!(!run(&["verify", "--key", "UBIQX-NOPE", "--pubkey", &pubkey]).0);
}

#[test]
fn embedded_dev_public_key_verifies_the_committed_samples() {
    // The public half of the development key pair is committed in ubiqx-core; a key issued
    // with the matching private key (kept outside the repo) verifies with no --pubkey.
    let sample = "UBIQX-PMRGK3LBNFWF62DBONUCEORCMM3TOODGHFTDCM3EMQZDQM3FGI2DCNZVGNTDCNRXMEYDQYJVGFRDOMDEGVRTEYRVGU4TQMZYMRRTMNZYGQ4WKZTEHE2TKYRWGA2TSMRCFQRGK6DQNFZGK427MF2CEORRHAZDCMRSGU3DAMBMEJUXG43VMVSF6YLUEI5DCNZYHE3DQOJWGAYCYITQNRQW4IR2EJWW63TUNBWHSX3NMFXGCZ3FMQRCYITTMVQXI4ZCHIYSYITTOVRCEORCON2WEX3EMV3F63LBNZQWOZLEL4YDAMBREIWCE5RCHIYX2-C3OV2CX6HHJXELQ6HHGTKY2WW2QTQUPN5IITZKYHECN6N52PFEGYY7GRRPJWAGE6VSFJO2UQSPLKDXF7IRC5ZZZBCC2LHGHWNNGPIBY";
    let claims = license::verify_key(sample).unwrap();
    assert_eq!(claims.plan, Plan::MonthlyManaged);
    assert_eq!(claims.sub, "sub_dev_managed_0001");
    let (ok, out, _) = run(&["verify", "--key", sample]);
    assert!(ok, "{out}");
    assert!(out.contains("plan:       monthly_managed"));
}
