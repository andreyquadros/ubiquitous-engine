//! The update checker: downloads the feed through the [`UpdateFeedSource`] port, compares it
//! with the running build (`ubiqx_core::update`), keeps the [`UpdateStatus`] the shells read,
//! notifies the user once per published build and emits [`EngineEvent::UpdateAvailable`].
//!
//! What was already announced survives restarts through the key/value store
//! (`update.notified_epoch`, `update.dismissed_epoch`), so a build is notified once even
//! across launches, and a dismissed banner stays dismissed until a newer build appears.

use std::sync::Arc;

use parking_lot::Mutex;
use ubiqx_core::ports::*;
use ubiqx_core::update::{self, kv_keys};
use ubiqx_core::*;

pub struct UpdateChecker {
    feed_url: String,
    /// The feed key of this machine (`darwin-aarch64`, …).
    target: String,
    source: Arc<dyn UpdateFeedSource>,
    kv: Arc<dyn KvRepo>,
    settings: Arc<dyn SettingsRepo>,
    notifier: Arc<dyn Notifier>,
    sink: Arc<dyn EventSink>,
    clock: Arc<dyn Clock>,
    status: Mutex<UpdateStatus>,
    /// The build epoch of the last `UpdateAvailable` event, so automatic checks announce a
    /// build only once per process.
    reported_epoch: Mutex<Option<i64>>,
    /// Serialises checks: a manual check while the scheduled one runs waits for it.
    running: tokio::sync::Mutex<()>,
}

