//! Whole apply → observe → remove cycles through the real argument parser, the real
//! back-ends and the real parsers, for each of the three systems.

use ubiqx_core::nofap::{Issue, Os, TierState};
use ubiqx_core::ProductId;
use ubiqx_nofap::hosts::{self, BlockInspection};
use ubiqx_nofap::plan::{EXIT_APPLIED, EXIT_PARTIAL};

use crate::sysblock::testing::{cf_servers, plan_of, Node, Rig, CF};

fn applied(rig: &Rig, plan: &ubiqx_core::nofap::SystemPlan) {
    let judged = rig.assess(plan);
    assert_eq!(judged.hosts, TierState::Applied, "{:?}", rig.os);
    assert_eq!(judged.resolver, TierState::Applied, "{:?}", rig.os);
    assert_eq!(judged.policies, TierState::Applied, "{:?}", rig.os);
    assert_eq!(judged.hosts_waiting, 0);
}

#[test]
fn apply_then_observe_is_applied_and_remove_restores_every_byte() {
    for os in Os::ALL {
        let rig = Rig::new(os);
        let files_before = rig.fs.files();
        let (mac_before, win_before) = (
            rig.mac.services.lock().clone(),
            rig.win.adapters.lock().clone(),
        );
        let plan = plan_of(ProductId::Ubiqx, &["blocked.example"]);

        let before = rig.assess(&plan);
        assert_eq!(before.hosts, TierState::NotApplied, "{os:?}");
        assert_eq!(before.resolver, TierState::NotApplied, "{os:?}");
        assert_eq!(before.policies, TierState::NotApplied, "{os:?}");

        assert_eq!(rig.apply(&plan), EXIT_APPLIED, "{os:?}");
        applied(&rig, &plan);
        let text = rig.hosts_text();
        assert!(matches!(
            hosts::inspect(text.as_bytes()),
            BlockInspection::Intact { .. }
        ));
        let names = hosts::block_names(text.as_bytes());
        assert_eq!(
            names[..3],
            [
                "canary.ubiqx-shield.invalid",
                "blocked.example",
                "www.blocked.example"
            ]
        );
        assert_eq!(names.len(), hosts::cap_for(os), "{os:?}");
        // What was there before the block is still there, byte for byte.
        let original = files_before[&rig.hosts_path()].clone();
        let Node::File { bytes, .. } = original else {
            panic!("the hosts file is a file")
        };
        assert!(text.as_bytes().starts_with(&bytes), "{os:?}");
        if os == Os::Windows {
            assert!(!text.replace("\r\n", "").contains('\n'), "CRLF throughout");
        }
        // The state file records what was applied.
        let state = rig.observe().state.expect("a state file");
        assert_eq!(state.owners.len(), 1);
        assert_eq!(state.owners[&ProductId::Ubiqx].extra, ["blocked.example"]);
        assert_eq!(state.hosts.unwrap().names as usize, names.len());
        assert!(state.report.iter().all(|report| report.ok));
        assert_eq!(state.applied_at, 1_795_100_000_000);

        // A second apply of the same plan changes nothing but the state file, and runs
        // no command that changes the system.
        rig.fs.clear_log();
        rig.clear_calls();
        assert_eq!(rig.apply(&plan), EXIT_APPLIED);
        let rewritten: Vec<String> = rig
            .fs
            .mutations()
            .into_iter()
            .filter(|change| !change.ends_with("state.json"))
            .collect();
        assert_eq!(rewritten, Vec::<String>::new(), "{os:?}");
        for call in rig.calls() {
            let line = call.line();
            let reads = [
                "-listallnetworkservices",
                "-getdnsservers",
                " read ",
                " query ",
                "is-active",
            ];
            let full_read = line.ends_with(crate::sysblock::windows::FULL_READ_SCRIPT);
            assert!(
                full_read || reads.iter().any(|read| line.contains(read)),
                "{os:?}: {line}"
            );
        }

        assert_eq!(rig.remove("ubiqx", "all"), EXIT_APPLIED, "{os:?}");
        assert_eq!(rig.fs.files(), files_before, "{os:?}");
        assert_eq!(*rig.mac.services.lock(), mac_before);
        assert_eq!(*rig.win.adapters.lock(), win_before);
        assert!(rig.mac.firefox.lock().is_empty());
        assert!(rig.win.registry.lock().is_empty());
        assert!(!rig.fs.snapshot().contains_key(&rig.state_file()));
        let after = rig.assess(&plan);
        assert_eq!(after.hosts, TierState::NotApplied, "{os:?}");
        assert_eq!(after.resolver, TierState::NotApplied, "{os:?}");
        assert_eq!(after.policies, TierState::NotApplied, "{os:?}");
        assert!(rig.observe().state.is_none());
    }
}

