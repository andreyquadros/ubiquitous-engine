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

    pub fn pause(&self) {
        *self.state.paused.write() = true;
        let now = self.state.now();
        if let Some(block) = self.state.segmenter.lock().close(now) {
            let _ = self.state.deps.repos.blocks.update(&block);
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
            self.state.deps.repos.blocks.update(&block)?;
        }
        self.state.apply_settings(s)
    }

    pub fn snooze_nudges(&self, minutes: i64) -> CoreResult<()> {
        let mut s = self.state.settings();
        s.nudges.snoozed_until = Some(self.state.now() + Duration::minutes(minutes));
        self.state.apply_settings(s)
    }

    /// Stores the API key and re-arms the AI path.
    pub fn set_api_key(&self, key: Option<&str>) -> CoreResult<()> {
        let secrets = &self.state.deps.platform.secrets;
        match key.map(str::trim).filter(|k| !k.is_empty()) {
            Some(k) => {
                secrets.set(secret_keys::ANTHROPIC_API_KEY, k)?;
                self.state.set_ai_health(AiHealth::Ok);
            }
            None => {
                secrets.delete(secret_keys::ANTHROPIC_API_KEY)?;
                self.state.set_ai_health(AiHealth::NotConfigured);
            }
        }
        Ok(())
    }

    pub fn api_key_hint(&self) -> CoreResult<Option<String>> {
        Ok(self
            .state
            .deps
            .platform
            .secrets
            .get(secret_keys::ANTHROPIC_API_KEY)?
            .map(|k| format!("…{}", &k[k.len().saturating_sub(4)..])))
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

    pub async fn classify_now(&self) -> CoreResult<ClassifyReport> {
        crate::classify::run_once(&self.state, true).await
    }

    pub async fn generate_report(
        &self,
        date: NaiveDate,
        category_id: &str,
    ) -> CoreResult<DailyReport> {
        crate::reports::generate(&self.state, date, category_id, true).await
    }

    /// Productivity recommendations from the advisor, cached per local day.
    pub async fn advice(&self, force: bool) -> CoreResult<Advice> {
        let state = self.state.clone();
        let today = crate::service::today();
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
        if !state.remote_allowed() {
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
                    .unwrap_or_else(|| "Sem categoria".into());
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

    pub fn request_permission(&self, kind: PermissionKind) -> CoreResult<()> {
        self.state.deps.platform.permissions.request(kind)
    }

    /// Deletes every block, report, nudge, screenshot and the stored key. Irreversible.
    pub fn delete_all_data(&self) -> CoreResult<()> {
        let repos = &self.state.deps.repos;
        let far_future = self.state.now() + Duration::days(3650);
        let now = self.state.now();
        if let Some(block) = self.state.segmenter.lock().close(now) {
            let _ = repos.blocks.update(&block);
        }
        for shot in repos.screenshots.delete_before(far_future)? {
            crate::screenshots::unlink(&shot);
        }
        repos.blocks.delete_before(far_future)?;
        for r in repos.reports.list_between(
            NaiveDate::from_ymd_opt(2000, 1, 1).unwrap_or_default(),
            NaiveDate::from_ymd_opt(2100, 1, 1).unwrap_or_default(),
        )? {
            repos.reports.delete(&r.id)?;
        }
        self.set_api_key(None)?;
        Ok(())
    }

    pub fn shutdown(&self) {
        self.cancel.cancel();
    }

    pub fn is_running(&self) -> bool {
        !self.cancel.is_cancelled()
    }
}
