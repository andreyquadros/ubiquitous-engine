//! Half-done runs: a command that fails in the middle of a tier, and a helper that is
//! killed between two steps. Whatever is left on the machine, the state file knows about
//! it and `remove` takes it off.

use std::panic::{catch_unwind, AssertUnwindSafe};

use ubiqx_core::nofap::{BrowserFamily, Os, PolicyCreated, SystemPlan, Tier, TierState};
use ubiqx_core::ProductId;
use ubiqx_nofap::plan::{self, EXIT_APPLIED, EXIT_PARTIAL};
use ubiqx_nofap::resolvers;

use crate::sysblock::testing::{cf_servers, plan_for, Rig, ALL_TIERS};

const CHROME_KEY: &str = "HKLM\\SOFTWARE\\Policies\\Google\\Chrome";
const FIREFOX_KEY: &str = "HKLM\\SOFTWARE\\Policies\\Mozilla\\Firefox\\DNSOverHTTPS";

fn made(browser: BrowserFamily, key: &str) -> PolicyCreated {
    PolicyCreated {
        browser,
        key: key.to_owned(),
    }
}

fn policies(owner: ProductId) -> SystemPlan {
    plan_for(owner, Tier::Policies)
}

fn resolver(owner: ProductId) -> SystemPlan {
    plan_for(owner, Tier::Resolver)
}

/// Runs `apply` in a helper that a planted fault kills half-way.
fn apply_until_killed(rig: &Rig, plan: &SystemPlan) {
    let encoded = plan::encode_plan(plan);
    let killed = catch_unwind(AssertUnwindSafe(|| {
        rig.helper(&["apply", "--plan-b64", &encoded, "--unverified", ALL_TIERS])
    }));
    assert!(killed.is_err(), "the planted fault ends the run");
    rig.faults().clear();
}

fn registry_names(rig: &Rig) -> Vec<(String, String)> {
    rig.win.registry.lock().keys().cloned().collect()
}

#[test]
fn windows_policy_values_written_before_a_failure_are_recorded_and_undone() {
    let rig = Rig::new(Os::Windows);
    // Chrome's two values are written; Firefox's first one is refused.
    rig.faults().fail("add HKLM\\SOFTWARE\\Policies\\Mozilla");
    assert_eq!(rig.apply(&policies(ProductId::Ubiqx)), EXIT_PARTIAL);
    assert_eq!(
        registry_names(&rig),
        vec![
            (CHROME_KEY.to_owned(), "DnsOverHttpsMode".to_owned()),
            (CHROME_KEY.to_owned(), "ForceGoogleSafeSearch".to_owned()),
        ]
    );
    let state = rig.observe().state.expect("a failed run leaves its report");
    assert!(state
        .policies_created
        .contains(&made(BrowserFamily::Chrome, "DnsOverHttpsMode")));
    assert!(state
        .policies_created
        .contains(&made(BrowserFamily::Chrome, "ForceGoogleSafeSearch")));
    let failed: Vec<Tier> = state
        .report
        .iter()
        .filter(|report| !report.ok)
        .map(|report| report.tier)
        .collect();
    assert_eq!(failed, vec![Tier::Policies]);
    // The way out takes off what the failed run wrote.
    rig.faults().clear();
    assert_eq!(rig.remove("ubiqx", "all"), EXIT_APPLIED);
    assert_eq!(registry_names(&rig), vec![]);
    assert_eq!(rig.fs.node(&rig.state_file()), None);
}

#[test]
fn windows_a_helper_killed_between_two_policy_values_leaves_nothing_it_cannot_undo() {
    let rig = Rig::new(Os::Windows);
    rig.faults()
        .die_at("add HKLM\\SOFTWARE\\Policies\\Mozilla\\Firefox\\DNSOverHTTPS /v Locked");
    apply_until_killed(&rig, &policies(ProductId::Ubiqx));
    // Three values are in the registry and the run never reached its last record.
    assert_eq!(
        registry_names(&rig),
        vec![
            (CHROME_KEY.to_owned(), "DnsOverHttpsMode".to_owned()),
            (CHROME_KEY.to_owned(), "ForceGoogleSafeSearch".to_owned()),
            (FIREFOX_KEY.to_owned(), "Enabled".to_owned()),
        ]
    );
    assert_eq!(rig.remove("ubiqx", "all"), EXIT_APPLIED);
    assert_eq!(registry_names(&rig), vec![]);
    assert_eq!(rig.fs.node(&rig.state_file()), None);
}

