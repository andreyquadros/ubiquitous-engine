//! Shared mutable state of a running engine.

use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use parking_lot::{Condvar, Mutex, RwLock};
use ubiqx_core::segmenter::{Segmenter, SegmenterConfig};
use ubiqx_core::*;

use crate::deps::EngineDeps;
use crate::focus::FocusRuntime;
use crate::update::UpdateChecker;

pub struct EngineState {
    pub deps: EngineDeps,
    pub settings: RwLock<Settings>,
    pub segmenter: Mutex<Segmenter>,
    pub tracker: RwLock<TrackerState>,
    pub ai_health: RwLock<AiHealth>,
    /// Last screenshot instant per open block id (block-driven capture cadence).
    pub last_capture: Mutex<HashMap<Id, DateTime<Utc>>>,
    /// Set while the user explicitly paused tracking from the tray/UI.
    pub paused: RwLock<bool>,
    /// Emitted once per missing permission so the UI is not flooded.
    pub permission_warned: Mutex<bool>,
    /// Set while `AiHealth::Paused` was caused by the monthly budget (as opposed to a rejected
    /// key), so the budget check is the only thing that lifts it.
    pub budget_paused: AtomicBool,
    /// Flipped by the tracker task once it has persisted the open block on shutdown, so
    /// `EngineHandle::shutdown` can wait for it.
    pub tracker_stopped: Mutex<bool>,
    pub tracker_stopped_cv: Condvar,
    /// Looks for newer builds in the update feed and remembers what was announced.
    pub update: UpdateChecker,
    /// The focus guard's in-memory state: the active session, per-key cooldowns and the
    /// message rotation.
    pub focus: Mutex<FocusRuntime>,
}

impl EngineState {
    pub fn new(
        deps: EngineDeps,
        settings: Settings,
        open_block: Option<ActivityBlock>,
    ) -> Arc<Self> {
        // Stored settings may predate a validation rule (or carry a language tag spelled
        // differently): they go through the same sanitising as a settings write.
        let mut settings = settings;
        settings.sanitize();
        let mut cfg = SegmenterConfig::from(&settings);
        cfg.private_mode = settings.is_private(deps.clock.now());
        let health = key_health(&deps, &settings);
        // A session left behind by the previous run continues (or is closed by the guard's
        // first pass when its end already passed).
        let focus = FocusRuntime::restore(deps.repos.focus.as_ref());
        let update = UpdateChecker::new(
            deps.build.clone(),
            deps.update_feed_url.clone(),
            deps.platform.update_feed.clone(),
            deps.repos.kv.clone(),
            deps.repos.settings.clone(),
            deps.platform.notifier.clone(),
            deps.sink.clone(),
            deps.clock.clone(),
        );
        Arc::new(Self {
            deps,
            settings: RwLock::new(settings),
            segmenter: Mutex::new(Segmenter::new(cfg).with_open_block(open_block)),
            tracker: RwLock::new(TrackerState::Running),
            ai_health: RwLock::new(health),
            last_capture: Mutex::new(HashMap::new()),
            paused: RwLock::new(false),
            permission_warned: Mutex::new(false),
            budget_paused: AtomicBool::new(false),
            tracker_stopped: Mutex::new(false),
            tracker_stopped_cv: Condvar::new(),
            update,
            focus: Mutex::new(focus),
        })
    }

    pub fn now(&self) -> DateTime<Utc> {
        self.deps.clock.now()
    }

    pub fn settings(&self) -> Settings {
        self.settings.read().clone()
    }

