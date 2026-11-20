//! The unprivileged side: what it reports, what it asks the helper for, and how it reads
//! the helper's exit.

use ubiqx_core::nofap::{
    ApplyOutcome, ObserveOpts, Os, RemoveScope, SystemShield, Tier, TierCapability, TierState,
};
use ubiqx_core::{CoreError, ProductId};
use ubiqx_nofap::gate::Gate;
use ubiqx_nofap::plan::{self, EXIT_FAILED, EXIT_GATED, EXIT_INVALID_PLAN};
use ubiqx_nofap::{hosts, resolvers};

use crate::sysblock::testing::{all_open, plan_of, Rig, ALL_TIERS};
use crate::sysblock::{ElevatedExit, NativeSystemShield};

fn strings(list: &[&str]) -> Vec<String> {
    list.iter().map(|item| (*item).to_owned()).collect()
}

#[test]
fn capabilities_follow_the_gate_and_the_packaging() {
    for os in Os::ALL {
        let rig = Rig::new(os);
        // The table as it ships: every tier is gated.
        let closed = rig
            .shield(ProductId::Ubiqx, Gate::new(os, Vec::new()))
            .capabilities();
        assert_eq!(closed.os, Some(os));
        for tier in Tier::ALL {
            assert_eq!(*closed.of(tier), TierCapability::Gated, "{os:?} {tier:?}");
        }
        assert!(closed.unverified.is_empty());
        // A tester's override opens what it names and is labelled.
        let partly = rig
            .shield(ProductId::Ubiqx, Gate::new(os, vec![Tier::Hosts]))
            .capabilities();
        assert_eq!(partly.hosts, TierCapability::Available);
        assert_eq!(partly.resolver, TierCapability::Gated);
        assert_eq!(partly.unverified, vec![Tier::Hosts]);
        // From an AppImage nothing can be applied, and the reason is given.
        let appimage = rig
            .shield(ProductId::Ubiqx, all_open(os))
            .running_from_appimage(true);
        let unsupported = TierCapability::Unsupported {
            reason: "appimage".into(),
        };
        assert_eq!(appimage.capabilities().hosts, unsupported);
        assert_eq!(appimage.capabilities().policies, unsupported);
        let seen = appimage.observe(ObserveOpts::QUICK).unwrap();
        assert_eq!(seen.hosts.error.as_deref(), Some("appimage"));
    }
}

#[test]
fn names_are_looked_up_only_when_the_caller_asks() {
    let rig = Rig::new(Os::Linux);
    let shield = rig.shield(ProductId::Ubiqx, all_open(Os::Linux));
    let quick = shield.observe(ObserveOpts::QUICK).unwrap();
    assert_eq!((quick.canary, quick.probe_ok), (None, None));
    shield.observe(ObserveOpts::FULL).unwrap();
    assert!(
        rig.names.asked.lock().is_empty(),
        "the reserved name never leaves unasked"
    );
    let canary = shield
        .observe(ObserveOpts {
            canary: true,
            ..ObserveOpts::QUICK
        })
        .unwrap();
    assert_eq!(canary.canary, Some(strings(&["203.0.113.10"])));
    assert_eq!(canary.probe_ok, None);
    assert_eq!(*rig.names.asked.lock(), [hosts::CANARY_NAME]);
    let probe = shield
        .observe(ObserveOpts {
            probe: true,
            ..ObserveOpts::QUICK
        })
        .unwrap();
    assert_eq!((probe.canary, probe.probe_ok), (None, Some(true)));
    assert_eq!(
        rig.names.asked.lock().last().map(String::as_str),
        Some(resolvers::PROBE_NAME)
    );
}

#[test]
fn apply_and_remove_send_exactly_these_arguments() {
    for os in Os::ALL {
        let rig = Rig::new(os);
        let shield = rig.shield(ProductId::Shield, all_open(os));
        assert_eq!(shield.owner(), ProductId::Shield);
        let plan = plan_of(ProductId::Shield, &["blocked.example"]);
        assert_eq!(
            shield.apply(&plan).unwrap(),
            ApplyOutcome::Applied,
            "{os:?}"
        );
        assert_eq!(
            shield
                .remove(ProductId::Shield, RemoveScope::Resolver)
                .unwrap(),
            ApplyOutcome::Applied
        );
        assert_eq!(
            shield.remove(ProductId::Shield, RemoveScope::All).unwrap(),
            ApplyOutcome::Applied
        );
        let encoded = plan::encode_plan(&plan);
        assert_eq!(
            *rig.elevated.lock(),
            vec![
                strings(&["apply", "--plan-b64", &encoded, "--unverified", ALL_TIERS]),
                strings(&["remove", "--owner", "shield", "--scope", "resolver"]),
                strings(&["remove", "--owner", "shield", "--scope", "all"]),
            ],
            "{os:?}"
        );
        assert!(rig.observe().state.is_none());
    }
}

