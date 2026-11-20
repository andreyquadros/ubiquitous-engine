//! The guard (L1): one pass every two seconds over the window in front.
//!
//! A blocked host in a browser tab is acted on in every pass it stays in front (there is
//! no enforcement cooldown), but only after two passes agreed on it and a fresh read of
//! the foreground and of the address, taken right before the action, still shows the same
//! application, window and host. Anything else goes to the sampler, which may ask for one
//! AI check; the check is handed back as a job and never awaited here.

use chrono::{DateTime, Utc};
use ubiqx_core::nofap::{CardAction, DayKind, GuardLimit, VisionPrefs};
use ubiqx_core::normalize::is_private_browsing_title;
use ubiqx_core::{CoreError, CoreResult, ForegroundWindow};
use ubiqx_nofap::episode::EpisodeCard;
use ubiqx_nofap::exclusions::{self, ExclusionLists};
use ubiqx_nofap::heuristics;
use ubiqx_nofap::matcher::{normalize_host, BlockSource, HostClass, Matcher, Verdict};
use ubiqx_nofap::messages::Notice;
use ubiqx_nofap::plan::bypass_suspected;
use ubiqx_nofap::primary::Role;
use ubiqx_nofap::sampling::{
    self, Decision, NotItEffect, SampleCtx, SampleEvent, Skip, WindowKind, HEIGHTENED_SECS,
};

use super::cards::{self, CardContext, Outbox};
use super::vision::{VisionJob, NOT_READY_FOR_SECS};
use super::{context_key, forget_old, DisplaySession, InFlight, NofapService};

/// What one pass did.
#[derive(Default)]
pub struct GuardOutcome {
    /// A blocked page was in front and was acted on. ubiqX's tracker then forgets the
    /// block it had open.
    pub hit: bool,
    /// One AI check to run on a task of its own.
    pub job: Option<VisionJob>,
}

/// The window in front as one pass read it.
struct Front {
    window: ForegroundWindow,
    /// The URL resolver knows this browser.
    supported: bool,
    has_url: bool,
    /// The address could not be read because the permission is missing.
    denied: bool,
    /// The host of the page, when the address is an ordinary one ([`page_of`]).
    host: Option<String>,
    /// What two reads of the address are compared by: the host, or the address itself.
    page: Option<String>,
    verdict: Option<Verdict>,
    path_signal: bool,
}

/// What a read address is about: the host of the page, and an identity to compare two
/// reads by. A reader view or a source view shows a page under the browser's own scheme
/// (`about:reader?url=…`, `view-source:…`): the matcher judges the page it shows, but the
/// text that stands where a host would is not one, so no card or request may name it.
/// The host is taken only when judging it alone gives the verdict of the whole address.
pub(crate) fn page_of(matcher: &Matcher, url: &str, verdict: Verdict) -> (Option<String>, String) {
    let host = normalize_host(url).filter(|host| matcher.verdict(host) == verdict);
    let page = host.clone().unwrap_or_else(|| url.trim().to_owned());
    (host, page)
}

impl Front {
    fn kind(&self) -> WindowKind {
        sampling::window_kind(
            &self.window.app_id,
            &self.window.app_name,
            self.supported,
            self.has_url,
        )
    }

    fn key(&self) -> u64 {
        context_key(
            self.page.as_deref(),
            &self.window.app_id,
            &self.window.window_title,
        )
    }
}

impl NofapService {
    /// One pass of the guard. Returns before any platform call while the module is off,
    /// while its configuration is unknown and while the other product runs the passes.
    pub fn guard_pass(&self) -> CoreResult<GuardOutcome> {
        let now = self.now();
        let matcher = {
            let rt = self.rt.lock();
            if !rt.config.enabled || rt.config_error {
                return Ok(GuardOutcome::default());
            }
            match rt.matcher.clone() {
                Some(matcher) => matcher,
                None => return Ok(GuardOutcome::default()),
            }
        };
        if self.role_at(now) == Role::Secondary {
            return Ok(GuardOutcome::default());
        }
        let matcher = self.matcher_at(matcher, now);

        let Some(window) = self.deps.activity.peek_foreground()? else {
            self.rt.lock().confirmer.clear();
            self.note_limit(None);
            return Ok(GuardOutcome::default());
        };
        // Both products, the desktop shell and its settings are never acted on.
        if crate::focus::is_exempt_window(&window.app_id, &window.app_name, None) {
            self.rt.lock().confirmer.clear();
            self.note_limit(None);
            return Ok(GuardOutcome::default());
        }
        let front = self.read_front(&matcher, window);
        match front.verdict {
            Some(Verdict::Blocked(source)) => self.enforce(&matcher, &front, source, now),
            _ => self.sample(&front, now),
        }
    }

