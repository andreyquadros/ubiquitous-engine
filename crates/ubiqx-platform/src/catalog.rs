//! Helpers shared by the Windows and Linux application catalogues: the name order the UI
//! expects, a 60-second cache around a scan, and the heuristics that keep uninstallers out
//! of the list.

use std::time::{Duration, Instant};

use parking_lot::Mutex;
use ubiqx_core::InstalledApp;

/// How long a scan is reused before the folders are read again.
pub const CACHE_TTL: Duration = Duration::from_secs(60);

/// Name order, case-insensitive, with the app id as a stable tie-break.
pub fn sort_apps(apps: &mut [InstalledApp]) {
    apps.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.bundle_id.cmp(&b.bundle_id))
    });
}

/// Whether a shortcut or desktop entry is an uninstaller, a help file or another entry
/// nobody blocks or focuses on, judged by its name and target.
pub fn looks_like_uninstaller(name: &str, target: &str) -> bool {
    let n = name.to_lowercase();
    let t = target.to_lowercase();
    n.contains("uninstall")
        || n.contains("desinstal")
        || n.contains("remove ")
        || n.starts_with("remover ")
        || t.contains("unins")
        || t.contains("uninstall")
}

/// A scan result kept for [`CACHE_TTL`].
#[derive(Debug, Default)]
pub struct CachedScan {
    cache: Mutex<Option<(Instant, Vec<InstalledApp>)>>,
}

impl CachedScan {
    /// The cached list, or a fresh `scan()` stored for the next minute.
    pub fn get_or_scan(&self, scan: impl FnOnce() -> Vec<InstalledApp>) -> Vec<InstalledApp> {
        if let Some((at, apps)) = self.cache.lock().as_ref() {
            if at.elapsed() < CACHE_TTL {
                return apps.clone();
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
        apps
    }

    /// The last scan, without refreshing it (for lookups on the sampler path that must
    /// never block on a scan).
    pub fn cached(&self) -> Option<Vec<InstalledApp>> {
        self.cache.lock().as_ref().map(|(_, apps)| apps.clone())
    }

    /// Runs `f` over the cached list (refreshed through `scan` when stale) without cloning
    /// it: for the per-sample name lookups.
    pub fn with<T>(
        &self,
        scan: impl FnOnce() -> Vec<InstalledApp>,
        f: impl FnOnce(&[InstalledApp]) -> T,
    ) -> T {
        let mut cache = self.cache.lock();
        let stale = cache
            .as_ref()
            .is_none_or(|(at, _)| at.elapsed() >= CACHE_TTL);
        if stale {
            let started = Instant::now();
            let apps = scan();
            tracing::debug!(
                count = apps.len(),
                ms = started.elapsed().as_millis(),
                "scanned installed applications"
            );
            *cache = Some((Instant::now(), apps));
        }
        f(cache
            .as_ref()
            .map(|(_, apps)| apps.as_slice())
            .unwrap_or(&[]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn app(name: &str, id: &str) -> InstalledApp {
        InstalledApp {
            name: name.into(),
            bundle_id: id.into(),
            path: String::new(),
        }
    }

    #[test]
    fn sorts_case_insensitively() {
        let mut apps = vec![app("xcode", "b"), app("Discord", "a"), app("discord", "0")];
        sort_apps(&mut apps);
        assert_eq!(apps[0].bundle_id, "0");
        assert_eq!(apps[1].bundle_id, "a");
        assert_eq!(apps[2].name, "xcode");
    }

    #[test]
    fn skips_uninstallers() {
        assert!(looks_like_uninstaller("Uninstall Slack", "slack.exe"));
        assert!(looks_like_uninstaller("Desinstalar", "x.exe"));
        assert!(looks_like_uninstaller("Slack", r"C:\Slack\unins000.exe"));
        assert!(!looks_like_uninstaller("Slack", r"C:\Slack\slack.exe"));
    }

    #[test]
    fn caches_for_a_minute() {
        let scans = AtomicUsize::new(0);
        let cache = CachedScan::default();
        assert!(cache.cached().is_none());
        for _ in 0..3 {
            let apps = cache.get_or_scan(|| {
                scans.fetch_add(1, Ordering::SeqCst);
                vec![app("A", "a")]
            });
            assert_eq!(apps.len(), 1);
        }
        assert_eq!(scans.load(Ordering::SeqCst), 1);
        assert_eq!(cache.cached().unwrap().len(), 1);
        let name = cache.with(
            || unreachable!("fresh cache is reused"),
            |apps| {
                apps.iter()
                    .find(|a| a.bundle_id == "a")
                    .map(|a| a.name.clone())
            },
        );
        assert_eq!(name.as_deref(), Some("A"));
    }
}
