//! The unprivileged side: what it reports, what it asks the helper for, and how it reads
//! the helper's exit.

use ubiqx_core::nofap::{
    ApplyOutcome, ObserveOpts, Os, RemoveScope, SystemShield, Tier, TierCapability, TierState,
};
use ubiqx_core::{CoreError, ProductId};
use ubiqx_nofap::gate::{self, Gate};
use ubiqx_nofap::plan::{self, EXIT_FAILED, EXIT_GATED, EXIT_INVALID_PLAN};
use ubiqx_nofap::{hosts, resolvers};

use crate::sysblock::testing::{
    all_open, closed_tier, plan_for, plan_of, unverified_arg, Rig, ALL_TIERS,
};
use crate::sysblock::{ElevatedExit, NativeSystemShield};

fn strings(list: &[&str]) -> Vec<String> {
    list.iter().map(|item| (*item).to_owned()).collect()
}

#[test]
fn capabilities_follow_the_gate_and_the_packaging() {
    for os in Os::ALL {
        let rig = Rig::new(os);
        // Without an override a tier is what the table says: gated until it is opened.
        let by_table = |tier: Tier| {
            if gate::tier_enabled(os, tier) {
                TierCapability::Available
            } else {
                TierCapability::Gated
            }
        };
        let plain = rig
            .shield(ProductId::Ubiqx, Gate::new(os, Vec::new()))
            .capabilities();
        assert_eq!(plain.os, Some(os));
        for tier in Tier::ALL {
            assert_eq!(*plain.of(tier), by_table(tier), "{os:?} {tier:?}");
        }
        assert!(plain.unverified.is_empty());
        // A tester's override opens what it names, and what it opened is labelled.
        let partly = rig
            .shield(ProductId::Ubiqx, Gate::new(os, vec![Tier::Hosts]))
            .capabilities();
        assert_eq!(partly.hosts, TierCapability::Available);
        assert_eq!(partly.resolver, by_table(Tier::Resolver));
        let labelled: Vec<Tier> = unverified_arg(os, &[Tier::Hosts])
            .map(|_| vec![Tier::Hosts])
            .unwrap_or_default();
        assert_eq!(partly.unverified, labelled);
        if let Some(tier) = closed_tier(os) {
            assert_eq!(*plain.of(tier), TierCapability::Gated, "{os:?} {tier:?}");
        }
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
    // Only the canary is asked as a name that is expected to be blocked: "no data" for
    // the probe name must never read as an answer.
    assert_eq!(*rig.names.asked_as_blocked.lock(), [hosts::CANARY_NAME]);
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
        // The override travels as an argument, naming what the table does not open.
        let mut apply = strings(&["apply", "--plan-b64", &encoded]);
        if let Some(tiers) = unverified_arg(os, &Tier::ALL) {
            apply.extend(strings(&["--unverified", &tiers]));
        }
        if closed_tier(os) == Some(Tier::Hosts) {
            assert_eq!(apply[3..], strings(&["--unverified", ALL_TIERS]));
        }
        assert_eq!(
            *rig.elevated.lock(),
            vec![
                apply,
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
    for os in Os::ALL {
        let rig = Rig::new(os);
        let open = rig.shield(ProductId::Ubiqx, all_open(os));
        // A tier the gate keeps closed: nobody is asked for a password for it.
        if let Some(tier) = closed_tier(os) {
            let by_table = Gate::new(os, Vec::new());
            let closed = rig.shield(ProductId::Ubiqx, by_table.clone());
            let plan = plan_for(ProductId::Ubiqx, tier);
            assert_eq!(
                closed.apply(&plan).unwrap(),
                ApplyOutcome::Unavailable("gated".into()),
                "{os:?} {tier:?}"
            );
            // What is left of the plan once the closed tier is taken out asks for
            // nothing: it is not sent either, and its preview shows no change.
            let nothing_left = plan::without_gated(&plan, &by_table);
            assert_eq!(
                closed.apply(&nothing_left).unwrap(),
                ApplyOutcome::Unavailable("gated".into())
            );
            assert_eq!(closed.preview(&plan).unwrap(), Vec::new());
            // With every tier open, a plan without a tier is the caller's mistake.
            assert!(matches!(
                open.apply(&nothing_left),
                Err(CoreError::Invalid(_))
            ));
        }
        // A plan with a name that cannot be blocked is an error, not a prompt.
        for name in ["localhost", "10.0.0.1", "portal.example.gov.br"] {
            let bad = plan_of(ProductId::Ubiqx, &[name]);
            assert!(
                matches!(open.apply(&bad), Err(CoreError::Invalid(_))),
                "{name}"
            );
        }
        assert!(rig.elevated.lock().is_empty(), "{os:?}");
        assert_eq!(rig.fs.mutations(), Vec::<String>::new(), "{os:?}");
    }
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
fn the_native_shield_of_this_machine_follows_the_gate_table() {
    // Reads the gate and the executable path only; nothing on the system is touched.
    let Some(os) = Os::current() else {
        assert!(NativeSystemShield::native(ProductId::Ubiqx).is_none());
        return;
    };
    let shield = NativeSystemShield::native(ProductId::Ubiqx).expect("this system has a back-end");
    let capabilities = shield.capabilities();
    assert_eq!(capabilities.os, Some(os));
    for tier in Tier::ALL {
        let opened = gate::tier_enabled(os, tier) || capabilities.unverified.contains(&tier);
        assert_eq!(
            *capabilities.of(tier) == TierCapability::Gated,
            !opened,
            "{tier:?}"
        );
    }
    // A plan for a tier that is gated is turned down before any elevation is attempted.
    // Only such a plan is ever handed to this shield: its elevator is the real one.
    let gated = Tier::ALL
        .into_iter()
        .find(|tier| *capabilities.of(*tier) == TierCapability::Gated);
    if let Some(tier) = gated {
        assert_eq!(
            shield.apply(&plan_for(ProductId::Ubiqx, tier)).unwrap(),
            ApplyOutcome::Unavailable("gated".into())
        );
    }
}
