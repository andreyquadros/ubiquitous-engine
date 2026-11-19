//! The macOS back-end: `networksetup` for DNS per network service, `defaults` for the
//! Firefox policy, `dscacheutil` and `killall` for the caches. Nothing is written for
//! the Chromium family: without MDM those browsers accept only "recommended" policies,
//! which the user can override, so writing them would buy a "managed" banner for no
//! effect. They are reported as present, and the judgement says so.

use std::path::{Path, PathBuf};

use serde_json::Value;
use ubiqx_core::nofap::{
    reason, BrowserFamily, InterfaceDns, Os, PolicyCreated, PolicyObservation, PolicyValue,
    ResolverId, ResolverObservation, SystemObservation,
};
use ubiqx_nofap::policies::{self, FIREFOX_ENABLED, FIREFOX_LOCKED};
use ubiqx_nofap::resolvers::{self, RESOLVERS};
use ubiqx_nofap::{plan, state_dir};

use super::{parse, Fs, OsBackend, Runner};

pub const NETWORKSETUP: &str = "/usr/sbin/networksetup";
pub const DEFAULTS: &str = "/usr/bin/defaults";
pub const DSCACHEUTIL: &str = "/usr/bin/dscacheutil";
pub const KILLALL: &str = "/usr/bin/killall";
pub const OSASCRIPT: &str = "/usr/bin/osascript";
pub const HOSTS_PATH: &str = "/etc/hosts";
/// What `networksetup -setdnsservers` takes instead of addresses to go back to automatic.
pub const DNS_EMPTY: &str = "empty";
/// The preferences domain of Firefox's policies (no MDM needed).
pub const FIREFOX_DOMAIN: &str = "/Library/Preferences/org.mozilla.firefox";
/// The switch Firefox reads before any policy of the domain, and the two fields of the
/// `DNSOverHTTPS` policy as the domain spells them.
pub const FIREFOX_SWITCH: &str = "EnterprisePoliciesEnabled";
pub const FIREFOX_ENABLED_PREF: &str = "DNSOverHTTPS__Enabled";
pub const FIREFOX_LOCKED_PREF: &str = "DNSOverHTTPS__Locked";
/// Where each browser is installed.
pub const APPLICATIONS: [(BrowserFamily, &str); 5] = [
    (BrowserFamily::Chrome, "/Applications/Google Chrome.app"),
    (BrowserFamily::Chromium, "/Applications/Chromium.app"),
    (BrowserFamily::Edge, "/Applications/Microsoft Edge.app"),
    (BrowserFamily::Brave, "/Applications/Brave Browser.app"),
    (BrowserFamily::Firefox, "/Applications/Firefox.app"),
];

/// The preference name of a Firefox policy key, and the value `defaults write` takes.
const FIREFOX_PREFS: [(&str, &str, &str); 2] = [
    (FIREFOX_ENABLED, FIREFOX_ENABLED_PREF, "FALSE"),
    (FIREFOX_LOCKED, FIREFOX_LOCKED_PREF, "TRUE"),
];

#[derive(Debug, Default, Clone, Copy)]
pub struct MacosBackend;

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|arg| (*arg).to_owned()).collect()
}

fn made(key: &str) -> PolicyCreated {
    PolicyCreated {
        browser: BrowserFamily::Firefox,
        key: key.to_owned(),
    }
}

impl MacosBackend {
    fn run(&self, runner: &dyn Runner, program: &str, list: &[&str]) -> Option<super::Output> {
        runner.run(program, &args(list), &self.child_env()).ok()
    }

    /// Runs a command that must succeed.
    fn must(&self, runner: &dyn Runner, program: &str, list: &[&str]) -> Result<(), String> {
        match self.run(runner, program, list) {
            Some(output) if output.success() => Ok(()),
            _ => Err(reason::FAILED.to_owned()),
        }
    }

