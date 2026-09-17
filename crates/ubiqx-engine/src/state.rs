//! Shared mutable state of a running engine.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use parking_lot::{Mutex, RwLock};
use ubiqx_core::segmenter::{Segmenter, SegmenterConfig};
use ubiqx_core::*;

use crate::deps::EngineDeps;

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
}

impl EngineState {
    pub fn new(
        deps: EngineDeps,
        settings: Settings,
        open_block: Option<ActivityBlock>,
    ) -> Arc<Self> {
        let mut cfg = SegmenterConfig::from(&settings);
        cfg.private_mode = settings.is_private(deps.clock.now());
        let health = if deps.ai.remote.is_none() && deps.ai.report_writer.is_none() {
            AiHealth::NotConfigured
        } else if settings.local_only || !deps.ai.requires_api_key {
            AiHealth::Ok
        } else {
            match deps
                .platform
                .secrets
                .get(ports::secret_keys::ANTHROPIC_API_KEY)
            {
                Ok(Some(k)) if !k.trim().is_empty() => AiHealth::Ok,
                _ => AiHealth::NotConfigured,
            }
        };
        Arc::new(Self {
            deps,
            settings: RwLock::new(settings),
            segmenter: Mutex::new(Segmenter::new(cfg).with_open_block(open_block)),
            tracker: RwLock::new(TrackerState::Running),
            ai_health: RwLock::new(health),
            last_capture: Mutex::new(HashMap::new()),
            paused: RwLock::new(false),
            permission_warned: Mutex::new(false),
        })
    }

    pub fn now(&self) -> DateTime<Utc> {
        self.deps.clock.now()
    }

    pub fn settings(&self) -> Settings {
        self.settings.read().clone()
    }

    /// Recomputes derived state after a settings change and persists the settings.
    pub fn apply_settings(&self, settings: Settings) -> CoreResult<()> {
        self.deps.repos.settings.save(&settings)?;
        let mut cfg = SegmenterConfig::from(&settings);
        cfg.private_mode = settings.is_private(self.now());
        self.segmenter.lock().update_config(cfg);
        *self.settings.write() = settings;
        self.refresh_tracker_state();
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

    /// Derives the tracker state from pause flag, settings and private mode.
    pub fn refresh_tracker_state(&self) {
        let s = self.settings.read();
        let state = if *self.paused.read() || !s.tracking_enabled {
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

    /// Whether remote classifiers may be called right now.
    pub fn remote_allowed(&self) -> bool {
        if self.settings.read().local_only {
            return false;
        }
        match self.ai_health() {
            AiHealth::Ok => true,
            AiHealth::Degraded { until, .. } => self.now() >= until,
            AiHealth::NotConfigured | AiHealth::Paused { .. } => false,
        }
    }
}