#[test]
fn windows_a_value_that_was_there_before_is_never_listed_even_when_the_run_dies() {
    let rig = Rig::new(Os::Windows);
    // Somebody else's value, equal to ours: not created by this helper, never deleted.
    rig.win.registry.lock().insert(
        (CHROME_KEY.to_owned(), "ForceGoogleSafeSearch".to_owned()),
        ("REG_DWORD".to_owned(), "0x1".to_owned()),
    );
    rig.faults()
        .die_at("add HKLM\\SOFTWARE\\Policies\\Mozilla\\Firefox\\DNSOverHTTPS /v Enabled");
    apply_until_killed(&rig, &policies(ProductId::Ubiqx));
    assert_eq!(rig.remove("ubiqx", "all"), EXIT_APPLIED);
    assert_eq!(
        registry_names(&rig),
        vec![(CHROME_KEY.to_owned(), "ForceGoogleSafeSearch".to_owned())]
    );
}

#[test]
fn macos_policy_values_written_before_a_failure_or_a_kill_are_undone() {
    for killed in [false, true] {
        let rig = Rig::new(Os::Macos);
        let plan = policies(ProductId::Ubiqx);
        if killed {
            rig.faults().die_at("DNSOverHTTPS__Locked -bool");
            apply_until_killed(&rig, &plan);
        } else {
            rig.faults().fail("DNSOverHTTPS__Locked -bool");
            assert_eq!(rig.apply(&plan), EXIT_PARTIAL);
            rig.faults().clear();
        }
        let written: Vec<String> = rig.mac.firefox.lock().keys().cloned().collect();
        assert_eq!(
            written,
            vec!["DNSOverHTTPS__Enabled", "EnterprisePoliciesEnabled"],
            "killed: {killed}"
        );
        assert_eq!(rig.remove("ubiqx", "all"), EXIT_APPLIED, "killed: {killed}");
        assert!(rig.mac.firefox.lock().is_empty(), "killed: {killed}");
        assert_eq!(rig.fs.node(&rig.state_file()), None, "killed: {killed}");
    }
}

#[test]
fn a_policy_value_of_ours_that_somebody_changed_is_put_back_by_the_next_apply() {
    // Windows: a registry value this helper created.
    let rig = Rig::new(Os::Windows);
    let plan = policies(ProductId::Ubiqx);
    assert_eq!(rig.apply(&plan), EXIT_APPLIED);
    rig.win.registry.lock().insert(
        (CHROME_KEY.to_owned(), "DnsOverHttpsMode".to_owned()),
        ("REG_SZ".to_owned(), "secure".to_owned()),
    );
    assert_eq!(rig.assess(&plan).policies, TierState::Changed);
    assert_eq!(rig.apply(&plan), EXIT_APPLIED);
    assert_eq!(
        rig.win
            .registry
            .lock()
            .get(&(CHROME_KEY.to_owned(), "DnsOverHttpsMode".to_owned()))
            .map(|(_, data)| data.clone())
            .as_deref(),
        Some("off")
    );
    assert_eq!(rig.assess(&plan).policies, TierState::Applied);

    // macOS: a preference this helper created.
    let rig = Rig::new(Os::Macos);
    assert_eq!(rig.apply(&plan), EXIT_APPLIED);
    rig.mac
        .firefox
        .lock()
        .insert("DNSOverHTTPS__Enabled".to_owned(), "1".to_owned());
    assert_eq!(rig.assess(&plan).policies, TierState::Changed);
    assert_eq!(rig.apply(&plan), EXIT_APPLIED);
    assert_eq!(
        rig.mac
            .firefox
            .lock()
            .get("DNSOverHTTPS__Enabled")
            .map(String::as_str),
        Some("0")
    );
    assert_eq!(rig.assess(&plan).policies, TierState::Applied);
}

