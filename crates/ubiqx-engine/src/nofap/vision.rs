//! The AI check (L5): whether it may run, one check, and what an answer does.
//!
//! A check sends one reduced image of one window and two constant strings; nothing else
//! leaves the machine, and nothing about the check is kept but a counter of the hour and
//! the cost row the AI client writes. Readiness is read-only and independent of the
//! productivity AI in both directions: it never reads ubiqX's AI health and never pauses
//! it, and it counts only its own spend.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use ubiqx_core::nofap::{
    AiReadiness, CardAction, CardKind, DayKind, ExplicitCheck, NofapEnvSnapshot, NofapEvent,
    NofapSite, SiteList, SiteOrigin, VisionOff, VisionPrefs, VisionState,
};
use ubiqx_core::ports::{AiUsageKind, PermissionState};
use ubiqx_core::scheduler::month_range;
use ubiqx_core::{CoreError, CoreResult};
use ubiqx_nofap::list::{self, HashTable};
use ubiqx_nofap::matcher::{normalize_host, parents, site_key};
use ubiqx_nofap::messages::Notice;
use ubiqx_nofap::sampling::{self, CheckError, Outcome, SampleEvent, Stage, WindowKind};

use super::cards::{self, CardContext, Outbox, ASK_CLOSE_SECS};
use super::{error_class, forget_old, mark_hit, salt_of, DisplaySession, InFlight, NofapService};

/// A key the provider rejected three times switches the check off; it is tried again
/// after this long.
pub const KEY_RETRY_SECS: i64 = 900;
/// A check that was found not ready is asked about again after this long, not in every
/// pass (readiness reads the ledger and the permission).
pub const NOT_READY_FOR_SECS: i64 = 10;

/// One check the guard asked for. It holds the "one check at a time" mark: dropping the
/// job, run or not, lets the next check start.
pub struct VisionJob {
    pub(crate) key: u64,
    /// The key the tracker knows the window by when it has no host.
    pub(crate) window_key: u64,
    pub(crate) kind: WindowKind,
    pub(crate) window_id: u32,
    pub(crate) app_id: String,
    pub(crate) host: Option<String>,
    /// The host is of class `Plain`: a positive answer may learn it.
    pub(crate) learnable: bool,
    /// The window itself carries a signal (for the two-refusals rule).
    pub(crate) local_signal: bool,
    /// This is the independent look "Não era isso" bought.
    pub(crate) recheck: bool,
    pub(crate) model: String,
    /// Held, never read: dropping it is what lets the next check start.
    pub(crate) _in_flight: InFlight,
}

/// What one check did.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct VisionOutcome {
    /// The answer was positive and was acted on. ubiqX's tracker then forgets the block
    /// it had open.
    pub hit: bool,
}

impl NofapService {
    /// Whether the AI check may run now, and why not (design 4.1, in its order). Reads
    /// the host's settings, the permission and the module's own spend; changes nothing.
    pub fn vision_ready(&self) -> Result<(), VisionOff> {
        self.vision_ready_in(&self.deps.env.snapshot())
    }

    pub(crate) fn vision_ready_in(&self, env: &NofapEnvSnapshot) -> Result<(), VisionOff> {
        let now = self.now();
        let prefs = self.vision_allowed_in(env)?;
        if self.month_spend(now).0 >= prefs.monthly_cap_usd {
            return Err(VisionOff::BudgetSpent);
        }
        let (day, _) = self.day_hour(now);
        if self.rt.lock().sampler.cap_reached(&prefs, &day, now) {
            return Err(VisionOff::CapReached);
        }
        Ok(())
    }

