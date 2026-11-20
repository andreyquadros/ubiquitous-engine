//! The resolver tier: set on what is active, undone only where it is still ours, and
//! never at the price of a machine without names.

use ubiqx_core::nofap::{ApplyOutcome, ObserveOpts, Os, SystemShield, Tier, TierReport, TierState};
use ubiqx_core::ProductId;
use ubiqx_nofap::hosts::{self, BlockInspection};
use ubiqx_nofap::plan::{EXIT_APPLIED, EXIT_PARTIAL};
use ubiqx_nofap::resolvers;

use crate::sysblock::testing::{all_open, cf_servers, plan_of, Rig, WinAdapter, CF};

/// Whether the pretend machine currently has the family resolver set.
fn resolver_set(rig: &Rig) -> bool {
    match rig.os {
        Os::Linux => rig.fs.text(resolvers::RESOLVED_DROPIN_PATH).is_some(),
        Os::Macos => rig
            .mac
            .services
            .lock()
            .iter()
            .any(|s| s.servers == cf_servers()),
        Os::Windows => rig
            .win
            .adapters
            .lock()
            .iter()
            .any(|a| a.fixed == cf_servers()),
    }
}

/// A machine on a network that blocks outside DNS: names resolve only while the family
/// resolver is not set.
fn blocking_network(os: Os) -> Rig {
    let rig = Rig::new(os);
    let machine = rig.clone();
    let mut blocked = Rig::with_names(os, move |_| {
        if resolver_set(&machine) {
            Vec::new()
        } else {
            vec!["203.0.113.10".to_owned()]
        }
    });
    // The lookup watches the machine the helper changes.
    (blocked.fs, blocked.mac, blocked.win, blocked.linux) = (rig.fs, rig.mac, rig.win, rig.linux);
    blocked
}

#[test]
fn a_resolver_that_breaks_name_resolution_is_undone_in_the_same_run() {
    for os in Os::ALL {
        let rig = blocking_network(os);
        let plan = plan_of(ProductId::Ubiqx, &[]);
        let shield = rig.shield(ProductId::Ubiqx, all_open(os));
        let outcome = shield.apply(&plan).unwrap();
        assert_eq!(
            outcome,
            ApplyOutcome::Partial(vec![TierReport {
                tier: Tier::Resolver,
                ok: false,
                reason: Some("unreachable".into()),
            }]),
            "{os:?}"
        );
        // One elevated run did all of it: there was no second prompt.
        assert_eq!(rig.elevated.lock().len(), 1);
        assert!(!resolver_set(&rig), "{os:?}: the change was undone");
        // It waited before giving up: eight tries, a second apart.
        assert_eq!(rig.pauses.lock().len(), 7, "{os:?}");
        assert!(rig.pauses.lock().iter().all(|pause| pause.as_secs() == 1));
        // The other tiers stayed applied.
        assert!(matches!(
            hosts::inspect(rig.hosts_text().as_bytes()),
            BlockInspection::Intact { .. }
        ));
        let judged = rig.assess(&plan);
        assert_eq!(judged.hosts, TierState::Applied, "{os:?}");
        assert_eq!(judged.policies, TierState::Applied, "{os:?}");
        // Not applied, and not drift: the state file does not claim the resolver.
        assert_eq!(judged.resolver, TierState::NotApplied, "{os:?}");
        let state = rig.observe().state.unwrap();
        assert_eq!(state.owners[&ProductId::Ubiqx].resolver, None);
        assert!(state.owners[&ProductId::Ubiqx].hosts);
        // Names resolve again.
        assert_eq!(rig.observe().probe_ok, Some(true));
    }
}

#[test]
fn an_offline_machine_gets_the_resolver_and_no_roll_back() {
    for os in Os::ALL {
        // Nothing resolves before the change either, so the change broke nothing.
        let rig = Rig::with_names(os, |_| Vec::new());
        let plan = plan_of(ProductId::Ubiqx, &[]);
        assert_eq!(rig.apply(&plan), EXIT_APPLIED, "{os:?}");
        assert!(resolver_set(&rig), "{os:?}");
        assert!(
            rig.pauses.lock().is_empty(),
            "no waiting for a probe that never worked"
        );
        assert_eq!(rig.assess(&plan).resolver, TierState::Applied, "{os:?}");
    }
}

