//! What makes the privileged side safe to run: exact command lines, absolute programs,
//! an environment it builds itself, a state directory it can trust, and a state file
//! whose content never becomes a command.

use ubiqx_core::nofap::Os;
use ubiqx_core::ProductId;
use ubiqx_nofap::plan::{self, EXIT_APPLIED, EXIT_FAILED};

use crate::sysblock::testing::{plan_of, Call, Node, Rig, ALL_TIERS, PROGRAM_FILES, SYSTEM32};
use crate::sysblock::windows;

fn lines(calls: &[Call]) -> Vec<String> {
    calls.iter().map(Call::line).collect()
}

fn pairs(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter()
        .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
        .collect()
}

/// The commands of one `apply` and one `remove` of a plan with all three tiers.
fn cycle(rig: &Rig) -> (Vec<Call>, Vec<Call>) {
    let plan = plan_of(ProductId::Ubiqx, &[]);
    rig.clear_calls();
    assert_eq!(rig.apply(&plan), EXIT_APPLIED, "{:?}", rig.os);
    let applied = rig.calls();
    rig.clear_calls();
    assert_eq!(rig.remove("ubiqx", "all"), EXIT_APPLIED, "{:?}", rig.os);
    (applied, rig.calls())
}

#[test]
fn macos_runs_exactly_these_commands() {
    let (applied, removed) = cycle(&Rig::new(Os::Macos));
    let look = [
        "/usr/sbin/networksetup -listallnetworkservices",
        "/usr/sbin/networksetup -getdnsservers USB 10/100/1000 LAN",
        "/usr/sbin/networksetup -getdnsservers Wi-Fi",
        "/usr/sbin/networksetup -getdnsservers Thunderbolt Bridge",
        "/usr/bin/defaults read /Library/Preferences/org.mozilla.firefox",
    ];
    let mut expected: Vec<&str> = look.to_vec();
    expected.extend([
        "/usr/sbin/networksetup -setdnsservers USB 10/100/1000 LAN 1.1.1.3 1.0.0.3 2606:4700:4700::1113 2606:4700:4700::1003",
        "/usr/sbin/networksetup -setdnsservers Wi-Fi 1.1.1.3 1.0.0.3 2606:4700:4700::1113 2606:4700:4700::1003",
        "/usr/bin/defaults read /Library/Preferences/org.mozilla.firefox",
        "/usr/bin/defaults write /Library/Preferences/org.mozilla.firefox EnterprisePoliciesEnabled -bool TRUE",
        "/usr/bin/defaults write /Library/Preferences/org.mozilla.firefox DNSOverHTTPS__Enabled -bool FALSE",
        "/usr/bin/defaults write /Library/Preferences/org.mozilla.firefox DNSOverHTTPS__Locked -bool TRUE",
        "/usr/bin/dscacheutil -flushcache",
        "/usr/bin/killall -HUP mDNSResponder",
    ]);
    assert_eq!(lines(&applied), expected);
    let mut expected: Vec<&str> = look.to_vec();
    expected.extend([
        "/usr/sbin/networksetup -setdnsservers USB 10/100/1000 LAN empty",
        "/usr/sbin/networksetup -setdnsservers Wi-Fi empty",
        "/usr/bin/defaults delete /Library/Preferences/org.mozilla.firefox DNSOverHTTPS__Enabled",
        "/usr/bin/defaults delete /Library/Preferences/org.mozilla.firefox DNSOverHTTPS__Locked",
        "/usr/bin/defaults read /Library/Preferences/org.mozilla.firefox",
        "/usr/bin/defaults delete /Library/Preferences/org.mozilla.firefox EnterprisePoliciesEnabled",
        "/usr/bin/dscacheutil -flushcache",
        "/usr/bin/killall -HUP mDNSResponder",
    ]);
    assert_eq!(lines(&removed), expected);
    let env = pairs(&[("PATH", "/usr/bin:/bin:/usr/sbin:/sbin"), ("LANG", "C")]);
    for call in applied.iter().chain(&removed) {
        assert!(call.program.starts_with('/'), "{}", call.program);
        assert_eq!(call.env, env, "{}", call.line());
    }
}