#[test]
fn apply_through_the_shield_reads_applied_for_what_was_asked() {
    for os in Os::ALL {
        let rig = Rig::new(os);
        let shield = rig.shield(ProductId::Ubiqx, all_open(os));
        // Hosts only: the other tiers stay off.
        let mut plan = plan_of(ProductId::Ubiqx, &[]);
        plan.resolver = None;
        plan.policies = false;
        let preview = shield.preview(&plan).unwrap();
        assert_eq!(preview.len(), 1, "{os:?}: {preview:?}");
        assert_eq!(preview[0].text_key, "preview.hosts_add");
        let names = hosts::cap_for(os) - 1;
        assert_eq!(
            preview[0].detail.as_deref(),
            Some(names.to_string().as_str())
        );
        assert_eq!(shield.apply(&plan).unwrap(), ApplyOutcome::Applied);
        let judged = rig.assess(&plan);
        assert_eq!(judged.hosts, TierState::Applied, "{os:?}");
        assert_eq!(judged.resolver, TierState::Off, "{os:?}");
        assert_eq!(judged.policies, TierState::Off, "{os:?}");
        assert_eq!(
            shield.preview(&plan).unwrap(),
            Vec::new(),
            "nothing left to change"
        );
    }
}

#[test]
fn a_plan_the_helper_would_refuse_never_reaches_the_prompt() {
    let rig = Rig::new(Os::Linux);
    let plan = plan_of(ProductId::Ubiqx, &[]);
    // The gate as it ships: nobody is asked for a password for a tier that is closed.
    let closed = rig.shield(ProductId::Ubiqx, Gate::new(Os::Linux, Vec::new()));
    assert_eq!(
        closed.apply(&plan).unwrap(),
        ApplyOutcome::Unavailable("gated".into())
    );
    // A preview of a gated plan shows no change at all.
    assert_eq!(closed.preview(&plan).unwrap(), Vec::new());
    // A plan with a name that cannot be blocked is an error, not a prompt.
    let open = rig.shield(ProductId::Ubiqx, all_open(Os::Linux));
    for name in ["localhost", "10.0.0.1", "portal.example.gov.br"] {
        let bad = plan_of(ProductId::Ubiqx, &[name]);
        assert!(
            matches!(open.apply(&bad), Err(CoreError::Invalid(_))),
            "{name}"
        );
    }
    assert!(rig.elevated.lock().is_empty());
    assert_eq!(rig.fs.mutations(), Vec::<String>::new());
}

#[test]
fn the_exit_of_the_elevated_run_becomes_the_outcome() {
    let rig = Rig::new(Os::Macos);
    let shield = rig.shield(ProductId::Ubiqx, all_open(Os::Macos));
    let plan = plan_of(ProductId::Ubiqx, &[]);
    let script = |exit: ElevatedExit| rig.elevations.lock().push_back(exit);

    script(ElevatedExit::Declined);
    assert_eq!(shield.apply(&plan).unwrap(), ApplyOutcome::Declined);
    script(ElevatedExit::Unavailable("no_polkit_agent".into()));
    assert_eq!(
        shield.apply(&plan).unwrap(),
        ApplyOutcome::Unavailable("no_polkit_agent".into())
    );
    script(ElevatedExit::Exited(EXIT_GATED));
    assert_eq!(
        shield.apply(&plan).unwrap(),
        ApplyOutcome::Unavailable("gated".into())
    );
    script(ElevatedExit::Exited(EXIT_FAILED));
    assert_eq!(
        shield.apply(&plan).unwrap(),
        ApplyOutcome::Unavailable("failed".into())
    );
    script(ElevatedExit::Exited(137));
    assert_eq!(
        shield.remove(ProductId::Ubiqx, RemoveScope::All).unwrap(),
        ApplyOutcome::Unavailable("failed".into())
    );
    script(ElevatedExit::Exited(EXIT_INVALID_PLAN));
    assert!(matches!(shield.apply(&plan), Err(CoreError::Invalid(_))));
    // None of these changed the machine.
    assert!(rig.observe().state.is_none());
    assert!(!rig.hosts_text().contains(hosts::BEGIN_PREFIX));

    // A partial run: the failed tiers come from the report in the state file.
    rig.mac.broken.lock().insert("/usr/bin/defaults".to_owned());
    let outcome = shield.apply(&plan).unwrap();
    let ApplyOutcome::Partial(failed) = outcome else {
        panic!("a partial outcome, got {outcome:?}")
    };
    assert_eq!(failed.len(), 1);
    assert_eq!((failed[0].tier, failed[0].ok), (Tier::Policies, false));
    assert_eq!(failed[0].reason.as_deref(), Some("failed"));
}

#[test]
fn the_native_shield_of_this_machine_ships_with_every_tier_gated() {
    // Reads the gate and the executable path only; nothing on the system is touched.
    // The test runs on the three systems in CI: each of them has a back-end.
    let shield = NativeSystemShield::native(ProductId::Ubiqx).expect("this system has a back-end");
    let capabilities = shield.capabilities();
    assert!(capabilities.os.is_some());
    assert_eq!(capabilities.os, Os::current());
    for tier in Tier::ALL {
        if !capabilities.unverified.contains(&tier) {
            assert_eq!(*capabilities.of(tier), TierCapability::Gated, "{tier:?}");
        }
    }
    // A gated plan is turned down before any elevation is attempted.
    if capabilities.unverified.is_empty() {
        let plan = plan_of(ProductId::Ubiqx, &[]);
        assert_eq!(
            shield.apply(&plan).unwrap(),
            ApplyOutcome::Unavailable("gated".into())
        );
    }
}