    /// The part of readiness the user can withdraw at any moment: the module and the
    /// check switched on, the consent for the provider now selected, a usable key, not
    /// "local only", the permission. Asked again right before every frame, so that
    /// nothing is captured or sent after one of them went (a check waits up to four
    /// seconds for its second frame). The budget and the caps are not part of it: they
    /// are the guard's question, and the second frame of a check belongs to the first.
    pub(crate) fn vision_allowed_in(
        &self,
        env: &NofapEnvSnapshot,
    ) -> Result<VisionPrefs, VisionOff> {
        let (prefs, key_rejected) = {
            let rt = self.rt.lock();
            if !rt.config.enabled || rt.config_error {
                return Err(VisionOff::Disabled);
            }
            (rt.config.vision.clone(), rt.sampler.key_rejected())
        };
        if !prefs.enabled {
            return Err(VisionOff::Disabled);
        }
        // Consent names a provider: another one selected since, or the managed route
        // without its own tick, pauses the check until the user agrees again.
        let agreed = prefs.consent.as_ref().is_some_and(|consent| {
            consent.provider == env.provider && (consent.managed || !env.provider.is_managed())
        });
        if !agreed {
            return Err(VisionOff::ConsentOutdated);
        }
        if self.deps.detector.is_none() {
            return Err(VisionOff::NoProvider);
        }
        match env.ai {
            AiReadiness::Ready => {}
            AiReadiness::NoKey => return Err(VisionOff::NoKey),
            AiReadiness::LocalOnly => return Err(VisionOff::LocalOnly),
            AiReadiness::NotOnboarded => return Err(VisionOff::NotOnboarded),
            AiReadiness::LicenceRequired => return Err(VisionOff::Licence),
        }
        if key_rejected {
            return Err(VisionOff::NoKey);
        }
        if self.deps.tuning.display == DisplaySession::Wayland {
            return Err(VisionOff::Wayland);
        }
        if self.deps.permissions.status().screen_recording == PermissionState::Denied {
            return Err(VisionOff::ScreenRecording);
        }
        Ok(prefs)
    }

    /// The state of the layer for the status: off with its reason, in backoff, or on.
    pub(crate) fn vision_state(&self, env: &NofapEnvSnapshot) -> VisionState {
        match self.vision_ready_in(env) {
            Err(reason) => VisionState::Off { reason },
            Ok(()) => match self.rt.lock().sampler.degraded_until(self.now()) {
                Some(until) => VisionState::Degraded { until },
                None => VisionState::On,
            },
        }
    }

    /// The module's own spend this month and the number of checks behind it: the ledger
    /// rows of kind `nofap_vision`, nothing else.
    pub(crate) fn month_spend(&self, now: DateTime<Utc>) -> (f64, u64) {
        self.deps
            .usage
            .totals_by_kind(month_range(now))
            .unwrap_or_default()
            .into_iter()
            .find(|(kind, _)| *kind == AiUsageKind::NofapVision)
            .map_or((0.0, 0), |(_, totals)| (totals.cost_usd, totals.calls))
    }

    /// Another provider, or another answer to "is it usable", since the sampler last ran:
    /// what it remembers (rejected keys, backoff) belonged to the old one.
    pub(crate) fn note_provider(&self, env: &NofapEnvSnapshot) {
        let now = self.now();
        let seen = (env.provider, env.ai);
        let changed = {
            let mut rt = self.rt.lock();
            let mut changed = rt.provider_seen.is_some_and(|before| before != seen);
            rt.provider_seen = Some(seen);
            // A key the provider rejected is tried again after a while: the user may
            // have put it right at the provider's (credit, a re-enabled key), and nothing
            // here would learn of it otherwise.
            if rt.sampler.key_rejected() {
                match rt.rejected_at {
                    None => rt.rejected_at = Some(now),
                    Some(at) => {
                        changed |= !(0..KEY_RETRY_SECS).contains(&(now - at).num_seconds());
                    }
                }
            }
            if changed {
                rt.sampler.clear();
                rt.recheck.clear();
                rt.rejected_at = None;
                rt.not_ready_at = None;
            }
            changed
        };
        if changed {
            self.seed_day_count();
            self.refresh_status();
        }
    }

    /// The host changed its AI settings (a key was stored or removed, another provider
    /// selected): what the sampler remembers about the old one is forgotten at once,
    /// instead of when the guard next notices.
    pub fn ai_settings_changed(&self) {
        {
            let mut rt = self.rt.lock();
            rt.sampler.clear();
            rt.recheck.clear();
            rt.rejected_at = None;
            rt.not_ready_at = None;
            rt.provider_seen = None;
        }
        self.seed_day_count();
        if self.enabled() {
            self.refresh_status();
        }
    }

