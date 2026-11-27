//! What the page and the card window ask of the service, and the two questions ubiqX's
//! tracker asks it.
//!
//! The configuration changes only here, through typed requests: a request that weakens
//! protection waits the delay the user chose and then needs a fresh confirmation
//! (`ubiqx_nofap::commitment` decides which is which). The exits that are never delayed
//! are their own calls: cancelling a request, restoring the network, erasing one's own
//! history.

use base64::Engine as _;
use chrono::{DateTime, Duration, NaiveDate, Utc};
use ubiqx_core::nofap::{
    CardAction, CardKind, DayCount, DayKind, NofapCard, NofapChange, NofapConfig, NofapEvent,
    NofapSite, SiteList, SiteOrigin, VisionConsent, DISPLAY_NAME,
};
use ubiqx_core::nofap_ipc::{
    ChangeOutcome, ConfigView, ConsentArgs, CostLine, EnableOptions, ExportResult, Mood, NofapView,
    ProgressView, SampleFrame, SiteEntry, SitesView, SupportView, TodayCounts, Trigger, UserEvent,
};
use ubiqx_core::{ActivitySample, CoreError, CoreResult};
use ubiqx_nofap::commitment::{self, Requested};
use ubiqx_nofap::exclusions::{self, ExclusionLists};
use ubiqx_nofap::heuristics;
use ubiqx_nofap::matcher::{normalize_host, site_key, BlockSource, HostClass, Verdict};
use ubiqx_nofap::progress::{self, PROGRESS_WINDOW_DAYS};
use ubiqx_nofap::sampling::{self, NotItEffect, SampleEvent, SNOOZE_SECS};
use ubiqx_nofap::{help, messages};

use super::cards::{self, CardContext, Outbox, AWAY_FLOW_SECS, SURF_FLOW_SECS};
use super::guard::page_of;
use super::{
    context_key, ensure_salt, error_class, recent, salt_of, unreadable_config, NofapService,
    Runtime, SystemMemory,
};

/// Every counter there is: the range "Exportar dados" asks the repository for.
const FIRST_DAY: &str = "0000-01-01";
const LAST_DAY: &str = "9999-12-31";

fn day_of(day: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(day, progress::DAY_FORMAT).ok()
}

fn trigger_kind(trigger: Trigger) -> DayKind {
    match trigger {
        Trigger::Tired => DayKind::TriggerTired,
        Trigger::Bored => DayKind::TriggerBored,
        Trigger::Stressed => DayKind::TriggerStressed,
        Trigger::Lonely => DayKind::TriggerLonely,
        Trigger::Hungry => DayKind::TriggerHungry,
        Trigger::Angry => DayKind::TriggerAngry,
        Trigger::Other => DayKind::TriggerOther,
    }
}

fn mood_kind(mood: Mood) -> DayKind {
    match mood {
        Mood::Ok => DayKind::CheckinOk,
        Mood::Meh => DayKind::CheckinMeh,
        Mood::Hard => DayKind::CheckinHard,
        Mood::VeryBad => DayKind::CheckinVeryBad,
    }
}

/// The host of a request as it is stored and matched: one spelling, with a dot in it.
fn typed_host(host: &str) -> CoreResult<String> {
    normalize_host(host)
        .filter(|name| name.contains('.'))
        .ok_or_else(|| CoreError::Invalid("that is not the name of a site".into()))
}

impl NofapService {
    // -----------------------------------------------------------------------------------------
    // The page
    // -----------------------------------------------------------------------------------------

    /// Everything the page shows, answered from memory. The system is read behind it, at
    /// most once a minute, and what it finds arrives as a `status` event.
    pub fn view(&self) -> CoreResult<NofapView> {
        self.read_for_view();
        self.view_now()
    }

    /// The view without asking for a system read (the answer to a command that just
    /// changed something).
    fn view_now(&self) -> CoreResult<NofapView> {
        let now = self.now();
        let status = self.refresh_status();
        let env = self.deps.env.snapshot();
        let (day, _) = self.day_hour(now);
        let counts = self.deps.repo.days_between(&day, &day)?;
        let blocks = [
            DayKind::BlockList,
            DayKind::BlockUser,
            DayKind::BlockKeyword,
        ]
        .into_iter()
        .fold(0u32, |sum, kind| {
            sum.saturating_add(progress::total_on(&counts, &day, kind))
        });
        let (month_usd, month_checks) = self.month_spend(now);
        let capabilities = self.deps.system.capabilities();
        let rt = self.rt.lock();
        Ok(NofapView {
            status,
            config: ConfigView::from(&rt.config),
            capabilities,
            today: TodayCounts {
                blocks,
                checks: progress::total_on(&counts, &day, DayKind::VisionCheck),
            },
            display_name: DISPLAY_NAME.to_owned(),
            product: self.deps.identity.id,
            provider: env.provider,
            ai: env.ai,
            cost_line: CostLine {
                month_usd,
                month_checks,
                cap_usd: rt.config.vision.monthly_cap_usd,
            },
        })
    }

