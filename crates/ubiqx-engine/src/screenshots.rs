//! Block-driven screenshot capture and file management.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use ubiqx_core::ports::*;
use ubiqx_core::*;

use crate::state::EngineState;

/// Minimum block age before its first screenshot: short blocks never get captured.
pub const FIRST_CAPTURE_AFTER_SECS: i64 = 20;

pub fn screenshots_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("screenshots.noindex")
}

/// Decides whether the open block deserves a screenshot right now.
pub fn should_capture(state: &EngineState, block: &ActivityBlock, now: DateTime<Utc>) -> bool {
    let s = state.settings.read();
    if s.screenshot_interval_secs == 0 || s.is_private(now) {
        return false;
    }
    if block.category_id.as_deref() == Some(system_categories::PRIVATE) {
        return false;
    }
    let vision_ok = s.vision_policy.allows(
        &block.app_id,
        &block.app_name,
        &s.blocked_apps,
        &s.vision_denied_apps,
    );
    if !vision_ok && !s.keep_screenshots_for_review {
        return false;
    }
    if (now - block.started_at).num_seconds() < FIRST_CAPTURE_AFTER_SECS {
        return false;
    }
    let last = state.last_capture.lock().get(&block.id).copied();
    match last {
        None => true,
        Some(t) => now - t >= Duration::seconds(s.screenshot_interval_secs as i64),
    }
}

/// Captures the block's window (or, without a window id, the display containing `bounds`),
/// stores the file and the metadata row, and attaches it to the block. Blocking: call inside
/// `spawn_blocking`.
pub fn capture_for_block(
    state: &Arc<EngineState>,
    block: &ActivityBlock,
    window_id: Option<u32>,
    bounds: Option<(i32, i32, u32, u32)>,
) -> CoreResult<Screenshot> {
    let now = state.now();
    let settings = state.settings();
    let platform = &state.deps.platform;

    // Never capture while a blocked app has a window on screen.
    let visible = platform.capturer.visible_apps().unwrap_or_default();
    if visible.iter().any(|v| {
        settings
            .blocked_apps
            .iter()
            .any(|b| b.eq_ignore_ascii_case(v))
    }) {
        return Err(CoreError::Invalid("blocked app visible".into()));
    }

    let target = match (window_id, bounds) {
        (Some(id), _) => CaptureTarget::Window(id),
        (None, Some((x, y, w, h))) => CaptureTarget::DisplayAt {
            x: x + (w as i32) / 2,
            y: y + (h as i32) / 2,
        },
        _ => CaptureTarget::PrimaryDisplay,
    };
    // No display fallback when the window capture fails: a whole-display grab would include
    // other apps' windows (password managers' panels, other users' content), which the
    // "active window only" promise forbids. The caller just skips this capture.
    let image = platform
        .capturer
        .capture(target, settings.screenshot_max_edge)?;

    let dir = screenshots_dir(&state.deps.data_dir).join(now.format("%Y-%m-%d").to_string());
    std::fs::create_dir_all(&dir).map_err(|e| CoreError::Platform(format!("mkdir: {e}")))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(
            screenshots_dir(&state.deps.data_dir),
            std::fs::Permissions::from_mode(0o700),
        );
    }
    let id = new_id();
    let path = dir.join(format!("{id}.jpg"));
    std::fs::write(&path, &image.bytes).map_err(|e| CoreError::Platform(format!("write: {e}")))?;

    let shot = Screenshot {
        id,
        at: now,
        path: path.to_string_lossy().to_string(),
        width: image.width,
        height: image.height,
        app_id: block.app_id.clone(),
        block_id: Some(block.id.clone()),
        sent_to_ai: false,
    };
    state.deps.repos.screenshots.insert(&shot)?;
    state.last_capture.lock().insert(block.id.clone(), now);
    state.deps.sink.emit(EngineEvent::ScreenshotTaken {
        screenshot_id: shot.id.clone(),
        block_id: Some(block.id.clone()),
    });
    Ok(shot)
}

/// Reads a stored screenshot as an `EncodedImage`.
pub fn load(shot: &Screenshot) -> CoreResult<EncodedImage> {
    let bytes = std::fs::read(&shot.path)
        .map_err(|e| CoreError::Platform(format!("read screenshot: {e}")))?;
    Ok(EncodedImage {
        bytes,
        mime: "image/jpeg".into(),
        width: shot.width,
        height: shot.height,
    })
}

/// Deletes the file of a screenshot (metadata row stays until retention removes it).
pub fn unlink(shot: &Screenshot) {
    if let Err(e) = std::fs::remove_file(&shot.path) {
        if e.kind() != std::io::ErrorKind::NotFound {
            tracing::debug!(path = %shot.path, error = %e, "could not delete screenshot");
        }
    }
}

/// Removes screenshots older than the configured retention.
pub fn cleanup(state: &EngineState) -> CoreResult<usize> {
    let hours = state.settings.read().screenshot_retention_hours.max(1) as i64;
    let before = state.now() - Duration::hours(hours);
    let rows = state.deps.repos.screenshots.delete_before(before)?;
    for r in &rows {
        unlink(r);
    }
    if !rows.is_empty() {
        tracing::info!(count = rows.len(), "screenshots purged");
    }
    // Remove empty day folders.
    if let Ok(entries) = std::fs::read_dir(screenshots_dir(&state.deps.data_dir)) {
        for e in entries.flatten() {
            let _ = std::fs::remove_dir(e.path());
        }
    }
    Ok(rows.len())
}