#[test]
fn linux_runs_exactly_these_commands() {
    let (applied, removed) = cycle(&Rig::new(Os::Linux));
    assert_eq!(
        lines(&applied),
        [
            "/usr/bin/systemctl is-active systemd-resolved",
            "/usr/bin/systemctl restart systemd-resolved",
            "/usr/bin/resolvectl flush-caches",
        ]
    );
    assert_eq!(
        lines(&removed),
        [
            "/usr/bin/systemctl is-active systemd-resolved",
            "/usr/bin/systemctl restart systemd-resolved",
            "/usr/bin/resolvectl flush-caches",
        ]
    );
    let env = pairs(&[("PATH", "/usr/bin:/bin:/usr/sbin:/sbin"), ("LANG", "C")]);
    for call in applied.iter().chain(&removed) {
        assert_eq!(call.env, env, "{}", call.line());
    }
}

const POWERSHELL: &str = "C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe -NoProfile -NonInteractive -Command ";
const FULL_READ: &str = "$ErrorActionPreference = 'Stop'; $rows = @(NetAdapter\\Get-NetAdapter | Where-Object { $_.Status -eq 'Up' } | ForEach-Object { [pscustomobject]@{ index = [int]$_.ifIndex; guid = [string]$_.InterfaceGuid; name = [string]$_.Name; servers = @(DnsClient\\Get-DnsClientServerAddress -InterfaceIndex $_.ifIndex | ForEach-Object { $_.ServerAddresses }) } }); ConvertTo-Json -InputObject $rows -Depth 3 -Compress";

fn windows_look() -> Vec<String> {
    let mut look = vec![format!("{POWERSHELL}{FULL_READ}")];
    for key in [
        "HKLM\\SOFTWARE\\Policies\\Google\\Chrome",
        "HKLM\\SOFTWARE\\Policies\\Chromium",
        "HKLM\\SOFTWARE\\Policies\\Microsoft\\Edge",
        "HKLM\\SOFTWARE\\Policies\\BraveSoftware\\Brave",
        "HKLM\\SOFTWARE\\Policies\\Mozilla\\Firefox\\DNSOverHTTPS",
    ] {
        look.push(format!("C:\\Windows\\System32\\reg.exe query {key}"));
    }
    look
}

#[test]
fn windows_runs_exactly_these_commands() {
    let (applied, removed) = cycle(&Rig::new(Os::Windows));
    let reg = "C:\\Windows\\System32\\reg.exe";
    let mut expected = windows_look();
    expected.extend([
        format!("{POWERSHELL}$ErrorActionPreference = 'Stop'; NetAdapter\\Get-NetAdapter | Where-Object {{ $_.Status -eq 'Up' }} | ForEach-Object {{ DnsClient\\Set-DnsClientServerAddress -InterfaceIndex $_.ifIndex -ServerAddresses ('1.1.1.3','1.0.0.3','2606:4700:4700::1113','2606:4700:4700::1003') }}"),
        format!("{reg} add HKLM\\SOFTWARE\\Policies\\Google\\Chrome /v DnsOverHttpsMode /t REG_SZ /d off /f"),
        format!("{reg} add HKLM\\SOFTWARE\\Policies\\Google\\Chrome /v ForceGoogleSafeSearch /t REG_DWORD /d 1 /f"),
        format!("{reg} add HKLM\\SOFTWARE\\Policies\\Mozilla\\Firefox\\DNSOverHTTPS /v Enabled /t REG_DWORD /d 0 /f"),
        format!("{reg} add HKLM\\SOFTWARE\\Policies\\Mozilla\\Firefox\\DNSOverHTTPS /v Locked /t REG_DWORD /d 1 /f"),
        "C:\\Windows\\System32\\ipconfig.exe /flushdns".to_owned(),
    ]);
    assert_eq!(lines(&applied), expected);
    let mut expected = windows_look();
    expected.extend([
        format!("{POWERSHELL}$ErrorActionPreference = 'Stop'; foreach ($i in @(12)) {{ DnsClient\\Set-DnsClientServerAddress -InterfaceIndex $i -ResetServerAddresses }}"),
        format!("{reg} delete HKLM\\SOFTWARE\\Policies\\Google\\Chrome /v DnsOverHttpsMode /f"),
        format!("{reg} delete HKLM\\SOFTWARE\\Policies\\Google\\Chrome /v ForceGoogleSafeSearch /f"),
        format!("{reg} delete HKLM\\SOFTWARE\\Policies\\Mozilla\\Firefox\\DNSOverHTTPS /v Enabled /f"),
        format!("{reg} delete HKLM\\SOFTWARE\\Policies\\Mozilla\\Firefox\\DNSOverHTTPS /v Locked /f"),
        "C:\\Windows\\System32\\ipconfig.exe /flushdns".to_owned(),
    ]);
    assert_eq!(lines(&removed), expected);
    // The three fixed scripts, spelled out: a change to one is a visible diff here.
    assert_eq!(windows::FULL_READ_SCRIPT, FULL_READ);
    assert_eq!(
        windows::SET_SCRIPT,
        "$ErrorActionPreference = 'Stop'; NetAdapter\\Get-NetAdapter | Where-Object { $_.Status -eq 'Up' } | ForEach-Object { DnsClient\\Set-DnsClientServerAddress -InterfaceIndex $_.ifIndex -ServerAddresses ({servers}) }"
    );
    assert_eq!(
        windows::RESET_SCRIPT,
        "$ErrorActionPreference = 'Stop'; foreach ($i in @({indexes})) { DnsClient\\Set-DnsClientServerAddress -InterfaceIndex $i -ResetServerAddresses }"
    );
    for call in applied.iter().chain(&removed) {
        assert!(
            !call.line().to_lowercase().contains("icacls"),
            "{}",
            call.line()
        );
        assert!(!call.line().contains("ExecutionPolicy"), "{}", call.line());
        assert!(!call.line().contains("netsh"), "{}", call.line());
    }
}

