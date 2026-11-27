//! Browser policies: our values where a browser can be managed, somebody else's choice
//! left alone, and an undo that removes only what this helper created.

use serde_json::json;
use ubiqx_core::nofap::{BrowserFamily, Issue, Os, PolicyCreated, TierState};
use ubiqx_core::ProductId;
use ubiqx_nofap::plan::EXIT_APPLIED;
use ubiqx_nofap::policies;

use crate::sysblock::testing::{plan_of, Node, Rig, PROGRAM_FILES};

fn policies_only(owner: ProductId) -> ubiqx_core::nofap::SystemPlan {
    let mut plan = plan_of(owner, &[]);
    plan.hosts = false;
    plan.resolver = None;
    plan
}

fn made(browser: BrowserFamily, key: &str) -> PolicyCreated {
    PolicyCreated {
        browser,
        key: key.to_owned(),
    }
}

const CHROME_FILE: &str = "/etc/opt/chrome/policies/managed/ubiqx-shield.json";
const FIREFOX_FILE: &str = "/etc/firefox/policies/policies.json";

#[test]
fn linux_writes_its_own_files_and_nothing_else() {
    let rig = Rig::new(Os::Linux);
    let plan = policies_only(ProductId::Ubiqx);
    let files_before = rig.fs.files();
    assert_eq!(rig.apply(&plan), EXIT_APPLIED);
    assert_eq!(
        rig.fs.text(CHROME_FILE).unwrap(),
        "{\n  \"DnsOverHttpsMode\": \"off\",\n  \"ForceGoogleSafeSearch\": true\n}\n"
    );
    assert_eq!(
        rig.fs.text(FIREFOX_FILE).unwrap(),
        "{\n  \"policies\": {\n    \"DNSOverHTTPS\": {\n      \"Enabled\": false,\n      \"Locked\": true\n    }\n  }\n}\n"
    );
    // Root-owned, readable by the browsers, in directories nobody else can write to.
    for file in [CHROME_FILE, FIREFOX_FILE] {
        let Some(Node::File { uid, mode, .. }) = rig.fs.node(file) else {
            panic!("{file} is a file")
        };
        assert_eq!((uid, mode), (0, 0o644));
    }
    assert_eq!(
        rig.fs.node("/etc/opt/chrome/policies/managed"),
        Some(Node::Dir {
            uid: 0,
            mode: 0o755
        })
    );
    // Nothing for browsers that are not installed.
    for absent in [
        "/etc/chromium",
        "/etc/chromium-browser",
        "/etc/opt/edge",
        "/etc/brave",
    ] {
        assert_eq!(rig.fs.node(absent), None, "{absent}");
    }
    assert_eq!(rig.assess(&plan).policies, TierState::Applied);
    let created = rig.observe().state.unwrap().policies_created;
    assert_eq!(
        created,
        vec![
            made(BrowserFamily::Chrome, "DnsOverHttpsMode"),
            made(BrowserFamily::Chrome, "ForceGoogleSafeSearch"),
            made(BrowserFamily::Firefox, "DNSOverHTTPS.Enabled"),
            made(BrowserFamily::Firefox, "DNSOverHTTPS.Locked"),
        ]
    );
    assert_eq!(rig.remove("ubiqx", "all"), EXIT_APPLIED);
    assert_eq!(rig.fs.files(), files_before);
}