#[test]
fn macos_a_resolver_that_fails_on_one_service_is_taken_off_the_others() {
    let rig = Rig::new(Os::Macos);
    // The user's own choice on a service that is switched off: never touched.
    rig.mac.services.lock()[2].servers = cf_servers();
    // The first enabled service takes the resolver, the second refuses it.
    rig.faults().fail("-setdnsservers Wi-Fi 1.1.1.3");
    let plan = resolver(ProductId::Ubiqx);
    assert_eq!(rig.apply(&plan), EXIT_PARTIAL);
    let servers: Vec<Vec<String>> = rig
        .mac
        .services
        .lock()
        .iter()
        .map(|service| service.servers.clone())
        .collect();
    assert_eq!(
        servers,
        vec![Vec::new(), Vec::new(), cf_servers()],
        "the service that was set is automatic again"
    );
    let state = rig.observe().state.expect("the report of the failed run");
    assert!(state.owners.is_empty(), "nobody has the resolver recorded");
    assert_eq!(rig.assess(&plan).resolver, TierState::NotApplied);
}

#[test]
fn a_resolver_another_owner_already_has_is_kept_when_an_update_fails() {
    let rig = Rig::new(Os::Macos);
    assert_eq!(rig.apply(&resolver(ProductId::Shield)), EXIT_APPLIED);
    // A new service appears, and it refuses the resolver.
    rig.mac
        .services
        .lock()
        .push(crate::sysblock::testing::Service {
            name: "Odd VPN".into(),
            enabled: true,
            servers: Vec::new(),
        });
    rig.faults().fail("-setdnsservers Odd VPN 1.1.1.3");
    assert_eq!(rig.apply(&resolver(ProductId::Ubiqx)), EXIT_PARTIAL);
    let services = rig.mac.services.lock().clone();
    assert_eq!(services[0].servers, cf_servers());
    assert_eq!(services[1].servers, cf_servers());
    let state = rig.observe().state.unwrap();
    assert_eq!(
        state.owners.keys().copied().collect::<Vec<_>>(),
        vec![ProductId::Shield],
        "the owner whose first try failed is not recorded"
    );
}

#[test]
fn linux_a_drop_in_whose_restart_fails_is_not_left_behind() {
    let rig = Rig::new(Os::Linux);
    rig.faults().fail("restart systemd-resolved");
    let plan = resolver(ProductId::Ubiqx);
    assert_eq!(rig.apply(&plan), EXIT_PARTIAL);
    assert_eq!(
        rig.fs.node(resolvers::RESOLVED_DROPIN_PATH),
        None,
        "a file the running service never loaded would take effect at the next boot"
    );
    assert_eq!(rig.assess(&plan).resolver, TierState::NotApplied);
}

#[test]
fn linux_removing_the_drop_in_never_starts_a_resolved_that_is_not_running() {
    let rig = Rig::new(Os::Linux);
    assert_eq!(rig.apply(&resolver(ProductId::Ubiqx)), EXIT_APPLIED);
    assert_eq!(*rig.linux.restarts.lock(), 1);
    // The user switched systemd-resolved off since.
    *rig.linux.resolved_active.lock() = false;
    assert_eq!(rig.remove("ubiqx", "all"), EXIT_APPLIED);
    assert_eq!(rig.fs.node(resolvers::RESOLVED_DROPIN_PATH), None);
    assert_eq!(
        *rig.linux.restarts.lock(),
        1,
        "`systemctl restart` would start the service"
    );
}

#[test]
fn a_helper_killed_after_the_hosts_block_or_the_resolver_can_still_be_undone() {
    for os in [Os::Macos, Os::Windows, Os::Linux] {
        let rig = Rig::new(os);
        let before = rig.fs.files();
        let mut plan = resolver(ProductId::Ubiqx);
        plan.hosts = true;
        // The hosts block is written first; the run dies in the cache flush at its end.
        rig.faults().die_at(match os {
            Os::Macos => "-flushcache",
            Os::Windows => "/flushdns",
            Os::Linux => "flush-caches",
        });
        apply_until_killed(&rig, &plan);
        assert_ne!(
            rig.fs.files(),
            before,
            "{os:?}: the run changed the machine"
        );
        assert_eq!(rig.remove("ubiqx", "all"), EXIT_APPLIED, "{os:?}");
        assert_eq!(rig.fs.files(), before, "{os:?}: every byte is back");
        match os {
            Os::Macos => assert!(rig
                .mac
                .services
                .lock()
                .iter()
                .all(|service| service.servers.is_empty())),
            Os::Windows => assert!(rig
                .win
                .adapters
                .lock()
                .iter()
                .all(|adapter| adapter.fixed.is_empty())),
            Os::Linux => {}
        }
    }
}
