//! The helper's command line: a closed set of verbs, the documented exit codes, and
//! nothing written or run for a request it refuses.

use chrono::{TimeZone, Utc};
use ubiqx_core::nofap::{
    NofapConfig, NofapSite, Os, RemoveScope, SiteList, SiteOrigin, SystemObservation,
};
use ubiqx_core::ProductId;
use ubiqx_nofap::plan::{self, ARG_BUDGET_WINDOWS, EXIT_APPLIED, EXIT_GATED, EXIT_INVALID_PLAN};
use ubiqx_nofap::{hosts, matcher};

use crate::sysblock::elevate;
use crate::sysblock::helper::{after_flag, apply_args, parse, remove_args, Verb};
use crate::sysblock::testing::{all_open, b64, plan_of, Rig, ALL_TIERS, CF, EXE};

/// Runs the helper and asserts that it refused with `code` before touching anything.
fn refused(rig: &Rig, args: &[&str], code: i32) {
    rig.fs.clear_log();
    rig.clear_calls();
    let (exit, out, err) = rig.helper(args);
    assert_eq!(exit, code, "{:?} {args:?}: {err}", rig.os);
    assert_eq!(out, "", "{args:?}");
    assert!(!err.trim().is_empty(), "a refusal says why: {args:?}");
    assert_eq!(rig.fs.mutations(), Vec::<String>::new(), "{args:?}");
    assert_eq!(rig.calls(), Vec::new(), "{args:?}");
    assert!(rig.names.asked.lock().is_empty(), "{args:?}");
}

#[test]
fn anything_that_is_not_a_verb_is_refused_with_exit_2() {
    for os in Os::ALL {
        let rig = Rig::new(os);
        let good = plan::encode_plan(&plan_of(ProductId::Ubiqx, &[]));
        for args in [
            &[][..],
            &["bogus"],
            &["run", "/bin/sh"],
            &["status", "extra"],
            &["--shield-helper", "status"],
            &["apply"],
            &["apply", "--plan-b64"],
            &["apply", &good],
            &["apply", "--plan-b64", &good, "--plan-b64", &good],
            &["apply", "--plan-b64", &good, "--unverified"],
            &["apply", "--plan-b64", &good, "--force", "yes"],
            &["apply", "--plan", &good],
            &["elevate-argv"],
            &["remove"],
            &["remove", "--owner", "ubiqx"],
            &["remove", "--scope", "all"],
            &["remove", "--owner", "root", "--scope", "all"],
            &["remove", "--owner", "ubiqx", "--scope", "everything"],
            &[
                "remove",
                "--owner",
                "ubiqx",
                "--scope",
                "all",
                "--path",
                "/etc/passwd",
            ],
            &["APPLY", "--plan-b64", &good],
        ] {
            refused(&rig, args, EXIT_INVALID_PLAN);
        }
    }
}

#[test]
fn a_plan_the_helper_cannot_accept_is_refused_before_anything_is_touched() {
    let never_blocked = "portal.example.gov.br";
    assert!(matcher::never_block().covers(never_blocked));
    let hand_made = |resolver: &str, extra: &str| {
        b64(&format!(
            "{{\"v\":1,\"owner\":\"ubiqx\",\"hosts\":true,\"extra\":[{extra}],\"resolver\":{resolver},\"policies\":false}}"
        ))
    };
    for os in Os::ALL {
        let rig = Rig::new(os);
        let tiers = ["--unverified", ALL_TIERS];
        let invalid = |plan: &str| {
            refused(
                &rig,
                &["apply", "--plan-b64", plan, tiers[0], tiers[1]],
                EXIT_INVALID_PLAN,
            );
        };
        // Oversized: one character more than the helper accepts.
        invalid(&"A".repeat(plan::HELPER_ARG_MAX + 1));
        // Not base64url, not JSON, not a plan.
        invalid("!!!");
        invalid(&b64("not json"));
        invalid(&b64("{}"));
        // An address where a resolver id belongs: there are no custom addresses.
        invalid(&hand_made("\"1.2.3.4\"", ""));
        invalid(&hand_made("\"2606:4700:4700::1113\"", ""));
        invalid(&hand_made("\"https://dns.example/query\"", ""));
        // An owner outside the closed set, another version, a member too many.
        invalid(&b64("{\"v\":1,\"owner\":\"root\",\"hosts\":true,\"extra\":[],\"resolver\":null,\"policies\":false}"));
        invalid(&b64("{\"v\":2,\"owner\":\"ubiqx\",\"hosts\":true,\"extra\":[],\"resolver\":null,\"policies\":false}"));
        invalid(&b64("{\"v\":1,\"owner\":\"ubiqx\",\"hosts\":true,\"extra\":[],\"resolver\":null,\"policies\":false,\"run\":\"id\"}"));
        // Names that must never reach the hosts file.
        invalid(&hand_made("null", &format!("\"{never_blocked}\"")));
        for name in [
            "127.0.0.1",
            "localhost",
            "a b.example",
            "evil.example\\n0.0.0.0 bank.example",
            "-x.example",
            "UPPER.example",
        ] {
            invalid(&hand_made("null", &format!("\"{name}\"")));
        }
        let many: Vec<String> = (0..401).map(|n| format!("\"site{n}.example\"")).collect();
        invalid(&hand_made("null", &many.join(",")));

        // A valid plan for a tier the gate keeps closed: exit 5, with and without an
        // override for another tier.
        let wanted = plan::encode_plan(&plan_of(ProductId::Shield, &["blocked.example"]));
        refused(&rig, &["apply", "--plan-b64", &wanted], EXIT_GATED);
        refused(
            &rig,
            &["apply", "--plan-b64", &wanted, "--unverified", "hosts"],
            EXIT_GATED,
        );
        refused(
            &rig,
            &[
                "apply",
                "--plan-b64",
                &wanted,
                "--unverified",
                "hosts,policies",
            ],
            EXIT_GATED,
        );
        refused(
            &rig,
            &["apply", "--plan-b64", &wanted, "--unverified", "everything"],
            EXIT_GATED,
        );
        assert_eq!(rig.hosts_text(), rig.observe().hosts.text.unwrap());
        assert!(rig.observe().state.is_none());
    }
}