fn windows_env() -> Vec<(String, String)> {
    pairs(&[
        ("SystemRoot", "C:\\Windows"),
        ("windir", "C:\\Windows"),
        ("PATH", "C:\\Windows\\System32"),
        (
            "PSModulePath",
            "C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\Modules",
        ),
    ])
}

/// What a hostile same-user process could set before the UAC prompt: an elevated process
/// started through UAC receives the user's variables.
const HOSTILE_DIR: &str = "C:\\Users\\Public\\evil";
const HOSTILE_VARIABLES: [&str; 6] = [
    "SystemRoot",
    "windir",
    "ProgramData",
    "PSModulePath",
    "ProgramFiles",
    "PATH",
];
const INNER_TEST: &str = "sysblock::tests::hardening::windows_cycle_inside_a_hostile_environment";

#[test]
fn windows_takes_no_path_from_the_environment() {
    // The variables are set for a child process only (this test binary, running the one
    // test below), so no other test ever sees them. On a Windows host a process cannot be
    // expected to start with a false `SystemRoot`; the back-end is the same code there.
    if std::env::consts::OS == "windows" {
        return;
    }
    let this_binary = std::env::current_exe().expect("the test binary");
    let mut child = std::process::Command::new(this_binary);
    child.args(["--exact", INNER_TEST, "--ignored"]);
    for name in HOSTILE_VARIABLES {
        child.env(name, HOSTILE_DIR);
    }
    let output = child.output().expect("the child test run");
    let printed = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "{printed}");
    assert!(
        printed.contains("1 passed"),
        "the inner test did not run: {printed}"
    );
}

#[test]
#[ignore = "run by windows_takes_no_path_from_the_environment, in a child process with a hostile environment"]
fn windows_cycle_inside_a_hostile_environment() {
    for name in HOSTILE_VARIABLES {
        assert_eq!(
            std::env::var(name).as_deref(),
            Ok(HOSTILE_DIR),
            "{name} is not set"
        );
    }
    let rig = Rig::new(Os::Windows);
    let (applied, removed) = cycle(&rig);
    assert_eq!(applied.len(), 12);
    assert_eq!(removed.len(), 12);
    for call in applied.iter().chain(&removed) {
        assert!(
            call.program.starts_with("C:\\Windows\\System32\\"),
            "{}",
            call.program
        );
        assert_eq!(call.env, windows_env(), "{}", call.line());
        assert!(!call.line().contains("evil"), "{}", call.line());
    }
    // Nothing was created anywhere but under the two directories Windows itself named.
    for path in rig.fs.snapshot().keys() {
        let known = path.starts_with(SYSTEM32)
            || path.starts_with(PROGRAM_FILES)
            || path == "C:"
            || path == "C:\\Windows";
        assert!(known && !path.contains("evil"), "{path}");
    }
}

