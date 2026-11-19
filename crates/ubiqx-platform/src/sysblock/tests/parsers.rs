//! The parsers against samples of what the tools print (see `fixtures/README.md`), and
//! the three back-ends' unprivileged reads through them.

use serde_json::json;
use ubiqx_core::nofap::{BrowserFamily, Os, PolicyValue};

use crate::sysblock::parse::{
    adapters_json, defaults_domain, ip_lines, key_leaf, name_servers, network_services,
    policy_file_values, reg_json, reg_values, resolv_conf_uses_stub,
};
use crate::sysblock::testing::{cf_servers, Rig, CF};

fn strings(list: &[&str]) -> Vec<String> {
    list.iter().map(|item| (*item).to_owned()).collect()
}

#[test]
fn networksetup_services_with_disabled_ones_and_a_header_in_any_language() {
    let english = network_services(include_str!("../fixtures/networksetup-services.txt"));
    assert_eq!(
        english,
        vec![
            ("USB 10/100/1000 LAN".to_owned(), true),
            ("Wi-Fi".to_owned(), true),
            ("iPhone USB".to_owned(), true),
            ("Thunderbolt Bridge".to_owned(), false),
        ]
    );
    let ptbr = network_services(include_str!("../fixtures/networksetup-services-ptbr.txt"));
    assert_eq!(
        ptbr,
        vec![
            ("Wi-Fi".to_owned(), true),
            ("Ethernet (USB-C)".to_owned(), true),
            ("Ponte Thunderbolt".to_owned(), false),
        ]
    );
    assert_eq!(network_services(""), Vec::new());
    assert_eq!(network_services("header only\n"), Vec::new());
    // Windows line endings and a blank line do not add services.
    assert_eq!(
        network_services("header\r\nWi-Fi\r\n\r\n*Off\r\n"),
        vec![("Wi-Fi".to_owned(), true), ("Off".to_owned(), false)]
    );
}

#[test]
fn networksetup_dns_servers_are_the_lines_that_are_addresses() {
    assert_eq!(
        ip_lines(include_str!("../fixtures/networksetup-dns-set.txt")),
        cf_servers()
    );
    assert_eq!(
        ip_lines(include_str!("../fixtures/networksetup-dns-none.txt")),
        Vec::<String>::new()
    );
    assert_eq!(
        ip_lines(include_str!("../fixtures/networksetup-dns-none-ptbr.txt")),
        Vec::<String>::new()
    );
    assert_eq!(
        ip_lines("192.168.0.1\nfe80::1%en0\nnot an address 1.1.1.1\n"),
        strings(&["192.168.0.1", "fe80::1%en0"])
    );
}

#[test]
fn powershell_json_with_one_and_several_adapters_and_no_ipv6() {
    let one = adapters_json(include_str!("../fixtures/powershell-adapters-one.json")).unwrap();
    assert_eq!(one.len(), 1);
    assert_eq!((one[0].index, one[0].name.as_str()), (12, "Wi-Fi"));
    assert_eq!(one[0].guid, "{11111111-2222-3333-4444-555555555555}");
    assert_eq!(one[0].servers, cf_servers());

    let several =
        adapters_json(include_str!("../fixtures/powershell-adapters-several.json")).unwrap();
    assert_eq!(several.len(), 4);
    assert_eq!(several[0].servers, strings(&["192.168.0.1", "fe80::1%12"]));
    // A single server arrives as a string, none as an empty list or null.
    assert_eq!(several[1].servers, strings(&["10.0.0.2"]));
    assert_eq!(
        several[1].guid, "{AAAAAAAA-BBBB-CCCC-DDDD-EEEEEEEEEEEE}",
        "upper case"
    );
    assert_eq!(several[2].name, "Conexão de Rede Bluetooth");
    assert!(several[2].servers.is_empty() && several[3].servers.is_empty());

    let no_ipv6 =
        adapters_json(include_str!("../fixtures/powershell-adapters-no-ipv6.json")).unwrap();
    assert_eq!(no_ipv6[0].servers, strings(&["1.1.1.3", "1.0.0.3"]));
    // Only the IPv4 pair is not the provider's whole set.
    assert!(!ubiqx_nofap::resolvers::is_exactly(CF, &no_ipv6[0].servers));

    assert_eq!(adapters_json("").unwrap(), Vec::new());
    assert_eq!(adapters_json("[]").unwrap(), Vec::new());
    assert_eq!(adapters_json("null").unwrap(), Vec::new());
    assert_eq!(
        adapters_json("Get-NetAdapter : The term is not recognized"),
        None
    );
    assert_eq!(adapters_json("[{\"name\":\"no index\"}]"), None);
    assert_eq!(adapters_json("7"), None);
}