#[test]
fn status_prints_the_observation_and_changes_nothing() {
    for os in Os::ALL {
        let rig = Rig::new(os);
        rig.apply(&plan_of(ProductId::Ubiqx, &["blocked.example"]));
        rig.fs.clear_log();
        rig.names.asked.lock().clear();
        let (exit, out, err) = rig.helper(&["status"]);
        assert_eq!((exit, err.as_str()), (EXIT_APPLIED, ""));
        assert!(
            rig.names.asked.lock().is_empty(),
            "`status` looks no name up"
        );
        let printed: SystemObservation = serde_json::from_str(&out).expect("valid JSON");
        let mut seen = rig.observe();
        (seen.canary, seen.probe_ok) = (None, None);
        assert_eq!(printed, seen, "{os:?}");
        assert!(printed.hosts.text.unwrap().contains(hosts::CANARY_NAME));
        assert_eq!(
            printed.state.unwrap().owners[&ProductId::Ubiqx].resolver,
            Some(CF)
        );
        assert_eq!(rig.fs.mutations(), Vec::<String>::new());
    }
}

#[test]
fn elevate_argv_prints_the_real_command_line_and_runs_nothing() {
    for os in Os::ALL {
        let rig = Rig::new(os);
        let encoded = plan::encode_plan(&plan_of(ProductId::Ubiqx, &[]));
        let (exit, out, _) = rig.helper(&[
            "elevate-argv",
            "--plan-b64",
            &encoded,
            "--unverified",
            "resolver, HOSTS ,junk",
        ]);
        assert_eq!(exit, EXIT_APPLIED);
        let printed: Vec<String> = serde_json::from_str(&out).expect("a JSON array of strings");
        let backend = crate::sysblock::testing::backend_for(os);
        // The tier list is normalised, never passed through as it came.
        let args = apply_args(&encoded, Some("hosts,resolver"));
        let expected = elevate::argv(
            os,
            &backend.elevation_tool(),
            std::path::Path::new(EXE),
            &args,
        );
        assert_eq!(printed, expected, "{os:?}");
        assert_eq!(rig.calls(), Vec::new());
        assert_eq!(rig.fs.mutations(), Vec::<String>::new());
        assert!(rig.elevated.lock().is_empty());
        refused(
            &rig,
            &["elevate-argv", "--plan-b64", "!!!"],
            EXIT_INVALID_PLAN,
        );
    }
}

#[test]
fn the_parser_reads_exactly_the_documented_forms() {
    let line = |words: &[&str]| -> Vec<String> { words.iter().map(|w| (*w).to_owned()).collect() };
    assert_eq!(parse(&line(&["status"])), Some(Verb::Status));
    assert_eq!(
        parse(&line(&[
            "apply",
            "--unverified",
            "hosts",
            "--plan-b64",
            "abc"
        ])),
        Some(Verb::Apply {
            plan: "abc".into(),
            unverified: Some("hosts".into())
        })
    );
    assert_eq!(
        parse(&line(&["elevate-argv", "--plan-b64", "abc"])),
        Some(Verb::ElevateArgv {
            plan: "abc".into(),
            unverified: None
        })
    );
    assert_eq!(
        parse(&line(&["remove", "--scope", "resolver", "--owner", "all"])),
        Some(Verb::Remove {
            owner: None,
            scope: RemoveScope::Resolver
        })
    );
    // What the app sends parses back to what it meant.
    assert_eq!(
        parse(&remove_args(ProductId::Shield, RemoveScope::All)),
        Some(Verb::Remove {
            owner: Some(ProductId::Shield),
            scope: RemoveScope::All
        })
    );
    assert_eq!(
        parse(&apply_args("abc", None)),
        Some(Verb::Apply {
            plan: "abc".into(),
            unverified: None
        })
    );
    assert!(apply_args("abc", Some("hosts"))
        .iter()
        .all(|arg| elevate::is_helper_arg(arg)));
    assert!(remove_args(ProductId::Ubiqx, RemoveScope::Resolver)
        .iter()
        .all(|arg| elevate::is_helper_arg(arg)));
}