#[test]
fn linux_covers_each_family_and_both_chromium_packagings() {
    let rig = Rig::new(Os::Linux);
    rig.fs.file("/usr/bin/chromium", b"");
    rig.fs.dir("/opt/microsoft/msedge");
    rig.fs.dir("/opt/brave.com/brave");
    let plan = policies_only(ProductId::Ubiqx);
    assert_eq!(rig.apply(&plan), EXIT_APPLIED);
    for (file, browser) in [
        (
            "/etc/chromium/policies/managed/ubiqx-shield.json",
            BrowserFamily::Chromium,
        ),
        (
            "/etc/chromium-browser/policies/managed/ubiqx-shield.json",
            BrowserFamily::Chromium,
        ),
        (
            "/etc/opt/edge/policies/managed/ubiqx-shield.json",
            BrowserFamily::Edge,
        ),
        (
            "/etc/brave/policies/managed/ubiqx-shield.json",
            BrowserFamily::Brave,
        ),
    ] {
        assert_eq!(
            rig.fs.text(file).unwrap(),
            policies::chromium_json(browser),
            "{file}"
        );
    }
    assert!(rig
        .fs
        .text("/etc/opt/edge/policies/managed/ubiqx-shield.json")
        .unwrap()
        .contains("\"ForceBingSafeSearch\": 2"));
    assert!(rig
        .fs
        .text("/etc/brave/policies/managed/ubiqx-shield.json")
        .unwrap()
        .contains("\"TorDisabled\": true"));
    assert_eq!(rig.assess(&plan).policies, TierState::Applied);
    // One of the two Chromium files deleted by hand: that browser is no longer applied.
    use crate::sysblock::Fs;
    rig.fs
        .remove_file(std::path::Path::new(
            "/etc/chromium-browser/policies/managed/ubiqx-shield.json",
        ))
        .unwrap();
    assert_ne!(rig.assess(&plan).policies, TierState::Applied);
}

#[test]
fn linux_leaves_a_managed_browser_alone() {
    let rig = Rig::new(Os::Linux);
    let foreign = include_bytes!("../fixtures/chrome-foreign-policy.json");
    let foreign_file = "/etc/opt/chrome/policies/managed/company.json";
    rig.fs.file(foreign_file, foreign);
    let theirs = b"{\"policies\":{\"DNSOverHTTPS\":{\"Enabled\":true,\"ProviderURL\":\"https://dns.example/q\"}}}";
    rig.fs.file(FIREFOX_FILE, theirs);
    let plan = policies_only(ProductId::Ubiqx);

    let seen = rig.observe();
    let chrome = seen
        .policies
        .iter()
        .find(|p| p.browser == BrowserFamily::Chrome)
        .unwrap();
    assert_eq!(
        chrome.foreign.len(),
        2,
        "the two keys this tier manages, as the other file sets them"
    );
    assert_eq!(chrome.foreign[0].value, json!("automatic"));
    let firefox = seen
        .policies
        .iter()
        .find(|p| p.browser == BrowserFamily::Firefox)
        .unwrap();
    assert_eq!(firefox.error.as_deref(), Some("conflict"));
    let judged = rig.assess(&plan);
    assert!(judged.issues.contains(&Issue::PolicyConflict));

    rig.fs.clear_log();
    assert_eq!(rig.apply(&plan), EXIT_APPLIED);
    assert_eq!(
        rig.fs.node(CHROME_FILE),
        None,
        "nothing written next to somebody else's choice"
    );
    assert_eq!(rig.fs.text(foreign_file).unwrap().as_bytes(), foreign);
    assert_eq!(rig.fs.text(FIREFOX_FILE).unwrap().as_bytes(), theirs);
    assert!(rig
        .fs
        .mutations()
        .iter()
        .all(|change| change.contains("ubiqx-shield")));
    assert!(rig.observe().state.unwrap().policies_created.is_empty());
    assert_eq!(
        rig.assess(&plan).policies,
        TierState::Unsupported {
            reason: "conflict".into()
        }
    );
    // Undo touches neither.
    assert_eq!(rig.remove("ubiqx", "all"), EXIT_APPLIED);
    assert_eq!(rig.fs.text(foreign_file).unwrap().as_bytes(), foreign);
    assert_eq!(rig.fs.text(FIREFOX_FILE).unwrap().as_bytes(), theirs);
}

