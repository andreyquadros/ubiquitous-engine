//! Browser policies on Linux: our own file `ubiqx-shield.json` in each Chromium-family
//! browser's `managed` directory, and Firefox's `policies.json`, created only when absent.
//! Somebody else's file is never edited: a value of theirs that differs from ours is
//! reported as `foreign`, a Firefox `policies.json` that is not ours as `conflict`.
//!
//! The Firefox snap reads `/etc/firefox/policies/policies.json` like the `.deb`, and the
//! Chromium snap reads `/etc/chromium-browser/policies/managed/`, so both are covered. A
//! browser installed only as a Flatpak is out of reach.

use std::io;
use std::path::{Path, PathBuf};

use ubiqx_core::nofap::{reason, BrowserFamily, Os, PolicyCreated, PolicyObservation, PolicyValue};
use ubiqx_nofap::policies::{self, OWN_FILE_NAME};

use super::linux::{create_dirs, FILE_MODE};
use super::{join, parent, parse, Fs};

/// Where one browser family is installed and where it reads policies.
pub struct BrowserPaths {
    pub browser: BrowserFamily,
    /// The `managed` directories it reads (one per packaging).
    pub managed: &'static [&'static str],
    /// Paths that exist when the browser is installed from a package or a snap.
    pub installed: &'static [&'static str],
    /// The system-wide Flatpak application directory.
    pub flatpak: &'static str,
}

pub const CHROMIUM_FAMILY: [BrowserPaths; 4] = [
    BrowserPaths {
        browser: BrowserFamily::Chrome,
        managed: &["/etc/opt/chrome/policies/managed"],
        installed: &["/opt/google/chrome", "/usr/bin/google-chrome"],
        flatpak: "/var/lib/flatpak/app/com.google.Chrome",
    },
    BrowserPaths {
        browser: BrowserFamily::Chromium,
        managed: &[
            "/etc/chromium/policies/managed",
            "/etc/chromium-browser/policies/managed",
        ],
        installed: &[
            "/usr/lib/chromium",
            "/usr/bin/chromium",
            "/usr/bin/chromium-browser",
            "/snap/bin/chromium",
        ],
        flatpak: "/var/lib/flatpak/app/org.chromium.Chromium",
    },
    BrowserPaths {
        browser: BrowserFamily::Edge,
        managed: &["/etc/opt/edge/policies/managed"],
        installed: &["/opt/microsoft/msedge", "/usr/bin/microsoft-edge"],
        flatpak: "/var/lib/flatpak/app/com.microsoft.Edge",
    },
    BrowserPaths {
        browser: BrowserFamily::Brave,
        managed: &["/etc/brave/policies/managed"],
        installed: &["/opt/brave.com/brave", "/usr/bin/brave-browser"],
        flatpak: "/var/lib/flatpak/app/com.brave.Browser",
    },
];

pub const FIREFOX_POLICIES: &str = "/etc/firefox/policies/policies.json";
pub const FIREFOX_INSTALLED: [&str; 5] = [
    "/usr/lib/firefox",
    "/usr/lib64/firefox",
    "/usr/lib/firefox-esr",
    "/usr/bin/firefox",
    "/snap/bin/firefox",
];
pub const FIREFOX_FLATPAK: &str = "/var/lib/flatpak/app/org.mozilla.firefox";

fn any_exists(fs: &dyn Fs, paths: &[&str]) -> bool {
    paths.iter().any(|path| fs.exists(Path::new(path)))
}

fn own_file(dir: &str) -> PathBuf {
    join(Os::Linux, Path::new(dir), OWN_FILE_NAME)
}

fn keys_of(browser: BrowserFamily) -> Vec<String> {
    policies::desired(browser)
        .into_iter()
        .map(|want| want.key)
        .collect()
}

fn read_text(fs: &dyn Fs, path: &Path) -> io::Result<String> {
    fs.read(path)
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
}

/// The managed keys set by policy files in `dir` that are not ours.
fn foreign_values(fs: &dyn Fs, dir: &str, keys: &[String]) -> Vec<PolicyValue> {
    let mut found = Vec::new();
    for name in fs.list(Path::new(dir)).unwrap_or_default() {
        if name == OWN_FILE_NAME || !name.ends_with(".json") {
            continue;
        }
        let file = join(Os::Linux, Path::new(dir), &name);
        if let Some(values) = read_text(fs, &file)
            .ok()
            .and_then(|text| parse::policy_file_values(&text, keys))
        {
            found.extend(values);
        }
    }
    found
}

