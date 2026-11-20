//! Installed applications: scans the usual application folders for `*.app` bundles and reads
//! each bundle's `Info.plist` through `plutil` (no plist parser dependency). The scan spawns
//! one `plutil` per bundle, so the result is cached for a minute.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use ubiqx_core::ports::AppCatalog;
use ubiqx_core::{CoreResult, InstalledApp};

/// How long a scan is reused before the folders are read again.
pub const CACHE_TTL: Duration = Duration::from_secs(60);

/// How deep below each root the scan looks (`/Applications/Utilities/X.app` is depth 2).
const MAX_DEPTH: usize = 2;

/// The folders scanned, in order; `~/Applications` is appended when the home is known.
fn roots() -> Vec<PathBuf> {
    let mut roots = vec![
        PathBuf::from("/Applications"),
        PathBuf::from("/Applications/Utilities"),
        PathBuf::from("/System/Applications"),
        PathBuf::from("/System/Applications/Utilities"),
    ];
    if let Some(home) = std::env::var_os("HOME") {
        roots.push(PathBuf::from(home).join("Applications"));
    }
    roots
}

/// What `Info.plist` says about a bundle, once converted to JSON.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleInfo {
    pub bundle_id: String,
    pub name: Option<String>,
}

/// Reads the identifier and the display/bundle name from the JSON form of an `Info.plist`.
/// `None` when the bundle has no identifier (helper bundles, broken installs).
pub fn parse_info_plist_json(json: &str) -> Option<BundleInfo> {
    let value: serde_json::Value = serde_json::from_str(json).ok()?;
    let string = |key: &str| {
        value
            .get(key)
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
    };
    let bundle_id = string("CFBundleIdentifier")?;
    let name = string("CFBundleDisplayName").or_else(|| string("CFBundleName"));
    Some(BundleInfo { bundle_id, name })
}

/// Runs `plutil -convert json -o - <Info.plist>` and parses the output.
fn read_bundle_info(app: &Path) -> Option<BundleInfo> {
    let plist = app.join("Contents").join("Info.plist");
    if !plist.is_file() {
        return None;
    }
    let out = Command::new("/usr/bin/plutil")
        .args(["-convert", "json", "-o", "-"])
        .arg(&plist)
        .output()
        .ok()?;
    if !out.status.success() {
        tracing::debug!(path = %plist.display(), "plutil failed");
        return None;
    }
    parse_info_plist_json(&String::from_utf8_lossy(&out.stdout))
}

/// The bundle's name when `Info.plist` has none: the file name without `.app`.
fn stem_name(app: &Path) -> String {
    app.file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default()
}

/// Collects every `*.app` under `dir`, at most `depth` levels deep (bundles are not entered).
fn collect_bundles(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if path.extension().is_some_and(|e| e == "app") {
            out.push(path);
        } else if depth > 1 {
            collect_bundles(&path, depth - 1, out);
        }
    }
}

/// Scans the roots now: one entry per bundle identifier (the first path wins), sorted by
/// name (case-insensitive), then by bundle id.
pub fn scan() -> Vec<InstalledApp> {
    let mut bundles = Vec::new();
    for root in roots() {
        collect_bundles(&root, MAX_DEPTH, &mut bundles);
    }
    let mut apps: Vec<InstalledApp> = Vec::with_capacity(bundles.len());
    for path in bundles {
        let Some(info) = read_bundle_info(&path) else {
            continue;
        };
        if apps.iter().any(|a| a.bundle_id == info.bundle_id) {
            continue;
        }
        apps.push(InstalledApp {
            name: info.name.unwrap_or_else(|| stem_name(&path)),
            bundle_id: info.bundle_id,
            path: path.to_string_lossy().to_string(),
        });
    }
    sort_apps(&mut apps);
    apps
}

/// Name order, case-insensitive, with the bundle id as a stable tie-break.
pub fn sort_apps(apps: &mut [InstalledApp]) {
    apps.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.bundle_id.cmp(&b.bundle_id))
    });
}

/// [`AppCatalog`] over the application folders, cached for [`CACHE_TTL`].
#[derive(Debug, Default)]
pub struct MacAppCatalog {
    cache: Mutex<Option<(Instant, Vec<InstalledApp>)>>,
}

impl AppCatalog for MacAppCatalog {
    fn installed_apps(&self) -> CoreResult<Vec<InstalledApp>> {
        if let Some((at, apps)) = self.cache.lock().as_ref() {
            if at.elapsed() < CACHE_TTL {
                return Ok(apps.clone());
            }
        }
        let started = Instant::now();
        let apps = scan();
        tracing::debug!(
            count = apps.len(),
            ms = started.elapsed().as_millis(),
            "scanned installed applications"
        );
        *self.cache.lock() = Some((Instant::now(), apps.clone()));
        Ok(apps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plist_json() {
        let info = parse_info_plist_json(
            r#"{"CFBundleIdentifier":"com.google.Chrome","CFBundleName":"Chrome","CFBundleDisplayName":"Google Chrome"}"#,
        )
        .unwrap();
        assert_eq!(info.bundle_id, "com.google.Chrome");
        assert_eq!(info.name.as_deref(), Some("Google Chrome"));
        let info =
            parse_info_plist_json(r#"{"CFBundleIdentifier":"x.y","CFBundleName":"Y"}"#).unwrap();
        assert_eq!(info.name.as_deref(), Some("Y"));
        assert!(parse_info_plist_json(r#"{"CFBundleName":"No id"}"#).is_none());
        assert!(parse_info_plist_json("not json").is_none());
    }

    #[test]
    fn sorts_by_name_case_insensitively() {
        let mut apps = vec![
            InstalledApp {
                name: "xcode".into(),
                bundle_id: "b".into(),
                path: String::new(),
            },
            InstalledApp {
                name: "Discord".into(),
                bundle_id: "a".into(),
                path: String::new(),
            },
        ];
        sort_apps(&mut apps);
        assert_eq!(apps[0].name, "Discord");
        assert_eq!(stem_name(Path::new("/Applications/Slack.app")), "Slack");
    }
}