    /// Runs one check: a frame, the detector, and what the answer asks for. The guard
    /// spawns this and keeps ticking; nothing here holds the runtime lock across a call.
    pub async fn vision_check(&self, job: VisionJob) -> CoreResult<VisionOutcome> {
        let Some(me) = self.me.upgrade() else {
            return Ok(VisionOutcome::default());
        };
        let Some((mut outcome, refused)) = me.look(&job, Stage::First).await else {
            return Ok(VisionOutcome::default());
        };
        if outcome == Outcome::Confirm {
            // `explicit` at medium confidence: one more frame a few seconds later decides.
            tokio::time::sleep(self.deps.tuning.confirm_after).await;
            match me.look(&job, Stage::Second).await {
                Some((second, _)) => outcome = second,
                None => return Ok(VisionOutcome::default()),
            }
        }
        if !self.enabled() {
            return Ok(VisionOutcome::default());
        }
        let due = match outcome {
            Outcome::Unsure => true,
            Outcome::Watch => {
                refused
                    && sampling::refusal_card_due(
                        &self.rt.lock().sampler,
                        job.key,
                        job.local_signal,
                    )
            }
            Outcome::Positive | Outcome::Clean | Outcome::Confirm => false,
        };
        // What an answer does reads the foreground, writes counters and shows a card: on
        // the blocking pool, never on the async threads.
        let hit = match outcome {
            Outcome::Positive => {
                let me = me.clone();
                tokio::task::spawn_blocking(move || me.act_on_positive(&job))
                    .await
                    .unwrap_or(false)
            }
            _ if due => {
                let me = me.clone();
                let _ = tokio::task::spawn_blocking(move || me.ask(&job)).await;
                false
            }
            Outcome::Unsure | Outcome::Watch | Outcome::Clean | Outcome::Confirm => false,
        };
        Ok(VisionOutcome { hit })
    }

    /// One frame and one answer. `None` when nothing was sent (no frame, the same picture
    /// as before) or the call failed; the sampler is told either way.
    async fn look(self: &Arc<Self>, job: &VisionJob, stage: Stage) -> Option<(Outcome, bool)> {
        let detector = self.deps.detector.clone()?;
        let (window_id, opts) = (job.window_id, sampling::frame_opts(job.kind));
        let frames = self.deps.frames.clone();
        let me = self.clone();
        let key = job.key;
        let frame = tokio::task::spawn_blocking(move || {
            // Asked again here, off the async threads (it reads the host's settings and
            // the permission): what was withdrawn since the guard asked for this check,
            // or since its first frame, stops it before anything is captured.
            let taken = if me.vision_allowed_in(&me.deps.env.snapshot()).is_err() {
                Ok(None)
            } else {
                frames.frame(window_id, opts)
            };
            match taken {
                Ok(Some(frame)) => Some(frame),
                other => {
                    me.no_frame(key, matches!(other, Err(CoreError::Permission(_))));
                    None
                }
            }
        })
        .await;
        let Ok(Some(frame)) = frame else {
            return None;
        };
        let now = self.now();
        let (day, _) = self.day_hour(now);
        {
            let mut rt = self.rt.lock();
            let same = stage == Stage::First
                && !job.recheck
                && sampling::frame_unchanged(&rt.sampler, job.key, frame.dhash, now);
            if same {
                sampling::record(&mut rt.sampler, job.key, SampleEvent::Unchanged, &day, now);
                return None;
            }
        }

        // The client retries on its own (four attempts, backing off), so the whole call
        // gets one bound here.
        let answer = tokio::time::timeout(
            self.deps.tuning.check_timeout,
            detector.check(&frame.image, &job.model),
        )
        .await;
        // The image is dropped here: no file, no row, no hash of it is kept.
        let dhash = frame.dhash;
        drop(frame);
        let me = self.clone();
        let (key, window_key) = (job.key, job.window_key);
        tokio::task::spawn_blocking(move || me.take_answer(key, window_key, stage, answer, dhash))
            .await
            .ok()
            .flatten()
    }

    /// No usable frame (the window is gone, the capture was refused or withdrawn): nothing
    /// is sent, and the interval starts again so the next pass does not retry.
    fn no_frame(&self, key: u64, denied: bool) {
        let now = self.now();
        let (day, _) = self.day_hour(now);
        sampling::record(
            &mut self.rt.lock().sampler,
            key,
            SampleEvent::Unchanged,
            &day,
            now,
        );
        if denied {
            self.refresh_status();
        }
    }