impl UpdateChecker {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        current: BuildInfo,
        feed_url: impl Into<String>,
        source: Arc<dyn UpdateFeedSource>,
        kv: Arc<dyn KvRepo>,
        settings: Arc<dyn SettingsRepo>,
        notifier: Arc<dyn Notifier>,
        sink: Arc<dyn EventSink>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self::with_target(
            current,
            feed_url,
            update::current_target(),
            source,
            kv,
            settings,
            notifier,
            sink,
            clock,
        )
    }

    /// [`UpdateChecker::new`] for an explicit feed target (tests run on any OS).
    #[allow(clippy::too_many_arguments)]
    pub fn with_target(
        current: BuildInfo,
        feed_url: impl Into<String>,
        target: impl Into<String>,
        source: Arc<dyn UpdateFeedSource>,
        kv: Arc<dyn KvRepo>,
        settings: Arc<dyn SettingsRepo>,
        notifier: Arc<dyn Notifier>,
        sink: Arc<dyn EventSink>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        let feed_url = feed_url.into();
        Self {
            status: Mutex::new(UpdateStatus::initial(current, feed_url.clone())),
            feed_url,
            target: target.into(),
            source,
            kv,
            settings,
            notifier,
            sink,
            clock,
            reported_epoch: Mutex::new(None),
            running: tokio::sync::Mutex::new(()),
        }
    }

    pub fn status(&self) -> UpdateStatus {
        self.status.lock().clone()
    }

    /// Whether automatic checks run at all (never for a development build).
    pub fn enabled(&self) -> bool {
        self.status.lock().enabled
    }

    /// A check the user asked for: always runs and compares, even on a development build,
    /// and announces an available build again through the event sink.
    pub async fn check_now(&self) -> CoreResult<UpdateStatus> {
        self.check(true).await
    }

    /// The scheduled check: silent when the build already reported is found again.
    pub async fn check_scheduled(&self) -> CoreResult<UpdateStatus> {
        self.check(false).await
    }

    async fn check(&self, manual: bool) -> CoreResult<UpdateStatus> {
        let _guard = self.running.lock().await;
        self.status.lock().checking = true;
        let fetched = self.source.fetch(&self.feed_url).await;
        let now = self.clock.now();
        let feed = match fetched {
            Ok(feed) => feed,
            Err(e) => {
                tracing::warn!(error = %e, url = %self.feed_url, "update check failed");
                let mut st = self.status.lock();
                st.checking = false;
                st.last_check = Some(now);
                st.last_error = Some(e.to_string());
                return Ok(st.clone());
            }
        };

        let current = self.status.lock().current.clone();
        let release = if feed.is_update_for(&current) {
            feed.release_for(&self.target)
        } else {
            None
        };
        let dismissed_epoch = self.kv_epoch(kv_keys::DISMISSED_EPOCH)?;
        let notified_epoch = self.kv_epoch(kv_keys::NOTIFIED_EPOCH)?;

        let status = {
            let mut st = self.status.lock();
            st.checking = false;
            st.last_check = Some(now);
            st.last_error = None;
            st.dismissed = release
                .as_ref()
                .map(|r| dismissed_epoch == Some(r.build.epoch))
                .unwrap_or(false);
            st.available = release.clone();
            st.clone()
        };

        let Some(release) = release else {
            tracing::info!(
                feed_version = %feed.version,
                feed_epoch = feed.build.epoch,
                "no update: the running build is current"
            );
            return Ok(status);
        };
        let epoch = release.build.epoch;
        tracing::info!(
            version = %release.version,
            epoch,
            sha = %release.build.sha,
            dismissed = status.dismissed,
            "update available"
        );

        // One notification per published build, never for a build the user dismissed.
        if notified_epoch != Some(epoch) && !status.dismissed {
            let lang = self
                .settings
                .load()
                .map(|s| s.ui_language())
                .unwrap_or_default();
            let (title, body) = update::notification_text(&release, lang);
            if let Err(e) = self.notifier.notify(&title, &body) {
                tracing::debug!(error = %e, "update notification failed");
            }
            self.kv.set(kv_keys::NOTIFIED_EPOCH, &epoch.to_string())?;
        }

        // The shells learn about a build once per process, or whenever the user asks.
        let announce = {
            let mut reported = self.reported_epoch.lock();
            let changed = *reported != Some(epoch);
            *reported = Some(epoch);
            manual || changed
        };
        if announce {
            self.sink.emit(EngineEvent::UpdateAvailable { release });
        }
        Ok(status)
    }

    /// Hides the banner for the build with this epoch. Dismissing anything else (a build
    /// that is no longer the available one) only records the preference.
    pub fn dismiss(&self, epoch: i64) -> CoreResult<UpdateStatus> {
        self.kv.set(kv_keys::DISMISSED_EPOCH, &epoch.to_string())?;
        let mut st = self.status.lock();
        st.dismissed = st
            .available
            .as_ref()
            .map(|r| r.build.epoch == epoch)
            .unwrap_or(false);
        Ok(st.clone())
    }

    fn kv_epoch(&self, key: &str) -> CoreResult<Option<i64>> {
        Ok(self.kv.get(key)?.and_then(|v| v.trim().parse().ok()))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::collections::HashMap;

    use chrono::{TimeZone, Utc};
    use ubiqx_core::update::{FeedBuild, PlatformAsset};

    use super::*;

    #[derive(Default)]
    struct MemKv(Mutex<HashMap<String, String>>);

    impl KvRepo for MemKv {
        fn get(&self, key: &str) -> CoreResult<Option<String>> {
            Ok(self.0.lock().get(key).cloned())
        }
        fn set(&self, key: &str, value: &str) -> CoreResult<()> {
            self.0.lock().insert(key.into(), value.into());
            Ok(())
        }
    }

    struct MemSettings(Mutex<Settings>);

    impl SettingsRepo for MemSettings {
        fn load(&self) -> CoreResult<Settings> {
            Ok(self.0.lock().clone())
        }
        fn save(&self, settings: &Settings) -> CoreResult<()> {
            *self.0.lock() = settings.clone();
            Ok(())
        }
    }

    #[derive(Default)]
    struct CollectNotifier(Mutex<Vec<(String, String)>>);

    impl Notifier for CollectNotifier {
        fn notify(&self, title: &str, body: &str) -> CoreResult<()> {
            self.0.lock().push((title.into(), body.into()));
            Ok(())
        }
    }

    #[derive(Default)]
    struct CollectSink(Mutex<Vec<EngineEvent>>);

    impl EventSink for CollectSink {
        fn emit(&self, event: EngineEvent) {
            self.0.lock().push(event);
        }
    }

    struct StaticFeed(Mutex<Option<UpdateFeed>>);

    #[async_trait::async_trait]
    impl UpdateFeedSource for StaticFeed {
        async fn fetch(&self, _url: &str) -> CoreResult<UpdateFeed> {
            self.0
                .lock()
                .clone()
                .ok_or_else(|| CoreError::Other("feed offline".into()))
        }
    }

    struct FixedClock;

    impl Clock for FixedClock {
        fn now(&self) -> chrono::DateTime<Utc> {
            Utc.with_ymd_and_hms(2026, 9, 18, 19, 20, 0).unwrap()
        }
    }

    fn feed(version: &str, epoch: i64) -> UpdateFeed {
        let mut platforms = BTreeMap::new();
        platforms.insert(
            "darwin-aarch64".to_string(),
            PlatformAsset {
                url: "https://example.test/ubiqX-macos-aarch64.dmg".into(),
                kind: "dmg".into(),
                size: Some(1),
                app_zip_url: None,
                alternates: Vec::new(),
            },
        );
        UpdateFeed {
            schema: 1,
            product: "ubiqX".into(),
            version: version.into(),
            build: FeedBuild {
                epoch,
                number: 27,
                sha: "a1b2c3d".into(),
                branch: "main".into(),
            },
            published_at: None,
            notes: "feat: x".into(),
            release_url: None,
            platforms,
        }
    }

    fn current(epoch: i64) -> BuildInfo {
        BuildInfo {
            version: "0.1.0".into(),
            epoch,
            number: 26,
            sha: "14c6e7f".into(),
            branch: "main".into(),
        }
    }

    struct Rig {
        checker: UpdateChecker,
        feed: Arc<StaticFeed>,
        kv: Arc<MemKv>,
        notifier: Arc<CollectNotifier>,
        sink: Arc<CollectSink>,
    }

    fn rig(current: BuildInfo, feed: Option<UpdateFeed>, lang: &str) -> Rig {
        let feed = Arc::new(StaticFeed(Mutex::new(feed)));
        let kv = Arc::new(MemKv::default());
        let notifier = Arc::new(CollectNotifier::default());
        let sink = Arc::new(CollectSink::default());
        let settings = Arc::new(MemSettings(Mutex::new(Settings {
            language: lang.into(),
            ..Settings::default()
        })));
        let checker = UpdateChecker::with_target(
            current,
            "https://example.test/latest.json",
            "darwin-aarch64",
            feed.clone(),
            kv.clone(),
            settings,
            notifier.clone(),
            sink.clone(),
            Arc::new(FixedClock),
        );
        Rig {
            checker,
            feed,
            kv,
            notifier,
            sink,
        }
    }

    fn events(rig: &Rig) -> usize {
        rig.sink
            .0
            .lock()
            .iter()
            .filter(|e| matches!(e, EngineEvent::UpdateAvailable { .. }))
            .count()
    }

    #[tokio::test]
    async fn newer_feed_is_available_and_announced_once() {
        let rig = rig(
            current(1_758_200_000),
            Some(feed("0.1.0", 1_758_221_040)),
            "pt-BR",
        );
        assert!(rig.checker.enabled());
        let st = rig.checker.check_scheduled().await.unwrap();
        let rel = st.available.as_ref().expect("available");
        assert_eq!(rel.build.epoch, 1_758_221_040);
        assert_eq!(rel.build.sha, "a1b2c3d");
        assert!(rel.download_url.ends_with(".dmg"));
        assert!(!st.dismissed);
        assert!(!st.checking);
        assert!(st.last_error.is_none());
        assert_eq!(st.last_check, Some(FixedClock.now()));
        assert_eq!(rig.checker.status(), st);

        let notes = rig.notifier.0.lock().clone();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].0, "Nova versão do ubiqX");
        assert!(notes[0].1.contains("(a1b2c3d) já está disponível"));
        assert_eq!(events(&rig), 1);
        assert_eq!(
            rig.kv.get(kv_keys::NOTIFIED_EPOCH).unwrap().as_deref(),
            Some("1758221040")
        );

        // The scheduled check finds the same build again: no second notification, no event.
        rig.checker.check_scheduled().await.unwrap();
        assert_eq!(rig.notifier.0.lock().len(), 1);
        assert_eq!(events(&rig), 1);

        // A manual check re-announces (the UI asked), still without another notification.
        let st = rig.checker.check_now().await.unwrap();
        assert!(st.available.is_some());
        assert_eq!(rig.notifier.0.lock().len(), 1);
        assert_eq!(events(&rig), 2);
    }

    #[tokio::test]
    async fn notification_follows_the_settings_language() {
        let rig = rig(
            current(1_758_200_000),
            Some(feed("0.1.0", 1_758_221_040)),
            "en",
        );
        rig.checker.check_scheduled().await.unwrap();
        let notes = rig.notifier.0.lock().clone();
        assert_eq!(notes[0].0, "New ubiqX version");
        assert!(notes[0]
            .1
            .contains("(a1b2c3d) is available. Open ubiqX to download it."));
    }

    #[tokio::test]
    async fn notified_epoch_survives_restarts() {
        let rig = rig(
            current(1_758_200_000),
            Some(feed("0.1.0", 1_758_221_040)),
            "pt-BR",
        );
        rig.kv.set(kv_keys::NOTIFIED_EPOCH, "1758221040").unwrap();
        let st = rig.checker.check_scheduled().await.unwrap();
        assert!(st.available.is_some());
        assert!(rig.notifier.0.lock().is_empty());
        // A newer build than the one notified is notified again.
        rig.feed.0.lock().replace(feed("0.1.0", 1_758_300_000));
        rig.checker.check_scheduled().await.unwrap();
        assert_eq!(rig.notifier.0.lock().len(), 1);
        assert_eq!(events(&rig), 2, "a different epoch is announced again");
    }

    #[tokio::test]
    async fn dismissal_is_per_build() {
        let rig = rig(
            current(1_758_200_000),
            Some(feed("0.1.0", 1_758_221_040)),
            "pt-BR",
        );
        rig.checker.check_scheduled().await.unwrap();
        let st = rig.checker.dismiss(1_758_221_040).unwrap();
        assert!(st.dismissed);
        assert!(st.available.is_some());
        assert_eq!(
            rig.kv.get(kv_keys::DISMISSED_EPOCH).unwrap().as_deref(),
            Some("1758221040")
        );
        // Still dismissed after the next check of the same build…
        let st = rig.checker.check_scheduled().await.unwrap();
        assert!(st.dismissed);
        // …but not for a newer one.
        rig.feed.0.lock().replace(feed("0.1.0", 1_758_300_000));
        let st = rig.checker.check_scheduled().await.unwrap();
        assert!(!st.dismissed);
        assert_eq!(st.available.unwrap().build.epoch, 1_758_300_000);
        // Dismissing an epoch that is not the available one changes nothing visible.
        assert!(!rig.checker.dismiss(1).unwrap().dismissed);
    }

    #[tokio::test]
    async fn dismissed_build_is_not_notified() {
        let rig = rig(
            current(1_758_200_000),
            Some(feed("0.1.0", 1_758_221_040)),
            "pt-BR",
        );
        rig.kv.set(kv_keys::DISMISSED_EPOCH, "1758221040").unwrap();
        let st = rig.checker.check_scheduled().await.unwrap();
        assert!(st.dismissed);
        assert!(rig.notifier.0.lock().is_empty());
        assert_eq!(events(&rig), 1, "the shells still learn the status");
    }

    #[tokio::test]
    async fn older_or_equal_feed_is_no_update() {
        let rig = rig(
            current(1_758_221_040),
            Some(feed("0.1.0", 1_758_200_000)),
            "pt-BR",
        );
        let st = rig.checker.check_now().await.unwrap();
        assert!(st.available.is_none());
        assert!(st.last_error.is_none());
        assert!(rig.notifier.0.lock().is_empty());
        assert_eq!(events(&rig), 0);

        rig.feed.0.lock().replace(feed("0.1.0", 1_758_221_040));
        assert!(rig.checker.check_now().await.unwrap().available.is_none());
        rig.feed.0.lock().replace(feed("0.0.9", 9_999_999_999));
        assert!(rig.checker.check_now().await.unwrap().available.is_none());
        // A greater version is an update regardless of epoch.
        rig.feed.0.lock().replace(feed("0.2.0", 1));
        assert!(rig.checker.check_now().await.unwrap().available.is_some());
    }

    #[tokio::test]
    async fn an_update_that_disappears_clears_the_status() {
        let rig = rig(
            current(1_758_200_000),
            Some(feed("0.1.0", 1_758_221_040)),
            "pt-BR",
        );
        assert!(rig.checker.check_now().await.unwrap().available.is_some());
        rig.feed.0.lock().replace(feed("0.1.0", 1_758_100_000));
        let st = rig.checker.check_now().await.unwrap();
        assert!(st.available.is_none());
        assert!(!st.dismissed);
    }

    #[tokio::test]
    async fn dev_build_is_disabled_but_a_manual_check_still_compares() {
        let rig = rig(
            BuildInfo::dev(),
            Some(feed("0.1.0", 1_758_221_040)),
            "pt-BR",
        );
        assert!(!rig.checker.enabled());
        assert!(!rig.checker.status().enabled);
        let st = rig.checker.check_now().await.unwrap();
        assert!(!st.enabled);
        assert!(
            st.available.is_some(),
            "a published build is newer than a dev one"
        );
        assert_eq!(rig.notifier.0.lock().len(), 1);
    }

    #[tokio::test]
    async fn fetch_failure_lands_in_last_error() {
        let rig = rig(current(1_758_200_000), None, "pt-BR");
        let st = rig.checker.check_now().await.unwrap();
        assert!(st.available.is_none());
        assert_eq!(st.last_error.as_deref(), Some("feed offline"));
        assert_eq!(st.last_check, Some(FixedClock.now()));
        assert!(!st.checking);
        assert_eq!(events(&rig), 0);
        // The next successful check clears the error.
        rig.feed.0.lock().replace(feed("0.1.0", 1_758_221_040));
        let st = rig.checker.check_now().await.unwrap();
        assert!(st.last_error.is_none());
        assert!(st.available.is_some());
    }

    #[tokio::test]
    async fn feed_without_an_asset_for_this_target_is_no_update() {
        let mut f = feed("0.1.0", 1_758_221_040);
        f.platforms.clear();
        let rig = rig(current(1_758_200_000), Some(f), "pt-BR");
        let st = rig.checker.check_now().await.unwrap();
        assert!(st.available.is_none());
        assert!(st.last_error.is_none());
    }
}