#[test]
fn two_owners_share_the_tiers_until_the_last_one_leaves() {
    for os in Os::ALL {
        let rig = Rig::new(os);
        let files_before = rig.fs.files();
        let ubiqx = plan_of(ProductId::Ubiqx, &["first.example"]);
        let mut shield = plan_of(ProductId::Shield, &["second.example"]);
        shield.policies = false;
        assert_eq!(rig.apply(&ubiqx), EXIT_APPLIED);
        assert_eq!(rig.apply(&shield), EXIT_APPLIED);
        let names = hosts::block_names(rig.hosts_text().as_bytes());
        assert!(names.contains(&"first.example".to_owned()));
        assert!(names.contains(&"www.second.example".to_owned()));
        applied(&rig, &ubiqx);

        // ubiqX leaves: the hosts block and the resolver stay for the other product,
        // without ubiqX's names; the policies only ubiqX wanted go.
        assert_eq!(rig.remove("ubiqx", "all"), EXIT_APPLIED, "{os:?}");
        let names = hosts::block_names(rig.hosts_text().as_bytes());
        assert!(!names.contains(&"first.example".to_owned()), "{os:?}");
        assert!(names.contains(&"second.example".to_owned()));
        let judged = rig.assess(&shield);
        assert_eq!(judged.hosts, TierState::Applied, "{os:?}");
        assert_eq!(judged.resolver, TierState::Applied, "{os:?}");
        assert_eq!(judged.policies, TierState::Off, "{os:?}");
        let state = rig.observe().state.expect("the other owner's state");
        assert_eq!(
            state.owners.keys().copied().collect::<Vec<_>>(),
            [ProductId::Shield]
        );
        assert!(state.policies_created.is_empty());
        // For ubiqX the tiers are now somebody else's.
        let seen_by_ubiqx = rig.assess(&ubiqx);
        assert_eq!(seen_by_ubiqx.hosts, TierState::InUseByOther, "{os:?}");

        // The second owner leaves: everything is gone, the state file and its directory
        // included.
        assert_eq!(rig.remove("shield", "all"), EXIT_APPLIED);
        assert_eq!(rig.fs.files(), files_before, "{os:?}");
        let state_dir = crate::sysblock::testing::backend_for(os).state_dir();
        assert!(!rig
            .fs
            .snapshot()
            .contains_key(&*state_dir.to_string_lossy()));
    }
}

#[test]
fn remove_owner_all_clears_both_owners_at_once() {
    for os in Os::ALL {
        let rig = Rig::new(os);
        let files_before = rig.fs.files();
        rig.apply(&plan_of(ProductId::Ubiqx, &[]));
        rig.apply(&plan_of(ProductId::Shield, &[]));
        assert_eq!(rig.remove("all", "all"), EXIT_APPLIED);
        assert_eq!(rig.fs.files(), files_before, "{os:?}");
        // And again, with nothing left to remove: safe, and still nothing written.
        rig.fs.clear_log();
        rig.clear_calls();
        assert_eq!(rig.remove("all", "all"), EXIT_APPLIED);
        assert_eq!(rig.fs.mutations(), Vec::<String>::new(), "{os:?}");
    }
}

#[test]
fn remove_scope_resolver_leaves_the_hosts_block_and_the_policies() {
    for os in Os::ALL {
        let rig = Rig::new(os);
        let plan = plan_of(ProductId::Shield, &[]);
        rig.apply(&plan);
        assert_eq!(rig.remove("shield", "resolver"), EXIT_APPLIED, "{os:?}");
        let observed = rig.observe();
        assert!(observed.resolver.dropin.is_none());
        assert!(observed
            .resolver
            .interfaces
            .iter()
            .all(|interface| interface.servers != cf_servers()));
        let wants = &observed.state.unwrap().owners[&ProductId::Shield];
        assert!(wants.hosts && wants.policies && wants.resolver.is_none());
        // What the app then shows: the resolver is off for this product, the rest stays.
        let mut restored = plan.clone();
        restored.resolver = None;
        let judged = rig.assess(&restored);
        assert_eq!(judged.hosts, TierState::Applied, "{os:?}");
        assert_eq!(judged.policies, TierState::Applied, "{os:?}");
        assert_eq!(judged.resolver, TierState::Off, "{os:?}");
    }
}