#[test]
fn a_probe_that_recovers_within_the_wait_keeps_the_resolver() {
    let rig = Rig::new(Os::Linux);
    let machine = rig.clone();
    let tries = std::sync::Arc::new(parking_lot::Mutex::new(0u32));
    let counted = tries.clone();
    let mut slow = Rig::with_names(Os::Linux, move |_| {
        if !resolver_set(&machine) {
            return vec!["203.0.113.10".to_owned()];
        }
        // The service takes a moment after its restart: the third try answers.
        let mut tries = counted.lock();
        *tries += 1;
        if *tries >= 3 {
            vec!["203.0.113.10".to_owned()]
        } else {
            Vec::new()
        }
    });
    (slow.fs, slow.linux) = (rig.fs, rig.linux);
    assert_eq!(slow.apply(&plan_of(ProductId::Ubiqx, &[])), EXIT_APPLIED);
    assert!(resolver_set(&slow));
    assert_eq!(slow.pauses.lock().len(), 2);
    assert_eq!(*slow.linux.restarts.lock(), 1, "no roll-back restart");
}

#[test]
fn linux_writes_the_drop_in_and_restarts_the_service_once() {
    let rig = Rig::new(Os::Linux);
    let plan = plan_of(ProductId::Ubiqx, &[]);
    assert_eq!(rig.apply(&plan), EXIT_APPLIED);
    assert_eq!(
        rig.fs.text("/etc/systemd/resolved.conf.d/90-ubiqx-shield.conf").unwrap(),
        "# ubiqx-shield: delete this file and restart systemd-resolved to undo\n[Resolve]\nDNS=1.1.1.3 1.0.0.3 2606:4700:4700::1113 2606:4700:4700::1003\nDomains=~.\n"
    );
    assert_eq!(*rig.linux.restarts.lock(), 1);
    // Applying again changes nothing and restarts nothing.
    assert_eq!(rig.apply(&plan), EXIT_APPLIED);
    assert_eq!(*rig.linux.restarts.lock(), 1);
    // The user edited the drop-in: it reads as changed, and a removal still deletes it.
    rig.fs
        .file(resolvers::RESOLVED_DROPIN_PATH, b"[Resolve]\nDNS=9.9.9.9\n");
    assert_eq!(rig.assess(&plan).resolver, TierState::Changed);
    assert_eq!(rig.remove("ubiqx", "resolver"), EXIT_APPLIED);
    assert_eq!(rig.fs.node(resolvers::RESOLVED_DROPIN_PATH), None);
    assert_eq!(*rig.linux.restarts.lock(), 2);
}

#[test]
fn linux_without_resolved_in_use_is_unsupported_and_nothing_is_written() {
    for case in ["inactive", "resolv.conf elsewhere", "no systemctl"] {
        let rig = Rig::new(Os::Linux);
        match case {
            "inactive" => *rig.linux.resolved_active.lock() = false,
            "resolv.conf elsewhere" => rig.fs.file("/etc/resolv.conf", b"nameserver 192.168.0.1\n"),
            _ => {
                rig.linux
                    .missing
                    .lock()
                    .insert("/usr/bin/systemctl".to_owned());
            }
        }
        let plan = plan_of(ProductId::Ubiqx, &[]);
        assert_eq!(
            rig.assess(&plan).resolver,
            TierState::Unsupported {
                reason: "no_resolved".into()
            },
            "{case}"
        );
        assert_eq!(rig.apply(&plan), EXIT_PARTIAL, "{case}");
        assert_eq!(rig.fs.node(resolvers::RESOLVED_DROPIN_PATH), None, "{case}");
        assert_eq!(rig.fs.node("/etc/systemd"), None, "{case}");
        assert_eq!(*rig.linux.restarts.lock(), 0, "{case}");
        // The other tiers were applied all the same.
        assert_eq!(rig.assess(&plan).hosts, TierState::Applied, "{case}");
    }
}

#[test]
fn macos_sets_enabled_services_and_resets_only_those_still_ours() {
    let rig = Rig::new(Os::Macos);
    let plan = plan_of(ProductId::Ubiqx, &[]);
    assert_eq!(rig.apply(&plan), EXIT_APPLIED);
    let servers = |rig: &Rig, name: &str| {
        let services = rig.mac.services.lock();
        services
            .iter()
            .find(|s| s.name == name)
            .unwrap()
            .servers
            .clone()
    };
    assert_eq!(servers(&rig, "Wi-Fi"), cf_servers());
    assert_eq!(servers(&rig, "USB 10/100/1000 LAN"), cf_servers());
    assert!(
        servers(&rig, "Thunderbolt Bridge").is_empty(),
        "a disabled service is left alone"
    );
    assert_eq!(rig.assess(&plan).resolver, TierState::Applied);

    // The user points Wi-Fi somewhere else afterwards.
    rig.mac.services.lock()[1].servers = vec!["9.9.9.9".into()];
    assert_eq!(rig.assess(&plan).resolver, TierState::Changed);
    assert_eq!(rig.remove("ubiqx", "resolver"), EXIT_APPLIED);
    assert_eq!(
        servers(&rig, "Wi-Fi"),
        ["9.9.9.9"],
        "what the user set is not touched"
    );
    assert!(servers(&rig, "USB 10/100/1000 LAN").is_empty());
}

