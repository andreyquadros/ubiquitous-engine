//! Application updates: the identity of the running build, the feed a CI publishes next to
//! every DMG, and the pure decision "is that feed an update for me?".
//!
//! Nothing here does I/O. Fetching the feed is the [`UpdateFeedSource`] port; the engine's
//! checker (`ubiqx_engine::update`) runs the comparison, remembers what was already announced
//! and tells the shells. The JSON shapes are shared with the desktop frontend and the publish
//! script, so the field names below are a contract: snake_case, exactly as serde emits them.
//!
//! [`UpdateFeedSource`]: crate::ports::UpdateFeedSource

use std::cmp::Ordering;
use std::collections::BTreeMap;

use chrono::{DateTime, Local, TimeZone, Utc};
use serde::{Deserialize, Serialize};

use crate::lang::UiLanguage;

/// Where the desktop app looks for the feed when nothing else is configured: the rolling
/// GitHub release `continuous` of the product's repository.
pub const DEFAULT_FEED_URL: &str =
    "https://github.com/andreyquadros/ubiquitous-engine/releases/download/continuous/latest.json";

/// Key/value keys the checker uses to remember what it already did across restarts.
pub mod kv_keys {
    /// The build epoch the user was last notified about (one notification per build).
    pub const NOTIFIED_EPOCH: &str = "update.notified_epoch";
    /// The build epoch the user dismissed from the banner.
    pub const DISMISSED_EPOCH: &str = "update.dismissed_epoch";
}

// ---------------------------------------------------------------------------------------------
// Build identity
// ---------------------------------------------------------------------------------------------

/// Identity of a build, stamped into the desktop binary at compile time (see the desktop
/// `build.rs`) and written into the feed by the publish script. Two builds compare by
/// `version` first and, when equal, by the commit `epoch`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildInfo {
    /// `CARGO_PKG_VERSION`, e.g. `0.1.0`.
    pub version: String,
    /// Unix seconds of the built commit (`git log -1 --format=%ct`); `0` = development build.
    pub epoch: i64,
    /// `git rev-list --count HEAD`; `0` in development.
    pub number: u64,
    /// Short (7-character) commit sha; `dev` when unknown.
    pub sha: String,
    /// Branch name; empty when unknown.
    pub branch: String,
}

impl BuildInfo {
    /// A development build: the crate version, no commit identity.
    pub fn dev() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            epoch: 0,
            number: 0,
            sha: "dev".to_string(),
            branch: String::new(),
        }
    }

    /// Builds the identity from the raw strings a build script stamps into the binary
    /// (`UBIQX_BUILD_EPOCH`, `UBIQX_BUILD_NUMBER`, `UBIQX_BUILD_SHA`, `UBIQX_BUILD_BRANCH`).
    /// Blank or unparsable numbers mean `0`; a blank sha means `dev`.
    pub fn from_stamp(version: &str, epoch: &str, number: &str, sha: &str, branch: &str) -> Self {
        let sha = sha.trim();
        Self {
            version: version.trim().to_string(),
            epoch: epoch.trim().parse().unwrap_or(0),
            number: number.trim().parse().unwrap_or(0),
            sha: if sha.is_empty() {
                "dev".to_string()
            } else {
                sha.to_string()
            },
            branch: branch.trim().to_string(),
        }
    }

    /// Whether this is a development build (no commit epoch), which never checks
    /// automatically.
    pub fn is_dev(&self) -> bool {
        self.epoch == 0
    }
}

impl Default for BuildInfo {
    fn default() -> Self {
        Self::dev()
    }
}

// ---------------------------------------------------------------------------------------------
// The feed (latest.json)
// ---------------------------------------------------------------------------------------------

/// The `build` object of the feed. Same fields as [`BuildInfo`] minus the version, which
/// lives at the top level of the feed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FeedBuild {
    pub epoch: i64,
    #[serde(default)]
    pub number: u64,
    #[serde(default)]
    pub sha: String,
    #[serde(default)]
    pub branch: String,
}

/// One downloadable artifact of the feed, keyed by target (`darwin-aarch64`, …).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformAsset {
    /// The DMG (or whatever `kind` says) to download.
    pub url: String,
    /// `dmg` today.
    #[serde(default = "default_kind")]
    pub kind: String,
    /// Size in bytes, when the publisher knows it.
    #[serde(default)]
    pub size: Option<u64>,
    /// A zipped `.app` next to the DMG, when published.
    #[serde(default)]
    pub app_zip_url: Option<String>,
}

