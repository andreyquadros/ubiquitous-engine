//! The status, the verify pass, the system tiers (L2 to L4) and the primary lock.
//!
//! Nothing here repairs anything by itself: re-applying a tier needs the operating
//! system's prompt, which only the user starts. The verify pass looks (unprivileged),
//! says what it found and, once per drift, offers to restore.

use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use ubiqx_core::nofap::{
    reason, ApplyOutcome, CardAction, CardKind, ChangeKind, DayKind, GuardState, Issue,
    NofapConfig, NofapEnvSnapshot, NofapEvent, NofapSite, NofapStatus, ObserveOpts, Os, Overall,
    PlannedChange, PreviousDns, RemoveScope, SystemCapabilities, SystemObservation, SystemPlan,
    Tier, TierCapability, TierState, VisionOff, VisionState, CONFIG_VERSION,
};
use ubiqx_core::ports::PermissionState;
use ubiqx_core::{CoreError, CoreResult};
use ubiqx_nofap::exclusions::{self, ExclusionLists};
use ubiqx_nofap::gate::Gate;
use ubiqx_nofap::help::{self, REMINDER_EVERY_DAYS};
use ubiqx_nofap::messages::Notice;
use ubiqx_nofap::plan::{
    self, AssessInput, CANARY_EVERY_SECS, DRIFT_CARD_EVERY_SECS, FULL_READ_EVERY_SECS,
    FULL_READ_RETRY_SECS,
};
use ubiqx_nofap::primary::{self, PrimaryLock, Role, HEARTBEAT_EVERY_SECS, LOCK_FILE};
use ubiqx_nofap::progress::{self, RETENTION_DAYS};
use ubiqx_nofap::{commitment, list};

use super::cards::{self, CardContext, Outbox};
use super::guard::bypass_recent;
use super::{error_class, unreadable_config, Background, NofapService, Runtime};

/// The system is read for `view` at most this often (integrator decision 21).
pub const VIEW_READ_EVERY_SECS: i64 = 60;

/// Whether `at` is at least `every` seconds before `now`, or not a usable time at all (a
/// time in the future means the clock was set back, and holds nothing).
fn due(at: Option<DateTime<Utc>>, every: i64, now: DateTime<Utc>) -> bool {
    at.is_none_or(|at| !(0..every).contains(&(now - at).num_seconds()))
}

fn in_place(state: &TierState) -> bool {
    matches!(state, TierState::Applied | TierState::UpdatePending)
}

fn drifted(state: &TierState) -> bool {
    matches!(state, TierState::Changed | TierState::Missing)
}

/// Whether a tier the user chose was last read as changed or missing.
fn in_drift(rt: &Runtime) -> bool {
    let chosen = &rt.config.system;
    rt.system.assessment.as_ref().is_some_and(|found| {
        (chosen.hosts && drifted(&found.hosts))
            || (chosen.resolver.is_some() && drifted(&found.resolver))
            || (chosen.policies && drifted(&found.policies))
    })
}

/// What a tier the user chose shows when this machine cannot have it at all.
fn overlay(found: TierState, chosen: bool, capability: &TierCapability) -> TierState {
    match capability {
        TierCapability::Unsupported { reason } if chosen => TierState::Unsupported {
            reason: reason.clone(),
        },
        TierCapability::Gated if chosen => TierState::Gated,
        TierCapability::Available | TierCapability::Gated | TierCapability::Unsupported { .. } => {
            found
        }
    }
}

fn chosen(config: &NofapConfig, tier: Tier) -> bool {
    match tier {
        Tier::Hosts => config.system.hosts,
        Tier::Resolver => config.system.resolver.is_some(),
        Tier::Policies => config.system.policies,
    }
}

/// The three tier states: what the last look found, or, before any look, what the
/// configuration and the capabilities alone say.
fn tier_states(rt: &Runtime, capabilities: Option<&SystemCapabilities>) -> [TierState; 3] {
    Tier::ALL.map(|tier| {
        let is_chosen = chosen(&rt.config, tier);
        let found = match (&rt.system.assessment, tier) {
            (Some(found), Tier::Hosts) => found.hosts.clone(),
            (Some(found), Tier::Resolver) => found.resolver.clone(),
            (Some(found), Tier::Policies) => found.policies.clone(),
            (None, _) if is_chosen => TierState::NotApplied,
            (None, _) => TierState::Off,
        };
        match capabilities {
            Some(capabilities) => overlay(found, is_chosen, capabilities.of(tier)),
            None => found,
        }
    })
}

/// A plan that asks for nothing: what "remove" previews.
fn empty_plan(owner: ubiqx_core::ProductId) -> SystemPlan {
    SystemPlan {
        v: CONFIG_VERSION,
        owner,
        hosts: false,
        extra: Vec::new(),
        resolver: None,
        policies: false,
    }
}

fn asks_nothing(plan: &SystemPlan) -> bool {
    !plan.hosts && plan.resolver.is_none() && !plan.policies
}

