//! Engine bootstrap: wires dependencies and spawns the runtime loops.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;
use tokio::time::MissedTickBehavior;
use tokio_util::sync::CancellationToken;
use ubiqx_core::segmenter::SegmenterConfig;
use ubiqx_core::*;

use crate::deps::EngineDeps;
use crate::handle::EngineHandle;
use crate::state::EngineState;
use crate::{classify, focus, nudges, reports, sampler, screenshots, tracker};

pub struct Engine;

/// Tunables for the background loops (tests shorten them).
#[derive(Debug, Clone)]
pub struct LoopConfig {
    pub classify_every: Duration,
    pub reports_every: Duration,
    pub nudges_every: Duration,
    pub retention_every: Duration,
    /// Wait before the first automatic update check, so start-up stays quiet.
    pub update_initial_delay: Duration,
    pub update_every: Duration,
    /// How often the focus guard looks at the foreground window.
    pub focus_every: Duration,
    /// Skip the sampler thread (the caller feeds samples itself).
    pub without_sampler: bool,
}

impl Default for LoopConfig {
    fn default() -> Self {
        Self {
            classify_every: Duration::from_secs(30),
            reports_every: Duration::from_secs(30),
            nudges_every: Duration::from_secs(60),
            retention_every: Duration::from_secs(3600),
            update_initial_delay: Duration::from_secs(45),
            update_every: Duration::from_secs(6 * 3600),
            focus_every: Duration::from_secs(2),
            without_sampler: false,
        }
    }
}

impl Engine {
    /// Starts the engine. Must be called from within a tokio runtime.
    pub fn start(deps: EngineDeps) -> CoreResult<EngineHandle> {
        Self::start_with(deps, LoopConfig::default()).map(|(h, _)| h)
    }

    /// Starts the engine and also returns the sample channel sender (for tests / replay).
    pub fn start_with(
        deps: EngineDeps,
        cfg: LoopConfig,
    ) -> CoreResult<(EngineHandle, mpsc::Sender<ActivitySample>)> {
        std::fs::create_dir_all(&deps.data_dir)
            .map_err(|e| CoreError::Platform(format!("data dir: {e}")))?;
        let settings = deps.repos.settings.load()?;
        let open_block = restore_open_block(&deps, &settings)?;
        let state = EngineState::new(deps, settings, open_block);
        let cancel = CancellationToken::new();
        let (tx, rx) = mpsc::channel::<ActivitySample>(256);

        if !cfg.without_sampler {
            sampler::spawn(state.clone(), tx.clone(), cancel.clone());
        }
        tokio::spawn(tracker::run(state.clone(), rx, cancel.clone()));
        tokio::spawn(classify_loop(
            state.clone(),
            cancel.clone(),
            cfg.classify_every,
        ));
        tokio::spawn(reports_loop(
            state.clone(),
            cancel.clone(),
            cfg.reports_every,
        ));
        tokio::spawn(nudges_loop(state.clone(), cancel.clone(), cfg.nudges_every));
        tokio::spawn(retention_loop(
            state.clone(),
            cancel.clone(),
            cfg.retention_every,
        ));
        tokio::spawn(update_loop(
            state.clone(),
            cancel.clone(),
            cfg.update_initial_delay,
            cfg.update_every,
        ));
        tokio::spawn(focus_loop(state.clone(), cancel.clone(), cfg.focus_every));

        state.refresh_tracker_state();
        state.deps.sink.emit(EngineEvent::AiHealth {
            health: state.ai_health(),
        });
        tracing::info!("ubiqX engine started");
        Ok((EngineHandle { state, cancel }, tx))
    }
}

/// Loads the open block left by the previous run. A block whose end is older than the
/// segmenter's gap tolerance (abrupt exit) or that cannot be extended because tracking is off
/// is closed and persisted now, instead of showing on the dashboard as still running.
fn restore_open_block(deps: &EngineDeps, settings: &Settings) -> CoreResult<Option<ActivityBlock>> {
    let Some(mut block) = deps.repos.blocks.open_block()? else {
        return Ok(None);
    };
    let now = deps.clock.now();
    let max_gap = chrono::Duration::seconds(SegmenterConfig::from(settings).max_gap_secs as i64);
    if settings.tracking_enabled && now - block.ended_at <= max_gap {
        return Ok(Some(block));
    }
    block.is_open = false;
    deps.repos.blocks.touch(&block)?;
    tracing::info!(block = %block.id, "closed stale open block from a previous run");
    Ok(None)
}