#[test]
fn a_state_directory_that_is_a_link_is_removed_without_following_it() {
    for os in [Os::Linux, Os::Macos] {
        let rig = Rig::new(os);
        let dir = crate::sysblock::testing::backend_for(os).state_dir();
        let dir = dir.to_string_lossy().into_owned();
        // Planted before the first run: a link to a directory with a state file that
        // claims an owner, and a file an attacker would like to see rewritten.
        rig.fs.dir("/home/ana/planted");
        rig.fs.file(
            "/home/ana/planted/state.json",
            b"{\"v\":1,\"owners\":{\"shield\":{\"hosts\":true,\"extra\":[\"planted.example\"]}}}",
        );
        rig.fs.file("/home/ana/planted/keep.txt", b"untouched");
        rig.fs.link(&dir, "/home/ana/planted");
        let planted_before: Vec<_> = rig
            .fs
            .snapshot()
            .into_iter()
            .filter(|(path, _)| path.starts_with("/home/ana"))
            .collect();

        assert_eq!(
            rig.apply(&plan_of(ProductId::Ubiqx, &[])),
            EXIT_APPLIED,
            "{os:?}"
        );
        // The link is gone, a real root directory took its place, and what it pointed
        // at was neither read nor changed.
        assert_eq!(
            rig.fs.node(&dir),
            Some(Node::Dir {
                uid: 0,
                mode: 0o755
            }),
            "{os:?}"
        );
        let planted_after: Vec<_> = rig
            .fs
            .snapshot()
            .into_iter()
            .filter(|(path, _)| path.starts_with("/home/ana"))
            .collect();
        assert_eq!(planted_after, planted_before);
        assert!(rig.fs.mutations().contains(&format!("remove {dir}")));
        assert!(!rig
            .fs
            .mutations()
            .iter()
            .any(|change| change.contains("/home/ana")));
        let state = rig.observe().state.unwrap();
        assert_eq!(
            state.owners.keys().copied().collect::<Vec<_>>(),
            [ProductId::Ubiqx]
        );
        assert!(!rig.hosts_text().contains("planted.example"));
    }
}

#[test]
fn a_state_directory_others_can_write_to_is_emptied_before_use() {
    for (uid, mode) in [(0, 0o775), (0, 0o777), (1000, 0o755)] {
        let rig = Rig::new(Os::Linux);
        rig.fs.dir_owned("/var/lib/ubiqx-shield", uid, mode);
        rig.fs.file_owned(
            "/var/lib/ubiqx-shield/state.json",
            b"{\"v\":1,\"owners\":{\"shield\":{\"hosts\":true,\"extra\":[\"planted.example\"]}}}",
            1000,
            0o644,
        );
        rig.fs.file("/var/lib/ubiqx-shield/other", b"x");
        assert_eq!(rig.apply(&plan_of(ProductId::Ubiqx, &[])), EXIT_APPLIED);
        assert_eq!(
            rig.fs.node("/var/lib/ubiqx-shield"),
            Some(Node::Dir {
                uid: 0,
                mode: 0o755
            }),
            "{uid} {mode:o}"
        );
        assert_eq!(rig.fs.node("/var/lib/ubiqx-shield/other"), None);
        assert!(!rig.hosts_text().contains("planted.example"));
        assert!(!rig
            .observe()
            .state
            .unwrap()
            .owners
            .contains_key(&ProductId::Shield));
    }
    // A plain file where the directory belongs is unlinked the same way.
    let rig = Rig::new(Os::Linux);
    rig.fs.file("/var/lib/ubiqx-shield", b"not a directory");
    assert_eq!(rig.apply(&plan_of(ProductId::Ubiqx, &[])), EXIT_APPLIED);
    assert!(matches!(
        rig.fs.node("/var/lib/ubiqx-shield"),
        Some(Node::Dir { .. })
    ));
}

#[test]
fn windows_refuses_a_reparse_point_at_the_state_path() {
    let state_dir = format!("{PROGRAM_FILES}\\ubiqx-shield");
    for planted in ["link", "file"] {
        let rig = Rig::new(Os::Windows);
        if planted == "link" {
            rig.fs.dir("C:\\Users\\Public\\target");
            rig.fs.link(&state_dir, "C:\\Users\\Public\\target");
        } else {
            rig.fs.file(&state_dir, b"x");
        }
        let before = rig.fs.snapshot();
        let encoded = plan::encode_plan(&plan_of(ProductId::Ubiqx, &[]));
        rig.fs.clear_log();
        for args in [
            &[
                "apply",
                "--plan-b64",
                encoded.as_str(),
                "--unverified",
                ALL_TIERS,
            ][..],
            &["remove", "--owner", "all", "--scope", "all"],
        ] {
            let (exit, _, err) = rig.helper(args);
            assert_eq!(exit, EXIT_FAILED, "{planted}: {err}");
        }
        assert_eq!(rig.fs.mutations(), Vec::<String>::new(), "{planted}");
        assert_eq!(rig.fs.snapshot(), before, "{planted}");
        assert_eq!(rig.calls(), Vec::new(), "{planted}: no command either");
    }
}