    /// The matcher to judge with: rebuilt first when a snooze ended since it was built
    /// (a matcher reads the time only when it is built).
    fn matcher_at(
        &self,
        matcher: std::sync::Arc<Matcher>,
        now: DateTime<Utc>,
    ) -> std::sync::Arc<Matcher> {
        let mut rt = self.rt.lock();
        if rt.snooze_ends.is_some_and(|ends| ends <= now) {
            self.reload_sites(&mut rt);
            return rt.matcher.clone().unwrap_or(matcher);
        }
        matcher
    }

    /// Reads the address of a supported browser and judges it.
    fn read_front(&self, matcher: &Matcher, window: ForegroundWindow) -> Front {
        let supported = self.deps.urls.supports(&window.app_id);
        let mut denied = false;
        let url = if supported {
            match self.deps.urls.resolve(&window) {
                Ok(url) => url.filter(|url| !url.trim().is_empty()),
                Err(CoreError::Permission(_)) => {
                    denied = true;
                    None
                }
                Err(_) => None,
            }
        } else {
            None
        };
        let (verdict, path_signal) = match url.as_deref() {
            Some(url) => {
                let (verdict, path_signal) = matcher.verdict_for_url(url);
                (Some(verdict), path_signal)
            }
            None => (None, false),
        };
        let (host, page) = match (url.as_deref(), verdict) {
            (Some(url), Some(verdict)) => {
                let (host, page) = page_of(matcher, url, verdict);
                (host, Some(page))
            }
            _ => (None, None),
        };
        Front {
            host,
            page,
            has_url: url.is_some(),
            window,
            supported,
            denied,
            verdict,
            path_signal,
        }
    }

    /// Remembers what keeps the guard from reading the window in front; the status says
    /// so when it changes.
    fn note_limit(&self, limit: Option<GuardLimit>) {
        let limit = match self.deps.tuning.display {
            DisplaySession::Wayland => Some(GuardLimit::Wayland),
            DisplaySession::Full | DisplaySession::TitleOnly => limit,
        };
        let changed = {
            let mut rt = self.rt.lock();
            let changed = rt.guard_limit != limit;
            rt.guard_limit = limit;
            changed
        };
        if changed {
            self.refresh_status();
        }
    }

    fn limit_of(&self, front: &Front) -> Option<GuardLimit> {
        let browser = matches!(
            front.kind(),
            WindowKind::Browser | WindowKind::UnreadableBrowser
        );
        if front.denied {
            Some(GuardLimit::AutomationDenied)
        } else if front.kind() == WindowKind::UnreadableBrowser {
            Some(GuardLimit::BrowserWithoutUrl)
        } else if browser && self.deps.tuning.display == DisplaySession::TitleOnly {
            Some(GuardLimit::TitleOnly)
        } else {
            None
        }
    }

    // -----------------------------------------------------------------------------------------
    // Enforcement
    // -----------------------------------------------------------------------------------------