fn observe_chromium(fs: &dyn Fs, paths: &BrowserPaths) -> Option<PolicyObservation> {
    let native = any_exists(fs, paths.installed) || any_exists(fs, paths.managed);
    let flatpak = fs.exists(Path::new(paths.flatpak));
    if !native && !flatpak {
        return None;
    }
    let keys = keys_of(paths.browser);
    let mut observation = PolicyObservation {
        browser: paths.browser,
        present: true,
        values: Vec::new(),
        foreign: Vec::new(),
        error: (!native).then(|| reason::FLATPAK.to_owned()),
    };
    // Applied means: our file is in every directory this family reads.
    let mut ours: Option<Vec<PolicyValue>> = None;
    let mut everywhere = true;
    for dir in paths.managed {
        observation.foreign.extend(foreign_values(fs, dir, &keys));
        match read_text(fs, &own_file(dir)) {
            Ok(text) => {
                let values = parse::policy_file_values(&text, &keys).unwrap_or_default();
                ours.get_or_insert(values);
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => everywhere = false,
            Err(_) => observation.error = Some(reason::UNREADABLE.to_owned()),
        }
    }
    if everywhere {
        observation.values = ours.unwrap_or_default();
    }
    Some(observation)
}

fn observe_firefox(fs: &dyn Fs) -> Option<PolicyObservation> {
    let file = Path::new(FIREFOX_POLICIES);
    let native = any_exists(fs, &FIREFOX_INSTALLED) || fs.exists(file);
    let flatpak = fs.exists(Path::new(FIREFOX_FLATPAK));
    if !native && !flatpak {
        return None;
    }
    let mut observation = PolicyObservation {
        browser: BrowserFamily::Firefox,
        present: true,
        values: Vec::new(),
        foreign: Vec::new(),
        error: (!native).then(|| reason::FLATPAK.to_owned()),
    };
    match read_text(fs, file) {
        Ok(text) if text == policies::firefox_json() => {
            observation.values = policies::desired(BrowserFamily::Firefox);
        }
        // A `policies.json` that is not ours: somebody manages this Firefox.
        Ok(_) => observation.error = Some(reason::CONFLICT.to_owned()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(_) => observation.error = Some(reason::UNREADABLE.to_owned()),
    }
    Some(observation)
}

/// The policy locations of every browser that is installed.
pub(crate) fn observe(fs: &dyn Fs) -> Vec<PolicyObservation> {
    let mut observed: Vec<PolicyObservation> = CHROMIUM_FAMILY
        .iter()
        .filter_map(|paths| observe_chromium(fs, paths))
        .collect();
    observed.extend(observe_firefox(fs));
    observed
}

fn write_new(fs: &dyn Fs, file: &Path, text: &str) -> Result<(), String> {
    let failed = |_| reason::FAILED.to_owned();
    if let Some(dir) = parent(Os::Linux, file) {
        create_dirs(fs, &dir).map_err(failed)?;
    }
    fs.write_atomic(file, text.as_bytes(), Some(FILE_MODE))
        .map_err(failed)
}

/// Writes our files for the browsers that are present, can be managed and hold nobody
/// else's choice. Everything in a file of ours is ours, so every key is listed as created.
pub(crate) fn apply(
    fs: &dyn Fs,
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
        if family == BrowserFamily::Firefox {
            // Created only when absent; an equal file is already ours.
            if !fs.exists(Path::new(FIREFOX_POLICIES)) {
                write_new(fs, Path::new(FIREFOX_POLICIES), &policies::firefox_json())?;
            }
        } else if let Some(paths) = CHROMIUM_FAMILY.iter().find(|paths| paths.browser == family) {
            let text = policies::chromium_json(family);
            for dir in paths.managed {
                write_new(fs, &own_file(dir), &text)?;
            }
        }
        for key in keys_of(family) {
            let made = PolicyCreated {
                browser: family,
                key,
            };
            if !created.contains(&made) {
                created.push(made);
            }
        }
    }
    Ok(created)
}

/// Deletes our own files: each `ubiqx-shield.json`, and a Firefox `policies.json` that
/// still is, byte for byte, the one this helper writes. `Ok(true)` when one was deleted.
pub(crate) fn remove(fs: &dyn Fs) -> Result<bool, String> {
    let mut ours: Vec<PathBuf> = CHROMIUM_FAMILY
        .iter()
        .flat_map(|paths| paths.managed.iter().map(|dir| own_file(dir)))
        .collect();
    let firefox = Path::new(FIREFOX_POLICIES);
    if read_text(fs, firefox).is_ok_and(|text| text == policies::firefox_json()) {
        ours.push(firefox.to_path_buf());
    }
    let mut deleted = false;
    for file in ours {
        match fs.remove_file(&file) {
            Ok(()) => deleted = true,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(_) => return Err(reason::FAILED.to_owned()),
        }
    }
    Ok(deleted)
}