#[test]
fn a_helper_that_is_not_root_stops_before_it_changes_anything() {
    let rig = Rig::new(Os::Linux);
    rig.fs.set_euid(1000);
    let before = rig.hosts_text();
    assert_eq!(rig.apply(&plan_of(ProductId::Ubiqx, &[])), EXIT_FAILED);
    assert_eq!(rig.hosts_text(), before);
    assert_eq!(rig.calls(), Vec::new());
    assert_eq!(
        rig.fs
            .node("/etc/systemd/resolved.conf.d/90-ubiqx-shield.conf"),
        None
    );
}

/// A state file a tampering process would write: every free-text field holds an address,
/// a path or a command.
const TAMPERED: &str = r#"{
  "v": 1, "applied_at": 6,
  "owners": {
    "shield": { "hosts": true, "resolver": null, "policies": true,
                "extra": ["6.6.6.6", "2001:db8::6", "localhost", "evil.example\n6.6.6.6 bank.example",
                          "/etc/passwd", "$(id)", "portal.example.gov.br", "kept.example"] }
  },
  "hosts": { "sha256": "6.6.6.6", "names": 6 },
  "policies_created": [
    { "browser": "chrome", "key": "6.6.6.6" },
    { "browser": "firefox", "key": "..\\..\\Run" },
    { "browser": "chrome", "key": "HomepageLocation" }
  ],
  "report": [ { "tier": "resolver", "ok": false, "reason": "6.6.6.6; rm -rf /" } ]
}"#;

#[test]
fn nothing_read_from_the_state_file_becomes_an_address_a_path_or_a_command() {
    for os in Os::ALL {
        let clean = Rig::new(os);
        let (clean_applied, clean_removed) = cycle(&clean);

        let rig = Rig::new(os);
        let backend = crate::sysblock::testing::backend_for(os);
        rig.fs.dir(&backend.state_dir().to_string_lossy());
        rig.fs.file(&rig.state_file(), TAMPERED.as_bytes());
        let files_before = rig.fs.files();
        let (applied, removed) = cycle(&rig);

        // The same commands as on a machine without the file, apart from the policies
        // the planted owner "wants" staying while it is listed.
        let dns = |calls: &[Call]| -> Vec<String> {
            lines(calls)
                .into_iter()
                .filter(|line| {
                    ["-setdnsservers", "Set-DnsClientServerAddress", "systemctl"]
                        .iter()
                        .any(|command| line.contains(command))
                })
                .collect()
        };
        assert_eq!(dns(&applied), dns(&clean_applied), "{os:?}");
        assert_eq!(dns(&removed), dns(&clean_removed), "{os:?}");
        for line in lines(&applied).iter().chain(&lines(&removed)) {
            for planted in [
                "6.6.6.6",
                "2001:db8::6",
                "passwd",
                "$(id)",
                "rm -rf",
                "Run",
                "HomepageLocation",
            ] {
                assert!(!line.contains(planted), "{os:?}: {line}");
            }
        }
        // Of the planted names only the one valid public hostname reached the hosts
        // file, as a name on a null-address line.
        let hosts = rig.hosts_text();
        for planted in [
            "6.6.6.6",
            "2001:db8::6",
            "localhost ",
            "bank.example",
            "passwd",
            "$(id)",
            "gov.br",
        ] {
            assert!(!hosts.contains(planted), "{os:?}: {planted}");
        }
        assert!(hosts.contains("kept.example www.kept.example"), "{os:?}");
        // Removing the planted owner as well restores the machine.
        assert_eq!(rig.remove("all", "all"), EXIT_APPLIED);
        let mut files_after = rig.fs.files();
        files_after.insert(rig.state_file(), files_before[&rig.state_file()].clone());
        assert_eq!(files_after, files_before, "{os:?}");
    }
}

#[test]
fn a_state_file_that_cannot_be_read_is_no_state() {
    for junk in [
        "",
        "not json",
        "{\"v\":2}",
        "{\"v\":1,\"owners\":{\"shield\":{\"resolver\":\"6.6.6.6\"}}}",
        "[]",
    ] {
        let rig = Rig::new(Os::Linux);
        rig.fs.dir("/var/lib/ubiqx-shield");
        rig.fs
            .file("/var/lib/ubiqx-shield/state.json", junk.as_bytes());
        let seen = rig.observe();
        assert_eq!(seen.state, None, "{junk}");
        assert_eq!(seen.state_error.as_deref(), Some("unreadable"), "{junk}");
        assert_eq!(rig.apply(&plan_of(ProductId::Ubiqx, &[])), EXIT_APPLIED);
        let state = rig.observe().state.expect("a fresh state");
        assert_eq!(state.owners.len(), 1, "{junk}");
    }
}