#[test]
fn the_small_binary_takes_the_verb_with_or_without_the_flag() {
    let line = |words: &[&str]| -> Vec<String> { words.iter().map(|w| (*w).to_owned()).collect() };
    let bare = line(&["remove", "--owner", "all", "--scope", "all"]);
    let flagged = line(&[
        "--shield-helper",
        "remove",
        "--owner",
        "all",
        "--scope",
        "all",
    ]);
    assert_eq!(after_flag(&bare), &bare[..]);
    assert_eq!(after_flag(&flagged), &bare[..]);
    assert_eq!(after_flag(&[]), &[] as &[String]);
    // Only one leading flag is skipped, and only in front.
    let twice = line(&["--shield-helper", "--shield-helper", "status"]);
    assert_eq!(parse(after_flag(&twice)), None);
    assert_eq!(
        parse(after_flag(&line(&["status", "--shield-helper"]))),
        None
    );
    // What `elevate-argv` prints for a plan ends in arguments this binary accepts.
    let rig = Rig::new(Os::Linux);
    let encoded = plan::encode_plan(&plan_of(ProductId::Ubiqx, &[]));
    let (_, out, _) = rig.helper(&["elevate-argv", "--plan-b64", &encoded]);
    let printed: Vec<String> = serde_json::from_str(&out).unwrap();
    // pkexec, the executable, then what the executable is given.
    assert_eq!(
        parse(after_flag(&printed[2..])),
        Some(Verb::Apply {
            plan: encoded,
            unverified: None
        })
    );
}

fn user_site(n: usize) -> NofapSite {
    NofapSite {
        id: format!("site-{n}"),
        list: SiteList::Block,
        origin: SiteOrigin::User,
        host: Some(format!("a-long-site-name-number-{n:04}.example")),
        host_key: [0; 16],
        created_at: Utc.timestamp_opt(1_790_000_000 + n as i64, 0).unwrap(),
        expires_at: None,
    }
}

#[test]
fn a_plan_longer_than_the_argument_budget_is_never_sent() {
    let mut config = NofapConfig::default();
    config.system.hosts = true;
    config.system.hosts_user_sites = true;
    let sites: Vec<NofapSite> = (0..300).map(user_site).collect();
    let built = plan::build_plan(&config, &sites, ProductId::Ubiqx, Os::Windows);
    let encoded = plan::encode_plan(&built.plan);
    assert!(encoded.len() <= ARG_BUDGET_WINDOWS, "{}", encoded.len());
    assert!(built.extras_left_out > 0);
    assert_eq!(built.plan.extra.len() + built.extras_left_out as usize, 300);
    // The most recently added names are the ones kept.
    assert_eq!(built.plan.extra[0], "a-long-site-name-number-0299.example");

    // The plan that fits goes through, and the helper writes each name's `www.` twin.
    let rig = Rig::new(Os::Windows);
    let shield = rig.shield(ProductId::Ubiqx, all_open(Os::Windows));
    use ubiqx_core::nofap::SystemShield;
    assert_eq!(
        shield.apply(&built.plan).unwrap(),
        ubiqx_core::nofap::ApplyOutcome::Applied
    );
    let names = hosts::block_names(rig.hosts_text().as_bytes());
    for name in &built.plan.extra {
        assert!(
            names.contains(name) && names.contains(&format!("www.{name}")),
            "{name}"
        );
    }
    assert!(!built.plan.extra.iter().any(|name| name.starts_with("www.")));

    // The same sites in one plan built by hand are over the budget: nothing is sent.
    let mut too_long = built.plan.clone();
    too_long.extra = sites.iter().filter_map(|site| site.host.clone()).collect();
    assert!(plan::encode_plan(&too_long).len() > ARG_BUDGET_WINDOWS);
    rig.elevated.lock().clear();
    assert!(shield.apply(&too_long).is_err());
    assert!(
        rig.elevated.lock().is_empty(),
        "no prompt for a plan that cannot travel"
    );
    // On Linux and macOS the same plan fits.
    let unix = Rig::new(Os::Linux);
    let outcome = unix
        .shield(ProductId::Ubiqx, all_open(Os::Linux))
        .apply(&too_long);
    assert_eq!(outcome.unwrap(), ubiqx_core::nofap::ApplyOutcome::Applied);
}
