//! The public API of a running engine.

use std::sync::Arc;

use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;
use ubiqx_core::ports::*;
use ubiqx_core::*;

use crate::classify::ClassifyReport;
use crate::learning::CorrectionOutcome;
use crate::state::EngineState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReclassifyScope {
    Block,
    Day,
    Month,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrivateModeDuration {
    Off,
    Minutes30,
    Hour1,
    UntilTomorrow,
    Indefinite,
}

/// How long `shutdown` waits for the tracker task to persist the open block.
pub const SHUTDOWN_GRACE: std::time::Duration = std::time::Duration::from_secs(2);

#[derive(Clone)]
pub struct EngineHandle {
    pub(crate) state: Arc<EngineState>,
    pub(crate) cancel: CancellationToken,
}

impl EngineHandle {
    pub fn state(&self) -> &Arc<EngineState> {
        &self.state
    }

    pub fn settings(&self) -> Settings {
        self.state.settings()
    }

    pub fn update_settings(&self, settings: Settings) -> CoreResult<()> {
        self.state.apply_settings(settings)
    }

    pub fn tracker_state(&self) -> TrackerState {
        *self.state.tracker.read()
    }

    pub fn ai_health(&self) -> AiHealth {
        self.state.ai_health()
    }

    /// The stored license key's verdict (verified at start and when set), with the managed
    /// plan's usage when the proxy answered. Re-evaluated here so an expiry that passed
    /// since the last check shows at once.
    pub fn license_status(&self) -> LicenseStatus {
        crate::license::current(&self.state)
    }

    /// Stores (or, with `None`/blank, removes) the license key, verifies it offline and, for
    /// a valid `monthly_managed` key, fetches the month's usage from the proxy (a proxy that
    /// cannot be reached keeps the local verdict, without usage). See
    /// [`crate::license::set_key`].
    pub async fn set_license_key(&self, key: Option<&str>) -> CoreResult<LicenseStatus> {
        crate::license::set_key(&self.state, key).await
    }

    /// Asks the proxy for the managed plan's usage again (no-op for other licenses).
    pub async fn refresh_license(&self) -> LicenseStatus {
        crate::license::reevaluate(&self.state);
        crate::license::refresh_managed_usage(&self.state).await
    }

    pub fn pause(&self) {
        *self.state.paused.write() = true;
        let now = self.state.now();
        if let Some(block) = self.state.segmenter.lock().close(now) {
            let _ = self.state.deps.repos.blocks.touch(&block);
            self.state.last_capture.lock().remove(&block.id);
        }
        self.state.refresh_tracker_state();
    }

    pub fn resume(&self) {
        *self.state.paused.write() = false;
        self.state.refresh_tracker_state();
    }

    pub fn set_private_mode(&self, duration: PrivateModeDuration) -> CoreResult<()> {
        let now = self.state.now();
        let mut s = self.state.settings();
        match duration {
            PrivateModeDuration::Off => {
                s.private_mode = false;
                s.private_until = None;
            }
            PrivateModeDuration::Minutes30 => {
                s.private_mode = true;
                s.private_until = Some(now + Duration::minutes(30));
            }
            PrivateModeDuration::Hour1 => {
                s.private_mode = true;
                s.private_until = Some(now + Duration::hours(1));
            }
            PrivateModeDuration::UntilTomorrow => {
                let tomorrow = chrono::Local::now()
                    .date_naive()
                    .succ_opt()
                    .unwrap_or(chrono::Local::now().date_naive());
                s.private_mode = true;
                s.private_until = Some(ubiqx_core::scheduler::day_range(tomorrow).from);
            }
            PrivateModeDuration::Indefinite => {
                s.private_mode = true;
                s.private_until = None;
            }
        }
        // Close the open block so the private span starts cleanly.
        if let Some(block) = self.state.segmenter.lock().close(now) {
            self.state.deps.repos.blocks.touch(&block)?;
            self.state.last_capture.lock().remove(&block.id);
        }
        self.state.apply_settings(s)
    }

    pub fn snooze_nudges(&self, minutes: i64) -> CoreResult<()> {
        let mut s = self.state.settings();
        s.nudges.snoozed_until = Some(self.state.now() + Duration::minutes(minutes));
        self.state.apply_settings(s)
    }

    /// Stores (or, with `None`/blank, deletes) the API key of `provider` under
    /// [`AiProvider::secret_key`]. When `provider` is the one selected in settings the AI path
    /// is re-armed (`Ok`) or disarmed (`NotConfigured`); keys of the other vendors are kept
    /// for a later switch and never touch health. The managed provider has no API key: its
    /// credential is the license ([`Self::set_license_key`]), so it is refused here.
    pub fn set_api_key(&self, provider: AiProvider, key: Option<&str>) -> CoreResult<()> {
        if provider.is_managed() {
            return Err(CoreError::Invalid(
                "the managed provider uses the license key (set_license_key)".into(),
            ));
        }
        let secrets = &self.state.deps.platform.secrets;
        let selected = self.state.settings.read().ai_provider == provider;
        match key.map(str::trim).filter(|k| !k.is_empty()) {
            Some(k) => {
                secrets.set(provider.secret_key(), k)?;
                if selected {
                    self.state.set_ai_health(AiHealth::Ok);
                }
            }
            None => {
                secrets.delete(provider.secret_key())?;
                if selected {
                    self.state.set_ai_health(AiHealth::NotConfigured);
                }
            }
        }
        Ok(())
    }

    /// The last four characters of `provider`'s stored key (`…abcd`), or `None` when it has
    /// no key.
    pub fn api_key_hint(&self, provider: AiProvider) -> CoreResult<Option<String>> {
        Ok(self
            .state
            .deps
            .platform
            .secrets
            .get(provider.secret_key())?
            .filter(|k| !k.trim().is_empty())
            .map(|k| format!("…{}", &k[k.len().saturating_sub(4)..])))
    }

    /// Key hint per provider, in [`AiProvider::ALL`] order.
    pub fn api_key_status(&self) -> CoreResult<Vec<(AiProvider, Option<String>)>> {
        AiProvider::ALL
            .into_iter()
            .map(|p| Ok((p, self.api_key_hint(p)?)))
            .collect()
    }

    pub fn reclassify(
        &self,
        block_id: &str,
        category_id: &str,
        note: Option<String>,
        scope: ReclassifyScope,
    ) -> CoreResult<CorrectionOutcome> {
        crate::learning::reclassify(&self.state, block_id, category_id, note, scope)
    }

    /// Turns the classifier's own answers for these review groups into user answers.
    pub fn confirm_groups(
        &self,
        date: chrono::NaiveDate,
        keys: &[String],
    ) -> CoreResult<CorrectionOutcome> {
        crate::learning::confirm_groups(&self.state, date, keys)
    }

    pub fn accept_rule_suggestion(&self, suggestion: &RuleSuggestion) -> CoreResult<Rule> {
        crate::learning::accept_suggestion(&self.state, suggestion)
    }

    pub fn add_manual_entry(
        &self,
        started_at: DateTime<Utc>,
        ended_at: DateTime<Utc>,
        category_id: &str,
        note: Option<String>,
    ) -> CoreResult<ActivityBlock> {
        crate::learning::add_manual_entry(&self.state, started_at, ended_at, category_id, note)
    }

    pub fn split_block(&self, block_id: &str, at: DateTime<Utc>) -> CoreResult<Id> {
        crate::learning::split_block(&self.state, block_id, at)
    }

    /// One classification pass now, ignoring the batching thresholds.
    /// [`CoreError::LicenseRequired`] under hard license enforcement while unlicensed.
    pub async fn classify_now(&self) -> CoreResult<ClassifyReport> {
        self.state.license_gate()?;
        crate::classify::run_once(&self.state, true).await
    }

    /// Generates (or regenerates) one report. [`CoreError::LicenseRequired`] under hard
    /// license enforcement while unlicensed.
    pub async fn generate_report(
        &self,
        date: NaiveDate,
        category_id: &str,
    ) -> CoreResult<DailyReport> {
        self.state.license_gate()?;
        crate::reports::generate(&self.state, date, category_id, true).await
    }

    /// Productivity recommendations from the advisor, cached per local day.
    /// [`CoreError::LicenseRequired`] under hard license enforcement while unlicensed.
    pub async fn advice(&self, force: bool) -> CoreResult<Advice> {
        let state = self.state.clone();
        state.license_gate()?;
        let today = crate::service::today(&state);
        let key = format!("advice_{today}");
        if !force {
            if let Some(cached) = state.deps.repos.kv.get(&key)? {
                if let Ok(a) = serde_json::from_str::<Advice>(&cached) {
                    return Ok(a);
                }
            }
        }
        let Some(advisor) = state.deps.ai.advisor.clone() else {
            return Err(CoreError::AiNotConfigured);
        };
        if !crate::classify::budget_allows(&state)? || !state.remote_allowed() {
            return Err(CoreError::AiNotConfigured);
        }
        let settings = state.settings();
        let st = state.clone();
        let (stats, totals, apps, nudges, categories) =
            tokio::task::spawn_blocking(move || -> CoreResult<_> {
                let now = st.now();
                let range = TimeRange::new(now - Duration::days(7), now);
                let blocks = st.deps.repos.blocks.list_in_range(range)?;
                let categories = st.deps.repos.categories.list(false)?;
                let stats = ubiqx_core::insights::compute_stats(&blocks, &categories, range, 0);
                let totals = st.deps.repos.blocks.totals_by_category(range)?;
                let apps = st.deps.repos.blocks.totals_by_app(range, 8)?;
                let nudges = st
                    .deps
                    .repos
                    .nudges
                    .list_recent(30)?
                    .into_iter()
                    .map(|n| n.kind)
                    .collect::<Vec<_>>();
                Ok((stats, totals, apps, nudges, categories))
            })
            .await
            .map_err(|e| CoreError::Other(e.to_string()))??;
        let category_totals = totals
            .into_iter()
            .map(|t| {
                let name = t
                    .category_id
                    .as_deref()
                    .and_then(|id| {
                        categories
                            .iter()
                            .find(|c| c.id == id)
                            .map(|c| c.name.clone())
                    })
                    .unwrap_or_else(|| {
                        settings
                            .ui_language()
                            .pick("Sem categoria", "Uncategorized")
                            .to_string()
                    });
                (name, t.secs)
            })
            .collect();
        let req = AdviceRequest {
            language: settings.language.clone(),
            stats,
            category_totals,
            top_apps: apps,
            recent_nudges: nudges,
            user_profile: settings.user_profile.clone(),
            model: settings.models.report.clone(),
        };
        let advice = advisor.advise(&req).await?;
        state
            .deps
            .repos
            .kv
            .set(&key, &serde_json::to_string(&advice).unwrap_or_default())?;
        Ok(advice)
    }

    /// Exports blocks (last 90 days), categories, rules and reports as JSON. Returns the path.
    pub fn export_json(&self) -> CoreResult<std::path::PathBuf> {
        let repos = &self.state.deps.repos;
        let now = self.state.now();
        let blocks = repos.blocks.list_in_range(TimeRange::new(
            now - Duration::days(90),
            now + Duration::days(1),
        ))?;
        let categories = repos.categories.list(true)?;
        let rules = repos.rules.list()?;
        let from = (now - Duration::days(90)).date_naive();
        let reports = repos
            .reports
            .list_between(from, now.date_naive() + Duration::days(1))?;
        let doc = serde_json::json!({
            "exported_at": now,
            "blocks": blocks,
            "categories": categories,
            "rules": rules,
            "reports": reports,
        });
        let dir = self.state.deps.data_dir.join("exports");
        std::fs::create_dir_all(&dir).map_err(|e| CoreError::Platform(e.to_string()))?;
        let path = dir.join(format!("ubiqx-export-{}.json", now.format("%Y%m%d-%H%M%S")));
        std::fs::write(
            &path,
            serde_json::to_vec_pretty(&doc).map_err(|e| CoreError::Other(e.to_string()))?,
        )
        .map_err(|e| CoreError::Platform(e.to_string()))?;
        Ok(path)
    }

    pub fn permissions(&self) -> PermissionStatus {
        self.state.deps.platform.permissions.status()
    }

    /// The update checker's state (running build, feed, available build, last check).
    pub fn update_status(&self) -> UpdateStatus {
        self.state.update.status()
    }

    /// Downloads the feed and compares it with the running build now, regardless of
    /// `settings.check_updates` (a development build compares too). A feed that cannot be
    /// fetched is reported in `UpdateStatus::last_error`, not as an `Err`.
    pub async fn check_for_updates(&self) -> CoreResult<UpdateStatus> {
        self.state.update.check_now().await
    }

    /// Hides the banner for the build with this commit epoch until a newer one appears.
    pub fn dismiss_update(&self, epoch: i64) -> CoreResult<UpdateStatus> {
        self.state.update.dismiss(epoch)
    }

    pub fn request_permission(&self, kind: PermissionKind) -> CoreResult<()> {
        self.state.deps.platform.permissions.request(kind)
    }

    // -- Focus guard ------------------------------------------------------------------------

    /// Installed applications, for the focus page's search (cached by the platform).
    pub fn installed_apps(&self) -> CoreResult<Vec<InstalledApp>> {
        crate::focus::installed_apps(&self.state)
    }

    /// Domains from the user's own activity, most time first.
    pub fn known_domains(&self, limit: usize) -> CoreResult<Vec<KnownDomain>> {
        crate::focus::known_domains(&self.state, limit)
    }

    pub fn focus_targets(&self) -> CoreResult<Vec<FocusTarget>> {
        crate::focus::list_targets(&self.state)
    }

    /// Adds a blocked app or site; adding one that exists re-enables it. Keys are normalised
    /// (domains lower-cased without scheme, path or `www.`).
    pub fn add_focus_target(
        &self,
        kind: FocusTargetKind,
        name: &str,
        key: &str,
    ) -> CoreResult<FocusTarget> {
        crate::focus::add_target(&self.state, kind, name, key)
    }

    pub fn set_focus_target_enabled(&self, id: &str, enabled: bool) -> CoreResult<FocusTarget> {
        crate::focus::set_target_enabled(&self.state, id, enabled)
    }

    pub fn remove_focus_target(&self, id: &str) -> CoreResult<()> {
        crate::focus::remove_target(&self.state, id)
    }

    /// Interventions, newest first.
    pub fn interventions(&self, limit: usize) -> CoreResult<Vec<Intervention>> {
        crate::focus::list_interventions(&self.state, limit)
    }

    pub fn focus_status(&self) -> CoreResult<FocusStatus> {
        crate::focus::status(&self.state)
    }

    /// Starts a focus session (`CoreError::Invalid` for a blank task or a length outside
    /// 5..=240 minutes); a running session is ended first.
    pub fn start_focus_session(&self, task: &str, minutes: u32) -> CoreResult<FocusSession> {
        crate::focus::start_session(&self.state, task, minutes)
    }

    /// Ends the running session early; `None` when there is none.
    pub fn stop_focus_session(&self) -> CoreResult<Option<FocusSession>> {
        crate::focus::stop_session(&self.state)
    }

    /// Shows the intervention window with a sample message; records nothing.
    pub fn test_intervention(&self) -> CoreResult<()> {
        crate::focus::test_intervention(&self.state)
    }

    /// Deletes everything derived from activity: blocks (the open one included), screenshots
    /// (rows and files), reports, corrections, learned rules, nudges, the AI usage ledger, the
    /// key/value cache (advice, report scheduling) and the JSON exports folder. Categories,
    /// user rules, settings and the API key are kept, as the confirmation dialog promises.
    /// Irreversible. The license key is kept too.
    pub fn delete_all_data(&self) -> CoreResult<()> {
        let repos = &self.state.deps.repos;
        let now = self.state.now();
        // Drop the in-memory open block; its row is about to be deleted anyway.
        let _ = self.state.segmenter.lock().close(now);
        self.state.last_capture.lock().clear();
        // Collect the file paths before the rows disappear.
        let shots = repos
            .screenshots
            .delete_before(now + Duration::days(3650))?;
        repos.maintenance.wipe_user_data()?;
        for shot in shots {
            crate::screenshots::unlink(&shot);
        }
        let _ = std::fs::remove_dir_all(crate::screenshots::screenshots_dir(
            &self.state.deps.data_dir,
        ));
        let _ = std::fs::remove_dir_all(self.state.deps.data_dir.join("exports"));
        // The usage ledger is empty now: lift a budget pause right away.
        if self
            .state
            .budget_paused
            .load(std::sync::atomic::Ordering::SeqCst)
        {
            let _ = crate::classify::budget_allows(&self.state);
        }
        Ok(())
    }

    /// Stops the loops and waits (at most [`SHUTDOWN_GRACE`]) for the tracker to persist the
    /// open block, so a shell may exit the process right after this returns. If the tracker
    /// does not answer in time, the block is closed and persisted from here.
    pub fn shutdown(&self) {
        self.cancel.cancel();
        {
            let mut stopped = self.state.tracker_stopped.lock();
            if !*stopped {
                self.state
                    .tracker_stopped_cv
                    .wait_for(&mut stopped, SHUTDOWN_GRACE);
            }
        }
        let now = self.state.now();
        if let Some(block) = self.state.segmenter.lock().close(now) {
            if let Err(e) = self.state.deps.repos.blocks.touch(&block) {
                tracing::warn!(error = %e, "could not persist block on shutdown");
            }
        }
    }

    pub fn is_running(&self) -> bool {
        !self.cancel.is_cancelled()
    }
}