    /// A blocked host is in front: act once two passes agreed and a fresh read confirms.
    fn enforce(
        &self,
        matcher: &Matcher,
        front: &Front,
        source: BlockSource,
        now: DateTime<Utc>,
    ) -> CoreResult<GuardOutcome> {
        let Some(page) = front.page.as_deref() else {
            return Ok(GuardOutcome::default());
        };
        let host = front.host.as_deref();
        let window = &front.window;
        let key = front.key();
        if !self.rt.lock().confirmer.observe(&window.app_id, key) {
            return Ok(GuardOutcome::default());
        }
        self.note_limit(None);

        // Read again, immediately before acting: on Windows the action is a keystroke into
        // whatever is active, and the first read may have come from the resolver's cache.
        let fresh_page = self
            .deps
            .urls
            .resolve_fresh(window)
            .ok()
            .flatten()
            .map(|url| page_of(matcher, &url, matcher.verdict_for_url(&url).0).1);
        let still_front = self.deps.activity.peek_foreground()?.is_some_and(|again| {
            again.app_id == window.app_id
                && (window.window_id.is_none() || again.window_id == window.window_id)
        });
        if !still_front || fresh_page.as_deref() != Some(page) {
            self.rt.lock().confirmer.clear();
            return Ok(GuardOutcome::default());
        }

        let enforcer = &self.deps.enforcer;
        if !enforcer.close_active_tab(&window.app_id).unwrap_or(false) {
            // A tab that cannot be closed is blanked; when that fails too, the card is
            // all there is.
            let _ = enforcer.blank_active_tab(&window.app_id);
        }

        let (day, _) = self.day_hour(now);
        let in_hosts_block = host.is_some_and(|host| matcher.in_hosts_tier(host));
        let (step, which, discreet, rotation, bypass) = {
            let mut rt = self.rt.lock();
            let step = rt.episodes.observe(now);
            sampling::record(&mut rt.sampler, key, SampleEvent::Heightened, &day, now);
            rt.hits.insert(key, now);
            forget_old(&mut rt.hits, now);
            // A name of the hosts block that still loads means DNS is being bypassed.
            let hosts = rt.system.assessment.as_ref().map(|found| &found.hosts);
            let bypass = hosts.is_some_and(|hosts| bypass_suspected(in_hosts_block, hosts));
            let news = bypass && !bypass_recent(rt.bypass_at, now);
            if bypass {
                rt.bypass_at = Some(now);
            }
            if step.hit == 1 {
                rt.repeat_owed = false;
            }
            // The third hit of an episode asks for the "repeat" card. When it comes
            // inside the minute the first card holds, the card is owed and shown at the
            // first later hit of the episode that may show one.
            let which = step
                .card
                .or_else(|| rt.repeat_owed.then_some(EpisodeCard::Repeat));
            if which.is_some() {
                rt.rotation = rt.rotation.wrapping_add(1);
            }
            (step, which, rt.config.ui.discreet, rt.rotation, news)
        };
        if step.count {
            self.bump(match source {
                BlockSource::User => DayKind::BlockUser,
                BlockSource::List | BlockSource::Learned => DayKind::BlockList,
                BlockSource::Keyword | BlockSource::Tld => DayKind::BlockKeyword,
            });
        }

        let mut outbox = Outbox {
            status: bypass,
            ..Outbox::default()
        };
        if let Some(which) = which {
            // A correction names the host; a page without one (a reader view) gets none.
            let extra = host.and_then(|_| self.correction_button(source));
            let repeat = which == EpisodeCard::Repeat;
            let card = cards::blocked(
                repeat,
                discreet,
                source == BlockSource::User,
                rotation,
                extra,
                now,
            );
            let context = CardContext {
                key: Some(key),
                app_id: Some(window.app_id.clone()),
                host: host.map(str::to_owned),
                window_id: window.window_id,
                browser: true,
                source: Some(source),
                ..CardContext::default()
            };
            let mut rt = self.rt.lock();
            let shown = rt.cards.may_show(now);
            rt.repeat_owed = repeat && !shown;
            if shown {
                rt.cards.keep(card.clone(), context, now);
                // The operating system keeps notifications, with their times: the
                // stand-in is sent for the first card of an episode only.
                outbox
                    .cards
                    .push((card, (!repeat).then_some(Notice::Blocked)));
            }
        }
        self.flush(outbox);
        Ok(GuardOutcome {
            hit: true,
            job: None,
        })
    }

    /// The correction a block card offers (design 5.4): "Não era isso" on a keyword or
    /// TLD block while it would be a snooze, "Pedir exceção" once it would be a delayed
    /// request, and for a host the AI check learned. A hit on the embedded list or on the
    /// user's own list offers none on the card; the page has the delayed request.
    fn correction_button(&self, source: BlockSource) -> Option<CardAction> {
        match source {
            BlockSource::Keyword | BlockSource::Tld => Some(match self.not_it_effect(source) {
                NotItEffect::Snooze => CardAction::NotIt,
                NotItEffect::Request => CardAction::RequestException,
            }),
            BlockSource::Learned => Some(CardAction::RequestException),
            BlockSource::User | BlockSource::List => None,
        }
    }

    /// What "Não era isso" on a block of `source` would do now.
    pub(crate) fn not_it_effect(&self, source: BlockSource) -> NotItEffect {
        let (day, _) = self.day_hour(self.now());
        let used = self
            .deps
            .repo
            .days_between(&day, &day)
            .map(|counts| ubiqx_nofap::progress::total_on(&counts, &day, DayKind::NotIt))
            // Counters that cannot be read must not hand out a snooze.
            .unwrap_or(u32::MAX);
        sampling::not_it_on_block(source, self.vision_ready().is_ok(), used)
    }

    // -----------------------------------------------------------------------------------------
    // Sampling for the AI check
    // -----------------------------------------------------------------------------------------