    /// The wizard's last button. Nothing was applied before it, and nothing in it is
    /// delayed: the delay starts once the module is on. Raises the operating system's
    /// prompt when a system tier was chosen and is open here.
    pub fn enable(&self, options: EnableOptions) -> CoreResult<NofapView> {
        if !options.self_use {
            return Err(CoreError::Invalid(
                "the tick \"it is for me, on my own computer\" is required".into(),
            ));
        }
        if options.vision && options.vision_consent.is_none() {
            return Err(CoreError::Invalid("the AI check needs consent".into()));
        }
        let now = self.now();
        let env = self.deps.env.snapshot();
        {
            let mut rt = self.rt.lock();
            if rt.config.enabled && !rt.config_error {
                // Running the wizard over a module that is on would be an off switch
                // without a delay.
                return Err(CoreError::Invalid(
                    "the module is already on; change it through requests".into(),
                ));
            }
            let mut config = if rt.config_error {
                NofapConfig::default()
            } else {
                rt.config.clone()
            };
            config.enabled = true;
            config.enabled_at = Some(now);
            config.self_use_at = Some(now);
            config.system.hosts = options.hosts;
            config.system.resolver = options.resolver;
            config.system.policies = options.policies;
            config.system.network_restored = false;
            config.vision.enabled = options.vision;
            if let Some(consent) = options.vision_consent {
                config.vision.consent = Some(VisionConsent {
                    provider: consent.provider,
                    managed: consent.managed,
                    at: now,
                });
            }
            config.vision.unknown_sites = options.vision_scope.unknown_sites;
            config.vision.media_apps = options.vision_scope.media_apps;
            config.vision.messaging_apps = options.vision_scope.messaging_apps;
            config.vision.private_windows = options.vision_scope.private_windows;
            config.delay_secs = options.delay_secs;
            config.values = options.values;
            config.ui.discreet = options.discreet;
            config.pending.clear();
            ensure_salt(&mut config);
            // Privacy-list entries that are there now count at once; later ones wait.
            let lists = ExclusionLists {
                apps: env.excluded_apps,
                domains: env.excluded_domains,
            };
            config.internal.exclusions_seen = exclusions::reconcile(&lists, &[], now);
            config.internal.last_reminder_at = None;
            self.replace_config(&mut rt, config)?;
            self.reload_sites(&mut rt);
            rt.system.view_read_at = None;
        }
        self.clear_memory();
        tracing::info!("nofap: enabled");
        if options.hosts || options.resolver.is_some() || options.policies {
            // A prompt the user declines, or a system that cannot be read, leaves the
            // tiers "not applied"; the module is on either way and the page says so.
            if let Err(error) = self.system_apply() {
                tracing::debug!(
                    class = error_class(&error),
                    "nofap: the tiers were not applied"
                );
            }
        } else if let Err(error) = self.read_system(true, false) {
            tracing::debug!(
                class = error_class(&error),
                "nofap: the system was not read"
            );
        }
        self.view_now()
    }

    /// Records that the user read about the AI check with this provider and agreed.
    pub fn consent_vision(&self, args: ConsentArgs) -> CoreResult<NofapView> {
        let now = self.now();
        {
            let mut rt = self.rt.lock();
            self.update_config(&mut rt, |config| {
                config.vision.consent = Some(VisionConsent {
                    provider: args.provider,
                    managed: args.managed,
                    at: now,
                });
                Ok(())
            })?;
        }
        self.view_now()
    }

    /// A change to the configuration or to the lists: applied now when it tightens or
    /// only concerns the user's own words, queued for the delay when it loosens. Asking
    /// again for a change that already waits answers with the waiting request (integrator
    /// decision 22).
    pub fn request(&self, change: NofapChange) -> CoreResult<ChangeOutcome> {
        let now = self.now();
        let change = match change {
            NofapChange::AddBlock { host } => NofapChange::AddBlock {
                host: typed_host(&host)?,
            },
            NofapChange::AddAllow { host } => NofapChange::AddAllow {
                host: typed_host(&host)?,
            },
            other => other,
        };
        let (requested, new) = {
            let mut rt = self.rt.lock();
            if rt.config_error {
                return Err(unreadable_config());
            }
            self.known_site(&rt, &change)?;
            let id = ubiqx_core::new_id();
            let requested = self.update_config(&mut rt, |config| {
                commitment::request(config, change, id.clone(), now)
            })?;
            // A change that already waits comes back with the id it has had all along.
            let new = matches!(&requested, Requested::Pending(waiting) if waiting.id == id);
            (requested, new)
        };
        match requested {
            Requested::Apply(change) => {
                self.apply_change(&change)?;
                Ok(ChangeOutcome::Applied(Box::new(self.view_now()?)))
            }
            Requested::Pending(waiting) => {
                if new {
                    // Each loosening request counts once; asking again does not.
                    self.bump(DayKind::OverrideRequest);
                    self.flush(Outbox {
                        events: vec![NofapEvent::PendingChanged],
                        status: true,
                        ..Outbox::default()
                    });
                }
                Ok(ChangeOutcome::Pending(commitment::pending_view(
                    &waiting, now,
                )))
            }
        }
    }

