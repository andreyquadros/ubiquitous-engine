//! Consumes samples, drives the segmenter, persists blocks and triggers screenshots.

use std::sync::Arc;

use chrono::Duration;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use ubiqx_core::normalize::normalize_title;
use ubiqx_core::ports::*;
use ubiqx_core::segmenter::PRIVATE_TITLE;
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
        if let Err(e) = state.deps.repos.blocks.touch(&block) {
            tracing::warn!(error = %e, "could not persist block on shutdown");
        }
    }
    // Let `EngineHandle::shutdown` return: the block is on disk.
    *state.tracker_stopped.lock() = true;
    state.tracker_stopped_cv.notify_all();
}

/// Feeds one sample through the segmenter and persists the outcome. Runs on a blocking thread.
pub fn handle_sample(state: &Arc<EngineState>, sample: ActivitySample) -> CoreResult<()> {
    let mut sample = sample;
    let settings = state.settings();
    let interval = Duration::seconds(settings.sample_interval_secs.max(1) as i64);
    let repos = &state.deps.repos;

    // A sample queued before pause()/disable (or still in flight during a capture) must not
    // reopen a block that will never be closed: drop it and close whatever is still open.
    if *state.paused.read() || !settings.tracking_enabled || !settings.onboarding_done {
        let closed = state.segmenter.lock().close(sample.at.min(state.now()));
        if let Some(block) = closed {
            repos.blocks.touch(&block)?;
            state.last_capture.lock().remove(&block.id);
        }
        return Ok(());
    }

    // Permission awareness: an empty title on a non-idle sample usually means Screen Recording
    // is off (macOS still reports the window id without it); the platform status is the gate.
    if sample.window_title.is_empty() && !sample.is_idle(settings.idle_threshold_secs) {
        let mut warned = state.permission_warned.lock();
        if !*warned
            && state.deps.platform.permissions.status().screen_recording == PermissionState::Denied
        {
            *warned = true;
            state.deps.sink.emit(EngineEvent::PermissionRequired {
                permission: "screen_recording".into(),
            });
        }
    }

    // The `nofap` module: a sample of a blocked page, or of a window that had a hit in the
    // last ten minutes, is recorded the way private browsing is. The time stays; the
    // title, the address and the window do not.
    if state.nofap.redacts(&sample) {
        sample.window_title = PRIVATE_TITLE.into();
        sample.url = None;
        sample.window_id = None;
    }

    let outcome = state.segmenter.lock().feed(&sample, interval);

    if let Some(mut closed) = outcome.closed {
        closed.is_open = false;
        let significant = state.segmenter.lock().is_significant(&closed);
        if significant {
            repos.blocks.touch(&closed)?;
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
        // Only the segmenter-owned columns: a user reclassification or a screenshot link
        // written meanwhile must survive.
        repos.blocks.touch(open)?;
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
                    // Never re-persist the stale `open` clone: pause() may have closed the
                    // block while the capture ran.
                    repos.blocks.set_screenshot(&open.id, &shot.id)?;
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

/// After a hit of the `nofap` module: the block the tracker had open is closed and made
/// private, so nothing of the page stays in the productivity data. Its title becomes
/// [`PRIVATE_TITLE`], its address, description and the text sent to the AI go, its
/// category becomes the private one, and its screenshots are deleted, rows and files.
/// A block too short to be kept is dropped altogether. Runs on a blocking thread.
pub fn scrub_open_block(state: &Arc<EngineState>) -> CoreResult<()> {
    let now = state.now();
    let (closed, significant) = {
        let mut segmenter = state.segmenter.lock();
        let Some(closed) = segmenter.close(now) else {
            return Ok(());
        };
        let significant = segmenter.is_significant(&closed);
        (closed, significant)
    };
    let repos = &state.deps.repos;
    state.last_capture.lock().remove(&closed.id);
    let mut block = closed;
    block.is_open = false;
    block.title = PRIVATE_TITLE.into();
    block.title_key = normalize_title(PRIVATE_TITLE, &block.app_name);
    block.url = None;
    block.domain = None;
    block.description = None;
    block.ai_payload = None;
    block.ai_sent_at = None;
    block.category_id = Some(system_categories::PRIVATE.to_string());
    block.confidence = 1.0;
    block.source = Some(ClassificationSource::Rule);
    block.needs_review = false;
    // The pointer goes with the row, before the screenshots do: a later write of a block
    // that still carried it would put it back.
    block.screenshot_id = None;
    if significant {
        repos.blocks.update(&block)?;
    } else {
        repos.blocks.delete(&block.id)?;
    }
    for shot in repos.screenshots.delete_for_block(&block.id)? {
        screenshots::unlink(&shot);
    }
    if significant {
        state.deps.sink.emit(EngineEvent::BlockClosed { block });
    }
    Ok(())
}