#[test]
fn macos_a_manual_dns_is_replaced_and_not_put_back() {
    let rig = Rig::new(Os::Macos);
    rig.mac.services.lock()[1].servers = vec!["192.168.0.2".into()];
    let plan = plan_of(ProductId::Ubiqx, &[]);
    // The preview names it before anything is changed.
    let shield = rig.shield(ProductId::Ubiqx, all_open(Os::Macos));
    let lines = shield.preview(&plan).unwrap();
    let previous = lines
        .iter()
        .find(|line| line.text_key == "dns.previous")
        .unwrap();
    assert_eq!(previous.detail.as_deref(), Some("192.168.0.2"));
    rig.apply(&plan);
    rig.remove("ubiqx", "all");
    // Undo never writes a recorded value: the service is back on automatic.
    assert!(rig.mac.services.lock()[1].servers.is_empty());
    assert!(!rig
        .calls()
        .iter()
        .any(|call| call.line().contains("192.168.0.2")));
}

#[test]
fn windows_sets_adapters_that_are_up_and_resets_only_those_still_ours() {
    let rig = Rig::new(Os::Windows);
    let plan = plan_of(ProductId::Ubiqx, &[]);
    assert_eq!(rig.apply(&plan), EXIT_APPLIED);
    assert_eq!(rig.win.adapters.lock()[0].fixed, cf_servers());
    assert!(
        rig.win.adapters.lock()[1].fixed.is_empty(),
        "an adapter that is down is left alone"
    );
    assert_eq!(rig.assess(&plan).resolver, TierState::Applied);

    // A second adapter comes up afterwards with the network's own DNS.
    rig.win.adapters.lock()[1].up = true;
    rig.win.adapters.lock()[1].automatic = vec!["10.0.0.1".into()];
    let judged = rig.assess(&plan);
    assert_eq!(judged.resolver, TierState::Changed);
    assert!(judged
        .issues
        .contains(&ubiqx_core::nofap::Issue::NewConnectionUnfiltered));
    // The next apply covers it.
    assert_eq!(rig.apply(&plan), EXIT_APPLIED);
    assert_eq!(rig.assess(&plan).resolver, TierState::Applied);

    // The user changes one by hand; the removal resets the other only.
    rig.win.adapters.lock()[0].fixed = vec!["9.9.9.9".into()];
    rig.clear_calls();
    assert_eq!(rig.remove("ubiqx", "resolver"), EXIT_APPLIED);
    assert_eq!(rig.win.adapters.lock()[0].fixed, ["9.9.9.9"]);
    assert!(rig.win.adapters.lock()[1].fixed.is_empty());
    let resets: Vec<String> = rig
        .calls()
        .iter()
        .map(|call| call.line())
        .filter(|line| line.contains("-ResetServerAddresses"))
        .collect();
    assert_eq!(resets.len(), 1);
    assert!(resets[0].contains("foreach ($i in @(7))"), "{}", resets[0]);
}

/// The lines of the calls that set DNS servers on Windows adapters.
fn windows_sets(rig: &Rig) -> Vec<String> {
    rig.calls()
        .iter()
        .map(|call| call.line())
        .filter(|line| line.contains("-ServerAddresses ("))
        .collect()
}

/// What the first smoke run on a real Windows machine showed: a cloud runner whose third
/// adapter (the accelerated-networking twin of the second) is up and unknown to the DNS
/// client. Asking for its servers fails, and so does setting them. Such an adapter is not
/// read, not set and not reset, and it does not stop the tier for the others.
#[test]
fn windows_leaves_out_an_adapter_the_dns_client_has_no_configuration_for() {
    let rig = Rig::new(Os::Windows);
    rig.win.adapters.lock().push(WinAdapter {
        index: 9,
        guid: "{99999999-8888-7777-6666-555555555555}".into(),
        name: "Ethernet 4".into(),
        up: true,
        configured: false,
        fixed: Vec::new(),
        automatic: Vec::new(),
    });
    let plan = plan_of(ProductId::Ubiqx, &[]);
    assert_eq!(rig.apply(&plan), EXIT_APPLIED);
    let sets = windows_sets(&rig);
    assert_eq!(sets.len(), 1, "{sets:?}");
    assert!(sets[0].contains("foreach ($i in @(12))"), "{}", sets[0]);
    assert_eq!(rig.win.adapters.lock()[0].fixed, cf_servers());
    assert!(rig.win.adapters.lock()[2].fixed.is_empty());

    let observed = rig.observe();
    assert_eq!(observed.resolver.error, None);
    let ids: Vec<&str> = observed
        .resolver
        .interfaces
        .iter()
        .map(|interface| interface.id.as_str())
        .collect();
    assert_eq!(ids, ["12"], "only the adapter the DNS client knows");
    assert_eq!(rig.assess(&plan).resolver, TierState::Applied);

    rig.clear_calls();
    assert_eq!(rig.remove("ubiqx", "all"), EXIT_APPLIED);
    assert!(rig.win.adapters.lock()[0].fixed.is_empty());
    let resets: Vec<String> = rig
        .calls()
        .iter()
        .map(|call| call.line())
        .filter(|line| line.contains("-ResetServerAddresses"))
        .collect();
    assert_eq!(resets.len(), 1, "{resets:?}");
    assert!(resets[0].contains("foreach ($i in @(12))"), "{}", resets[0]);
}