#[test]
fn linux_a_foreign_file_that_agrees_is_no_conflict_and_a_flatpak_is_out_of_reach() {
    let rig = Rig::new(Os::Linux);
    rig.fs.file(
        "/etc/opt/chrome/policies/managed/company.json",
        b"{\"DnsOverHttpsMode\":\"off\",\"HomepageLocation\":\"https://intranet.example/\"}",
    );
    rig.fs.dir("/var/lib/flatpak/app/com.brave.Browser");
    let plan = policies_only(ProductId::Ubiqx);
    let brave = |rig: &Rig| {
        let seen = rig.observe();
        seen.policies
            .into_iter()
            .find(|p| p.browser == BrowserFamily::Brave)
            .unwrap()
    };
    assert_eq!(brave(&rig).error.as_deref(), Some("flatpak"));
    assert_eq!(rig.apply(&plan), EXIT_APPLIED);
    assert!(
        rig.fs.node(CHROME_FILE).is_some(),
        "an equal value is nobody's conflict"
    );
    assert_eq!(
        rig.fs.node("/etc/brave"),
        None,
        "nothing is written for a Flatpak browser"
    );
    let judged = rig.assess(&plan);
    assert_eq!(judged.policies, TierState::Applied);
    assert!(!judged.issues.contains(&Issue::PolicyConflict));
}

#[test]
fn linux_undo_deletes_a_firefox_file_only_while_it_is_still_ours() {
    let rig = Rig::new(Os::Linux);
    let plan = policies_only(ProductId::Ubiqx);
    rig.apply(&plan);
    // Somebody extended the file after the apply: it is theirs now.
    let edited = b"{\"policies\":{\"DNSOverHTTPS\":{\"Enabled\":false,\"Locked\":true},\"DisableTelemetry\":true}}";
    rig.fs.file(FIREFOX_FILE, edited);
    assert_eq!(rig.remove("ubiqx", "all"), EXIT_APPLIED);
    assert_eq!(rig.fs.text(FIREFOX_FILE).unwrap().as_bytes(), edited);
    assert_eq!(
        rig.fs.node(CHROME_FILE),
        None,
        "our own file goes whatever it holds"
    );
}

const CHROME_KEY: &str = "HKLM\\SOFTWARE\\Policies\\Google\\Chrome";
const EDGE_KEY: &str = "HKLM\\SOFTWARE\\Policies\\Microsoft\\Edge";
const FIREFOX_KEY: &str = "HKLM\\SOFTWARE\\Policies\\Mozilla\\Firefox\\DNSOverHTTPS";

fn reg(rig: &Rig, key: &str, name: &str) -> Option<(String, String)> {
    rig.win
        .registry
        .lock()
        .get(&(key.to_owned(), name.to_owned()))
        .cloned()
}

fn set_reg(rig: &Rig, key: &str, name: &str, kind: &str, data: &str) {
    rig.win.registry.lock().insert(
        (key.to_owned(), name.to_owned()),
        (kind.to_owned(), data.to_owned()),
    );
}