#[test]
fn reg_query_output_values_types_and_keys() {
    let values = reg_values(include_str!("../fixtures/reg-policy-chrome.txt"));
    let names: Vec<&str> = values.iter().map(|value| value.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "DnsOverHttpsMode",
            "ForceGoogleSafeSearch",
            "HomepageLocation",
            "RestoreOnStartupURLs"
        ]
    );
    assert!(values
        .iter()
        .all(|value| value.key == "HKEY_LOCAL_MACHINE\\SOFTWARE\\Policies\\Google\\Chrome"));
    assert_eq!(reg_json(&values[0]), Some(json!("off")));
    assert_eq!(reg_json(&values[1]), Some(json!(1)));
    assert_eq!(values[2].data, "https://intranet.example/start page");
    assert_eq!(reg_json(&values[3]), None, "a type this tier does not read");

    let servers = reg_values(include_str!("../fixtures/reg-nameserver.txt"));
    assert_eq!(
        servers.len(),
        2,
        "the localised summary line is not a value"
    );
    assert_eq!(
        key_leaf(&servers[0].key),
        "{11111111-2222-3333-4444-555555555555}"
    );
    assert_eq!(
        name_servers(&servers[0].data),
        strings(&["1.1.1.3", "1.0.0.3"])
    );
    assert_eq!(servers[1].data, "");
    assert_eq!(name_servers(&servers[1].data), Vec::<String>::new());
    assert_eq!(
        name_servers("1.1.1.3 1.0.0.3,evil.example"),
        strings(&["1.1.1.3", "1.0.0.3", "evil.example"])
    );
    // The same output with Unix line endings parses the same.
    let unix = include_str!("../fixtures/reg-policy-chrome.txt").replace("\r\n", "\n");
    assert_eq!(reg_values(&unix), values);
    assert_eq!(
        reg_values("ERROR: The system was unable to find the key.\r\n"),
        Vec::new()
    );
    assert_eq!(
        reg_json(&reg_values("HKEY_X\\K\n    V    REG_DWORD    0xff\n")[0]),
        Some(json!(255))
    );
    assert_eq!(
        reg_json(&reg_values("HKEY_X\\K\n    V    REG_DWORD    ff\n")[0]),
        None
    );
}

#[test]
fn defaults_read_gives_the_top_level_keys() {
    let domain = defaults_domain(include_str!("../fixtures/defaults-firefox.txt"));
    assert_eq!(
        domain,
        vec![
            ("DNSOverHTTPS__Enabled".to_owned(), json!(0)),
            ("DNSOverHTTPS__Locked".to_owned(), json!(1)),
            ("EnterprisePoliciesEnabled".to_owned(), json!(1)),
            ("ExtensionSettings".to_owned(), json!(null)),
            ("Homepage".to_owned(), json!("https://example.com/")),
            ("ManagedBookmarks".to_owned(), json!(null)),
        ]
    );
    assert_eq!(defaults_domain(""), Vec::new());
    assert_eq!(defaults_domain("{\n}\n"), Vec::new());
}

#[test]
fn resolv_conf_points_at_resolved_only_with_the_stub_line() {
    assert!(resolv_conf_uses_stub(include_str!(
        "../fixtures/resolv-conf-stub.txt"
    )));
    assert!(!resolv_conf_uses_stub(include_str!(
        "../fixtures/resolv-conf-direct.txt"
    )));
    assert!(!resolv_conf_uses_stub(""));
    assert!(resolv_conf_uses_stub("nameserver\t127.0.0.53\n"));
    assert!(!resolv_conf_uses_stub("nameserver 127.0.0.530\n"));
}