    /// The top-level keys of Firefox's policy domain; empty when the domain does not
    /// exist (`defaults` then exits 1).
    fn firefox_domain(&self, runner: &dyn Runner) -> Option<Vec<(String, Value)>> {
        let output = self.run(runner, DEFAULTS, &["read", FIREFOX_DOMAIN])?;
        Some(if output.success() {
            parse::defaults_domain(&output.stdout)
        } else {
            Vec::new()
        })
    }
}

impl OsBackend for MacosBackend {
    fn os(&self) -> Os {
        Os::Macos
    }

    fn hosts_path(&self) -> PathBuf {
        PathBuf::from(HOSTS_PATH)
    }

    fn state_dir(&self) -> PathBuf {
        PathBuf::from(state_dir::STATE_DIR_MACOS)
    }

    fn child_env(&self) -> Vec<(String, String)> {
        vec![
            (
                "PATH".to_owned(),
                "/usr/bin:/bin:/usr/sbin:/sbin".to_owned(),
            ),
            ("LANG".to_owned(), "C".to_owned()),
        ]
    }

    fn elevation_tool(&self) -> String {
        OSASCRIPT.to_owned()
    }

    fn observe_resolver(
        &self,
        runner: &dyn Runner,
        _fs: &dyn Fs,
        _full: bool,
    ) -> ResolverObservation {
        let unreadable = || ResolverObservation {
            full: true,
            error: Some(reason::UNREADABLE.to_owned()),
            ..ResolverObservation::default()
        };
        let listed = match self.run(runner, NETWORKSETUP, &["-listallnetworkservices"]) {
            Some(output) if output.success() => parse::network_services(&output.stdout),
            _ => return unreadable(),
        };
        let mut interfaces = Vec::new();
        for (name, active) in listed {
            let Some(output) = self.run(runner, NETWORKSETUP, &["-getdnsservers", &name]) else {
                return unreadable();
            };
            interfaces.push(InterfaceDns {
                id: name.clone(),
                name,
                active,
                servers: parse::ip_lines(&output.stdout),
            });
        }
        ResolverObservation {
            interfaces,
            full: true,
            ..ResolverObservation::default()
        }
    }

    fn observe_policies(&self, runner: &dyn Runner, fs: &dyn Fs) -> Vec<PolicyObservation> {
        let mut observed = Vec::new();
        for (browser, app) in APPLICATIONS {
            if !fs.exists(Path::new(app)) {
                continue;
            }
            let mut observation = PolicyObservation {
                browser,
                present: true,
                values: Vec::new(),
                foreign: Vec::new(),
                error: None,
            };
            if browser == BrowserFamily::Firefox {
                match self.firefox_domain(runner) {
                    None => observation.error = Some(reason::UNREADABLE.to_owned()),
                    Some(domain) => {
                        let find = |pref: &str| {
                            domain
                                .iter()
                                .find(|(key, _)| key == pref)
                                .map(|(_, value)| value)
                        };
                        // Policies switched off by somebody else: not ours to switch on.
                        let off = |value: &Value| policies::same_value(value, &Value::from(0));
                        if find(FIREFOX_SWITCH).is_some_and(off) {
                            observation.error = Some(reason::CONFLICT.to_owned());
                        }
                        for (key, pref, _) in FIREFOX_PREFS {
                            if let Some(value) = find(pref) {
                                observation.values.push(PolicyValue {
                                    key: key.to_owned(),
                                    value: value.clone(),
                                });
                            }
                        }
                    }
                }
            }
            observed.push(observation);
        }
        observed
    }

    fn apply_resolver(
        &self,
        id: ResolverId,
        observed: &ResolverObservation,
        runner: &dyn Runner,
        _fs: &dyn Fs,
    ) -> Result<(), String> {
        let addresses = resolvers::lookup(id).addresses();
        for service in observed.interfaces.iter().filter(|service| service.active) {
            let mut list = vec!["-setdnsservers", service.id.as_str()];
            list.extend(addresses);
            self.must(runner, NETWORKSETUP, &list)?;
        }
        Ok(())
    }

