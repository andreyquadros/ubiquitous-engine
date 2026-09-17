//! The sampler runs on its own OS thread because every platform call is blocking
//! (window server queries, AppleScript). It produces one `ActivitySample` per interval.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use ubiqx_core::*;

use crate::state::EngineState;

/// Spawns the sampler thread. Samples are sent to `tx`; the thread exits when `cancel` fires.
pub fn spawn(state: Arc<EngineState>, tx: mpsc::Sender<ActivitySample>, cancel: CancellationToken) {
    std::thread::Builder::new()
        .name("ubiqx-sampler".into())
        .spawn(move || run(state, tx, cancel))
        .expect("spawn sampler thread");
}

fn run(state: Arc<EngineState>, tx: mpsc::Sender<ActivitySample>, cancel: CancellationToken) {
    let mut last_title: Option<(String, String)> = None;
    let mut last_url: Option<String> = None;
    loop {
        if cancel.is_cancelled() {
            return;
        }
        let interval = state.settings.read().sample_interval_secs.clamp(1, 60);
        let tracking = {
            let s = state.settings.read();
            s.tracking_enabled && !*state.paused.read()
        };
        if tracking {
            match take_sample(&state, &mut last_title, &mut last_url) {
                Ok(Some(sample)) => {
                    if tx.blocking_send(sample).is_err() {
                        return;
                    }
                }
                Ok(None) => {}
                Err(e) => tracing::warn!(error = %e, "sampling failed"),
            }
        }
        // Sleep in small steps so shutdown is prompt.
        let mut slept = 0u64;
        while slept < interval as u64 * 1000 {
            if cancel.is_cancelled() {
                return;
            }
            std::thread::sleep(Duration::from_millis(250));
            slept += 250;
        }
    }
}

fn take_sample(
    state: &EngineState,
    last_title: &mut Option<(String, String)>,
    last_url: &mut Option<String>,
) -> CoreResult<Option<ActivitySample>> {
    let platform = &state.deps.platform;
    let now = state.now();
    let idle = platform.idle.idle_secs().unwrap_or(None);
    let Some(win) = platform.activity.foreground()? else {
        return Ok(None);
    };

    // Resolve the browser URL only when the app is a browser and the title changed.
    let mut url = None;
    if platform.urls.supports(&win.app_id) {
        let key = (win.app_id.clone(), win.window_title.clone());
        if last_title.as_ref() == Some(&key) {
            url = last_url.clone();
        } else {
            match platform.urls.resolve(&win) {
                Ok(u) => url = u,
                Err(CoreError::Permission(msg)) => {
                    tracing::warn!(%msg, "browser automation not permitted");
                    state.deps.sink.emit(EngineEvent::PermissionRequired {
                        permission: "automation".into(),
                    });
                }
                Err(e) => tracing::debug!(error = %e, "url resolution failed"),
            }
            *last_title = Some(key);
            *last_url = url.clone();
        }
    }

    Ok(Some(ActivitySample {
        at: now,
        app_name: win.app_name,
        app_id: win.app_id,
        window_title: win.window_title,
        url,
        idle_secs: idle,
        window_id: win.window_id,
    }))
}