    /// A request that removes a site must name one the user typed.
    fn known_site(&self, rt: &Runtime, change: &NofapChange) -> CoreResult<()> {
        let (id, list) = match change {
            NofapChange::RemoveBlock { id } => (id, SiteList::Block),
            NofapChange::RemoveAllow { id } => (id, SiteList::Allow),
            _ => return Ok(()),
        };
        let known = rt
            .sites
            .iter()
            .any(|site| &site.id == id && site.list == list && site.origin == SiteOrigin::User);
        if known {
            Ok(())
        } else {
            Err(CoreError::NotFound("no such site on the list".into()))
        }
    }

    /// Makes a change take effect: the configuration part first (stored before it counts,
    /// see [`NofapService::update_config`]), then everything else.
    fn apply_change(&self, change: &NofapChange) -> CoreResult<()> {
        let now = self.now();
        let was_on = {
            let mut rt = self.rt.lock();
            let was_on = rt.config.enabled;
            self.update_config(&mut rt, |config| {
                commitment::apply_to_config(config, change, now);
                Ok(())
            })?;
            was_on
        };
        self.carry_out(change, was_on)
    }

    /// What a change does besides the configuration, which is already stored: the list
    /// part, a fresh look at the machine, and for "turn it off" the end of the passes and
    /// the removal of the system tiers. `was_on`: the module was on before the change.
    fn carry_out(&self, change: &NofapChange, was_on: bool) -> CoreResult<()> {
        let now = self.now();
        let mut outbox = Outbox {
            status: true,
            ..Outbox::default()
        };
        {
            let mut rt = self.rt.lock();
            let sites_changed = match change {
                NofapChange::AddBlock { host } => {
                    self.add_site(&mut rt, SiteList::Block, host, now)?
                }
                NofapChange::AddAllow { host } => {
                    self.add_site(&mut rt, SiteList::Allow, host, now)?
                }
                NofapChange::RemoveBlock { id } | NofapChange::RemoveAllow { id } => {
                    self.deps.repo.delete_site(id)?;
                    true
                }
                NofapChange::ForgetLearned => {
                    self.deps.repo.delete_sites_by_origin(SiteOrigin::Learned)?;
                    true
                }
                _ => false,
            };
            if sites_changed {
                self.reload_sites(&mut rt);
                outbox.events.push(NofapEvent::SitesChanged);
            }
            if matches!(change, NofapChange::Disable) && was_on {
                outbox.events.push(NofapEvent::PendingChanged);
            }
        }
        // A choice about the system tiers, or a site that belongs in the hosts block,
        // changes what the machine is compared with: look again (unprivileged).
        let system_changed = matches!(
            change,
            NofapChange::SetHosts { .. }
                | NofapChange::SetResolver { .. }
                | NofapChange::SetPolicies { .. }
                | NofapChange::SetHostsUserSites { .. }
                | NofapChange::AddBlock { .. }
                | NofapChange::RemoveBlock { .. }
        );
        if system_changed && self.enabled() {
            if let Err(error) = self.read_system(false, false) {
                tracing::debug!(
                    class = error_class(&error),
                    "nofap: the system was not read"
                );
            }
        }
        // A tier switched off by a confirmed request comes off the machine now, through
        // the operating system's prompt (the person just confirmed it): the plan sent is
        // the smaller one, or a removal when nothing is asked any more. A resolver taken
        // off with "Restaurar minha rede" stays off. A declined prompt leaves the tier
        // there, and the page offers "Aplicar" for it.
        let tier_off = matches!(
            change,
            NofapChange::SetHosts { on: false }
                | NofapChange::SetResolver { resolver: None }
                | NofapChange::SetPolicies { on: false }
        );
        if tier_off && self.enabled() && self.listed_on_system() {
            if let Err(error) = self.system_restore() {
                tracing::debug!(
                    class = error_class(&error),
                    "nofap: the tier was not taken off"
                );
            }
        }
        if matches!(change, NofapChange::Disable) {
            // L1 and L5 stop first; then the operating system is asked to take L2 to L4
            // off. A declined prompt leaves them applied, and the page says so.
            tracing::info!("nofap: disabled");
            self.release_primary();
            self.clear_memory();
            // What the state file lists for this product comes off; when the machine was
            // never looked at in this run, it is looked at now.
            if self.listed_now() {
                // The module is off whatever the removal does: a prompt that fails leaves
                // the tiers on the machine, the page says so and offers to remove them.
                if let Err(error) = self.remove_all() {
                    tracing::debug!(
                        class = error_class(&error),
                        "nofap: the tiers were not removed"
                    );
                }
            }
        }
        self.flush(outbox);
        Ok(())
    }