    /// Recomputes derived state after a settings change and persists the settings.
    ///
    /// Turning tracking off (or flipping private mode) closes the open block right away, so the
    /// dashboard never keeps extending a block while nothing is being sampled.
    pub fn apply_settings(&self, mut settings: Settings) -> CoreResult<()> {
        // Model ids of another vendor never survive a provider switch, and the language tag
        // is stored in its canonical spelling (`pt-BR` / `en`).
        settings.sanitize();
        self.deps.repos.settings.save(&settings)?;
        let now = self.now();
        let mut cfg = SegmenterConfig::from(&settings);
        cfg.private_mode = settings.is_private(now);
        let (closing, budget_changed, provider_changed) = {
            let cur = self.settings.read();
            (
                (cur.tracking_enabled && !settings.tracking_enabled)
                    || cur.is_private(now) != cfg.private_mode,
                cur.ai_monthly_budget_usd != settings.ai_monthly_budget_usd,
                cur.ai_provider != settings.ai_provider,
            )
        };
        let closed = {
            let mut seg = self.segmenter.lock();
            let closed = if closing { seg.close(now) } else { None };
            seg.update_config(cfg);
            closed
        };
        if let Some(block) = closed {
            self.deps.repos.blocks.touch(&block)?;
            self.last_capture.lock().remove(&block.id);
        }
        // Another vendor answers from now on: its stored key decides whether the AI path is
        // armed, exactly like at start-up (a pause caused by the old vendor's key is over).
        let health = provider_changed.then(|| key_health(&self.deps, &settings));
        *self.settings.write() = settings;
        self.refresh_tracker_state();
        if let Some(health) = health {
            self.set_ai_health(health);
        }
        // A raised (or removed) budget resumes the AI immediately instead of on the next pass;
        // health is otherwise left alone because this path also serves snooze/private toggles.
        if budget_changed && self.budget_paused.load(std::sync::atomic::Ordering::SeqCst) {
            let _ = crate::classify::budget_allows(self);
        }
        Ok(())
    }

    pub fn set_tracker_state(&self, state: TrackerState) {
        let changed = {
            let mut g = self.tracker.write();
            if *g != state {
                *g = state;
                true
            } else {
                false
            }
        };
        if changed {
            self.deps.sink.emit(EngineEvent::TrackerState { state });
        }
    }

    /// Derives the tracker state from pause flag, settings and private mode. Nothing is
    /// recorded before onboarding is done (the user has not consented yet).
    pub fn refresh_tracker_state(&self) {
        let s = self.settings.read();
        let state = if *self.paused.read() || !s.tracking_enabled || !s.onboarding_done {
            TrackerState::Paused
        } else if s.is_private(self.now()) {
            TrackerState::Private
        } else {
            match *self.tracker.read() {
                TrackerState::Idle => TrackerState::Idle,
                TrackerState::Blocked => TrackerState::Blocked,
                _ => TrackerState::Running,
            }
        };
        drop(s);
        self.set_tracker_state(state);
    }

    pub fn set_ai_health(&self, health: AiHealth) {
        let changed = {
            let mut g = self.ai_health.write();
            if *g != health {
                *g = health.clone();
                true
            } else {
                false
            }
        };
        if changed {
            tracing::info!(?health, "AI health changed");
            self.deps.sink.emit(EngineEvent::AiHealth { health });
        }
    }

    pub fn ai_health(&self) -> AiHealth {
        self.ai_health.read().clone()
    }

    /// Whether remote classifiers may be called right now. Remote AI is armed by consent, not
    /// by key presence alone: a Keychain key left over from a previous install must not send
    /// anything before the user finishes onboarding.
    pub fn remote_allowed(&self) -> bool {
        {
            let s = self.settings.read();
            if s.local_only || !s.onboarding_done {
                return false;
            }
        }
        match self.ai_health() {
            AiHealth::Ok => true,
            AiHealth::Degraded { until, .. } => self.now() >= until,
            AiHealth::NotConfigured | AiHealth::Paused { .. } => false,
        }
    }
}

/// AI health derived from the deps and the key of the selected provider: `NotConfigured`
/// without any remote component, `Ok` when no key is needed (local mode, fakes) and
/// otherwise `Ok`/`NotConfigured` by the presence of `settings.ai_provider`'s key in the
/// secret store.
pub(crate) fn key_health(deps: &EngineDeps, settings: &Settings) -> AiHealth {
    if deps.ai.remote.is_none() && deps.ai.report_writer.is_none() {
        AiHealth::NotConfigured
    } else if settings.local_only || !deps.ai.requires_api_key {
        AiHealth::Ok
    } else {
        match deps.platform.secrets.get(settings.ai_provider.secret_key()) {
            Ok(Some(k)) if !k.trim().is_empty() => AiHealth::Ok,
            _ => AiHealth::NotConfigured,
        }
    }
}