fn interval(period: Duration) -> tokio::time::Interval {
    let mut i = tokio::time::interval(period);
    i.set_missed_tick_behavior(MissedTickBehavior::Skip);
    i
}

async fn classify_loop(state: Arc<EngineState>, cancel: CancellationToken, every: Duration) {
    let mut tick = interval(every);
    loop {
        tokio::select! {
            _ = cancel.cancelled() => break,
            _ = tick.tick() => {
                match classify::run_once(&state, false).await {
                    Ok(r) if r.local + r.remote + r.vision > 0 => tracing::info!(?r, "classified"),
                    Ok(_) => {}
                    Err(e) => tracing::warn!(error = %e, "classification pass failed"),
                }
            }
        }
    }
}

async fn reports_loop(state: Arc<EngineState>, cancel: CancellationToken, every: Duration) {
    let mut tick = interval(every);
    // The first pass waits one period (30 s in production), like the nudges and the focus
    // guard: an immediate pass at start-up raced report settings written through the handle
    // right after the engine came up (seen as a flaky pipeline test on slower machines).
    tick.tick().await;
    loop {
        tokio::select! {
            _ = cancel.cancelled() => break,
            _ = tick.tick() => {
                if let Err(e) = reports::run_due(&state).await {
                    tracing::warn!(error = %e, "report scheduler failed");
                }
            }
        }
    }
}

async fn nudges_loop(state: Arc<EngineState>, cancel: CancellationToken, every: Duration) {
    let mut tick = interval(every);
    tick.tick().await; // do not nudge at start-up
    loop {
        tokio::select! {
            _ = cancel.cancelled() => break,
            _ = tick.tick() => {
                let st = state.clone();
                if let Err(e) = tokio::task::spawn_blocking(move || nudges::run_once(&st)).await.unwrap_or_else(|e| Err(CoreError::Other(e.to_string()))) {
                    tracing::warn!(error = %e, "nudge evaluation failed");
                }
                // Timed private mode expiry.
                let s = state.settings();
                if s.private_mode && s.private_until.map(|t| state.now() >= t).unwrap_or(false) {
                    let mut s = s;
                    s.private_mode = false;
                    s.private_until = None;
                    let _ = state.apply_settings(s);
                }
            }
        }
    }
}

async fn retention_loop(state: Arc<EngineState>, cancel: CancellationToken, every: Duration) {
    let mut tick = interval(every);
    loop {
        tokio::select! {
            _ = cancel.cancelled() => break,
            _ = tick.tick() => {
                let st = state.clone();
                if let Err(e) = tokio::task::spawn_blocking(move || screenshots::cleanup(&st)).await.unwrap_or_else(|e| Err(CoreError::Other(e.to_string()))) {
                    tracing::warn!(error = %e, "retention failed");
                }
            }
        }
    }
}

/// The focus guard: every `every`, looks at the foreground window, holds blocked apps and
/// sites and ends a focus session at its planned end. Platform calls are blocking, so each
/// pass runs on the blocking pool.
async fn focus_loop(state: Arc<EngineState>, cancel: CancellationToken, every: Duration) {
    let mut tick = interval(every);
    // The first pass waits one period (like the nudges): an immediate pass at start-up would
    // race the guard passes and session calls made through the handle meanwhile.
    tick.tick().await;
    loop {
        tokio::select! {
            _ = cancel.cancelled() => break,
            _ = tick.tick() => {
                let st = state.clone();
                if let Err(e) = tokio::task::spawn_blocking(move || focus::guard_once(&st)).await.unwrap_or_else(|e| Err(CoreError::Other(e.to_string()))) {
                    tracing::warn!(error = %e, "focus guard pass failed");
                }
            }
        }
    }
}

/// Automatic update checks: one shortly after start, then every `every`, each only while
/// the user wants them (`settings.check_updates`) and the build is a published one (a
/// development build has nothing to compare against). A manual check from the UI is
/// independent of this loop.
async fn update_loop(
    state: Arc<EngineState>,
    cancel: CancellationToken,
    initial_delay: Duration,
    every: Duration,
) {
    let mut delay = initial_delay;
    loop {
        tokio::select! {
            _ = cancel.cancelled() => break,
            _ = tokio::time::sleep(delay) => {
                delay = every;
                if !state.update.enabled() || !state.settings().check_updates {
                    continue;
                }
                if let Err(e) = state.update.check_scheduled().await {
                    tracing::warn!(error = %e, "update check failed");
                }
            }
        }
    }
}