    /// Stores a site the user typed, in plain text (they read and edit their own list).
    /// `false` when it was already there.
    fn add_site(
        &self,
        rt: &mut Runtime,
        list: SiteList,
        host: &str,
        now: DateTime<Utc>,
    ) -> CoreResult<bool> {
        self.update_config(rt, |config| {
            ensure_salt(config);
            Ok(())
        })?;
        let host_key = site_key(&salt_of(&rt.config), host);
        let known = rt.sites.iter().any(|site| {
            site.list == list && site.origin == SiteOrigin::User && site.host_key == host_key
        });
        if known {
            return Ok(false);
        }
        self.deps.repo.upsert_site(&NofapSite {
            id: ubiqx_core::new_id(),
            list,
            origin: SiteOrigin::User,
            host: Some(host.to_owned()),
            host_key,
            created_at: now,
            expires_at: None,
        })?;
        Ok(true)
    }

    /// The fresh confirmation of a request that waited its delay.
    pub fn confirm_pending(&self, id: &str) -> CoreResult<NofapView> {
        let now = self.now();
        let (change, was_on) = {
            let mut rt = self.rt.lock();
            let was_on = rt.config.enabled;
            // One write: the request leaves the queue and its change enters the
            // configuration together, or neither does. A request that was taken out but
            // not applied (or applied in memory only) would be worse than an error.
            let change = self.update_config(&mut rt, |config| {
                let change = commitment::confirm(config, id, now)?;
                commitment::apply_to_config(config, &change, now);
                Ok(change)
            })?;
            (change, was_on)
        };
        self.carry_out(&change, was_on)?;
        self.flush(Outbox {
            events: vec![NofapEvent::PendingChanged],
            ..Outbox::default()
        });
        self.view_now()
    }

    /// "Manter proteção": always immediate, it only tightens.
    pub fn cancel_pending(&self, id: &str) -> CoreResult<NofapView> {
        {
            let mut rt = self.rt.lock();
            self.update_config(&mut rt, |config| {
                if commitment::cancel(config, id) {
                    Ok(())
                } else {
                    Err(CoreError::NotFound("no such waiting request".into()))
                }
            })?;
        }
        self.bump(DayKind::OverrideCancel);
        self.flush(Outbox {
            events: vec![NofapEvent::PendingChanged],
            ..Outbox::default()
        });
        self.view_now()
    }

    /// The lists the user reads and edits. A learned site is a count, never a name.
    pub fn sites(&self) -> SitesView {
        let rt = self.rt.lock();
        let typed = |list: SiteList| -> Vec<SiteEntry> {
            rt.sites
                .iter()
                .filter(|site| site.list == list && site.origin == SiteOrigin::User)
                .filter_map(|site| {
                    Some(SiteEntry {
                        id: site.id.clone(),
                        host: site.host.clone()?,
                        created_at: site.created_at,
                    })
                })
                .collect()
        };
        let learned = rt
            .sites
            .iter()
            .filter(|site| site.origin == SiteOrigin::Learned)
            .count();
        SitesView {
            blocked: typed(SiteList::Block),
            allowed: typed(SiteList::Allow),
            learned_count: u32::try_from(learned).unwrap_or(u32::MAX),
        }
    }

    /// The "Apoio" tab: the user's own words, progress as they defined it, today's
    /// check-in and whether the help card is due.
    pub fn support(&self) -> CoreResult<SupportView> {
        let now = self.now();
        let (today, _) = self.day_hour(now);
        let today_date = day_of(&today).unwrap_or_else(|| now.date_naive());
        let from = today_date
            .checked_sub_signed(Duration::days(i64::from(PROGRESS_WINDOW_DAYS) - 1))
            .map_or_else(|| today.clone(), progress::day_key);
        let counts = self.deps.repo.days_between(&from, &today)?;
        let rt = self.rt.lock();
        let since = rt.config.enabled_at.map(|at| {
            let (day, _) = self.day_hour(at);
            day_of(&day).unwrap_or_else(|| at.date_naive())
        });
        Ok(SupportView {
            values: rt.config.values.clone(),
            plans: rt.config.plans.clone(),
            progress: ProgressView {
                window_days: PROGRESS_WINDOW_DAYS,
                days: progress::days_your_way(&counts, since, today_date, PROGRESS_WINDOW_DAYS),
                streak: rt
                    .config
                    .ui
                    .show_streak
                    .then(|| progress::streak(&counts, since, today_date)),
                after_reset: progress::lapse_in_window(&counts, today_date, PROGRESS_WINDOW_DAYS),
            },
            checkin_today: checkin_of(&counts, &today),
            help_due: help::help_due(&counts, rt.config.internal.last_help_at, today_date),
            resources: help::resource_views(),
        })
    }