    /// What the detector answered, told to the sampler: the outcome and whether the model
    /// refused, or `None` when the call failed. Runs on the blocking pool (a counter is
    /// written, the status may be computed again).
    fn take_answer(
        &self,
        key: u64,
        window_key: u64,
        stage: Stage,
        answer: Result<CoreResult<ExplicitCheck>, tokio::time::error::Elapsed>,
        dhash: u64,
    ) -> Option<(Outcome, bool)> {
        let now = self.now();
        let (day, _) = self.day_hour(now);
        match answer {
            Ok(Ok(check)) => {
                let outcome = sampling::outcome(&check, stage);
                self.bump(DayKind::VisionCheck);
                let was_degraded = {
                    let mut rt = self.rt.lock();
                    let was = rt.sampler.degraded_until(now).is_some();
                    let event = SampleEvent::Checked {
                        outcome,
                        refused: check.refused,
                        dhash,
                    };
                    sampling::record(&mut rt.sampler, key, event, &day, now);
                    if outcome != Outcome::Clean {
                        // An answer other than `none` is a signal for that window: the
                        // tracker takes no productivity screenshot of it for ten minutes.
                        rt.signals.insert(key, now);
                        rt.signals.insert(window_key, now);
                        forget_old(&mut rt.signals, now);
                    }
                    was
                };
                if was_degraded {
                    tracing::info!("nofap: the AI check answers again");
                    self.refresh_status();
                }
                Some((outcome, check.refused))
            }
            failed => {
                let error = match failed {
                    Ok(Err(CoreError::AiRejected(_) | CoreError::AiNotConfigured)) => {
                        CheckError::KeyRejected
                    }
                    // A timeout, the network, an outage, a rate limit.
                    _ => CheckError::Transient,
                };
                let news = {
                    let mut rt = self.rt.lock();
                    let before = (
                        rt.sampler.degraded_until(now).is_some(),
                        rt.sampler.key_rejected(),
                    );
                    sampling::record(&mut rt.sampler, key, SampleEvent::Failed(error), &day, now);
                    let after = (
                        rt.sampler.degraded_until(now).is_some(),
                        rt.sampler.key_rejected(),
                    );
                    before != after
                };
                if news {
                    // A change of state, not a check: the one thing this layer logs.
                    tracing::info!("nofap: the AI check is paused after repeated failures");
                    self.refresh_status();
                }
                None
            }
        }
    }

    /// A positive answer: read the foreground again and, in a browser, the address; act
    /// only when the window and the host are those that were captured. Otherwise the
    /// verdict is dropped: nothing is closed and nothing is learned.
    fn act_on_positive(&self, job: &VisionJob) -> bool {
        let now = self.now();
        let (day, _) = self.day_hour(now);
        let Ok(Some(window)) = self.deps.activity.peek_foreground() else {
            return false;
        };
        if window.window_id != Some(job.window_id) || window.app_id != job.app_id {
            return false;
        }
        let tab_host = match (job.kind, job.host.as_deref()) {
            (WindowKind::Browser, Some(host)) => {
                let fresh = self
                    .deps
                    .urls
                    .resolve_fresh(&window)
                    .ok()
                    .flatten()
                    .as_deref()
                    .and_then(normalize_host);
                if fresh.as_deref() != Some(host) {
                    return false;
                }
                Some(host)
            }
            _ => None,
        };

        let mut outbox = Outbox::default();
        let mut context = CardContext {
            key: Some(job.key),
            app_id: Some(job.app_id.clone()),
            host: job.host.clone(),
            window_id: Some(job.window_id),
            browser: tab_host.is_some(),
            ..CardContext::default()
        };
        let exception_only = {
            let mut rt = self.rt.lock();
            if job.recheck {
                // The second, independent look agreed: for the rest of the day the button
                // on this window asks for a delayed exception.
                sampling::record(
                    &mut rt.sampler,
                    job.key,
                    SampleEvent::NotItConfirmed,
                    &day,
                    now,
                );
            }
            rt.hits.insert(job.key, now);
            rt.hits.insert(job.window_key, now);
            forget_old(&mut rt.hits, now);
            mark_hit(&mut rt, &job.app_id, tab_host, now);
            rt.sampler.exception_only(job.key, &day)
        };
        let correction = match (exception_only, job.host.is_some()) {
            (false, _) => Some(CardAction::NotIt),
            (true, true) => Some(CardAction::RequestException),
            // Without a host there is nothing an exception could name.
            (true, false) => None,
        };

        let card = match tab_host {
            Some(host) => {
                let enforcer = &self.deps.enforcer;
                if !enforcer.close_active_tab(&job.app_id).unwrap_or(false) {
                    let _ = enforcer.blank_active_tab(&job.app_id);
                }
                let learned = if job.learnable && !self.is_popular(host) {
                    self.learn(host, now)
                } else {
                    None
                };
                let mut actions = vec![CardAction::Ok];
                actions.extend(correction);
                actions.extend([CardAction::Surf, CardAction::AddToList]);
                let mut card = cards::card(
                    CardKind::VisionClosed,
                    "vision.closed",
                    false,
                    actions,
                    None,
                    now,
                );
                if learned.is_some() {
                    card.note_key = Some("vision.added".to_owned());
                    outbox.events.push(NofapEvent::SitesChanged);
                }
                context.learned_id = learned;
                (card, Some(Notice::Blocked))
            }
            None => {
                // Another application: its windows are hidden, which can be undone. It is
                // closed only when the user presses the button.
                let _ = self.deps.enforcer.hide_others(self.deps.identity.bundle_id);
                let mut actions = vec![CardAction::CloseWindow];
                actions.extend(correction);
                let card = cards::card(
                    CardKind::VisionAsk,
                    "vision.soft",
                    false,
                    actions,
                    Some(ASK_CLOSE_SECS),
                    now,
                );
                (card, Some(Notice::Drift))
            }
        };
        self.bump(DayKind::VisionHit);
        {
            let mut rt = self.rt.lock();
            if rt.cards.may_show(now) {
                rt.cards.keep(card.0.clone(), context, now);
                outbox.cards.push(card);
            }
        }
        self.flush(outbox);
        true
    }