#[test]
fn remove_after_a_partial_apply_cleans_up_what_was_applied() {
    for os in Os::ALL {
        let rig = Rig::new(os);
        let files_before = rig.fs.files();
        // The resolver commands fail; the other two tiers go through.
        match os {
            Os::Macos => rig
                .mac
                .broken
                .lock()
                .insert("/usr/sbin/networksetup".to_owned()),
            Os::Linux => {
                *rig.linux.resolved_active.lock() = false;
                true
            }
            Os::Windows => {
                rig.win.adapters.lock().clear();
                rig.win.broken.lock().insert(format!(
                    "{}\\WindowsPowerShell\\v1.0\\powershell.exe",
                    crate::sysblock::testing::SYSTEM32
                ))
            }
        };
        let plan = plan_of(ProductId::Ubiqx, &[]);
        assert_eq!(rig.apply(&plan), EXIT_PARTIAL, "{os:?}");
        let state = rig.observe().state.expect("a report in the state file");
        let failed: Vec<_> = state.report.iter().filter(|report| !report.ok).collect();
        assert_eq!(failed.len(), 1, "{os:?}: {:?}", state.report);
        assert_eq!(failed[0].tier, ubiqx_core::nofap::Tier::Resolver);
        let expected = if os == Os::Linux {
            "no_resolved"
        } else {
            "unreadable"
        };
        assert_eq!(failed[0].reason.as_deref(), Some(expected));
        // A tier that failed on its first try is not recorded as wanted.
        assert_eq!(state.owners[&ProductId::Ubiqx].resolver, None);
        assert!(state.owners[&ProductId::Ubiqx].hosts);
        assert!(hosts::inspect(rig.hosts_text().as_bytes()) != BlockInspection::Absent);

        rig.mac.broken.lock().clear();
        rig.win.broken.lock().clear();
        assert_eq!(rig.remove("ubiqx", "all"), EXIT_APPLIED, "{os:?}");
        assert_eq!(rig.fs.files(), files_before, "{os:?}");
    }
}

#[test]
fn a_block_edited_by_hand_reads_changed_and_the_next_apply_repairs_it() {
    for os in Os::ALL {
        let rig = Rig::new(os);
        let plan = plan_of(ProductId::Ubiqx, &[]);
        rig.apply(&plan);
        let damaged = rig
            .hosts_text()
            .replacen("0.0.0.0 canary", "0.0.0.0 kanary", 1);
        rig.fs.file(&rig.hosts_path(), damaged.as_bytes());
        assert_eq!(rig.assess(&plan).hosts, TierState::Changed, "{os:?}");
        assert_eq!(rig.apply(&plan), EXIT_APPLIED);
        assert_eq!(rig.assess(&plan).hosts, TierState::Applied, "{os:?}");
        // Deleted by hand: missing, and a removal afterwards is still clean.
        let without = hosts::splice(rig.hosts_text().as_bytes(), None);
        rig.fs.file(&rig.hosts_path(), &without);
        assert_eq!(rig.assess(&plan).hosts, TierState::Missing, "{os:?}");
        assert_eq!(rig.remove("ubiqx", "all"), EXIT_APPLIED);
    }
}

#[test]
fn the_canary_is_judged_from_the_lookup_the_observation_was_asked_for() {
    let blocked = Rig::with_names(Os::Linux, |name| {
        if name == hosts::CANARY_NAME {
            vec!["0.0.0.0".to_owned()]
        } else {
            vec!["203.0.113.10".to_owned()]
        }
    });
    let plan = plan_of(ProductId::Ubiqx, &[]);
    blocked.apply(&plan);
    assert!(!blocked
        .assess(&plan)
        .issues
        .contains(&Issue::HostsNotHonoured));
    // A machine whose resolver ignores the hosts file answers something else.
    let ignored = Rig::new(Os::Linux);
    ignored.apply(&plan);
    assert!(ignored
        .assess(&plan)
        .issues
        .contains(&Issue::HostsNotHonoured));
}