    /// Something the user did in a flow: a counter of the hour, never a stored event.
    pub fn record(&self, event: UserEvent) -> CoreResult<()> {
        let now = self.now();
        match event {
            UserEvent::Lapse { triggers } => {
                self.bump(DayKind::Lapse);
                triggers
                    .into_iter()
                    .for_each(|trigger| self.bump(trigger_kind(trigger)));
            }
            UserEvent::Checkin { mood, triggers } => {
                self.bump(mood_kind(mood));
                triggers
                    .into_iter()
                    .for_each(|trigger| self.bump(trigger_kind(trigger)));
            }
            UserEvent::SurfStarted => {
                self.bump(DayKind::SurfStarted);
                self.rt.lock().cards.start_flow(SURF_FLOW_SECS, now);
            }
            UserEvent::SurfDone => {
                self.bump(DayKind::SurfDone);
                self.rt.lock().cards.end_flow();
            }
            UserEvent::Panic => self.bump(DayKind::Panic),
            UserEvent::StepAway => {
                // Ten minutes away from the screen: the other windows are hidden, which
                // the user can undo at any moment. Nothing is locked.
                self.rt.lock().cards.start_flow(AWAY_FLOW_SECS, now);
                let _ = self.deps.enforcer.hide_others(self.deps.identity.bundle_id);
            }
        }
        Ok(())
    }

    // -----------------------------------------------------------------------------------------
    // The card window
    // -----------------------------------------------------------------------------------------

    /// The card with this id; the neutral discreet card for an id nobody knows.
    pub fn card(&self, id: &str) -> NofapCard {
        let now = self.now();
        self.rt
            .lock()
            .cards
            .get(id)
            .unwrap_or_else(|| cards::neutral(id, now))
    }

    /// A button of a card. The window calls this for every button and then runs its own
    /// flow; a card whose ten minutes are over, or an id nobody knows, does nothing.
    pub fn card_action(&self, id: &str, action: CardAction) -> CoreResult<()> {
        let now = self.now();
        let Some((kind, context)) = self.rt.lock().cards.context(id, now) else {
            return Ok(());
        };
        let vision_card = matches!(kind, CardKind::VisionClosed | CardKind::VisionAsk);
        // The window stays open for what it shows next after a correction or an addition;
        // every other button ends the card.
        let stays = matches!(
            action,
            CardAction::NotIt | CardAction::RequestException | CardAction::AddToList
        );
        if stays && self.rt.lock().cards.was_used(id, action) {
            return Ok(());
        }
        match action {
            CardAction::Support | CardAction::Surf => {
                // The window records the flow itself; here it only counts as busy.
                self.rt.lock().cards.start_flow(SURF_FLOW_SECS, now);
                return Ok(());
            }
            CardAction::Help => return Ok(()),
            CardAction::Ok | CardAction::Later => {}
            CardAction::CloseWindow => self.close_window(&context),
            CardAction::NotIt if vision_card => self.not_it_on_vision(&context)?,
            CardAction::NotIt => self.not_it_on_block(&context)?,
            CardAction::RequestException => self.request_exception(&context)?,
            CardAction::AddToList => {
                if let Some(host) = &context.host {
                    self.request(NofapChange::AddBlock { host: host.clone() })?;
                }
            }
            CardAction::Restore => {
                self.system_restore()?;
            }
            CardAction::RestoreNetwork => {
                self.restore_network()?;
            }
            CardAction::KeepProtection => {
                if let Some(pending) = &context.pending_id {
                    self.cancel_pending(pending)?;
                }
            }
            CardAction::TurnOff => {
                if let Some(pending) = &context.pending_id {
                    self.confirm_pending(pending)?;
                }
            }
        }
        if stays {
            self.rt.lock().cards.mark_used(id, action);
        } else {
            self.rt.lock().cards.forget(id);
            self.deps.presenter.dismiss(id);
            self.deps
                .sink
                .emit(NofapEvent::CardClosed { id: id.to_owned() });
        }
        Ok(())
    }

    /// "Fechar a janela": the only path to closing something on a vision answer. A
    /// browser tab is closed only while it still shows the host the card is about;
    /// another application is asked to quit.
    fn close_window(&self, context: &CardContext) {
        let Some(app_id) = &context.app_id else {
            return;
        };
        if !context.browser {
            let _ = self.deps.enforcer.quit_app(app_id);
            return;
        }
        let Ok(Some(window)) = self.deps.activity.peek_foreground() else {
            return;
        };
        let fresh = self
            .deps
            .urls
            .resolve_fresh(&window)
            .ok()
            .flatten()
            .as_deref()
            .and_then(normalize_host);
        let same = window.app_id == *app_id
            && (context.window_id.is_none() || window.window_id == context.window_id)
            && fresh.is_some()
            && fresh == context.host;
        if same && !self.deps.enforcer.close_active_tab(app_id).unwrap_or(false) {
            let _ = self.deps.enforcer.blank_active_tab(app_id);
        }
    }