/// The set script names its adapters, so with none to name it is not started at all: an
/// empty list would be a script that does nothing and says it worked.
#[test]
fn windows_with_no_adapter_to_set_starts_no_set_script() {
    let rig = Rig::new(Os::Windows);
    for adapter in rig.win.adapters.lock().iter_mut() {
        adapter.configured = false;
    }
    let plan = plan_of(ProductId::Ubiqx, &[]);
    assert_eq!(rig.apply(&plan), EXIT_APPLIED);
    assert_eq!(windows_sets(&rig), Vec::<String>::new());
    let observed = rig.observe();
    assert_eq!(observed.resolver.error, None);
    assert!(observed.resolver.interfaces.is_empty());
    assert!(rig.win.adapters.lock().iter().all(|a| a.fixed.is_empty()));

    // The adapter gets its configuration (the cable goes in) with the network's own
    // DNS: the judgement sees it, and the next apply covers it.
    rig.win.adapters.lock()[0].configured = true;
    assert_eq!(rig.assess(&plan).resolver, TierState::Changed);
    assert_eq!(rig.apply(&plan), EXIT_APPLIED);
    assert_eq!(windows_sets(&rig).len(), 1);
    assert_eq!(rig.win.adapters.lock()[0].fixed, cf_servers());
    assert_eq!(rig.assess(&plan).resolver, TierState::Applied);
}

#[test]
fn windows_quick_read_follows_the_adapters_of_the_last_full_read() {
    let rig = Rig::new(Os::Windows);
    let shield = rig.shield(ProductId::Ubiqx, all_open(Os::Windows));
    // No full read yet: the quick read has nothing to look at and says so.
    let quick = shield.observe(ObserveOpts::QUICK).unwrap().resolver;
    assert!(!quick.full);
    assert_eq!(quick.error.as_deref(), Some("unreadable"));
    assert!(quick.interfaces.is_empty());

    let full = shield.observe(ObserveOpts::FULL).unwrap().resolver;
    assert!(full.full && full.error.is_none());
    assert_eq!(
        full.interfaces[0].servers,
        ["192.168.0.1"],
        "what the network hands out"
    );

    rig.apply(&plan_of(ProductId::Ubiqx, &[]));
    rig.clear_calls();
    let quick = shield.observe(ObserveOpts::QUICK).unwrap().resolver;
    assert_eq!(quick.error, None);
    assert_eq!(quick.interfaces.len(), 1);
    assert_eq!(
        (
            quick.interfaces[0].id.as_str(),
            quick.interfaces[0].name.as_str()
        ),
        ("12", "Wi-Fi")
    );
    assert!(resolvers::is_exactly(CF, &quick.interfaces[0].servers));
    // The quick read is two registry queries and no PowerShell.
    let dns_reads: Vec<String> = rig
        .calls()
        .iter()
        .map(|call| call.line())
        .filter(|line| !line.contains("SOFTWARE\\Policies"))
        .collect();
    assert_eq!(
        dns_reads,
        [
            "C:\\Windows\\System32\\reg.exe query HKLM\\SYSTEM\\CurrentControlSet\\Services\\Tcpip\\Parameters\\Interfaces /s /v NameServer",
            "C:\\Windows\\System32\\reg.exe query HKLM\\SYSTEM\\CurrentControlSet\\Services\\Tcpip6\\Parameters\\Interfaces /s /v NameServer",
        ]
    );
    // Reset by hand: the quick read sees it.
    rig.win.adapters.lock()[0].fixed.clear();
    let quick = shield.observe(ObserveOpts::QUICK).unwrap().resolver;
    assert!(quick.interfaces[0].servers.is_empty());
}

#[test]
fn a_dns_read_that_fails_changes_no_dns() {
    let rig = Rig::new(Os::Macos);
    rig.mac
        .broken
        .lock()
        .insert("/usr/sbin/networksetup".to_owned());
    let seen = rig.observe().resolver;
    assert_eq!(seen.error.as_deref(), Some("unreadable"));
    let plan = plan_of(ProductId::Ubiqx, &[]);
    assert_eq!(rig.apply(&plan), EXIT_PARTIAL);
    assert!(!rig
        .calls()
        .iter()
        .any(|call| call.line().contains("-setdnsservers")));
}