#[test]
fn a_run_writes_only_the_tiers_its_own_gate_opens() {
    for os in Os::ALL {
        let rig = Rig::new(os);
        // ubiqX applied all three tiers with a tester's override.
        let ubiqx = plan_of(ProductId::Ubiqx, &["first.example"]);
        rig.apply(&ubiqx);
        // The user takes the resolver off by hand afterwards.
        match os {
            Os::Linux => rig
                .fs
                .file(ubiqx_nofap::resolvers::RESOLVED_DROPIN_PATH, b"[Resolve]\n"),
            Os::Macos => rig
                .mac
                .services
                .lock()
                .iter_mut()
                .for_each(|s| s.servers.clear()),
            Os::Windows => rig
                .win
                .adapters
                .lock()
                .iter_mut()
                .for_each(|a| a.fixed.clear()),
        }
        // The standalone app then applies policies only, and its run opens nothing else.
        let mut shield = plan_of(ProductId::Shield, &[]);
        shield.hosts = false;
        shield.resolver = None;
        let encoded = ubiqx_nofap::plan::encode_plan(&shield);
        rig.clear_calls();
        let (exit, _, err) =
            rig.helper(&["apply", "--plan-b64", &encoded, "--unverified", "policies"]);
        assert_eq!(exit, EXIT_APPLIED, "{os:?}: {err}");
        // The resolver ubiqX still wants is not put back by a run whose gate is closed
        // for it; the hosts block that was there is kept, with ubiqX's names.
        let resolver_closed =
            !ubiqx_nofap::gate::tier_enabled(os, ubiqx_core::nofap::Tier::Resolver);
        for call in rig.calls().into_iter().filter(|_| resolver_closed) {
            let line = call.line();
            assert!(
                !line.contains("-setdnsservers")
                    && !line.contains("Set-DnsClient")
                    && !line.contains("restart"),
                "{os:?}: {line}"
            );
        }
        let names = hosts::block_names(rig.hosts_text().as_bytes());
        assert!(names.contains(&"first.example".to_owned()), "{os:?}");
        let state = rig.observe().state.unwrap();
        assert_eq!(state.owners.len(), 2);
        assert_eq!(
            state.owners[&ProductId::Ubiqx].resolver,
            Some(CF),
            "the other owner's wants are kept"
        );
        // A tier nobody wants any more comes off whatever the gate says.
        assert_eq!(rig.remove("all", "all"), EXIT_APPLIED);
        assert!(!rig.hosts_text().contains(hosts::BEGIN_PREFIX), "{os:?}");
    }
}

#[test]
fn an_owner_that_asks_for_nothing_is_not_kept_in_the_state() {
    for os in Os::ALL {
        let rig = Rig::new(os);
        let files_before = rig.fs.files();
        rig.apply(&plan_of(ProductId::Ubiqx, &[]));
        // The same owner applies a plan without any tier: that is a removal.
        let mut nothing = plan_of(ProductId::Ubiqx, &[]);
        nothing.hosts = false;
        nothing.resolver = None;
        nothing.policies = false;
        assert_eq!(rig.apply(&nothing), EXIT_APPLIED, "{os:?}");
        assert_eq!(rig.fs.files(), files_before, "{os:?}");
        assert!(rig.observe().state.is_none(), "{os:?}");
    }
}

#[test]
fn restoring_the_network_takes_the_resolver_off_for_both_owners() {
    // "Restaurar minha rede" fixes a broken network: DNS goes back to automatic even when
    // the other product also had the family resolver, and that product then sees its
    // resolver as not applied (its "Religar"), not as somebody else's.
    for os in Os::ALL {
        let rig = Rig::new(os);
        let ubiqx = plan_of(ProductId::Ubiqx, &[]);
        let shield = plan_of(ProductId::Shield, &[]);
        assert_eq!(rig.apply(&ubiqx), EXIT_APPLIED);
        assert_eq!(rig.apply(&shield), EXIT_APPLIED);
        assert_eq!(rig.remove("ubiqx", "resolver"), EXIT_APPLIED, "{os:?}");
        let observed = rig.observe();
        assert!(observed.resolver.dropin.is_none(), "{os:?}");
        assert!(
            observed
                .resolver
                .interfaces
                .iter()
                .all(|interface| interface.servers != cf_servers()),
            "{os:?}: the provider's servers are still set"
        );
        let state = observed.state.expect("both owners stay");
        for owner in [ProductId::Ubiqx, ProductId::Shield] {
            let wants = &state.owners[&owner];
            assert!(wants.hosts && wants.policies, "{os:?} {owner:?}");
            assert!(wants.resolver.is_none(), "{os:?} {owner:?}");
        }
        let seen_by_shield = rig.assess(&shield);
        assert_eq!(seen_by_shield.resolver, TierState::NotApplied, "{os:?}");
        assert_eq!(seen_by_shield.hosts, TierState::Applied, "{os:?}");
    }
}