    /// "Não era isso" on a vision card: a correction, not an off switch. The host that
    /// verdict learned is unlearned and one fresh, independent look follows twenty
    /// seconds later, whatever the caps say. When that look cannot come (the check was
    /// switched off, its consent or key went, the provider rejected the key), unlearning
    /// would be a pass with nothing watching: the button is a delayed request instead, as
    /// on a block card.
    fn not_it_on_vision(&self, context: &CardContext) -> CoreResult<()> {
        let now = self.now();
        let (day, _) = self.day_hour(now);
        let Some(key) = context.key else {
            return Ok(());
        };
        // Without a host nothing was learned, so nothing is unlearned either.
        let can_look =
            context.host.is_none() || self.vision_allowed_in(&self.deps.env.snapshot()).is_ok();
        if !can_look || self.rt.lock().sampler.exception_only(key, &day) {
            // The second look agreed with the first, or no look can come: today this is
            // a delayed request.
            return self.request_exception(context);
        }
        let mut outbox = Outbox::default();
        {
            let mut rt = self.rt.lock();
            if let Some(learned) = &context.learned_id {
                self.deps.repo.delete_site(learned)?;
                self.reload_sites(&mut rt);
                outbox.events.push(NofapEvent::SitesChanged);
            }
            sampling::record(&mut rt.sampler, key, SampleEvent::NotIt, &day, now);
            rt.recheck.insert(key);
        }
        self.bump(DayKind::NotIt);
        self.flush(outbox);
        Ok(())
    }

    /// "Não era isso" on a block card: a 24-hour snooze of the keyword and TLD rules for
    /// that host while the AI check is there to keep looking, and a delayed request in
    /// every other case.
    fn not_it_on_block(&self, context: &CardContext) -> CoreResult<()> {
        let now = self.now();
        let (Some(source), Some(host)) = (context.source, &context.host) else {
            return Ok(());
        };
        let effect = self.not_it_effect(source);
        self.bump(DayKind::NotIt);
        match effect {
            NotItEffect::Request => self.request_exception(context),
            NotItEffect::Snooze => {
                let mut rt = self.rt.lock();
                self.deps.repo.upsert_site(&NofapSite {
                    id: ubiqx_core::new_id(),
                    list: SiteList::Allow,
                    origin: SiteOrigin::Snooze,
                    host: None,
                    host_key: site_key(&salt_of(&rt.config), host),
                    created_at: now,
                    expires_at: Some(now + Duration::seconds(SNOOZE_SECS)),
                })?;
                self.reload_sites(&mut rt);
                Ok(())
            }
        }
    }

    /// "Pedir exceção": a request to allow the host, which waits the delay like any other
    /// loosening.
    fn request_exception(&self, context: &CardContext) -> CoreResult<()> {
        if let Some(host) = &context.host {
            self.request(NofapChange::AddAllow { host: host.clone() })?;
        }
        Ok(())
    }

    /// A preview of the blocked card, as the user set it up. Counts nothing.
    pub fn test_card(&self) {
        let now = self.now();
        let card = {
            let mut rt = self.rt.lock();
            let card = cards::blocked(false, rt.config.ui.discreet, false, rt.rotation, None, now);
            rt.cards.keep(card.clone(), CardContext::default(), now);
            card
        };
        self.present(&card, None);
    }

    /// "Ver um exemplo do que é enviado": one frame of the last window that was a
    /// candidate for the AI check, taken with the options a check uses. `None` when no
    /// such window is remembered or it is gone. The picture is shown in the page; it is
    /// not sent and not stored.
    pub fn sample_frame(&self) -> CoreResult<Option<SampleFrame>> {
        let Some((window_id, kind)) = self.rt.lock().last_candidate else {
            return Ok(None);
        };
        let frame = self
            .deps
            .frames
            .frame(window_id, sampling::frame_opts(kind))?;
        Ok(frame.map(|frame| SampleFrame {
            base64: base64::engine::general_purpose::STANDARD.encode(&frame.image.bytes),
            mime: frame.image.mime,
            width: frame.image.width,
            height: frame.image.height,
        }))
    }

    // -----------------------------------------------------------------------------------------
    // The user's data
    // -----------------------------------------------------------------------------------------

    /// "Apagar meu histórico": the counters, the snoozes, and what the person wrote (the
    /// values sentence and the plans). Protection settings and the lists stay. Never
    /// delayed: erasing one's own data is a right and weakens no tier.
    pub fn erase_history(&self) -> CoreResult<()> {
        {
            let mut rt = self.rt.lock();
            if !rt.config_error {
                // The cleared document goes in first, so the repository's erase also
                // takes the old one out of the database's log.
                self.update_config(&mut rt, |config| {
                    config.values.clear();
                    config.plans.clear();
                    config.internal.last_help_at = None;
                    Ok(())
                })?;
            }
            self.deps.repo.erase_history()?;
            self.reload_sites(&mut rt);
        }
        self.clear_memory();
        self.refresh_status();
        Ok(())
    }

