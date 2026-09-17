//! Consumes samples, drives the segmenter, persists blocks and triggers screenshots.

use std::sync::Arc;

use chrono::Duration;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use ubiqx_core::ports::*;
use ubiqx_core::*;

use crate::screenshots;
use crate::state::EngineState;

pub async fn run(
    state: Arc<EngineState>,
    mut rx: mpsc::Receiver<ActivitySample>,
    cancel: CancellationToken,
) {
    loop {
        let sample = tokio::select! {
            _ = cancel.cancelled() => break,
            s = rx.recv() => match s { Some(s) => s, None => break },
        };
        let st = state.clone();
        let res = tokio::task::spawn_blocking(move || handle_sample(&st, sample)).await;
        match res {
            Ok(Err(e)) => tracing::warn!(error = %e, "tracker failed to handle sample"),
            Err(e) => tracing::error!(error = %e, "tracker task panicked"),
            _ => {}
        }
    }
    // Close the open block on shutdown so no time is lost or inflated.
    let now = state.now();
    let closed = state.segmenter.lock().close(now);
    if let Some(block) = closed {
        if let Err(e) = state.deps.repos.blocks.update(&block) {
            tracing::warn!(error = %e, "could not persist block on shutdown");
        }
    }
}

/// Feeds one sample through the segmenter and persists the outcome. Runs on a blocking thread.
pub fn handle_sample(state: &Arc<EngineState>, sample: ActivitySample) -> CoreResult<()> {
    let settings = state.settings();
    let interval = Duration::seconds(settings.sample_interval_secs.max(1) as i64);
    let repos = &state.deps.repos;

    // Permission awareness: an empty title from a real window means Screen Recording is off.
    if sample.window_title.is_empty()
        && sample.window_id.is_none()
        && !sample.is_idle(settings.idle_threshold_secs)
    {
        let status = state.deps.platform.permissions.status();
        if status.screen_recording == PermissionState::Denied {
            let mut warned = state.permission_warned.lock();
            if !*warned {
                *warned = true;
                state.deps.sink.emit(EngineEvent::PermissionRequired {
                    permission: "screen_recording".into(),
                });
            }
        }
    }

    let outcome = state.segmenter.lock().feed(&sample, interval);

    if let Some(mut closed) = outcome.closed {
        closed.is_open = false;
        let significant = state.segmenter.lock().is_significant(&closed);
        if significant {
            repos.blocks.update(&closed)?;
            state.deps.sink.emit(EngineEvent::BlockClosed {
                block: closed.clone(),
            });
        } else {
            // Too short to matter: drop it so the timeline stays readable.
            repos.blocks.delete(&closed.id)?;
        }
        state.last_capture.lock().remove(&closed.id);
    }
    if let Some(opened) = outcome.opened {
        repos.blocks.insert(&opened)?;
        state
            .deps
            .sink
            .emit(EngineEvent::BlockOpened { block: opened });
    } else if let Some(open) = &outcome.open {
        repos.blocks.update(open)?;
    }

    let idle = sample.is_idle(settings.idle_threshold_secs);
    if idle {
        state.set_tracker_state(TrackerState::Idle);
    } else if settings.is_private(state.now()) {
        state.set_tracker_state(TrackerState::Private);
    } else {
        state.set_tracker_state(TrackerState::Running);
    }

    // Block-driven screenshot.
    if let Some(open) = outcome.open {
        if !idle && screenshots::should_capture(state, &open, state.now()) {
            let bounds = None; // bounds are not carried on samples; capturer falls back to the window id
            match screenshots::capture_for_block(state, &open, sample.window_id, bounds) {
                Ok(shot) => {
                    let mut b = open.clone();
                    b.screenshot_id = Some(shot.id);
                    repos.blocks.update(&b)?;
                }
                Err(CoreError::Permission(_)) => {
                    // Already surfaced through PermissionRequired; avoid retrying every sample.
                    state
                        .last_capture
                        .lock()
                        .insert(open.id.clone(), state.now());
                }
                Err(e) => {
                    tracing::debug!(error = %e, "screenshot skipped");
                    state
                        .last_capture
                        .lock()
                        .insert(open.id.clone(), state.now());
                }
            }
        }
    }
    Ok(())
}