#[test]
fn windows_writes_named_values_and_removes_only_its_own() {
    let rig = Rig::new(Os::Windows);
    rig.fs.file(
        &format!("{PROGRAM_FILES} (x86)\\Microsoft\\Edge\\Application\\msedge.exe"),
        b"",
    );
    // Already there before us, with the value we would write: not ours to delete.
    set_reg(&rig, EDGE_KEY, "ForceGoogleSafeSearch", "REG_DWORD", "0x1");
    // An unrelated policy of somebody else.
    set_reg(
        &rig,
        EDGE_KEY,
        "HomepageLocation",
        "REG_SZ",
        "https://intranet.example/",
    );
    let plan = policies_only(ProductId::Ubiqx);
    assert_eq!(rig.apply(&plan), EXIT_APPLIED);
    assert_eq!(
        reg(&rig, CHROME_KEY, "DnsOverHttpsMode"),
        Some(("REG_SZ".into(), "off".into()))
    );
    assert_eq!(
        reg(&rig, CHROME_KEY, "ForceGoogleSafeSearch"),
        Some(("REG_DWORD".into(), "0x1".into()))
    );
    assert_eq!(
        reg(&rig, EDGE_KEY, "DnsOverHttpsMode"),
        Some(("REG_SZ".into(), "off".into()))
    );
    assert_eq!(
        reg(&rig, EDGE_KEY, "ForceBingSafeSearch"),
        Some(("REG_DWORD".into(), "0x2".into()))
    );
    assert_eq!(
        reg(&rig, FIREFOX_KEY, "Enabled"),
        Some(("REG_DWORD".into(), "0x0".into()))
    );
    assert_eq!(
        reg(&rig, FIREFOX_KEY, "Locked"),
        Some(("REG_DWORD".into(), "0x1".into()))
    );
    assert_eq!(rig.assess(&plan).policies, TierState::Applied);
    let created = rig.observe().state.unwrap().policies_created;
    assert!(created.contains(&made(BrowserFamily::Edge, "ForceBingSafeSearch")));
    assert!(
        !created.contains(&made(BrowserFamily::Edge, "ForceGoogleSafeSearch")),
        "a value that was there before is not listed as created"
    );
    // A value of ours somebody changed after the apply is theirs now.
    set_reg(&rig, CHROME_KEY, "DnsOverHttpsMode", "REG_SZ", "secure");

    assert_eq!(rig.remove("ubiqx", "all"), EXIT_APPLIED);
    let left: Vec<(String, String)> = rig.win.registry.lock().keys().cloned().collect();
    assert_eq!(
        left,
        vec![
            (CHROME_KEY.to_owned(), "DnsOverHttpsMode".to_owned()),
            (EDGE_KEY.to_owned(), "ForceGoogleSafeSearch".to_owned()),
            (EDGE_KEY.to_owned(), "HomepageLocation".to_owned()),
        ]
    );
    assert_eq!(
        reg(&rig, CHROME_KEY, "DnsOverHttpsMode").unwrap().1,
        "secure"
    );
}

#[test]
fn windows_leaves_a_managed_browser_alone() {
    let rig = Rig::new(Os::Windows);
    set_reg(&rig, CHROME_KEY, "DnsOverHttpsMode", "REG_SZ", "secure");
    set_reg(
        &rig,
        CHROME_KEY,
        "DnsOverHttpsTemplates",
        "REG_SZ",
        "https://dns.example/q",
    );
    let plan = policies_only(ProductId::Ubiqx);
    assert!(rig.assess(&plan).issues.contains(&Issue::PolicyConflict));
    rig.clear_calls();
    assert_eq!(rig.apply(&plan), EXIT_APPLIED);
    assert_eq!(
        reg(&rig, CHROME_KEY, "DnsOverHttpsMode").unwrap().1,
        "secure"
    );
    assert_eq!(
        reg(&rig, CHROME_KEY, "ForceGoogleSafeSearch"),
        None,
        "not even the other value"
    );
    assert!(!rig
        .calls()
        .iter()
        .any(|call| call.line().contains("add HKLM\\SOFTWARE\\Policies\\Google")));
    // Firefox, which nobody manages, is written.
    assert_eq!(reg(&rig, FIREFOX_KEY, "Locked").unwrap().1, "0x1");
    assert!(rig.assess(&plan).issues.contains(&Issue::PolicyConflict));
    assert_eq!(rig.remove("ubiqx", "all"), EXIT_APPLIED);
    assert_eq!(
        reg(&rig, CHROME_KEY, "DnsOverHttpsMode").unwrap().1,
        "secure"
    );
    assert_eq!(
        reg(&rig, CHROME_KEY, "DnsOverHttpsTemplates").unwrap().1,
        "https://dns.example/q"
    );
    assert_eq!(reg(&rig, FIREFOX_KEY, "Locked"), None);
}