    /// "Apagar tudo": the configuration, the lists and the history. Only while the module
    /// is off, because wiping the configuration of a running module would be an off
    /// switch without a delay. It is also the way out of a stored configuration that
    /// cannot be read.
    ///
    /// Tiers that are still on the machine (a removal that was declined, a configuration
    /// that could not be read) are not forgotten with the rest: the machine is read again
    /// right after, and the configuration keeps the moment the module was first enabled,
    /// so the page keeps showing them and offering "Remover do sistema", also after a
    /// restart.
    pub fn reset(&self) -> CoreResult<()> {
        let now = self.now();
        {
            let rt = self.rt.lock();
            if rt.config.enabled && !rt.config_error {
                return Err(CoreError::Invalid("only while the module is off".into()));
            }
        }
        let left = self.listed_now();
        {
            let mut rt = self.rt.lock();
            if rt.config.enabled && !rt.config_error {
                return Err(CoreError::Invalid("only while the module is off".into()));
            }
            let mut fresh = NofapConfig::default();
            if left {
                fresh.enabled_at = rt.config.enabled_at.or(Some(now));
            }
            self.replace_config(&mut rt, fresh)?;
            // What was known about the machine was judged against the old choices.
            rt.system = SystemMemory::default();
            for origin in SiteOrigin::ALL {
                self.deps.repo.delete_sites_by_origin(origin)?;
            }
            self.deps.repo.erase_history()?;
            self.reload_sites(&mut rt);
        }
        self.clear_memory();
        if left {
            if let Err(error) = self.read_system(true, false) {
                tracing::debug!(
                    class = error_class(&error),
                    "nofap: the system was not read"
                );
            }
        }
        self.flush(Outbox {
            events: vec![NofapEvent::SitesChanged],
            status: true,
            ..Outbox::default()
        });
        Ok(())
    }

    /// "Exportar dados": the configuration without its bookkeeping, the sites the user
    /// typed, the number of learned sites and the counters, as one JSON file named after
    /// the machine id. No learned name, no salt, no visited site.
    pub fn export(&self) -> CoreResult<ExportResult> {
        let now = self.now();
        if self.rt.lock().config_error {
            // The defaults in memory are not the user's configuration: no file says so.
            return Err(unreadable_config());
        }
        let counts = self.deps.repo.days_between(FIRST_DAY, LAST_DAY)?;
        let sites = self.sites();
        let config = ConfigView::from(&self.rt.lock().config);
        let document = serde_json::json!({
            "exported_at": now,
            "product": self.deps.identity.id,
            "config": config,
            "sites": sites,
            "days": counts,
        });
        let dir = &self.deps.export_dir;
        std::fs::create_dir_all(dir).map_err(|error| CoreError::Platform(error.to_string()))?;
        let path = dir.join(format!("shield-{}.json", now.format("%Y%m%d-%H%M%S")));
        let bytes = serde_json::to_vec_pretty(&document)
            .map_err(|error| CoreError::Other(error.to_string()))?;
        std::fs::write(&path, bytes).map_err(|error| CoreError::Platform(error.to_string()))?;
        Ok(ExportResult {
            path: path.to_string_lossy().into_owned(),
        })
    }

    /// The shell is quitting. Quitting is never prevented, but while the module is on it
    /// is the cheapest way around it, so it is counted (towards the help heuristic); the
    /// primary lock goes, so the other product can take over.
    pub fn on_quit(&self) {
        if self.enabled() {
            self.bump(DayKind::QuitOn);
        }
        self.release_primary();
    }

    // -----------------------------------------------------------------------------------------
    // What ubiqX's tracker asks
    // -----------------------------------------------------------------------------------------

    /// Whether the tracker must record this sample as private: its host is blocked, or
    /// its window had a hit in the last ten minutes. A hit leaves nothing in the
    /// productivity data.
    ///
    /// The lists are asked also while the module is off or the other product runs the
    /// passes: they are in memory and touch no port, and a page the standalone product
    /// blocks would otherwise be kept with its title and address while ubiqX's own module
    /// is off. While it is off only a listed host counts (the embedded list and the
    /// person's own blocks), not the keyword and TLD rules, which can be wrong about a
    /// page. What only the other product knows, its own sites and its AI's answers, is not
    /// shared and stays out of reach; PRIVACY.md says so.
    pub fn redacts(&self, sample: &ActivitySample) -> bool {
        let now = self.now();
        let rt = self.rt.lock();
        // The same reading of the address as the guard's, so both name the window alike.
        let judged = match (&rt.matcher, sample.url.as_deref()) {
            (Some(matcher), Some(url)) => {
                let verdict = matcher.verdict_for_url(url).0;
                Some((verdict, page_of(matcher, url, verdict).1))
            }
            _ => None,
        };
        let blocked = judged
            .as_ref()
            .is_some_and(|(verdict, _)| verdict.is_blocked());
        if !rt.config.enabled {
            return judged
                .as_ref()
                .is_some_and(|(verdict, _)| listed_block(*verdict));
        }
        if blocked {
            return true;
        }
        let page = judged.as_ref().map(|(_, page)| page.as_str());
        let key = context_key(page, &sample.app_id, &sample.window_title);
        let window_key = context_key(None, &sample.app_id, &sample.window_title);
        recent(rt.hits.get(&key), now) || recent(rt.hits.get(&window_key), now)
    }