impl NofapService {
    /// The right to raise the operating system's prompt: one at a time. A second "Aplicar",
    /// "Restaurar" or "Remover" while a prompt is open is refused (the card's button and
    /// the page's can overlap), so nobody is asked for the password twice in a row.
    fn one_prompt(&self) -> CoreResult<parking_lot::MutexGuard<'_, ()>> {
        self.elevating.try_lock().ok_or_else(|| {
            CoreError::Invalid("the operating system is already asking for the password".into())
        })
    }

    // -----------------------------------------------------------------------------------------
    // Status
    // -----------------------------------------------------------------------------------------

    /// The status from what is in memory, the host's settings and the permissions. With
    /// the module off nothing outside the service is read.
    pub(crate) fn compute_status(&self) -> NofapStatus {
        let now = self.now();
        {
            let rt = self.rt.lock();
            if rt.config_error {
                // Unknown is not "off": the page must not read as a module nobody enabled,
                // and it says why (integrator decision 33).
                return NofapStatus {
                    overall: Overall::Reduced,
                    issues: vec![Issue::ConfigUnreadable],
                    ..NofapStatus::off()
                };
            }
            if !rt.config.enabled {
                // Tiers that are still on the machine keep showing, so the page can offer
                // to remove them.
                let [hosts, resolver, policies] = tier_states(&rt, None);
                let known = rt.system.assessment.is_some();
                return NofapStatus {
                    hosts: if known { hosts } else { TierState::Off },
                    resolver: if known { resolver } else { TierState::Off },
                    policies: if known { policies } else { TierState::Off },
                    ..NofapStatus::off()
                };
            }
        }
        let env = self.deps.env.snapshot();
        let capabilities = self.deps.system.capabilities();
        let secondary = self.rt.lock().role == Role::Secondary;
        let vision = if secondary {
            VisionState::Off {
                reason: VisionOff::Disabled,
            }
        } else {
            self.vision_state(&env)
        };
        // A secondary runs no pass and asks the platform nothing.
        let automation_denied =
            !secondary && self.deps.permissions.status().automation == PermissionState::Denied;

        let rt = self.rt.lock();
        if rt.config_error || !rt.config.enabled {
            // Switched off (or found unreadable) while the ports were asked: what was
            // read belongs to a module that is no longer on.
            drop(rt);
            return self.compute_status();
        }
        let [hosts, resolver, policies] = tier_states(&rt, Some(&capabilities));
        let pending: Vec<_> = rt
            .config
            .pending
            .iter()
            .map(|waiting| commitment::pending_view(waiting, now))
            .collect();
        let mut issues: Vec<Issue> = rt
            .system
            .assessment
            .as_ref()
            .map(|found| found.issues.clone())
            .unwrap_or_default();
        if bypass_recent(rt.bypass_at, now) {
            issues.push(Issue::DnsBypassSuspected);
        }
        let limit = rt
            .guard_limit
            .or(automation_denied.then_some(ubiqx_core::nofap::GuardLimit::AutomationDenied));
        if automation_denied || limit == Some(ubiqx_core::nofap::GuardLimit::AutomationDenied) {
            issues.push(Issue::AutomationDenied);
        }
        if let VisionState::Off { reason } = &vision {
            issues.extend(match reason {
                VisionOff::ScreenRecording => Some(Issue::ScreenRecordingDenied),
                VisionOff::BudgetSpent => Some(Issue::VisionBudgetSpent),
                VisionOff::CapReached => Some(Issue::VisionCapReached),
                VisionOff::NoProvider | VisionOff::NoKey | VisionOff::Licence => {
                    Some(Issue::VisionNoProvider)
                }
                VisionOff::ConsentOutdated => Some(Issue::ConsentOutdated),
                VisionOff::Disabled
                | VisionOff::LocalOnly
                | VisionOff::NotOnboarded
                | VisionOff::Wayland => None,
            });
        }
        let guard = if secondary || rt.matcher.is_none() {
            GuardState::Off
        } else {
            match limit {
                Some(reason) => GuardState::Limited { reason },
                None => GuardState::On,
            }
        };
        if secondary {
            issues.push(Issue::OtherProductPrimary);
        }

        let tiers = [
            (Tier::Hosts, &hosts),
            (Tier::Resolver, &resolver),
            (Tier::Policies, &policies),
        ];
        let tier_short = rt.system.assessment.is_some()
            && tiers.iter().any(|(tier, state)| {
                chosen(&rt.config, *tier)
                    && matches!(
                        state,
                        TierState::Changed
                            | TierState::Missing
                            | TierState::NotApplied
                            | TierState::Unsupported { .. }
                    )
            });
        let any_tier = tiers.iter().any(|(_, state)| in_place(state));
        let issue_short = issues.iter().any(|issue| {
            matches!(
                issue,
                Issue::HostsNotHonoured
                    | Issue::DnsBypassSuspected
                    | Issue::ResolverUnreachable
                    | Issue::NewConnectionUnfiltered
                    | Issue::PolicyConflict
            )
        });
        // The user's own caps reached are the plan working, not a gap.
        let vision_short = matches!(
            &vision,
            VisionState::Off { reason }
                if !matches!(
                    reason,
                    VisionOff::Disabled | VisionOff::CapReached | VisionOff::BudgetSpent
                )
        );
        // Nothing is stopping sites in the window in front.
        let nothing_left =
            !matches!(guard, GuardState::On) && vision != VisionState::On && !any_tier;
        // The user's own lists could not be read: a site they blocked may be missing.
        let lists_short = rt.sites_error;
        let overall = if secondary {
            Overall::Secondary
        } else if rt
            .config
            .pending
            .iter()
            .any(|waiting| waiting.change.kind() == ChangeKind::Disable)
        {
            Overall::PendingOff
        } else if tier_short || issue_short || vision_short || nothing_left || lists_short {
            Overall::Reduced
        } else {
            Overall::Protected
        };
        NofapStatus {
            overall,
            guard,
            hosts,
            resolver,
            policies,
            vision,
            issues,
            pending,
            hosts_waiting: rt
                .system
                .assessment
                .as_ref()
                .map_or(0, |found| found.hosts_waiting),
            extras_left_out: rt.system.extras_left_out,
        }
    }

    // -----------------------------------------------------------------------------------------
    // The verify pass
    // -----------------------------------------------------------------------------------------

    /// One verify pass, credited with the running time since the previous one (measured
    /// on the monotonic clock, so the system clock cannot shorten a wait).
    pub fn verify_pass(&self) -> CoreResult<()> {
        let elapsed = {
            let mut last = self.verified.lock();
            let elapsed = last.elapsed();
            *last = Instant::now();
            elapsed
        };
        self.verify_pass_after(elapsed)
    }

    /// One verify pass that credits `elapsed` of running time to what is waiting: the
    /// pending requests and the new privacy-list entries. It never elevates. With the
    /// module off it returns before any platform call.
    pub fn verify_pass_after(&self, elapsed: Duration) -> CoreResult<()> {
        let now = self.now();
        if self.read_config_again() {
            self.refresh_status();
        }
        {
            let rt = self.rt.lock();
            if !rt.config.enabled || rt.config_error {
                return Ok(());
            }
        }
        let primary = self.role_at(now) == Role::Primary;
        let elapsed_secs = u32::try_from(elapsed.as_secs()).unwrap_or(u32::MAX);
        let env = self.deps.env.snapshot();
        let mut outbox = Outbox {
            status: true,
            ..Outbox::default()
        };
        // A step that fails (a write that is refused, a system that cannot be read) does
        // not take the other steps with it, and what was queued is sent either way: a
        // card that was kept but never shown would hold back the next one for nothing.
        let mut failed: Option<CoreError> = None;
        let mut step = |result: CoreResult<()>| {
            if let Err(error) = result {
                failed.get_or_insert(error);
            }
        };
        step(self.tick_waits(&env, elapsed_secs, primary, now, &mut outbox));
        self.housekeeping(now);
        if primary {
            step(self.read_system(false, false));
            step(self.drift_and_outage(now, &mut outbox));
            step(self.reminders(now, &mut outbox));
        }
        self.flush(outbox);
        failed.map_or(Ok(()), Err)
    }

    /// While the stored configuration could not be read, each verify pass reads it again
    /// (and asks the repository nothing else). A read that works replaces the defaults in
    /// memory, ends the degraded state and takes the issue away (integrator decision 33).
    /// Returns whether it did.
    fn read_config_again(&self) -> bool {
        let mut rt = self.rt.lock();
        if !rt.config_error {
            return false;
        }
        let mut config = match self.deps.repo.load_config() {
            Ok(config) => config,
            Err(error) => {
                tracing::debug!(
                    class = error_class(&error),
                    "nofap: the stored configuration still cannot be read"
                );
                return false;
            }
        };
        config.sanitize();
        rt.config = config;
        rt.config_error = false;
        // The learned sites are matched with the salt of the configuration just read.
        self.reload_sites(&mut rt);
        drop(rt);
        self.clear_memory();
        tracing::info!("nofap: the stored configuration was read");
        true
    }

    /// Credits running time to the waiting requests and to new exclusions, drops requests
    /// nobody confirmed in a day, and announces the ones that became ready.
    fn tick_waits(
        &self,
        env: &NofapEnvSnapshot,
        elapsed_secs: u32,
        primary: bool,
        now: DateTime<Utc>,
        outbox: &mut Outbox,
    ) -> CoreResult<()> {
        let mut rt = self.rt.lock();
        let lists = ExclusionLists {
            apps: env.excluded_apps.clone(),
            domains: env.excluded_domains.clone(),
        };
        // Running time that could not be stored is not credited: a wait never ends in
        // memory only.
        let (expired, ready) = self.update_config(&mut rt, |config| {
            let expired = commitment::expire(config, now);
            let ready = commitment::tick(config, elapsed_secs, now);
            let seen = exclusions::reconcile_since(
                &lists,
                &config.internal.exclusions_seen,
                config.enabled_at,
                now,
            );
            config.internal.exclusions_seen = seen;
            exclusions::observe(&mut config.internal.exclusions_seen, elapsed_secs);
            Ok((expired, ready))
        })?;
        if !expired.is_empty() || !ready.is_empty() {
            outbox.events.push(NofapEvent::PendingChanged);
        }
        // Only "turn it off" is asked about in a card; other requests wait on the page.
        // The card is owed while the request is ready and unconfirmed: when another card
        // was up at the moment it became ready (or the other product ran the passes), it
        // is shown at the first later pass that may show one. Once per request in a run.
        let Runtime {
            config,
            ready_carded,
            ..
        } = &mut *rt;
        ready_carded.retain(|id| config.pending.iter().any(|waiting| &waiting.id == id));
        let owed = config
            .pending
            .iter()
            .find(|waiting| {
                waiting.change.kind() == ChangeKind::Disable
                    && !ready_carded.contains(&waiting.id)
                    && commitment::pending_view(waiting, now).ready
            })
            .map(|waiting| waiting.id.clone());
        if let (true, Some(id)) = (primary, owed) {
            if rt.cards.may_show(now) {
                let card = cards::card(
                    CardKind::PendingReady,
                    "pending.ready",
                    false,
                    vec![CardAction::KeepProtection, CardAction::TurnOff],
                    None,
                    now,
                );
                let context = CardContext {
                    pending_id: Some(id.clone()),
                    ..CardContext::default()
                };
                rt.cards.keep(card.clone(), context, now);
                rt.ready_carded.insert(id);
                outbox.cards.push((card, Some(Notice::Drift)));
            }
        }
        Ok(())
    }

    /// Ends snoozes that ran out and, once a local day, prunes old counters.
    fn housekeeping(&self, now: DateTime<Utc>) {
        let (today, _) = self.day_hour(now);
        let mut rt = self.rt.lock();
        let purged = self.deps.repo.purge_expired_sites(now).unwrap_or(0);
        // Lists that could not be read are read again until it works.
        if purged > 0 || rt.sites_error || rt.snooze_ends.is_some_and(|ends| ends <= now) {
            self.reload_sites(&mut rt);
        }
        if rt.pruned_on.as_deref() != Some(today.as_str()) {
            let cutoff = chrono::NaiveDate::parse_from_str(&today, progress::DAY_FORMAT)
                .ok()
                .and_then(|day| day.checked_sub_signed(chrono::Duration::days(RETENTION_DAYS)));
            if let Some(cutoff) = cutoff {
                if let Err(error) = self
                    .deps
                    .repo
                    .delete_days_before(&progress::day_key(cutoff))
                {
                    tracing::debug!(
                        class = error_class(&error),
                        "nofap: counters were not pruned"
                    );
                }
            }
            rt.pruned_on = Some(today);
        }
    }

    /// Looks at the machine (unprivileged) and judges what it finds. `full` asks for the
    /// full DNS read; `lookups` resolves the canary and the probe now where they apply
    /// (after an apply, when the page opens) instead of waiting for their turn.
    pub(crate) fn read_system(&self, full: bool, lookups: bool) -> CoreResult<()> {
        let now = self.now();
        let capabilities = self.deps.system.capabilities();
        let usable = Tier::ALL
            .iter()
            .any(|tier| !matches!(capabilities.of(*tier), TierCapability::Unsupported { .. }));
        let Some(os) = capabilities.os.filter(|_| usable) else {
            // No system layer here: there is nothing to look at.
            let mut rt = self.rt.lock();
            rt.system.assessment = None;
            rt.system.observation = None;
            return Ok(());
        };
        let gate = Gate::new(os, capabilities.unverified.clone());
        let (config, sites, opts) = {
            let rt = self.rt.lock();
            let memory = &rt.system;
            let found = memory.assessment.as_ref();
            let hosts_in_place = found.is_some_and(|found| in_place(&found.hosts));
            let quick_failed = memory.full_resolver.is_none()
                || memory
                    .observation
                    .as_ref()
                    .is_some_and(|seen| !seen.resolver.full && seen.resolver.error.is_some());
            let full_every = if quick_failed {
                FULL_READ_RETRY_SECS
            } else {
                FULL_READ_EVERY_SECS
            };
            let opts = ObserveOpts {
                full: full
                    || (os == Os::Windows
                        && (memory.full_at.is_none() || due(memory.full_at, full_every, now))),
                // The reserved name is looked up only while the block is in the file.
                canary: hosts_in_place
                    && (lookups || due(memory.canary_at, CANARY_EVERY_SECS, now)),
                probe: found.is_some_and(|found| found.resolver == TierState::Applied),
            };
            (rt.config.clone(), rt.sites.clone(), opts)
        };
        let mut observed = self.deps.system.observe(opts)?;
        let built = plan::build_plan(&config, &sites, self.deps.identity.id, os);

        let mut rt = self.rt.lock();
        self.carry(&mut rt, &mut observed, os, now);
        let judge = |rt: &Runtime, observed: &SystemObservation| {
            plan::assess(
                &AssessInput {
                    config: &config,
                    plan: &built.plan,
                    os,
                    gate: &gate,
                    hosts_tier: list::hosts_tier(),
                    probe_fail_streak: rt.system.probe_fail_streak,
                },
                observed,
            )
        };
        let mut found = judge(&rt, &observed);
        // A block that was just applied gets its canary looked up at once.
        let canary_now = lookups && in_place(&found.hosts) && !opts.canary;
        let probe_now = lookups && found.resolver == TierState::Applied && !opts.probe;
        if canary_now || probe_now {
            drop(rt);
            let again = self.deps.system.observe(ObserveOpts {
                full: false,
                canary: canary_now,
                probe: probe_now,
            })?;
            rt = self.rt.lock();
            observed.canary = again.canary.or(observed.canary);
            observed.probe_ok = again.probe_ok.or(observed.probe_ok);
            self.carry(&mut rt, &mut observed, os, now);
            found = judge(&rt, &observed);
        }
        if !in_place(&found.hosts) {
            // An answer from before the block went away says nothing about the next one.
            rt.system.canary = None;
            rt.system.canary_at = None;
        }
        rt.system.extras_left_out = built.extras_left_out;
        rt.system.assessment = Some(found);
        rt.system.observation = Some(observed);
        Ok(())
    }

    /// Puts what earlier looks know into an observation that did not look: the last
    /// canary answer (integrator decision 11) and, on Windows, the resolver part of the
    /// last full read when the quick one found nothing usable. Counts failed probes.
    fn carry(
        &self,
        rt: &mut Runtime,
        observed: &mut SystemObservation,
        os: Os,
        now: DateTime<Utc>,
    ) {
        let memory = &mut rt.system;
        match &observed.canary {
            Some(answers) => {
                memory.canary = Some(answers.clone());
                memory.canary_at = Some(now);
            }
            None => observed.canary = memory.canary.clone(),
        }
        if os == Os::Windows {
            if observed.resolver.full {
                memory.full_resolver = Some(observed.resolver.clone());
                memory.full_at = Some(now);
            } else if observed.resolver.error.is_some() {
                if let Some(full) = &memory.full_resolver {
                    observed.resolver = full.clone();
                }
            }
        }
        match observed.probe_ok.take() {
            Some(true) => memory.probe_fail_streak = 0,
            Some(false) => memory.probe_fail_streak = memory.probe_fail_streak.saturating_add(1),
            None => {}
        }
    }

    /// Once per drift, and at most once in six hours, a calm card that offers to restore;
    /// once per outage, the card that offers to give the network back. Nothing is called
    /// tampering, nothing is counted against the user and no timer starts.
    fn drift_and_outage(&self, now: DateTime<Utc>, outbox: &mut Outbox) -> CoreResult<()> {
        let mut rt = self.rt.lock();
        if rt.system.assessment.is_none() {
            return Ok(());
        }
        if !in_drift(&rt) {
            rt.system.in_drift = false;
            rt.system.drift_carded = false;
        } else {
            if !rt.system.in_drift {
                rt.system.in_drift = true;
                drop(rt);
                self.bump(DayKind::Drift);
                rt = self.rt.lock();
            }
            if !rt.system.drift_carded {
                let quiet = !due(
                    rt.config.internal.last_drift_card_at,
                    DRIFT_CARD_EVERY_SECS,
                    now,
                );
                if quiet {
                    rt.system.drift_carded = true;
                } else if rt.cards.may_show(now) {
                    // A card that cannot be shown now (another one is up) is tried
                    // again in the next pass.
                    rt.system.drift_carded = true;
                    let card = cards::card(
                        CardKind::Drift,
                        "drift.card",
                        false,
                        vec![CardAction::Restore, CardAction::Later],
                        None,
                        now,
                    );
                    rt.cards.keep(card.clone(), CardContext::default(), now);
                    outbox.cards.push((card, Some(Notice::Drift)));
                    // The card is shown whether or not its time can be stored: this run
                    // remembers it was shown, and the six quiet hours are a courtesy.
                    let stored = self.update_config(&mut rt, |config| {
                        config.internal.last_drift_card_at = Some(now);
                        Ok(())
                    });
                    if let Err(error) = stored {
                        tracing::debug!(
                            class = error_class(&error),
                            "nofap: the time of a card was not stored"
                        );
                    }
                }
            }
        }

        let unreachable = rt
            .system
            .assessment
            .as_ref()
            .is_some_and(|found| found.issues.contains(&Issue::ResolverUnreachable));
        if !unreachable {
            rt.system.unreachable_carded = false;
        } else if !rt.system.unreachable_carded && rt.cards.may_show(now) {
            rt.system.unreachable_carded = true;
            let card = cards::card(
                CardKind::ResolverUnreachable,
                "resolver.unreachable",
                false,
                vec![CardAction::RestoreNetwork, CardAction::Later],
                None,
                now,
            );
            rt.cards.keep(card.clone(), CardContext::default(), now);
            outbox.cards.push((card, Some(Notice::Drift)));
        }
        Ok(())
    }

    /// The reminder that the module is on (every thirty days) and the professional-help
    /// card (at most once in seven days, when the week's counters ask for it).
    fn reminders(&self, now: DateTime<Utc>, outbox: &mut Outbox) -> CoreResult<()> {
        let (today, _) = self.day_hour(now);
        let Ok(today_date) = chrono::NaiveDate::parse_from_str(&today, progress::DAY_FORMAT) else {
            return Ok(());
        };
        let week_start = today_date
            .checked_sub_signed(chrono::Duration::days(help::HELP_WINDOW_DAYS - 1))
            .map_or_else(|| today.clone(), progress::day_key);
        let counts = self.deps.repo.days_between(&week_start, &today)?;

        let mut rt = self.rt.lock();
        let since = rt.config.internal.last_reminder_at.or(rt.config.enabled_at);
        let remind = since.is_some_and(|since| (now - since).num_days() >= REMINDER_EVERY_DAYS);
        if remind && rt.cards.may_show(now) {
            // Stored first: a reminder whose time cannot be kept would come back with
            // every pass.
            self.update_config(&mut rt, |config| {
                config.internal.last_reminder_at = Some(now);
                Ok(())
            })?;
            let card = cards::card(
                CardKind::Reminder,
                "reminder.on",
                false,
                vec![CardAction::Ok, CardAction::Help],
                None,
                now,
            );
            rt.cards.keep(card.clone(), CardContext::default(), now);
            outbox.cards.push((card, Some(Notice::Drift)));
        }
        let help_due = help::help_due(&counts, rt.config.internal.last_help_at, today_date);
        if help_due && rt.cards.may_show(now) {
            self.update_config(&mut rt, |config| {
                config.internal.last_help_at = Some(now);
                Ok(())
            })?;
            let card = cards::card(
                CardKind::Help,
                "help.pattern",
                false,
                vec![CardAction::Help, CardAction::Ok],
                None,
                now,
            );
            rt.cards.keep(card.clone(), CardContext::default(), now);
            outbox.cards.push((card, Some(Notice::Drift)));
            drop(rt);
            self.bump(DayKind::HelpShown);
        }
        Ok(())
    }

    /// Reads the system for the page, at most once a minute and never in the way of the
    /// answer: `view` carries the last observation, and a fresh one arrives as a `status`
    /// event (integrator decisions 14 and 21). Nothing is read for a module that was
    /// never enabled.
    pub(crate) fn read_for_view(&self) {
        let now = self.now();
        {
            let mut rt = self.rt.lock();
            let ever_on = rt.config.enabled || rt.config.enabled_at.is_some();
            if rt.config_error
                || !ever_on
                || !due(rt.system.view_read_at, VIEW_READ_EVERY_SECS, now)
            {
                return;
            }
            rt.system.view_read_at = Some(now);
        }
        match self.deps.tuning.background {
            Background::Inline => self.read_now(),
            Background::Thread => {
                let Some(me) = self.me.upgrade() else { return };
                if self.reading.swap(true, std::sync::atomic::Ordering::SeqCst) {
                    return;
                }
                let spawned = std::thread::Builder::new()
                    .name("ubiqx-shield-read".into())
                    .spawn(move || {
                        me.read_now();
                        me.reading.store(false, std::sync::atomic::Ordering::SeqCst);
                    });
                if spawned.is_err() {
                    self.reading
                        .store(false, std::sync::atomic::Ordering::SeqCst);
                }
            }
        }
    }

    fn read_now(&self) {
        if let Err(error) = self.read_system(true, true) {
            tracing::debug!(
                class = error_class(&error),
                "nofap: the system was not read"
            );
        }
        self.refresh_status();
    }

    // -----------------------------------------------------------------------------------------
    // The tiers
    // -----------------------------------------------------------------------------------------

    /// The plan this product would send now and the gate it is sent through; `None` on a
    /// system without a back-end. "Aplicar" after "Restaurar minha rede" is "Religar", so
    /// the plan asks for the resolver again.
    fn plan_to_send(&self) -> Option<(SystemPlan, SystemPlan)> {
        let capabilities = self.deps.system.capabilities();
        let os = capabilities.os?;
        let gate = Gate::new(os, capabilities.unverified.clone());
        let (mut config, sites): (NofapConfig, Vec<NofapSite>) = {
            let rt = self.rt.lock();
            (rt.config.clone(), rt.sites.clone())
        };
        config.system.network_restored = false;
        let wanted = plan::build_plan(&config, &sites, self.deps.identity.id, os).plan;
        let sent = plan::without_gated(&wanted, &gate);
        Some((wanted, sent))
    }

    /// The exact list of changes "Aplicar" would make, for the user to read before the
    /// operating system asks for the password. With the module off: what removing takes
    /// away.
    pub fn system_preview(&self) -> CoreResult<Vec<PlannedChange>> {
        let enabled = self.enabled();
        let plan = match self.plan_to_send() {
            Some((_, sent)) if enabled => sent,
            Some(_) => empty_plan(self.deps.identity.id),
            None => return Ok(Vec::new()),
        };
        self.deps.system.preview(&plan)
    }

    /// "Aplicar", "Restaurar" and "Religar": applies the tiers the user chose, through the
    /// operating system's prompt. Blocks for as long as the prompt is open; the runtime
    /// lock is not held meanwhile.
    pub fn system_apply(&self) -> CoreResult<ApplyOutcome> {
        {
            let rt = self.rt.lock();
            if rt.config_error {
                return Err(unreadable_config());
            }
            if !rt.config.enabled {
                return Err(CoreError::Invalid("only while the module is on".into()));
            }
        }
        let _prompt = self.one_prompt()?;
        let owner = self.deps.identity.id;
        let Some((wanted, sent)) = self.plan_to_send() else {
            return Ok(ApplyOutcome::Unavailable(reason::UNSUPPORTED_OS.into()));
        };
        let before = self.deps.system.observe(ObserveOpts::FULL)?;
        let listed = before
            .state
            .as_ref()
            .and_then(|state| state.owners.get(&owner));
        let outcome = if !asks_nothing(&sent) {
            if sent.resolver.is_some() && listed.is_none_or(|wants| wants.resolver.is_none()) {
                self.remember_dns(&before)?;
            }
            self.deps.system.apply(&sent)?
        } else if listed.is_some() {
            // Nothing is asked for any more: what is still there for this product goes.
            self.deps.system.remove(owner, RemoveScope::All)?
        } else if !asks_nothing(&wanted) {
            ApplyOutcome::Unavailable(reason::GATED.into())
        } else {
            ApplyOutcome::Applied
        };
        if matches!(outcome, ApplyOutcome::Applied | ApplyOutcome::Partial(_)) {
            tracing::info!("nofap: the system tiers were applied");
            let mut rt = self.rt.lock();
            // The tiers are on the machine either way: a note that cannot be stored must
            // not keep the outcome and the fresh read from the page.
            let stored = self.update_config(&mut rt, |config| {
                config.system.network_restored = false;
                Ok(())
            });
            if let Err(error) = stored {
                tracing::debug!(
                    class = error_class(&error),
                    "nofap: the configuration was not stored"
                );
            }
        }
        self.read_system(true, true)?;
        self.end_repaired_drift();
        self.refresh_status();
        Ok(outcome)
    }

    /// A drift ends when the machine is read without one, also right after "Restaurar":
    /// what is taken out again before the next verify pass is a drift of its own.
    fn end_repaired_drift(&self) {
        let mut rt = self.rt.lock();
        if !in_drift(&rt) {
            rt.system.in_drift = false;
            rt.system.drift_carded = false;
        }
    }

    /// Records the DNS every interface had before the resolver tier is applied, also the
    /// ones on automatic DNS. For display, and so that an interface that appears later is
    /// known to be new; nothing recorded here is ever written back to the system.
    fn remember_dns(&self, observed: &SystemObservation) -> CoreResult<()> {
        let previous: Vec<PreviousDns> = observed
            .resolver
            .interfaces
            .iter()
            .map(|interface| PreviousDns {
                interface: interface.id.clone(),
                servers: interface.servers.clone(),
            })
            .collect();
        let mut rt = self.rt.lock();
        self.update_config(&mut rt, |config| {
            config.internal.previous_dns = previous;
            Ok(())
        })
    }

    /// "Remover do sistema": takes this product's tiers off the machine. Only while the
    /// module is off; turning it off is what waits the delay.
    pub fn system_remove(&self) -> CoreResult<ApplyOutcome> {
        if self.enabled() {
            return Err(CoreError::Invalid("only while the module is off".into()));
        }
        self.remove_all()
    }

    pub(crate) fn remove_all(&self) -> CoreResult<ApplyOutcome> {
        let _prompt = self.one_prompt()?;
        let outcome = self
            .deps
            .system
            .remove(self.deps.identity.id, RemoveScope::All)?;
        if matches!(outcome, ApplyOutcome::Applied | ApplyOutcome::Partial(_)) {
            tracing::info!("nofap: the system tiers were removed");
        }
        self.read_system(true, false)?;
        self.refresh_status();
        Ok(outcome)
    }

    /// "Restaurar minha rede": DNS goes back to automatic where it is still ours; the
    /// other tiers stay. Never delayed (a broken network is a safety problem) and not
    /// counted as an override.
    pub fn restore_network(&self) -> CoreResult<ApplyOutcome> {
        if self.rt.lock().config_error {
            return Err(unreadable_config());
        }
        let _prompt = self.one_prompt()?;
        let outcome = self
            .deps
            .system
            .remove(self.deps.identity.id, RemoveScope::Resolver)?;
        if matches!(outcome, ApplyOutcome::Applied | ApplyOutcome::Partial(_)) {
            tracing::info!("nofap: the network was restored");
            {
                let mut rt = self.rt.lock();
                rt.system.probe_fail_streak = 0;
                // DNS is back to automatic either way; when that cannot be stored the
                // next look reports the resolver as missing, which is the truth.
                let stored = self.update_config(&mut rt, |config| {
                    config.system.network_restored = true;
                    Ok(())
                });
                if let Err(error) = stored {
                    tracing::debug!(
                        class = error_class(&error),
                        "nofap: the configuration was not stored"
                    );
                }
            }
            self.bump(DayKind::NetworkRestore);
        }
        self.read_system(true, false)?;
        self.refresh_status();
        Ok(outcome)
    }

    /// Whether at least one system tier is in place: which of the two quit sentences is
    /// true (`quit.confirm_system` or `quit.confirm_none`). When the machine was not
    /// looked at yet in this run (a quit right after the start), it is looked at now.
    pub fn system_applied(&self) -> bool {
        let unknown = {
            let rt = self.rt.lock();
            (rt.config.enabled || rt.config.enabled_at.is_some())
                && !rt.config_error
                && rt.system.observation.is_none()
        };
        if unknown && self.read_system(false, false).is_ok() {
            self.refresh_status();
        }
        let rt = self.rt.lock();
        let status = &rt.status;
        in_place(&status.hosts) || in_place(&status.resolver) || in_place(&status.policies)
    }

    // -----------------------------------------------------------------------------------------
    // Two products on one machine
    // -----------------------------------------------------------------------------------------

    /// Which of the two products runs the passes. Every thirty seconds the lock file both
    /// share is read; as primary this process then writes its own heartbeat. Without a
    /// shared directory there is no lock and this process is always primary.
    pub(crate) fn role_at(&self, now: DateTime<Utc>) -> Role {
        let Some(dir) = &self.deps.shared_dir else {
            return Role::Primary;
        };
        // One decision at a time: a role decided from a lock read before the other
        // loop's write must not overwrite the newer one.
        let _deciding = self.role_lock.lock();
        let holding = {
            let rt = self.rt.lock();
            if !rt.config.enabled || rt.config_error {
                // Switched off since this pass began (the lock was given up under
                // `role_lock`): a module that is off writes no heartbeat, or the other
                // product would stand back for one nobody runs. The pass does nothing.
                return Role::Secondary;
            }
            if !due(rt.heartbeat_at, HEARTBEAT_EVERY_SECS, now) {
                return rt.role;
            }
            rt.holding
        };
        let path = dir.join(LOCK_FILE);
        let lock = std::fs::read_to_string(&path)
            .ok()
            .and_then(|text| primary::parse_lock(&text));
        let me = self.deps.identity.id;
        let role = if holding {
            primary::decide_holding(lock.as_ref(), me, now)
        } else {
            primary::decide(lock.as_ref(), me, now)
        };
        if role == Role::Primary {
            let heartbeat = PrimaryLock {
                product: me,
                pid: std::process::id(),
                heartbeat_at: now,
            };
            let written = std::fs::create_dir_all(dir)
                .and_then(|()| std::fs::write(&path, primary::render_lock(&heartbeat)));
            if written.is_err() {
                tracing::debug!("nofap: the primary lock was not written");
            }
        }
        let changed = {
            let mut rt = self.rt.lock();
            let changed = rt.role != role;
            rt.role = role;
            rt.holding = role == Role::Primary;
            rt.heartbeat_at = Some(now);
            changed
        };
        if changed {
            tracing::info!(primary = role == Role::Primary, "nofap: the role changed");
            self.refresh_status();
        }
        role
    }

    /// Deletes the lock when it is this product's: on disable and on quit, so "the other
    /// app" is never one whose module is off.
    pub fn release_primary(&self) {
        let Some(dir) = &self.deps.shared_dir else {
            return;
        };
        let _deciding = self.role_lock.lock();
        {
            let mut rt = self.rt.lock();
            rt.heartbeat_at = None;
            rt.role = Role::Primary;
            if !std::mem::take(&mut rt.holding) {
                return;
            }
        }
        let path = dir.join(LOCK_FILE);
        let mine = std::fs::read_to_string(&path)
            .ok()
            .and_then(|text| primary::parse_lock(&text))
            .is_some_and(|lock| lock.product == self.deps.identity.id);
        if mine {
            let _ = std::fs::remove_file(&path);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::TimeZone;
    use ubiqx_ai::fake::MemoryUsageRepo;
    use ubiqx_core::clock::FixedClock;
    use ubiqx_core::nofap::{AiReadiness, NofapEnv};
    use ubiqx_core::nofap_ipc::{EnableOptions, VisionScope};
    use ubiqx_core::ports::{
        ActivitySource, BrowserUrlResolver, Notifier, PermissionChecker, PermissionKind,
        PermissionStatus,
    };
    use ubiqx_core::{AiProvider, ForegroundWindow, UiLanguage};
    use ubiqx_platform::mock::MockEnforcer;

    use super::*;
    use crate::deps::NofapPorts;
    use crate::nofap::{DisplaySession, NofapDeps, NofapHost, Tuning};

    /// The host's ports with nothing behind them: no window in front, no key.
    struct Nothing;

    impl NofapEnv for Nothing {
        fn snapshot(&self) -> NofapEnvSnapshot {
            NofapEnvSnapshot {
                language: UiLanguage::En,
                provider: AiProvider::OpenAi,
                vision_model: String::new(),
                ai: AiReadiness::NoKey,
                excluded_apps: Vec::new(),
                excluded_domains: Vec::new(),
            }
        }
    }

    impl ActivitySource for Nothing {
        fn foreground(&self) -> CoreResult<Option<ForegroundWindow>> {
            Ok(None)
        }
    }

    impl BrowserUrlResolver for Nothing {
        fn supports(&self, _app_id: &str) -> bool {
            false
        }

        fn resolve(&self, _window: &ForegroundWindow) -> CoreResult<Option<String>> {
            Ok(None)
        }
    }

    impl PermissionChecker for Nothing {
        fn status(&self) -> PermissionStatus {
            PermissionStatus {
                screen_recording: PermissionState::Granted,
                automation: PermissionState::Granted,
                accessibility: PermissionState::Granted,
            }
        }

        fn request(&self, _kind: PermissionKind) -> CoreResult<()> {
            Ok(())
        }
    }

    impl Notifier for Nothing {
        fn notify(&self, _title: &str, _body: &str) -> CoreResult<()> {
            Ok(())
        }
    }

    /// The race this guards against cannot be staged through the public calls (a pass
    /// checks "on" and asks for its role without a port call in between), so the role is
    /// asked for directly, in the state a pass finds when the module was switched off
    /// right after its check.
    #[test]
    fn a_module_that_is_off_writes_no_heartbeat() {
        let tmp = tempfile::tempdir().unwrap();
        let shared = tmp.path().join("shared");
        let lock = shared.join(LOCK_FILE);
        let nothing = Arc::new(Nothing);
        let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 9, 17, 15, 0, 0).unwrap());
        let service = NofapService::new(NofapDeps::assemble(
            NofapPorts {
                shared_dir: Some(shared),
                tuning: Tuning {
                    background: Background::Inline,
                    display: DisplaySession::Full,
                    ..Tuning::default()
                },
                ..NofapPorts::inert()
            },
            NofapHost {
                env: nothing.clone(),
                activity: nothing.clone(),
                urls: nothing.clone(),
                enforcer: Arc::new(MockEnforcer::default()),
                permissions: nothing.clone(),
                notifier: nothing,
                usage: Arc::new(MemoryUsageRepo::new()),
                clock: Arc::new(clock),
                identity: &ubiqx_core::product::SHIELD,
                export_dir: tmp.path().join("exports"),
            },
        ));
        service
            .enable(EnableOptions {
                self_use: true,
                hosts: false,
                resolver: None,
                policies: false,
                vision: false,
                vision_consent: None,
                vision_scope: VisionScope {
                    unknown_sites: true,
                    media_apps: true,
                    messaging_apps: false,
                    private_windows: true,
                },
                delay_secs: 0,
                values: String::new(),
                discreet: true,
            })
            .unwrap();
        let now = service.now();
        assert_eq!(service.role_at(now), Role::Primary);
        assert!(lock.exists(), "the product that runs the passes says so");

        // "Turn it off" lands between a pass's check and its question: the module goes
        // off, then the lock is given up (the order `carry_out` keeps).
        service.rt.lock().config.enabled = false;
        service.release_primary();
        assert!(!lock.exists());
        assert_eq!(
            service.role_at(now),
            Role::Secondary,
            "the pass does nothing"
        );
        assert!(!lock.exists(), "a module that is off wrote a heartbeat");

        // On again, it takes the role back.
        service.rt.lock().config.enabled = true;
        assert_eq!(service.role_at(now), Role::Primary);
        assert!(lock.exists());
    }
}