#[test]
fn policy_files_give_only_the_keys_asked_for() {
    let keys = strings(&["DnsOverHttpsMode", "ForceGoogleSafeSearch"]);
    let values = policy_file_values(
        include_str!("../fixtures/chrome-foreign-policy.json"),
        &keys,
    )
    .unwrap();
    assert_eq!(
        values,
        vec![
            PolicyValue {
                key: "DnsOverHttpsMode".into(),
                value: json!("automatic")
            },
            PolicyValue {
                key: "ForceGoogleSafeSearch".into(),
                value: json!(true)
            },
        ]
    );
    assert_eq!(policy_file_values("{}", &keys), Some(Vec::new()));
    assert_eq!(policy_file_values("not json", &keys), None);
    assert_eq!(policy_file_values("[1]", &keys), None);
}

#[test]
fn systemctl_and_resolv_conf_decide_what_linux_reports() {
    let rig = Rig::new(Os::Linux);
    let seen = rig.observe().resolver;
    assert_eq!(seen.resolved_active, Some(true));
    assert_eq!(seen.resolv_conf_uses_resolved, Some(true));
    assert!(seen.interfaces.is_empty() && seen.dropin.is_none() && seen.error.is_none());
    // `inactive` with exit 3, as systemctl answers for a stopped unit.
    *rig.linux.resolved_active.lock() = false;
    assert_eq!(rig.observe().resolver.resolved_active, Some(false));
    // No systemctl at all.
    *rig.linux.resolved_active.lock() = true;
    rig.linux
        .missing
        .lock()
        .insert("/usr/bin/systemctl".to_owned());
    assert_eq!(rig.observe().resolver.resolved_active, Some(false));
    // The service runs but resolv.conf goes elsewhere.
    rig.linux.missing.lock().clear();
    rig.fs.file(
        "/etc/resolv.conf",
        include_bytes!("../fixtures/resolv-conf-direct.txt"),
    );
    let direct = rig.observe().resolver;
    assert_eq!(direct.resolved_active, Some(true));
    assert_eq!(direct.resolv_conf_uses_resolved, Some(false));
}

#[test]
fn the_three_reads_see_a_clean_machine() {
    let mac = Rig::new(Os::Macos).observe();
    let names: Vec<(&str, bool)> = mac
        .resolver
        .interfaces
        .iter()
        .map(|service| (service.name.as_str(), service.active))
        .collect();
    assert_eq!(
        names,
        [
            ("USB 10/100/1000 LAN", true),
            ("Wi-Fi", true),
            ("Thunderbolt Bridge", false)
        ]
    );
    assert!(mac
        .resolver
        .interfaces
        .iter()
        .all(|service| service.servers.is_empty()));
    let browsers: Vec<BrowserFamily> = mac.policies.iter().map(|seen| seen.browser).collect();
    assert_eq!(browsers, [BrowserFamily::Chrome, BrowserFamily::Firefox]);

    let windows = Rig::new(Os::Windows).observe();
    assert_eq!(
        windows.resolver.interfaces.len(),
        1,
        "only the adapter that is up"
    );
    assert_eq!(windows.resolver.interfaces[0].id, "12");
    assert_eq!(
        windows.resolver.interfaces[0].servers,
        strings(&["192.168.0.1"])
    );
    assert!(windows.resolver.full);
    assert_eq!(
        windows.hosts.text.as_deref(),
        Some(crate::sysblock::testing::HOSTS_WINDOWS)
    );

    let linux = Rig::new(Os::Linux).observe();
    assert_eq!(
        linux.hosts.text.as_deref(),
        Some(crate::sysblock::testing::HOSTS_UNIX)
    );
    assert!(linux.hosts.error.is_none() && linux.state.is_none() && linux.state_error.is_none());
    let browsers: Vec<BrowserFamily> = linux.policies.iter().map(|seen| seen.browser).collect();
    assert_eq!(browsers, [BrowserFamily::Chrome, BrowserFamily::Firefox]);
    // The paths each back-end names.
    let path = |os: Os| crate::sysblock::testing::backend_for(os).hosts_path();
    assert_eq!(path(Os::Linux).to_string_lossy(), "/etc/hosts");
    assert_eq!(path(Os::Macos).to_string_lossy(), "/etc/hosts");
    assert_eq!(
        path(Os::Windows).to_string_lossy(),
        "C:\\Windows\\System32\\drivers\\etc\\hosts"
    );
}