#[test]
fn macos_writes_firefox_only_and_says_so_for_the_chromium_family() {
    let rig = Rig::new(Os::Macos);
    let plan = policies_only(ProductId::Ubiqx);
    assert_eq!(rig.apply(&plan), EXIT_APPLIED);
    let domain = rig.mac.firefox.lock().clone();
    let expected: Vec<(&str, &str)> = vec![
        ("DNSOverHTTPS__Enabled", "0"),
        ("DNSOverHTTPS__Locked", "1"),
        ("EnterprisePoliciesEnabled", "1"),
    ];
    let found: Vec<(&str, &str)> = domain
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    assert_eq!(found, expected);
    let judged = rig.assess(&plan);
    assert_eq!(judged.policies, TierState::Applied);
    assert!(
        judged.issues.contains(&Issue::PolicyAdvisoryOnly),
        "Chrome is installed and gets nothing"
    );
    assert!(!rig
        .calls()
        .iter()
        .any(|call| call.line().contains("com.google.Chrome")));
    assert_eq!(rig.remove("ubiqx", "all"), EXIT_APPLIED);
    assert!(rig.mac.firefox.lock().is_empty());
}

#[test]
fn macos_keeps_a_switch_and_policies_that_were_there_before() {
    let rig = Rig::new(Os::Macos);
    rig.mac
        .firefox
        .lock()
        .insert("EnterprisePoliciesEnabled".into(), "1".into());
    rig.mac
        .firefox
        .lock()
        .insert("DisableTelemetry".into(), "1".into());
    let plan = policies_only(ProductId::Ubiqx);
    assert_eq!(rig.apply(&plan), EXIT_APPLIED);
    assert_eq!(rig.mac.firefox.lock().len(), 4);
    let created = rig.observe().state.unwrap().policies_created;
    assert!(!created.contains(&made(BrowserFamily::Firefox, "EnterprisePoliciesEnabled")));
    assert_eq!(rig.remove("ubiqx", "all"), EXIT_APPLIED);
    let left: Vec<String> = rig.mac.firefox.lock().keys().cloned().collect();
    assert_eq!(left, ["DisableTelemetry", "EnterprisePoliciesEnabled"]);

    // The switch we created stays while somebody else's policy needs it.
    let rig = Rig::new(Os::Macos);
    rig.apply(&plan);
    rig.mac
        .firefox
        .lock()
        .insert("DisableTelemetry".into(), "1".into());
    assert_eq!(rig.remove("ubiqx", "all"), EXIT_APPLIED);
    let left: Vec<String> = rig.mac.firefox.lock().keys().cloned().collect();
    assert_eq!(left, ["DisableTelemetry", "EnterprisePoliciesEnabled"]);
}

#[test]
fn macos_leaves_a_firefox_somebody_else_configured() {
    // DoH switched on by policy: a value that differs from ours and that we did not create.
    let rig = Rig::new(Os::Macos);
    rig.mac
        .firefox
        .lock()
        .insert("EnterprisePoliciesEnabled".into(), "1".into());
    rig.mac
        .firefox
        .lock()
        .insert("DNSOverHTTPS__Enabled".into(), "1".into());
    let plan = policies_only(ProductId::Ubiqx);
    let before = rig.mac.firefox.lock().clone();
    assert_eq!(rig.apply(&plan), EXIT_APPLIED);
    assert_eq!(*rig.mac.firefox.lock(), before);
    assert!(rig.assess(&plan).issues.contains(&Issue::PolicyConflict));
    // Policies switched off as a whole.
    let rig = Rig::new(Os::Macos);
    rig.mac
        .firefox
        .lock()
        .insert("EnterprisePoliciesEnabled".into(), "0".into());
    assert_eq!(rig.apply(&plan), EXIT_APPLIED);
    assert_eq!(rig.mac.firefox.lock().len(), 1);
    assert!(rig.assess(&plan).issues.contains(&Issue::PolicyConflict));
}

#[test]
fn a_browser_installed_after_the_apply_reads_update_pending_and_the_next_apply_covers_it() {
    let rig = Rig::new(Os::Linux);
    let plan = policies_only(ProductId::Ubiqx);
    rig.apply(&plan);
    rig.fs.dir("/opt/brave.com/brave");
    assert_eq!(rig.assess(&plan).policies, TierState::UpdatePending);
    assert_eq!(rig.apply(&plan), EXIT_APPLIED);
    assert_eq!(rig.assess(&plan).policies, TierState::Applied);
    assert!(rig
        .fs
        .node("/etc/brave/policies/managed/ubiqx-shield.json")
        .is_some());
}