    /// Nothing blocked is in front: ask the sampler whether this window is looked at now.
    fn sample(&self, front: &Front, now: DateTime<Utc>) -> CoreResult<GuardOutcome> {
        let window = &front.window;
        let kind = front.kind();
        let key = front.key();
        // The tracker knows a window without a host by its application and title.
        let window_key = context_key(None, &window.app_id, &window.window_title);
        let (prefs, seen, enabled_at, delay_secs) = {
            let mut rt = self.rt.lock();
            rt.confirmer.clear();
            (
                rt.config.vision.clone(),
                rt.config.internal.exclusions_seen.clone(),
                rt.config.enabled_at,
                rt.config.delay_secs,
            )
        };
        self.note_limit(self.limit_of(front));
        if !prefs.enabled || prefs.consent.is_none() {
            return Ok(GuardOutcome::default());
        }

        let env = self.deps.env.snapshot();
        self.note_provider(&env);
        let lists = ExclusionLists {
            apps: env.excluded_apps.clone(),
            domains: env.excluded_domains.clone(),
        };
        let effective = exclusions::effective(&lists, &seen, enabled_at, delay_secs, now);
        let ctx = sample_ctx(front, kind, key, &prefs, &effective);

        let (day, _) = self.day_hour(now);
        let decision = {
            let mut rt = self.rt.lock();
            sampling::record(&mut rt.sampler, key, SampleEvent::Front, &day, now);
            if ctx.local_signal() {
                rt.signals.insert(key, now);
                rt.signals.insert(window_key, now);
                forget_old(&mut rt.signals, now);
            }
            let decision = sampling::decide(&ctx, &rt.sampler, &day, now);
            let candidate = !matches!(
                decision,
                Decision::Skip(Skip::NotCandidate | Skip::Excluded | Skip::NoWindowId)
            );
            if let (true, Some(id)) = (candidate, window.window_id) {
                // "Ver um exemplo do que é enviado" shows this window, never the app's own.
                rt.last_candidate = Some((id, kind));
            }
            decision
        };
        let (Decision::Capture { .. }, Some(window_id)) = (decision, window.window_id) else {
            return Ok(GuardOutcome::default());
        };
        let asked_lately = self
            .rt
            .lock()
            .not_ready_at
            .is_some_and(|at| (0..NOT_READY_FOR_SECS).contains(&(now - at).num_seconds()));
        if asked_lately {
            return Ok(GuardOutcome::default());
        }
        if self.vision_ready_in(&env).is_err() {
            self.rt.lock().not_ready_at = Some(now);
            return Ok(GuardOutcome::default());
        }
        let Some(in_flight) = InFlight::begin(&self.in_flight) else {
            return Ok(GuardOutcome::default());
        };
        let recheck = self.rt.lock().recheck.remove(&key);
        Ok(GuardOutcome {
            hit: false,
            job: Some(VisionJob {
                key,
                window_key,
                kind,
                window_id,
                app_id: window.app_id.clone(),
                host: front.host.clone(),
                learnable: matches!(
                    front.verdict,
                    Some(Verdict::Unknown(HostClass::Plain { .. }))
                ),
                local_signal: ctx.local_signal(),
                recheck,
                model: env.vision_model,
                _in_flight: in_flight,
            }),
        })
    }
}

/// Whether a bypass suspicion is still fresh (it lasts as long as the heightened state).
pub(crate) fn bypass_recent(at: Option<DateTime<Utc>>, now: DateTime<Utc>) -> bool {
    at.is_some_and(|at| (0..HEIGHTENED_SECS).contains(&(now - at).num_seconds()))
}

/// What the sampler is told about the window in front: its kind, its verdict and its
/// signals. No address, title or application name goes in.
fn sample_ctx(
    front: &Front,
    kind: WindowKind,
    key: u64,
    prefs: &VisionPrefs,
    effective: &ExclusionLists,
) -> SampleCtx {
    let window = &front.window;
    let browser = matches!(kind, WindowKind::Browser | WindowKind::UnreadableBrowser);
    let mut ctx = SampleCtx::new(key, kind, prefs);
    ctx.verdict = front.verdict;
    ctx.has_window_id = window.window_id.is_some();
    ctx.private_window = browser && is_private_browsing_title(&window.window_title);
    ctx.title_signal = heuristics::title_signal(&window.window_title);
    ctx.path_signal = front.path_signal;
    ctx.excluded = exclusions::excludes(
        effective,
        &window.app_id,
        &window.app_name,
        front.host.as_deref(),
    );
    ctx
}