fn default_kind() -> String {
    "dmg".to_string()
}

/// `latest.json`, as attached to the rolling release by the publish script.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateFeed {
    #[serde(default = "default_schema")]
    pub schema: u32,
    #[serde(default)]
    pub product: String,
    pub version: String,
    pub build: FeedBuild,
    #[serde(default)]
    pub published_at: Option<DateTime<Utc>>,
    /// Subject and body of the built commit (a few dozen lines at most).
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub release_url: Option<String>,
    #[serde(default)]
    pub platforms: BTreeMap<String, PlatformAsset>,
}

fn default_schema() -> u32 {
    1
}

impl UpdateFeed {
    /// The build identity the feed describes.
    pub fn build_info(&self) -> BuildInfo {
        BuildInfo {
            version: self.version.clone(),
            epoch: self.build.epoch,
            number: self.build.number,
            sha: self.build.sha.clone(),
            branch: self.build.branch.clone(),
        }
    }

    /// Whether the feed describes a build newer than `current` (see [`is_newer`]).
    pub fn is_update_for(&self, current: &BuildInfo) -> bool {
        is_newer(&self.version, self.build.epoch, current)
    }

    /// The release for `target` (see [`pick_platform`]), or `None` when the feed has no
    /// artifact this machine can install.
    pub fn release_for(&self, target: &str) -> Option<ReleaseInfo> {
        let (_, asset) = pick_platform(&self.platforms, target)?;
        Some(ReleaseInfo {
            version: self.version.clone(),
            build: self.build_info(),
            published_at: self.published_at,
            notes: self.notes.clone(),
            download_url: asset.url.clone(),
            app_zip_url: asset.app_zip_url.clone(),
            release_url: self.release_url.clone(),
            kind: asset.kind.clone(),
        })
    }
}

// ---------------------------------------------------------------------------------------------
// What the shells see
// ---------------------------------------------------------------------------------------------

/// A downloadable newer build, already resolved for this machine's target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseInfo {
    pub version: String,
    pub build: BuildInfo,
    pub published_at: Option<DateTime<Utc>>,
    pub notes: String,
    pub download_url: String,
    pub app_zip_url: Option<String>,
    pub release_url: Option<String>,
    /// `dmg`.
    pub kind: String,
}

/// The state of the update checker, as read by the UI and the tray.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateStatus {
    pub current: BuildInfo,
    pub feed_url: String,
    /// Automatic checks run at all (false for development builds; a manual check still
    /// works).
    pub enabled: bool,
    pub available: Option<ReleaseInfo>,
    /// The user dismissed exactly the build in `available`.
    pub dismissed: bool,
    pub last_check: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    /// A check is running right now.
    pub checking: bool,
}

