//! Browser policies on Windows: named values under each browser's own key in
//! `HKLM\SOFTWARE\Policies`, read and written with `reg.exe`. A value that is already
//! there and is not ours is never overwritten, and undo deletes only values this helper
//! created that still equal what it wrote.

use std::path::Path;

use serde_json::Value;
use ubiqx_core::nofap::{reason, BrowserFamily, PolicyCreated, PolicyObservation, PolicyValue};
use ubiqx_nofap::policies::{self, FIREFOX_POLICY};

use super::windows::WindowsBackend;
use super::{parse, Fs, OsBackend, Runner};

/// The policy key of each browser family.
pub const POLICY_KEYS: [(BrowserFamily, &str); 5] = [
    (
        BrowserFamily::Chrome,
        "HKLM\\SOFTWARE\\Policies\\Google\\Chrome",
    ),
    (
        BrowserFamily::Chromium,
        "HKLM\\SOFTWARE\\Policies\\Chromium",
    ),
    (
        BrowserFamily::Edge,
        "HKLM\\SOFTWARE\\Policies\\Microsoft\\Edge",
    ),
    (
        BrowserFamily::Brave,
        "HKLM\\SOFTWARE\\Policies\\BraveSoftware\\Brave",
    ),
    (
        BrowserFamily::Firefox,
        "HKLM\\SOFTWARE\\Policies\\Mozilla\\Firefox\\DNSOverHTTPS",
    ),
];

/// Where each browser's executable is under `<ProgramFiles>` or its 32-bit twin.
pub const EXECUTABLES: [(BrowserFamily, &str); 5] = [
    (
        BrowserFamily::Chrome,
        "Google\\Chrome\\Application\\chrome.exe",
    ),
    (BrowserFamily::Chromium, "Chromium\\Application\\chrome.exe"),
    (
        BrowserFamily::Edge,
        "Microsoft\\Edge\\Application\\msedge.exe",
    ),
    (
        BrowserFamily::Brave,
        "BraveSoftware\\Brave-Browser\\Application\\brave.exe",
    ),
    (BrowserFamily::Firefox, "Mozilla Firefox\\firefox.exe"),
];
/// Suffix of the 32-bit program directory.
pub const X86_SUFFIX: &str = " (x86)";

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|arg| (*arg).to_owned()).collect()
}

fn key_of(browser: BrowserFamily) -> &'static str {
    POLICY_KEYS
        .iter()
        .find(|(family, _)| *family == browser)
        .map_or("", |(_, key)| key)
}

/// The registry value name of a policy key: Firefox's pair is flattened with a dot in
/// `PolicyValue` keys and lives as two values under the policy's own sub-key.
fn value_name(key: &str) -> &str {
    key.strip_prefix(FIREFOX_POLICY)
        .and_then(|rest| rest.strip_prefix('.'))
        .unwrap_or(key)
}

fn installed(backend: &WindowsBackend, fs: &dyn Fs, browser: BrowserFamily) -> bool {
    let base = backend.program_files();
    EXECUTABLES
        .iter()
        .filter(|(family, _)| *family == browser)
        .any(|(_, exe)| {
            [
                format!("{base}\\{exe}"),
                format!("{base}{X86_SUFFIX}\\{exe}"),
            ]
            .iter()
            .any(|path| fs.exists(Path::new(path)))
        })
}

/// The values of every browser that is installed or already has a policy key.
pub(crate) fn observe(
    backend: &WindowsBackend,
    runner: &dyn Runner,
    fs: &dyn Fs,
) -> Vec<PolicyObservation> {
    let mut observed = Vec::new();
    for (browser, key) in POLICY_KEYS {
        let ran = runner.run(&backend.reg(), &args(&["query", key]), &backend.child_env());
        let has_key = ran.as_ref().is_ok_and(|output| output.success());
        if !has_key && !installed(backend, fs, browser) {
            continue;
        }
        let mut observation = PolicyObservation {
            browser,
            present: true,
            values: Vec::new(),
            foreign: Vec::new(),
            error: ran.is_err().then(|| reason::UNREADABLE.to_owned()),
        };
        let found = ran
            .map(|output| parse::reg_values(&output.stdout))
            .unwrap_or_default();
        for want in policies::desired(browser) {
            let value = found
                .iter()
                .find(|value| value.name.eq_ignore_ascii_case(value_name(&want.key)))
                .and_then(parse::reg_json);
            if let Some(value) = value {
                observation.values.push(PolicyValue {
                    key: want.key,
                    value,
                });
            }
        }
        observed.push(observation);
    }
    observed
}

/// The type and data `reg.exe add` takes for one of our values.
fn reg_data(value: &Value) -> Option<(&'static str, String)> {
    match value {
        Value::String(text) => Some(("REG_SZ", text.clone())),
        Value::Bool(flag) => Some(("REG_DWORD", u8::from(*flag).to_string())),
        Value::Number(number) => Some(("REG_DWORD", number.as_u64()?.to_string())),
        _ => None,
    }
}

fn reg(backend: &WindowsBackend, runner: &dyn Runner, list: &[&str]) -> Result<(), String> {
    match runner.run(&backend.reg(), &args(list), &backend.child_env()) {
        Ok(output) if output.success() => Ok(()),
        _ => Err(reason::FAILED.to_owned()),
    }
}

/// Adds the values that are missing for the browsers that are present, readable and hold
/// nobody else's choice. A value is listed as created only when this run wrote it.
pub(crate) fn apply(
    backend: &WindowsBackend,
    runner: &dyn Runner,
    observed: &[PolicyObservation],
    created: &[PolicyCreated],
) -> Result<Vec<PolicyCreated>, String> {
    let mut created = created.to_vec();
    let conflicts = policies::conflicts(observed, &created);
    for browser in observed {
        let family = browser.browser;
        if !browser.present || browser.error.is_some() || conflicts.contains(&family) {
            continue;
        }
        for want in policies::desired(family) {
            if browser.values.iter().any(|found| found.key == want.key) {
                continue;
            }
            let (kind, data) = reg_data(&want.value).ok_or(reason::FAILED)?;
            let name = value_name(&want.key);
            let list = [
                "add",
                key_of(family),
                "/v",
                name,
                "/t",
                kind,
                "/d",
                &data,
                "/f",
            ];
            reg(backend, runner, &list)?;
            let made = PolicyCreated {
                browser: family,
                key: want.key,
            };
            if !created.contains(&made) {
                created.push(made);
            }
        }
    }
    Ok(created)
}

/// Deletes the values `created` lists that still equal ours. The names come from our own
/// table: a key the state file names and the table does not is ignored.
pub(crate) fn remove(
    backend: &WindowsBackend,
    runner: &dyn Runner,
    observed: &[PolicyObservation],
    created: &[PolicyCreated],
) -> Result<bool, String> {
    let mut deleted = false;
    for browser in observed {
        let family = browser.browser;
        for want in policies::desired(family) {
            let ours = created
                .iter()
                .any(|made| made.browser == family && made.key == want.key);
            let unchanged = browser.values.iter().any(|found| {
                found.key == want.key && policies::same_value(&found.value, &want.value)
            });
            if ours && unchanged {
                let list = ["delete", key_of(family), "/v", value_name(&want.key), "/f"];
                reg(backend, runner, &list)?;
                deleted = true;
            }
        }
    }
    Ok(deleted)
}