    /// The soft card ("Posso estar enganado…"), at most once per window in thirty
    /// minutes, and only while that window is still in front: an answer comes up to
    /// fourteen seconds after its frame, and a card about a window the user has left
    /// would be about nothing on screen (and its "Fechar a janela" would quit an
    /// application in the background). Nothing is done to the window.
    fn ask(&self, job: &VisionJob) {
        let still_front = matches!(
            self.deps.activity.peek_foreground(),
            Ok(Some(window)) if window.window_id == Some(job.window_id) && window.app_id == job.app_id
        );
        if !still_front || !self.enabled() {
            return;
        }
        let now = self.now();
        let (day, _) = self.day_hour(now);
        let mut outbox = Outbox::default();
        {
            let mut rt = self.rt.lock();
            if !rt.sampler.ask_card_allowed(job.key, now) || !rt.cards.may_show(now) {
                return;
            }
            sampling::record(
                &mut rt.sampler,
                job.key,
                SampleEvent::AskCardShown,
                &day,
                now,
            );
            let correction = if rt.sampler.exception_only(job.key, &day) {
                job.host.is_some().then_some(CardAction::RequestException)
            } else {
                Some(CardAction::NotIt)
            };
            let mut actions = vec![CardAction::CloseWindow];
            actions.extend(correction);
            let card = cards::card(
                CardKind::VisionAsk,
                "vision.soft",
                false,
                actions,
                Some(ASK_CLOSE_SECS),
                now,
            );
            let context = CardContext {
                key: Some(job.key),
                app_id: Some(job.app_id.clone()),
                host: job.host.clone(),
                window_id: Some(job.window_id),
                browser: job.kind == WindowKind::Browser && job.host.is_some(),
                ..CardContext::default()
            };
            rt.cards.keep(card.clone(), context, now);
            outbox.cards.push((card, None));
        }
        self.flush(outbox);
    }

    /// Stores a learned site: its salted hash, never its name. Returns the row's id.
    fn learn(&self, host: &str, now: DateTime<Utc>) -> Option<String> {
        let mut rt = self.rt.lock();
        let site = NofapSite {
            id: ubiqx_core::new_id(),
            list: SiteList::Block,
            origin: SiteOrigin::Learned,
            host: None,
            host_key: site_key(&salt_of(&rt.config), host),
            created_at: now,
            expires_at: None,
        };
        match self.deps.repo.upsert_site(&site) {
            Ok(()) => {
                self.reload_sites(&mut rt);
                Some(site.id)
            }
            Err(error) => {
                tracing::debug!(class = error_class(&error), "nofap: a site was not stored");
                None
            }
        }
    }

    /// Whether the host, or a parent of it, is a well-known site. Such a host is never
    /// learned, also when a signal word in its name made the matcher call it plain.
    fn is_popular(&self, host: &str) -> bool {
        HashTable::parse(list::embedded_popular(), list::POPULAR_TAG)
            .is_ok_and(|popular| parents(host).iter().any(|name| popular.contains(name)))
    }
}