impl UpdateStatus {
    /// The idle status of a fresh checker.
    pub fn initial(current: BuildInfo, feed_url: impl Into<String>) -> Self {
        Self {
            enabled: !current.is_dev(),
            current,
            feed_url: feed_url.into(),
            available: None,
            dismissed: false,
            last_check: None,
            last_error: None,
            checking: false,
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Decisions
// ---------------------------------------------------------------------------------------------

/// Compares two `x.y.z` versions numerically. Anything after `-` or `+` (pre-release or build
/// metadata) is ignored, missing components count as `0` and a non-numeric component as `0`,
/// so `1.2` == `1.2.0` and `0.2.0-beta.1` == `0.2.0`.
pub fn compare_versions(a: &str, b: &str) -> Ordering {
    let pa = version_components(a);
    let pb = version_components(b);
    let n = pa.len().max(pb.len());
    for i in 0..n {
        let x = pa.get(i).copied().unwrap_or(0);
        let y = pb.get(i).copied().unwrap_or(0);
        match x.cmp(&y) {
            Ordering::Equal => continue,
            other => return other,
        }
    }
    Ordering::Equal
}

fn version_components(v: &str) -> Vec<u64> {
    let core = v
        .trim()
        .trim_start_matches(['v', 'V'])
        .split(['-', '+'])
        .next()
        .unwrap_or("");
    core.split('.')
        .map(|c| c.trim().parse::<u64>().unwrap_or(0))
        .collect()
}

/// Whether a feed entry (`version`, commit `epoch`) is an update for `current`: a greater
/// version, or the same version built from a later commit.
pub fn is_newer(feed_version: &str, feed_epoch: i64, current: &BuildInfo) -> bool {
    match compare_versions(feed_version, &current.version) {
        Ordering::Greater => true,
        Ordering::Less => false,
        Ordering::Equal => feed_epoch > current.epoch,
    }
}

/// The feed key of this machine: `<os>-<arch>` with `darwin` for macOS
/// (`darwin-aarch64`, `darwin-x86_64`, `linux-x86_64`, …).
pub fn current_target() -> String {
    target_for(std::env::consts::OS, std::env::consts::ARCH)
}

/// [`current_target`] for the given `std::env::consts` values.
pub fn target_for(os: &str, arch: &str) -> String {
    let os = match os {
        "macos" => "darwin",
        other => other,
    };
    format!("{os}-{arch}")
}

/// Picks the artifact for `target`: the exact key, then `darwin-universal`, then any
/// `darwin-*` key (a universal DMG installs on either architecture). Only macOS targets fall
/// back; on other operating systems only an exact key matches, so a feed with macOS assets
/// alone yields `None` there.
pub fn pick_platform<'a>(
    platforms: &'a BTreeMap<String, PlatformAsset>,
    target: &str,
) -> Option<(&'a str, &'a PlatformAsset)> {
    if let Some((k, v)) = platforms.get_key_value(target) {
        return Some((k.as_str(), v));
    }
    if !target.starts_with("darwin-") {
        return None;
    }
    if let Some((k, v)) = platforms.get_key_value("darwin-universal") {
        return Some((k.as_str(), v));
    }
    platforms
        .iter()
        .find(|(k, _)| k.starts_with("darwin-"))
        .map(|(k, v)| (k.as_str(), v))
}

// ---------------------------------------------------------------------------------------------
// User-facing text
// ---------------------------------------------------------------------------------------------

/// The build epoch as a short local date and time: `18/09 15:04` in Portuguese,
/// `Sep 18 15:04` in English. Used by the notification and the tray label.
pub fn format_build_date(epoch: i64, lang: UiLanguage) -> String {
    format_build_date_in(epoch, lang, &Local)
}

/// [`format_build_date`] in an explicit time zone (tests).
pub fn format_build_date_in<Tz: TimeZone>(epoch: i64, lang: UiLanguage, tz: &Tz) -> String
where
    Tz::Offset: std::fmt::Display,
{
    let Some(t) = tz.timestamp_opt(epoch, 0).single() else {
        return String::new();
    };
    match lang {
        UiLanguage::PtBr => t.format("%d/%m %H:%M").to_string(),
        UiLanguage::En => t.format("%b %d %H:%M").to_string(),
    }
}

/// Title and body of the "new version" notification in `lang`.
pub fn notification_text(release: &ReleaseInfo, lang: UiLanguage) -> (String, String) {
    let date = format_build_date(release.build.epoch, lang);
    let sha = release.build.sha.as_str();
    match lang {
        UiLanguage::PtBr => (
            "Nova versão do ubiqX".to_string(),
            format!("Build {date} ({sha}) já está disponível. Abra o ubiqX para baixar."),
        ),
        UiLanguage::En => (
            "New ubiqX version".to_string(),
            format!("Build {date} ({sha}) is available. Open ubiqX to download it."),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::FixedOffset;

    const FEED: &str = r#"{ "schema": 1, "product": "ubiqX", "version": "0.1.0", "build": { "epoch": 1758221040, "number": 27, "sha": "14c6e7f", "branch": "main" }, "published_at": "2026-09-18T19:10:00Z", "notes": "feat: something\n\nbody", "release_url": "https://github.com/andreyquadros/ubiquitous-engine/releases/tag/continuous", "platforms": { "darwin-aarch64": { "url": "https://github.com/andreyquadros/ubiquitous-engine/releases/download/continuous/ubiqX-macos-aarch64.dmg", "kind": "dmg", "size": 12345678, "app_zip_url": "https://github.com/andreyquadros/ubiquitous-engine/releases/download/continuous/ubiqX-macos-aarch64.app.zip" } } }"#;

    fn current(version: &str, epoch: i64) -> BuildInfo {
        BuildInfo {
            version: version.into(),
            epoch,
            number: 26,
            sha: "abc1234".into(),
            branch: "main".into(),
        }
    }

    #[test]
    fn compares_versions_numerically() {
        assert_eq!(compare_versions("0.1.0", "0.1.0"), Ordering::Equal);
        assert_eq!(compare_versions("0.2.0", "0.10.0"), Ordering::Less);
        assert_eq!(compare_versions("1.0.0", "0.9.9"), Ordering::Greater);
        assert_eq!(compare_versions("1.2", "1.2.0"), Ordering::Equal);
        assert_eq!(compare_versions("0.2.0-beta.1", "0.2.0"), Ordering::Equal);
        assert_eq!(compare_versions("0.2.0+build.7", "0.2.0"), Ordering::Equal);
        assert_eq!(compare_versions("v0.3.0", "0.2.9"), Ordering::Greater);
        assert_eq!(compare_versions("", "0.0.0"), Ordering::Equal);
    }

    #[test]
    fn newer_by_version_then_by_epoch() {
        let cur = current("0.1.0", 1_758_200_000);
        assert!(
            is_newer("0.1.1", 0, &cur),
            "greater version wins even without epoch"
        );
        assert!(
            !is_newer("0.0.9", 9_999_999_999, &cur),
            "smaller version never updates"
        );
        assert!(
            is_newer("0.1.0", 1_758_221_040, &cur),
            "same version, later commit"
        );
        assert!(!is_newer("0.1.0", 1_758_200_000, &cur), "same build");
        assert!(!is_newer("0.1.0", 1_758_100_000, &cur), "older commit");
        // A development build (epoch 0) sees any published build of its version as newer.
        assert!(is_newer("0.1.0", 1, &BuildInfo::dev()));
        assert!(!is_newer("0.1.0", 0, &BuildInfo::dev()));
    }

    #[test]
    fn stamps_parse_leniently() {
        let b = BuildInfo::from_stamp("0.1.0", "1758221040", "27", "14c6e7f", "main");
        assert_eq!(
            b,
            BuildInfo {
                version: "0.1.0".into(),
                epoch: 1_758_221_040,
                number: 27,
                sha: "14c6e7f".into(),
                branch: "main".into(),
            }
        );
        let d = BuildInfo::from_stamp("0.1.0", "", "x", " ", "");
        assert_eq!(d.epoch, 0);
        assert_eq!(d.number, 0);
        assert_eq!(d.sha, "dev");
        assert_eq!(d.branch, "");
        assert!(d.is_dev());
        assert!(BuildInfo::dev().is_dev());
        assert!(!b.is_dev());
    }

    #[test]
    fn targets_follow_std_consts() {
        assert_eq!(target_for("macos", "aarch64"), "darwin-aarch64");
        assert_eq!(target_for("macos", "x86_64"), "darwin-x86_64");
        assert_eq!(target_for("linux", "x86_64"), "linux-x86_64");
        assert!(current_target().contains('-'));
    }

    #[test]
    fn platform_fallbacks() {
        let asset = |url: &str| PlatformAsset {
            url: url.into(),
            kind: "dmg".into(),
            size: None,
            app_zip_url: None,
        };
        let mut platforms = BTreeMap::new();
        platforms.insert("darwin-x86_64".to_string(), asset("x86"));
        // Any darwin key serves an aarch64 Mac when nothing closer exists.
        assert_eq!(
            pick_platform(&platforms, "darwin-aarch64").map(|(k, _)| k),
            Some("darwin-x86_64")
        );
        platforms.insert("darwin-universal".to_string(), asset("universal"));
        assert_eq!(
            pick_platform(&platforms, "darwin-aarch64").map(|(k, _)| k),
            Some("darwin-universal")
        );
        platforms.insert("darwin-aarch64".to_string(), asset("arm"));
        assert_eq!(
            pick_platform(&platforms, "darwin-aarch64").map(|(_, a)| a.url.as_str()),
            Some("arm")
        );
        // Other operating systems never take a macOS artifact.
        assert!(pick_platform(&platforms, "linux-x86_64").is_none());
        assert!(pick_platform(&BTreeMap::new(), "darwin-aarch64").is_none());
    }

    #[test]
    fn parses_the_feed_from_the_contract() {
        let feed: UpdateFeed = serde_json::from_str(FEED).unwrap();
        assert_eq!(feed.schema, 1);
        assert_eq!(feed.product, "ubiqX");
        assert_eq!(feed.version, "0.1.0");
        assert_eq!(feed.build.epoch, 1_758_221_040);
        assert_eq!(feed.build.number, 27);
        assert_eq!(feed.build.sha, "14c6e7f");
        assert_eq!(feed.build.branch, "main");
        assert_eq!(
            feed.published_at,
            Some(Utc.with_ymd_and_hms(2026, 9, 18, 19, 10, 0).unwrap())
        );
        assert!(feed.notes.starts_with("feat: something"));
        let asset = &feed.platforms["darwin-aarch64"];
        assert_eq!(asset.kind, "dmg");
        assert_eq!(asset.size, Some(12_345_678));
        assert!(asset.app_zip_url.as_deref().unwrap().ends_with(".app.zip"));

        let release = feed.release_for("darwin-aarch64").unwrap();
        assert_eq!(release.version, "0.1.0");
        assert_eq!(release.build.epoch, 1_758_221_040);
        assert_eq!(release.build.sha, "14c6e7f");
        assert!(release.download_url.ends_with("ubiqX-macos-aarch64.dmg"));
        assert_eq!(release.kind, "dmg");
        assert_eq!(release.release_url, feed.release_url);
        assert!(feed.release_for("linux-x86_64").is_none());

        assert!(feed.is_update_for(&current("0.1.0", 1_758_200_000)));
        assert!(!feed.is_update_for(&current("0.1.0", 1_758_221_040)));
        assert!(!feed.is_update_for(&current("0.2.0", 0)));

        // Optional fields may be missing entirely.
        let minimal: UpdateFeed = serde_json::from_str(
            r#"{"version":"0.1.0","build":{"epoch":5},"platforms":{"darwin-aarch64":{"url":"u"}}}"#,
        )
        .unwrap();
        assert_eq!(minimal.schema, 1);
        assert_eq!(minimal.platforms["darwin-aarch64"].kind, "dmg");
        assert_eq!(minimal.published_at, None);
    }

    #[test]
    fn status_and_release_serialize_snake_case() {
        let feed: UpdateFeed = serde_json::from_str(FEED).unwrap();
        let release = feed.release_for("darwin-aarch64").unwrap();
        let mut status = UpdateStatus::initial(current("0.1.0", 1_758_200_000), "https://x/y");
        status.available = Some(release);
        let v = serde_json::to_value(&status).unwrap();
        assert_eq!(v["current"]["sha"], "abc1234");
        assert_eq!(v["enabled"], true);
        assert_eq!(v["dismissed"], false);
        assert_eq!(v["checking"], false);
        assert!(v["last_check"].is_null());
        assert!(v["last_error"].is_null());
        assert_eq!(v["available"]["build"]["epoch"], 1_758_221_040);
        assert_eq!(v["available"]["published_at"], "2026-09-18T19:10:00Z");
        assert_eq!(v["available"]["kind"], "dmg");
        assert!(v["available"]["download_url"].is_string());
        assert!(v["available"]["app_zip_url"].is_string());
        assert!(v["available"]["release_url"].is_string());
        assert!(!UpdateStatus::initial(BuildInfo::dev(), "u").enabled);
    }

    #[test]
    fn formats_dates_and_notification_text() {
        // 2026-09-18 19:10:00 UTC seen from UTC-3 is 16:10 local time.
        let tz = FixedOffset::west_opt(3 * 3600).unwrap();
        assert_eq!(
            format_build_date_in(1_789_758_600, UiLanguage::PtBr, &tz),
            "18/09 16:10"
        );
        assert_eq!(
            format_build_date_in(1_789_758_600, UiLanguage::En, &tz),
            "Sep 18 16:10"
        );
        let feed: UpdateFeed = serde_json::from_str(FEED).unwrap();
        let release = feed.release_for("darwin-aarch64").unwrap();
        let (title, body) = notification_text(&release, UiLanguage::PtBr);
        assert_eq!(title, "Nova versão do ubiqX");
        assert!(body.starts_with("Build "));
        assert!(body.contains("(14c6e7f) já está disponível. Abra o ubiqX para baixar."));
        let (title, body) = notification_text(&release, UiLanguage::En);
        assert_eq!(title, "New ubiqX version");
        assert!(body.contains("(14c6e7f) is available. Open ubiqX to download it."));
    }
}