    /// Whether the block the tracker has open is the one a hit was just about: the same
    /// application and, when both the hit and the block name a host, the same host (or a
    /// blocked one). The tracker scrubs its open block after a hit only when this says
    /// so: the guard acts within seconds of a page coming to the front, often before the
    /// tracker has sampled it, and the block still open then is the work the user did
    /// before, which must not be turned into a private block.
    pub fn covers_open_block(&self, app_id: &str, url: Option<&str>) -> bool {
        self.covers(app_id, url, None)
    }

    /// Whether a block the tracker is closing now was open when a hit of the last ten
    /// minutes came, and is that hit's (the test of [`NofapService::covers_open_block`]
    /// for a hit that came after `started_at`). An answer of the AI check is acted on and
    /// the tracker's open block scrubbed a moment later, on another task: a sample that
    /// arrives in between (the tab that comes to the front once the page is closed)
    /// closes the page's block the ordinary way, and the scrub then finds another block
    /// open. The tracker asks this before it keeps a block it closes, and makes it
    /// private instead.
    pub fn covers_closed_block(
        &self,
        app_id: &str,
        url: Option<&str>,
        started_at: DateTime<Utc>,
    ) -> bool {
        self.covers(app_id, url, Some(started_at))
    }

    fn covers(&self, app_id: &str, url: Option<&str>, since: Option<DateTime<Utc>>) -> bool {
        let now = self.now();
        let rt = self.rt.lock();
        if !rt.config.enabled {
            return false;
        }
        let blocked = match (&rt.matcher, url) {
            (Some(matcher), Some(url)) => matcher.verdict_for_url(url).0.is_blocked(),
            _ => false,
        };
        // An open block is scrubbed when its page is blocked; a closed one only when a
        // hit came while it was open.
        if blocked && since.is_none() {
            return true;
        }
        let host = url.and_then(normalize_host);
        rt.hit_marks.iter().any(|mark| {
            recent(Some(&mark.at), now)
                && since.is_none_or(|since| mark.at > since)
                && mark.app_id.eq_ignore_ascii_case(app_id)
                // Without a host on either side, the application is all there is to go
                // by, and the block goes.
                && (blocked
                    || match (&mark.host, &host) {
                        (Some(hit), Some(open)) => hit == open,
                        _ => true,
                    })
        })
    }

    /// Whether the tracker must not take a productivity screenshot of this window: its
    /// host is blocked or a mixed platform, or it carried a signal or had a hit in the
    /// last ten minutes. While the module is off only a blocked host is vetoed (see
    /// [`NofapService::redacts`]).
    pub fn capture_veto(&self, app_id: &str, domain: Option<&str>, title: &str) -> bool {
        let now = self.now();
        let rt = self.rt.lock();
        let host = domain.and_then(normalize_host);
        if !rt.config.enabled {
            return match (&rt.matcher, host.as_deref()) {
                (Some(matcher), Some(host)) => listed_block(matcher.verdict(host)),
                _ => false,
            };
        }
        let by_host = match (&rt.matcher, host.as_deref()) {
            (Some(matcher), Some(host)) => matches!(
                matcher.verdict(host),
                Verdict::Blocked(_)
                    | Verdict::Unknown(HostClass::Mixed | HostClass::Plain { signal: true })
            ),
            _ => false,
        };
        let keys = [
            context_key(host.as_deref(), app_id, title),
            context_key(None, app_id, title),
        ];
        by_host
            || heuristics::title_signal(title)
            || keys
                .iter()
                .any(|key| recent(rt.hits.get(key), now) || recent(rt.signals.get(key), now))
    }

    /// The language of the host, for the tray labels (`ubiqx_nofap::messages`).
    pub fn language(&self) -> ubiqx_core::UiLanguage {
        self.deps.env.snapshot().language
    }

    /// The quit dialog: the sentence that is true for what is applied right now, and its
    /// two buttons ("Manter aberto" is the primary one).
    pub fn quit_dialog(&self) -> messages::QuitDialog {
        messages::quit_dialog(self.system_applied(), self.language())
    }
}

/// A host on a list: the embedded one, one the person typed or one the AI check learned
/// (not a keyword or TLD rule).
fn listed_block(verdict: Verdict) -> bool {
    matches!(
        verdict,
        Verdict::Blocked(BlockSource::List | BlockSource::User | BlockSource::Learned)
    )
}

/// Today's check-in answer: the mood counted today, the latest hour first.
fn checkin_of(counts: &[DayCount], today: &str) -> Option<Mood> {
    counts
        .iter()
        .filter(|count| count.day == today && count.n > 0)
        .filter_map(|count| {
            let mood = Mood::ALL
                .into_iter()
                .find(|mood| mood_kind(*mood) == count.kind)?;
            Some((count.hour, mood))
        })
        .max_by_key(|(hour, _)| *hour)
        .map(|(_, mood)| mood)
}