    fn remove_resolver(
        &self,
        observed: &SystemObservation,
        runner: &dyn Runner,
        _fs: &dyn Fs,
    ) -> Result<bool, String> {
        let mut undone = false;
        for row in RESOLVERS {
            for service in plan::reset_targets(observed, row.id) {
                self.must(
                    runner,
                    NETWORKSETUP,
                    &["-setdnsservers", &service, DNS_EMPTY],
                )?;
                undone = true;
            }
        }
        Ok(undone)
    }

    fn apply_policies(
        &self,
        observed: &[PolicyObservation],
        created: &[PolicyCreated],
        runner: &dyn Runner,
        _fs: &dyn Fs,
    ) -> Result<Vec<PolicyCreated>, String> {
        let mut created = created.to_vec();
        let conflicts = policies::conflicts(observed, &created);
        let firefox = observed.iter().find(|browser| {
            browser.browser == BrowserFamily::Firefox
                && browser.present
                && browser.error.is_none()
                && !conflicts.contains(&BrowserFamily::Firefox)
        });
        let Some(firefox) = firefox else {
            return Ok(created);
        };
        let domain = self.firefox_domain(runner).ok_or(reason::FAILED)?;
        if !domain.iter().any(|(key, _)| key == FIREFOX_SWITCH) {
            self.must(
                runner,
                DEFAULTS,
                &["write", FIREFOX_DOMAIN, FIREFOX_SWITCH, "-bool", "TRUE"],
            )?;
            created.push(made(FIREFOX_SWITCH));
        }
        for (key, pref, value) in FIREFOX_PREFS {
            if firefox.values.iter().any(|found| found.key == key) {
                continue;
            }
            self.must(
                runner,
                DEFAULTS,
                &["write", FIREFOX_DOMAIN, pref, "-bool", value],
            )?;
            created.push(made(key));
        }
        created.dedup();
        Ok(created)
    }

    fn remove_policies(
        &self,
        observed: &[PolicyObservation],
        created: &[PolicyCreated],
        runner: &dyn Runner,
        _fs: &dyn Fs,
    ) -> Result<bool, String> {
        if !created
            .iter()
            .any(|made| made.browser == BrowserFamily::Firefox)
        {
            return Ok(false);
        }
        let ours = |key: &str| created.contains(&made(key));
        let wanted = policies::desired(BrowserFamily::Firefox);
        let found = observed
            .iter()
            .find(|browser| browser.browser == BrowserFamily::Firefox);
        let mut deleted = Vec::new();
        for (key, pref, _) in FIREFOX_PREFS {
            // Only a value this helper created and that still is what it wrote.
            let still_ours = found.is_some_and(|browser| {
                browser.values.iter().any(|value| {
                    value.key == key
                        && wanted.iter().any(|want| {
                            want.key == key && policies::same_value(&want.value, &value.value)
                        })
                })
            });
            if ours(key) && still_ours {
                self.must(runner, DEFAULTS, &["delete", FIREFOX_DOMAIN, pref])?;
                deleted.push(pref);
            }
        }
        // The switch goes last, and only when nothing else is left for it to switch on.
        let domain = self.firefox_domain(runner).ok_or(reason::FAILED)?;
        let alone = domain.len() == 1 && domain[0].0 == FIREFOX_SWITCH;
        if ours(FIREFOX_SWITCH) && alone {
            self.must(
                runner,
                DEFAULTS,
                &["delete", FIREFOX_DOMAIN, FIREFOX_SWITCH],
            )?;
            deleted.push(FIREFOX_SWITCH);
        }
        Ok(!deleted.is_empty())
    }

    fn flush_dns(&self, runner: &dyn Runner) {
        let _ = self.run(runner, DSCACHEUTIL, &["-flushcache"]);
        let _ = self.run(runner, KILLALL, &["-HUP", "mDNSResponder"]);
    }
}
